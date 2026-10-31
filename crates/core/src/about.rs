//! The About page actions (S-094b, backlog/S-094b-about-page-actions.md):
//! the two links, the text the Copy version button puts on the clipboard,
//! and the bilingual strings. Pure: opening a link and writing the clipboard
//! are injected by the daemon, so nothing here touches the system.

use crate::i18n::Lang;

/// The About links as `(id, url)`. The window carries only the id.
pub const LINKS: [(&str, &str); 2] = [
    ("website", "https://kawaro.sbarah.com/"),
    ("source", "https://github.com/M7MMAD-OMAR/kawaro"),
];

/// The URL of a link id; `None` for any other string, exactly.
pub fn link_url(id: &str) -> Option<&'static str> {
    LINKS.iter().find(|(i, _)| *i == id).map(|(_, url)| *url)
}

/// What Copy version writes: the version and nothing else.
pub fn copy_text(version: &str) -> String {
    version.to_string()
}

/// The strings of the About actions. The two link labels are the accessible
/// labels and name the full target; the visible text is the short name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AboutStrings {
    pub website: &'static str,
    pub source: &'static str,
    pub copy_version: &'static str,
    pub copied: &'static str,
    pub check_updates: &'static str,
    pub website_label: String,
    pub source_label: String,
}

fn url_of(id: &str) -> &'static str {
    link_url(id).unwrap_or_default()
}

pub fn strings(lang: Lang) -> AboutStrings {
    let (site, code) = (url_of("website"), url_of("source"));
    match lang {
        Lang::En => AboutStrings {
            website: "Website",
            source: "Source code",
            copy_version: "Copy version",
            copied: "Copied",
            check_updates: "Check for updates",
            website_label: format!("Website: {site}"),
            source_label: format!("Source code: {code}"),
        },
        Lang::Ar => AboutStrings {
            website: "الموقع",
            source: "الشيفرة المصدرية",
            copy_version: "انسخ الإصدار",
            copied: "تم النسخ",
            check_updates: "ابحث عن تحديث",
            website_label: format!("الموقع: {site}"),
            source_label: format!("الشيفرة المصدرية: {code}"),
        },
    }
}
