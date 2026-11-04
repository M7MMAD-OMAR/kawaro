//! Daemon protocol: one JSON object per line over a Unix socket at
//! `$XDG_RUNTIME_DIR/kawaro.sock`. Each connection sends one [`Request`]
//! and reads one [`Response`] line, except `Open` and `Run` with
//! `wait = true`, which keep the connection and stream [`Response::Event`]
//! lines until the job finishes.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const SOCKET_NAME: &str = "kawaro.sock";

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
    /// The daemon version, and whether `Run` accepts `params` (S-120). A
    /// client with values to send asks first, so an older daemon cannot
    /// silently drop them.
    Version,
    /// Show the hive for these files. `token` is an xdg-activation token
    /// (Wayland) or startup id (X11) from the caller, if it has one.
    Open {
        files: Vec<PathBuf>,
        #[serde(default)]
        token: Option<String>,
        /// The pointer position in global logical coordinates (S-110,
        /// `kawaro open --at X,Y`): `"at": [X, Y]`. Old clients omit it, and so
        /// does a request without one, keeping the old wire line.
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(default)]
        at: Option<(f64, f64)>,
    },
    /// Show the hive waiting for files dragged onto it (S-014). The files
    /// come with the drop, so the request carries only the activation token.
    Drop {
        #[serde(default)]
        token: Option<String>,
        /// The pointer position in global logical coordinates, as
        /// `hyprctl cursorpos` prints it (S-019 decision 1): `"at": [X, Y]`.
        #[serde(default)]
        at: Option<(f64, f64)>,
        /// The press of a held key (`kawaro drop --hold`, S-019e): the
        /// daemon's long press gate shows the hive after `hold_ms`.
        #[serde(default)]
        hold: bool,
    },
    /// The release of that key (`kawaro drop --release`).
    DropRelease,
    /// Another key went down while the drop key is held (S-102): ends a
    /// pending press that has not shown yet.
    DropOtherKey,
    /// The last hold outcome, for `kawaro doctor` (S-102).
    HoldStatus,
    /// Run an action without showing the hive.
    Run {
        action: String,
        files: Vec<PathBuf>,
        #[serde(default)]
        wait: bool,
        /// Explicit option values for this run only (S-120), resolved by the
        /// daemon onto the action params. Absent means the action's
        /// `[action.params]` as they are.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        params: Option<toml::Table>,
    },
    /// Action ids and labels that match these files, for scripts and tests.
    Actions {
        files: Vec<PathBuf>,
    },
    CancelAll,
    /// Open the settings window, as the tray menu does. `token` is an
    /// xdg-activation token (Wayland) or startup id (X11) from the caller
    /// (S-131), if it has one; old clients omit it.
    Settings {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        token: Option<String>,
    },
    /// Re-read config and action files.
    Reload,
    Quit,
    /// Check for an update now, whatever `update_check` says (S-054b).
    UpdateCheck,
    /// Install the verified update that is ready (S-054b).
    UpdateApply,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum Response {
    Ok,
    /// The answer to `Version` (S-120): the daemon version and whether
    /// `Request::Run` accepts `params`.
    Version {
        version: String,
        run_params: bool,
    },
    Error {
        message: String,
    },
    Actions {
        actions: Vec<ActionSummary>,
    },
    Event {
        event: Event,
    },
    /// The answer to `UpdateCheck`. The JSON key is `state` because the
    /// response is tagged with `status` (S-054b ruling).
    /// The answer to `HoldStatus`: the reason word of the last hold that
    /// ended without a hive and how long ago, if any.
    Hold {
        last: Option<String>,
        ago_ms: Option<u64>,
    },
    Update {
        #[serde(rename = "state")]
        status: String,
        current: String,
        latest: Option<String>,
        /// Download progress, only while `downloading` (S-099).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        percent: Option<u8>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        size: Option<u64>,
        /// The notes of `latest` from the signed manifest.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        notes: Option<crate::update::manifest::Notes>,
    },
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
    /// A task deliberately wrote nothing (S-133).
    TaskSkipped {
        job: u64,
        input: PathBuf,
        reason: crate::engine::SkipReason,
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
        /// Tasks that deliberately wrote nothing (S-133); old lines parse.
        #[serde(default)]
        skipped: usize,
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
            J::TaskDone { job, output, .. } => Event::TaskDone { job, output },
            J::TaskSkipped {
                job, input, reason, ..
            } => Event::TaskSkipped { job, input, reason },
            J::TaskFailed {
                job, input, error, ..
            } => Event::TaskFailed { job, input, error },
            J::Finished {
                job,
                ok,
                failed,
                skipped,
                cancelled,
                ..
            } => Event::Finished {
                job,
                ok,
                failed,
                skipped,
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
            at: None,
        };
        let line = serde_json::to_string(&r).unwrap();
        assert!(!line.contains('\n'));
        assert!(line.contains(r#""cmd":"open""#));
        assert_eq!(serde_json::from_str::<Request>(&line).unwrap(), r);
        let ping: Request = serde_json::from_str(r#"{"cmd":"ping"}"#).unwrap();
        assert_eq!(ping, Request::Ping);
    }

    #[test]
    fn s120_run_params_round_trip_and_stay_off_the_wire_when_absent() {
        let none = Request::Run {
            action: String::from("image.to-webp"),
            files: vec!["/a/b.png".into()],
            wait: false,
            params: None,
        };
        let line = serde_json::to_string(&none).unwrap();
        assert!(!line.contains("params"), "{line}");
        assert_eq!(serde_json::from_str::<Request>(&line).unwrap(), none);

        let mut table = toml::Table::new();
        table.insert(String::from("quality"), toml::Value::Integer(60));
        let with = Request::Run {
            action: String::from("image.to-webp"),
            files: vec!["/a/b.png".into()],
            wait: true,
            params: Some(table),
        };
        let line = serde_json::to_string(&with).unwrap();
        assert!(line.contains(r#""params":{"quality":60}"#), "{line}");
        assert_eq!(serde_json::from_str::<Request>(&line).unwrap(), with);

        // An old client line without the key still parses to no params.
        let old: Request =
            serde_json::from_str(r#"{"cmd":"run","action":"a","files":["/x.png"]}"#).unwrap();
        assert!(matches!(old, Request::Run { params: None, .. }), "{old:?}");
    }

    #[test]
    fn s120_version_request_and_response_shape() {
        assert_eq!(
            serde_json::to_string(&Request::Version).unwrap(),
            r#"{"cmd":"version"}"#
        );
        let answer = Response::Version {
            version: String::from("0.5.0"),
            run_params: true,
        };
        assert_eq!(
            serde_json::to_string(&answer).unwrap(),
            r#"{"status":"version","version":"0.5.0","run_params":true}"#
        );
        assert_eq!(
            serde_json::from_str::<Response>(
                r#"{"status":"version","version":"1","run_params":false}"#
            )
            .unwrap(),
            Response::Version {
                version: String::from("1"),
                run_params: false,
            }
        );
    }
}
