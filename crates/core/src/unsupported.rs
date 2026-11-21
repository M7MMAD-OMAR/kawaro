//! S-096: why a selection has no actions, and what to do next. Pure: no I/O,
//! no process, no network. The daemon decides what to show from [`Outcome`].

use crate::action::{ActionDef, Registry};
use crate::i18n::Lang;
use crate::media::{self, MediaKind};
use crate::sync::lock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Supported,
    Unsupported {
        ext: String,
    },
    MissingEngine {
        tool: &'static str,
    },
    Mixed {
        supported: Vec<PathBuf>,
        unsupported: Vec<PathBuf>,
    },
    NoSelection,
}

/// One external tool with its official link and package name.
#[derive(Debug, Clone, Copy)]
pub struct InstallHint {
    pub tool: &'static str,
    pub url: &'static str,
    pub package: &'static str,
}

/// The curated help of the tools Kawaro does not bundle: the general engines
/// (S-096) first, then the PDF tools and Tesseract (S-145, S-146). The page
/// and the package name come from the tree: `crates/engines/src/poppler.rs`,
/// `crates/engines/src/qpdf.rs`, `crates/engines/src/ocr.rs` and
/// `docs/engines.md`. Nothing here is downloaded, installed or run.
const HINTS: &[InstallHint] = &[
    InstallHint {
        tool: "FFmpeg",
        url: "https://ffmpeg.org/download.html",
        package: "ffmpeg",
    },
    InstallHint {
        tool: "LibreOffice",
        url: "https://www.libreoffice.org/download/download/",
        package: "libreoffice",
    },
    InstallHint {
        tool: "Ghostscript",
        url: "https://ghostscript.com/releases/gsdnld.html",
        package: "ghostscript",
    },
    InstallHint {
        tool: "Poppler",
        url: "https://poppler.freedesktop.org/",
        package: "poppler-utils",
    },
    InstallHint {
        tool: "qpdf",
        url: "https://qpdf.readthedocs.io/",
        package: "qpdf",
    },
    InstallHint {
        tool: "Tesseract",
        url: "https://tesseract-ocr.github.io/tessdoc/",
        package: "tesseract-ocr",
    },
];

/// Every curated external-tool hint, in the displayed order.
pub fn install_hints() -> &'static [InstallHint] {
    HINTS
}

/// The hint of one tool, the general engines and the PDF tools alike.
pub fn install_hint(tool: &str) -> Option<&'static InstallHint> {
    HINTS.iter().find(|h| h.tool == tool)
}

pub fn tool_name(engine_id: &str) -> Option<&'static str> {
    match engine_id {
        "ffmpeg" => Some("FFmpeg"),
        "office" => Some("LibreOffice"),
        "ghostscript" => Some("Ghostscript"),
        "poppler" => Some("Poppler"),
        "qpdf" => Some("qpdf"),
        "ocr" => Some("Tesseract"),
        _ => None,
    }
}

#[derive(PartialEq)]
enum Verdict {
    Runnable,
    Unsupported,
    Missing(&'static str),
}

fn verdict(file: &Path, reg: &Registry, ok: &dyn Fn(&str) -> bool) -> Verdict {
    let ext = media::extension(file);
    let kind = media::detect(file);
    let mut missing = None;
    for a in reg.all() {
        if !a.accepts.contains(&kind) || a.skip_ext.iter().any(|s| s.eq_ignore_ascii_case(&ext)) {
            continue;
        }
        if ok(&a.engine) {
            return Verdict::Runnable;
        }
        missing = missing.or_else(|| tool_name(&a.engine));
    }
    match missing {
        Some(tool) => Verdict::Missing(tool),
        None => Verdict::Unsupported,
    }
}

pub fn classify(files: &[PathBuf], reg: &Registry, ok: &dyn Fn(&str) -> bool) -> Outcome {
    if files.is_empty() {
        return Outcome::NoSelection;
    }
    let (mut supported, mut unsupported) = (Vec::new(), Vec::new());
    let (mut first_ext, mut first_tool) = (None, None);
    for f in files {
        match verdict(f, reg, ok) {
            Verdict::Runnable => supported.push(f.clone()),
            Verdict::Unsupported => {
                first_ext.get_or_insert_with(|| media::extension(f));
                unsupported.push(f.clone());
            }
            Verdict::Missing(tool) => {
                first_tool.get_or_insert(tool);
                unsupported.push(f.clone());
            }
        }
    }
    if unsupported.is_empty() {
        Outcome::Supported
    } else if !supported.is_empty() {
        Outcome::Mixed {
            supported,
            unsupported,
        }
    } else if let Some(ext) = first_ext {
        Outcome::Unsupported { ext }
    } else {
        Outcome::MissingEngine {
            tool: first_tool.unwrap_or("FFmpeg"),
        }
    }
}

pub fn missing_engine_message(tool: &str, lang: Lang) -> String {
    let (url, package) = install_hint(tool).map_or(("", ""), |h| (h.url, h.package));
    match lang {
        Lang::En => format!("{tool} is not installed. Install the package {package}: {url}"),
        Lang::Ar => format!("الأداة غير مثبتة: {tool}. ثبّت الحزمة {package} من {url}"),
    }
}

/// The request link: a fixed template plus the extension only.
pub fn request_url(ext: &str) -> String {
    let ext = ext.to_ascii_lowercase();
    let ok = (1..=16).contains(&ext.len()) && ext.bytes().all(|b| b.is_ascii_alphanumeric());
    let ext = if ok { ext.as_str() } else { "unknown" };
    format!(
        "https://github.com/M7MMAD-OMAR/kawaro/issues/new?title=Format+request%3A+.{ext}\
         &body=Please+add+support+for+.{ext}+files."
    )
}

pub fn notification(outcome: &Outcome, lang: Lang) -> Option<String> {
    match (outcome, lang) {
        (Outcome::NoSelection, Lang::En) => {
            Some("No file selected. Select a file, then press the shortcut again.".into())
        }
        (Outcome::NoSelection, Lang::Ar) => {
            Some("لم يُحدَّد أي ملف. حدّد ملفاً ثم اضغط الاختصار مرة أخرى.".into())
        }
        (Outcome::Unsupported { ext }, Lang::En) => {
            Some(format!(".{ext} files are not supported yet"))
        }
        (Outcome::Unsupported { ext }, Lang::Ar) => {
            Some(format!("ملفات \u{200e}.{ext} غير مدعومة بعد"))
        }
        (Outcome::MissingEngine { tool }, Lang::En) => Some(format!("{tool} is not installed")),
        (Outcome::MissingEngine { tool }, Lang::Ar) => Some(format!("{tool} غير مثبت")),
        _ => None,
    }
}

/// Centre text: the extensions (at most 3, then `+K`) and the file count.
pub fn centre_text(files: &[PathBuf], lang: Lang) -> String {
    let mut exts: Vec<String> = Vec::new();
    for f in files {
        let e = media::extension(f);
        if !exts.contains(&e) {
            exts.push(e);
        }
    }
    let shown = |e: &String| format!(".{e}");
    if exts.len() > 1 {
        let mut parts: Vec<String> = exts.iter().take(3).map(shown).collect();
        if exts.len() > 3 {
            parts.push(format!("+{}", exts.len() - 3));
        }
        return parts.join(" ");
    }
    let ext = exts.first().map(shown).unwrap_or_default();
    match (lang, files.len()) {
        (Lang::En, 1) => format!("File {ext}"),
        (Lang::En, n) => format!("{n} files {ext}"),
        (Lang::Ar, 1) => format!("ملف \u{200e}{ext}"),
        (Lang::Ar, n) => format!("{n} ملفات \u{200e}{ext}"),
    }
}

pub fn unsupported_message(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "This format is not supported yet",
        Lang::Ar => "هذه الصيغة غير مدعومة بعد",
    }
}

pub fn request_label(lang: Lang) -> &'static str {
    match lang {
        Lang::En => "Request this format",
        Lang::Ar => "اطلب هذه الصيغة",
    }
}

pub fn request_hover(lang: Lang) -> &'static str {
    match lang {
        Lang::En => {
            "Opens a request page in your browser. Nothing is sent automatically. Kawaro has no telemetry."
        }
        Lang::Ar => {
            "تفتح صفحة الطلب في متصفحك. لا يُرسل شيء تلقائياً. لا يجمع كوارو أي بيانات استخدام."
        }
    }
}

/// The accessible text of a greyed cell: the extension (no dot), that it is
/// unavailable, and why.
pub fn skipped_label(ext: &str, reason: &str, lang: Lang) -> String {
    match lang {
        Lang::En => format!(".{ext}: unavailable. {reason}"),
        Lang::Ar => format!("\u{200e}.{ext}: غير متاح. {reason}"),
    }
}

/// The accessible text of the `+K` cell: how many more files are skipped.
pub fn more_label(k: usize, lang: Lang) -> String {
    match lang {
        Lang::En => format!("{k} more files are not supported"),
        Lang::Ar => format!("{k} ملفات أخرى غير مدعومة"),
    }
}

/// The label of an available action in the file's family, if the file's
/// family is known and some action of it is runnable.
pub fn suggestion(
    files: &[PathBuf],
    reg: &Registry,
    ok: &dyn Fn(&str) -> bool,
    lang: Lang,
) -> Option<String> {
    let kind = media::detect(files.first()?);
    if kind == MediaKind::Other {
        return None;
    }
    reg.all()
        .iter()
        .filter(|a| a.accepts.contains(&kind) && ok(&a.engine))
        .min_by(|x, y| (x.ring, x.order, &x.id).cmp(&(y.ring, y.order, &y.id)))
        .map(|a| a.label.get(lang).to_string())
}

// ------------------------------------------------------------ S-145 capability reports

/// Why a registered action is not currently runnable (S-145).
///
/// Every variant is distinct so CLI, MCP and UI never confuse a policy
/// restriction or an incompatible selection with missing software, and never
/// treat an unknown or timed-out probe as available.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum UnavailableReason {
    /// The external program required by this action's engine is not installed.
    MissingExecutable { program: String },
    /// The program is installed, but the requested language data is absent.
    MissingLanguageData { language: String },
    /// The selected files do not fit the action's accepted media kinds,
    /// `skip_ext` list, or file count bounds.
    UnsupportedInput,
    /// The explicit or default options for this action did not validate.
    InvalidOptions,
    /// Disabled by local configuration or policy (for example MCP off).
    DisabledByPolicy,
    /// Probing the tool timed out, failed, or returned an unusable report.
    ProbeUnknown,
}

/// Availability and explanation for one registered action (S-145), shared by
/// CLI, MCP and UI.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityReport {
    pub action_id: String,
    pub runnable: bool,
    pub reason: Option<UnavailableReason>,
    pub help_id: Option<String>,
}

/// Outcome of one read-only tool probe (S-145).
///
/// A timed-out, failing, or malformed probe is [`ProbeOutcome::Unknown`],
/// never [`ProbeOutcome::Ready`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProbeOutcome {
    /// The tool answered its read-only probe. `items` holds optional
    /// capabilities reported by the probe (for example installed OCR language
    /// identifiers).
    Ready { items: Vec<String> },
    /// The tool executable is not present at the probed path.
    Missing,
    /// The probe timed out, exited abnormally, or produced an unusable
    /// report. Never treated as runnable.
    Unknown,
}

impl ProbeOutcome {
    /// Converts this probe outcome into an [`UnavailableReason`] for `program`,
    /// or `None` when the tool is ready.
    pub fn into_reason(&self, program: &str) -> Option<UnavailableReason> {
        match self {
            ProbeOutcome::Ready { .. } => None,
            ProbeOutcome::Missing => Some(UnavailableReason::MissingExecutable {
                program: program.to_string(),
            }),
            ProbeOutcome::Unknown => Some(UnavailableReason::ProbeUnknown),
        }
    }

    /// Checks that every identifier in `required` is present in a ready probe.
    ///
    /// Returns `None` when ready and every required language is present,
    /// [`UnavailableReason::MissingLanguageData`] when the executable answered
    /// but one or more requested languages are absent,
    /// [`UnavailableReason::MissingExecutable`] when missing, and
    /// [`UnavailableReason::ProbeUnknown`] when the probe failed or timed out.
    pub fn check_language(&self, program: &str, required: &[&str]) -> Option<UnavailableReason> {
        match self {
            ProbeOutcome::Missing => Some(UnavailableReason::MissingExecutable {
                program: program.to_string(),
            }),
            ProbeOutcome::Unknown => Some(UnavailableReason::ProbeUnknown),
            ProbeOutcome::Ready { items } => {
                let missing: Vec<&str> = required
                    .iter()
                    .copied()
                    .filter(|lang| !items.iter().any(|item| item.as_str() == *lang))
                    .collect();
                if missing.is_empty() {
                    None
                } else {
                    Some(UnavailableReason::MissingLanguageData {
                        language: missing.join("+"),
                    })
                }
            }
        }
    }
}

/// Thread-safe cache of read-only tool probes, keyed by tool path (S-145).
///
/// Probes never install software or run `sudo`. Entries remain cached until
/// [`Self::invalidate`], [`Self::invalidate_all`], or [`Self::refresh`] is
/// called explicitly.
#[derive(Debug, Default)]
pub struct ProbeCache {
    entries: Mutex<HashMap<PathBuf, ProbeOutcome>>,
}

impl ProbeCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the cached outcome for `tool_path`, if any.
    pub fn get(&self, tool_path: &Path) -> Option<ProbeOutcome> {
        lock(&self.entries).get(tool_path).cloned()
    }

    /// Stores `outcome` for `tool_path`.
    pub fn insert(&self, tool_path: PathBuf, outcome: ProbeOutcome) {
        lock(&self.entries).insert(tool_path, outcome);
    }

    /// Returns the cached outcome for `tool_path`, or runs `probe` once
    /// without holding the cache lock and stores the result.
    pub fn get_or_probe(
        &self,
        tool_path: &Path,
        probe: impl FnOnce(&Path) -> ProbeOutcome,
    ) -> ProbeOutcome {
        if let Some(cached) = self.get(tool_path) {
            return cached;
        }
        let outcome = probe(tool_path);
        self.insert(tool_path.to_path_buf(), outcome.clone());
        outcome
    }

    /// Removes the cached entry for `tool_path`, forcing the next lookup to
    /// re-probe.
    pub fn invalidate(&self, tool_path: &Path) {
        lock(&self.entries).remove(tool_path);
    }

    /// Clears every cached probe entry, for example after PATH changes or a
    /// user-requested refresh.
    pub fn invalidate_all(&self) {
        lock(&self.entries).clear();
    }

    /// Invalidates `tool_path`, runs `probe` fresh without holding the lock,
    /// caches the new outcome and returns it.
    pub fn refresh(
        &self,
        tool_path: &Path,
        probe: impl FnOnce(&Path) -> ProbeOutcome,
    ) -> ProbeOutcome {
        self.invalidate(tool_path);
        let outcome = probe(tool_path);
        self.insert(tool_path.to_path_buf(), outcome.clone());
        outcome
    }
}

/// Primary executable name for an external engine id, or `None` for built-in
/// pure Rust engines.
pub fn program_name(engine_id: &str) -> Option<&'static str> {
    match engine_id {
        "ffmpeg" => Some("ffmpeg"),
        "office" => Some("soffice"),
        "ghostscript" => Some("gs"),
        "poppler" => Some("pdftotext"),
        "qpdf" => Some("qpdf"),
        "ocr" => Some("tesseract"),
        _ => None,
    }
}

/// Curated help id for an unavailable action, or `None` when no install/setup
/// help applies (for example policy restrictions, invalid options, or
/// incompatible input).
pub fn help_id_for(engine_id: &str, reason: &UnavailableReason) -> Option<String> {
    match reason {
        UnavailableReason::MissingExecutable { program } => {
            if tool_name(engine_id).is_some() {
                return Some(engine_id.to_string());
            }
            match program.to_ascii_lowercase().as_str() {
                "ffmpeg" | "ffprobe" => Some("ffmpeg".to_string()),
                "soffice" | "libreoffice" => Some("office".to_string()),
                "gs" | "gswin64c" | "gswin32c" | "ghostscript" => Some("ghostscript".to_string()),
                "pdftotext" | "poppler" => Some("poppler".to_string()),
                "qpdf" => Some("qpdf".to_string()),
                "tesseract" => Some("ocr".to_string()),
                _ => None,
            }
        }
        UnavailableReason::MissingLanguageData { .. } => Some("ocr-lang".to_string()),
        UnavailableReason::ProbeUnknown => tool_name(engine_id).map(|_| engine_id.to_string()),
        UnavailableReason::UnsupportedInput
        | UnavailableReason::InvalidOptions
        | UnavailableReason::DisabledByPolicy => None,
    }
}

/// Resolves a curated `help_id` back to the single [`InstallHint`] table.
pub fn install_hint_for_help(help_id: &str) -> Option<&'static InstallHint> {
    if help_id == "ocr-lang" {
        return install_hint("Tesseract");
    }
    if let Some(tool) = tool_name(help_id) {
        return install_hint(tool);
    }
    install_hint(help_id)
}

/// Converts a plain `engine_id -> bool` availability answer into the typed
/// callback result expected by [`capability_reports`].
pub fn default_engine_reason(engine_id: &str, available: bool) -> Option<UnavailableReason> {
    if available {
        None
    } else {
        Some(UnavailableReason::MissingExecutable {
            program: program_name(engine_id).unwrap_or(engine_id).to_string(),
        })
    }
}

/// Localized explanation of one [`UnavailableReason`] (English or Arabic).
pub fn reason_message(reason: &UnavailableReason, lang: Lang) -> String {
    match (reason, lang) {
        (UnavailableReason::MissingExecutable { program }, Lang::En) => {
            format!("Required program `{program}` is not installed")
        }
        (UnavailableReason::MissingExecutable { program }, Lang::Ar) => {
            format!("البرنامج المطلوب غير مثبت: {program}")
        }
        (UnavailableReason::MissingLanguageData { language }, Lang::En) => {
            format!("OCR language data `{language}` is not installed")
        }
        (UnavailableReason::MissingLanguageData { language }, Lang::Ar) => {
            format!("بيانات لغة التعرف الضوئي غير مثبتة: {language}")
        }
        (UnavailableReason::UnsupportedInput, Lang::En) => {
            "Selected files are not supported by this action".to_string()
        }
        (UnavailableReason::UnsupportedInput, Lang::Ar) => {
            "الملفات المحددة غير مدعومة لهذا الإجراء".to_string()
        }
        (UnavailableReason::InvalidOptions, Lang::En) => "Action options are not valid".to_string(),
        (UnavailableReason::InvalidOptions, Lang::Ar) => "خيارات الإجراء غير صالحة".to_string(),
        (UnavailableReason::DisabledByPolicy, Lang::En) => {
            "Disabled by configuration policy".to_string()
        }
        (UnavailableReason::DisabledByPolicy, Lang::Ar) => "معطّل وفق سياسة الإعدادات".to_string(),
        (UnavailableReason::ProbeUnknown, Lang::En) => {
            "Tool probe failed or timed out; refresh after checking the installation".to_string()
        }
        (UnavailableReason::ProbeUnknown, Lang::Ar) => {
            "تعذّر فحص الأداة أو انتهت مهلتها؛ حدّث الحالة بعد التحقق من التثبيت".to_string()
        }
    }
}

/// Curated read-only setup guidance for a `help_id` and `reason`. Never runs
/// an installer, never invokes `sudo`, and never names a shell install
/// command.
pub fn help_message(help_id: &str, reason: &UnavailableReason, lang: Lang) -> Option<String> {
    let hint = install_hint_for_help(help_id)?;
    match reason {
        UnavailableReason::MissingExecutable { .. } => {
            Some(missing_engine_message(hint.tool, lang))
        }
        UnavailableReason::MissingLanguageData { language } => Some(match lang {
            Lang::En => format!(
                "Install the Tesseract traineddata package for {language} (for example tesseract-ocr-ara or tesseract-ocr-eng on Debian/Ubuntu, tesseract-langpack-ara or tesseract-langpack-eng on Fedora): {}",
                hint.url
            ),
            Lang::Ar => format!(
                "ثبّت حزمة بيانات اللغة في Tesseract للغة {language} (مثل tesseract-ocr-ara أو tesseract-ocr-eng): {}",
                hint.url
            ),
        }),
        UnavailableReason::ProbeUnknown => Some(match lang {
            Lang::En => format!(
                "Check that {} (package {}) runs locally, then refresh: {}",
                hint.tool, hint.package, hint.url
            ),
            Lang::Ar => format!(
                "تحقق من عمل {} (الحزمة {}) محلياً ثم حدّث الحالة: {}",
                hint.tool, hint.package, hint.url
            ),
        }),
        UnavailableReason::UnsupportedInput
        | UnavailableReason::InvalidOptions
        | UnavailableReason::DisabledByPolicy => None,
    }
}

/// Selection filter matching [`Registry::matching`]: media kind, `skip_ext`,
/// and file count bounds.
fn action_accepts_files(action: &ActionDef, files: &[&Path]) -> bool {
    let n = files.len();
    if n == 0 || n < action.min_files || (action.max_files != 0 && n > action.max_files) {
        return false;
    }
    if !files
        .iter()
        .all(|f| action.accepts.contains(&media::detect(f)))
    {
        return false;
    }
    let skipped = files.iter().all(|f| {
        let ext = media::extension(f);
        action.skip_ext.iter().any(|s| s.eq_ignore_ascii_case(&ext))
    });
    !skipped
}

/// Builds the capability report list for every action in `reg`, sorted in
/// `(ring, order, id)` order, so CLI, MCP and UI share one implementation
/// (S-145).
///
/// When `files` is non-empty, selection compatibility is checked first using
/// the same rules as [`Registry::matching`]: incompatible actions receive
/// [`UnavailableReason::UnsupportedInput`] without probing their engine.
/// Otherwise `engine_check` decides whether the action is runnable (`None`)
/// or unavailable (`Some(reason)`).
pub fn capability_reports(
    reg: &Registry,
    files: &[&Path],
    engine_check: &dyn Fn(&ActionDef) -> Option<UnavailableReason>,
) -> Vec<CapabilityReport> {
    let mut actions: Vec<&ActionDef> = reg.all().iter().collect();
    actions.sort_by(|x, y| (x.ring, x.order, &x.id).cmp(&(y.ring, y.order, &y.id)));
    actions
        .into_iter()
        .map(|action| {
            let reason = if !files.is_empty() && !action_accepts_files(action, files) {
                Some(UnavailableReason::UnsupportedInput)
            } else {
                engine_check(action)
            };
            match reason {
                None => CapabilityReport {
                    action_id: action.id.clone(),
                    runnable: true,
                    reason: None,
                    help_id: None,
                },
                Some(reason) => {
                    let help_id = help_id_for(&action.engine, &reason);
                    CapabilityReport {
                        action_id: action.id.clone(),
                        runnable: false,
                        reason: Some(reason),
                        help_id,
                    }
                }
            }
        })
        .collect()
}

/// Formats an Arabic counted phrase with standard number agreement for 1, 2,
/// 3..=10 and 11+, using Western digits.
pub fn arabic_plural(n: usize, one: &str, two: &str, few: &str, many: &str) -> String {
    match n {
        1 => one.to_string(),
        2 => two.to_string(),
        3..=10 => format!("{n} {few}"),
        _ => format!("{n} {many}"),
    }
}

/// Short centre-idle hint when the hive shows runnable cells and `count`
/// matching actions still need setup (S-145). Empty when `count == 0`.
pub fn unavailable_actions_hint(count: usize, lang: Lang) -> String {
    if count == 0 {
        return String::new();
    }
    match lang {
        Lang::En => match count {
            1 => "1 more action needs setup, see kawaro doctor".to_string(),
            n => format!("{n} more actions need setup, see kawaro doctor"),
        },
        Lang::Ar => {
            let head = arabic_plural(
                count,
                "إجراء آخر يحتاج إعداداً",
                "إجراءان آخران يحتاجان إعداداً",
                "إجراءات أخرى تحتاج إعداداً",
                "إجراءً آخر يحتاج إعداداً",
            );
            format!("{head}، راجع kawaro doctor")
        }
    }
}
