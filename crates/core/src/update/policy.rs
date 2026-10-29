//! What a verified manifest means for the installed version.

use super::manifest::{Asset, Manifest, Notes};
use super::{Channel, UpdateError, version};

/// Every asset url must start with this.
const URL_PREFIX: &str = "https://github.com/M7MMAD-OMAR/kawaro/releases/download/";

#[derive(Clone, Debug, PartialEq)]
pub struct Update {
    pub version: String,
    pub notes_url: String,
    pub notes: Notes,
}

#[derive(Debug, PartialEq)]
pub enum Decision {
    UpToDate,
    Available(Update),
}

/// ADR 0006 section 3, in order: channel, version, urls.
pub fn evaluate(m: &Manifest, installed: &str, channel: Channel) -> Result<Decision, UpdateError> {
    if m.channel != channel {
        return Err(UpdateError::ChannelMismatch);
    }
    if !version::is_newer(installed, &m.version)? {
        return Ok(Decision::UpToDate);
    }
    if m.assets.iter().any(|a| !a.url.starts_with(URL_PREFIX)) {
        return Err(UpdateError::BadUrl);
    }
    Ok(Decision::Available(Update {
        version: m.version.clone(),
        notes_url: m.notes_url.clone(),
        notes: m.notes.clone(),
    }))
}

/// The platform to pick a package for: `os` and `arch` as the manifest
/// spells them, `pkg` one of `msi`, `rpm`, `deb`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Platform {
    pub os: String,
    pub arch: String,
    pub pkg: String,
}

pub fn select_asset(m: &Manifest, p: Platform) -> Option<&Asset> {
    m.assets
        .iter()
        .find(|a| a.os == p.os && a.arch == p.arch && a.kind == p.pkg)
}
