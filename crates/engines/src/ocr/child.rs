//! The bounded child runner of the OCR adapter (S-146-B).
//!
//! This is not a second process-tree implementation. It spawns through
//! [`crate::process::ProcessTree`], so the Unix process group and the Windows
//! kill-on-close job are the ones every other engine uses. What it adds is a
//! bounded drain: no line buffer, no message channel, one fixed read chunk per
//! reader thread, a stdout capture capped at [`STDOUT_LIMIT`] and a stderr
//! tail capped at [`STDERR_KEEP`].
//!
//! The main loop only polls the child, the cancel token, the deadline and a
//! caller hook, so a child that floods its output cannot starve a cancel or a
//! timeout. The whole tree is killed and reaped on every exit path, and both
//! reader threads are joined before this returns, so none is left blocked on
//! a pipe.

use crate::process::ProcessTree;
use kawaro_core::{CancelToken, EngineError};
use std::io::Read;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// The most stdout the probe keeps, in bytes. Above it is an error, never a
/// silent truncation.
pub(crate) const STDOUT_LIMIT: usize = 64 * 1024;
/// The last bytes of stderr kept for a diagnostic.
pub(crate) const STDERR_KEEP: usize = 4096;
/// The fixed read chunk of each reader thread. No line is ever allocated.
const CHUNK: usize = 8 * 1024;
/// The poll of the child loop.
const POLL: Duration = Duration::from_millis(25);

/// The result of one bounded child.
#[derive(Debug)]
pub(crate) struct Outcome {
    pub status: ExitStatus,
    /// The captured stdout, only when the caller asked for it.
    pub stdout: Vec<u8>,
    /// The last [`STDERR_KEEP`] bytes of stderr.
    pub stderr: String,
    /// True when a stderr line matched the line matcher of the run.
    ///
    /// The matcher runs while the child writes, so a warning that is more
    /// than [`STDERR_KEEP`] bytes before the end is still seen (S-151).
    pub matched: bool,
}

/// The stdout capture: bounded bytes and an overflow flag. A child that
/// writes more than [`STDOUT_LIMIT`] is stopped and reported, not truncated.
#[derive(Default)]
struct Capture {
    data: Mutex<Vec<u8>>,
    overflow: AtomicBool,
    failed: AtomicBool,
}

impl Capture {
    fn push(&self, chunk: &[u8]) {
        let mut data = self.data.lock().unwrap_or_else(|p| p.into_inner());
        let room = STDOUT_LIMIT.saturating_sub(data.len());
        if chunk.len() > room {
            data.extend_from_slice(&chunk[..room]);
            self.overflow.store(true, Ordering::Relaxed);
        } else {
            data.extend_from_slice(chunk);
        }
    }

    fn fail(&self) {
        self.failed.store(true, Ordering::Relaxed);
    }

    fn overflowed(&self) -> bool {
        self.overflow.load(Ordering::Relaxed)
    }

    fn failed(&self) -> bool {
        self.failed.load(Ordering::Relaxed)
    }

    fn bytes(&self) -> Vec<u8> {
        self.data.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }
}

/// The stderr reader: the last [`STDERR_KEEP`] bytes for a diagnostic, and
/// the whole lines for a matcher that runs while the child writes.
struct Tail {
    data: Mutex<Vec<u8>>,
    line: Mutex<Vec<u8>>,
    matched: AtomicBool,
    matcher: Option<fn(&str) -> bool>,
}

impl Tail {
    fn new(matcher: Option<fn(&str) -> bool>) -> Self {
        Self {
            data: Mutex::new(Vec::new()),
            line: Mutex::new(Vec::new()),
            matched: AtomicBool::new(false),
            matcher,
        }
    }

    fn push(&self, chunk: &[u8]) {
        {
            let mut data = self.data.lock().unwrap_or_else(|p| p.into_inner());
            data.extend_from_slice(chunk);
            let excess = data.len().saturating_sub(STDERR_KEEP);
            if excess > 0 {
                data.drain(..excess);
            }
        }
        let Some(matcher) = self.matcher else {
            return;
        };
        // Every complete line is matched while it streams, so a warning
        // that the tail drops later is still seen.
        let mut rest = chunk;
        while let Some(pos) = rest.iter().position(|&byte| byte == b'\n') {
            let (line, tail) = rest.split_at(pos);
            let mut pending = self.line.lock().unwrap_or_else(|p| p.into_inner());
            pending.extend_from_slice(line);
            let hit = {
                let text = String::from_utf8_lossy(&pending);
                matcher(&text)
            };
            pending.clear();
            if hit {
                self.matched.store(true, Ordering::Relaxed);
            }
            rest = &tail[1..];
        }
        let mut pending = self.line.lock().unwrap_or_else(|p| p.into_inner());
        pending.extend_from_slice(rest);
    }

    /// Matches the last line, when the child ended without a newline.
    fn finish(&self) {
        let Some(matcher) = self.matcher else {
            return;
        };
        let mut pending = self.line.lock().unwrap_or_else(|p| p.into_inner());
        if pending.is_empty() {
            return;
        }
        let hit = {
            let text = String::from_utf8_lossy(&pending);
            matcher(&text)
        };
        pending.clear();
        if hit {
            self.matched.store(true, Ordering::Relaxed);
        }
    }

    fn bytes(&self) -> Vec<u8> {
        self.data.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }
}

/// Reads one stream in fixed chunks until EOF. A reader error stops that
/// stream and marks the capture, never the other stream.
fn spawn_reader(
    mut stream: impl Read + Send + 'static,
    capture: Option<Arc<Capture>>,
    tail: Option<Arc<Tail>>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut buf = [0u8; CHUNK];
        loop {
            match stream.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if let Some(capture) = &capture {
                        capture.push(&buf[..n]);
                    }
                    if let Some(tail) = &tail {
                        tail.push(&buf[..n]);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    if let Some(capture) = &capture {
                        capture.fail();
                    }
                    break;
                }
            }
        }
        // The last line of a child that ended without a newline is matched
        // too.
        if let Some(tail) = &tail {
            tail.finish();
        }
    })
}

/// Runs `cmd` as one [`ProcessTree`] with bounded output.
///
/// `capture_stdout` keeps at most [`STDOUT_LIMIT`] bytes of stdout; above it
/// the run fails instead of truncating. Stderr always keeps only its last
/// [`STDERR_KEEP`] bytes. `match_line` is called for every stderr line while
/// the child writes; the result says in [`Outcome::matched`] whether one of
/// them matched. `poll` runs once per loop iteration and may stop the run
/// with an error, which is how the extraction watches its output file.
pub(crate) fn run_bounded(
    cmd: &mut Command,
    cancel: &CancelToken,
    timeout: Duration,
    capture_stdout: bool,
    match_line: Option<fn(&str) -> bool>,
    poll: &mut dyn FnMut() -> Result<(), EngineError>,
) -> Result<Outcome, EngineError> {
    cancel.check()?;
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut tree = ProcessTree::spawn(cmd)?;
    let stdout = tree.child_mut().stdout.take();
    let stderr = tree.child_mut().stderr.take();
    let capture = capture_stdout.then(|| Arc::new(Capture::default()));
    let tail = Arc::new(Tail::new(match_line));
    let stdout_reader = stdout.map(|stream| spawn_reader(stream, capture.clone(), None));
    let stderr_reader = stderr.map(|stream| spawn_reader(stream, None, Some(tail.clone())));

    let deadline = Instant::now() + timeout;
    let stop = loop {
        if capture.as_ref().is_some_and(|capture| capture.overflowed()) {
            break Err(overflow_error());
        }
        if let Err(e) = poll() {
            break Err(e);
        }
        match tree.child_mut().try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => {}
            Err(e) => break Err(e.into()),
        }
        if cancel.is_cancelled() {
            break Err(EngineError::Cancelled);
        }
        if Instant::now() >= deadline {
            break Err(EngineError::Failed(format!(
                "timed out after {} s",
                timeout.as_secs_f64()
            )));
        }
        std::thread::sleep(POLL);
    };
    // Also on success and on every error: nothing the tool left running may
    // outlive the task.
    tree.kill_all();
    drop(tree);
    if let Some(handle) = stdout_reader {
        let _ = handle.join();
    }
    if let Some(handle) = stderr_reader {
        let _ = handle.join();
    }
    let status = stop?;
    // The child may have written past the capture and exited before the loop
    // saw the flag.
    if capture.as_ref().is_some_and(|capture| capture.overflowed()) {
        return Err(overflow_error());
    }
    if capture.as_ref().is_some_and(|capture| capture.failed()) {
        return Err(EngineError::Failed(
            "could not read the child's standard output".to_string(),
        ));
    }
    let stdout = capture
        .as_ref()
        .map_or_else(Vec::new, |capture| capture.bytes());
    let stderr = String::from_utf8_lossy(&tail.bytes()).into_owned();
    Ok(Outcome {
        status,
        stdout,
        stderr,
        matched: tail.matched.load(Ordering::Relaxed),
    })
}

fn overflow_error() -> EngineError {
    EngineError::Failed(format!(
        "the child wrote more than {STDOUT_LIMIT} bytes to standard output"
    ))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn sh(script: &str) -> Command {
        let mut cmd = Command::new("/bin/sh");
        cmd.args(["-c", script]);
        cmd
    }

    #[cfg(unix)]
    fn allow() -> Result<(), EngineError> {
        Ok(())
    }

    // Unix only: `/bin/sh` and the `/dev/zero` and `/proc` fixtures below.

    #[cfg(unix)]
    #[test]
    fn s146b_a_single_huge_stdout_line_is_refused_not_truncated() {
        // One 70,000 byte line: above the 64 KiB capture.
        let mut cmd = sh("head -c 70000 /dev/zero | tr '\\0' a");
        let r = run_bounded(
            &mut cmd,
            &CancelToken::new(),
            Duration::from_secs(5),
            true,
            None,
            &mut allow,
        );
        match r {
            Err(EngineError::Failed(message)) => {
                assert!(message.contains("standard output"), "{message}");
                assert!(message.contains(&STDOUT_LIMIT.to_string()), "{message}");
            }
            other => panic!("expected the overflow error, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn s146b_a_small_probe_output_is_captured() {
        let mut cmd = sh("printf 'ara\\neng\\n'");
        let outcome = run_bounded(
            &mut cmd,
            &CancelToken::new(),
            Duration::from_secs(5),
            true,
            None,
            &mut allow,
        )
        .expect("the small output fits");
        assert!(outcome.status.success());
        assert_eq!(String::from_utf8_lossy(&outcome.stdout), "ara\neng\n");
    }

    #[cfg(unix)]
    #[test]
    fn s146b_a_discarded_flood_is_drained_and_not_retained() {
        // Two MiB of stdout: the drain must keep none of it.
        let mut cmd = sh("head -c 2097152 /dev/zero | tr '\\0' a");
        let outcome = run_bounded(
            &mut cmd,
            &CancelToken::new(),
            Duration::from_secs(10),
            false,
            None,
            &mut allow,
        )
        .expect("the flood drains");
        assert!(outcome.status.success());
        assert!(outcome.stdout.is_empty(), "discarded stdout is kept");
    }

    #[cfg(unix)]
    #[test]
    fn s146b_the_stderr_tail_is_bounded() {
        let mut cmd = sh("head -c 20000 /dev/zero | tr '\\0' a >&2; echo -n END >&2");
        let outcome = run_bounded(
            &mut cmd,
            &CancelToken::new(),
            Duration::from_secs(10),
            false,
            None,
            &mut allow,
        )
        .expect("the child exits");
        assert_eq!(outcome.stderr.len(), STDERR_KEEP);
        assert!(outcome.stderr.ends_with("aaEND"), "{}", outcome.stderr);
    }

    #[cfg(unix)]
    #[test]
    fn s146b_a_timeout_kills_the_tree() {
        let mut cmd = sh("sleep 30");
        let r = run_bounded(
            &mut cmd,
            &CancelToken::new(),
            Duration::from_millis(50),
            false,
            None,
            &mut allow,
        );
        match r {
            Err(EngineError::Failed(message)) => {
                assert!(message.contains("timed out"), "{message}")
            }
            other => panic!("expected a timeout, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn s146b_a_cancel_kills_the_tree_and_reaps_the_grandchild() {
        let dir = tempfile::tempdir().unwrap();
        let pidfile = dir.path().join("pid");
        let mut cmd = sh("sleep 30 & echo $! > \"$1\"; wait");
        cmd.arg("sh").arg(&pidfile);
        let cancel = CancelToken::new();
        let watcher = {
            let cancel = cancel.clone();
            std::thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(3);
                loop {
                    if let Ok(text) = std::fs::read_to_string(&pidfile)
                        && let Ok(pid) = text.trim().parse::<i32>()
                    {
                        cancel.cancel();
                        return Some(pid);
                    }
                    if Instant::now() >= deadline {
                        return None;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
            })
        };
        let r = run_bounded(
            &mut cmd,
            &cancel,
            Duration::from_secs(10),
            false,
            None,
            &mut allow,
        );
        let grandchild = watcher.join().unwrap().expect("no grandchild pid");
        match r {
            Err(EngineError::Cancelled) => {}
            other => panic!("expected Cancelled, got {other:?}"),
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let gone = match std::fs::read_to_string(format!("/proc/{grandchild}/stat")) {
                Err(_) => true,
                Ok(stat) => stat
                    .rsplit_once(')')
                    .and_then(|(_, rest)| rest.split_whitespace().next())
                    .is_some_and(|state| state == "Z"),
            };
            if gone {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "grandchild {grandchild} survived"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[cfg(unix)]
    #[test]
    fn s151_a_line_matched_early_is_kept_after_more_than_the_tail() {
        // The warning comes first, then more stderr than the kept tail.
        // The matcher runs while the child writes, so the warning is still
        // seen, even though the tail no longer holds it.
        let mut cmd =
            sh("printf 'WARNING: timed out\\n' >&2; head -c 20000 /dev/zero | tr '\\0' a >&2");
        let outcome = run_bounded(
            &mut cmd,
            &CancelToken::new(),
            Duration::from_secs(10),
            false,
            Some(|line| line.contains("timed out")),
            &mut allow,
        )
        .expect("the child exits");
        assert!(outcome.matched, "the early warning was lost");
        assert!(
            !outcome.stderr.contains("timed out"),
            "the tail still holds the warning"
        );
    }

    #[cfg(unix)]
    #[test]
    fn s151_a_line_without_the_marker_is_not_matched() {
        let mut cmd = sh("printf 'page 2 fine\\n' >&2");
        let outcome = run_bounded(
            &mut cmd,
            &CancelToken::new(),
            Duration::from_secs(10),
            false,
            Some(|line| line.contains("timed out")),
            &mut allow,
        )
        .expect("the child exits");
        assert!(!outcome.matched, "a line without the marker matched");
    }

    #[cfg(unix)]
    #[test]
    fn s146b_the_poll_hook_can_stop_the_run() {
        let mut cmd = sh("sleep 30");
        let mut calls = 0;
        let mut poll = || -> Result<(), EngineError> {
            calls += 1;
            if calls >= 2 {
                Err(EngineError::Failed("poll said stop".into()))
            } else {
                Ok(())
            }
        };
        let r = run_bounded(
            &mut cmd,
            &CancelToken::new(),
            Duration::from_secs(10),
            false,
            None,
            &mut poll,
        );
        match r {
            Err(EngineError::Failed(message)) => assert_eq!(message, "poll said stop"),
            other => panic!("expected the poll error, got {other:?}"),
        }
        assert!(calls >= 2);
    }
}
