//! The signed feed manifest, schema 1 (ADR 0006 section 2).

use super::{Channel, UpdateError};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Asset {
    pub kind: String,
    pub os: String,
    pub arch: String,
    pub name: String,
    pub url: String,
    pub size: u64,
    pub sha256: String,
}

/// The short "what is new" lines of a release, in both languages (S-099).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Notes {
    pub en: Vec<String>,
    pub ar: Vec<String>,
}

const NOTE_LINES: std::ops::RangeInclusive<usize> = 3..=5;
const NOTE_MAX_CHARS: usize = 80;

impl Notes {
    /// The notes in `value` when each language has 3 to 5 lines and every
    /// line is non empty, one line and at most 80 characters.
    pub fn from_value(value: &serde_json::Value) -> Result<Notes, String> {
        let lang = |key: &str| -> Result<Vec<String>, String> {
            let lines: Vec<String> = value
                .get(key)
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .ok_or_else(|| format!("\"{key}\" must be a list of strings"))?;
            if !NOTE_LINES.contains(&lines.len()) {
                return Err(format!("\"{key}\" needs 3 to 5 lines"));
            }
            let bad = |l: &String| {
                l.is_empty() || l.contains(['\n', '\r']) || l.chars().count() > NOTE_MAX_CHARS
            };
            if lines.iter().any(bad) {
                return Err(format!("\"{key}\" has an empty, long or multi line entry"));
            }
            Ok(lines)
        };
        Ok(Notes {
            en: lang("en")?,
            ar: lang("ar")?,
        })
    }
}

/// Absent or invalid notes become empty and never reject the manifest.
fn lenient_notes<'de, D: Deserializer<'de>>(d: D) -> Result<Notes, D::Error> {
    let value = serde_json::Value::deserialize(d)?;
    Ok(Notes::from_value(&value).unwrap_or_default())
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub channel: Channel,
    pub version: String,
    pub published: String,
    pub notes_url: String,
    #[serde(default, deserialize_with = "lenient_notes")]
    pub notes: Notes,
    pub assets: Vec<Asset>,
}

impl Manifest {
    /// Reads and checks a manifest. The schema is looked at first, so a
    /// future schema is `UnsupportedSchema` whatever else it holds.
    pub fn parse(bytes: &[u8]) -> Result<Manifest, UpdateError> {
        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| UpdateError::InvalidJson)?;
        match value.get("schema").and_then(|s| s.as_u64()) {
            Some(1) => {}
            Some(_) => return Err(UpdateError::UnsupportedSchema),
            None => return Err(UpdateError::InvalidJson),
        }
        let m: Manifest = serde_json::from_value(value).map_err(|_| UpdateError::InvalidJson)?;
        for a in &m.assets {
            if !matches!(a.kind.as_str(), "msi" | "rpm" | "deb") {
                return Err(UpdateError::BadKind);
            }
            if a.size == 0 {
                return Err(UpdateError::BadSize);
            }
            let hex = a.sha256.len() == 64
                && a.sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
            if !hex {
                return Err(UpdateError::BadSha256);
            }
        }
        Ok(m)
    }
}
