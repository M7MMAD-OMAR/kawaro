//! Minisign signature check of the manifest (ADR 0006 section 1), with
//! `minisign-verify` (MIT, no dependencies; docs/engines.md).

use super::UpdateError;
use minisign_verify::{PublicKey, Signature};

/// The trusted public keys: a signature from any of them is accepted.
pub const UPDATE_KEYS: &[&str] = &[include_str!(
    "../../../../packaging/keys/sbarhive-update.pub"
)];

/// A key as the text of a `.pub` file or as its bare base64 line.
fn load_key(text: &str) -> Result<PublicKey, UpdateError> {
    let text = text.trim();
    let key = if text.contains('\n') {
        PublicKey::decode(text)
    } else {
        PublicKey::from_base64(text)
    };
    key.map_err(|_| UpdateError::BadKey)
}

/// Succeeds when `text` is a minisign public key.
pub fn parse_key(text: &str) -> Result<(), UpdateError> {
    load_key(text).map(|_| ())
}

/// Ok when `minisig` is a signature of `bytes` by one of `keys`. Keys that do
/// not parse never verify anything.
pub fn verify_manifest(bytes: &[u8], minisig: &str, keys: &[&str]) -> Result<(), UpdateError> {
    let sig = Signature::decode(minisig).map_err(|_| UpdateError::BadSignature)?;
    let trusted = keys.iter().filter_map(|k| load_key(k).ok());
    if trusted
        .into_iter()
        .any(|k| k.verify(bytes, &sig, false).is_ok())
    {
        Ok(())
    } else {
        Err(UpdateError::BadSignature)
    }
}
