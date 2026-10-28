//! S-090 (backlog/S-090-settings-updates-about.md): the daemon side of the
//! Updates section. `exchange` is pinned against a fake service on a Unix
//! socket in a temporary `XDG_RUNTIME_DIR`, `update_round_trip` is pinned by
//! the request it puts on the wire, and `welcome_window` by the window it
//! stores. What `update_round_trip` shows in the window afterwards cannot be
//! observed from here; the review log names the seam.

use super::*;
use kawaro_core::ipc::{Request, Response};
use kawaro_core::transport::{Bound, Listener, endpoint};
use std::io::{BufRead, BufReader, Write};
use std::sync::mpsc;
use std::time::Duration;

/// The tests below point `XDG_RUNTIME_DIR` at their own directory, so they
/// run one at a time.
static ENV: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// A fake service that answers one connection with `reply` (raw text, a
/// newline is appended) and reports the request line it read.
struct Fake {
    _dir: tempfile::TempDir,
    seen: mpsc::Receiver<String>,
    old: Option<std::ffi::OsString>,
}

impl Fake {
    fn start(reply: Option<&'static str>) -> Fake {
        let dir = tempfile::tempdir().unwrap();
        let old = std::env::var_os("XDG_RUNTIME_DIR");
        // SAFETY: serialised by `ENV`; no other test reads this variable.
        unsafe { std::env::set_var("XDG_RUNTIME_DIR", dir.path()) };
        let (tx, seen) = mpsc::channel();
        if let Some(reply) = reply {
            let Bound::Listening(listener) = Listener::bind(&endpoint()).unwrap() else {
                panic!("nothing else listens in the temporary directory");
            };
            std::thread::spawn(move || {
                let stream = listener.accept().unwrap();
                let mut line = String::new();
                BufReader::new(stream.try_clone().unwrap())
                    .read_line(&mut line)
                    .unwrap();
                let _ = tx.send(line);
                let mut stream = stream;
                let _ = writeln!(stream, "{reply}");
            });
        }
        Fake {
            _dir: dir,
            seen,
            old,
        }
    }

    fn request(&self) -> Request {
        let line = self
            .seen
            .recv_timeout(Duration::from_secs(5))
            .expect("the fake service received a request line");
        serde_json::from_str(&line).expect("the request line is a Request")
    }
}

impl Drop for Fake {
    fn drop(&mut self) {
        // SAFETY: as in `start`.
        unsafe {
            match &self.old {
                Some(v) => std::env::set_var("XDG_RUNTIME_DIR", v),
                None => std::env::remove_var("XDG_RUNTIME_DIR"),
            }
        }
    }
}

#[test]
fn s090_exchange_maps_an_update_answer() {
    let _g = lock(&ENV);
    let fake = Fake::start(Some(
        r#"{"status":"update","state":"ready","current":"0.4.0","latest":"0.4.1"}"#,
    ));
    let got = exchange(&Request::UpdateCheck);
    assert_eq!(
        fake.request(),
        Request::UpdateCheck,
        "the request on the wire"
    );
    assert_eq!(
        got,
        Some(Response::Update {
            status: "ready".into(),
            current: "0.4.0".into(),
            latest: Some("0.4.1".into()),
        })
    );
}

#[test]
fn s090_exchange_maps_an_error_answer_and_sends_apply_as_written() {
    let _g = lock(&ENV);
    let fake = Fake::start(Some(r#"{"status":"error","message":"no update ready"}"#));
    let got = exchange(&Request::UpdateApply);
    assert_eq!(fake.request(), Request::UpdateApply);
    assert_eq!(
        got,
        Some(Response::Error {
            message: "no update ready".into()
        })
    );
}

#[test]
fn s090_exchange_maps_ok() {
    let _g = lock(&ENV);
    let fake = Fake::start(Some(r#"{"status":"ok"}"#));
    assert_eq!(exchange(&Request::UpdateApply), Some(Response::Ok));
    assert_eq!(fake.request(), Request::UpdateApply);
}

#[test]
fn s090_exchange_is_none_when_no_service_listens() {
    let _g = lock(&ENV);
    let _fake = Fake::start(None);
    assert_eq!(exchange(&Request::UpdateCheck), None);
}

#[test]
fn s090_exchange_is_none_for_an_answer_that_is_not_a_response() {
    let _g = lock(&ENV);
    let fake = Fake::start(Some("this is not json"));
    assert_eq!(exchange(&Request::UpdateCheck), None);
    let _ = fake.request();
}

#[test]
fn s090_update_round_trip_puts_the_request_on_the_wire() {
    let _g = lock(&ENV);
    for (request, deferred) in [
        (Request::UpdateCheck, false),
        (Request::UpdateApply, false),
        (Request::UpdateApply, true),
    ] {
        let fake = Fake::start(Some(r#"{"status":"ok"}"#));
        update_round_trip(request.clone(), deferred);
        assert_eq!(fake.request(), request, "deferred={deferred}");
    }
}

#[test]
fn s090_welcome_window_is_created_lazily_once_the_service_gave_its_arguments() {
    i_slint_backend_testing::init_no_event_loop();
    assert!(WELCOME.with(|w| w.borrow().is_none()));
    // Without arguments nothing is built and nothing fails.
    assert!(welcome_window().is_ok());
    assert!(WELCOME.with(|w| w.borrow().is_none()), "no args, no window");

    let state: Shared = std::sync::Arc::new(std::sync::Mutex::new(crate::state::State::new(
        Config::default(),
        &kawaro_engines::all(),
    )));
    WELCOME_ARGS.with(|a| *a.borrow_mut() = Some((Lang::En, state)));
    welcome_window().expect("the window builds");
    assert!(WELCOME.with(|w| w.borrow().is_some()), "window stored");
    // A second call keeps the one window.
    let first = WELCOME.with(|w| w.borrow().as_ref().map(|x| x as *const WelcomeWindow));
    welcome_window().expect("second call");
    let second = WELCOME.with(|w| w.borrow().as_ref().map(|x| x as *const WelcomeWindow));
    assert_eq!(first, second);
}

fn show(version: &str, state: kawaro_ui::UpdateState, running: bool) -> UpdateView {
    UpdateView::Show {
        version: version.to_string(),
        state,
        running,
    }
}

fn update(status: &str, current: &str) -> Response {
    Response::Update {
        status: status.to_string(),
        current: current.to_string(),
        latest: None,
    }
}

#[test]
fn s090_update_view_table() {
    use kawaro_ui::UpdateState as S;
    let apply = Request::UpdateApply;
    let check = Request::UpdateCheck;
    let err = || Response::Error {
        message: "boom".to_string(),
    };
    let cases: Vec<(&str, Request, Option<Response>, bool, UpdateView)> = vec![
        // No service: unavailable, not running, whatever the flags.
        (
            "none check",
            check.clone(),
            None,
            false,
            show("1.0", S::Unavailable, false),
        ),
        (
            "none apply",
            apply.clone(),
            None,
            false,
            show("1.0", S::Unavailable, false),
        ),
        (
            "none apply deferred",
            apply.clone(),
            None,
            true,
            show("1.0", S::Unavailable, false),
        ),
        // An Update answer shows the service's current version, not ours.
        (
            "update up-to-date",
            check.clone(),
            Some(update("up-to-date", "2.0")),
            false,
            show("2.0", S::UpToDate, true),
        ),
        (
            "update ready",
            check.clone(),
            Some(update("ready", "2.0")),
            false,
            show("2.0", S::Ready, true),
        ),
        (
            "update downloading",
            check.clone(),
            Some(update("downloading", "2.0")),
            false,
            show("2.0", S::Downloading(0), true),
        ),
        (
            "update offline",
            check.clone(),
            Some(update("offline", "2.0")),
            false,
            show("2.0", S::Unavailable, true),
        ),
        (
            "update odd status",
            apply.clone(),
            Some(update("weird", "3.1")),
            true,
            show("3.1", S::Failed("weird".to_string()), true),
        ),
        // Ok for an apply: waiting text when deferred, else checking.
        (
            "ok apply deferred",
            apply.clone(),
            Some(Response::Ok),
            true,
            UpdateView::Deferred,
        ),
        (
            "ok apply",
            apply.clone(),
            Some(Response::Ok),
            false,
            show("1.0", S::Checking, true),
        ),
        // Ok for anything else is up to date, deferred flag or not.
        (
            "ok check",
            check.clone(),
            Some(Response::Ok),
            false,
            show("1.0", S::UpToDate, true),
        ),
        (
            "ok check deferred",
            check.clone(),
            Some(Response::Ok),
            true,
            show("1.0", S::UpToDate, true),
        ),
        // Errors keep the passed version and the message.
        (
            "error check",
            check.clone(),
            Some(err()),
            false,
            show("1.0", S::Failed("boom".to_string()), true),
        ),
        (
            "error apply deferred",
            apply.clone(),
            Some(err()),
            true,
            show("1.0", S::Failed("boom".to_string()), true),
        ),
    ];
    for (name, request, answer, deferred, want) in cases {
        assert_eq!(
            update_view(&request, answer, deferred, "1.0"),
            want,
            "{name}"
        );
    }
}

#[test]
fn s090_apply_deferred_table() {
    // jobs, hive_open, deferred
    for (jobs, hive, want) in [
        (0, false, false),
        (0, true, true),
        (1, false, true),
        (1, true, true),
        (3, false, true),
    ] {
        assert_eq!(apply_deferred(jobs, hive), want, "jobs={jobs} hive={hive}");
    }
}
