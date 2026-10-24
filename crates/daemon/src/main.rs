#![cfg_attr(windows, windows_subsystem = "windows")]
//! `sbarhived`: the SbarHive background service. Owns the hive window, the
//! job queue, the tray icon and the socket in `$XDG_RUNTIME_DIR`.

mod args;
// Only the Windows shell glue pushes through it; its tests run everywhere.
#[cfg_attr(not(windows), allow(dead_code))]
mod batch;
mod jobs;
#[cfg(windows)]
mod logfile;
mod notify;
// Only the D-Bus side (portal_bus) uses it, and that is Unix only.
#[cfg_attr(not(unix), allow(dead_code))]
mod portal;
#[cfg(unix)]
mod portal_bus;
mod server;
#[cfg(windows)]
mod shell_open;
mod state;
// The tray menu text is used by the Windows tray from S-046 on.
#[cfg_attr(windows, allow(dead_code))]
mod text;
mod tray;
mod ui;
mod watch;

#[cfg(test)]
mod args_tests;
// The tester's header comment is a list rustdoc wants indented.
#[cfg(test)]
#[allow(clippy::doc_lazy_continuation)]
mod batch_tests;
#[cfg(test)]
mod portal_tests;

use state::lock;

use sbarhive_core::config::Config;
use sbarhive_core::queue::Queue;
use sbarhive_core::transport::{self, Bound, Listener};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Mutex, mpsc};

fn workers() -> usize {
    std::thread::available_parallelism()
        .map(|n| (n.get() / 2).max(1))
        .unwrap_or(1)
}

fn main() -> ExitCode {
    #[cfg(windows)]
    log_to_file();
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let mode = match args::parse(&argv) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let endpoint = transport::endpoint();
    // `open` talks to a running service when there is one (S-046 AC6).
    let open_files = match mode {
        args::Mode::Open(files) => {
            if let Some(code) = send_open(&endpoint, &files) {
                return code;
            }
            Some(files.into_iter().map(PathBuf::from).collect::<Vec<_>>())
        }
        args::Mode::Daemon => None,
    };
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
    // Removes the socket file on every return below.
    let _cleanup = listener.cleanup();

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
    // No service answered: this process is the service and shows the hive
    // for the files it was started with.
    if let Some(files) = open_files
        && let Err(e) = server::open(&state, files, None)
    {
        eprintln!("sbarhived: {e}");
    }
    #[cfg(unix)]
    {
        let portal_shortcut = lock(&state).config.portal_shortcut;
        portal_bus::start(portal_shortcut, lang, state.clone());
    }

    let result = slint::run_event_loop_until_quit();
    queue.cancel_all();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("sbarhived: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Sends one `open` line to the service on `endpoint` and waits for its
/// reply. `None` when no service answers, so the caller becomes the service.
fn send_open(endpoint: &transport::Endpoint, files: &[String]) -> Option<ExitCode> {
    use sbarhive_core::ipc::{Request, Response};
    use std::io::{BufRead, BufReader, Write};
    let mut stream = transport::Stream::connect(endpoint).ok()?;
    #[cfg(windows)]
    shell_open::allow_foreground();
    let request = Request::Open {
        files: files.iter().map(PathBuf::from).collect(),
        token: None,
    };
    let line = serde_json::to_string(&request).ok()?;
    if writeln!(stream, "{line}").is_err() {
        eprintln!("sbarhived: cannot send the request");
        return Some(ExitCode::FAILURE);
    }
    let mut reply = String::new();
    let read = BufReader::new(stream).read_line(&mut reply);
    match (read, serde_json::from_str::<Response>(&reply)) {
        (Ok(_), Ok(Response::Error { message })) => {
            eprintln!("sbarhived: {message}");
            Some(ExitCode::FAILURE)
        }
        (Ok(_), Ok(_)) => Some(ExitCode::SUCCESS),
        _ => {
            eprintln!("sbarhived: no reply from the running service");
            Some(ExitCode::FAILURE)
        }
    }
}

/// The service has no console on Windows: stderr goes to the log file.
#[cfg(windows)]
fn log_to_file() {
    let Some(path) = sbarhive_core::paths::log_file() else {
        return;
    };
    if let Ok(file) = logfile::open_append(&path) {
        let _ = logfile::redirect_stderr(file);
    }
}
