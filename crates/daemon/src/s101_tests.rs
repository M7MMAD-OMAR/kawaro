//! S-101 (backlog/S-101-update-ui-defects.md), the in-binary tests the
//! daemon being a binary crate keeps out of `tests/`: A1 (a second check
//! while the first downloads), A3 (apply counts), B1 (the hold before the
//! quit), B2 (the marker with a visible window) and B3 (the start that
//! reopens the page). Everything runs on fakes: no network, no desktop.

use super::*;
use crate::notify::{Sent, report_update_result_with};
use crate::reopen::{
    ApplyOrigin, MARKER_FILE, Marker, before_auto_install, before_install, on_start,
};
use crate::ui;
use crate::update::{
    INSTALL_HOLD, check_or_busy, hold_before_quit, hold_for_installing, set_installing,
};
use kawaro_ui::UpdateResult as Shown;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Mutex, mpsc::Receiver};
use std::time::Instant;

const NOW: u64 = 1_800_000_000;

fn word(r: &Response) -> String {
    match r {
        Response::Update { status, .. } => status.clone(),
        other => panic!("{other:?}"),
    }
}

// ---------------------------------------------------------------- A1

/// Serves the manifest and the signature at once and holds the package: it
/// writes 7 of its 17 bytes, tells the test, and waits to be released.
struct Gate {
    calls: AtomicUsize,
    package_calls: AtomicUsize,
    started: mpsc::Sender<()>,
    release: Mutex<Receiver<()>>,
}

impl Fetcher for Gate {
    fn get(&self, url: &str, dest: &Path, _max: u64) -> Result<(), FetchError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if url.ends_with(".minisig") {
            fs::write(dest, fx("s054b/manifest-stable-0.4.0.json.minisig")).unwrap();
        } else if url.ends_with(".json") {
            fs::write(dest, fx("s054b/manifest-stable-0.4.0.json")).unwrap();
        } else {
            self.package_calls.fetch_add(1, Ordering::SeqCst);
            fs::write(dest, &GOOD[..7]).unwrap();
            self.started.send(()).unwrap();
            let _ = crate::state::lock(&self.release).recv_timeout(Duration::from_secs(5));
            fs::write(dest, GOOD).unwrap();
        }
        Ok(())
    }
}

#[test]
fn ac_a1_a_second_check_while_the_first_downloads_answers_at_once_and_fetches_nothing() {
    let (st, _rig) = state_for("0.3.3", Os::Linux);
    let state = Mutex::new(st);
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let gate = Gate {
        calls: AtomicUsize::new(0),
        package_calls: AtomicUsize::new(0),
        started: started_tx,
        release: Mutex::new(release_rx),
    };
    let cfg = config(UpdateMode::Ask);
    std::thread::scope(|s| {
        let first = s.spawn(|| {
            let mut guard = state.lock().unwrap();
            check_or_busy(Some(&mut guard), "0.3.3", &cfg, &gate, BASE)
        });
        started_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("the download started");
        let begun = Instant::now();
        let mut busy = state.try_lock().ok();
        assert!(busy.is_none(), "the download holds the update lock");
        let (reply, effects) = check_or_busy(busy.as_deref_mut(), "0.3.3", &cfg, &gate, BASE);
        assert!(begun.elapsed() < Duration::from_millis(500));
        assert_eq!(word(&reply), "downloading");
        assert!(effects.is_empty());
        // The progress is global, and another test may end its own download
        // a moment before this read, so ask again a few times.
        let mut with_percent = false;
        for _ in 0..50 {
            let (r, _) = check_or_busy(None, "0.3.3", &cfg, &gate, BASE);
            if matches!(
                r,
                Response::Update {
                    percent: Some(_),
                    size: Some(_),
                    ..
                }
            ) {
                with_percent = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(with_percent, "the busy answer carries the percent and size");
        assert_eq!(gate.package_calls.load(Ordering::SeqCst), 1);
        release_tx.send(()).unwrap();
        let (done, _) = first.join().unwrap();
        assert_eq!(word(&done), "ready", "the first check keeps its verdict");
    });
    assert_eq!(gate.package_calls.load(Ordering::SeqCst), 1, "one download");
    assert_eq!(
        gate.calls.load(Ordering::SeqCst),
        3,
        "manifest, signature, package"
    );
}

// ---------------------------------------------------------------- A3

#[test]
fn ac_a3_ask_mode_never_applies_by_itself_and_one_update_now_applies_once() {
    for os in [Os::Windows, Os::Linux] {
        let (mut st, _rig) = state_for("0.3.3", os);
        let cfg = config(UpdateMode::Ask);
        let fake = Fake::new(Feed::Good);
        let mut applies = 0;
        let count = |effects: Vec<Effect>| effects.iter().filter(|e| **e == Effect::Apply).count();
        for i in 0..3 {
            applies += count(check_now(&mut st, &cfg, &fake, BASE + i).1);
        }
        for h in 0..48 {
            applies += count(tick(&mut st, &cfg, &fake, BASE + h * HOUR));
        }
        assert_eq!(applies, 0, "{os:?}");
        assert_eq!(st.phase, Phase::Ready("0.4.0".into()), "{os:?}");
        let (reply, effects) = request_apply(&mut st);
        assert!(matches!(reply, Response::Ok));
        assert_eq!(count(effects), 1, "{os:?}");
    }
}

// ---------------------------------------------------------------- B1

#[test]
fn ac_b1_the_daemon_stays_alive_1500_ms_answering_installing_before_it_quits() {
    let (st, _rig) = state_for("0.4.2", Os::Windows);
    set_installing(Some("0.4.3".into()));
    let cfg = config(UpdateMode::Ask);
    let clock = Cell::new(10_000u64);
    let replies = RefCell::new(Vec::new());
    let m = Mutex::new(st);
    let _held = m.lock().unwrap();
    hold_for_installing(
        clock.get(),
        || clock.get(),
        |d| {
            let mut busy = m.try_lock().ok();
            let (r, _) = check_or_busy(
                busy.as_deref_mut(),
                "0.4.2",
                &cfg,
                &Fake::new(Feed::Good),
                0,
            );
            replies.borrow_mut().push(word(&r));
            clock.set(clock.get() + d.as_millis() as u64);
        },
    );
    // The quit effect comes here, after the hold.
    assert!(clock.get() - 10_000 >= INSTALL_HOLD.as_millis() as u64);
    let replies = replies.into_inner();
    assert!(!replies.is_empty(), "at least one reply before the quit");
    assert!(replies.iter().all(|w| w == "installing"), "{replies:?}");
    set_installing(None);
}

#[test]
fn ac_b1_nothing_waits_when_no_install_runs() {
    set_installing(None);
    let begun = Instant::now();
    hold_before_quit();
    assert!(begun.elapsed() < Duration::from_millis(200));
}

#[test]
fn ac_b1_an_install_that_started_long_ago_does_not_hold_again() {
    let spent = Cell::new(0u64);
    hold_for_installing(0, || 5_000, |_| spent.set(spent.get() + 1));
    assert_eq!(spent.get(), 0);
}

// ---------------------------------------------------------------- B2

#[test]
fn ac_b2_an_automatic_install_writes_the_marker_only_with_the_window_visible() {
    let dir = tempfile::tempdir().unwrap();
    assert!(!before_auto_install(
        dir.path(),
        false,
        "0.4.2",
        "0.4.3",
        NOW
    ));
    assert!(
        !dir.path().join(MARKER_FILE).exists(),
        "tray and background write none"
    );
    assert!(before_auto_install(dir.path(), true, "0.4.2", "0.4.3", NOW));
    let text = fs::read_to_string(dir.path().join(MARKER_FILE)).unwrap();
    assert_eq!(
        Marker::parse(&text),
        Some(Marker {
            from: "0.4.2".into(),
            to: "0.4.3".into(),
            at: NOW
        })
    );
    // The click on Update now keeps its own rule.
    let other = tempfile::tempdir().unwrap();
    assert!(!before_install(
        other.path(),
        ApplyOrigin::Auto,
        true,
        "0.4.2",
        "0.4.3",
        NOW
    ));
}

// ---------------------------------------------------------------- B3

fn marker_file(dir: &Path, to: &str, at: u64) {
    let m = Marker {
        from: "0.4.2".into(),
        to: to.into(),
        at,
    };
    fs::write(dir.join(MARKER_FILE), m.to_text()).unwrap();
}

#[test]
fn ac_b3_a_start_after_a_failed_install_opens_the_page_once_with_the_failed_banner() {
    let dir = tempfile::tempdir().unwrap();
    marker_file(dir.path(), "0.4.3", NOW - 10);
    let failure = Outcome::Failed(Failure::Text("msiexec 1603".into()));
    write_result(dir.path(), &result("0.4.3", failure)).unwrap();
    let before = ui::settings_opens();
    assert!(on_start(dir.path(), "0.4.2", NOW, &Config::default()));
    assert_eq!(ui::settings_opens(), before + 1, "exactly one open");
    assert_eq!(ui::last_settings_page(), Some(3));
    assert_eq!(
        ui::last_settings_result(),
        Some(Shown::Failed("0.4.3".into()))
    );
    assert!(
        !dir.path().join(MARKER_FILE).exists(),
        "the marker is used once"
    );
    assert!(
        dir.path().join(RESULT_FILE).exists(),
        "the result file stays for the toast"
    );
    assert!(!on_start(dir.path(), "0.4.2", NOW, &Config::default()));
    assert_eq!(ui::settings_opens(), before + 1);
    // The toast still takes the result afterwards.
    let (mut sink, mut sent) = (Rec::default(), Sent::default());
    report_update_result_with(&mut sink, &mut sent, Lang::En, dir.path());
    assert_eq!(sink.shown.len(), 1);
    assert!(!dir.path().join(RESULT_FILE).exists());
}

#[test]
fn ac_b3_a_start_into_the_new_version_opens_the_page_with_the_updated_banner() {
    let dir = tempfile::tempdir().unwrap();
    marker_file(dir.path(), "0.4.3", NOW - 10);
    let before = ui::settings_opens();
    assert!(on_start(dir.path(), "0.4.3", NOW, &Config::default()));
    assert_eq!(ui::settings_opens(), before + 1);
    assert_eq!(
        ui::last_settings_result(),
        Some(Shown::Updated("0.4.3".into()))
    );
}

#[test]
fn ac_b3_the_old_version_without_a_failed_result_opens_nothing() {
    let dir = tempfile::tempdir().unwrap();
    marker_file(dir.path(), "0.4.3", NOW - 10);
    let before = ui::settings_opens();
    assert!(!on_start(dir.path(), "0.4.2", NOW, &Config::default()));
    assert_eq!(ui::settings_opens(), before);
    assert!(!dir.path().join(MARKER_FILE).exists());
}
