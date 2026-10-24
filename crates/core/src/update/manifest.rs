//! The signed feed manifest, schema 1 (ADR 0006 section 2).

use super::{Channel, UpdateError};
use serde::Deserialize;

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

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub channel: Channel,
    pub version: String,
    pub published: String,
    pub notes_url: String,
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
