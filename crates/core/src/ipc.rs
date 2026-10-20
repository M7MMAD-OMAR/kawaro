//! Daemon protocol: one JSON object per line over a Unix socket at
//! `$XDG_RUNTIME_DIR/sbarhive.sock`. Each connection sends one [`Request`]
//! and reads one [`Response`] line, except `Open` and `Run` with
//! `wait = true`, which keep the connection and stream [`Response::Event`]
//! lines until the job finishes.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const SOCKET_NAME: &str = "sbarhive.sock";

pub fn socket_path() -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    dir.join(SOCKET_NAME)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "kebab-case")]
pub enum Request {
    Ping,
    /// Show the hive for these files. `token` is an xdg-activation token
    /// (Wayland) or startup id (X11) from the caller, if it has one.
    Open {
        files: Vec<PathBuf>,
        #[serde(default)]
        token: Option<String>,
    },
    /// Show the hive waiting for files dragged onto it (S-014). The files
    /// come with the drop, so the request carries only the activation token.
    Drop {
        #[serde(default)]
        token: Option<String>,
    },
    /// Run an action without showing the hive.
    Run {
        action: String,
        files: Vec<PathBuf>,
        #[serde(default)]
        wait: bool,
    },
    /// Action ids and labels that match these files, for scripts and tests.
    Actions {
        files: Vec<PathBuf>,
    },
    CancelAll,
    /// Open the settings window, as the tray menu does.
    Settings,
    /// Re-read config and action files.
    Reload,
    Quit,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum Response {
    Ok,
    Error { message: String },
    Actions { actions: Vec<ActionSummary> },
    Event { event: Event },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActionSummary {
    pub id: String,
    pub ring: u8,
    pub code: String,
    pub label: String,
}

/// Serializable mirror of [`crate::queue::JobEvent`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Event {
    Started {
        job: u64,
        action_id: String,
        tasks: usize,
    },
    Progress {
        job: u64,
        fraction: f32,
    },
    TaskDone {
        job: u64,
        output: PathBuf,
    },
    TaskFailed {
        job: u64,
        input: PathBuf,
        error: String,
    },
    Finished {
        job: u64,
        ok: usize,
        failed: usize,
        cancelled: bool,
    },
}

impl From<crate::queue::JobEvent> for Event {
    fn from(e: crate::queue::JobEvent) -> Self {
        use crate::queue::JobEvent as J;
        match e {
            J::Started {
                job,
                action_id,
                tasks,
            } => Event::Started {
                job,
                action_id,
                tasks,
            },
            J::Progress { job, fraction } => Event::Progress { job, fraction },
            J::TaskDone { job, output } => Event::TaskDone { job, output },
            J::TaskFailed { job, input, error } => Event::TaskFailed { job, input, error },
            J::Finished {
                job,
                ok,
                failed,
                cancelled,
            } => Event::Finished {
                job,
                ok,
                failed,
                cancelled,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_round_trip_as_single_json_lines() {
        let r = Request::Open {
            files: vec!["/a b/c.png".into()],
            token: Some("t".into()),
        };
        let line = serde_json::to_string(&r).unwrap();
        assert!(!line.contains('\n'));
        assert!(line.contains(r#""cmd":"open""#));
        assert_eq!(serde_json::from_str::<Request>(&line).unwrap(), r);
        let ping: Request = serde_json::from_str(r#"{"cmd":"ping"}"#).unwrap();
        assert_eq!(ping, Request::Ping);
    }
}
