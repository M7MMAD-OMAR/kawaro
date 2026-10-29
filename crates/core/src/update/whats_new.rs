//! The "what is new" lines of the installed version (S-099a A5): bundled at
//! build time from `docs/whats-new/<version>.json`, so showing them needs no
//! network.

use std::path::Path;

use super::manifest::Notes;
use crate::i18n::Lang;

const BUNDLED: &str = include!(concat!(env!("OUT_DIR"), "/whats_new.rs"));

/// The bundled lines for `lang`; empty when the file is missing or invalid.
pub fn installed(lang: Lang) -> Vec<String> {
    let notes = serde_json::from_str(BUNDLED)
        .ok()
        .and_then(|v| Notes::from_value(&v).ok())
        .unwrap_or_default();
    match lang {
        Lang::En => notes.en,
        Lang::Ar => notes.ar,
    }
}

/// Reads `<dir>/<version>.json` and applies the manifest rule; the release
/// gate for the bundled file.
pub fn validate_dir(dir: &Path, version: &str) -> Result<Notes, String> {
    let path = dir.join(format!("{version}.json"));
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    Notes::from_value(&value).map_err(|e| format!("{}: {e}", path.display()))
}
