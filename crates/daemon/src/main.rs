//! `sbarhived`: the SbarHive background service. Owns the hive window, the
//! job queue, the tray icon and the socket in `$XDG_RUNTIME_DIR`.

mod jobs;
mod notify;
mod portal;
mod portal_bus;
mod server;
mod state;
mod text;
mod tray;
mod ui;
mod watch;

#[cfg(test)]
mod portal_tests;

use state::lock;

use sbarhive_core::config::Config;
use sbarhive_core::ipc::socket_path;
use sbarhive_core::queue::Queue;
use sbarhive_core::transport::{self, Bound, Listener};
use std::process::ExitCode;
use std::sync::{Arc, Mutex, mpsc};

fn workers() -> usize {
    std::thread::available_parallelism()
        .map(|n| (n.get() / 2).max(1))
        .unwrap_or(1)
}

fn main() -> ExitCode {
    let endpoint = transport::endpoint();
    let listener = match Listener::bind(&endpoint) {
        Ok(Bound::Listening(l)) => l,
        Ok(Bound::AlreadyRunning) => {
            eprintln!("sbarhived: already running");
            return ExitCode::SUCCESS;
        }
        Err(e) => {
            eprintln!("sbarhived: cannot listen on {endpoint}: {e}");
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
    let portal_shortcut = lock(&state).config.portal_shortcut;
    portal_bus::start(portal_shortcut, lang, state.clone());

    let result = slint::run_event_loop_until_quit();
    queue.cancel_all();
    let _ = std::fs::remove_file(socket_path());
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("sbarhived: {e}");
            ExitCode::FAILURE
        }
    }
}
