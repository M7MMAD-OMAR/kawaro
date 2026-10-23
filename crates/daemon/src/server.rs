//! Socket server: one request per connection, see `sbarhive_core::ipc`.

use crate::state::{Shared, common_kind, lock};
use crate::{notify, text, ui};
use sbarhive_core::ipc::{Request, Response};
use sbarhive_core::queue::Queue;
use sbarhive_core::transport::{Listener, Stream, accept_backoff};
use std::io::{self, BufRead, BufReader, Read, Write};
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

fn handle(mut stream: Stream, state: &Shared, queue: &Arc<Queue>) {
    let Ok(line) = read_request(&mut stream) else {
        return;
    };
    let req: Request = match serde_json::from_str(&line) {
        Ok(r) => r,
        Err(e) => return error(&mut stream, format!("bad request: {e}")),
    };
    // The held drop key (S-019e): the gate on the event loop decides when
    // the hive shows, so the client returns at once.
    if let Some(signal) = sbarhive_core::hold::from_request(&req) {
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
        Request::Open { files, token } => {
            let mut st = lock(state);
            let actions = st.matching(&files);
            if actions.is_empty() {
                let lang = st.lang;
                drop(st);
                notify::info(lang, text::no_actions(lang));
                return error(&mut stream, text::no_actions(lang));
            }
            let lang = st.lang;
            let centre = text::selection(lang, files.len(), common_kind(&files));
            let cells = actions
                .iter()
                .map(|a| ui::cell(a, lang))
                .collect::<Vec<_>>();
            st.pending = Some(files.clone());
            drop(st);
            ui::present(centre, files, cells, token);
            reply(&mut stream, &Response::Ok);
        }
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
            let config = lock(state).config.clone();
            ui::open_settings(config);
            reply(&mut stream, &Response::Ok);
        }
        Request::Reload => {
            lock(state).reload();
            reply(&mut stream, &Response::Ok);
        }
        // Taken by the gate above.
        Request::DropRelease => reply(&mut stream, &Response::Ok),
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
        use sbarhive_core::transport::{Bound, Endpoint, Listener, Stream};
        let dir = tempfile::tempdir().unwrap();
        let e = Endpoint::from_path(dir.path().join("sbarhive.sock"));
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
