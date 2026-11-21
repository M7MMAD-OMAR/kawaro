//! Printed text from a still picture through the system Tesseract (S-146).
//!
//! Tesseract stays a system program: nothing is linked, nothing is
//! downloaded and no model or language data is fetched. The action is
//! hidden while the executable is missing, exactly like the other child
//! engines.
//!
//! One task, one picture, one child. The source is prepared by
//! [`crate::image::ocr_prep`], which reuses the image loader, so the child
//! sees the oriented, white flattened picture as one private PNG. The child
//! runs as `tesseract <private png> <private base> -l <language> txt` with
//! every argument separate, so no file name and no language string is ever
//! read as an option or through a shell.
//!
//! The recognized text is one UTF-8 file. Its bytes are written unchanged:
//! no digit, diacritic, spelling or reading order is normalized here. A
//! whitespace only or empty result is the diagnostic `No printed text
//! recognized` and no output file. A missing file is its own error, and a
//! file over 8 MiB is refused rather than truncated.
//!
//! The child runs through the bounded runner in `child`, which reuses the
//! shared `crate::process::ProcessTree`: stdout is drained in fixed chunks,
//! the probe capture is capped at 64 KiB and an overflow is an error, stderr
//! keeps only its last 4 KiB, and the whole tree is killed and reaped on
//! every exit path. The extraction also watches the output file length while
//! the child runs, best effort between polls, and then reads it through a
//! hard `Read::take` bound before any UTF-8 validation.
//!
//! Limits, all of them admission policy and none of them a measurement: one
//! child at a time per process, a 5 second language probe, a 240 second
//! extraction, 64 MiB of source bytes, 16,000,000 source pixels and 8 MiB of
//! recognized text.

use crate::image::ocr_prep;
use crate::locate::{self, Program};
use crate::locks::{read, write};
use crate::office::{
    failure_message, input_error, missing, missing_on_not_found, single_input, work_dir,
};
use kawaro_core::engine::TaskOutcome;
use kawaro_core::unsupported::{ProbeCache, ProbeOutcome, UnavailableReason};
use kawaro_core::{CancelToken, Engine, EngineError, MediaKind, Progress, Task};
use std::ffi::OsString;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::{Mutex, MutexGuard, OnceLock, RwLock, TryLockError};
use std::time::Duration;

pub(crate) mod child;

/// Where to get Tesseract, for the message when it is missing.
pub const TESSERACT_URL: &str = "https://tesseract-ocr.github.io/tessdoc/";
/// The language choices the built-in action offers.
pub const SUPPORTED_LANGUAGES: [&str; 3] = ["eng", "ara", "ara+eng"];
/// The largest recognized text file accepted, in bytes.
pub const MAX_OUTPUT_BYTES: u64 = 8 * 1024 * 1024;
/// The budget of one `--list-langs` child.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
/// The budget of one extraction child.
pub const EXTRACTION_TIMEOUT: Duration = Duration::from_secs(240);
/// The diagnostic of a picture the engine read with no printed text.
pub const NO_TEXT: &str = "No printed text recognized";

/// The poll of the permit wait and of the child loop.
const POLL: Duration = Duration::from_millis(25);
/// The most language lines read from the probe.
const MAX_LISTED_LINES: usize = 4096;
/// The language data the adapter can check, the subset of the installed
/// languages it knows.
const KNOWN_LANGUAGES: [&str; 2] = ["ara", "eng"];

/// One Tesseract child at a time in this process. Every job of every queue
/// goes through it, so a batch of pictures never starts a second child.
static PERMIT: Mutex<()> = Mutex::new(());

/// Process-wide cache of `tesseract --list-langs` probes, keyed by tool path
/// (S-145, S-146). Callers refresh or invalidate it explicitly after a tool or
/// language pack install.
static LANG_PROBE_CACHE: OnceLock<ProbeCache> = OnceLock::new();

fn lang_cache() -> &'static ProbeCache {
    LANG_PROBE_CACHE.get_or_init(ProbeCache::new)
}

/// The result of the process-wide program lookup (S-151).
///
/// The lookup runs once per process. A refresh looks the program up again
/// and keeps the new answer, so a tool installed after the daemon started
/// is seen by every engine.
#[derive(Clone, Debug)]
pub(crate) enum Lookup {
    /// The lookup never ran.
    Unknown,
    /// The lookup ran and the program is not installed.
    Missing,
    /// The lookup found the program.
    Found(PathBuf),
}

/// The process-wide lookup of the Tesseract program (S-151).
static TESSERACT: RwLock<Lookup> = RwLock::new(Lookup::Unknown);

/// The program paths this process has looked up (S-151).
///
/// They stay for the life of the process, so the reference the public
/// accessor hands out stays valid while the engine runs. One entry per
/// distinct path keeps this small.
static PATHS: RwLock<Vec<&'static Path>> = RwLock::new(Vec::new());

/// The path of `path`, kept for the life of the process. The same path
/// always gives the same reference.
pub(crate) fn intern_path(path: &Path) -> &'static Path {
    let mut paths = write(&PATHS);
    if let Some(known) = paths
        .iter()
        .copied()
        .find(|known| known.as_os_str() == path.as_os_str())
    {
        return known;
    }
    let kept: &'static Path = Box::leak(path.to_path_buf().into_boxed_path());
    paths.push(kept);
    kept
}

/// The program of the process-wide lookup, looked up once (S-151).
///
/// `cache` holds the answer of the last lookup, so a program installed
/// later is only seen after a refresh.
pub(crate) fn lookup_program(cache: &RwLock<Lookup>, program: Program) -> Option<PathBuf> {
    // The answer of the last lookup, when there is one. The read lock is
    // released before any write, so the two never wait for each other.
    let cached: Option<Option<PathBuf>> = match &*read(cache) {
        Lookup::Found(path) => Some(Some(path.clone())),
        Lookup::Missing => Some(None),
        Lookup::Unknown => None,
    };
    match cached {
        Some(found) => found,
        None => {
            let found = locate::find(program);
            *write(cache) = match &found {
                Some(path) => Lookup::Found(path.clone()),
                None => Lookup::Missing,
            };
            found
        }
    }
}

/// Looks the program up again and keeps the new answer for the whole
/// process (S-151).
pub(crate) fn refresh_lookup(cache: &RwLock<Lookup>, program: Program) -> Option<PathBuf> {
    let found = locate::find(program);
    *write(cache) = match &found {
        Some(path) => Lookup::Found(path.clone()),
        None => Lookup::Missing,
    };
    found
}

/// The right to run the one child, released on drop. The guard is held for
/// its drop: the permit is the absence of a second child.
pub(crate) struct Permit {
    _guard: MutexGuard<'static, ()>,
}

/// Takes the child permit, checking `cancel` before the wait, while waiting
/// and after it. A poisoned lock is still a lock: a holder that panicked is
/// gone and the child limit stays at one.
pub(crate) fn acquire_permit(cancel: &CancelToken) -> Result<Permit, EngineError> {
    cancel.check()?;
    loop {
        // A poisoned lock is still a lock: a holder that panicked is gone and
        // the child limit stays at one.
        let guard = match PERMIT.try_lock() {
            Ok(guard) => guard,
            Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
            Err(TryLockError::WouldBlock) => {
                cancel.check()?;
                std::thread::sleep(POLL);
                continue;
            }
        };
        // The cancel may have fired while this thread waited for the permit.
        // The guard drops on this return, so the permit is released.
        cancel.check()?;
        return Ok(Permit { _guard: guard });
    }
}

/// The Tesseract adapter. The program is looked up again by
/// [`Engine::refresh`], so a tool installed after the daemon started is
/// seen.
pub struct OcrEngine {
    /// The resolved program, or none when the tool is not installed.
    program: RwLock<Option<&'static Path>>,
    /// True when the program was given by hand: a refresh never changes
    /// it.
    pinned: bool,
}

impl Default for OcrEngine {
    /// Uses `tesseract` as `locate::find` sees it, from the process wide
    /// lookup: the answer of the last refresh, or a first lookup.
    fn default() -> Self {
        Self {
            program: RwLock::new(
                lookup_program(&TESSERACT, Program::Tesseract).map(|path| intern_path(&path)),
            ),
            pinned: false,
        }
    }
}

impl OcrEngine {
    /// Looks up `tesseract` freshly from the current environment without
    /// reusing the cached answer, for explicit capability refreshes after
    /// installation or PATH changes (S-145, S-151).
    ///
    /// The fresh answer also becomes the process wide one, so an engine
    /// built later with [`Default`] sees the same program.
    pub fn discover() -> Self {
        Self {
            program: RwLock::new(
                refresh_lookup(&TESSERACT, Program::Tesseract).map(|path| intern_path(&path)),
            ),
            pinned: false,
        }
    }

    /// Runs this program instead of the detected `tesseract`. For tests and
    /// for unusual installations.
    ///
    /// The program is fixed: a refresh never changes it.
    pub fn with_program(program: impl Into<PathBuf>) -> Self {
        let program = program.into();
        Self {
            program: RwLock::new(Some(intern_path(&program))),
            pinned: true,
        }
    }

    /// The resolved program of this engine, when there is one. It is the
    /// program of the last refresh, never a new lookup.
    fn program_path(&self) -> Option<&'static Path> {
        let guard = read(&self.program);
        *guard
    }

    /// The resolved program, when the executable is there.
    ///
    /// This is the availability of the engine: a lookup, never a child. A
    /// missing language is not known here; it is checked inside the task or
    /// through [`Self::check_language`].
    pub fn program(&self) -> Option<&Path> {
        self.program_path().filter(|path| path.is_file())
    }

    /// Runs or reads the cached bounded `tesseract --list-langs` probe keyed
    /// by this engine's resolved tool path (S-145, S-146).
    ///
    /// Never installs software, never runs `sudo`, and never treats a
    /// timed-out or failing probe as ready. A cancelled probe returns
    /// [`ProbeOutcome::Unknown`] without caching the cancellation.
    pub fn probe_languages(&self, cancel: &CancelToken) -> ProbeOutcome {
        if cancel.is_cancelled() {
            return ProbeOutcome::Unknown;
        }
        let Some(program) = self.program() else {
            return ProbeOutcome::Missing;
        };
        if let Some(cached) = lang_cache().get(program) {
            return cached;
        }
        match run_language_probe(program, cancel) {
            Ok(outcome) => {
                lang_cache().insert(program.to_path_buf(), outcome.clone());
                outcome
            }
            Err(_) => ProbeOutcome::Unknown,
        }
    }

    /// The cached language probe of this engine's tool, without running a
    /// child. The hive show path reads only this, so a cold or hung probe can
    /// never delay the hive (150 ms budget).
    pub fn cached_language_probe(&self) -> Option<ProbeOutcome> {
        self.program().and_then(|program| lang_cache().get(program))
    }

    /// Fills the language probe cache on a background thread, so the first
    /// hive show already has an answer. A thread that cannot start leaves
    /// the cache cold; the adapter still preflights before any OCR runs.
    pub fn warm_language_probe(&self) {
        let engine = Self {
            program: RwLock::new(self.program_path()),
            pinned: false,
        };
        let _ = std::thread::Builder::new()
            .name(String::from("kawaro-ocr-probe"))
            .spawn(move || {
                let _ = engine.probe_languages(&CancelToken::new());
            });
    }

    /// Invalidates this engine's cached language probe and runs a fresh
    /// bounded probe.
    pub fn refresh_language_probe(&self, cancel: &CancelToken) -> ProbeOutcome {
        let current = self.program_path();
        if let Some(program) = current {
            lang_cache().invalidate(program);
        }
        self.probe_languages(cancel)
    }

    /// Clears all cached OCR language probe results.
    pub fn invalidate_language_cache() {
        lang_cache().invalidate_all();
    }

    /// Clears the cached OCR language probe result for one tool path.
    pub fn invalidate_language_cache_for(program: &Path) {
        lang_cache().invalidate(program);
    }

    /// Preflights a requested `language` option before any OCR runs, returning
    /// a typed [`UnavailableReason`] when the executable, language data,
    /// option value, or probe is not ready (S-145, S-146).
    pub fn check_language(
        &self,
        language: &str,
        cancel: &CancelToken,
    ) -> Result<(), UnavailableReason> {
        let Some(components) = language_components(language) else {
            return Err(UnavailableReason::InvalidOptions);
        };
        if self.program().is_none() {
            return Err(UnavailableReason::MissingExecutable {
                program: "tesseract".to_string(),
            });
        }
        match self
            .probe_languages(cancel)
            .check_language("tesseract", &components)
        {
            None => Ok(()),
            Some(reason) => Err(reason),
        }
    }

    /// The language choices of the installed Tesseract, in the order of
    /// [`SUPPORTED_LANGUAGES`].
    ///
    /// This runs one bounded `--list-langs` child and is not part of
    /// availability. It returns only the choices whose every component is
    /// installed, so `ara+eng` appears only when both are.
    pub fn installed_languages(
        &self,
        cancel: &CancelToken,
    ) -> Result<Vec<&'static str>, EngineError> {
        let program = self
            .program_path()
            .ok_or_else(|| missing("Tesseract", TESSERACT_URL, None))?;
        if !program.is_file() {
            return Err(missing("Tesseract", TESSERACT_URL, Some(program)));
        }
        // The same one child at a time rule as a job, so a surface that asks
        // for the languages never runs a second child beside a running task.
        let _permit = acquire_permit(cancel)?;
        cancel.check()?;
        let installed = list_languages(program, cancel)?;
        Ok(SUPPORTED_LANGUAGES
            .into_iter()
            .filter(|choice| missing_languages(choice, &installed).is_none())
            .collect())
    }

    /// The installed languages of `language`, from the cached probe when it
    /// holds every part of the language, and from one fresh bounded probe
    /// otherwise (S-151).
    ///
    /// A cache miss, or a cached list without the requested language, runs
    /// the probe again: the language data may have been installed since the
    /// cache was filled. The fresh answer replaces the cached one.
    fn installed_languages_cached(
        &self,
        language: &str,
        program: &Path,
        cancel: &CancelToken,
    ) -> Result<Vec<&'static str>, EngineError> {
        let cached = lang_cache().get(program);
        if let Some(components) = language_components(language)
            && let Some(ProbeOutcome::Ready { items }) = &cached
        {
            let listed = components
                .iter()
                .copied()
                .filter(|component| items.iter().any(|item| item.as_str() == *component))
                .count();
            if listed == components.len() {
                return Ok(components);
            }
        }
        let fresh = list_languages(program, cancel)?;
        lang_cache().insert(
            program.to_path_buf(),
            ProbeOutcome::Ready {
                items: fresh.iter().map(|item| (*item).to_string()).collect(),
            },
        );
        Ok(fresh)
    }

    /// The `to-text` op: prepare the picture, run the child and write its
    /// text atomically to `task.output`.
    fn to_text(&self, task: &Task, cancel: &CancelToken) -> Result<(), EngineError> {
        let language = language_param(task)?;
        let input = single_input(task)?;
        if !input.is_file() {
            return Err(input_error(input, "no such file"));
        }
        let program = self
            .program_path()
            .ok_or_else(|| missing("Tesseract", TESSERACT_URL, None))?;
        if !program.is_file() {
            return Err(missing("Tesseract", TESSERACT_URL, Some(program)));
        }
        // One child at a time, and the wait is cancellable.
        let _permit = acquire_permit(cancel)?;
        // The cancel may have fired while the permit was waited for.
        cancel.check()?;
        // The language data is checked inside the task, before any image prep
        // or OCR extraction child starts. The cached probe is used when it
        // holds the requested language, so a batch of pictures runs no probe
        // per job (S-151).
        let installed = self.installed_languages_cached(&language, program, cancel)?;
        if let Some(missing_langs) = missing_languages(&language, &installed) {
            return Err(EngineError::Unavailable(format!(
                "Tesseract is installed but the language data {} is missing; \
                 install the {} traineddata, see {TESSERACT_URL}",
                missing_langs.join(" and "),
                missing_langs.join(" and "),
            )));
        }
        let work = work_dir(None, "kawaro-ocr-")?;
        let work_path =
            std::path::absolute(work.path()).unwrap_or_else(|_| work.path().to_path_buf());
        let picture = work_path.join("source.png");
        let prepared = ocr_prep::prepare(input, &picture, cancel)?;
        // The text file name is fixed inside the private directory: it is
        // never derived from the document or from its text.
        let base = work_path.join("text");
        let output = base.with_extension("txt");
        let mut cmd = Command::new(program);
        cmd.args(argv(&prepared.path, &base, &language))
            .current_dir(&work_path);
        // Best effort, not a hard disk quota: the file length is read between
        // polls, so Tesseract can write past the limit before the next poll
        // sees it. The bounded read below is the exact limit.
        let watched = output.clone();
        let mut watch = || -> Result<(), EngineError> {
            if std::fs::metadata(&watched)
                .is_ok_and(|meta| meta.is_file() && meta.len() > MAX_OUTPUT_BYTES)
            {
                return Err(EngineError::Failed(format!(
                    "Tesseract wrote more than {MAX_OUTPUT_BYTES} bytes of text; stopped"
                )));
            }
            Ok(())
        };
        let outcome = child::run_bounded(
            &mut cmd,
            cancel,
            EXTRACTION_TIMEOUT,
            false,
            None,
            &mut watch,
        )
        .map_err(|e| missing_on_not_found(e, "Tesseract", TESSERACT_URL, program))?;
        if !outcome.status.success() {
            return Err(EngineError::Failed(failure_message(
                "tesseract failed",
                outcome.status,
                &outcome.stderr,
            )));
        }
        // A zero exit alone is not proof: the text file must be there, and it
        // is read under a hard byte bound before it is validated. An empty
        // file reaches the no-text diagnostic, not a missing-file error.
        let bytes = read_output(&output, outcome.status, &outcome.stderr)?;
        // The bytes are validated, not rebuilt: the recognized text keeps
        // the characters the engine returned.
        recognized_text(&bytes)?;
        cancel.check()?;
        // `task.output` is the queue's temp path; the queue finalizes it.
        std::fs::write(&task.output, &bytes).map_err(EngineError::Io)?;
        Ok(())
    }
}

impl Engine for OcrEngine {
    fn id(&self) -> &'static str {
        "ocr"
    }

    fn available(&self) -> bool {
        self.program().is_some()
    }

    /// Looks `tesseract` up again, so an install after the daemon started is
    /// seen (S-151). No child runs here.
    ///
    /// The program given by hand through [`Self::with_program`] never
    /// changes. The cached language probe of the old and of the new program
    /// is dropped, so the next probe answers for the tool that is there.
    fn refresh(&self) {
        if self.pinned {
            return;
        }
        let previous = self.program_path();
        let found = refresh_lookup(&TESSERACT, Program::Tesseract);
        if let Some(previous) = previous {
            lang_cache().invalidate(previous);
        }
        if let Some(path) = &found {
            lang_cache().invalidate(path);
        }
        *write(&self.program) = found.map(|path| intern_path(&path));
    }

    fn capabilities(&self) -> Vec<(&'static str, Vec<MediaKind>)> {
        vec![("to-text", vec![MediaKind::Image])]
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
            "to-text" => self.to_text(task, cancel)?,
            op => return Err(EngineError::UnsupportedOp(op.to_string())),
        }
        progress(1.0);
        Ok(TaskOutcome::Written)
    }
}

/// The argv of one extraction: the private PNG, the private output base, the
/// explicit language and the `txt` config name. Every value is one argument,
/// so a path with spaces or dashes stays one argument.
pub(crate) fn argv(input: &Path, output_base: &Path, language: &str) -> Vec<OsString> {
    vec![
        input.as_os_str().to_os_string(),
        output_base.as_os_str().to_os_string(),
        OsString::from("-l"),
        OsString::from(language),
        OsString::from("txt"),
    ]
}

/// The `lang` param: one of [`SUPPORTED_LANGUAGES`], exact.
pub(crate) fn language_param(task: &Task) -> Result<String, EngineError> {
    let bad = |reason: String| EngineError::BadParam {
        name: "lang".into(),
        reason,
    };
    let value = task
        .params
        .get("lang")
        .ok_or_else(|| bad("set `lang` to one of eng, ara or ara+eng".into()))?;
    let text = value
        .as_str()
        .ok_or_else(|| bad("must be a string: eng, ara or ara+eng".into()))?;
    if !SUPPORTED_LANGUAGES.contains(&text) {
        return Err(bad(format!(
            "`{text}` is not one of {}",
            SUPPORTED_LANGUAGES.join(", ")
        )));
    }
    Ok(text.to_string())
}

/// The traineddata a choice needs. `None` for an unknown choice, which the
/// param check already refused.
pub(crate) fn language_components(language: &str) -> Option<Vec<&'static str>> {
    match language {
        "eng" => Some(vec!["eng"]),
        "ara" => Some(vec!["ara"]),
        "ara+eng" => Some(vec!["ara", "eng"]),
        _ => None,
    }
}

/// The components of `language` that `installed` does not hold.
///
/// `None` when every component is installed, which includes an unknown
/// language, so the caller reports what it knows.
pub(crate) fn missing_languages(language: &str, installed: &[&str]) -> Option<Vec<&'static str>> {
    let components = language_components(language)?;
    let missing: Vec<&'static str> = components
        .into_iter()
        .filter(|component| !installed.contains(component))
        .collect();
    (!missing.is_empty()).then_some(missing)
}

/// The languages a `--list-langs` report lists, in the order they appear.
///
/// Only the languages the adapter knows are returned, and only when the line
/// is one identifier: the header line, the count and any prose are ignored,
/// so a report the adapter does not understand gives no language rather than
/// a guessed one.
pub(crate) fn parse_listed_languages(stdout: &str) -> Vec<&'static str> {
    let mut found: Vec<&'static str> = Vec::new();
    for line in stdout.lines().take(MAX_LISTED_LINES) {
        let line = line.trim();
        if line.is_empty() || line.ends_with(':') {
            continue;
        }
        // One identifier on the line, no argument and no description.
        if line.split_whitespace().count() != 1 {
            continue;
        }
        if !line
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        {
            continue;
        }
        if let Some(known) = KNOWN_LANGUAGES.iter().find(|known| **known == line)
            && !found.contains(known)
        {
            found.push(*known);
        }
    }
    found
}

/// Runs one bounded `tesseract --list-langs` and returns the languages it
/// lists. The whole tree is killed on cancel, on the timeout and after a
/// normal exit.
fn list_languages(program: &Path, cancel: &CancelToken) -> Result<Vec<&'static str>, EngineError> {
    let mut cmd = Command::new(program);
    cmd.arg("--list-langs");
    let outcome = child::run_bounded(&mut cmd, cancel, PROBE_TIMEOUT, true, None, &mut || {
        Ok::<(), EngineError>(())
    })
    .map_err(|e| missing_on_not_found(e, "Tesseract", TESSERACT_URL, program))?;
    if !outcome.status.success() {
        return Err(EngineError::Unavailable(format!(
            "Tesseract could not list its languages ({}): {}",
            outcome.status,
            failure_message(
                "tesseract --list-langs failed",
                outcome.status,
                &outcome.stderr
            )
        )));
    }
    let stdout = String::from_utf8_lossy(&outcome.stdout);
    Ok(parse_listed_languages(&stdout))
}

/// Reads the text file under a hard bound, so a growing file is never read
/// whole. A missing file is the distinct "wrote no text file" error; an empty
/// file returns no bytes and reaches [`recognized_text`], which gives the
/// no-text diagnostic.
fn read_output(path: &Path, status: ExitStatus, stderr: &str) -> Result<Vec<u8>, EngineError> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(EngineError::Failed(failure_message(
                "tesseract wrote no text file",
                status,
                stderr,
            )));
        }
        Err(e) => return Err(output_error(path, e)),
    };
    let mut bytes = Vec::new();
    let mut limited = file.take(MAX_OUTPUT_BYTES + 1);
    limited
        .read_to_end(&mut bytes)
        .map_err(|e| output_error(path, e))?;
    if bytes.len() as u64 > MAX_OUTPUT_BYTES {
        return Err(EngineError::Failed(format!(
            "the recognized text is above the limit of {MAX_OUTPUT_BYTES} bytes"
        )));
    }
    Ok(bytes)
}

/// The recognized text, checked: at most [`MAX_OUTPUT_BYTES`] bytes, valid
/// UTF-8 and not whitespace only.
pub(crate) fn recognized_text(bytes: &[u8]) -> Result<&str, EngineError> {
    if bytes.len() as u64 > MAX_OUTPUT_BYTES {
        return Err(EngineError::Failed(format!(
            "the recognized text is {} bytes, above the limit of {MAX_OUTPUT_BYTES} bytes",
            bytes.len()
        )));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| EngineError::Failed("the recognized text is not valid UTF-8".to_string()))?;
    if text.trim().is_empty() {
        return Err(EngineError::Failed(NO_TEXT.to_string()));
    }
    Ok(text)
}

/// Runs one bounded `tesseract --list-langs` probe and translates its result into a [`ProbeOutcome`].
///
/// Returns `Err(EngineError::Cancelled)` when cancelled so callers do not
/// cache a cancellation as a broken tool. Timeouts, non-zero exit codes, and
/// oversized reports become [`ProbeOutcome::Unknown`], never ready.
fn run_language_probe(program: &Path, cancel: &CancelToken) -> Result<ProbeOutcome, EngineError> {
    // No run permit: the probe is a short bounded child, so a discovery call
    // never waits for a running extraction.
    cancel.check()?;
    let mut cmd = Command::new(program);
    cmd.arg("--list-langs");
    match child::run_bounded(&mut cmd, cancel, PROBE_TIMEOUT, true, None, &mut || {
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
            let items = parse_listed_languages(&stdout)
                .into_iter()
                .map(String::from)
                .collect();
            Ok(ProbeOutcome::Ready { items })
        }
    }
}

/// A text file the child left that cannot be read.
fn output_error(path: &Path, error: std::io::Error) -> EngineError {
    EngineError::Failed(format!(
        "cannot read the text {} Tesseract produced: {error}",
        path.display()
    ))
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
    fn s146b_a_cancel_while_waiting_for_the_permit_is_honoured() {
        // Hold the one permit, then cancel while a second caller waits. The
        // wait checks the cancel token, so the second caller returns
        // Cancelled and the held permit is released by its own drop.
        let held = acquire_permit(&CancelToken::new()).expect("the permit is free");
        let cancel = CancelToken::new();
        let watcher = {
            let cancel = cancel.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(50));
                cancel.cancel();
            })
        };
        let r = acquire_permit(&cancel);
        watcher.join().unwrap();
        assert!(matches!(r, Err(EngineError::Cancelled)));
        drop(held);
    }

    #[test]
    fn s146_the_argv_is_one_argument_per_value() {
        let args = argv(
            Path::new("/tmp/a b/-weird.png"),
            Path::new("/tmp/work/text"),
            "ara+eng",
        );
        let args: Vec<&str> = args.iter().map(|a| a.to_str().unwrap()).collect();
        assert_eq!(
            args,
            [
                "/tmp/a b/-weird.png",
                "/tmp/work/text",
                "-l",
                "ara+eng",
                "txt",
            ]
        );
    }

    #[test]
    fn s146_the_language_param_is_one_of_the_supported_choices() {
        assert_eq!(language_param(&task("lang = \"eng\"")).unwrap(), "eng");
        assert_eq!(language_param(&task("lang = \"ara\"")).unwrap(), "ara");
        assert_eq!(
            language_param(&task("lang = \"ara+eng\"")).unwrap(),
            "ara+eng"
        );
        for bad in [
            "",
            "lang = \"eng+ara\"",
            "lang = \"deu\"",
            "lang = \"ara+eng+deu\"",
            "lang = \"\"",
            "lang = 1",
        ] {
            match language_param(&task(bad)) {
                Err(EngineError::BadParam { name, .. }) => assert_eq!(name, "lang", "{bad}"),
                other => panic!("{bad}: expected BadParam, got {other:?}"),
            }
        }
        // A missing lang is a bad param, not a silent default.
        assert!(matches!(
            language_param(&task("")),
            Err(EngineError::BadParam { .. })
        ));
    }

    #[test]
    fn s146_a_missing_language_is_named_per_component() {
        assert_eq!(missing_languages("eng", &["eng"]), None);
        assert_eq!(missing_languages("ara+eng", &["ara", "eng"]), None);
        assert_eq!(missing_languages("ara", &["eng"]), Some(vec!["ara"]));
        assert_eq!(missing_languages("ara+eng", &["eng"]), Some(vec!["ara"]));
        assert_eq!(missing_languages("ara+eng", &[]), Some(vec!["ara", "eng"]));
        // A choice with no components is not a missing language.
        assert_eq!(missing_languages("deu", &[]), None);
    }

    #[test]
    fn s146_the_probe_parses_identifiers_and_skips_the_header() {
        let report = "List of available languages in /usr/share/tessdata/ (3):\n\
                       ara\n\
                       eng\n\
                       osd\n";
        assert_eq!(parse_listed_languages(report), ["ara", "eng"]);
        assert!(parse_listed_languages("List of available languages (1):\n").is_empty());
        assert!(parse_listed_languages("").is_empty());
        // A version line, an argument and prose are not a language.
        assert!(parse_listed_languages("tesseract 5.3.4\n-l eng\nArabic and English\n").is_empty());
        // The same language twice is one entry.
        assert_eq!(parse_listed_languages("eng\neng\n"), ["eng"]);
    }

    #[test]
    fn s146_the_recognized_text_keeps_its_characters() {
        // Arabic, a digit and a diacritic are kept as they are.
        let text = "نص عربي 123";
        let bytes = text.as_bytes();
        assert_eq!(recognized_text(bytes).unwrap(), text);
        assert_eq!(recognized_text("a\r\nb".as_bytes()).unwrap(), "a\r\nb");
        assert_eq!(
            recognized_text(" a ".as_bytes()).unwrap(),
            " a ",
            "the recognized bytes are not trimmed"
        );
    }

    #[test]
    fn s146_a_blank_or_oversized_result_is_refused() {
        for blank in ["", " ", "\n", "\t\r\n  "] {
            match recognized_text(blank.as_bytes()) {
                Err(EngineError::Failed(message)) => assert_eq!(message, NO_TEXT, "{blank:?}"),
                other => panic!("{blank:?}: expected the no text diagnostic, got {other:?}"),
            }
        }
        // An invalid byte is an encoding failure, never a replacement.
        match recognized_text(&[0x41, 0xFF, 0x0A]) {
            Err(EngineError::Failed(message)) => assert!(message.contains("UTF-8"), "{message}"),
            other => panic!("expected an encoding error, got {other:?}"),
        }
        let too_big = vec![b'a'; MAX_OUTPUT_BYTES as usize + 1];
        assert!(recognized_text(&too_big).is_err());
    }

    #[test]
    fn s146_a_cancelled_task_stops_before_the_first_child() {
        // No program, no input: the cancel comes first, so nothing is read
        // and no child runs.
        let engine = OcrEngine {
            program: RwLock::new(None),
            pinned: false,
        };
        let cancel = CancelToken::new();
        cancel.cancel();
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("a.png");
        std::fs::write(&input, b"x").unwrap();
        let task = Task {
            action_id: "test.to-text".into(),
            op: "to-text".into(),
            params: toml::from_str("lang = \"eng\"").unwrap(),
            inputs: vec![input],
            output: dir.path().join("out.txt"),
        };
        match engine.run(&task, &|_| {}, &cancel) {
            Err(EngineError::Cancelled) => {}
            other => panic!("expected Cancelled, got {other:?}"),
        }
    }

    #[test]
    fn s146_an_engine_without_a_program_names_tesseract_and_its_page() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("a.png");
        std::fs::write(&input, b"x").unwrap();
        let task = Task {
            action_id: "test.to-text".into(),
            op: "to-text".into(),
            params: toml::from_str("lang = \"eng\"").unwrap(),
            inputs: vec![input],
            output: dir.path().join("out.txt"),
        };
        let engine = OcrEngine {
            program: RwLock::new(None),
            pinned: false,
        };
        match engine.run(&task, &|_| {}, &CancelToken::new()) {
            Err(EngineError::Unavailable(message)) => {
                assert!(message.contains("Tesseract"), "{message}");
                assert!(message.contains(TESSERACT_URL), "{message}");
            }
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[test]
    fn s146_the_action_is_registered_with_its_language_choice() {
        // The built-in action file and the engine id agree.
        let registry = kawaro_core::action::Registry::builtin();
        let action = registry.get("image.to-text").expect("the OCR action");
        assert_eq!(action.engine, "ocr");
        assert_eq!(action.op, "to-text");
        assert!(action.accepts.contains(&MediaKind::Image));
        let options = action.options.as_ref().expect("the language option");
        let field = options
            .fields
            .iter()
            .find(|f| f.id == "lang")
            .expect("the lang field");
        let ids: Vec<&str> = field.values.iter().map(|v| v.id.as_str()).collect();
        assert_eq!(ids, SUPPORTED_LANGUAGES.to_vec());
        assert_eq!(
            action.params.get("lang").and_then(|v| v.as_str()),
            Some("ara+eng")
        );
    }

    #[test]
    fn s146_the_timeouts_are_the_policy_values() {
        assert_eq!(PROBE_TIMEOUT, Duration::from_secs(5));
        assert_eq!(EXTRACTION_TIMEOUT, Duration::from_secs(240));
        assert_eq!(MAX_OUTPUT_BYTES, 8 * 1024 * 1024);
    }
}
