//! The system side of the update scheduler (S-054b): the real state, the
//! saved schedule, notifications and the installer. Nothing here is decided;
//! the rules are in `update.rs` and `kawaro_core::update`. Covered live by
//! tests/e2e/run.sh and the S-052 live check, so it is left out of the
//! mutants run (`.cargo/mutants.toml`).

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use kawaro_core::config::Config;
use kawaro_core::i18n::Lang;
use kawaro_core::paths::{CURRENT, cache_dir_with, update_state_file_with};
use kawaro_core::transport::Bound;
use kawaro_core::update::flow::platform_with;
use kawaro_core::update::schedule::State as Schedule;
use kawaro_core::update::verify::UPDATE_KEYS;

use crate::notify::{self, UpdateKind};
use crate::state::{Shared, lock};
use crate::update::{Effect, UpdateEnv, UpdateState};

static SHARED: OnceLock<Mutex<UpdateState>> = OnceLock::new();
#[cfg(target_os = "linux")]
static IDLE: OnceLock<UpdateState> = OnceLock::new();

fn env() -> UpdateEnv {
    let cache = cache_dir_with(CURRENT, &|k| std::env::var_os(k), &|| None)
        .unwrap_or_else(|| std::env::temp_dir().join("kawaro-cache"));
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

/// The length of a part file, for the progress snapshot.
pub fn part_len(path: &std::path::Path) -> Option<u64> {
    std::fs::metadata(path).ok().map(|m| m.len())
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
        let _ = crate::update::write_atomic(&path, &schedule.to_json());
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
            // Never waits for a download that holds the update lock.
            let mut guard = shared().try_lock().ok();
            if let Some(update) = guard.as_mut() {
                update.running_jobs = jobs;
                update.hive_open = hive_open;
            }
            crate::update::request_apply_or_busy(guard.as_deref_mut())
        };
        if let kawaro_core::ipc::Response::Error { message } = response {
            eprintln!("kawarod: update: {message}");
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
    let mut version = String::new();
    // The install holds the update lock, so a poll is answered from
    // `set_installing` instead (S-099 A3).
    let result = {
        let mut update = lock(shared());
        crate::update::apply_with(&mut update, |s| {
            let (Some(package), Some(ready)) = (&s.package, &s.ready) else {
                return Err("no verified update to install".to_string());
            };
            version = ready.version.clone();
            crate::update::set_installing(Some(version.clone()));
            install(
                lang,
                package.path(),
                &ready.asset.sha256,
                &s.env.platform.pkg,
            )
            .map_err(|()| "the install did not start".to_string())
        })
    };
    crate::tray::refresh();
    match result {
        Err(_) => {
            crate::update::set_installing(None);
            if !version.is_empty() {
                notify::notify_update(lang, UpdateKind::Failed, &version);
            }
        }
        Ok(()) if cfg!(unix) => {
            notify::notify_update(lang, UpdateKind::Updated, &version);
            restart_daemon();
        }
        Ok(()) => {}
    }
}

#[cfg(windows)]
fn install(_lang: Lang, path: &std::path::Path, _sha256: &str, _kind: &str) -> Result<(), ()> {
    // Starts the helper copy and asks this daemon to quit over IPC.
    kawaro_core::update::apply::apply_msi(path).map_err(|_| ())
}

#[cfg(unix)]
fn install(lang: Lang, path: &std::path::Path, sha256: &str, kind: &str) -> Result<(), ()> {
    use kawaro_core::update::apply::{
        PkgKind, Plan, SystemRunner, apply_now, linux_plan, privileged_argv_checked, probe_facts,
    };
    let installed_kind = if kind == "deb" {
        PkgKind::Deb
    } else {
        PkgKind::Rpm
    };
    let plan = linux_plan(&probe_facts(Some(installed_kind)));
    // The CLI that runs the privileged step sits next to this daemon.
    let cli = daemon_exe().and_then(|e| e.parent().map(|d| d.join("kawaro")));
    let cli = cli.unwrap_or_else(|| PathBuf::from("/usr/bin/kawaro"));
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
            apply_now(&plan, &cli, path, sha256, &SystemRunner).map_err(|_| ())
        }
        Plan::RepoUpgrade(_) => apply_now(&plan, &cli, path, sha256, &SystemRunner).map_err(|_| ()),
    }
}

/// This program's path, without the ` (deleted)` Linux adds after a package
/// replaced the file.
#[cfg(unix)]
fn daemon_exe() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(crate::update::clean_exe_path(&exe))
}

/// The variable the updater sets on the daemon it starts, so that daemon
/// waits for the old one to free the socket.
const RESTARTED: &str = "KAWARO_RESTARTED";
/// Retries of the restarted daemon, an eighth of `RESTART_DELAY` apart.
const RESTART_TRIES: u32 = 20;

/// A normal start returns `first` at once. A start the updater made retries
/// the bind while the old daemon still owns the socket.
pub fn wait_for_old_daemon<E>(
    first: Result<Bound, E>,
    mut again: impl FnMut() -> Result<Bound, E>,
) -> Result<Bound, E> {
    if std::env::var_os(RESTARTED).is_none() {
        return first;
    }
    let mut bound = first;
    for _ in 0..RESTART_TRIES {
        if !matches!(bound, Ok(Bound::AlreadyRunning)) {
            break;
        }
        std::thread::sleep(crate::update::RESTART_DELAY / 8);
        bound = again();
    }
    bound
}

/// Starts the new daemon directly (no shell), in its own process group with
/// no terminal, and quits. The new one waits for the socket by itself.
#[cfg(unix)]
fn restart_daemon() {
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;
    if let Some(exe) = daemon_exe() {
        let (program, args) = crate::update::restart_command(&exe);
        let _ = std::process::Command::new(program)
            .args(args)
            .env(RESTARTED, "1")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn();
    }
    crate::ui::quit();
}

#[cfg(windows)]
fn restart_daemon() {}
