//! What an update leaves on disk and when it goes (S-106 D2): the download
//! folders `<cache>/updates/<version>` of versions that are installed or
//! older, and the helper copy folders in the temp folder. Names are decided
//! by pure functions; the two `clean_*` functions only delete what they name.
//! Nothing here touches a system owned folder.

use super::version::parse;
use std::cmp::Ordering;
use std::path::Path;

/// The prefix of the helper copy folder (`apply/windows.rs`).
pub const HELPER_PREFIX: &str = "kawaro-update-helper-";

/// The folder names that are plain versions less than or equal to
/// `installed`, in input order. Names that are not plain versions
/// (`notes.txt`, `0.4.4-x`) and newer versions are kept.
pub fn stale_update_dirs(entries: &[&str], installed: &str) -> Vec<String> {
    let Ok(installed) = parse(installed) else {
        return Vec::new();
    };
    entries
        .iter()
        .filter(|name| {
            parse(name).is_ok_and(|v| {
                v.pre.is_empty() && v.cmp_precedence(&installed) != Ordering::Greater
            })
        })
        .map(|name| name.to_string())
        .collect()
}

/// The helper folders, as (name, modified time in seconds), that are strictly
/// older than `max_age` seconds at `now`.
pub fn stale_helper_dirs(entries: &[(&str, u64)], now: u64, max_age: u64) -> Vec<String> {
    entries
        .iter()
        .filter(|(name, modified)| {
            name.starts_with(HELPER_PREFIX) && now.saturating_sub(*modified) > max_age
        })
        .map(|(name, _)| name.to_string())
        .collect()
}

/// Removes the stale version folders under `updates`. Returns how many went;
/// a folder that cannot be removed stays and is tried again at the next start.
pub fn clean_updates(updates: &Path, installed: &str) -> usize {
    let names = dir_names(updates);
    let refs: Vec<&str> = names.iter().map(|(n, _)| n.as_str()).collect();
    stale_update_dirs(&refs, installed)
        .iter()
        .filter(|n| std::fs::remove_dir_all(updates.join(n)).is_ok())
        .count()
}

/// The sub folders of `dir` as (name, modified time in seconds).
pub fn dir_names(dir: &Path) -> Vec<(String, u64)> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    read.flatten()
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            if !meta.is_dir() {
                return None;
            }
            let secs = meta
                .modified()
                .ok()?
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?
                .as_secs();
            Some((e.file_name().to_str()?.to_string(), secs))
        })
        .collect()
}
