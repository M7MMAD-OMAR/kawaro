//! Office engine (S-009): LibreOffice in headless mode as a child process.
//!
//! Every run copies the input into a private work directory, starts
//! LibreOffice with its own throwaway user profile inside that directory (so
//! it never meets the owner's running LibreOffice or profile), converts into
//! `out/`, moves the single result to `task.output`, and deletes the work
//! directory. LibreOffice reports no progress: 0.0 at start, 1.0 at the end.
//!
//! This module also holds the child process runner shared with the
//! Ghostscript engine: its own process group, a timeout, and cancellation,
//! both of which kill the whole group.

use crate::locate::{self, Program};
use sbarhive_core::{CancelToken, Engine, EngineError, MediaKind, Progress, Task};
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::OnceLock;
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Default `timeout_s` for one LibreOffice conversion.
pub const DEFAULT_TIMEOUT_S: f64 = 180.0;

const WRITER: &[&str] = &["doc", "docx", "odt", "rtf", "txt", "md"];
const CALC: &[&str] = &["xls", "xlsx", "ods", "csv"];
const IMPRESS: &[&str] = &["ppt", "pptx", "odp"];
const ALL_DOCS: &[&str] = &[
    "doc", "docx", "odt", "rtf", "txt", "md", "xls", "xlsx", "ods", "csv", "ppt", "pptx", "odp",
];

pub struct OfficeEngine {
    program: Option<PathBuf>,
    prefix_args: Vec<OsString>,
    temp_root: Option<PathBuf>,
}

impl Default for OfficeEngine {
    /// Uses LibreOffice as `locate::find` sees it, looked up once per process.
    fn default() -> Self {
        static FOUND: OnceLock<Option<PathBuf>> = OnceLock::new();
        let program = FOUND.get_or_init(|| locate::find(Program::Office)).clone();
        Self {
            program,
            prefix_args: Vec::new(),
            temp_root: None,
        }
    }
}

impl OfficeEngine {
    /// Runs `program prefix_args.. <libreoffice args>` instead of the detected
    /// LibreOffice. For tests and for unusual installations.
    pub fn with_program(program: impl Into<PathBuf>, prefix_args: Vec<OsString>) -> Self {
        Self {
            program: Some(program.into()),
            prefix_args,
            temp_root: None,
        }
    }

    /// Directory for the private work directories. Default: the system temp dir.
    pub fn temp_root(mut self, dir: impl Into<PathBuf>) -> Self {
        self.temp_root = Some(dir.into());
        self
    }
}

impl Engine for OfficeEngine {
    fn id(&self) -> &'static str {
        "office"
    }

    fn available(&self) -> bool {
        self.program.as_deref().is_some_and(Path::is_file)
    }

    fn capabilities(&self) -> Vec<(&'static str, Vec<MediaKind>)> {
        ["to-pdf", "convert-doc", "convert-sheet", "convert-slides"]
            .into_iter()
            .map(|op| (op, vec![MediaKind::Document]))
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
        let input = single_input(task)?;
        let in_ext = sbarhive_core::media::extension(input);
        let out_ext = sbarhive_core::media::extension(&task.output);
        let filter = plan(&task.op, input, &in_ext, &out_ext)?;
        let timeout = timeout_param(task, DEFAULT_TIMEOUT_S)?;
        let program = self
            .program
            .clone()
            .ok_or_else(|| missing("LibreOffice", OFFICE_URL, None))?;

        let work = work_dir(self.temp_root.as_deref(), "sbarhive-office-")?;
        let profile = work.path().join("profile");
        let out_dir = work.path().join("out");
        std::fs::create_dir(&out_dir)?;
        // A private copy: LibreOffice never writes lock files next to the
        // original, and odd file names cannot be read as options.
        let copy = work.path().join(format!("in.{in_ext}"));
        std::fs::copy(input, &copy).map_err(|e| input_error(input, e))?;

        let mut cmd = Command::new(&program);
        cmd.args(&self.prefix_args)
            .arg(format!("-env:UserInstallation={}", file_url(&profile)))
            .args([
                "--headless",
                "--invisible",
                "--nologo",
                "--nodefault",
                "--norestore",
                "--nolockcheck",
                "--convert-to",
                filter,
                "--outdir",
            ])
            .arg(&out_dir)
            .arg(&copy)
            .current_dir(work.path())
            .env("SAL_USE_VCLPLUGIN", "svp");
        let outcome = run_child(&mut cmd, cancel, timeout, &mut |_| {}).map_err(|e| match e {
            EngineError::Io(err) if err.kind() == std::io::ErrorKind::NotFound => {
                missing("LibreOffice", OFFICE_URL, Some(&program))
            }
            other => other,
        })?;

        let produced = out_dir.join(format!("in.{out_ext}"));
        if !is_nonempty_file(&produced) {
            return Err(EngineError::Failed(failure_message(
                "LibreOffice produced no output",
                outcome.status,
                &outcome.stderr,
            )));
        }
        move_file(&produced, &task.output)?;
        work.close()?;
        progress(1.0);
        Ok(())
    }
}

/// The `--convert-to` argument for `op`, after checking that the input
/// belongs to the op's document family and the output extension fits.
fn plan(op: &str, input: &Path, in_ext: &str, out_ext: &str) -> Result<&'static str, EngineError> {
    let (family, family_name, targets): (&[&str], &str, &[&str]) = match op {
        "to-pdf" => (ALL_DOCS, "an office document", &["pdf"]),
        "convert-doc" => (WRITER, "a text document", &["docx", "odt"]),
        "convert-sheet" => (CALC, "a spreadsheet", &["xlsx", "ods"]),
        "convert-slides" => (IMPRESS, "a presentation", &["pptx", "odp"]),
        _ => return Err(EngineError::UnsupportedOp(op.to_string())),
    };
    if !family.contains(&in_ext) {
        return Err(EngineError::Input {
            path: input.to_path_buf(),
            reason: format!("`{op}` needs {family_name}"),
        });
    }
    if !targets.contains(&out_ext) {
        return Err(EngineError::BadParam {
            name: "output".into(),
            reason: format!("`{op}` writes {}, not `{out_ext}`", targets.join(" or ")),
        });
    }
    Ok(convert_to(out_ext))
}

/// LibreOffice output filter for a target extension. The Office Open XML
/// filters are named so LibreOffice never picks a legacy variant.
fn convert_to(out_ext: &str) -> &'static str {
    match out_ext {
        "docx" => "docx:MS Word 2007 XML",
        "xlsx" => "xlsx:Calc MS Excel 2007 XML",
        "pptx" => "pptx:Impress MS PowerPoint 2007 XML",
        "odt" => "odt",
        "ods" => "ods",
        "odp" => "odp",
        _ => "pdf",
    }
}

pub(crate) fn single_input(task: &Task) -> Result<&Path, EngineError> {
    match task.inputs.as_slice() {
        [one] => Ok(one),
        other => Err(EngineError::BadParam {
            name: "inputs".into(),
            reason: format!("expected one input file, got {}", other.len()),
        }),
    }
}

/// `timeout_s` from the task, a positive number of seconds.
pub(crate) fn timeout_param(task: &Task, default_s: f64) -> Result<Duration, EngineError> {
    let bad = |reason: &str| EngineError::BadParam {
        name: "timeout_s".into(),
        reason: reason.into(),
    };
    let secs = match task.params.get("timeout_s") {
        None => default_s,
        Some(_) => task
            .param_f64("timeout_s")
            .ok_or_else(|| bad("must be a number"))?,
    };
    if !(secs > 0.0 && secs.is_finite()) {
        return Err(bad("must be a positive number of seconds"));
    }
    Ok(Duration::from_secs_f64(secs))
}

pub(crate) fn input_error(path: &Path, err: impl std::fmt::Display) -> EngineError {
    EngineError::Input {
        path: path.to_path_buf(),
        reason: err.to_string(),
    }
}

/// A private work directory under `root` (or the system temp dir), removed on drop.
pub(crate) fn work_dir(
    root: Option<&Path>,
    prefix: &str,
) -> Result<tempfile::TempDir, EngineError> {
    let mut builder = tempfile::Builder::new();
    builder.prefix(prefix);
    Ok(match root {
        Some(root) => builder.tempdir_in(root)?,
        None => builder.tempdir()?,
    })
}

pub(crate) fn is_nonempty_file(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.len() > 0)
}

/// Rename, or copy when `to` is on another file system.
pub(crate) fn move_file(from: &Path, to: &Path) -> Result<(), EngineError> {
    if std::fs::rename(from, to).is_err() {
        std::fs::copy(from, to)?;
        std::fs::remove_file(from)?;
    }
    Ok(())
}

pub(crate) fn failure_message(what: &str, status: ExitStatus, stderr: &str) -> String {
    let stderr = stderr.trim();
    if stderr.is_empty() {
        format!("{what} ({status})")
    } else {
        format!("{what} ({status}): {stderr}")
    }
}

/// Where to get LibreOffice, for the message when it is missing.
const OFFICE_URL: &str = "https://www.libreoffice.org/download/";

/// The error for a program that is not installed: its name, the path tried
/// if any, and where to get it.
pub(crate) fn missing(name: &str, url: &str, tried: Option<&Path>) -> EngineError {
    let tried = tried.map_or(String::new(), |p| format!(" at {}", p.display()));
    EngineError::Unavailable(format!("{name} not found{tried}; install it from {url}"))
}

/// `file://` URL for an absolute path of this system.
fn file_url(path: &Path) -> String {
    file_url_str(&path.to_string_lossy(), cfg!(windows))
}

/// `file://` URL for an absolute path, percent-encoding every byte outside
/// the unreserved set and `/`. A Windows path (`C:\a`, or `\\?\C:\a`)
/// becomes `file:///C:/a`, keeping the drive colon.
fn file_url_str(path: &str, windows: bool) -> String {
    let mut url = String::from("file://");
    let (path, keep): (String, &[u8]) = if windows {
        // A UNC path, verbatim or plain, is the host form file://server/share.
        let path = if let Some(rest) = path.strip_prefix(r"\\?\UNC\") {
            rest
        } else if let Some(rest) = path.strip_prefix(r"\\?\") {
            url.push('/');
            rest
        } else if let Some(rest) = path.strip_prefix(r"\\") {
            rest
        } else {
            url.push('/');
            path
        };
        (path.replace('\\', "/"), b"/-._~:")
    } else {
        (path.to_string(), b"/-._~")
    };
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || keep.contains(&byte) {
            url.push(byte as char);
        } else {
            url.push_str(&format!("%{byte:02X}"));
        }
    }
    url
}

pub(crate) fn path_dirs() -> Vec<PathBuf> {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default()
}

/// First `name` found as an executable file in `dirs`, names in priority order.
pub(crate) fn find_program(names: &[&str], dirs: &[PathBuf]) -> Option<PathBuf> {
    names
        .iter()
        .flat_map(|name| dirs.iter().map(move |dir| dir.join(name)))
        .find(|candidate| is_executable(candidate))
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

pub(crate) mod tree;

// The non-Unix versions live in their own file, so `.cargo/mutants.toml`
// can leave them out: a Linux build never compiles them.
#[cfg(not(unix))]
mod non_unix;
#[cfg(not(unix))]
use non_unix::is_executable;

pub(crate) struct ChildOutcome {
    pub status: ExitStatus,
    /// The last few KiB of standard error, for error messages.
    pub stderr: String,
}

const POLL: Duration = Duration::from_millis(25);
const STDERR_KEEP: usize = 4096;

/// Runs `cmd` in its own process group, feeding each stdout line to
/// `on_line`. Cancellation and the timeout kill the whole group, so helper
/// processes the tool starts die with it. A non-zero exit is returned in the
/// outcome, not as an error.
pub(crate) fn run_child(
    cmd: &mut Command,
    cancel: &CancelToken,
    timeout: Duration,
    on_line: &mut dyn FnMut(&str),
) -> Result<ChildOutcome, EngineError> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    #[cfg(unix)]
    let mut child = cmd.spawn()?;
    // On Windows a kill-on-close job holds the tree (ADR 0005 section 3).
    #[cfg(not(unix))]
    let (job, mut child) = non_unix::spawn_in_job(cmd)?;
    let (line_tx, lines) = mpsc::channel::<String>();
    let stdout = child.stdout.take().expect("stdout is piped");
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if line_tx.send(line).is_err() {
                break;
            }
        }
    });
    let (err_tx, err_rx) = mpsc::channel::<String>();
    let mut stderr = child.stderr.take().expect("stderr is piped");
    std::thread::spawn(move || {
        let mut tail = Vec::new();
        let mut buf = [0u8; 4096];
        while let Ok(n) = stderr.read(&mut buf) {
            if n == 0 {
                break;
            }
            tail.extend_from_slice(&buf[..n]);
            let excess = tail.len().saturating_sub(STDERR_KEEP);
            tail.drain(..excess);
        }
        let _ = err_tx.send(String::from_utf8_lossy(&tail).into_owned());
    });

    let deadline = Instant::now() + timeout;
    let stop = loop {
        while let Ok(line) = lines.try_recv() {
            on_line(&line);
        }
        if let Some(status) = child.try_wait()? {
            break Ok(status);
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
        match lines.recv_timeout(POLL) {
            Ok(line) => on_line(&line),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => std::thread::sleep(POLL),
        }
    };
    // Also on success: nothing the tool left running may outlive the task.
    #[cfg(unix)]
    kill_group(&mut child);
    #[cfg(not(unix))]
    non_unix::terminate_job(&job);
    let status = match stop {
        Ok(status) => status,
        Err(e) => {
            let _ = child.wait();
            return Err(e);
        }
    };
    // Lines still buffered after exit.
    while let Ok(line) = lines.recv_timeout(POLL) {
        on_line(&line);
    }
    let stderr = err_rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap_or_default();
    Ok(ChildOutcome { status, stderr })
}

#[cfg(unix)]
fn kill_group(child: &mut std::process::Child) {
    let pgid = child.id() as libc::pid_t;
    // SAFETY: plain syscall on the group this function's caller created.
    unsafe {
        libc::killpg(pgid, libc::SIGKILL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn office_plan_maps_targets_to_filters() {
        let p = Path::new("x");
        assert_eq!(plan("to-pdf", p, "pptx", "pdf").unwrap(), "pdf");
        assert_eq!(plan("to-pdf", p, "csv", "pdf").unwrap(), "pdf");
        assert_eq!(plan("to-pdf", p, "md", "pdf").unwrap(), "pdf");
        assert_eq!(
            plan("convert-doc", p, "rtf", "docx").unwrap(),
            "docx:MS Word 2007 XML"
        );
        assert_eq!(plan("convert-doc", p, "docx", "odt").unwrap(), "odt");
        assert_eq!(
            plan("convert-sheet", p, "csv", "xlsx").unwrap(),
            "xlsx:Calc MS Excel 2007 XML"
        );
        assert_eq!(plan("convert-sheet", p, "xls", "ods").unwrap(), "ods");
        assert_eq!(
            plan("convert-slides", p, "ppt", "pptx").unwrap(),
            "pptx:Impress MS PowerPoint 2007 XML"
        );
        assert_eq!(plan("convert-slides", p, "pptx", "odp").unwrap(), "odp");
        assert!(matches!(
            plan("convert-doc", p, "xlsx", "docx"),
            Err(EngineError::Input { .. })
        ));
        assert!(matches!(
            plan("convert-slides", p, "odt", "pptx"),
            Err(EngineError::Input { .. })
        ));
        assert!(matches!(
            plan("to-pdf", p, "pdf", "pdf"),
            Err(EngineError::Input { .. })
        ));
        assert!(matches!(
            plan("convert-sheet", p, "ods", "docx"),
            Err(EngineError::BadParam { .. })
        ));
        assert!(matches!(
            plan("nope", p, "odt", "pdf"),
            Err(EngineError::UnsupportedOp(_))
        ));
    }

    #[test]
    fn office_file_url_percent_encodes() {
        assert_eq!(
            file_url(Path::new("/tmp/a b/p%1")),
            "file:///tmp/a%20b/p%251"
        );
        assert_eq!(file_url(Path::new("/t/A-z_0.9~")), "file:///t/A-z_0.9~");
        assert_eq!(file_url(Path::new("/ü")), "file:///%C3%BC");
    }

    #[cfg(unix)]
    #[test]
    fn office_find_program_respects_name_order_and_exec_bit() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let (a, b) = (dir.path().join("a"), dir.path().join("b"));
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        let make = |p: &Path, mode: u32| {
            std::fs::write(p, "").unwrap();
            std::fs::set_permissions(p, std::fs::Permissions::from_mode(mode)).unwrap();
        };
        make(&a.join("libreoffice"), 0o755);
        make(&b.join("soffice"), 0o755);
        make(&a.join("plain"), 0o644);
        let dirs = vec![a.clone(), b.clone()];
        assert_eq!(
            find_program(&["soffice", "libreoffice"], &dirs),
            Some(b.join("soffice"))
        );
        assert_eq!(
            find_program(&["libreoffice", "soffice"], &dirs),
            Some(a.join("libreoffice"))
        );
        assert_eq!(find_program(&["plain"], &dirs), None);
        assert_eq!(find_program(&["missing"], &dirs), None);
        std::fs::create_dir(a.join("adir")).unwrap();
        std::fs::set_permissions(a.join("adir"), std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(find_program(&["adir"], &dirs), None);
    }

    #[test]
    fn office_path_dirs_reads_path() {
        // Every unix PATH holds `sh`; an empty or bogus list would not find it.
        assert!(find_program(&["sh"], &path_dirs()).is_some());
    }

    #[test]
    fn office_timeout_param_defaults_and_validates() {
        let task = |params: &str| Task {
            action_id: "t".into(),
            op: "to-pdf".into(),
            params: toml::from_str(params).unwrap(),
            inputs: vec![],
            output: PathBuf::from("o.pdf"),
        };
        assert_eq!(
            timeout_param(&task(""), 180.0).unwrap(),
            Duration::from_secs(180)
        );
        assert_eq!(
            timeout_param(&task("timeout_s = 2"), 180.0).unwrap(),
            Duration::from_secs(2)
        );
        assert_eq!(
            timeout_param(&task("timeout_s = 0.5"), 1.0).unwrap(),
            Duration::from_millis(500)
        );
        for bad in [
            "timeout_s = 0",
            "timeout_s = -1",
            "timeout_s = \"x\"",
            "timeout_s = nan",
            "timeout_s = inf",
        ] {
            assert!(timeout_param(&task(bad), 180.0).is_err(), "{bad}");
        }
    }

    #[test]
    fn office_run_child_reports_lines_status_and_stderr() {
        let mut cmd = Command::new("/bin/sh");
        cmd.args(["-c", "echo one; echo two; echo oops >&2; exit 3"]);
        let mut lines = Vec::new();
        let outcome = run_child(
            &mut cmd,
            &CancelToken::new(),
            Duration::from_secs(10),
            &mut |l| lines.push(l.to_string()),
        )
        .unwrap();
        assert_eq!(lines, ["one", "two"]);
        assert_eq!(outcome.status.code(), Some(3));
        assert_eq!(outcome.stderr.trim(), "oops");
        let msg = failure_message("gs failed", outcome.status, &outcome.stderr);
        assert!(
            msg.starts_with("gs failed (") && msg.ends_with("): oops"),
            "{msg}"
        );
        let msg = failure_message("x", outcome.status, "  ");
        assert!(msg.starts_with("x (") && msg.ends_with(')'), "{msg}");
    }

    #[test]
    fn office_run_child_keeps_only_the_stderr_tail() {
        let mut cmd = Command::new("/bin/sh");
        cmd.args([
            "-c",
            "head -c 20000 /dev/zero | tr '\\0' a >&2; echo -n END >&2",
        ]);
        let outcome = run_child(
            &mut cmd,
            &CancelToken::new(),
            Duration::from_secs(10),
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(outcome.stderr.len(), STDERR_KEEP);
        assert!(outcome.stderr.ends_with("aaEND"));
    }

    #[test]
    fn office_move_file_and_nonempty_checks() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        std::fs::write(&a, "data").unwrap();
        assert!(is_nonempty_file(&a));
        let b = dir.path().join("b");
        move_file(&a, &b).unwrap();
        assert!(!a.exists());
        assert_eq!(std::fs::read_to_string(&b).unwrap(), "data");
        std::fs::write(&a, "").unwrap();
        assert!(!is_nonempty_file(&a));
        assert!(!is_nonempty_file(dir.path()));
        assert!(!is_nonempty_file(&dir.path().join("missing")));
        // Rename fails into a missing directory, and the copy fallback reports it.
        assert!(move_file(&b, &dir.path().join("no/such/dir/c")).is_err());
    }

    // S-025b. `file_url_str(path: &str, windows: bool) -> String`, and an
    // engine without a program names LibreOffice and its download page.

    #[test]
    fn ac10_file_url_str_windows_drive_letter_and_space() {
        assert_eq!(file_url_str("C:\\a b\\x", true), "file:///C:/a%20b/x");
    }

    #[test]
    fn ac10_file_url_str_windows_arabic_name() {
        assert_eq!(
            file_url_str("C:\\ملف.docx", true),
            "file:///C:/%D9%85%D9%84%D9%81.docx"
        );
    }

    #[test]
    fn ac10_file_url_str_windows_drops_verbatim_prefix() {
        assert_eq!(file_url_str("\\\\?\\C:\\a", true), "file:///C:/a");
    }

    #[test]
    fn ac10_file_url_str_unix_unchanged() {
        assert_eq!(file_url_str("/a b/c", false), "file:///a%20b/c");
        assert_eq!(
            file_url_str("/tmp/a b/p%1", false),
            "file:///tmp/a%20b/p%251"
        );
        assert_eq!(file_url_str("/ü", false), "file:///%C3%BC");
        if !cfg!(windows) {
            assert_eq!(file_url(Path::new("/a b/c")), file_url_str("/a b/c", false));
        }
    }

    // S-028a AC1 (backlog/S-028-review-fixes.md): a UNC path, verbatim or
    // plain, becomes the host form `file://server/share/...` (two slashes).

    #[test]
    fn s028_ac1_file_url_str_verbatim_unc_is_host_form() {
        assert_eq!(
            file_url_str(r"\\?\UNC\srv\share\a b.docx", true),
            "file://srv/share/a%20b.docx"
        );
    }

    #[test]
    fn s028_ac1_file_url_str_plain_unc_is_host_form() {
        assert_eq!(file_url_str(r"\\srv\share\x", true), "file://srv/share/x");
    }

    #[test]
    fn s028_ac1_file_url_str_verbatim_unc_arabic_name() {
        assert_eq!(
            file_url_str("\\\\?\\UNC\\srv\\share\\ملف.docx", true),
            "file://srv/share/%D9%85%D9%84%D9%81.docx"
        );
    }

    #[test]
    fn ac11_office_without_program_names_libreoffice_and_link() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("a.txt");
        std::fs::write(&input, "x").unwrap();
        let engine = OfficeEngine {
            program: None,
            ..OfficeEngine::with_program("unused", vec![])
        }
        .temp_root(dir.path());
        let task = Task {
            action_id: "test.to-pdf".into(),
            op: "to-pdf".into(),
            params: toml::from_str("").unwrap(),
            inputs: vec![input],
            output: dir.path().join("a.pdf"),
        };
        let r = engine.run(&task, &|_| {}, &CancelToken::new());
        match r {
            Err(EngineError::Unavailable(m)) => {
                assert!(m.contains("LibreOffice"), "{m}");
                assert!(m.contains("https://www.libreoffice.org/download/"), "{m}");
            }
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }
}
