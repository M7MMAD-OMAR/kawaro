//! S-096: why a selection has no actions, and what to do next. Pure: no I/O,
//! no process, no network. The daemon decides what to show from [`Outcome`].

use crate::action::Registry;
use crate::i18n::Lang;
use crate::media::{self, MediaKind};
use std::path::{Path, PathBuf};

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
];

pub fn install_hints() -> &'static [InstallHint] {
    HINTS
}

pub fn tool_name(engine_id: &str) -> Option<&'static str> {
    match engine_id {
        "ffmpeg" => Some("FFmpeg"),
        "office" => Some("LibreOffice"),
        "ghostscript" => Some("Ghostscript"),
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
    let h = HINTS.iter().find(|h| h.tool == tool);
    let (url, package) = h.map_or(("", ""), |h| (h.url, h.package));
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
