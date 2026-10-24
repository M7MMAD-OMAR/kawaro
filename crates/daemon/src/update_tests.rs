//! S-054b (backlog/S-054b-update-daemon-tray.md), the daemon side. Written by
//! the tester from the acceptance criteria and the review log. Nothing here
//! touches the network, `pkexec`, a package manager or the desktop: the fetcher
//! is a fake, the manifest and signature are the throwaway fixtures of S-053a
//! (key A, `manifest-stable-0.4.0.json`), and the notification and apply calls
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
//!   `sbarhived: notify: <message>` and one `show(app_name, <message>)`; a second
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

use sbarhive_core::config::{Config, UpdateMode};
use sbarhive_core::i18n::Lang;
use sbarhive_core::ipc::Response;
use sbarhive_core::paths::Os;
use sbarhive_core::update::fetch::{FetchError, Fetcher};
use sbarhive_core::update::flow::CheckState;
use sbarhive_core::update::policy::Platform;
use sbarhive_core::update::result::{
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
const REL: &str = "https://github.com/M7MMAD-OMAR/sbarhive/releases/download/v0.4.0";
const PKG_NAME: &str = "sbarhive-0.4.0-1.x86_64.rpm";
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
            _ if url.ends_with(".minisig") => fx("manifest-stable-0.4.0.json.minisig"),
            _ if url.ends_with(".json") => fx("manifest-stable-0.4.0.json"),
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
        keys: vec![String::from_utf8(fx("key-a.pub")).unwrap()],
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
        vec!["Update now", "Cancel all jobs", "Settings", "Quit SbarHive"]
    );
    assert_eq!(
        text::menu_labels(Lang::Ar, &st),
        vec!["حدّث الآن", "إلغاء كل المهام", "الإعدادات", "إنهاء صبار هايف"]
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
        "SbarHive was updated to 0.4.0."
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
        "تم تحديث صبار هايف إلى 0.4.0."
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
    assert_eq!(sink.lines, vec![format!("sbarhived: notify: {msg}")]);
    assert_eq!(sink.shown, vec![("SbarHive".to_string(), msg)]);

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
            .all(|l| l.starts_with("sbarhived: notify: "))
    );
    notify_update_with(&mut sink, &mut sent, Lang::En, UpdateKind::Failed, "0.4.0");
    assert_eq!(sink.lines.len(), 4);
}

#[test]
fn ac6_the_arabic_notification_uses_the_arabic_app_name() {
    let (mut sink, mut sent) = (Rec::default(), Sent::default());
    notify_update_with(&mut sink, &mut sent, Lang::Ar, UpdateKind::Updated, "0.4.0");
    assert_eq!(sink.shown[0].0, "صبار هايف");
    assert_eq!(
        sink.lines[0],
        "sbarhived: notify: تم تحديث صبار هايف إلى 0.4.0."
    );
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
            "sbarhived: notify: {}",
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
            "sbarhived: notify: {}",
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
    assert!(notify.contains("sbarhived: notify:"));
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
    assert!(server.contains("request_apply("));
    assert!(server.contains("check_now("));
}

#[test]
fn ac5_the_linux_tray_uses_the_update_menu_and_tooltip_functions() {
    let tray = code("tray/linux.rs");
    assert!(tray.contains("menu_labels("));
    assert!(tray.contains("update_item_enabled("));
    assert!(tray.contains("tooltip_text("));
}
