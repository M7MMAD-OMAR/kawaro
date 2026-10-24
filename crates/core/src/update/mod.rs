//! The pure update rules of ADR 0006 (S-053a): versions, the signed
//! manifest, the policy that decides what is offered, the check schedule.
//! No network and no files; callers pass bytes and text in.

pub mod fetch;
pub mod flow;
pub mod manifest;
pub mod policy;
pub mod schedule;
pub mod verify;
pub mod version;

use serde::{Deserialize, Serialize};

/// Why an update input was refused.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum UpdateError {
    #[error("not a semantic version")]
    BadVersion,
    #[error("unsupported manifest schema")]
    UnsupportedSchema,
    #[error("the manifest is not valid")]
    InvalidJson,
    #[error("sha256 is not 64 lowercase hex digits")]
    BadSha256,
    #[error("size is zero")]
    BadSize,
    #[error("package kind is not msi, rpm or deb")]
    BadKind,
    #[error("the manifest is for another channel")]
    ChannelMismatch,
    #[error("an asset url is outside the release downloads")]
    BadUrl,
    #[error("no trusted key verifies the signature")]
    BadSignature,
    #[error("not a minisign public key")]
    BadKey,
    #[error("the network is not reachable")]
    Offline,
    #[error("no curl program was found")]
    Unavailable,
    #[error("the server answered HTTP {0}")]
    Http(u16),
    #[error("the package does not match its signed sha256")]
    HashMismatch,
    #[error("the package does not match its signed size")]
    SizeMismatch,
    #[error("the manifest has no package for this platform")]
    NoAsset,
    #[error("a file could not be read or written")]
    Io,
}

/// The release line a user follows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    #[default]
    Stable,
    Beta,
}

/// The channel to follow when none is configured: `Beta` for a version with a
/// prerelease part, otherwise `Stable` (ADR 0006, manager decision).
pub fn default_channel(installed: &str) -> Result<Channel, UpdateError> {
    let v = version::parse(installed)?;
    Ok(if v.pre.is_empty() {
        Channel::Stable
    } else {
        Channel::Beta
    })
}
