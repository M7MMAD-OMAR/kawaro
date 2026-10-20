//! `sbarhived`: the SbarHive background service. Owns the hive window, the
//! job queue, the tray icon and the socket in `$XDG_RUNTIME_DIR`.

mod jobs;
mod notify;
mod server;
mod state;
mod text;
mod tray;
mod ui;
mod watch;

use state::lock;

use sbarhive_core::config::Config;
use sbarhive_core::ipc::{Request, Response, socket_path};
use sbarhive_core::queue::Queue;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::ExitCode;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

fn already_running() -> bool {
    let Ok(mut s) = UnixStream::connect(socket_path()) else {
        return false;
    };
    let _ = s.set_read_timeout(Some(Duration::from_millis(500)));
    let line = serde_json::to_string(&Request::Ping).unwrap_or_default();
    if writeln!(s, "{line}").is_err() {
        return false;
    }
    let mut reply = String::new();
    BufReader::new(s).read_line(&mut reply).is_ok()
        && matches!(serde_json::from_str(&reply), Ok(Response::Ok))
}

fn workers() -> usize {
    std::thread::available_parallelism()
        .map(|n| (n.get() / 2).max(1))
        .unwrap_or(1)
}

fn main() -> ExitCode {
    if already_running() {
        eprintln!("sbarhived: already running");
        return ExitCode::SUCCESS;
    }
    let path = socket_path();
    let _ = std::fs::remove_file(&path);
    let listener = match UnixListener::bind(&path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("sbarhived: cannot listen on {}: {e}", path.display());
            return ExitCode::FAILURE;
        }
    };

    let (config, warning) = Config::load();
    if let Some(w) = warning {
        eprintln!("sbarhived: settings: {w}");
    }
    let lang = config.lang();
    if let Err(e) = sbarhive_ui::init() {
        eprintln!("sbarhived: no display: {e}");
        return ExitCode::FAILURE;
    }
    // Matches the desktop file, so compositors, window rules and portals
    // can tell SbarHive windows apart from any other.
    if let Err(e) = slint::set_xdg_app_id("io.github.m7mmad_omar.SbarHive") {
        eprintln!("sbarhived: app id: {e}");
    }
    let engines = sbarhive_engines::all();
    let (ev_tx, ev_rx) = mpsc::channel();
    let queue = Arc::new(Queue::new(engines.clone(), workers(), ev_tx));
    let state: state::Shared = Arc::new(Mutex::new(state::State::new(config, &engines)));

    let theme = lock(&state).config.theme;
    if let Err(e) = ui::build(lang, theme, &state, &queue) {
        eprintln!("sbarhived: cannot create the windows: {e}");
        return ExitCode::FAILURE;
    }

    let tray = tray::start(lang, queue.clone(), state.clone());
    {
        let state = state.clone();
        std::thread::spawn(move || jobs::pump(ev_rx, state, tray));
    }
    {
        let (state, queue) = (state.clone(), queue.clone());
        std::thread::spawn(move || server::serve(listener, state, queue));
    }

    {
        let (state, queue) = (state.clone(), queue.clone());
        std::thread::spawn(move || watch::run(state, queue));
    }

    let result = slint::run_event_loop_until_quit();
    queue.cancel_all();
    let _ = std::fs::remove_file(&path);
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("sbarhived: {e}");
            ExitCode::FAILURE
        }
    }
}
