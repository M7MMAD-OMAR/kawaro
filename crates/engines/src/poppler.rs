//! PDF to text through poppler `pdftotext` (S-126, ADR 0013).
//!
//! poppler stays a system program: nothing is linked and nothing is
//! downloaded. The action is hidden when the tool is missing, exactly like the
//! other child engines.
//!
//! The text comes from `pdftotext -layout -enc UTF-8` and is written as UTF-8
//! without a byte order mark and with LF line endings. The Arabic and RTL
//! limits are recorded in ADR 0013: the tool returns text in content order, so
//! a producer that stores visual order, or a subset font without a ToUnicode
//! map, gives reversed or missing characters, and no bidi reordering is done
//! here.

use crate::locate::{self, Program};
use crate::office::{
    failure_message, input_error, missing, missing_on_not_found, single_input, timeout_param,
};
use crate::process::ProcessTree;
use kawaro_core::{CancelToken, Engine, EngineError, MediaKind, Progress, Task};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Where to get poppler, for the message when it is missing.
const POPPLER_URL: &str = "https://poppler.freedesktop.org/";
/// Default `timeout_s` for one run.
pub const DEFAULT_TIMEOUT_S: f64 = 300.0;
const POLL: Duration = Duration::from_millis(25);
/// The last bytes of standard error kept for error messages.
const STDERR_KEEP: usize = 4096;

pub struct PopplerEngine {
    program: Option<PathBuf>,
}

impl Default for PopplerEngine {
    /// Uses `pdftotext` as `locate::find` sees it, looked up once.
    fn default() -> Self {
        static FOUND: OnceLock<Option<PathBuf>> = OnceLock::new();
        Self {
            program: FOUND
                .get_or_init(|| locate::find(Program::Pdftotext))
                .clone(),
        }
    }
}

impl PopplerEngine {
    /// Runs this program instead of the detected `pdftotext`.
    pub fn with_program(program: impl Into<PathBuf>) -> Self {
        Self {
            program: Some(program.into()),
        }
    }

    /// The `to-text` op: run the tool and write its text to `task.output`.
    fn to_text(&self, task: &Task, cancel: &CancelToken) -> Result<(), EngineError> {
        let range = pages_param(task)?;
        let input = absolute_input(task)?;
        if !looks_like_pdf(&input)? {
            return Err(input_error(&input, "not a PDF file"));
        }
        // The page count is only needed to refuse an out of range range, so
        // the document is parsed with lopdf only when the caller narrowed the
        // pages. Encrypted files are also caught here when they are parsed.
        if let Some(range) = range
            && let Some(count) = page_count(&input)?
            && (range.0 > count || range.1 > count)
        {
            return Err(EngineError::BadParam {
                name: "pages".into(),
                reason: format!(
                    "page {} to {} is outside this PDF, which has {count} pages",
                    range.0, range.1
                ),
            });
        }
        let program = self
            .program
            .clone()
            .ok_or_else(|| missing("pdftotext", POPPLER_URL, None))?;
        let bytes = run_tool(&program, &input, range, task, cancel)?;
        let text = normalized(bytes);
        std::fs::write(&task.output, &text).map_err(|e| {
            EngineError::Failed(format!("cannot write {}: {e}", task.output.display()))
        })
    }
}

impl Engine for PopplerEngine {
    fn id(&self) -> &'static str {
        "poppler"
    }

    fn available(&self) -> bool {
        self.program.as_deref().is_some_and(Path::is_file)
    }

    fn capabilities(&self) -> Vec<(&'static str, Vec<MediaKind>)> {
        vec![("to-text", vec![MediaKind::Pdf])]
    }

    fn run(
        &self,
        task: &Task,
        progress: Progress,
        cancel: &CancelToken,
    ) -> Result<(), EngineError> {
        progress(0.0);
        cancel.check()?;
        match task.op.as_str() {
            "to-text" => self.to_text(task, cancel)?,
            op => return Err(EngineError::UnsupportedOp(op.to_string())),
        }
        progress(1.0);
        Ok(())
    }
}

/// The single input as an absolute path, so the tool never reads a name
/// starting with `-` as an option.
fn absolute_input(task: &Task) -> Result<PathBuf, EngineError> {
    let input = single_input(task)?;
    if !input.is_file() {
        return Err(input_error(input, "no such file"));
    }
    std::path::absolute(input).map_err(|e| input_error(input, e))
}

/// A file whose first KiB does not hold the PDF header is refused before the
/// child runs, so the error names the file.
fn looks_like_pdf(path: &Path) -> Result<bool, EngineError> {
    let mut header = [0u8; 1024];
    let mut file = std::fs::File::open(path).map_err(|e| input_error(path, e))?;
    let read = file.read(&mut header).map_err(|e| input_error(path, e))?;
    Ok(header[..read].windows(5).any(|window| window == b"%PDF-"))
}

/// The page count for the range check. An encrypted file is an input error; a
/// file lopdf cannot parse is left to the child, which reports the real
/// error.
fn page_count(input: &Path) -> Result<Option<usize>, EngineError> {
    match lopdf::Document::load(input) {
        Ok(doc) if doc.is_encrypted() => {
            Err(input_error(input, "the PDF is encrypted with a password"))
        }
        Ok(doc) => Ok(Some(doc.get_pages().len())),
        Err(_) => Ok(None),
    }
}

/// The `pages` param: empty for every page, or one 1-based range `N` or
/// `N-M`. This action takes one contiguous group only, so a comma list or
/// anything else is `BadParam` named `pages`.
fn pages_param(task: &Task) -> Result<Option<(usize, usize)>, EngineError> {
    let bad = |reason: String| EngineError::BadParam {
        name: "pages".into(),
        reason,
    };
    let text = match task.params.get("pages") {
        None => return Ok(None),
        Some(value) => value
            .as_str()
            .ok_or_else(|| bad("must be a string: empty, N or N-M".into()))?,
    };
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    let number = |part: &str| part.trim().parse::<usize>().ok().filter(|page| *page >= 1);
    let (first, last) = match text.split_once('-') {
        Some((first, last)) => match (number(first), number(last)) {
            (Some(first), Some(last)) if first <= last => (first, last),
            _ => {
                return Err(bad(format!(
                    "`{text}` is not a range like 2-4 with 1 <= first <= last"
                )));
            }
        },
        None => match number(text) {
            Some(page) => (page, page),
            None => return Err(bad(format!("`{text}` is not a page number"))),
        },
    };
    Ok(Some((first, last)))
}

/// Runs `pdftotext` and returns its standard output. The whole tree is killed
/// on cancel, on the timeout and after a normal exit.
fn run_tool(
    program: &Path,
    input: &Path,
    range: Option<(usize, usize)>,
    task: &Task,
    cancel: &CancelToken,
) -> Result<Vec<u8>, EngineError> {
    let timeout = timeout_param(task, DEFAULT_TIMEOUT_S)?;
    let mut cmd = Command::new(program);
    cmd.args(["-layout", "-enc", "UTF-8"]);
    if let Some((first, last)) = range {
        cmd.arg("-f")
            .arg(first.to_string())
            .arg("-l")
            .arg(last.to_string());
    }
    cmd.arg(input).arg("-");
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut tree = ProcessTree::spawn(&mut cmd)
        .map_err(|e| missing_on_not_found(EngineError::Io(e), "pdftotext", POPPLER_URL, program))?;
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
                "pdftotext timed out after {} s",
                timeout.as_secs_f64()
            )));
        }
        std::thread::sleep(POLL);
    };
    // Nothing the tool started may outlive the task.
    tree.kill_all();
    let bytes = out_thread.join().unwrap_or_default();
    let stderr = err_thread.join().unwrap_or_default();
    if !status.success() {
        if is_password_error(&stderr) {
            return Err(input_error(input, "the PDF is encrypted with a password"));
        }
        return Err(EngineError::Failed(failure_message(
            "pdftotext failed",
            status,
            &stderr,
        )));
    }
    Ok(bytes)
}

/// True when the tool message says the document is encrypted.
fn is_password_error(stderr: &str) -> bool {
    let lower = stderr.to_ascii_lowercase();
    lower.contains("encrypted") || lower.contains("password")
}

/// UTF-8 without a byte order mark and with LF line endings: a leading BOM is
/// dropped and every carriage return is removed, so the file is the same on
/// every system. Everything else the tool wrote is kept, including the form
/// feed page separators.
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

    fn task(params: &str) -> Task {
        Task {
            action_id: "test.to-text".into(),
            op: "to-text".into(),
            params: toml::from_str(params).unwrap(),
            inputs: vec![],
            output: PathBuf::from("out.txt"),
        }
    }

    #[test]
    fn s126_pages_accepts_empty_page_and_range() {
        assert_eq!(pages_param(&task("")).unwrap(), None);
        assert_eq!(pages_param(&task("pages = \"\"")).unwrap(), None);
        assert_eq!(pages_param(&task("pages = \"3\"")).unwrap(), Some((3, 3)));
        assert_eq!(pages_param(&task("pages = \"2-4\"")).unwrap(), Some((2, 4)));
        assert_eq!(
            pages_param(&task("pages = \" 2 - 4 \"")).unwrap(),
            Some((2, 4))
        );
    }

    #[test]
    fn s126_pages_rejects_multi_group_and_bad_ranges() {
        for bad in ["0", "3-1", "1,3", "1-", "-2", "a", "1-2-3", "1-2,3", "2.5"] {
            let result = pages_param(&task(&format!("pages = \"{bad}\"")));
            match result {
                Err(EngineError::BadParam { name, .. }) => assert_eq!(name, "pages", "{bad}"),
                other => panic!("{bad}: expected BadParam, got {other:?}"),
            }
        }
        // A wrong type is refused too.
        assert!(pages_param(&task("pages = 3")).is_err());
    }

    #[test]
    fn s126_output_has_no_bom_and_no_carriage_return() {
        assert_eq!(normalized(b"plain\r\nline\r\n".to_vec()), b"plain\nline\n");
        assert_eq!(normalized(b"\xEF\xBB\xBFtext".to_vec()), b"text");
        // A form feed page separator is kept.
        assert_eq!(normalized(b"page one\x0C".to_vec()), b"page one\x0C");
    }
}
