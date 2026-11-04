//! qpdf child engine (S-128): fast web view, lossless cleanup and a structure
//! and encryption check through the system `qpdf` program.
//!
//! qpdf is Apache-2.0 and is never linked or downloaded: it is taken from the
//! system like Ghostscript and poppler, and its actions are hidden when it is
//! missing. Every run goes through a killable `ProcessTree`, the input is
//! only read, and the output is written to `task.output` by the core queue.

use crate::locate::{self, Program};
use crate::office::{
    failure_message, input_error, is_nonempty_file, missing, missing_on_not_found, move_file,
    single_input, timeout_param, work_dir,
};
use crate::process::ProcessTree;
use kawaro_core::{CancelToken, Engine, EngineError, MediaKind, Progress, Task};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Where to get qpdf, for the message when it is missing.
const QPDF_URL: &str = "https://qpdf.readthedocs.io/";
/// Default `timeout_s` for one run.
pub const DEFAULT_TIMEOUT_S: f64 = 300.0;
/// qpdf's exit code for warnings: the file is readable and the report is still
/// worth writing, so `check` treats it as success.
const WARNINGS: i32 = 2;
const POLL: Duration = Duration::from_millis(25);
/// The last bytes of standard error kept for error messages.
const STDERR_KEEP: usize = 4096;

pub struct QpdfEngine {
    program: Option<PathBuf>,
}

impl Default for QpdfEngine {
    /// Uses `qpdf` as `locate::find` sees it, looked up once.
    fn default() -> Self {
        static FOUND: OnceLock<Option<PathBuf>> = OnceLock::new();
        Self {
            program: FOUND.get_or_init(|| locate::find(Program::Qpdf)).clone(),
        }
    }
}

impl QpdfEngine {
    /// Runs this program instead of the detected `qpdf`.
    pub fn with_program(program: impl Into<PathBuf>) -> Self {
        Self {
            program: Some(program.into()),
        }
    }

    fn program(&self) -> Result<PathBuf, EngineError> {
        self.program
            .clone()
            .ok_or_else(|| missing("qpdf", QPDF_URL, None))
    }

    /// `linearize` and `cleanup`: a structural rewrite to a new PDF.
    fn rewrite(&self, task: &Task, op: &str, cancel: &CancelToken) -> Result<(), EngineError> {
        let program = self.program()?;
        let input = absolute_input(task)?;
        let work = work_dir(None, "kawaro-qpdf-")?;
        let out = work.path().join("out.pdf");
        let args = rewrite_args(op, &input, &out)?;
        let outcome = run(&program, &args, task, cancel)?;
        if !outcome.status.success() {
            return Err(tool_error(&input, outcome.status, &outcome.stderr));
        }
        if !is_nonempty_file(&out) {
            return Err(EngineError::Failed("qpdf wrote no PDF".into()));
        }
        move_file(&out, &task.output)?;
        work.close()?;
        Ok(())
    }

    /// `check`: qpdf's structure and encryption report as one UTF-8 text file.
    /// Exit codes 0 and 2 are a report, and so is any run that wrote output,
    /// including an encrypted file: the report is the point of this op. Only a
    /// run with no output at all stays an error.
    fn check(&self, task: &Task, cancel: &CancelToken) -> Result<(), EngineError> {
        let program = self.program()?;
        let input = absolute_input(task)?;
        let args = check_args(&input);
        let outcome = run(&program, &args, task, cancel)?;
        let warned = outcome.status.code() == Some(WARNINGS);
        let reported = !outcome.stdout.is_empty() || !outcome.stderr.is_empty();
        if !outcome.status.success() && !warned && !reported {
            return Err(tool_error(&input, outcome.status, &outcome.stderr));
        }
        let mut text = outcome.stdout;
        if !outcome.stderr.is_empty() {
            if !text.is_empty() && !text.ends_with(b"\n") {
                text.push(b'\n');
            }
            text.extend_from_slice(outcome.stderr.as_bytes());
        }
        let text = normalized(text);
        std::fs::write(&task.output, &text).map_err(|e| {
            EngineError::Failed(format!("cannot write {}: {e}", task.output.display()))
        })
    }
}

impl Engine for QpdfEngine {
    fn id(&self) -> &'static str {
        "qpdf"
    }

    fn available(&self) -> bool {
        self.program.as_deref().is_some_and(Path::is_file)
    }

    fn capabilities(&self) -> Vec<(&'static str, Vec<MediaKind>)> {
        ["linearize", "cleanup", "check"]
            .into_iter()
            .map(|op| (op, vec![MediaKind::Pdf]))
            .collect()
    }

    fn run(
        &self,
        task: &Task,
        progress: Progress,
        cancel: &CancelToken,
    ) -> Result<(), EngineError> {
        progress(0.0);
        cancel.check()?;
        let op = task.op.as_str();
        match op {
            "linearize" | "cleanup" => self.rewrite(task, op, cancel)?,
            "check" => self.check(task, cancel)?,
            other => return Err(EngineError::UnsupportedOp(other.to_string())),
        }
        progress(1.0);
        Ok(())
    }
}

/// The arguments of a rewrite op, with absolute input and output paths.
fn rewrite_args(op: &str, input: &Path, output: &Path) -> Result<Vec<String>, EngineError> {
    let input = input.to_string_lossy().into_owned();
    let output = output.to_string_lossy().into_owned();
    Ok(match op {
        "linearize" => vec!["--linearize".to_string(), input, output],
        "cleanup" => vec![
            "--object-streams=generate".to_string(),
            "--compress-streams=y".to_string(),
            "--recompress-flate".to_string(),
            "--compression-level=9".to_string(),
            input,
            output,
        ],
        other => return Err(EngineError::UnsupportedOp(other.to_string())),
    })
}

/// The arguments of `check`: the structure and encryption report goes to
/// standard output.
fn check_args(input: &Path) -> Vec<String> {
    vec![
        "--check".to_string(),
        "--show-encryption".to_string(),
        input.to_string_lossy().into_owned(),
    ]
}

/// The single input as an absolute path, so qpdf never reads a name starting
/// with `-` as an option.
fn absolute_input(task: &Task) -> Result<PathBuf, EngineError> {
    let input = single_input(task)?;
    if !input.is_file() {
        return Err(input_error(input, "no such file"));
    }
    std::path::absolute(input).map_err(|e| input_error(input, e))
}

struct Outcome {
    status: ExitStatus,
    stdout: Vec<u8>,
    /// The last bytes of standard error, as lossy text.
    stderr: String,
}

/// Runs `qpdf` in a killable tree and captures both streams. The tree is
/// killed on cancel, on the timeout and after a normal exit.
fn run(
    program: &Path,
    args: &[String],
    task: &Task,
    cancel: &CancelToken,
) -> Result<Outcome, EngineError> {
    let timeout = timeout_param(task, DEFAULT_TIMEOUT_S)?;
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut tree = ProcessTree::spawn(&mut cmd)
        .map_err(|e| missing_on_not_found(EngineError::Io(e), "qpdf", QPDF_URL, program))?;
    let stdout = tree.child_mut().stdout.take().expect("stdout is piped");
    let out_thread = std::thread::spawn(move || {
        let mut reader = stdout;
        let mut bytes = Vec::new();
        let _ = reader.read_to_end(&mut bytes);
        bytes
    });
    let mut stderr = tree.child_mut().stderr.take().expect("stderr is piped");
    let err_thread = std::thread::spawn(move || {
        let mut tail: Vec<u8> = Vec::new();
        let mut buffer = [0u8; 4096];
        while let Ok(read) = stderr.read(&mut buffer) {
            if read == 0 {
                break;
            }
            tail.extend_from_slice(&buffer[..read]);
            let excess = tail.len().saturating_sub(STDERR_KEEP);
            tail.drain(..excess);
        }
        String::from_utf8_lossy(&tail).into_owned()
    });
    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(status) = tree.child_mut().try_wait()? {
            break status;
        }
        if cancel.is_cancelled() {
            tree.kill_all();
            return Err(EngineError::Cancelled);
        }
        if Instant::now() >= deadline {
            tree.kill_all();
            return Err(EngineError::Failed(format!(
                "qpdf timed out after {} s",
                timeout.as_secs_f64()
            )));
        }
        std::thread::sleep(POLL);
    };
    // Nothing the tool started may outlive the task.
    tree.kill_all();
    let stdout = out_thread.join().unwrap_or_default();
    let stderr = err_thread.join().unwrap_or_default();
    Ok(Outcome {
        status,
        stdout,
        stderr,
    })
}

/// The error of a failed qpdf run: a password message is an input error with a
/// clear reason, anything else is `Failed` with the stderr tail.
fn tool_error(input: &Path, status: ExitStatus, stderr: &str) -> EngineError {
    if is_password_error(stderr) {
        input_error(input, "the PDF is encrypted and needs a password")
    } else {
        EngineError::Failed(failure_message("qpdf failed", status, stderr))
    }
}

/// True when qpdf's message says the file needs a password.
fn is_password_error(stderr: &str) -> bool {
    let lower = stderr.to_ascii_lowercase();
    lower.contains("password") || lower.contains("encrypted")
}

/// UTF-8 without a byte order mark and with LF line endings.
fn normalized(mut bytes: Vec<u8>) -> Vec<u8> {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        bytes.drain(..3);
    }
    bytes.retain(|byte| *byte != b'\r');
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(op: &str, input: &Path, output: &Path) -> Task {
        Task {
            action_id: format!("test.{op}"),
            op: op.into(),
            params: toml::from_str("").unwrap(),
            inputs: vec![input.to_path_buf()],
            output: output.to_path_buf(),
        }
    }

    #[test]
    fn s128_args_build_the_three_commands() {
        let input = Path::new("/in/a.pdf");
        let output = Path::new("/tmp/out.pdf");
        assert_eq!(
            rewrite_args("linearize", input, output).unwrap(),
            ["--linearize", "/in/a.pdf", "/tmp/out.pdf"]
        );
        assert_eq!(
            rewrite_args("cleanup", input, output).unwrap(),
            [
                "--object-streams=generate",
                "--compress-streams=y",
                "--recompress-flate",
                "--compression-level=9",
                "/in/a.pdf",
                "/tmp/out.pdf",
            ]
        );
        assert_eq!(
            check_args(input),
            ["--check", "--show-encryption", "/in/a.pdf"]
        );
        assert!(matches!(
            rewrite_args("bogus", input, output),
            Err(EngineError::UnsupportedOp(_))
        ));
    }

    /// A fake qpdf script, executable, with the given body.
    #[cfg(unix)]
    fn fake_qpdf(dir: &Path, body: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let program = dir.join("qpdf");
        std::fs::write(&program, body).unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        program
    }

    #[cfg(unix)]
    #[test]
    fn s128_check_writes_the_report_instead_of_failing_on_an_encrypted_file() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("a.pdf");
        std::fs::write(&input, b"%PDF-1.4\n").unwrap();
        // qpdf exit code 3 with a report on both streams: the report is the point.
        let program = fake_qpdf(
            dir.path(),
            "#!/bin/sh\necho \"file is encrypted\"\necho \"password required\" >&2\nexit 3\n",
        );
        let engine = QpdfEngine::with_program(&program);
        let out = dir.path().join("report.txt");
        engine
            .run(&task("check", &input, &out), &|_| {}, &CancelToken::new())
            .unwrap();
        let text = std::fs::read_to_string(&out).unwrap();
        assert!(text.contains("file is encrypted"), "{text}");
        assert!(text.contains("password required"), "{text}");

        // A hard failure with no output at all stays an error.
        let program = fake_qpdf(dir.path(), "#!/bin/sh\nexit 3\n");
        let engine = QpdfEngine::with_program(&program);
        let out = dir.path().join("failed.txt");
        match engine.run(&task("check", &input, &out), &|_| {}, &CancelToken::new()) {
            Err(EngineError::Failed(_)) => {}
            other => panic!("expected Failed, got {other:?}"),
        }
        assert!(!out.exists());
    }

    #[cfg(unix)]
    #[test]
    fn s128_an_encrypted_rewrite_is_an_input_error() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("a.pdf");
        std::fs::write(&input, b"%PDF-1.4\n").unwrap();
        let program = fake_qpdf(
            dir.path(),
            "#!/bin/sh\necho \"input file is encrypted\" >&2\nexit 3\n",
        );
        let engine = QpdfEngine::with_program(&program);
        let out = dir.path().join("linearized.pdf");
        match engine.run(
            &task("linearize", &input, &out),
            &|_| {},
            &CancelToken::new(),
        ) {
            Err(EngineError::Input { reason, .. }) => {
                assert!(reason.contains("encrypted"), "{reason}");
            }
            other => panic!("expected Input, got {other:?}"),
        }
        assert!(!out.exists());
    }

    #[test]
    fn s128_capabilities_cover_the_three_ops() {
        let caps = QpdfEngine::default().capabilities();
        let ops: Vec<&str> = caps.iter().map(|(op, _)| *op).collect();
        assert_eq!(ops, ["linearize", "cleanup", "check"]);
        for (op, kinds) in &caps {
            assert_eq!(kinds, &vec![MediaKind::Pdf], "{op}");
        }
        assert_eq!(QpdfEngine::default().id(), "qpdf");
    }

    #[test]
    fn s128_missing_program_is_unavailable_naming_qpdf() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("a.pdf");
        std::fs::write(&input, b"%PDF-1.5\n").unwrap();
        let engine = QpdfEngine::with_program("/nonexistent/kawaro-no-qpdf");
        assert!(!engine.available());
        for op in ["linearize", "cleanup", "check"] {
            let out = dir.path().join(format!("{op}.out"));
            match engine.run(&task(op, &input, &out), &|_| {}, &CancelToken::new()) {
                Err(EngineError::Unavailable(message)) => {
                    let lower = message.to_lowercase();
                    assert!(lower.contains("qpdf"), "{message}");
                    assert!(
                        message.contains("https://qpdf.readthedocs.io/"),
                        "{message}"
                    );
                }
                other => panic!("{op}: expected Unavailable, got {other:?}"),
            }
            assert!(!out.exists(), "{op} wrote an output");
        }
    }

    #[test]
    fn s128_password_and_report_text_helpers() {
        assert!(is_password_error("qpdf: invalid password"));
        assert!(is_password_error("the input file is ENCRYPTED"));
        assert!(!is_password_error("syntax error near xref"));
        assert_eq!(
            normalized(b"\xEF\xBB\xBFchecking\r\nfile is not encrypted\r\n".to_vec()),
            b"checking\nfile is not encrypted\n"
        );
    }
}
