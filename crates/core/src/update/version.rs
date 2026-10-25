//! Semantic version comparison.

use super::UpdateError;
use semver::Version;
use std::cmp::Ordering;

/// `text` as a semantic version.
pub fn parse(text: &str) -> Result<Version, UpdateError> {
    Version::parse(text).map_err(|_| UpdateError::BadVersion)
}

/// True when `candidate` has higher precedence than `installed`. Build
/// metadata does not count.
pub fn is_newer(installed: &str, candidate: &str) -> Result<bool, UpdateError> {
    let (a, b) = (parse(installed)?, parse(candidate)?);
    Ok(b.cmp_precedence(&a) == Ordering::Greater)
}
