//! Searchable PDF through the optional system OCRmyPDF child process (S-150, ADR 0017).
//!
//! OCRmyPDF stays an optional system program: nothing is linked, bundled,
//! downloaded, or installed. Missing tools hide the action from the hive and
//! surface typed [`UnavailableReason`] diagnostics with read-only guidance.
//!
//! Every run reuses the bounded child runner in [`crate::ocr::child`] (which
//! spawns through [`crate::process::ProcessTree`]), the shared single-child
//! OCR concurrency permit in [`crate::ocr`], and the cached Tesseract language
//! probe in [`OcrEngine`]. The version probe (`ocrmypdf --version`) is bounded
//! (5 s, 64 KiB stdout cap) and cached in [`ProbeCache`]; any version below
//! `14.4.0` is refused because older releases predate default digital-signature
//! refusal (`DigitalSignatureError`).
//!
//! First-scope contract:
//! - One input PDF, one new searchable PDF written into a private work
//!   directory and moved to `task.output` after verification. Only the core
//!   queue finalizes `<stem>-searchable.pdf`; the original input is never
//!   modified.
//! - Fixed non-destructive argv (`--skip-text`, `--output-type pdf`,
//!   `--optimize 0`, `--jobs 1`, `--tesseract-timeout 180`, `--quiet`, `--`).
//!   Never passes `--force-ocr`, `--redo-ocr`, `--invalidate-digital-signatures`,
//!   or `--sidecar`.
//! - Digitally signed PDFs (exit 2 / `DigitalSignatureError`), encrypted PDFs
//!   (exit 8 / `EncryptedPdfError`), and corrupt or non-PDF inputs fail with
//!   `EngineError::Input` and commit no output.
//! - Per-page timeout warnings on stderr fail the task even if OCRmyPDF exits 0,
//!   committing no partial output.
//! - Admission and output bounds (policy values, not measured guarantees):
//!   128 MiB input PDF, 250 pages, 256 MiB output PDF, 180 s per-page OCR
//!   timeout, 600 s whole-job timeout, 5 s / 64 KiB version probe.

use crate::locate::Program;
use crate::locks::{read, write};
use crate::ocr::{
    self, Lookup, OcrEngine, TESSERACT_URL, intern_path, lookup_program, refresh_lookup,
};
use crate::office::{
    failure_message, input_error, missing, missing_on_not_found, move_file, single_input, work_dir,
};
use kawaro_core::engine::TaskOutcome;
use kawaro_core::unsupported::{ProbeCache, ProbeOutcome, UnavailableReason};
use kawaro_core::{CancelToken, Engine, EngineError, MediaKind, Progress, Task};
use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::{OnceLock, RwLock};
use std::time::Duration;

/// Where to get OCRmyPDF, for diagnostics when it is missing or unsupported.
pub const OCRMYPDF_URL: &str = "https://ocrmypdf.readthedocs.io/en/stable/installation.html";
/// Minimum supported OCRmyPDF version (default digital-signature refusal).
pub const MIN_VERSION: (u32, u32, u32) = (14, 4, 0);
/// The language choices the built-in action offers.
pub const SUPPORTED_LANGUAGES: [&str; 3] = ocr::SUPPORTED_LANGUAGES;
/// Budget of one bounded `ocrmypdf --version` probe.
pub const PROBE_TIMEOUT: Duration = ocr::PROBE_TIMEOUT;
/// Per-page Tesseract timeout passed to `--tesseract-timeout`, in seconds.
pub const PER_PAGE_TIMEOUT_SECS: u64 = 180;
/// Whole-job wall-clock timeout enforced by the bounded child runner.
pub const JOB_TIMEOUT: Duration = Duration::from_secs(600);
/// Maximum accepted input PDF file size, in bytes (128 MiB).
pub const MAX_INPUT_BYTES: u64 = 128 * 1024 * 1024;
/// Maximum accepted input PDF page count.
pub const MAX_INPUT_PAGES: usize = 250;
/// Maximum accepted output PDF file size, in bytes (256 MiB).
pub const MAX_OUTPUT_BYTES: u64 = 256 * 1024 * 1024;

/// Process-wide cache of `ocrmypdf --version` probes, keyed by tool path.
static VERSION_PROBE_CACHE: OnceLock<ProbeCache> = OnceLock::new();

fn version_cache() -> &'static ProbeCache {
    VERSION_PROBE_CACHE.get_or_init(ProbeCache::new)
}

/// The process-wide lookup of the OCRmyPDF program (S-151).
static OCRMYPDF: RwLock<Lookup> = RwLock::new(Lookup::Unknown);

/// The OCRmyPDF adapter.
pub struct OcrmypdfEngine {
    /// The resolved program, or none when the tool is not installed.
    program: RwLock<Option<&'static Path>>,
    /// True when the program was given by hand: a refresh never changes
    /// it.
    pinned: bool,
    tesseract: OcrEngine,
    temp_root: Option<PathBuf>,
    job_timeout: Duration,
    max_output_bytes: u64,
}

impl Default for OcrmypdfEngine {
    /// Uses `ocrmypdf` as `locate::find` sees it, from the process wide
    /// lookup: the answer of the last refresh, or a first lookup.
    fn default() -> Self {
        Self {
            program: RwLock::new(
                lookup_program(&OCRMYPDF, Program::Ocrmypdf).map(|path| intern_path(&path)),
            ),
            pinned: false,
            tesseract: OcrEngine::default(),
            temp_root: None,
            job_timeout: JOB_TIMEOUT,
            max_output_bytes: MAX_OUTPUT_BYTES,
        }
    }
}

impl OcrmypdfEngine {
    /// Looks up `ocrmypdf` and `tesseract` freshly from the current
    /// environment, without reusing the cached answer (S-145, S-151).
    ///
    /// The fresh answers also become the process wide ones, so an engine
    /// built later with [`Default`] sees the same programs.
    pub fn discover() -> Self {
        Self {
            program: RwLock::new(
                refresh_lookup(&OCRMYPDF, Program::Ocrmypdf).map(|path| intern_path(&path)),
            ),
            pinned: false,
            tesseract: OcrEngine::discover(),
            temp_root: None,
            job_timeout: JOB_TIMEOUT,
            max_output_bytes: MAX_OUTPUT_BYTES,
        }
    }

    /// Runs `program` instead of the detected `ocrmypdf`, with the default
    /// `tesseract` lookup.
    ///
    /// The OCRmyPDF program is fixed: a refresh never changes it.
    pub fn with_program(program: impl Into<PathBuf>) -> Self {
        let program = program.into();
        Self {
            program: RwLock::new(Some(intern_path(&program))),
            pinned: true,
            tesseract: OcrEngine::default(),
            temp_root: None,
            job_timeout: JOB_TIMEOUT,
            max_output_bytes: MAX_OUTPUT_BYTES,
        }
    }

    /// Runs explicit `ocrmypdf` and `tesseract` paths. For tests and unusual
    /// installations.
    ///
    /// Both programs are fixed: a refresh never changes them.
    pub fn with_programs(ocrmypdf: impl Into<PathBuf>, tesseract: impl Into<PathBuf>) -> Self {
        let ocrmypdf = ocrmypdf.into();
        let tesseract = tesseract.into();
        Self {
            program: RwLock::new(Some(intern_path(&ocrmypdf))),
            pinned: true,
            tesseract: OcrEngine::with_program(tesseract),
            temp_root: None,
            job_timeout: JOB_TIMEOUT,
            max_output_bytes: MAX_OUTPUT_BYTES,
        }
    }

    /// Directory for private task work directories. Default: the system temp dir.
    pub fn temp_root(mut self, dir: impl Into<PathBuf>) -> Self {
        self.temp_root = Some(dir.into());
        self
    }

    /// Overrides the whole-job wall-clock timeout. Default: [`JOB_TIMEOUT`].
    pub fn with_job_timeout(mut self, timeout: Duration) -> Self {
        self.job_timeout = timeout;
        self
    }

    /// Overrides the output PDF byte cap. Default: [`MAX_OUTPUT_BYTES`].
    pub fn with_max_output_bytes(mut self, bytes: u64) -> Self {
        self.max_output_bytes = bytes;
        self
    }

    /// The resolved `ocrmypdf` program, when the executable file exists.
    ///
    /// Availability is a lookup, never a child process.
    pub fn program(&self) -> Option<&Path> {
        self.program_path().filter(|path| path.is_file())
    }

    /// The resolved program of this engine, when there is one. It is the
    /// program of the last refresh, never a new lookup.
    fn program_path(&self) -> Option<&'static Path> {
        let guard = read(&self.program);
        *guard
    }

    /// The companion Tesseract handle used for language probing.
    pub fn tesseract(&self) -> &OcrEngine {
        &self.tesseract
    }

    /// Runs or reads the cached bounded `ocrmypdf --version` probe keyed by
    /// this engine's resolved tool path.
    ///
    /// Returns [`ProbeOutcome::Missing`] when absent, [`ProbeOutcome::Ready`]
    /// with the parsed version string when `>= 14.4.0`, and
    /// [`ProbeOutcome::Unknown`] on timeout, overflow, non-zero exit,
    /// malformed output, or versions below `14.4.0`. A cancelled probe returns
    /// [`ProbeOutcome::Unknown`] without caching the cancellation.
    pub fn probe_version(&self, cancel: &CancelToken) -> ProbeOutcome {
        if cancel.is_cancelled() {
            return ProbeOutcome::Unknown;
        }
        let Some(program) = self.program() else {
            return ProbeOutcome::Missing;
        };
        if let Some(cached) = version_cache().get(program) {
            return cached;
        }
        match run_version_probe(program, cancel) {
            Ok(outcome) => {
                version_cache().insert(program.to_path_buf(), outcome.clone());
                outcome
            }
            Err(_) => ProbeOutcome::Unknown,
        }
    }

    /// Reads the cached version probe without running a child. Used on the
    /// hive show path so a cold or hung probe never blocks the UI.
    pub fn cached_version_probe(&self) -> Option<ProbeOutcome> {
        self.program()
            .and_then(|program| version_cache().get(program))
    }

    /// Fills the version probe cache on a background thread.
    pub fn warm_version_probe(&self) {
        let engine = Self {
            program: RwLock::new(self.program_path()),
            pinned: false,
            tesseract: OcrEngine::default(),
            temp_root: None,
            job_timeout: self.job_timeout,
            max_output_bytes: self.max_output_bytes,
        };
        let _ = std::thread::Builder::new()
            .name(String::from("kawaro-ocrmypdf-probe"))
            .spawn(move || {
                let _ = engine.probe_version(&CancelToken::new());
            });
    }

    /// Invalidates this engine's cached version probe and runs a fresh bounded
    /// probe.
    pub fn refresh_version_probe(&self, cancel: &CancelToken) -> ProbeOutcome {
        let current = self.program_path();
        if let Some(program) = current {
            version_cache().invalidate(program);
        }
        self.probe_version(cancel)
    }

    /// Clears all cached `ocrmypdf --version` probe results.
    pub fn invalidate_version_cache() {
        version_cache().invalidate_all();
    }

    /// Clears the cached `ocrmypdf --version` probe result for one tool path.
    pub fn invalidate_version_cache_for(program: &Path) {
        version_cache().invalidate(program);
    }

    /// Preflights `ocrmypdf` presence and version (`>= 14.4.0`), `tesseract`
    /// presence, and requested language traineddata before any OCR runs.
    pub fn check_ready(
        &self,
        language: &str,
        cancel: &CancelToken,
    ) -> Result<(), UnavailableReason> {
        if ocr::language_components(language).is_none() {
            return Err(UnavailableReason::InvalidOptions);
        }
        if self.program().is_none() {
            return Err(UnavailableReason::MissingExecutable {
                program: "ocrmypdf".to_string(),
            });
        }
        if let Some(reason) = self.probe_version(cancel).into_reason("ocrmypdf") {
            return Err(reason);
        }
        self.tesseract.check_language(language, cancel)
    }

    fn make_searchable(&self, task: &Task, cancel: &CancelToken) -> Result<(), EngineError> {
        let language = ocr::language_param(task)?;
        let input = single_input(task)?;
        let meta = std::fs::metadata(input).map_err(|_| input_error(input, "no such file"))?;
        if !meta.is_file() {
            return Err(input_error(input, "no such file"));
        }
        if meta.len() == 0 {
            return Err(input_error(input, "empty PDF file"));
        }
        if meta.len() > MAX_INPUT_BYTES {
            return Err(input_error(
                input,
                format!(
                    "PDF is {} bytes, above the limit of {MAX_INPUT_BYTES} bytes",
                    meta.len()
                ),
            ));
        }

        let program = self
            .program_path()
            .ok_or_else(|| missing("OCRmyPDF", OCRMYPDF_URL, None))?;
        if !program.is_file() {
            return Err(missing("OCRmyPDF", OCRMYPDF_URL, Some(program)));
        }

        let abs_input = std::path::absolute(input).map_err(|e| input_error(input, e))?;
        if !looks_like_pdf(&abs_input)? {
            return Err(input_error(input, "not a PDF file"));
        }
        check_pdf_admission(&abs_input)?;

        cancel.check()?;
        let version_probe = self.probe_version(cancel);
        cancel.check()?;
        match version_probe {
            ProbeOutcome::Ready { .. } => {}
            ProbeOutcome::Missing => {
                return Err(missing("OCRmyPDF", OCRMYPDF_URL, Some(program)));
            }
            ProbeOutcome::Unknown => {
                return Err(EngineError::Unavailable(format!(
                    "OCRmyPDF {}.{}.{} or newer is required and the version probe failed or was below the floor; see {OCRMYPDF_URL}",
                    MIN_VERSION.0, MIN_VERSION.1, MIN_VERSION.2
                )));
            }
        }

        let lang_check = self.tesseract.check_language(&language, cancel);
        cancel.check()?;
        if let Err(reason) = lang_check {
            return Err(map_language_reason(
                &language,
                reason,
                self.tesseract.program(),
            ));
        }

        // One OCR child at a time per process, shared with OcrEngine.
        let _permit = ocr::acquire_permit(cancel)?;
        cancel.check()?;

        let work = work_dir(self.temp_root.as_deref(), "kawaro-ocrmypdf-")?;
        let work_path =
            std::path::absolute(work.path()).unwrap_or_else(|_| work.path().to_path_buf());
        let tmp_dir = work_path.join("tmp");
        std::fs::create_dir(&tmp_dir)?;
        let produced = work_path.join("searchable.pdf");

        let mut cmd = Command::new(program);
        cmd.args(argv(&abs_input, &produced, &language))
            .current_dir(&work_path)
            .env("TMPDIR", &tmp_dir)
            .env("TEMP", &tmp_dir)
            .env("TMP", &tmp_dir);

        let watched = produced.clone();
        let limit = self.max_output_bytes;
        let mut watch = || -> Result<(), EngineError> {
            if std::fs::metadata(&watched).is_ok_and(|m| m.is_file() && m.len() > limit) {
                return Err(EngineError::Failed(format!(
                    "OCRmyPDF wrote more than {limit} bytes; stopped"
                )));
            }
            Ok(())
        };

        // The per-page timeout warning is looked for while the child
        // writes, so a warning that is more than the kept stderr tail
        // before the end still fails the task (S-151, ADR 0017).
        let outcome = ocr::child::run_bounded(
            &mut cmd,
            cancel,
            self.job_timeout,
            false,
            Some(is_per_page_timeout_line),
            &mut watch,
        )
        .map_err(|e| missing_on_not_found(e, "OCRmyPDF", OCRMYPDF_URL, program))?;

        if !outcome.status.success() {
            return Err(map_child_error(input, outcome.status, &outcome.stderr));
        }

        if outcome.matched || has_per_page_timeout_warning(&outcome.stderr) {
            return Err(EngineError::Failed(failure_message(
                "OCRmyPDF did not complete every page (per-page OCR timeout)",
                outcome.status,
                &outcome.stderr,
            )));
        }

        validate_output(
            &produced,
            self.max_output_bytes,
            outcome.status,
            &outcome.stderr,
        )?;
        cancel.check()?;
        transfer_output(&produced, &task.output)?;
        work.close()?;
        Ok(())
    }
}

impl Engine for OcrmypdfEngine {
    fn id(&self) -> &'static str {
        "ocrmypdf"
    }

    fn available(&self) -> bool {
        self.program().is_some()
    }

    /// Looks `ocrmypdf` up again, so an install after the daemon started is
    /// seen (S-151). No child runs here.
    ///
    /// The program given by hand through [`Self::with_program`] and
    /// [`Self::with_programs`] never changes. The cached version probe is
    /// dropped, so the version floor is checked again for the tool that is
    /// there now.
    fn refresh(&self) {
        if !self.pinned {
            let found = refresh_lookup(&OCRMYPDF, Program::Ocrmypdf);
            *write(&self.program) = found.map(|path| intern_path(&path));
        }
        // The version floor is checked again after the refresh.
        Self::invalidate_version_cache();
        // The companion Tesseract handle sees its own new program.
        self.tesseract.refresh();
    }

    fn capabilities(&self) -> Vec<(&'static str, Vec<MediaKind>)> {
        vec![("make-searchable", vec![MediaKind::Pdf])]
    }

    fn run(
        &self,
        task: &Task,
        progress: Progress,
        cancel: &CancelToken,
    ) -> Result<TaskOutcome, EngineError> {
        progress(0.0);
        cancel.check()?;
        match task.op.as_str() {
            "make-searchable" => self.make_searchable(task, cancel)?,
            op => return Err(EngineError::UnsupportedOp(op.to_string())),
        }
        progress(1.0);
        Ok(TaskOutcome::Written)
    }
}

/// Fixed non-destructive argv for one OCRmyPDF run. Every value is one
/// distinct `OsString`, never shell-joined.
pub(crate) fn argv(input: &Path, output: &Path, language: &str) -> Vec<OsString> {
    vec![
        OsString::from("--language"),
        OsString::from(language),
        OsString::from("--skip-text"),
        OsString::from("--output-type"),
        OsString::from("pdf"),
        OsString::from("--optimize"),
        OsString::from("0"),
        OsString::from("--jobs"),
        OsString::from("1"),
        OsString::from("--tesseract-timeout"),
        OsString::from(PER_PAGE_TIMEOUT_SECS.to_string()),
        OsString::from("--quiet"),
        OsString::from("--"),
        input.as_os_str().to_os_string(),
        output.as_os_str().to_os_string(),
    ]
}

/// Parses a semantic-like `(major, minor, patch)` triple from `ocrmypdf --version`
/// output, tolerating optional leading tool names (such as `ocrmypdf 16.11.0`)
/// and Debian/PEP 440 suffixes (such as `15.2.0+dfsg1-1` or `17.0.0.post1`).
pub fn parse_version(stdout: &str) -> Option<(u32, u32, u32)> {
    let line = stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())?;
    for token in line.split_whitespace() {
        let token = token.strip_prefix('v').unwrap_or(token);
        let mut parts = token.split('.');
        let major_str = parts.next()?;
        let minor_str = match parts.next() {
            Some(part) => part,
            None => continue,
        };
        if major_str.is_empty() || !major_str.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let Ok(major) = major_str.parse::<u32>() else {
            continue;
        };
        let minor_digits: String = minor_str
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if minor_digits.is_empty() {
            continue;
        }
        let Ok(minor) = minor_digits.parse::<u32>() else {
            continue;
        };
        let patch = match parts.next() {
            Some(patch_str) => {
                let patch_digits: String = patch_str
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                if patch_digits.is_empty() {
                    0
                } else {
                    match patch_digits.parse::<u32>() {
                        Ok(value) => value,
                        Err(_) => continue,
                    }
                }
            }
            None => 0,
        };
        return Some((major, minor, patch));
    }
    None
}

/// Returns whether `version` meets the minimum safe floor (`>= 14.4.0`).
pub fn version_supported(version: (u32, u32, u32)) -> bool {
    version >= MIN_VERSION
}

/// Runs one bounded `ocrmypdf --version` probe (5 s, 64 KiB stdout cap) under
/// the shared OCR permit and maps it to a [`ProbeOutcome`].
fn run_version_probe(program: &Path, cancel: &CancelToken) -> Result<ProbeOutcome, EngineError> {
    let _permit = ocr::acquire_permit(cancel)?;
    cancel.check()?;
    let mut cmd = Command::new(program);
    cmd.arg("--version");
    match ocr::child::run_bounded(&mut cmd, cancel, PROBE_TIMEOUT, true, None, &mut || {
        Ok::<(), EngineError>(())
    }) {
        Err(EngineError::Cancelled) => Err(EngineError::Cancelled),
        Err(EngineError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(ProbeOutcome::Missing)
        }
        Err(_) => Ok(ProbeOutcome::Unknown),
        Ok(outcome) if !outcome.status.success() => Ok(ProbeOutcome::Unknown),
        Ok(outcome) => {
            let stdout = String::from_utf8_lossy(&outcome.stdout);
            match parse_version(&stdout) {
                Some(version) if version_supported(version) => Ok(ProbeOutcome::Ready {
                    items: vec![format!("{}.{}.{}", version.0, version.1, version.2)],
                }),
                _ => Ok(ProbeOutcome::Unknown),
            }
        }
    }
}

/// Checks that the first KiB of `path` contains the `%PDF-` header.
fn looks_like_pdf(path: &Path) -> Result<bool, EngineError> {
    let mut header = [0u8; 1024];
    let mut file = std::fs::File::open(path).map_err(|e| input_error(path, e))?;
    let read = file.read(&mut header).map_err(|e| input_error(path, e))?;
    Ok(header[..read].windows(5).any(|window| window == b"%PDF-"))
}

/// Enforces the encrypted-PDF refusal and 250-page admission cap when `lopdf`
/// can parse the input. Files `lopdf` cannot parse are left to `ocrmypdf` so
/// the child reports the real input error (signed, encrypted, or corrupt).
fn check_pdf_admission(path: &Path) -> Result<(), EngineError> {
    match lopdf::Document::load(path) {
        Ok(doc) if doc.is_encrypted() => Err(input_error(
            path,
            "encrypted or password-protected PDFs are not supported",
        )),
        Ok(doc) => {
            let pages = doc.get_pages().len();
            if pages > MAX_INPUT_PAGES {
                return Err(input_error(
                    path,
                    format!("PDF has {pages} pages, above the limit of {MAX_INPUT_PAGES} pages"),
                ));
            }
            Ok(())
        }
        Err(_) => Ok(()),
    }
}

/// Translates a typed Tesseract language preflight failure into an `EngineError`.
fn map_language_reason(
    requested: &str,
    reason: UnavailableReason,
    tesseract_path: Option<&Path>,
) -> EngineError {
    match reason {
        UnavailableReason::MissingExecutable { .. } => {
            missing("Tesseract", TESSERACT_URL, tesseract_path)
        }
        UnavailableReason::MissingLanguageData { language } => {
            let display = language.replace('+', " and ");
            EngineError::Unavailable(format!(
                "Tesseract is installed but the language data {display} is missing; \
                 install the {display} traineddata, see {TESSERACT_URL}"
            ))
        }
        UnavailableReason::InvalidOptions => EngineError::BadParam {
            name: "lang".into(),
            reason: format!(
                "`{requested}` is not one of {}",
                SUPPORTED_LANGUAGES.join(", ")
            ),
        },
        UnavailableReason::ProbeUnknown
        | UnavailableReason::UnsupportedInput
        | UnavailableReason::DisabledByPolicy => EngineError::Unavailable(format!(
            "Tesseract language probe failed or timed out; see {TESSERACT_URL}"
        )),
    }
}

/// Returns whether stderr contains a per-page OCR timeout or timeout-skip
/// warning. Normal `--skip-text` lines noting that a page already has text are
/// ignored so mixed born-digital/scanned PDFs succeed cleanly.
pub(crate) fn has_per_page_timeout_warning(stderr: &str) -> bool {
    stderr.lines().any(is_per_page_timeout_line)
}

/// Whether one stderr line reports a per-page OCR timeout or timeout-skip
/// warning.
///
/// The same rule as [`has_per_page_timeout_warning`], for one line: a normal
/// `--skip-text` line noting that a page already has text is ignored, so mixed
/// born-digital/scanned PDFs succeed cleanly.
///
/// The child runner calls this for every line while the child writes, so a
/// warning that is more than the kept stderr tail before the end is still
/// seen (S-151, ADR 0017).
pub(crate) fn is_per_page_timeout_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    if lower.contains("already has text") {
        return false;
    }
    lower.contains("timed out")
        || lower.contains("tesseract timeout")
        || lower.contains("tesseract-timeout")
        || lower.contains("timeout expired")
        || lower.contains("skipping ocr")
        || lower.contains("skipped due to timeout")
        || lower.contains("page timeout")
}

/// Maps OCRmyPDF exit codes and stderr diagnostics to typed [`EngineError`]s.
fn map_child_error(input: &Path, status: ExitStatus, stderr: &str) -> EngineError {
    classify_child_error(
        input,
        status.code(),
        &failure_message("ocrmypdf failed", status, stderr),
        &failure_message("invalid or unsupported PDF input", status, stderr),
        &failure_message(
            "OCRmyPDF is missing a required system dependency",
            status,
            stderr,
        ),
        &failure_message("OCRmyPDF produced an invalid PDF", status, stderr),
        stderr,
    )
}

fn classify_child_error(
    input: &Path,
    code: Option<i32>,
    generic_msg: &str,
    input_msg: &str,
    dep_msg: &str,
    invalid_out_msg: &str,
    stderr: &str,
) -> EngineError {
    let lower = stderr.to_ascii_lowercase();
    let is_signature =
        lower.contains("digitalsignatureerror") || lower.contains("digital signature");
    let is_encrypted = lower.contains("encryptedpdferror")
        || lower.contains("encrypted pdf")
        || lower.contains("password protected")
        || lower.contains("password-protected");

    if is_signature {
        return input_error(
            input,
            "digitally signed PDFs are refused to avoid invalidating signatures",
        );
    }
    if code == Some(8) || is_encrypted {
        return input_error(
            input,
            "encrypted or password-protected PDFs are not supported",
        );
    }
    match code {
        Some(2) => input_error(input, input_msg),
        Some(3) => EngineError::Unavailable(dep_msg.to_string()),
        Some(4) => EngineError::Failed(invalid_out_msg.to_string()),
        _ => EngineError::Failed(generic_msg.to_string()),
    }
}

/// Verifies the private output PDF exists, is non-empty, fits within
/// `max_output_bytes`, and begins with a `%PDF-` header.
fn validate_output(
    path: &Path,
    max_output_bytes: u64,
    status: ExitStatus,
    stderr: &str,
) -> Result<(), EngineError> {
    let meta = match std::fs::metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(EngineError::Failed(failure_message(
                "ocrmypdf wrote no PDF file",
                status,
                stderr,
            )));
        }
        Err(e) => {
            return Err(EngineError::Failed(format!(
                "cannot read the PDF {} OCRmyPDF produced: {e}",
                path.display()
            )));
        }
    };
    if !meta.is_file() || meta.len() == 0 {
        return Err(EngineError::Failed(failure_message(
            "ocrmypdf wrote an empty PDF file",
            status,
            stderr,
        )));
    }
    if meta.len() > max_output_bytes {
        return Err(EngineError::Failed(format!(
            "OCRmyPDF wrote {} bytes, above the limit of {max_output_bytes} bytes",
            meta.len()
        )));
    }
    if !looks_like_pdf(path)? {
        return Err(EngineError::Failed(failure_message(
            "ocrmypdf wrote a non-PDF output file",
            status,
            stderr,
        )));
    }
    Ok(())
}

/// Moves the verified private output PDF into `task.output` (which is already
/// the queue's temporary file), removing `task.output` if transfer fails.
fn transfer_output(from: &Path, to: &Path) -> Result<(), EngineError> {
    if let Err(err) = move_file(from, to) {
        let _ = std::fs::remove_file(to);
        return Err(err);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s150_parse_version_handles_release_and_distro_strings() {
        assert_eq!(parse_version("14.0.1\n"), Some((14, 0, 1)));
        assert_eq!(parse_version("14.0.1+dfsg1-1\n"), Some((14, 0, 1)));
        assert_eq!(parse_version("14.4.0\n"), Some((14, 4, 0)));
        assert_eq!(parse_version("15.2.0+dfsg1-1\n"), Some((15, 2, 0)));
        assert_eq!(parse_version("16.7.0+dfsg1-3\n"), Some((16, 7, 0)));
        assert_eq!(parse_version("ocrmypdf 16.11.0\n"), Some((16, 11, 0)));
        assert_eq!(parse_version("17.0.0.post1\n"), Some((17, 0, 0)));
        assert_eq!(parse_version("14.4\n"), Some((14, 4, 0)));
        assert_eq!(parse_version(""), None);
        assert_eq!(parse_version("not a version"), None);
        assert_eq!(parse_version("14"), None);
    }

    #[test]
    fn s150_version_floor_rejects_below_14_4_0() {
        assert!(!version_supported((14, 0, 1)));
        assert!(!version_supported((14, 3, 9)));
        assert!(version_supported((14, 4, 0)));
        assert!(version_supported((15, 2, 0)));
        assert!(version_supported((16, 7, 0)));
        assert!(version_supported((17, 0, 0)));
    }

    #[test]
    fn s150_argv_uses_fixed_non_destructive_flags_and_distinct_args() {
        for lang in SUPPORTED_LANGUAGES {
            let args = argv(
                Path::new("/tmp/my scans/-weird file.pdf"),
                Path::new("/tmp/work/searchable.pdf"),
                lang,
            );
            let strs: Vec<&str> = args.iter().map(|a| a.to_str().unwrap()).collect();
            assert_eq!(
                strs,
                [
                    "--language",
                    lang,
                    "--skip-text",
                    "--output-type",
                    "pdf",
                    "--optimize",
                    "0",
                    "--jobs",
                    "1",
                    "--tesseract-timeout",
                    "180",
                    "--quiet",
                    "--",
                    "/tmp/my scans/-weird file.pdf",
                    "/tmp/work/searchable.pdf",
                ]
            );
            for forbidden in [
                "--force-ocr",
                "--redo-ocr",
                "--invalidate-digital-signatures",
                "--sidecar",
                "--rotate-pages",
                "--deskew",
                "--clean",
            ] {
                assert!(!strs.contains(&forbidden), "{forbidden} in {strs:?}");
            }
        }
    }

    #[test]
    fn s150_per_page_timeout_warning_detector_ignores_normal_skip_text() {
        assert!(!has_per_page_timeout_warning(""));
        assert!(!has_per_page_timeout_warning(
            "1: page already has text! - skipping all processing on this page\n"
        ));
        assert!(has_per_page_timeout_warning(
            "WARNING - 2: Tesseract timed out\n"
        ));
        assert!(has_per_page_timeout_warning(
            "WARNING - Page 3: skipping OCR on this page\n"
        ));
        assert!(has_per_page_timeout_warning(
            "WARNING - Page 4 skipped due to timeout\n"
        ));
        assert!(has_per_page_timeout_warning(
            "ERROR - tesseract-timeout reached\n"
        ));
    }

    #[test]
    fn s151_the_per_page_timeout_matcher_works_on_one_line() {
        assert!(is_per_page_timeout_line("WARNING - 2: Tesseract timed out"));
        assert!(is_per_page_timeout_line(
            "ERROR - tesseract-timeout reached"
        ));
        assert!(!is_per_page_timeout_line(
            "1: page already has text! - skipping all processing on this page"
        ));
        assert!(!is_per_page_timeout_line(""));
        // The whole report and the single line agree.
        let report = "1: page already has text!\nWARNING - 2: Tesseract timed out\n";
        assert!(has_per_page_timeout_warning(report));
        assert!(report.lines().any(is_per_page_timeout_line));
    }

    #[test]
    fn s150_exit_code_classification_maps_signed_encrypted_and_corrupt_to_input() {
        let p = Path::new("/tmp/doc.pdf");
        match classify_child_error(
            p,
            Some(2),
            "generic",
            "invalid",
            "dep",
            "out",
            "DigitalSignatureError: PDF has a digital signature",
        ) {
            EngineError::Input { path, reason } => {
                assert_eq!(path, p);
                assert!(reason.contains("digitally signed"), "{reason}");
            }
            other => panic!("expected Input for signed PDF, got {other:?}"),
        }
        match classify_child_error(
            p,
            Some(8),
            "generic",
            "invalid",
            "dep",
            "out",
            "EncryptedPdfError: PDF is encrypted",
        ) {
            EngineError::Input { path, reason } => {
                assert_eq!(path, p);
                assert!(reason.contains("encrypted"), "{reason}");
            }
            other => panic!("expected Input for encrypted PDF, got {other:?}"),
        }
        match classify_child_error(p, Some(2), "generic", "invalid", "dep", "out", "bad pdf") {
            EngineError::Input { path, .. } => assert_eq!(path, p),
            other => panic!("expected Input for exit 2, got {other:?}"),
        }
        match classify_child_error(p, Some(3), "generic", "invalid", "dep", "out", "no gs") {
            EngineError::Unavailable(msg) => assert_eq!(msg, "dep"),
            other => panic!("expected Unavailable for exit 3, got {other:?}"),
        }
        match classify_child_error(p, Some(4), "generic", "invalid", "dep", "out", "bad out") {
            EngineError::Failed(msg) => assert_eq!(msg, "out"),
            other => panic!("expected Failed for exit 4, got {other:?}"),
        }
    }
}
