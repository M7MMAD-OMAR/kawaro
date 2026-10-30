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
    let result = open(&state, vec![PathBuf::from("unknown.zzz")], None);
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
