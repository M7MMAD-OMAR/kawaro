//! `sbarhived`: the SbarHive background service. Owns the hive window, the
//! job queue, the tray icon and the socket in `$XDG_RUNTIME_DIR`.

mod jobs;
mod notify;
mod server;
mod state;
mod text;
mod tray;
mod ui;

use sbarhive_core::i18n::Lang;
use sbarhive_core::ipc::{Request, Response, socket_path};
use sbarhive_core::queue::Queue;
use state::lock;
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

    let lang = Lang::detect();
    if let Err(e) = sbarhive_ui::init() {
        eprintln!("sbarhived: no display: {e}");
        return ExitCode::FAILURE;
    }
    let hive = match sbarhive_ui::Hive::new(lang) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("sbarhived: cannot create the hive: {e}");
            return ExitCode::FAILURE;
        }
    };

    let engines = sbarhive_engines::all();
    let (ev_tx, ev_rx) = mpsc::channel();
    let queue = Arc::new(Queue::new(engines.clone(), workers(), ev_tx));
    let state: state::Shared = Arc::new(Mutex::new(state::State::new(lang, &engines)));

    {
        let (state, queue) = (state.clone(), queue.clone());
        hive.on_choose(move |id| {
            let mut st = lock(&state);
            let files = st.pending.take().unwrap_or_default();
            match st.registry.get(&id).cloned() {
                Some(action) if !files.is_empty() => {
                    jobs::submit(&mut st, &queue, action, files);
                }
                _ => eprintln!("sbarhived: nothing to run for `{id}`"),
            }
        });
    }
    {
        let state = state.clone();
        hive.on_dismiss(move || lock(&state).pending = None);
    }
    ui::install(hive);

    let tray = tray::start(lang, queue.clone());
    {
        let state = state.clone();
        std::thread::spawn(move || jobs::pump(ev_rx, state, tray));
    }
    {
        let (state, queue) = (state.clone(), queue.clone());
        std::thread::spawn(move || server::serve(listener, state, queue));
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
