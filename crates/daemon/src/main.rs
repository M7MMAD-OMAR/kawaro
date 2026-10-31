#![cfg_attr(windows, windows_subsystem = "windows")]
//! `kawarod`: the Kawaro background service. Owns the hive window, the
//! job queue, the tray icon and the socket in `$XDG_RUNTIME_DIR`.

mod args;
// Only the Windows shell glue pushes through it; its tests run everywhere.
#[cfg_attr(not(windows), allow(dead_code))]
mod batch;
#[cfg_attr(not(windows), allow(dead_code))]
mod foreground_logic;
mod jobs;
#[cfg(windows)]
mod logfile;
mod notify;
mod notify_logic;
mod plan;
// Only the D-Bus side (portal_bus) uses it, and that is Unix only.
#[cfg_attr(not(unix), allow(dead_code))]
mod portal;
#[cfg(unix)]
mod portal_bus;
mod progress;
mod registration;
mod server;
#[cfg(windows)]
mod shell;
// Its tests run everywhere; only the Windows shell uses it.
#[cfg_attr(not(windows), allow(dead_code))]
mod shell_logic;
#[cfg(windows)]
mod shell_open;
mod state;
// The tray menu text is used by the Windows tray from S-046 on.
mod reopen;
mod reopen_logic;
#[cfg_attr(windows, allow(dead_code))]
mod text;
mod tray;
mod ui;
mod update;
mod update_sys;
mod watch;

#[cfg(test)]
mod args_tests;
#[cfg(test)]
mod batch_tests;
#[cfg(test)]
mod portal_tests;
#[cfg(test)]
mod s049_notify_tests;
#[cfg(test)]
mod s067_tests;
#[cfg(test)]
mod s084_drag_tests;
#[cfg(test)]
mod s087_welcome_tests;
#[cfg(test)]
mod s094b_tests;
#[cfg(test)]
mod s096_tests;
#[cfg(test)]
mod s108_tests;
#[cfg(test)]
mod s108b_tests;
#[cfg(test)]
mod server_tests;
#[cfg(test)]
mod shell_logic_tests;
#[cfg(test)]
mod update_tests;

use state::lock;

use kawaro_core::config::Config;
use kawaro_core::ipc::Request;
use kawaro_core::queue::Queue;
use kawaro_core::transport::{self, Bound, Listener};
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
    // `open` and `settings` talk to a running service when there is one
    // (S-046 AC6, S-086 AC1).
    let (open_files, open_settings) = match mode {
        args::Mode::Open(files) => {
            if let Some(code) = send_open(&endpoint, &files) {
                return code;
            }
            (
                Some(files.into_iter().map(PathBuf::from).collect::<Vec<_>>()),
                false,
            )
        }
        args::Mode::Settings => {
            if let Some(code) = send_settings(&endpoint) {
                return code;
            }
            (None, true)
        }
        args::Mode::Daemon => (None, false),
    };
    // A daemon the updater started waits for the old one to free the socket.
    let bound =
        update_sys::wait_for_old_daemon(Listener::bind(&endpoint), || Listener::bind(&endpoint));
    let listener = match bound {
        Ok(Bound::Listening(l)) => l,
        Ok(Bound::AlreadyRunning) => {
            eprintln!("kawarod: already running");
            return ExitCode::SUCCESS;
        }
        Err(e) => {
            eprintln!("kawarod: cannot listen on {endpoint}: {e}");
            return ExitCode::FAILURE;
        }
    };
    // Removes the socket file on every return below.
    let _cleanup = listener.cleanup();
    // S-085 D3: the bilingual Start shortcut and app name on Windows, best
    // effort; a no-op elsewhere.
    kawaro_core::identity::sync();
    // S-106 D2: old downloads and helper copies.
    update_sys::cleanup_at_start();

    let (config, warning) = Config::load();
    if let Some(w) = warning {
        eprintln!("kawarod: settings: {w}");
    }
    let lang = config.lang();
    if let Err(e) = kawaro_ui::init() {
        eprintln!("kawarod: no display: {e}");
        return ExitCode::FAILURE;
    }
    // Matches the desktop file, so compositors, window rules and portals
    // can tell Kawaro windows apart from any other.
    if let Err(e) = slint::set_xdg_app_id("io.github.m7mmad_omar.Kawaro") {
        eprintln!("kawarod: app id: {e}");
    }
    let engines = kawaro_engines::all();
    let (ev_tx, ev_rx) = mpsc::channel();
    let queue = Arc::new(Queue::new(engines.clone(), workers(), ev_tx));
    let state: state::Shared = Arc::new(Mutex::new(state::State::new(config, &engines)));

    let theme = lock(&state).config.theme;
    if let Err(e) = ui::build(lang, theme, &state, &queue) {
        eprintln!("kawarod: cannot create the windows: {e}");
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
    // After the bind: the thread sleeps 30 s before its first check.
    update::start(state.clone());
    // No service answered: this process is the service and shows the hive
    // for the files it was started with, or the settings window when it was
    // started as `kawarod settings` (S-086 AC1).
    if let Some(files) = open_files
        && let Err(e) = server::open(&state, files, None)
    {
        eprintln!("kawarod: {e}");
    }
    if open_settings {
        server::settings(&state);
    }
    // After an update the user started from the settings window, the new
    // daemon reopens it on the Updates page (S-099 B5).
    if let Some(dir) = Config::path().and_then(|p| p.parent().map(std::path::Path::to_path_buf)) {
        let config = lock(&state).config.clone();
        reopen::on_start(
            &dir,
            env!("CARGO_PKG_VERSION"),
            update_sys::unix_now(),
            &config,
        );
    }
    #[cfg(unix)]
    {
        let portal_shortcut = lock(&state).config.portal_shortcut;
        portal_bus::start(portal_shortcut, lang, state.clone());
    }

    let result = slint::run_event_loop_until_quit();
    queue.cancel_all();
    #[cfg(windows)]
    tray::stop();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("kawarod: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Sends one `open` line to the service on `endpoint` and waits for its
/// reply. `None` when no service answers, so the caller becomes the service.
fn send_open(endpoint: &transport::Endpoint, files: &[String]) -> Option<ExitCode> {
    let request = Request::Open {
        files: files.iter().map(PathBuf::from).collect(),
        token: None,
    };
    send_request(endpoint, &request)
}

/// Sends one `settings` request to the service on `endpoint` (S-086 AC1),
/// the same `Request::Settings` path `kawaro settings` uses, and waits for
/// its reply. `None` when no service answers, so the caller becomes the
/// service and opens the window itself.
fn send_settings(endpoint: &transport::Endpoint) -> Option<ExitCode> {
    send_request(endpoint, &Request::Settings)
}

/// Sends one request line to the service on `endpoint` and waits for its
/// reply. `None` when no service answers, so the caller becomes the service.
fn send_request(endpoint: &transport::Endpoint, request: &Request) -> Option<ExitCode> {
    use kawaro_core::ipc::Response;
    use std::io::{BufRead, BufReader, Write};
    let mut stream = transport::Stream::connect(endpoint).ok()?;
    #[cfg(windows)]
    shell_open::allow_foreground();
    let line = serde_json::to_string(request).ok()?;
    if writeln!(stream, "{line}").is_err() {
        eprintln!("kawarod: cannot send the request");
        return Some(ExitCode::FAILURE);
    }
    let mut reply = String::new();
    let read = BufReader::new(stream).read_line(&mut reply);
    match (read, serde_json::from_str::<Response>(&reply)) {
        (Ok(_), Ok(Response::Error { message })) => {
            eprintln!("kawarod: {message}");
            Some(ExitCode::FAILURE)
        }
        (Ok(_), Ok(_)) => Some(ExitCode::SUCCESS),
        _ => {
            eprintln!("kawarod: no reply from the running service");
            Some(ExitCode::FAILURE)
        }
    }
}

/// The service has no console on Windows: stderr goes to the log file.
#[cfg(windows)]
fn log_to_file() {
    let Some(path) = kawaro_core::paths::log_file() else {
        return;
    };
    if let Ok(file) = logfile::open_append(&path) {
        let _ = logfile::redirect_stderr(file);
    }
}
