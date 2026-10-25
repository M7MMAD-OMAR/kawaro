//! S-054b (backlog/S-054b-update-daemon-tray.md), the daemon side. Written by
//! the tester from the acceptance criteria and the review log. Nothing here
//! touches the network, `pkexec`, a package manager or the desktop: the fetcher
//! is a fake, the manifest and signature are the throwaway fixtures of S-053a
//! (key D, `s054b/manifest-stable-0.4.0.json`: 17 byte assets), and the notification and apply calls
//! are recorders.
//!
//! Contract (the implementer follows it; the tests are not edited). The module
//! `mod update;` is the implementer's to declare in `main.rs`. DECISION marks a
//! choice the story leaves open.
//!
//! `crate::update` (`crates/daemon/src/update.rs`), all `pub`:
//! - `FIRST_CHECK_DELAY: Duration` = 30 s, `TICK_INTERVAL: Duration` = 1 h,
//!   `first_tick_at(start_secs: u64) -> u64` = `start + 30`.
//! - `Phase` (`Clone + Debug + PartialEq`): `Idle`, `Downloading { version:
//!   String, downloaded: u64, size: u64 }`, `Ready(String)`, `Failed(String)`.
//! - `UpdateEnv { installed: String, keys: Vec<String>, platform:
//!   policy::Platform, os: paths::Os, updates_dir: PathBuf, scratch: PathBuf }`
//!   (plain pub fields). DECISION: the AC's `tick(&mut UpdateState, &Config,
//!   &dyn Fetcher, now)` has no slot for the running version, the trusted keys
//!   or the cache folder, and the real ones (`flow::check`) cannot be faked, so
//!   `UpdateState` carries them. `UpdateState::production()` fills them from the
//!   running build (`CARGO_PKG_VERSION`, `UPDATE_KEYS`, `platform_with`, `CURRENT`,
//!   the cache folder).
//! - `UpdateState { pub env: UpdateEnv, pub check: flow::CheckState, pub phase:
//!   Phase, pub package: Option<flow::VerifiedPackage>, pub apply_requested:
//!   bool, pub running_jobs: usize, pub hive_open: bool }` and
//!   `UpdateState::new(env: UpdateEnv) -> UpdateState` (phase `Idle`, the three
//!   flags false/0). The glue sets `running_jobs` and `hive_open` before a tick.
//! - `Effect` (`Clone + Debug + PartialEq`): `Notify(UpdateKind, String)` and
//!   `Apply`. DECISION: `tick` does not call a notifier or an apply function; it
//!   returns what the glue must do, so a test counts effects instead of
//!   recorders. The glue calls `notify::notify_update` and the apply function.
//! - `tick(&mut UpdateState, &Config, &dyn Fetcher, now: u64) -> Vec<Effect>`: the
//!   whole scheduler step. It runs `flow::check_with` (24 h interval, automatic)
//!   with the state's env. On `Available(u)`: when the phase is already
//!   `Ready(u.version)` nothing is downloaded; otherwise the phase goes
//!   `Downloading`, then `download_verified` into `updates_dir/<version>`: ok gives
//!   `Ready(version)` and keeps the package, an error gives `Failed(version)` (no
//!   file remains, `package` is `None`). The version of an update that is not the
//!   one in `Ready` replaces it. Effects of that step: `Notify(Ready, v)` when
//!   the phase becomes `Ready` and the mode does not apply by itself,
//!   `Notify(Failed, v)` when it becomes `Failed`. Then the apply step: `Apply`
//!   (once, after which `apply_requested` is false and the same package is not
//!   applied again) when the phase is `Ready` and `can_apply(running_jobs,
//!   hive_open)` and either `apply_requested` or `auto_action(update_mode,
//!   env.os) == ApplyWhenIdle`.
//! - `request_apply(&mut UpdateState) -> (Response, Vec<Effect>)` (`UpdateApply`):
//!   phase not `Ready`: `(Response::Error { message }, [])` with `message`
//!   containing `no verified update`, nothing started, `apply_requested` false.
//!   `Ready` and `can_apply`: `(Ok, [Apply])`. `Ready` and busy: `(Ok, [])` and
//!   `apply_requested = true`, so the next tick with `can_apply` applies.
//! - `check_now(&mut UpdateState, &Config, &dyn Fetcher, now) -> (Response,
//!   Vec<Effect>)` (`UpdateCheck`, manual: ignores `update_check = false`). The
//!   response is `Response::Update { status, current: env.installed, latest:
//!   check.last_seen }` with `status` one of `up-to-date`, `ready` (an update is
//!   ready), `failed` (its download failed), `offline`, `unavailable`,
//!   `rejected`. The effects are those of a tick's download step.
//! - `start(...)` and the thread glue are not tested here (source checks below).
//!
//! `crate::notify`:
//! - `UpdateKind { Ready, Updated, Failed }` (`Copy + Clone + Debug + PartialEq +
//!   Eq + Hash`), `trait UpdateSink { fn log(&mut self, line: &str); fn show(&mut
//!   self, summary: &str, body: &str); }`, `Sent` (`Default`): the (kind,
//!   version) pairs already shown.
//! - `notify_update_with(sink: &mut dyn UpdateSink, sent: &mut Sent, lang: Lang,
//!   kind: UpdateKind, version: &str)`: first time for a pair: one `log` line
//!   `kawarod: notify: <message>` and one `show(app_name, <message>)`; a second
//!   call with the same pair does nothing. DECISION: the AC's
//!   `notify_update(lang, kind, version)` is the production wrapper over a global
//!   `Sent` and the real desktop sink; it is only compile checked here, so no
//!   test pops a notification on the live desktop.
//! - `report_update_result_with(sink, sent, lang, dir: &Path)`: `take_result(dir)`
//!   of S-054a; `Installed` gives `Updated(version)`, `Failed(_)` gives
//!   `Failed(version)`, none gives nothing. `report_update_result(lang)` is the
//!   production start-up call.
//!
//! `crate::text`: `update_message(lang, UpdateKind, version) -> String`,
//! `update_now(lang)`, `update_now_hint(lang)`, `menu_labels(lang, &UpdateState)
//! -> Vec<String>`, `update_item_enabled(&UpdateState) -> bool`,
//! `tooltip_text(lang, &UpdateState, active: &[(String, f32)]) -> String`.
//! DECISION: `Vec<String>` cannot carry "enabled", so a second pure function
//! does. The tray shows `menu_labels` and `update_item_enabled`.

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use kawaro_core::config::{Config, UpdateMode};
use kawaro_core::i18n::Lang;
use kawaro_core::ipc::Response;
use kawaro_core::paths::Os;
use kawaro_core::update::fetch::{FetchError, Fetcher};
use kawaro_core::update::flow::CheckState;
use kawaro_core::update::policy::Platform;
use kawaro_core::update::result::{
    Failure, Outcome, RESULT_FILE, UpdateResult, take_result, write_result,
};

use crate::notify::{
    Sent, UpdateKind, UpdateSink, notify_update, notify_update_with, report_update_result,
    report_update_result_with,
};
use crate::text;
use crate::update::{
    Effect, FIRST_CHECK_DELAY, Phase, TICK_INTERVAL, UpdateEnv, UpdateState, check_now,
    first_tick_at, request_apply, tick,
};

const HOUR: u64 = 3600;
const BASE: u64 = 1_800_000_000;
const REL: &str = "https://github.com/M7MMAD-OMAR/kawaro/releases/download/v0.4.0";
const PKG_NAME: &str = "kawaro-0.4.0-1.x86_64.rpm";
const GOOD: &[u8] = b"first-asset-bytes";
const TAMPERED: &[u8] = b"first-asset-bytez";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/update")
}

fn fx(rel: &str) -> Vec<u8> {
    let p = fixtures().join(rel);
    fs::read(&p).unwrap_or_else(|e| panic!("fixture {} must exist: {e}", p.display()))
}

#[derive(Clone, Copy)]
enum Feed {
    /// Manifest, signature and the right package bytes.
    Good,
    /// Manifest and signature, package bytes that do not match the signed hash.
    Tampered,
    /// Every request fails as offline.
    Offline,
}

struct Fake {
    feed: Feed,
    urls: RefCell<Vec<String>>,
}

impl Fake {
    fn new(feed: Feed) -> Fake {
        Fake {
            feed,
            urls: RefCell::new(Vec::new()),
        }
    }
    fn calls(&self) -> usize {
        self.urls.borrow().len()
    }
}

impl Fetcher for Fake {
    fn get(&self, url: &str, dest: &Path, _max: u64) -> Result<(), FetchError> {
        self.urls.borrow_mut().push(url.to_string());
        let bytes = match self.feed {
            Feed::Offline => return Err(FetchError::Offline),
            _ if url.ends_with(".minisig") => fx("s054b/manifest-stable-0.4.0.json.minisig"),
            _ if url.ends_with(".json") => fx("s054b/manifest-stable-0.4.0.json"),
            Feed::Good => GOOD.to_vec(),
            Feed::Tampered => TAMPERED.to_vec(),
        };
        fs::write(dest, bytes).unwrap();
        Ok(())
    }
}

struct Rig {
    _dir: tempfile::TempDir,
    updates: PathBuf,
}

fn state_for(installed: &str, os: Os) -> (UpdateState, Rig) {
    let dir = tempfile::tempdir().unwrap();
    let updates = dir.path().join("updates");
    let scratch = dir.path().join("scratch");
    fs::create_dir_all(&scratch).unwrap();
    let env = UpdateEnv {
        installed: installed.to_string(),
        keys: vec![String::from_utf8(fx("s054b/key-d.pub")).unwrap()],
        platform: Platform {
            os: "linux".into(),
            arch: "x86_64".into(),
            pkg: "rpm".into(),
        },
        os,
        updates_dir: updates.clone(),
        scratch,
    };
    (UpdateState::new(env), Rig { _dir: dir, updates })
}

fn config(mode: UpdateMode) -> Config {
    Config {
        update_mode: mode,
        ..Config::default()
    }
}

fn ready_state(os: Os, mode: UpdateMode) -> (UpdateState, Rig, Config) {
    let (mut st, rig) = state_for("0.3.3", os);
    let cfg = config(UpdateMode::Ask);
    let effects = tick(&mut st, &cfg, &Fake::new(Feed::Good), BASE);
    assert_eq!(st.phase, Phase::Ready("0.4.0".into()), "{effects:?}");
    (st, rig, config(mode))
}

fn listing(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir)
        .map(|rd| {
            rd.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

// ------------------------------------------------------------ AC2

/// Ticks every hour for 48 hours; returns the hours in which the fetcher was called.
fn hours_with_calls(st: &mut UpdateState, cfg: &Config, fake: &Fake) -> Vec<u64> {
    let mut hours = Vec::new();
    for h in 0..48u64 {
        let before = fake.calls();
        tick(st, cfg, fake, BASE + h * HOUR);
        if fake.calls() > before {
            hours.push(h);
        }
    }
    hours
}

#[test]
fn ac2_no_update_checks_at_hour_0_and_24_with_four_fetch_calls() {
    let (mut st, _rig) = state_for("0.4.0", Os::Linux);
    let fake = Fake::new(Feed::Good);
    let hours = hours_with_calls(&mut st, &Config::default(), &fake);
    assert_eq!(hours, vec![0, 24]);
    assert_eq!(fake.calls(), 4);
    assert_eq!(st.phase, Phase::Idle);
    assert_eq!(st.check.last_seen.as_deref(), Some("0.4.0"));
    assert_eq!(st.check.schedule.last_check, Some(BASE + 24 * HOUR));
    let urls = fake.urls.borrow();
    assert!(urls[0].ends_with("/updates/stable.json"), "{urls:?}");
    assert!(
        urls[1].ends_with("/updates/stable.json.minisig"),
        "{urls:?}"
    );
}

#[test]
fn ac2_with_update_check_off_there_are_no_calls() {
    let (mut st, _rig) = state_for("0.3.3", Os::Linux);
    let fake = Fake::new(Feed::Good);
    let cfg = Config {
        update_check: false,
        ..Config::default()
    };
    let hours = hours_with_calls(&mut st, &cfg, &fake);
    assert_eq!(hours, Vec::<u64>::new());
    assert_eq!(fake.calls(), 0);
    assert_eq!(st.phase, Phase::Idle);
    assert_eq!(st.check, CheckState::default());
}

#[test]
fn ac2_offline_follows_the_1_2_4_hour_backoff_not_every_tick() {
    let (mut st, _rig) = state_for("0.3.3", Os::Linux);
    let fake = Fake::new(Feed::Offline);
    let hours = hours_with_calls(&mut st, &Config::default(), &fake);
    assert_eq!(hours, vec![0, 1, 3, 7, 15, 31]);
    assert_eq!(fake.calls(), 6);
    assert_eq!(st.check.schedule.failures, 6);
    assert_eq!(st.phase, Phase::Idle);
}

#[test]
fn ac2_tick_has_the_signature_of_the_story_and_stays_silent_when_disabled() {
    let f: fn(&mut UpdateState, &Config, &dyn Fetcher, u64) -> Vec<Effect> = tick;
    let (mut st, _rig) = state_for("0.3.3", Os::Linux);
    let fake = Fake::new(Feed::Good);
    let cfg = Config {
        update_check: false,
        ..Config::default()
    };
    assert_eq!(f(&mut st, &cfg, &fake, BASE), Vec::<Effect>::new());
    assert_eq!(fake.calls(), 0);
}

// ------------------------------------------------------------ AC3

#[test]
fn ac3_the_first_check_waits_30_seconds_after_start() {
    assert_eq!(FIRST_CHECK_DELAY, Duration::from_secs(30));
    assert_eq!(TICK_INTERVAL, Duration::from_secs(3600));
    assert_eq!(first_tick_at(1_000), 1_030);
    assert_eq!(first_tick_at(0), 30);
}

// ------------------------------------------------------------ AC4

#[test]
fn ac4_a_good_package_makes_the_state_ready_and_keeps_the_file() {
    let (mut st, rig) = state_for("0.3.3", Os::Linux);
    let fake = Fake::new(Feed::Good);
    let effects = tick(&mut st, &Config::default(), &fake, BASE);
    assert_eq!(st.phase, Phase::Ready("0.4.0".into()));
    assert_eq!(
        effects,
        vec![Effect::Notify(UpdateKind::Ready, "0.4.0".into())]
    );
    assert_eq!(
        fs::read(rig.updates.join("0.4.0").join(PKG_NAME)).unwrap(),
        GOOD
    );
    assert!(st.package.is_some());
    assert_eq!(fake.calls(), 3);
    let urls = fake.urls.borrow();
    assert_eq!(urls[2], format!("{REL}/{PKG_NAME}"));
}

#[test]
fn ac4_a_tampered_package_fails_and_no_file_remains() {
    let (mut st, rig) = state_for("0.3.3", Os::Linux);
    let effects = tick(
        &mut st,
        &Config::default(),
        &Fake::new(Feed::Tampered),
        BASE,
    );
    assert_eq!(st.phase, Phase::Failed("0.4.0".into()));
    assert_eq!(
        effects,
        vec![Effect::Notify(UpdateKind::Failed, "0.4.0".into())]
    );
    assert!(st.package.is_none());
    assert_eq!(listing(&rig.updates.join("0.4.0")), Vec::<String>::new());
}

#[test]
fn ac4_a_ready_version_is_not_downloaded_again_at_the_next_check() {
    let (mut st, _rig) = state_for("0.3.3", Os::Linux);
    let fake = Fake::new(Feed::Good);
    let hours = hours_with_calls(&mut st, &Config::default(), &fake);
    assert_eq!(hours, vec![0, 24]);
    // 2 manifest fetches + 1 package at hour 0, 2 manifest fetches at hour 24.
    assert_eq!(fake.calls(), 5);
    assert_eq!(st.phase, Phase::Ready("0.4.0".into()));
}

#[test]
fn ac4_ask_mode_never_applies_without_a_request_on_linux_and_windows() {
    for os in [Os::Linux, Os::Windows] {
        let (mut st, _rig) = state_for("0.3.3", os);
        let fake = Fake::new(Feed::Good);
        let cfg = config(UpdateMode::Ask);
        let mut applies = 0;
        for h in 0..48 {
            let effects = tick(&mut st, &cfg, &fake, BASE + h * HOUR);
            applies += effects.iter().filter(|e| **e == Effect::Apply).count();
        }
        assert_eq!(applies, 0, "{os:?}");
        assert_eq!(st.phase, Phase::Ready("0.4.0".into()));
    }
}

#[test]
fn ac4_auto_mode_on_windows_applies_once_when_idle() {
    let (mut st, _rig) = state_for("0.3.3", Os::Windows);
    let fake = Fake::new(Feed::Good);
    let cfg = config(UpdateMode::Auto);
    let first = tick(&mut st, &cfg, &fake, BASE);
    assert_eq!(first, vec![Effect::Apply]);
    for h in 1..48 {
        let effects = tick(&mut st, &cfg, &fake, BASE + h * HOUR);
        assert!(!effects.contains(&Effect::Apply), "hour {h}: {effects:?}");
    }
}

#[test]
fn ac4_auto_mode_on_windows_waits_while_a_job_runs_or_the_hive_is_open() {
    let (mut st, _rig) = state_for("0.3.3", Os::Windows);
    let fake = Fake::new(Feed::Good);
    let cfg = config(UpdateMode::Auto);
    st.running_jobs = 1;
    assert_eq!(tick(&mut st, &cfg, &fake, BASE), Vec::<Effect>::new());
    st.running_jobs = 0;
    st.hive_open = true;
    assert_eq!(tick(&mut st, &cfg, &fake, BASE + 30), Vec::<Effect>::new());
    st.hive_open = false;
    assert_eq!(tick(&mut st, &cfg, &fake, BASE + 60), vec![Effect::Apply]);
    assert_eq!(tick(&mut st, &cfg, &fake, BASE + 90), Vec::<Effect>::new());
}

#[test]
fn ac4_auto_mode_on_linux_only_notifies_never_applies() {
    let (mut st, _rig) = state_for("0.3.3", Os::Linux);
    let cfg = config(UpdateMode::Auto);
    let effects = tick(&mut st, &cfg, &Fake::new(Feed::Good), BASE);
    assert_eq!(
        effects,
        vec![Effect::Notify(UpdateKind::Ready, "0.4.0".into())]
    );
}

// ------------------------------------------------------------ AC5

fn ar_en() -> [Lang; 2] {
    [Lang::Ar, Lang::En]
}

#[test]
fn ac5_menu_labels_in_both_languages_with_update_now_first() {
    let (st, _rig) = state_for("0.3.3", Os::Linux);
    assert_eq!(
        text::menu_labels(Lang::En, &st),
        vec!["Update now", "Cancel all jobs", "Settings", "Quit Kawaro"]
    );
    assert_eq!(
        text::menu_labels(Lang::Ar, &st),
        vec!["حدّث الآن", "إلغاء كل المهام", "الإعدادات", "إنهاء كوارو"]
    );
    assert_eq!(text::update_now(Lang::En), "Update now");
    assert_eq!(text::update_now(Lang::Ar), "حدّث الآن");
}

#[test]
fn ac5_update_now_is_enabled_only_when_ready() {
    let (mut st, _rig) = state_for("0.3.3", Os::Linux);
    let cases = [
        (Phase::Idle, false),
        (
            Phase::Downloading {
                version: "0.4.0".into(),
                downloaded: 5,
                size: 10,
            },
            false,
        ),
        (Phase::Ready("0.4.0".into()), true),
        (Phase::Failed("0.4.0".into()), false),
    ];
    for (phase, enabled) in cases {
        st.phase = phase.clone();
        assert_eq!(text::update_item_enabled(&st), enabled, "{phase:?}");
        // The label list is the same in every state.
        assert_eq!(text::menu_labels(Lang::En, &st)[0], "Update now");
    }
}

#[test]
fn ac5_the_menu_entry_has_a_descriptive_label_in_both_languages() {
    for lang in ar_en() {
        let hint = text::update_now_hint(lang);
        assert!(!hint.trim().is_empty());
        assert_ne!(hint, text::update_now(lang));
        assert!(!hint.contains('\u{2014}') && !hint.contains('\u{2013}'));
    }
    assert!(
        text::update_now_hint(Lang::Ar)
            .chars()
            .any(|c| ('\u{0621}'..='\u{064A}').contains(&c))
    );
    assert!(
        !text::update_now_hint(Lang::En)
            .chars()
            .any(|c| ('\u{0600}'..='\u{06FF}').contains(&c))
    );
}

fn downloading(downloaded: u64, size: u64) -> Phase {
    Phase::Downloading {
        version: "0.4.0".into(),
        downloaded,
        size,
    }
}

#[test]
fn ac5_tooltip_shows_the_rounded_percent_while_downloading() {
    let (mut st, _rig) = state_for("0.3.3", Os::Linux);
    let en = |st: &UpdateState| text::tooltip_text(Lang::En, st, &[]);
    let ar = |st: &UpdateState| text::tooltip_text(Lang::Ar, st, &[]);
    for (done, size, pct) in [
        (0, 1000, 0),
        (420, 1000, 42),
        (421, 1000, 42),
        (429, 1000, 43),
        (1000, 1000, 100),
        (6, 1000, 1),
        (4, 1000, 0),
    ] {
        st.phase = downloading(done, size);
        assert_eq!(
            en(&st),
            format!("Downloading update {pct}%"),
            "{done}/{size}"
        );
        assert_eq!(ar(&st), format!("تنزيل التحديث {pct}٪"), "{done}/{size}");
    }
}

#[test]
fn ac5_tooltip_survives_a_zero_size_and_an_overshoot() {
    let (mut st, _rig) = state_for("0.3.3", Os::Linux);
    st.phase = downloading(10, 0);
    assert_eq!(
        text::tooltip_text(Lang::En, &st, &[]),
        "Downloading update 0%"
    );
    st.phase = downloading(2000, 1000);
    assert_eq!(
        text::tooltip_text(Lang::En, &st, &[]),
        "Downloading update 100%"
    );
}

#[test]
fn ac5_tooltip_is_the_idle_text_in_every_other_state() {
    let (mut st, _rig) = state_for("0.3.3", Os::Linux);
    for phase in [
        Phase::Idle,
        Phase::Ready("0.4.0".into()),
        Phase::Failed("0.4.0".into()),
    ] {
        st.phase = phase;
        for lang in ar_en() {
            assert_eq!(text::tooltip_text(lang, &st, &[]), text::idle(lang));
        }
    }
}

#[test]
fn ac5_tooltip_keeps_the_job_lines_when_no_update_is_downloading() {
    let (st, _rig) = state_for("0.3.3", Os::Linux);
    let active = vec![("PNG to WEBP".to_string(), 0.5), ("PDF".to_string(), 0.07)];
    assert_eq!(
        text::tooltip_text(Lang::En, &st, &active),
        "PNG to WEBP: 50%\nPDF: 7%"
    );
}

// ------------------------------------------------------------ AC6

#[derive(Default)]
struct Rec {
    lines: Vec<String>,
    shown: Vec<(String, String)>,
}

impl UpdateSink for Rec {
    fn log(&mut self, line: &str) {
        self.lines.push(line.to_string());
    }
    fn show(&mut self, summary: &str, body: &str) {
        self.shown.push((summary.to_string(), body.to_string()));
    }
}

#[test]
fn ac6_messages_in_both_languages() {
    assert_eq!(
        text::update_message(Lang::En, UpdateKind::Ready, "0.4.0"),
        "Update 0.4.0 is ready. Choose Update now in the tray menu."
    );
    assert_eq!(
        text::update_message(Lang::En, UpdateKind::Updated, "0.4.0"),
        "Kawaro was updated to 0.4.0."
    );
    assert_eq!(
        text::update_message(Lang::En, UpdateKind::Failed, "0.4.0"),
        "Update 0.4.0 failed. The current version is unchanged."
    );
    assert_eq!(
        text::update_message(Lang::Ar, UpdateKind::Ready, "0.4.0"),
        "التحديث 0.4.0 جاهز. اختر «حدّث الآن» من قائمة الدرج."
    );
    assert_eq!(
        text::update_message(Lang::Ar, UpdateKind::Updated, "0.4.0"),
        "تم تحديث كوارو إلى 0.4.0."
    );
    assert_eq!(
        text::update_message(Lang::Ar, UpdateKind::Failed, "0.4.0"),
        "تعذّر التحديث 0.4.0. النسخة الحالية لم تتغير."
    );
}

#[test]
fn ac6_one_line_and_one_notification_per_kind_and_version() {
    let (mut sink, mut sent) = (Rec::default(), Sent::default());
    notify_update_with(&mut sink, &mut sent, Lang::En, UpdateKind::Ready, "0.4.0");
    let msg = text::update_message(Lang::En, UpdateKind::Ready, "0.4.0");
    assert_eq!(sink.lines, vec![format!("kawarod: notify: {msg}")]);
    assert_eq!(sink.shown, vec![("Kawaro".to_string(), msg)]);

    notify_update_with(&mut sink, &mut sent, Lang::En, UpdateKind::Ready, "0.4.0");
    assert_eq!(
        sink.lines.len(),
        1,
        "the same kind and version does nothing"
    );
    assert_eq!(sink.shown.len(), 1);

    notify_update_with(&mut sink, &mut sent, Lang::Ar, UpdateKind::Ready, "0.4.0");
    assert_eq!(
        sink.lines.len(),
        1,
        "another language is still the same pair"
    );
}

#[test]
fn ac6_a_different_version_or_kind_sends_again() {
    let (mut sink, mut sent) = (Rec::default(), Sent::default());
    notify_update_with(&mut sink, &mut sent, Lang::En, UpdateKind::Ready, "0.4.0");
    notify_update_with(&mut sink, &mut sent, Lang::En, UpdateKind::Ready, "0.4.1");
    notify_update_with(&mut sink, &mut sent, Lang::En, UpdateKind::Failed, "0.4.0");
    notify_update_with(&mut sink, &mut sent, Lang::En, UpdateKind::Updated, "0.4.0");
    assert_eq!(sink.lines.len(), 4);
    assert_eq!(sink.shown.len(), 4);
    assert!(
        sink.lines
            .iter()
            .all(|l| l.starts_with("kawarod: notify: "))
    );
    notify_update_with(&mut sink, &mut sent, Lang::En, UpdateKind::Failed, "0.4.0");
    assert_eq!(sink.lines.len(), 4);
}

#[test]
fn ac6_the_arabic_notification_uses_the_arabic_app_name() {
    let (mut sink, mut sent) = (Rec::default(), Sent::default());
    notify_update_with(&mut sink, &mut sent, Lang::Ar, UpdateKind::Updated, "0.4.0");
    assert_eq!(sink.shown[0].0, "كوارو");
    assert_eq!(sink.lines[0], "kawarod: notify: تم تحديث كوارو إلى 0.4.0.");
}

fn result(version: &str, outcome: Outcome) -> UpdateResult {
    UpdateResult {
        version: version.to_string(),
        outcome,
        at: 7,
    }
}

#[test]
fn ac6_the_start_up_result_is_reported_once_and_the_file_is_removed() {
    let dir = tempfile::tempdir().unwrap();
    write_result(dir.path(), &result("0.4.0", Outcome::Installed)).unwrap();
    let (mut sink, mut sent) = (Rec::default(), Sent::default());
    report_update_result_with(&mut sink, &mut sent, Lang::En, dir.path());
    assert_eq!(
        sink.lines,
        vec![format!(
            "kawarod: notify: {}",
            text::update_message(Lang::En, UpdateKind::Updated, "0.4.0")
        )]
    );
    assert_eq!(sink.shown.len(), 1);
    assert!(!dir.path().join(RESULT_FILE).exists());
    assert_eq!(take_result(dir.path()), None);
    report_update_result_with(&mut sink, &mut sent, Lang::En, dir.path());
    assert_eq!(
        sink.lines.len(),
        1,
        "the second start-up call sends nothing"
    );
}

#[test]
fn ac6_a_failed_install_result_is_reported_as_failed() {
    let dir = tempfile::tempdir().unwrap();
    let failure = Outcome::Failed(Failure::Text("msiexec could not start".into()));
    write_result(dir.path(), &result("0.4.1", failure)).unwrap();
    let (mut sink, mut sent) = (Rec::default(), Sent::default());
    report_update_result_with(&mut sink, &mut sent, Lang::Ar, dir.path());
    assert_eq!(
        sink.lines,
        vec![format!(
            "kawarod: notify: {}",
            text::update_message(Lang::Ar, UpdateKind::Failed, "0.4.1")
        )]
    );
}

#[test]
fn ac6_no_result_file_sends_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (mut sink, mut sent) = (Rec::default(), Sent::default());
    report_update_result_with(&mut sink, &mut sent, Lang::En, dir.path());
    assert!(sink.lines.is_empty() && sink.shown.is_empty());
}

#[test]
fn ac6_the_production_entry_points_have_the_story_signatures() {
    let _: fn(Lang, UpdateKind, &str) = notify_update;
    let _: fn(Lang) = report_update_result;
}

// ------------------------------------------------------------ AC7

#[test]
fn ac7_apply_without_a_verified_update_is_an_error_and_starts_nothing() {
    let (mut st, _rig) = state_for("0.3.3", Os::Linux);
    for phase in [
        Phase::Idle,
        Phase::Failed("0.4.0".into()),
        downloading(1, 2),
    ] {
        st.phase = phase.clone();
        let (resp, effects) = request_apply(&mut st);
        match resp {
            Response::Error { message } => {
                assert!(message.contains("no verified update"), "{message}")
            }
            other => panic!("{phase:?}: {other:?}"),
        }
        assert_eq!(effects, Vec::<Effect>::new(), "{phase:?}");
        assert!(!st.apply_requested, "{phase:?}");
    }
}

#[test]
fn ac7_apply_when_ready_and_idle_answers_ok_and_applies_now() {
    let (mut st, _rig, _cfg) = ready_state(Os::Linux, UpdateMode::Ask);
    let (resp, effects) = request_apply(&mut st);
    assert_eq!(resp, Response::Ok);
    assert_eq!(effects, vec![Effect::Apply]);
    assert!(!st.apply_requested);
}

#[test]
fn ac7_apply_with_a_running_job_defers_to_the_first_tick_with_can_apply() {
    let (mut st, _rig, cfg) = ready_state(Os::Linux, UpdateMode::Ask);
    let fake = Fake::new(Feed::Good);
    st.running_jobs = 1;
    let (resp, effects) = request_apply(&mut st);
    assert_eq!(resp, Response::Ok);
    assert_eq!(effects, Vec::<Effect>::new());
    assert!(st.apply_requested);

    assert_eq!(tick(&mut st, &cfg, &fake, BASE + 30), Vec::<Effect>::new());
    assert!(st.apply_requested, "still waiting while the job runs");
    st.running_jobs = 0;
    st.hive_open = true;
    assert_eq!(tick(&mut st, &cfg, &fake, BASE + 60), Vec::<Effect>::new());
    st.hive_open = false;
    assert_eq!(tick(&mut st, &cfg, &fake, BASE + 90), vec![Effect::Apply]);
    assert!(!st.apply_requested);
    assert_eq!(tick(&mut st, &cfg, &fake, BASE + 120), Vec::<Effect>::new());
    assert_eq!(fake.calls(), 0, "no check was due in between");
}

// ------------------------------------------------------------ UpdateCheck

fn status_of(resp: &Response) -> (String, String, Option<String>) {
    match resp {
        Response::Update {
            status,
            current,
            latest,
        } => (status.clone(), current.clone(), latest.clone()),
        other => panic!("{other:?}"),
    }
}

#[test]
fn manual_check_up_to_date_offline_and_ready() {
    let off = Config {
        update_check: false,
        ..Config::default()
    };

    let (mut st, _rig) = state_for("0.4.0", Os::Linux);
    let fake = Fake::new(Feed::Good);
    let (resp, effects) = check_now(&mut st, &off, &fake, BASE);
    assert_eq!(
        status_of(&resp),
        ("up-to-date".into(), "0.4.0".into(), Some("0.4.0".into()))
    );
    assert_eq!(effects, Vec::<Effect>::new());
    assert_eq!(
        fake.calls(),
        2,
        "a manual check ignores update_check = false"
    );

    let (mut st, _rig) = state_for("0.3.3", Os::Linux);
    let (resp, effects) = check_now(&mut st, &off, &Fake::new(Feed::Offline), BASE);
    assert_eq!(status_of(&resp), ("offline".into(), "0.3.3".into(), None));
    assert_eq!(effects, Vec::<Effect>::new());

    let (mut st, _rig) = state_for("0.3.3", Os::Linux);
    let (resp, effects) = check_now(&mut st, &off, &Fake::new(Feed::Good), BASE);
    assert_eq!(
        status_of(&resp),
        ("ready".into(), "0.3.3".into(), Some("0.4.0".into()))
    );
    assert_eq!(
        effects,
        vec![Effect::Notify(UpdateKind::Ready, "0.4.0".into())]
    );
    assert_eq!(st.phase, Phase::Ready("0.4.0".into()));

    let (mut st, _rig) = state_for("0.3.3", Os::Linux);
    let (resp, _) = check_now(&mut st, &off, &Fake::new(Feed::Tampered), BASE);
    assert_eq!(
        status_of(&resp),
        ("failed".into(), "0.3.3".into(), Some("0.4.0".into()))
    );
}

// ------------------------------------------------------------ sources

fn code(file: &str) -> String {
    let path = format!("{}/src/{file}", env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{path}: {e}"))
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn ac3_the_update_thread_starts_after_the_socket_is_bound_and_main_never_checks() {
    let main = code("main.rs");
    let bind = main.find("Listener::bind(").expect("main binds the socket");
    let start = main
        .find("update::start(")
        .expect("main starts the update thread through update::start");
    assert!(bind < start, "the update thread starts after the bind");
    assert!(!main.contains("check_with("));
    assert!(!main.contains("flow::check"));
    assert!(!main.contains("update::tick("));

    let update = code("update.rs");
    assert!(update.contains("FIRST_CHECK_DELAY"));
    assert!(update.contains("thread::spawn"));
    assert!(update.contains("thread::sleep"));
    let sleep = update.find("thread::sleep").unwrap();
    let first_tick = update.find("tick(").expect("the thread ticks");
    assert!(
        update.find("FIRST_CHECK_DELAY").unwrap() <= sleep,
        "the delay is the first sleep"
    );
    assert!(first_tick > 0);
}

#[test]
fn ac6_update_notifications_never_open_a_window() {
    let notify = code("notify.rs");
    assert!(notify.contains("notify_update_with"));
    assert!(notify.contains("log_line("));
    let update = code("update.rs");
    for word in [
        "slint",
        "ui::present",
        "ui::open_settings",
        "set_foreground",
    ] {
        assert!(!update.contains(word), "update.rs must not use {word}");
    }
}

#[test]
fn ac7_the_server_answers_the_two_update_requests() {
    let server = code("server.rs");
    assert!(server.contains("Request::UpdateCheck"));
    assert!(server.contains("Request::UpdateApply"));
    assert!(server.contains("request_apply_or_busy("));
    assert!(server.contains("check_or_busy("));
    assert!(server.contains("try_lock"));
}

#[test]
fn ac5_the_linux_tray_uses_the_update_menu_and_tooltip_functions() {
    let tray = code("tray/linux.rs");
    assert!(tray.contains("menu_labels("));
    assert!(tray.contains("update_item_enabled("));
    assert!(tray.contains("tooltip_from("));
    assert!(tray.contains("try_lock"));
}

// ------------------------------------------------------------ round 3: mutants

#[test]
fn update_manual_texts_are_exact_in_both_languages() {
    assert_eq!(
        text::update_manual(Lang::En, "sudo dnf upgrade -y kawaro"),
        "To install the update, run: sudo dnf upgrade -y kawaro"
    );
    assert_eq!(
        text::update_manual(Lang::Ar, "sudo dnf upgrade -y kawaro"),
        "لتثبيت التحديث نفّذ: sudo dnf upgrade -y kawaro"
    );
    assert_eq!(
        text::update_manual(Lang::En, "https://example.org/releases"),
        "To install the update, run: https://example.org/releases"
    );
    assert_eq!(text::update_manual(Lang::Ar, ""), "لتثبيت التحديث نفّذ: ");
}

#[test]
fn tick_due_pins_every_boundary() {
    use crate::update::tick_due;
    let h = 3600;
    // Never ticked: due, whatever the clock says.
    assert!(tick_due(None, 0, false));
    assert!(tick_due(None, BASE, false));
    // The hour: one second before is not due, exactly at and after is.
    assert!(!tick_due(Some(BASE), BASE, false));
    assert!(!tick_due(Some(BASE), BASE + 1, false));
    assert!(!tick_due(Some(BASE), BASE + h - 1, false));
    assert!(tick_due(Some(BASE), BASE + h, false));
    assert!(tick_due(Some(BASE), BASE + h + 1, false));
    assert!(tick_due(Some(BASE), BASE + 24 * h, false));
    // A clock that went backwards is due, by one second or by a lot.
    assert!(tick_due(Some(BASE), BASE - 1, false));
    assert!(tick_due(Some(BASE), 0, false));
    // A waiting install is due at any time, even right after a tick.
    assert!(tick_due(Some(BASE), BASE, true));
    assert!(tick_due(Some(BASE), BASE + 1, true));
    assert!(tick_due(None, BASE, true));
    assert!(tick_due(Some(BASE), BASE + h, true));
    // Small numbers: the subtraction must not be confused with other operators.
    assert!(!tick_due(Some(10), 20, false));
    assert!(!tick_due(Some(1), 3600, false));
    assert!(tick_due(Some(1), 3601, false));
    assert_eq!(TICK_INTERVAL.as_secs(), h);
}

struct Unavailable(RefCell<usize>);

impl Fetcher for Unavailable {
    fn get(&self, _url: &str, _dest: &Path, _max: u64) -> Result<(), FetchError> {
        *self.0.borrow_mut() += 1;
        Err(FetchError::Unavailable)
    }
}

#[test]
fn manual_check_without_curl_answers_unavailable() {
    let (mut st, _rig) = state_for("0.3.3", Os::Linux);
    let fake = Unavailable(RefCell::new(0));
    let (resp, effects) = check_now(&mut st, &Config::default(), &fake, BASE);
    assert_eq!(
        status_of(&resp),
        ("unavailable".into(), "0.3.3".into(), None)
    );
    assert_eq!(effects, Vec::<Effect>::new());
    assert_eq!(*fake.0.borrow(), 1);
    assert_eq!(st.phase, Phase::Idle);
    assert_eq!(st.check.schedule.failures, 1);
    assert_eq!(st.check.schedule.last_failure, Some(BASE));
    assert_eq!(st.check.last_seen, None);
}

// ------------------------------------------------------------ REWORK 1
//
// Contract for the implementer (S-054b REWORK #1). Everything is pure or
// plain IO and runs on Linux.
//
// - `notify::notifications_allowed(is_test: bool, no_notify_set: bool) -> bool`:
//   true only for `(false, false)`; `show` and `finished` use it.
// - New module `crate::progress` (`mod progress;` in main.rs), a holder that is
//   separate from the update lock:
//   `percent(done: u64, size: u64) -> u64` (rounded to nearest, 0 for size 0,
//   never above 100); `Progress` (`Default`, interior `Mutex`, all methods take
//   `&self`): `new()`, `begin(&self, version: &str, size: u64, part: &Path)`,
//   `end(&self)`, `snapshot(&self, stat: &dyn Fn(&Path) -> Option<u64>) ->
//   Option<Phase>`: `None` when no download is active, otherwise
//   `Some(Phase::Downloading { version, downloaded: stat(part).unwrap_or(0),
//   size })`. `progress::shared() -> &'static Progress` is the daemon's one.
// - `text::tooltip_from(lang, update: Option<&UpdateState>, progress:
//   Option<&Phase>, active: &[(String, f32)]) -> String`: a `Some(Downloading)`
//   progress wins and shows `tooltip_text`'s download line; otherwise
//   `tooltip_text(lang, update, active)` when the update lock was got; otherwise
//   (lock busy, no progress) the idle text, or the job lines when jobs run.
// - `update::request_apply_or_busy(st: Option<&mut UpdateState>) -> (Response,
//   Vec<Effect>)`: `None` (the lock is busy) answers `Response::Error` whose
//   message contains `no verified update to install`, no effects; `Some` is
//   `request_apply`.
// - `update::check_or_busy(st: Option<&mut UpdateState>, current: &str, config:
//   &Config, fetcher: &dyn Fetcher, now: u64) -> (Response, Vec<Effect>)`: `None`
//   answers `Response::Update { status: "downloading", current, latest: None }`,
//   no effects, the fetcher is not called; `Some` is `check_now`.
//   The socket server and `apply_from_menu` use `try_lock().ok()` with these.
// - `update::write_atomic(path: &Path, text: &str) -> io::Result<()>`: creates
//   the parent, writes a sibling temp file whose name ends in `.tmp`, renames it
//   over `path`, removes the temp file when anything fails. `save_schedule`
//   uses it.
// - `update::RESTART_DELAY: Duration` = 2 s; `update::clean_exe_path(&Path) ->
//   PathBuf` strips a trailing ` (deleted)`; `update::restart_command(exe:
//   &Path) -> (PathBuf, Vec<String>)` = (the cleaned exe, no arguments): no
//   shell. `restart_daemon` spawns that directly.

mod rework1 {
    use super::*;
    use crate::notify::notifications_allowed;
    use crate::progress::{Progress, percent};
    use crate::update::{
        RESTART_DELAY, check_or_busy, clean_exe_path, request_apply_or_busy, restart_command,
        write_atomic,
    };
    use std::sync::Mutex;

    #[test]
    fn notifications_are_allowed_only_outside_tests_and_without_the_switch() {
        assert!(notifications_allowed(false, false));
        assert!(!notifications_allowed(true, false));
        assert!(!notifications_allowed(false, true));
        assert!(!notifications_allowed(true, true));
    }

    // ---------------------------------------------------- progress

    #[test]
    fn percent_is_rounded_and_bounded() {
        for (done, size, want) in [
            (0, 1000, 0),
            (4, 1000, 0),
            (6, 1000, 1),
            (420, 1000, 42),
            (421, 1000, 42),
            (429, 1000, 43),
            (999, 1000, 100),
            (1000, 1000, 100),
            (2000, 1000, 100),
            (10, 0, 0),
            (0, 0, 0),
            (1, 3, 33),
            (2, 3, 67),
            (3_145_728 / 2, 3_145_728, 50),
        ] {
            assert_eq!(percent(done, size), want, "{done}/{size}");
        }
        // No overflow on big files.
        assert_eq!(percent(u64::MAX / 2, u64::MAX), 50);
    }

    #[test]
    fn progress_is_none_until_begin_and_after_end() {
        let p = Progress::new();
        let stat = |_: &Path| Some(10);
        assert_eq!(p.snapshot(&stat), None);
        p.begin("0.4.0", 1000, Path::new("/c/updates/0.4.0/x.rpm.part"));
        assert!(p.snapshot(&stat).is_some());
        p.end();
        assert_eq!(p.snapshot(&stat), None);
        assert_eq!(Progress::default().snapshot(&stat), None);
    }

    #[test]
    fn the_snapshot_stats_the_part_file_it_was_given() {
        let p = Progress::new();
        p.begin("0.4.0", 1000, Path::new("/c/updates/0.4.0/x.rpm.part"));
        let asked = RefCell::new(Vec::new());
        let stat = |path: &Path| {
            asked.borrow_mut().push(path.to_path_buf());
            Some(421)
        };
        assert_eq!(
            p.snapshot(&stat),
            Some(Phase::Downloading {
                version: "0.4.0".into(),
                downloaded: 421,
                size: 1000
            })
        );
        assert_eq!(
            *asked.borrow(),
            vec![PathBuf::from("/c/updates/0.4.0/x.rpm.part")]
        );
        // A missing part file counts as zero bytes.
        assert_eq!(
            p.snapshot(&|_: &Path| None),
            Some(Phase::Downloading {
                version: "0.4.0".into(),
                downloaded: 0,
                size: 1000
            })
        );
        // A second download replaces the first.
        p.begin("0.5.0", 50, Path::new("/c/y.part"));
        assert_eq!(
            p.snapshot(&|_: &Path| Some(5)),
            Some(Phase::Downloading {
                version: "0.5.0".into(),
                downloaded: 5,
                size: 50
            })
        );
    }

    #[test]
    fn a_real_part_file_is_measured_by_its_length() {
        let dir = tempfile::tempdir().unwrap();
        let part = dir.path().join("x.rpm.part");
        fs::write(&part, vec![0u8; 421]).unwrap();
        let p = Progress::new();
        p.begin("0.4.0", 1000, &part);
        let stat = |q: &Path| fs::metadata(q).ok().map(|m| m.len());
        let snap = p.snapshot(&stat).unwrap();
        assert_eq!(
            text::tooltip_from(Lang::En, None, Some(&snap), &[]),
            "Downloading update 42%"
        );
    }

    #[test]
    fn the_tooltip_shows_the_download_while_the_update_lock_is_not_available() {
        let p = Progress::new();
        p.begin("0.4.0", 1000, Path::new("/c/x.part"));
        let snap = p.snapshot(&|_: &Path| Some(421));
        assert_eq!(
            text::tooltip_from(Lang::En, None, snap.as_ref(), &[]),
            "Downloading update 42%"
        );
        assert_eq!(
            text::tooltip_from(Lang::Ar, None, snap.as_ref(), &[]),
            "تنزيل التحديث 42٪"
        );
        // 0 and 100 percent.
        let zero = p.snapshot(&|_: &Path| Some(0));
        assert_eq!(
            text::tooltip_from(Lang::En, None, zero.as_ref(), &[]),
            "Downloading update 0%"
        );
        let full = p.snapshot(&|_: &Path| Some(1000));
        assert_eq!(
            text::tooltip_from(Lang::Ar, None, full.as_ref(), &[]),
            "تنزيل التحديث 100٪"
        );
        // After end() the tooltip is the idle text again.
        p.end();
        let snap = p.snapshot(&|_: &Path| Some(421));
        for lang in ar_en() {
            assert_eq!(
                text::tooltip_from(lang, None, snap.as_ref(), &[]),
                text::idle(lang)
            );
        }
    }

    #[test]
    fn the_tooltip_falls_back_to_the_state_and_the_job_lines() {
        let (mut st, _rig) = state_for("0.3.3", Os::Linux);
        // Lock got, nothing downloading: the same as tooltip_text.
        assert_eq!(
            text::tooltip_from(Lang::En, Some(&st), None, &[]),
            text::tooltip_text(Lang::En, &st, &[])
        );
        st.phase = Phase::Ready("0.4.0".into());
        assert_eq!(
            text::tooltip_from(Lang::Ar, Some(&st), None, &[]),
            text::idle(Lang::Ar)
        );
        // Lock busy, no download, jobs running: the job lines.
        let active = vec![("PDF".to_string(), 0.07)];
        assert_eq!(text::tooltip_from(Lang::En, None, None, &active), "PDF: 7%");
        // A download wins over the state.
        let dl = Phase::Downloading {
            version: "0.4.0".into(),
            downloaded: 1,
            size: 2,
        };
        assert_eq!(
            text::tooltip_from(Lang::En, Some(&st), Some(&dl), &[]),
            "Downloading update 50%"
        );
    }

    // ---------------------------------------------------- busy answers

    #[test]
    fn apply_answers_at_once_when_the_update_lock_is_busy() {
        let (st, _rig, _cfg) = ready_state(Os::Linux, UpdateMode::Ask);
        let m = Mutex::new(st);
        let held = m.lock().unwrap();
        let mut got = m.try_lock().ok();
        assert!(got.is_none(), "the lock is held, try_lock must fail");
        let (resp, effects) = request_apply_or_busy(got.as_deref_mut());
        match resp {
            Response::Error { message } => {
                assert!(
                    message.contains("no verified update to install"),
                    "{message}"
                )
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(effects, Vec::<Effect>::new());
        drop(held);
        // Free again: the real answer, even though the update is Ready.
        let mut got = m.try_lock().ok();
        let (resp, effects) = request_apply_or_busy(got.as_deref_mut());
        assert_eq!(resp, Response::Ok);
        assert_eq!(effects, vec![Effect::Apply]);
    }

    #[test]
    fn apply_with_the_lock_gives_the_plain_answer_when_nothing_is_ready() {
        let (mut st, _rig) = state_for("0.3.3", Os::Linux);
        let (resp, effects) = request_apply_or_busy(Some(&mut st));
        match resp {
            Response::Error { message } => {
                assert!(
                    message.contains("no verified update to install"),
                    "{message}"
                )
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(effects, Vec::<Effect>::new());
    }

    #[test]
    fn check_answers_downloading_at_once_when_the_update_lock_is_busy() {
        let (st, _rig) = state_for("0.3.3", Os::Linux);
        let m = Mutex::new(st);
        let _held = m.lock().unwrap();
        let mut got = m.try_lock().ok();
        let fake = Fake::new(Feed::Good);
        let (resp, effects) =
            check_or_busy(got.as_deref_mut(), "0.3.3", &Config::default(), &fake, BASE);
        assert_eq!(
            status_of(&resp),
            ("downloading".into(), "0.3.3".into(), None)
        );
        assert_eq!(effects, Vec::<Effect>::new());
        assert_eq!(fake.calls(), 0, "a busy check never fetches");
    }

    #[test]
    fn check_with_the_lock_is_the_manual_check() {
        let (mut st, _rig) = state_for("0.4.0", Os::Linux);
        let fake = Fake::new(Feed::Good);
        let (resp, _) = check_or_busy(Some(&mut st), "0.4.0", &Config::default(), &fake, BASE);
        assert_eq!(
            status_of(&resp),
            ("up-to-date".into(), "0.4.0".into(), Some("0.4.0".into()))
        );
        assert_eq!(fake.calls(), 2);
    }

    fn code_of(file: &str) -> String {
        super::code(file)
    }

    fn body_of(src: &str, header: &str) -> String {
        let start = src
            .find(header)
            .unwrap_or_else(|| panic!("{header} exists"));
        let rest = &src[start..];
        let end = rest.find("\n}\n").map_or(rest.len(), |e| e + 3);
        rest[..end].to_string()
    }

    #[test]
    fn the_server_and_the_menu_never_wait_on_the_update_lock() {
        let server = code_of("server.rs");
        assert!(server.contains("request_apply_or_busy("));
        assert!(server.contains("check_or_busy("));
        assert!(server.contains("try_lock"));
        assert!(!server.contains("lock(crate::update_sys::shared())"));
        assert!(!server.contains("update::request_apply(&mut"));
        assert!(!server.contains("update::check_now("));

        let sys = code_of("update_sys.rs");
        let menu = body_of(&sys, "pub fn apply_from_menu");
        assert!(menu.contains("request_apply_or_busy("), "{menu}");
        assert!(menu.contains("try_lock"), "{menu}");
        assert!(!menu.contains("lock(shared())"), "{menu}");
        assert!(!menu.contains("request_apply(&mut"), "{menu}");
    }

    #[test]
    fn the_tray_reads_the_progress_not_only_the_update_lock() {
        let tray = code_of("tray/linux.rs");
        assert!(tray.contains("tooltip_from("));
        assert!(tray.contains("progress::"));
    }

    #[test]
    fn the_download_publishes_its_progress() {
        let upd = code_of("update.rs");
        assert!(upd.contains("progress::shared()"));
        assert!(upd.contains(".begin("));
        assert!(upd.contains(".end()"));
    }

    // ---------------------------------------------------- schedule file

    #[test]
    fn write_atomic_leaves_one_complete_file_and_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub/update-state.json");
        write_atomic(&path, "{\"failures\":0}").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"failures\":0}");
        write_atomic(&path, "{\"failures\":2}").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"failures\":2}");
        assert_eq!(listing(path.parent().unwrap()), vec!["update-state.json"]);
        // A long old content is fully replaced by a short new one.
        write_atomic(&path, "{}").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{}");
    }

    #[test]
    fn a_failing_rename_keeps_the_old_content_and_leaves_no_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        // The target is a non empty directory: renaming a file over it fails.
        let target = dir.path().join("update-state.json");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("old.txt"), "old").unwrap();
        assert!(write_atomic(&target, "{}").is_err());
        assert_eq!(fs::read_to_string(target.join("old.txt")).unwrap(), "old");
        assert_eq!(listing(dir.path()), vec!["update-state.json"]);
    }

    #[test]
    fn the_schedule_is_saved_through_write_atomic_not_fs_write() {
        let sys = code_of("update_sys.rs");
        assert!(sys.contains("write_atomic("));
        assert!(!sys.contains("fs::write"));
    }

    // ---------------------------------------------------- restart

    #[test]
    fn the_restart_command_runs_the_exe_directly_without_a_shell() {
        assert_eq!(RESTART_DELAY, Duration::from_secs(2));
        assert_eq!(
            restart_command(Path::new("/usr/bin/kawarod")),
            (PathBuf::from("/usr/bin/kawarod"), Vec::<String>::new())
        );
        assert_eq!(
            restart_command(Path::new("/opt/kawaro/kawarod (deleted)")),
            (PathBuf::from("/opt/kawaro/kawarod"), Vec::<String>::new())
        );
        assert_eq!(
            clean_exe_path(Path::new("/usr/bin/kawarod (deleted)")),
            PathBuf::from("/usr/bin/kawarod")
        );
        assert_eq!(
            clean_exe_path(Path::new("/usr/bin/kawarod")),
            PathBuf::from("/usr/bin/kawarod")
        );
        assert_eq!(
            clean_exe_path(Path::new("/usr/bin/(deleted)/kawarod")),
            PathBuf::from("/usr/bin/(deleted)/kawarod")
        );
    }

    #[test]
    fn update_sys_starts_no_shell() {
        let sys = code_of("update_sys.rs");
        for word in ["/bin/sh", "sh -c", "Command::new(\"sh\")", "\"-c\""] {
            assert!(!sys.contains(word), "update_sys.rs must not contain {word}");
        }
        assert!(sys.contains("restart_command("));
    }
}
