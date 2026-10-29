//! S-087 (backlog/S-087-first-run-welcome.md): the daemon side of the
//! first-run welcome. AC1 routes `Request::Settings` on the `welcome_seen`
//! marker and adds the bilingual tray item, AC4 installs the bundled sample
//! and opens the hive for exactly that file through the same request path
//! the Explorer menu entry uses. The window work is posted to an event loop
//! no test owns, so the seams in `ui` count the calls (like `s086_sources`
//! describes for settings).

use crate::state::{Shared, State};
use crate::ui;
use kawaro_core::config::Config;
use kawaro_core::i18n::Lang;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// The state of a daemon with `config` and the real engines, for the
/// direct calls below (the hive must find actions for the sample).
fn state_with(config: Config) -> Shared {
    Arc::new(Mutex::new(State::new(config, &kawaro_engines::all())))
}

fn sha256(path: &Path) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    Sha256::digest(std::fs::read(path).unwrap()).to_vec()
}

// ------------------------------------------------------------ AC1

/// The request routing over the socket, as `s086_settings_routing` feeds
/// it. Unix only, like that module.
#[cfg(unix)]
mod socket {
    use super::{Config, Shared};
    use crate::server::handle;
    use crate::state::State;
    use kawaro_core::queue::Queue;
    use kawaro_core::transport::{Bound, Endpoint, Listener, Stream};
    use std::io::{BufRead, BufReader, Write};
    use std::sync::{Arc, Mutex, mpsc};

    /// One request through a fresh fake listener and the real `handle`, with
    /// `config` as the daemon's loaded settings, and the reply line the
    /// caller would see.
    pub fn route_with(config: Config, request: &str) -> String {
        let dir = tempfile::tempdir().unwrap();
        let endpoint = Endpoint::from_path(dir.path().join("kawaro.sock"));
        let Bound::Listening(listener) = Listener::bind(&endpoint).unwrap() else {
            panic!("fresh socket must bind");
        };
        let mut client = Stream::connect(&endpoint).unwrap();
        let server = listener.accept().unwrap();
        writeln!(client, "{request}").unwrap();
        let state: Shared = Arc::new(Mutex::new(State::new(config, &[])));
        let (tx, _rx) = mpsc::channel();
        let queue = Arc::new(Queue::new(kawaro_engines::all(), 1, tx));
        handle(server, &state, &queue);
        let mut reply = String::new();
        BufReader::new(&mut client)
            .read_line(&mut reply)
            .expect("one reply line");
        reply.trim_end().to_string()
    }

    pub fn route(request: &str) -> String {
        route_with(Config::default(), request)
    }
}

#[cfg(unix)]
use socket::{route, route_with};

#[cfg(unix)]
#[test]
fn ac1_no_marker_the_settings_request_opens_the_welcome_window() {
    let (w, s) = (ui::welcome_opens(), ui::settings_opens());
    assert_eq!(route(r#"{"cmd":"settings"}"#), r#"{"status":"ok"}"#);
    assert_eq!(
        ui::welcome_opens(),
        w + 1,
        "the route reaches ui::present_welcome"
    );
    assert_eq!(ui::settings_opens(), s, "the settings window stays shut");
}

#[cfg(unix)]
#[test]
fn ac1_with_the_marker_the_settings_request_opens_the_settings_window() {
    let c = Config {
        welcome_seen: true,
        ..Config::default()
    };
    let (w, s) = (ui::welcome_opens(), ui::settings_opens());
    assert_eq!(route_with(c, r#"{"cmd":"settings"}"#), r#"{"status":"ok"}"#);
    assert_eq!(
        ui::settings_opens(),
        s + 1,
        "the route reaches ui::open_settings"
    );
    assert_eq!(ui::welcome_opens(), w, "the welcome stays shut");
}

#[cfg(unix)]
#[test]
fn ac1_other_commands_open_no_welcome() {
    let w = ui::welcome_opens();
    assert_eq!(route(r#"{"cmd":"ping"}"#), r#"{"status":"ok"}"#);
    assert_eq!(ui::welcome_opens(), w, "ping must not open the welcome");
}

#[test]
fn ac1_the_tray_item_is_bilingual_get_started() {
    assert_eq!(crate::text::get_started(Lang::En), "Get started");
    assert_eq!(crate::text::get_started(Lang::Ar), "ابدأ هنا");
}

#[test]
fn ac1_the_tray_menu_carries_get_started_at_index_one_in_both_languages() {
    let state = crate::update::UpdateState::new(crate::update::UpdateEnv {
        installed: String::new(),
        keys: vec![],
        platform: kawaro_core::update::policy::Platform {
            os: String::new(),
            arch: String::new(),
            pkg: String::new(),
        },
        os: kawaro_core::paths::Os::Linux,
        updates_dir: Default::default(),
        scratch: Default::default(),
    });
    for lang in [Lang::En, Lang::Ar] {
        let labels = crate::text::menu_labels(lang, &state);
        assert_eq!(labels.len(), 6, "{lang:?} {labels:?}");
        assert_eq!(labels[1], crate::text::get_started(lang), "{lang:?}");
        let mut distinct = labels.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(distinct.len(), 6, "duplicate menu labels: {labels:?}");
    }
}

#[test]
fn ac1_save_welcome_seen_sets_only_the_marker_and_keeps_the_other_settings() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("config.toml");
    let mut c = Config {
        quality: kawaro_core::config::Quality::Max,
        ..Config::default()
    };
    c.output_folder = "Out".into();
    let state = state_with(c.clone());
    ui::save_language_choice(&state, &p, kawaro_core::i18n::Lang::Ar).unwrap();
    assert!(!state.lock().unwrap().config.welcome_seen);
    let (loaded, warn) = Config::load_from(&p);
    assert!(warn.is_none());
    assert!(!loaded.welcome_seen);
    assert_eq!(loaded.language, kawaro_core::config::LangSetting::Ar);
    assert_eq!(loaded.quality, c.quality);
    assert_eq!(loaded.output_folder, c.output_folder);
}

#[test]
fn language_choice_does_not_change_state_if_save_fails() {
    let dir = tempfile::tempdir().unwrap();
    let original = Config::default();
    let state = state_with(original.clone());
    let result = ui::save_language_choice(&state, dir.path(), kawaro_core::i18n::Lang::En);
    assert!(result.is_err());
    let st = state.lock().unwrap();
    assert_eq!(st.config, original);
    assert_eq!(st.lang, original.lang());
}

#[test]
fn ac1_the_routing_counts_come_from_the_two_ui_seams() {
    let state = state_with(Config::default());
    let (w, s) = (ui::welcome_opens(), ui::settings_opens());
    crate::server::settings(&state);
    assert_eq!(ui::welcome_opens(), w + 1);
    assert_eq!(ui::settings_opens(), s);
    let c = Config {
        welcome_seen: true,
        ..Config::default()
    };
    let state = state_with(c);
    crate::server::settings(&state);
    assert_eq!(ui::settings_opens(), s + 1);
    assert_eq!(ui::welcome_opens(), w + 1);
}

// ------------------------------------------------------------ AC4

#[test]
fn ac4_try_sample_installs_the_sample_and_opens_the_hive_for_exactly_it() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_with(Config::default());
    let sample = ui::try_sample(&state, dir.path()).expect("the sample installs");
    assert_eq!(sample, dir.path().join("sample.png"));
    assert!(sample.exists(), "the sample lands in the writable folder");
    // The bundled bytes, unchanged by the install.
    assert_eq!(
        std::fs::read(&sample).unwrap(),
        kawaro_ui::welcome::sample_bytes(),
        "the installed sample is the bundled image"
    );
    let before = sha256(&sample);
    // The hive opens for exactly that one file, through `server::open`, the
    // same request path the Explorer menu entry takes.
    let pending = state.lock().unwrap().pending.clone();
    assert_eq!(
        pending,
        Some(vec![sample.clone()]),
        "the hive shows the sample"
    );
    assert_eq!(before, sha256(&sample), "opening never touches the sample");
}

#[test]
fn ac4_the_conversion_result_lands_beside_the_untouched_sample() {
    use kawaro_core::engine::CancelToken;
    use kawaro_core::queue::run_job;
    use kawaro_core::{ActionDef, Registry};
    use kawaro_engines::image::ImageEngine;
    use std::sync::mpsc;

    let dir = tempfile::tempdir().unwrap();
    let state = state_with(Config::default());
    let sample = ui::try_sample(&state, dir.path()).expect("the sample installs");
    let before = sha256(&sample);

    // One real image action over the sample, like a drop on a cell runs it.
    let registry = Registry::builtin();
    let action: &ActionDef = registry
        .get("image.rotate-90")
        .expect("the sample image has the rotate action");
    let (tx, _rx) = mpsc::channel();
    let outputs = run_job(
        1,
        action,
        std::slice::from_ref(&sample),
        &ImageEngine::default(),
        &CancelToken::new(),
        &tx,
    );
    assert_eq!(outputs.len(), 1, "one result for one sample: {outputs:?}");
    let out = &outputs[0];
    assert_ne!(out, &sample, "the result is a new file");
    assert_eq!(
        out.parent(),
        sample.parent(),
        "the result lands beside the sample"
    );
    assert!(out.exists());
    assert_eq!(sha256(&sample), before, "the sample is untouched");
}
