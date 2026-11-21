//! Daemon protocol: one JSON object per line over a Unix socket at
//! `$XDG_RUNTIME_DIR/kawaro.sock`. Each connection sends one [`Request`]
//! and reads one [`Response`] line, except `Open` and `Run` with
//! `wait = true`, which keep the connection and stream [`Response::Event`]
//! lines until the job finishes.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const SOCKET_NAME: &str = "kawaro.sock";

/// The path of the daemon socket.
///
/// It is `$XDG_RUNTIME_DIR/kawaro.sock` when the variable is set. Without
/// it the socket lives in a private directory of this user,
/// `temp_dir()/kawaro-<uid>/kawaro.sock`, created with mode 0700 when it is
/// missing, so no other user can reach the socket (S-151).
pub fn socket_path() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        return PathBuf::from(dir).join(SOCKET_NAME);
    }
    fallback_dir().join(SOCKET_NAME)
}

/// The private directory of the socket when `XDG_RUNTIME_DIR` is not set.
#[cfg(unix)]
fn fallback_dir() -> PathBuf {
    let dir = match current_uid() {
        Some(uid) => std::env::temp_dir().join(format!("kawaro-{uid}")),
        None => std::env::temp_dir().join("kawaro"),
    };
    ensure_private_dir(&dir);
    dir
}

/// The private directory of the socket when `XDG_RUNTIME_DIR` is not set.
///
/// Windows has a named pipe per user and session, so it never needs a
/// directory.
#[cfg(not(unix))]
fn fallback_dir() -> PathBuf {
    std::env::temp_dir()
}

/// The directory of the socket, created with mode 0700 when it is missing.
///
/// An existing directory is closed again when this user owns it, so an old
/// wide mode cannot leave the socket reachable for others.
#[cfg(unix)]
fn ensure_private_dir(dir: &std::path::Path) {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
    let private = std::fs::Permissions::from_mode(0o700);
    let mut builder = std::fs::DirBuilder::new();
    builder.mode(0o700);
    if builder.create(dir).is_ok() {
        let _ = std::fs::set_permissions(dir, private);
        return;
    }
    let Ok(meta) = std::fs::symlink_metadata(dir) else {
        return;
    };
    if meta.file_type().is_dir() && current_uid().is_some_and(|uid| meta.uid() == uid) {
        let _ = std::fs::set_permissions(dir, private);
    }
}

/// The effective user id of this process.
///
/// Linux and every system with a `/proc`: the `Uid:` line of
/// `/proc/self/status`, the same source the update path uses. The other Unix
/// systems have no `/proc`, so the owner of a file this process creates is
/// read instead: the kernel gives that file to the effective user id.
#[cfg(unix)]
pub(crate) fn current_uid() -> Option<u32> {
    if let Some(uid) = std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| crate::update::apply::effective_uid(&status))
    {
        return Some(uid);
    }
    probe_uid()
}

/// The owner of a file this process just created: its effective user id.
#[cfg(unix)]
fn probe_uid() -> Option<u32> {
    use std::os::unix::fs::MetadataExt;
    let path = std::env::temp_dir().join(format!(
        ".kawaro-uid-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or_default()
    ));
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .ok()?;
    let uid = std::fs::metadata(&path).ok().map(|meta| meta.uid());
    drop(file);
    let _ = std::fs::remove_file(&path);
    uid
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
    /// The jobs of this session, one bounded summary each (S-144-C). The
    /// history is session memory: a job that left the ledger is unknown, and
    /// nothing is read from disk.
    Jobs,
    /// The detail of one job (S-144-C): its units, their input paths, their
    /// terminal state and their output. A job with no session record answers
    /// `Unknown`, never a fabricated finished job.
    JobResults {
        job: u64,
        /// The daemon instance the reference was taken from (S-148 AC3). It
        /// is absent in a request of a client that names the daemon that
        /// answers, and it is refused when it belongs to an earlier daemon
        /// lifecycle, so a job number reused after a restart is never taken
        /// for the current one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        instance: Option<String>,
    },
    /// Submit a retry of a finished job (S-144-C). The daemon checks the
    /// stored action, its options and the current inputs, then submits the
    /// stored action through the normal queue path. A retry that is already
    /// pending is refused, so a double trigger never starts two children.
    RetryJob {
        job: u64,
        /// The daemon instance the reference was taken from (S-148 AC3).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        instance: Option<String>,
    },
    /// Forget the completed jobs of this session (S-144-C). Active jobs keep
    /// running, no output file is touched, and nothing is replayed.
    ClearJobResults,
    /// Cancel one job of the queue (S-144-C). The queue already cancels a
    /// single job; this is the explicit entry the CLI uses.
    CancelJob {
        job: u64,
        /// The daemon instance the reference was taken from (S-148 AC3).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        instance: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum Response {
    Ok,
    /// The answer to `Version` (S-120): the daemon version and whether
    /// `Request::Run` accepts `params`.
    ///
    /// `job_results` (S-144-C) says the daemon answers the job results
    /// commands. A field an older daemon does not send is absent and reads
    /// as false, and a false value is not written, so the old reply keeps
    /// its exact published shape.
    ///
    /// `instance` (S-148 AC3) is the identity of the daemon process. A
    /// daemon older than this one does not send it, and it reads as absent.
    Version {
        version: String,
        run_params: bool,
        #[serde(default, skip_serializing_if = "is_false")]
        job_results: bool,
        /// The identity of the daemon process (S-148 AC3). A client that
        /// checks a job reference needs it, and an old daemon has none.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        instance: Option<String>,
    },
    /// The answer to `Jobs`: one bounded summary per job, the running ones
    /// first, then the queued ones, then the finished ones.
    Jobs {
        jobs: Vec<crate::job_report::JobSummary>,
        /// The identity of the daemon process (S-148 AC3), so a client can
        /// keep the reference of a job it reads now.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        instance: Option<String>,
    },
    /// The answer to `JobResults`: the bounded detail of that job.
    JobResults {
        job: crate::job_report::JobDetail,
    },
    /// The answer to `RetryJob`: the new child job of the retry.
    Retry {
        job: u64,
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

/// The `skip_serializing_if` of the `job_results` capability: an older daemon
/// never wrote the field, so a false value is not written either.
fn is_false(value: &bool) -> bool {
    !*value
}

/// Serializable mirror of [`crate::queue::JobEvent`].
///
/// The wire shape is exactly what it was. The internal fields of the queue
/// events (the manifest of a started job and the unit ordinal of a task)
/// are dropped here and never reach the wire.
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
                ..
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
            job_results: true,
            instance: None,
        };
        assert_eq!(
            serde_json::to_string(&answer).unwrap(),
            r#"{"status":"version","version":"0.5.0","run_params":true,"job_results":true}"#
        );
        assert_eq!(
            serde_json::from_str::<Response>(
                r#"{"status":"version","version":"1","run_params":false}"#
            )
            .unwrap(),
            Response::Version {
                version: String::from("1"),
                run_params: false,
                job_results: false,
                instance: None,
            }
        );
    }

    // S-144-C: an old daemon never wrote the capability, so its exact reply
    // still parses, reads as false and is written back unchanged.
    #[test]
    fn s144c_an_old_version_reply_keeps_its_exact_shape() {
        let old = r#"{"status":"version","version":"0.5.0","run_params":true}"#;
        let answer: Response = serde_json::from_str(old).unwrap();
        assert_eq!(
            answer,
            Response::Version {
                version: String::from("0.5.0"),
                run_params: true,
                job_results: false,
                instance: None,
            }
        );
        assert_eq!(serde_json::to_string(&answer).unwrap(), old);
    }

    #[test]
    fn s144c_the_job_results_requests_round_trip() {
        let cases = [
            (Request::Jobs, r#"{"cmd":"jobs"}"#),
            (
                Request::JobResults {
                    job: 7,
                    instance: None,
                },
                r#"{"cmd":"job-results","job":7}"#,
            ),
            (
                Request::RetryJob {
                    job: 7,
                    instance: None,
                },
                r#"{"cmd":"retry-job","job":7}"#,
            ),
            (Request::ClearJobResults, r#"{"cmd":"clear-job-results"}"#),
            (
                Request::CancelJob {
                    job: 7,
                    instance: None,
                },
                r#"{"cmd":"cancel-job","job":7}"#,
            ),
        ];
        for (request, line) in cases {
            assert_eq!(serde_json::to_string(&request).unwrap(), line);
            assert_eq!(serde_json::from_str::<Request>(line).unwrap(), request);
        }
    }

    // S-148 AC3: a job reference may carry the daemon instance it was taken
    // from. A request without one keeps the old wire line, and an old line
    // without the field still parses.
    #[test]
    fn s148_a_job_request_carries_an_optional_instance() {
        let cases = [
            (
                Request::JobResults {
                    job: 7,
                    instance: Some(String::from("k-1a-2b")),
                },
                r#"{"cmd":"job-results","job":7,"instance":"k-1a-2b"}"#,
            ),
            (
                Request::RetryJob {
                    job: 7,
                    instance: Some(String::from("k-1a-2b")),
                },
                r#"{"cmd":"retry-job","job":7,"instance":"k-1a-2b"}"#,
            ),
            (
                Request::CancelJob {
                    job: 7,
                    instance: Some(String::from("k-1a-2b")),
                },
                r#"{"cmd":"cancel-job","job":7,"instance":"k-1a-2b"}"#,
            ),
        ];
        for (request, line) in cases {
            assert_eq!(serde_json::to_string(&request).unwrap(), line);
            assert_eq!(serde_json::from_str::<Request>(line).unwrap(), request);
        }
        // The lines of an older client, without the field, still parse and
        // read as the current session of the daemon that answers.
        let old = r#"{"cmd":"cancel-job","job":7}"#;
        assert_eq!(
            serde_json::from_str::<Request>(old).unwrap(),
            Request::CancelJob {
                job: 7,
                instance: None,
            }
        );
    }

    // S-148 AC3: the answers of the daemon carry its instance, and an older
    // answer without one still parses and reads as no instance.
    #[test]
    fn s148_the_answers_carry_the_daemon_instance() {
        let answer = Response::Version {
            version: String::from("0.5.0"),
            run_params: true,
            job_results: true,
            instance: Some(String::from("k-1a-2b")),
        };
        assert_eq!(
            serde_json::to_string(&answer).unwrap(),
            r#"{"status":"version","version":"0.5.0","run_params":true,"job_results":true,"instance":"k-1a-2b"}"#
        );
        let old = r#"{"status":"version","version":"0.5.0","run_params":true,"job_results":true}"#;
        assert_eq!(
            serde_json::from_str::<Response>(old).unwrap(),
            Response::Version {
                version: String::from("0.5.0"),
                run_params: true,
                job_results: true,
                instance: None,
            }
        );
        let jobs = Response::Jobs {
            jobs: Vec::new(),
            instance: Some(String::from("k-1a-2b")),
        };
        assert_eq!(
            serde_json::to_string(&jobs).unwrap(),
            r#"{"status":"jobs","jobs":[],"instance":"k-1a-2b"}"#
        );
        let old = r#"{"status":"jobs","jobs":[]}"#;
        assert_eq!(
            serde_json::from_str::<Response>(old).unwrap(),
            Response::Jobs {
                jobs: Vec::new(),
                instance: None,
            }
        );
    }
}
