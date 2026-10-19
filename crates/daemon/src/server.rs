//! Socket server: one request per connection, see `sbarhive_core::ipc`.

use crate::state::{Shared, common_kind, lock};
use crate::{notify, text, ui};
use sbarhive_core::ipc::{Request, Response};
use sbarhive_core::queue::Queue;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::Arc;

pub fn serve(listener: UnixListener, state: Shared, queue: Arc<Queue>) {
    for stream in listener.incoming().flatten() {
        let (state, queue) = (state.clone(), queue.clone());
        std::thread::spawn(move || handle(stream, &state, &queue));
    }
}

fn reply(stream: &mut UnixStream, r: &Response) {
    if let Ok(line) = serde_json::to_string(r) {
        let _ = writeln!(stream, "{line}");
    }
}

fn error(stream: &mut UnixStream, message: impl Into<String>) {
    reply(
        stream,
        &Response::Error {
            message: message.into(),
        },
    );
}

fn handle(mut stream: UnixStream, state: &Shared, queue: &Arc<Queue>) {
    let mut line = String::new();
    if BufReader::new(&stream).read_line(&mut line).is_err() {
        return;
    }
    let req: Request = match serde_json::from_str(&line) {
        Ok(r) => r,
        Err(e) => return error(&mut stream, format!("bad request: {e}")),
    };
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
            st.pending = Some(files);
            drop(st);
            ui::present(centre, cells, token);
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
        Request::Reload => {
            lock(state).reload();
            reply(&mut stream, &Response::Ok);
        }
        Request::Quit => {
            reply(&mut stream, &Response::Ok);
            ui::quit();
        }
    }
}
