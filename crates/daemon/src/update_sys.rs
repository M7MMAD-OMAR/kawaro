//! The system side of the update scheduler (S-054b): the real state, the
//! saved schedule, notifications and the installer. Nothing here is decided;
//! the rules are in `update.rs` and `sbarhive_core::update`. Covered live by
//! tests/e2e/run.sh and the S-052 live check, so it is left out of the
//! mutants run (`.cargo/mutants.toml`).

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use sbarhive_core::config::Config;
use sbarhive_core::i18n::Lang;
use sbarhive_core::paths::{CURRENT, cache_dir_with, update_state_file_with};
use sbarhive_core::update::flow::platform_with;
use sbarhive_core::update::schedule::State as Schedule;
use sbarhive_core::update::verify::UPDATE_KEYS;

use crate::notify::{self, UpdateKind};
use crate::state::{Shared, lock};
use crate::update::{Effect, UpdateEnv, UpdateState};

static SHARED: OnceLock<Mutex<UpdateState>> = OnceLock::new();
#[cfg(target_os = "linux")]
static IDLE: OnceLock<UpdateState> = OnceLock::new();

fn env() -> UpdateEnv {
    let cache = cache_dir_with(CURRENT, &|k| std::env::var_os(k), &|| None)
        .unwrap_or_else(|| std::env::temp_dir().join("sbarhive-cache"));
    UpdateEnv {
        installed: env!("CARGO_PKG_VERSION").to_string(),
        keys: UPDATE_KEYS.iter().map(|k| k.to_string()).collect(),
        platform: platform_with(CURRENT, std::env::consts::ARCH, &|p| {
            std::path::Path::new(p).exists()
        }),
        os: CURRENT,
        updates_dir: cache.join("updates"),
        scratch: cache.join(format!("check-{}", std::process::id())),
    }
}

/// The one update state of the daemon.
pub fn shared() -> &'static Mutex<UpdateState> {
    SHARED.get_or_init(|| {
        let env = env();
        let _ = std::fs::create_dir_all(&env.scratch);
        Mutex::new(UpdateState::new(env))
    })
}

/// An idle state for the tray to show while a check or download holds the
/// real one, so the tray never waits on the network.
#[cfg(target_os = "linux")]
pub fn idle_view() -> &'static UpdateState {
    IDLE.get_or_init(|| UpdateState::new(env()))
}

pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn schedule_file() -> Option<PathBuf> {
    update_state_file_with(CURRENT, &|k| std::env::var_os(k), &|| None)
}

pub fn load_schedule() {
    if let Some(text) = schedule_file().and_then(|p| std::fs::read_to_string(p).ok()) {
        lock(shared()).check.schedule = Schedule::from_json(&text);
    }
}

pub fn save_schedule(schedule: &Schedule) {
    if let Some(path) = schedule_file() {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, schedule.to_json());
    }
}

/// The settings, the number of running jobs and whether the hive is open,
/// read before the update lock is taken so the two locks never nest.
pub fn snapshot(state: &Shared) -> (Config, usize, bool) {
    let st = lock(state);
    (st.config.clone(), st.jobs.len(), st.pending.is_some())
}

/// True when a verified update waits; false while a check or download holds
/// the state, so a menu never waits on the network.
#[cfg(windows)]
pub fn update_ready() -> bool {
    shared()
        .try_lock()
        .is_ok_and(|update| crate::text::update_item_enabled(&update))
}

/// `Update now` from a tray menu: asks for the install on its own thread.
pub fn apply_from_menu(state: Shared) {
    std::thread::spawn(move || {
        let (_, jobs, hive_open) = snapshot(&state);
        let lang = lock(&state).lang;
        let (response, effects) = {
            let mut update = lock(shared());
            update.running_jobs = jobs;
            update.hive_open = hive_open;
            crate::update::request_apply(&mut update)
        };
        if let sbarhive_core::ipc::Response::Error { message } = response {
            eprintln!("sbarhived: update: {message}");
        }
        run_effects(effects, lang);
    });
}

/// Does what a step asked for. The install runs on its own thread, so the
/// caller (a tick or a socket reply) never waits for `pkexec`.
pub fn run_effects(effects: Vec<Effect>, lang: Lang) {
    for effect in effects {
        match effect {
            Effect::Notify(kind, version) => notify::notify_update(lang, kind, &version),
            Effect::Apply => {
                std::thread::spawn(move || apply(lang));
            }
        }
    }
}

fn apply(lang: Lang) {
    let (path, sha256, version, kind) = {
        let update = lock(shared());
        let (Some(package), Some(ready)) = (&update.package, &update.ready) else {
            return;
        };
        (
            package.path().to_path_buf(),
            ready.asset.sha256.clone(),
            ready.version.clone(),
            update.env.platform.pkg.clone(),
        )
    };
    let started = install(lang, &path, &sha256, &kind);
    crate::tray::refresh();
    if let Err(()) = started {
        notify::notify_update(lang, UpdateKind::Failed, &version);
    } else if cfg!(unix) {
        notify::notify_update(lang, UpdateKind::Updated, &version);
        restart_daemon();
    }
}

#[cfg(windows)]
fn install(_lang: Lang, path: &std::path::Path, _sha256: &str, _kind: &str) -> Result<(), ()> {
    // Starts the helper copy and asks this daemon to quit over IPC.
    sbarhive_core::update::apply::apply_msi(path).map_err(|_| ())
}

#[cfg(unix)]
fn install(lang: Lang, path: &std::path::Path, sha256: &str, kind: &str) -> Result<(), ()> {
    use sbarhive_core::update::apply::{
        CommandRunner, Facts, PKEXEC, PkgKind, Plan, apply_now, linux_plan, privileged_argv_checked,
    };
    struct Runner;
    impl CommandRunner for Runner {
        fn run(&self, argv: &[String]) -> Result<i32, sbarhive_core::update::UpdateError> {
            std::process::Command::new(&argv[0])
                .args(&argv[1..])
                .status()
                .map_err(|_| sbarhive_core::update::UpdateError::Io)?
                .code()
                .ok_or(sbarhive_core::update::UpdateError::Io)
        }
    }
    let exists = |p: &str| std::path::Path::new(p).exists();
    let installed_kind = if kind == "deb" {
        PkgKind::Deb
    } else {
        PkgKind::Rpm
    };
    let repo_configured = ["/etc/yum.repos.d", "/etc/apt/sources.list.d"]
        .iter()
        .filter_map(|d| std::fs::read_dir(d).ok())
        .flatten()
        .flatten()
        .any(|e| {
            let name = e.file_name().to_string_lossy().to_lowercase();
            (name.contains("sbarhive") && name.ends_with(".repo")) || name == "sbarhive.sources"
        });
    let plan = linux_plan(&Facts {
        has_dnf: exists("/usr/bin/dnf"),
        has_apt: exists("/usr/bin/apt-get"),
        has_pkexec: exists(PKEXEC),
        repo_configured,
        installed_kind: Some(installed_kind),
    });
    // The CLI that runs the privileged step sits next to this daemon.
    let cli = daemon_exe().and_then(|e| e.parent().map(|d| d.join("sbarhive")));
    let cli = cli.unwrap_or_else(|| PathBuf::from("/usr/bin/sbarhive"));
    match &plan {
        Plan::ShowCommand(cmd) => {
            let how = cmd.replace("{file}", &path.to_string_lossy());
            notify::info(lang, &crate::text::update_manual(lang, &how));
            Err(())
        }
        Plan::ShowRelease(url) => {
            notify::info(lang, &crate::text::update_manual(lang, url));
            Err(())
        }
        Plan::Local(k) => {
            privileged_argv_checked(&cli, path, sha256, *k).map_err(|_| ())?;
            apply_now(&plan, &cli, path, sha256, &Runner).map_err(|_| ())
        }
        Plan::RepoUpgrade(_) => apply_now(&plan, &cli, path, sha256, &Runner).map_err(|_| ()),
    }
}

/// This program's path, without the ` (deleted)` Linux adds after a package
/// replaced the file.
#[cfg(unix)]
fn daemon_exe() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let text = exe.to_string_lossy();
    Some(PathBuf::from(
        text.strip_suffix(" (deleted)").unwrap_or(&text),
    ))
}

/// Starts the new daemon a moment after this one quits and frees the socket.
#[cfg(unix)]
fn restart_daemon() {
    if let Some(exe) = daemon_exe() {
        let _ = std::process::Command::new("/bin/sh")
            .args(["-c", "sleep 2; exec \"$0\""])
            .arg(exe)
            .spawn();
    }
    crate::ui::quit();
}

#[cfg(windows)]
fn restart_daemon() {}
