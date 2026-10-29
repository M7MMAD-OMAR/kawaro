//! The update scheduler of the daemon (S-054b, ADR 0006 sections 5 and 8).
//!
//! `tick`, `check_now` and `request_apply` change an [`UpdateState`] and say
//! what the glue must do next as a list of [`Effect`]s; none of them shows a
//! window or runs an installer, so every rule is testable with a fake
//! fetcher and a counted list of effects. The system side (the real state,
//! notifications, the installer) is in `update_sys.rs`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use kawaro_core::config::Config;
use kawaro_core::ipc::Response;
use kawaro_core::paths::Os;
use kawaro_core::update::apply::{AutoAction, auto_action, can_apply};
use kawaro_core::update::effective_channel;
use kawaro_core::update::fetch::{CurlFetcher, Fetcher};
use kawaro_core::update::flow::{
    CheckEnv, CheckOutcome, CheckState, INTERVAL_SECS, Update, VerifiedPackage, check_with,
    download_verified, part_path,
};
use kawaro_core::update::manifest::Notes;
use kawaro_core::update::policy::Platform;

use crate::notify::UpdateKind;
use crate::progress::{self, Progress};
use crate::state::{Shared, lock};
use crate::update_sys;

/// The first check waits this long after start, so it never competes with
/// the hive or the first jobs.
pub const FIRST_CHECK_DELAY: Duration = Duration::from_secs(30);
/// The thread asks `tick` at least this often; a check itself runs at most
/// once a day, or after the backoff of a failure.
pub const TICK_INTERVAL: Duration = Duration::from_secs(3600);

/// The second of the first tick, counted from the start.
pub fn first_tick_at(start_secs: u64) -> u64 {
    start_secs + FIRST_CHECK_DELAY.as_secs()
}

/// Where the update stands.
#[derive(Clone, Debug, PartialEq)]
pub enum Phase {
    Idle,
    Downloading {
        version: String,
        downloaded: u64,
        size: u64,
    },
    Ready(String),
    /// The install of this version runs; the daemon exits when it works
    /// (S-099 A3).
    Installing(String),
    Failed(String),
}

/// What the running build knows, passed in so a test can set it.
pub struct UpdateEnv {
    pub installed: String,
    pub keys: Vec<String>,
    pub platform: Platform,
    pub os: Os,
    /// `<cache>/updates`; a version lives in `<updates_dir>/<version>`.
    pub updates_dir: PathBuf,
    /// Where the manifest and the signature land while a check runs.
    pub scratch: PathBuf,
}

pub struct UpdateState {
    pub env: UpdateEnv,
    pub check: CheckState,
    pub phase: Phase,
    pub package: Option<VerifiedPackage>,
    /// The update the package belongs to.
    pub ready: Option<Update>,
    /// The user asked to install while a job ran or the hive was open.
    pub apply_requested: bool,
    /// An install of the current package was already started.
    pub applied: bool,
    pub running_jobs: usize,
    pub hive_open: bool,
}

impl UpdateState {
    pub fn new(env: UpdateEnv) -> UpdateState {
        UpdateState {
            env,
            check: CheckState::default(),
            phase: Phase::Idle,
            package: None,
            ready: None,
            apply_requested: false,
            applied: false,
            running_jobs: 0,
            hive_open: false,
        }
    }
}

/// What the glue does after a step.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Notify(UpdateKind, String),
    Apply,
}

/// Runs one check and, for a newer version, the verified download.
fn check_step(
    st: &mut UpdateState,
    config: &Config,
    fetcher: &dyn Fetcher,
    now: u64,
    manual: bool,
) -> (CheckOutcome, Vec<Effect>) {
    let mut followed = config.clone();
    followed.update_channel =
        effective_channel(config.configured_update_channel(), &st.env.installed);
    let keys: Vec<&str> = st.env.keys.iter().map(String::as_str).collect();
    let env = CheckEnv {
        installed: &st.env.installed,
        keys: &keys,
        platform: st.env.platform.clone(),
        interval_secs: INTERVAL_SECS,
        scratch: &st.env.scratch,
    };
    let outcome = check_with(&env, &followed, &mut st.check, fetcher, now, manual);
    let mut effects = Vec::new();
    if let CheckOutcome::Available(update) = &outcome {
        download_step(st, config, update, fetcher, &mut effects);
    }
    (outcome, effects)
}

/// Ends the published progress when the download function returns.
struct Download<'a>(&'a Progress);

impl Drop for Download<'_> {
    fn drop(&mut self) {
        self.0.end();
    }
}

/// Downloads and verifies `update` unless that version is already ready.
fn download_step(
    st: &mut UpdateState,
    config: &Config,
    update: &Update,
    fetcher: &dyn Fetcher,
    effects: &mut Vec<Effect>,
) {
    let version = update.version.clone();
    if st.phase == Phase::Ready(version.clone()) {
        return;
    }
    st.phase = Phase::Downloading {
        version: version.clone(),
        downloaded: 0,
        size: update.asset.size,
    };
    st.package = None;
    st.ready = None;
    let dir = st.env.updates_dir.join(&version);
    // The tray reads the progress without the update lock, from the size of
    // the part file; the guard ends it on every path out of this function.
    let part = part_path(&dir.join(&update.asset.name));
    progress::shared().begin(&version, update.asset.size, &part);
    let _download = Download(progress::shared());
    match download_verified(update, &dir, fetcher) {
        Ok(package) => {
            st.phase = Phase::Ready(version.clone());
            st.package = Some(package);
            st.ready = Some(update.clone());
            st.applied = false;
            if auto_action(config.update_mode, st.env.os) == AutoAction::NotifyReady {
                effects.push(Effect::Notify(UpdateKind::Ready, version));
            }
        }
        Err(_) => {
            st.phase = Phase::Failed(version.clone());
            effects.push(Effect::Notify(UpdateKind::Failed, version));
        }
    }
}

/// True when a ready package waits for a moment to install: the user asked,
/// or the mode installs by itself and has not yet.
pub fn apply_waiting(st: &UpdateState, config: &Config) -> bool {
    matches!(st.phase, Phase::Ready(_))
        && (st.apply_requested
            || (!st.applied
                && auto_action(config.update_mode, st.env.os) == AutoAction::ApplyWhenIdle))
}

/// The whole scheduler step: a check when one is due, then the install if
/// one is waiting and nothing is running.
pub fn tick(st: &mut UpdateState, config: &Config, fetcher: &dyn Fetcher, now: u64) -> Vec<Effect> {
    if matches!(st.phase, Phase::Installing(_)) {
        return Vec::new();
    }
    let (_, mut effects) = check_step(st, config, fetcher, now, false);
    if apply_waiting(st, config) && can_apply(st.running_jobs, st.hive_open) {
        st.apply_requested = false;
        st.applied = true;
        effects.push(Effect::Apply);
    }
    effects
}

/// `UpdateApply`: installs now, or when nothing runs, or refuses.
pub fn request_apply(st: &mut UpdateState) -> (Response, Vec<Effect>) {
    if !matches!(st.phase, Phase::Ready(_)) {
        let message = "no verified update to install".to_string();
        return (Response::Error { message }, Vec::new());
    }
    if can_apply(st.running_jobs, st.hive_open) {
        st.applied = true;
        (Response::Ok, vec![Effect::Apply])
    } else {
        st.apply_requested = true;
        (Response::Ok, Vec::new())
    }
}

/// `UpdateApply` when the update lock may be busy (`None`, a check or a
/// download holds it): there is nothing verified to install yet, so answer
/// at once instead of waiting.
pub fn request_apply_or_busy(st: Option<&mut UpdateState>) -> (Response, Vec<Effect>) {
    match st {
        Some(st) => request_apply(st),
        None => {
            let message = "no verified update to install".to_string();
            (Response::Error { message }, Vec::new())
        }
    }
}

/// `UpdateCheck` when the update lock may be busy: the answer is the status
/// word `downloading` with the running version, and nothing is fetched.
pub fn check_or_busy(
    st: Option<&mut UpdateState>,
    current: &str,
    config: &Config,
    fetcher: &dyn Fetcher,
    now: u64,
) -> (Response, Vec<Effect>) {
    match st {
        Some(st) => check_now(st, config, fetcher, now),
        None => {
            // A download holds the lock; its progress is kept apart.
            let stat = |p: &Path| std::fs::metadata(p).ok().map(|m| m.len());
            let response = match crate::progress::shared().snapshot(&stat) {
                Some(phase) => downloading_reply(current, &phase),
                None if installing_version().is_some() => Response::Update {
                    status: "installing".to_string(),
                    current: current.to_string(),
                    latest: installing_version(),
                    percent: None,
                    size: None,
                    notes: None,
                },
                None => Response::Update {
                    status: "downloading".to_string(),
                    current: current.to_string(),
                    latest: None,
                    percent: None,
                    size: None,
                    notes: None,
                },
            };
            (response, Vec::new())
        }
    }
}

/// The `downloading` answer: the version being fetched, the percent and the
/// size from the signed manifest (S-099a A2).
pub fn downloading_reply(current: &str, snapshot: &Phase) -> Response {
    let (latest, percent, size) = match snapshot {
        Phase::Downloading {
            version,
            downloaded,
            size,
        } => (
            Some(version.clone()),
            Some(crate::progress::percent(*downloaded, *size) as u8),
            Some(*size),
        ),
        _ => (None, None, None),
    };
    Response::Update {
        status: "downloading".to_string(),
        current: current.to_string(),
        latest,
        percent,
        size,
        notes: None,
    }
}

/// `UpdateCheck`: a check the user asked for, which `update_check = false`
/// does not stop.
pub fn check_now(
    st: &mut UpdateState,
    config: &Config,
    fetcher: &dyn Fetcher,
    now: u64,
) -> (Response, Vec<Effect>) {
    if matches!(st.phase, Phase::Installing(_)) {
        return (reply_for(st), Vec::new());
    }
    let (outcome, effects) = check_step(st, config, fetcher, now, true);
    let status = match outcome {
        CheckOutcome::UpToDate => "up-to-date",
        CheckOutcome::Available(_) if matches!(st.phase, Phase::Ready(_)) => "ready",
        CheckOutcome::Available(_) => "failed",
        CheckOutcome::Offline => "offline",
        CheckOutcome::Unavailable => "unavailable",
        _ => "rejected",
    };
    let response = Response::Update {
        status: status.to_string(),
        current: st.env.installed.clone(),
        latest: st.check.last_seen.clone(),
        percent: None,
        size: None,
        notes: ready_notes(st).filter(|_| status == "ready"),
    };
    (response, effects)
}

/// The notes of the verified update, when it has any.
fn ready_notes(st: &UpdateState) -> Option<Notes> {
    st.ready
        .as_ref()
        .map(|u| u.notes.clone())
        .filter(|n| *n != Notes::default())
}

/// The answer for the phase the state is in now (S-099 A3): `downloading`
/// with the percent and size, `ready`, `installing`, `failed`, or
/// `up-to-date` when nothing is going on.
pub fn reply_for(st: &UpdateState) -> Response {
    let (status, latest, notes) = match &st.phase {
        Phase::Downloading { .. } => return downloading_reply(&st.env.installed, &st.phase),
        Phase::Ready(v) => ("ready", Some(v.clone()), ready_notes(st)),
        Phase::Installing(v) => ("installing", Some(v.clone()), None),
        Phase::Failed(v) => ("failed", Some(v.clone()), None),
        Phase::Idle => ("up-to-date", st.check.last_seen.clone(), None),
    };
    Response::Update {
        status: status.to_string(),
        current: st.env.installed.clone(),
        latest,
        percent: None,
        size: None,
        notes,
    }
}

/// Installs the verified update through `install` (S-099 A3). No verified
/// update is an error and `install` never runs. The phase is `Installing`
/// while `install` runs, and `Failed` when it errs; when it works the
/// daemon exits and the phase stays `Installing`.
pub fn apply_with(
    st: &mut UpdateState,
    install: impl FnOnce(&UpdateState) -> Result<(), String>,
) -> Result<(), String> {
    let Phase::Ready(version) = st.phase.clone() else {
        return Err("no verified update to install".to_string());
    };
    st.phase = Phase::Installing(version.clone());
    let result = install(st);
    if result.is_err() {
        st.phase = Phase::Failed(version);
    }
    result
}

/// The version being installed, kept apart from the update lock like the
/// download progress: the install holds the lock, and a poll must still be
/// answered at once. Tests keep it per thread, so they never see each other's.
#[cfg(not(test))]
static INSTALLING: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
#[cfg(test)]
thread_local! {
    static INSTALLING: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

#[cfg(not(test))]
pub fn set_installing(version: Option<String>) {
    *lock(&INSTALLING) = version;
}

#[cfg(test)]
pub fn set_installing(version: Option<String>) {
    INSTALLING.with(|c| *c.borrow_mut() = version);
}

#[cfg(not(test))]
fn installing_version() -> Option<String> {
    lock(&INSTALLING).clone()
}

#[cfg(test)]
fn installing_version() -> Option<String> {
    INSTALLING.with(|c| c.borrow().clone())
}

/// After an install that did not start: the verified package is still on
/// disk, so the phase returns to ready and a second Update now retries
/// without a new check.
pub fn install_failed(st: &mut UpdateState) {
    set_installing(None);
    if let Phase::Failed(v) = &st.phase
        && st.package.is_some()
    {
        st.phase = Phase::Ready(v.clone());
        st.applied = false;
    }
}

/// True when the thread should call `tick` now: an hour passed since the
/// last one, or an install waits for a quiet moment.
pub fn tick_due(last: Option<u64>, now: u64, apply_is_waiting: bool) -> bool {
    apply_is_waiting || last.is_none_or(|t| now < t || now - t >= TICK_INTERVAL.as_secs())
}

/// Writes `text` to a sibling temporary file and renames it over `path`, so
/// a crash never leaves a half written file; the temporary file is removed
/// when anything fails.
pub fn write_atomic(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{}.tmp", std::process::id()));
    let tmp = path.with_file_name(name);
    let done = std::fs::write(&tmp, text).and_then(|()| std::fs::rename(&tmp, path));
    if done.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    done
}

/// How long a restarted daemon waits for the old one to free the socket (it
/// retries about every eighth of it, for ten times this).
pub const RESTART_DELAY: Duration = Duration::from_secs(2);

pub use kawaro_core::update::apply::clean_exe_path;

/// The program that starts the new daemon, and its arguments (none: the new
/// daemon learns it was restarted from the environment, never from a shell).
#[cfg_attr(windows, allow(dead_code))]
pub fn restart_command(exe: &Path) -> (PathBuf, Vec<String>) {
    (clean_exe_path(exe), Vec::new())
}

/// Starts the update thread. Called after the socket is bound; the thread
/// sleeps before it does anything, so startup and the hive are untouched.
pub fn start(state: Shared) {
    std::thread::spawn(move || run(&state));
}

fn run(state: &Shared) {
    let lang = lock(state).lang;
    crate::notify::report_update_result(lang);
    let first = first_tick_at(update_sys::unix_now());
    std::thread::sleep(FIRST_CHECK_DELAY);
    update_sys::load_schedule();
    let mut last = None;
    loop {
        let now = update_sys::unix_now().max(first);
        let (config, jobs, hive_open) = update_sys::snapshot(state);
        let effects = {
            let mut update = lock(update_sys::shared());
            update.running_jobs = jobs;
            update.hive_open = hive_open;
            if tick_due(last, now, apply_waiting(&update, &config)) {
                last = Some(now);
                let effects = tick(&mut update, &config, &CurlFetcher::system(), now);
                update_sys::save_schedule(&update.check.schedule);
                effects
            } else {
                Vec::new()
            }
        };
        crate::tray::refresh();
        update_sys::run_effects(effects, lang);
        std::thread::sleep(kawaro_core::update::apply::RETRY_INTERVAL);
    }
}
