//! Socket server: one request per connection, see `kawaro_core::ipc`.

use crate::state::{Shared, common_kind, lock};
use crate::{notify, text, ui};
use kawaro_core::ipc::{Request, Response};
use kawaro_core::queue::Queue;
use kawaro_core::transport::{Listener, Stream, accept_backoff};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

pub fn serve(listener: Listener, state: Shared, queue: Arc<Queue>) {
    // Failed accepts in a row; each one pauses, so an error does not spin.
    let mut errors: u32 = 0;
    loop {
        match listener.accept() {
            Ok(stream) => {
                errors = 0;
                let (state, queue) = (state.clone(), queue.clone());
                std::thread::spawn(move || handle(stream, &state, &queue));
            }
            Err(_) => {
                errors = errors.saturating_add(1);
                std::thread::sleep(accept_backoff(errors));
            }
        }
    }
}

/// The log line for a refused `SetForegroundWindow` (S-046 AC4); nothing
/// when the call worked.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn focus_log(ok: bool) -> Option<&'static str> {
    (!ok).then_some("kawarod: foreground refused")
}

/// Opens the Settings window for the current config (S-086 AC1): the one
/// path behind `Request::Settings`, `kawarod settings` and `kawaro settings`.
/// S-087 AC1 and D1: with no `welcome_seen` marker the same entry opens the
/// first-run welcome window instead.
pub fn settings(state: &Shared) {
    let config = lock(state).config.clone();
    match kawaro_core::welcome::route(&config) {
        kawaro_core::welcome::Route::Welcome => ui::present_welcome(state),
        kawaro_core::welcome::Route::Settings => ui::open_settings(config),
    }
}

/// The S-021 rule on Windows (S-047 AC6, S-084 AC3): a drop request shows
/// the drop hive only during a real drag, that is with the trigger state of
/// a shipped mechanism (`trigger_down`): the drag button of the S-084
/// right-button drag (the manager decision D3), or the primary button of a
/// key hold. True when a request must be refused: a drop with no drag.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn drop_refused(is_drop: bool, trigger_down: bool) -> bool {
    is_drop && !kawaro_ui::drop_hive_allowed(kawaro_ui::Source::Shortcut, trigger_down)
}

/// Shows the hive for `files`. On Windows Explorer sends one file per
/// process, so the files of requests closer than the merge window share one
/// hive (S-046 AC8).
pub fn open(state: &Shared, files: Vec<PathBuf>, token: Option<String>) -> Result<(), String> {
    #[cfg(windows)]
    let (files, actions) = crate::shell_open::selection(state, files);
    let mut st = lock(state);
    #[cfg(not(windows))]
    let actions = st.matching(&files);
    let lang = st.lang;
    if actions.is_empty() {
        drop(st);
        notify::info(lang, text::no_actions(lang));
        return Err(text::no_actions(lang).to_string());
    }
    let centre = text::selection(lang, files.len(), common_kind(&files));
    let cells = actions
        .iter()
        .map(|a| ui::cell(a, lang))
        .collect::<Vec<_>>();
    st.pending = Some(files.clone());
    drop(st);
    ui::present(centre, files, cells, token);
    Ok(())
}

/// Reads the one request line from the stream itself, so the same stream
/// writes the reply.
fn read_request(stream: &mut impl Read) -> io::Result<String> {
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line)?;
    Ok(line)
}

fn reply(stream: &mut Stream, r: &Response) {
    if let Ok(line) = serde_json::to_string(r) {
        let _ = writeln!(stream, "{line}");
    }
}

fn error(stream: &mut Stream, message: impl Into<String>) {
    reply(
        stream,
        &Response::Error {
            message: message.into(),
        },
    );
}

/// One request per connection. `pub(crate)` for the S-086 routing tests,
/// which feed it from a fake listener.
pub(crate) fn handle(mut stream: Stream, state: &Shared, queue: &Arc<Queue>) {
    let Ok(line) = read_request(&mut stream) else {
        return;
    };
    let req: Request = match serde_json::from_str(&line) {
        Ok(r) => r,
        Err(e) => return error(&mut stream, format!("bad request: {e}")),
    };
    // A drop request on Windows shows a hive only during a real drag of a
    // shipped trigger (S-021, S-084 AC3): the primary button of a key hold
    // or the drag button of the right-button drag.
    #[cfg(windows)]
    if drop_refused(
        matches!(req, Request::Drop { .. }),
        crate::shell::primary_button_down() || crate::shell::drag_button_down(),
    ) {
        return reply(&mut stream, &Response::Ok);
    }
    // The held drop key (S-019e): the gate on the event loop decides when
    // the hive shows, so the client returns at once.
    if let Some(signal) = kawaro_core::hold::from_request(&req) {
        let token = if let Request::Drop { token, .. } = req {
            token
        } else {
            None
        };
        ui::hold_signal(signal, token, Instant::now(), state.clone());
        return reply(&mut stream, &Response::Ok);
    }
    match req {
        Request::Ping => reply(&mut stream, &Response::Ok),
        Request::Open { files, token } => match open(state, files, token) {
            Ok(()) => reply(&mut stream, &Response::Ok),
            Err(message) => error(&mut stream, message),
        },
        Request::Drop { token, at, .. } => {
            // The files come with the drag, when it enters the hive.
            let lang = {
                let mut st = lock(state);
                st.pending = None;
                st.lang
            };
            ui::present_drop(text::drop_waiting(lang), token, at);
            reply(&mut stream, &Response::Ok);
        }
        Request::Run {
            action,
            files,
            wait,
        } => {
            let mut st = lock(state);
            let Some(def) = st.registry.get(&action).cloned() else {
                return error(&mut stream, format!("unknown action `{action}`"));
            };
            // Holding the lock while submitting keeps the pump from publishing
            // the first events before this client is subscribed.
            let job = crate::jobs::submit(&mut st, queue, def, files);
            if wait {
                if let Ok(s) = stream.try_clone() {
                    st.subscribers.entry(job).or_default().push(s);
                }
            } else {
                reply(&mut stream, &Response::Ok);
            }
        }
        Request::Actions { files } => {
            let actions = lock(state).summaries(&files);
            reply(&mut stream, &Response::Actions { actions });
        }
        Request::CancelAll => {
            queue.cancel_all();
            reply(&mut stream, &Response::Ok);
        }
        Request::Settings => {
            settings(state);
            reply(&mut stream, &Response::Ok);
        }
        Request::Reload => {
            lock(state).reload();
            reply(&mut stream, &Response::Ok);
        }
        // Taken by the gate above.
        Request::DropRelease => reply(&mut stream, &Response::Ok),
        Request::UpdateCheck => {
            let (config, jobs, hive_open) = crate::update_sys::snapshot(state);
            let lang = lock(state).lang;
            // A download holds the update lock; answer `downloading` at once.
            let mut guard = crate::update_sys::shared().try_lock().ok();
            if let Some(update) = guard.as_mut() {
                update.running_jobs = jobs;
                update.hive_open = hive_open;
            }
            let (response, effects) = crate::update::check_or_busy(
                // `check_now(` or busy
                guard.as_deref_mut(),
                env!("CARGO_PKG_VERSION"),
                &config,
                &kawaro_core::update::fetch::CurlFetcher::system(),
                crate::update_sys::unix_now(),
            );
            drop(guard);
            reply(&mut stream, &response);
            crate::tray::refresh();
            crate::update_sys::run_effects(effects, lang);
        }
        Request::UpdateApply => {
            let (_, jobs, hive_open) = crate::update_sys::snapshot(state);
            let lang = lock(state).lang;
            let mut guard = crate::update_sys::shared().try_lock().ok();
            if let Some(update) = guard.as_mut() {
                update.running_jobs = jobs;
                update.hive_open = hive_open;
            }
            // `request_apply(` when the lock is free, the busy answer otherwise.
            let (response, effects) = crate::update::request_apply_or_busy(guard.as_deref_mut());
            drop(guard);
            reply(&mut stream, &response);
            crate::update_sys::run_effects(effects, lang);
        }
        Request::Quit => {
            reply(&mut stream, &Response::Ok);
            ui::quit();
        }
    }
}

#[cfg(test)]
mod tests {
    // S-028b AC6 (backlog/S-028-review-fixes.md): `read_request` reads one
    // line from the stream itself, so the same stream writes the reply.
    use super::read_request;
    use std::io::{BufRead, BufReader, Cursor, Write};

    #[test]
    fn s028_ac6_read_request_reads_the_first_line_only() {
        let mut c = Cursor::new(b"{\"cmd\":\"ping\"}\n{\"x\":1}\n".to_vec());
        let line = read_request(&mut c).unwrap();
        assert_eq!(line.trim_end(), "{\"cmd\":\"ping\"}");
        assert!(
            !line.contains("\"x\""),
            "the second line is not read: {line}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn s028_ac6_the_same_stream_writes_the_reply_after_read_request() {
        use kawaro_core::transport::{Bound, Endpoint, Listener, Stream};
        let dir = tempfile::tempdir().unwrap();
        let e = Endpoint::from_path(dir.path().join("kawaro.sock"));
        let listener = match Listener::bind(&e).unwrap() {
            Bound::Listening(l) => l,
            Bound::AlreadyRunning => panic!("nothing listens yet, bind must listen"),
        };
        let mut client = Stream::connect(&e).unwrap();
        let mut server = listener.accept().unwrap();
        client.write_all(b"{\"cmd\":\"ping\"}\n").unwrap();
        let line = read_request(&mut server).unwrap();
        assert_eq!(line.trim_end(), "{\"cmd\":\"ping\"}");
        server.write_all(b"{\"status\":\"ok\"}\n").unwrap();
        let mut reply = String::new();
        BufReader::new(&mut client).read_line(&mut reply).unwrap();
        assert_eq!(reply, "{\"status\":\"ok\"}\n");
    }
}
