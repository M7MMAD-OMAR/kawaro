//! S-046 AC8 (backlog/S-046-windows-hive-window.md): `server::open` refuses
//! a selection no action fits, before any window is built.

use crate::server::open;
use crate::state::State;
use kawaro_core::config::Config;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[test]
fn ac8_open_presents_the_reason_when_no_action_matches_the_file() {
    let state = Arc::new(Mutex::new(State::new(Config::default(), &[])));
    let result = open(&state, vec![PathBuf::from("unknown.zzz")], None, None);
    // S-096: the hive explains instead of failing; no job is pending.
    assert!(result.is_ok(), "{result:?}");
    assert!(state.lock().unwrap().pending.is_none());
}

// S-086 AC1 (backlog/S-086-icon-opens-window.md): the request routing of
// `kawarod settings` (and of `kawaro settings`, which sends the same
// `Request::Settings`). A fake listener from `core::transport` feeds `handle`
// one `settings` line and the test proves the route reaches
// `ui::open_settings`; the window itself is posted to an event loop no test
// owns, so the seam in `ui` counts the calls. The test contract the ACs
// leave open: a `settings` request always replies `ok` (the window opens
// asynchronously, so there is no failure answer), and only the `settings`
// command reaches `open_settings`. S-087 D1: these tests run with the
// `welcome_seen` marker set, which is the only change the first-run welcome
// makes to S-086 AC1; without the marker the same route opens the welcome
// window (proven in `s087_welcome_tests`).
#[cfg(unix)]
mod s086_settings_routing {
    use crate::server::handle;
    use crate::state::{Shared, State};
    use crate::ui;
    use kawaro_core::config::Config;
    use kawaro_core::queue::Queue;
    use kawaro_core::transport::{Bound, Endpoint, Listener, Stream};
    use std::io::{BufRead, BufReader, Write};
    use std::sync::{Arc, Mutex, mpsc};

    /// One request through a fresh fake listener and the real `handle`, and
    /// the reply line the caller would see. The marker is set (S-087 D1).
    fn route(request: &str) -> String {
        let dir = tempfile::tempdir().unwrap();
        let endpoint = Endpoint::from_path(dir.path().join("kawaro.sock"));
        let Bound::Listening(listener) = Listener::bind(&endpoint).unwrap() else {
            panic!("fresh socket must bind");
        };
        let mut client = Stream::connect(&endpoint).unwrap();
        let server = listener.accept().unwrap();
        // One line, newline included: `handle` reads a full line.
        writeln!(client, "{request}").unwrap();
        let state: Shared = Arc::new(Mutex::new(State::new(seen_config(), &[])));
        let (tx, _rx) = mpsc::channel();
        let queue = Arc::new(Queue::new(kawaro_engines::all(), 1, tx));
        handle(server, &state, &queue);
        let mut reply = String::new();
        BufReader::new(&mut client)
            .read_line(&mut reply)
            .expect("one reply line");
        reply.trim_end().to_string()
    }

    /// A config whose welcome has been seen (S-087 D1): the settings entry
    /// opens the settings window exactly as S-086 AC1 requires.
    fn seen_config() -> Config {
        Config {
            welcome_seen: true,
            ..Config::default()
        }
    }

    #[test]
    fn s086_ac1_a_settings_request_reaches_open_settings_and_replies_ok() {
        let before = ui::settings_opens();
        assert_eq!(route(r#"{"cmd":"settings"}"#), r#"{"status":"ok"}"#);
        assert_eq!(
            ui::settings_opens(),
            before + 1,
            "the route reaches ui::open_settings"
        );
    }

    #[test]
    fn s086_ac1_other_commands_do_not_open_settings() {
        let before = ui::settings_opens();
        assert_eq!(route(r#"{"cmd":"ping"}"#), r#"{"status":"ok"}"#);
        assert_eq!(ui::settings_opens(), before, "ping must not open settings");
    }
}

// S-148 AC3 and AC4 (backlog/S-148-job-control-without-tray.md): the
// identity of the daemon and the job references that carry it.
//
// A job reference of an earlier daemon is refused and cancels nothing. A
// reference of this daemon cancels the job it names and nothing else. A
// request without an instance means the current session, like before. A
// missing or finished job gets an explicit answer, and a repeated cancel
// is harmless.
#[cfg(unix)]
mod s148_job_references {
    use crate::server::{handle, instance_id};
    use crate::state::{Shared, State};
    use kawaro_core::config::Config;
    use kawaro_core::engine::{EngineError, TaskOutcome};
    use kawaro_core::queue::{JobId, Queue};
    use kawaro_core::transport::{Bound, Endpoint, Listener, Stream};
    use kawaro_core::{ActionDef, CancelToken, Engine, MediaKind, Progress, Registry, Task};
    use std::io::{BufRead, BufReader, Write};
    use std::sync::{Arc, Mutex, mpsc};
    use std::time::{Duration, Instant};

    /// The text of a stale job reference. It never changes, so a script can
    /// match it.
    const STALE: &str = "stale job reference: the daemon restarted, run `kawaro jobs` again";

    /// An engine that waits until its job is cancelled, so a test holds a
    /// job that is active for as long as it needs. It records the action it
    /// was cancelled in, so the test sees which job stopped.
    struct Waiting {
        cancelled: Arc<Mutex<Vec<String>>>,
    }

    impl Engine for Waiting {
        fn id(&self) -> &'static str {
            "waiting"
        }

        fn capabilities(&self) -> Vec<(&'static str, Vec<MediaKind>)> {
            Vec::new()
        }

        fn run(
            &self,
            task: &Task,
            _progress: Progress,
            cancel: &CancelToken,
        ) -> Result<TaskOutcome, EngineError> {
            while !cancel.is_cancelled() {
                std::thread::sleep(Duration::from_millis(5));
            }
            self.cancelled.lock().unwrap().push(task.action_id.clone());
            Err(EngineError::Cancelled)
        }
    }

    /// The daemon state, its queue and the folder the test inputs live in.
    struct Fixture {
        state: Shared,
        queue: Arc<Queue>,
        cancelled: Arc<Mutex<Vec<String>>>,
        dir: tempfile::TempDir,
    }

    /// A daemon with `workers` workers and the waiting engine.
    fn daemon(workers: usize) -> Fixture {
        let cancelled = Arc::new(Mutex::new(Vec::new()));
        let engine = Arc::new(Waiting {
            cancelled: cancelled.clone(),
        });
        let (tx, _rx) = mpsc::channel();
        let queue = Arc::new(Queue::new(vec![engine], workers, tx));
        let state: Shared = Arc::new(Mutex::new(State::new(Config::default(), &[])));
        Fixture {
            state,
            queue,
            cancelled,
            dir: tempfile::tempdir().unwrap(),
        }
    }

    /// A copy of a built-in action that runs on the waiting engine.
    fn waiting_action(id: &str) -> ActionDef {
        let mut action = Registry::builtin()
            .get("image.to-webp")
            .cloned()
            .expect("the built-in action");
        action.id = String::from(id);
        action.engine = String::from("waiting");
        action
    }

    /// Submits a job on the waiting engine and returns its number. The
    /// daemon keeps the job in its state, so a cancel of that number finds
    /// it.
    fn submit(fixture: &Fixture, id: &str) -> JobId {
        let input = fixture.dir.path().join(format!("{id}.png"));
        let files = vec![input];
        let action = waiting_action(id);
        let mut st = fixture.state.lock().unwrap();
        crate::jobs::submit(&mut st, &fixture.queue, action, files)
    }

    /// One request through a fresh fake listener and the real `handle`, and
    /// the reply line the caller would see.
    fn route(fixture: &Fixture, request: &str) -> String {
        let dir = tempfile::tempdir().unwrap();
        let endpoint = Endpoint::from_path(dir.path().join("kawaro.sock"));
        let Bound::Listening(listener) = Listener::bind(&endpoint).unwrap() else {
            panic!("a fresh socket must bind");
        };
        let mut client = Stream::connect(&endpoint).unwrap();
        let server = listener.accept().unwrap();
        // One line, newline included: `handle` reads a full line.
        writeln!(client, "{request}").unwrap();
        handle(server, &fixture.state, &fixture.queue);
        let mut reply = String::new();
        BufReader::new(&mut client)
            .read_line(&mut reply)
            .expect("one reply line");
        reply.trim_end().to_string()
    }

    /// True when the engine of that action saw its cancel.
    fn was_cancelled(fixture: &Fixture, id: &str) -> bool {
        fixture.cancelled.lock().unwrap().iter().any(|a| a == id)
    }

    /// Waits until the condition holds, up to a few seconds.
    fn wait_for(mut condition: impl FnMut() -> bool) -> bool {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if condition() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        condition()
    }

    #[test]
    fn s148_ac3_a_stale_instance_is_refused_and_cancels_nothing() {
        let fixture = daemon(1);
        let job = submit(&fixture, "wait.a");
        let reply = route(
            &fixture,
            &format!(r#"{{"cmd":"cancel-job","job":{job},"instance":"k-1a-2b"}}"#),
        );
        let expected = format!(r#"{{"status":"error","message":"{STALE}"}}"#);
        assert_eq!(reply, expected);
        // The job keeps running: a reference of an earlier daemon never
        // reaches the queue.
        std::thread::sleep(Duration::from_millis(200));
        assert!(!was_cancelled(&fixture, "wait.a"), "the job was cancelled");
        assert_eq!(fixture.queue.active(), 1, "the job is still active");
    }

    #[test]
    fn s148_ac3_a_matching_instance_cancels_only_that_job() {
        let fixture = daemon(2);
        let first = submit(&fixture, "wait.a");
        submit(&fixture, "wait.b");
        let reply = route(
            &fixture,
            &format!(
                r#"{{"cmd":"cancel-job","job":{first},"instance":"{instance}"}}"#,
                instance = instance_id()
            ),
        );
        assert_eq!(reply, r#"{"status":"ok"}"#);
        assert!(
            wait_for(|| was_cancelled(&fixture, "wait.a")),
            "the named job stops"
        );
        assert!(
            !was_cancelled(&fixture, "wait.b"),
            "the other job keeps running"
        );
    }

    #[test]
    fn s148_ac3_a_request_without_an_instance_means_the_current_session() {
        let fixture = daemon(1);
        let job = submit(&fixture, "wait.a");
        // The line of an older client: no instance field at all.
        let reply = route(&fixture, &format!(r#"{{"cmd":"cancel-job","job":{job}}}"#));
        assert_eq!(reply, r#"{"status":"ok"}"#);
        assert!(wait_for(|| was_cancelled(&fixture, "wait.a")));
    }

    #[test]
    fn s148_ac4_a_missing_job_is_an_explicit_answer() {
        let fixture = daemon(1);
        let reply = route(&fixture, r#"{"cmd":"cancel-job","job":404}"#);
        assert_eq!(
            reply,
            r#"{"status":"error","message":"no such active job: 404"}"#
        );
        assert!(fixture.cancelled.lock().unwrap().is_empty());
    }

    #[test]
    fn s148_ac4_a_repeated_cancel_is_harmless() {
        let fixture = daemon(2);
        let first = submit(&fixture, "wait.a");
        submit(&fixture, "wait.b");
        let request = format!(
            r#"{{"cmd":"cancel-job","job":{first},"instance":"{instance}"}}"#,
            instance = instance_id()
        );
        assert_eq!(route(&fixture, &request), r#"{"status":"ok"}"#);
        // The same request again asks for the same job. It never cancels
        // another one, and it touches no file. The daemon keeps the job in
        // its state until the pump hears that it finished, so the second
        // cancel still finds it.
        assert_eq!(route(&fixture, &request), r#"{"status":"ok"}"#);
        assert!(wait_for(|| was_cancelled(&fixture, "wait.a")));
        assert!(
            !was_cancelled(&fixture, "wait.b"),
            "the other job keeps running"
        );
    }

    #[test]
    fn s148_ac3_the_answers_carry_the_instance_of_this_daemon() {
        let fixture = daemon(1);
        let expected = format!(r#""instance":"{instance}""#, instance = instance_id());
        let reply = route(&fixture, r#"{"cmd":"version"}"#);
        assert!(reply.contains(&expected), "{reply}");
        let reply = route(&fixture, r#"{"cmd":"jobs"}"#);
        assert!(reply.contains(&expected), "{reply}");
    }
}
