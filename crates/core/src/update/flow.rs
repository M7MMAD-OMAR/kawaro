//! The update check and the verified download (ADR 0006 sections 3 to 6).
//!
//! The network is a [`Fetcher`], so the whole flow runs on fakes in tests.

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::fetch::{FetchError, Fetcher, MANIFEST_MAX_BYTES, SIGNATURE_MAX_BYTES, url_is_clean};
use super::manifest::{Asset, Manifest};
use super::policy::{Decision, Platform, evaluate, select_asset};
use super::schedule::{State, allowed, due};
use super::verify::{UPDATE_KEYS, verify_manifest};
use super::{Channel, UpdateError};
use crate::config::Config;
use crate::paths::{CURRENT, Os, cache_dir_with};

/// Where the manifests are published.
const FEED: &str = "https://github.com/M7MMAD-OMAR/sbarhive/releases/download/updates";
/// Automatic checks run at most this often.
const INTERVAL_SECS: u64 = 86_400;
/// Package files are hashed in reads of this size.
const CHUNK: usize = 65_536;

/// A newer version and the package chosen for this platform.
#[derive(Clone, Debug, PartialEq)]
pub struct Update {
    pub version: String,
    pub notes_url: String,
    pub asset: Asset,
}

/// What `update-state.json` remembers: the schedule and the last version seen.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CheckState {
    pub schedule: State,
    pub last_seen: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum CheckOutcome {
    Disabled,
    NotDue,
    UpToDate,
    Available(Update),
    Offline,
    Unavailable,
    Rejected(UpdateError),
}

/// Everything `check` decides by itself, passed in so a test can set it.
pub struct CheckEnv<'a> {
    pub installed: &'a str,
    pub keys: &'a [&'a str],
    pub platform: Platform,
    pub interval_secs: u64,
    /// Where the two fetches land; empty again afterwards.
    pub scratch: &'a Path,
}

/// A package whose size and SHA-256 matched the signed manifest. Only
/// [`download_verified`] makes one.
#[derive(Debug)]
pub struct VerifiedPackage {
    path: PathBuf,
}

impl VerifiedPackage {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Why a check stopped before a verdict.
enum Stop {
    Offline,
    Unavailable,
    Rejected(UpdateError),
}

impl From<FetchError> for Stop {
    fn from(e: FetchError) -> Stop {
        match e {
            FetchError::Offline => Stop::Offline,
            FetchError::Unavailable => Stop::Unavailable,
            other => Stop::Rejected(fetch_to_update_error(other)),
        }
    }
}

fn fetch_to_update_error(e: FetchError) -> UpdateError {
    match e {
        FetchError::Offline => UpdateError::Offline,
        FetchError::Unavailable => UpdateError::Unavailable,
        FetchError::Http(n) => UpdateError::Http(n),
        FetchError::TooLarge => UpdateError::SizeMismatch,
        FetchError::BadUrl => UpdateError::BadUrl,
    }
}

fn channel_name(c: Channel) -> &'static str {
    match c {
        Channel::Stable => "stable",
        Channel::Beta => "beta",
    }
}

/// The package kind for this machine: MSI on Windows, DEB where `apt-get`
/// is and `dnf` is not, otherwise RPM (ADR 0006 section 7).
pub fn platform_with(os: Os, arch: &str, has_program: &dyn Fn(&str) -> bool) -> Platform {
    let (name, pkg) = match os {
        Os::Windows => ("windows", "msi"),
        Os::Linux if has_program("/usr/bin/dnf") => ("linux", "rpm"),
        Os::Linux if has_program("/usr/bin/apt-get") => ("linux", "deb"),
        Os::Linux => ("linux", "rpm"),
    };
    Platform {
        os: name.into(),
        arch: arch.into(),
        pkg: pkg.into(),
    }
}

/// `Disabled` or `NotDue` when an automatic check must not run.
fn gate(
    config: &Config,
    state: &CheckState,
    interval_secs: u64,
    now: u64,
    manual: bool,
) -> Option<CheckOutcome> {
    if !allowed(config.update_check, manual) {
        return Some(CheckOutcome::Disabled);
    }
    if !manual && !due(&state.schedule, now, interval_secs) {
        return Some(CheckOutcome::NotDue);
    }
    None
}

/// Fetches into `dest`, reads it and removes it, whatever happened.
fn fetch_bytes(
    fetcher: &dyn Fetcher,
    url: &str,
    dest: &Path,
    max_bytes: u64,
) -> Result<Vec<u8>, Stop> {
    let fetched = fetcher.get(url, dest, max_bytes);
    let bytes = fs::read(dest);
    let _ = fs::remove_file(dest);
    fetched?;
    bytes.map_err(|_| Stop::Rejected(UpdateError::Io))
}

/// The verdict on a verified manifest: its version and the update, if any.
fn judge(
    env: &CheckEnv,
    config: &Config,
    manifest: &[u8],
    signature: &[u8],
) -> Result<(String, Option<Update>), UpdateError> {
    let signature = std::str::from_utf8(signature).map_err(|_| UpdateError::BadSignature)?;
    verify_manifest(manifest, signature, env.keys)?;
    let m = Manifest::parse(manifest)?;
    match evaluate(&m, env.installed, config.update_channel)? {
        Decision::UpToDate => Ok((m.version, None)),
        Decision::Available(found) => {
            if m.assets.iter().any(|a| !url_is_clean(&a.url)) {
                return Err(UpdateError::BadUrl);
            }
            let asset = select_asset(&m, env.platform.clone())
                .ok_or(UpdateError::NoAsset)?
                .clone();
            Ok((
                m.version,
                Some(Update {
                    version: found.version,
                    notes_url: found.notes_url,
                    asset,
                }),
            ))
        }
    }
}

fn fetch_and_judge(
    env: &CheckEnv,
    config: &Config,
    fetcher: &dyn Fetcher,
) -> Result<(String, Option<Update>), Stop> {
    let url = format!("{FEED}/{}.json", channel_name(config.update_channel));
    let manifest = fetch_bytes(
        fetcher,
        &url,
        &env.scratch.join("manifest.json"),
        MANIFEST_MAX_BYTES,
    )?;
    let signature = fetch_bytes(
        fetcher,
        &format!("{url}.minisig"),
        &env.scratch.join("manifest.json.minisig"),
        SIGNATURE_MAX_BYTES,
    )?;
    judge(env, config, &manifest, &signature).map_err(Stop::Rejected)
}

/// The check flow. A verified manifest resets the failures; anything else
/// counts one failure (the backoff of `schedule::due` needs `last_failure`).
pub fn check_with(
    env: &CheckEnv,
    config: &Config,
    state: &mut CheckState,
    fetcher: &dyn Fetcher,
    now: u64,
    manual: bool,
) -> CheckOutcome {
    if let Some(outcome) = gate(config, state, env.interval_secs, now, manual) {
        return outcome;
    }
    match fetch_and_judge(env, config, fetcher) {
        Ok((version, update)) => {
            state.schedule.last_check = Some(now);
            state.schedule.failures = 0;
            state.last_seen = Some(version);
            update.map_or(CheckOutcome::UpToDate, CheckOutcome::Available)
        }
        Err(stop) => {
            state.schedule.failures = state.schedule.failures.saturating_add(1);
            state.schedule.last_failure = Some(now);
            match stop {
                Stop::Offline => CheckOutcome::Offline,
                Stop::Unavailable => CheckOutcome::Unavailable,
                Stop::Rejected(e) => CheckOutcome::Rejected(e),
            }
        }
    }
}

/// [`check_with`] for the running build: its version, the trusted keys, the
/// running platform, a 24 hour interval and a scratch folder in the cache.
pub fn check(
    config: &Config,
    state: &mut CheckState,
    fetcher: &dyn Fetcher,
    now: u64,
    manual: bool,
) -> CheckOutcome {
    if let Some(outcome) = gate(config, state, INTERVAL_SECS, now, manual) {
        return outcome;
    }
    let scratch = cache_dir_with(CURRENT, &|k| std::env::var_os(k), &|| None)
        .map(|dir| dir.join(format!("check-{}", std::process::id())))
        .filter(|dir| fs::create_dir_all(dir).is_ok());
    let Some(scratch) = scratch else {
        state.schedule.failures = state.schedule.failures.saturating_add(1);
        state.schedule.last_failure = Some(now);
        return CheckOutcome::Rejected(UpdateError::Io);
    };
    let env = CheckEnv {
        installed: env!("CARGO_PKG_VERSION"),
        keys: UPDATE_KEYS,
        platform: platform_with(CURRENT, std::env::consts::ARCH, &|p| Path::new(p).exists()),
        interval_secs: INTERVAL_SECS,
        scratch: &scratch,
    };
    let outcome = check_with(&env, config, state, fetcher, now, manual);
    let _ = fs::remove_dir_all(&scratch);
    if let Some(cache) = scratch.parent() {
        // Leaves no empty cache folder behind; fails harmlessly when it holds files.
        let _ = fs::remove_dir(cache);
    }
    outcome
}

/// True for a file name that stays inside its folder.
fn name_is_plain(name: &str) -> bool {
    !name.is_empty() && url_is_clean(name) && !name.contains('/') && !name.contains(':')
}

/// SHA-256 of a file as lowercase hex, read in 64 KiB chunks.
pub(super) fn sha256_hex(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; CHUNK];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// Downloads the package to `<dir>/<name>.part`, checks its size and SHA-256
/// against the signed manifest and renames it to `<dir>/<name>`. A package
/// that fails never stays as a file. After success the other version folders
/// of `<cache>/updates/` are deleted.
pub fn download_verified(
    update: &Update,
    dir: &Path,
    fetcher: &dyn Fetcher,
) -> Result<VerifiedPackage, UpdateError> {
    let asset = &update.asset;
    if !url_is_clean(&asset.url) || !name_is_plain(&asset.name) {
        return Err(UpdateError::BadUrl);
    }
    fs::create_dir_all(dir).map_err(|_| UpdateError::Io)?;
    let part = dir.join(format!("{}.part", asset.name));
    if let Err(e) = fetch_and_check(asset, &part, fetcher) {
        let _ = fs::remove_file(&part);
        return Err(e);
    }
    let path = dir.join(&asset.name);
    fs::rename(&part, &path).map_err(|_| {
        let _ = fs::remove_file(&part);
        UpdateError::Io
    })?;
    remove_other_versions(dir);
    Ok(VerifiedPackage { path })
}

fn fetch_and_check(asset: &Asset, part: &Path, fetcher: &dyn Fetcher) -> Result<(), UpdateError> {
    fetcher
        .get(&asset.url, part, asset.size)
        .map_err(fetch_to_update_error)?;
    let len = fs::metadata(part).map_err(|_| UpdateError::Io)?.len();
    if len != asset.size {
        return Err(UpdateError::SizeMismatch);
    }
    if sha256_hex(part).map_err(|_| UpdateError::Io)? != asset.sha256 {
        return Err(UpdateError::HashMismatch);
    }
    Ok(())
}

/// Deletes the sibling folders of `dir` when `dir` lives in a folder named
/// `updates`; anywhere else nothing is touched.
fn remove_other_versions(dir: &Path) {
    let Some(parent) = dir
        .parent()
        .filter(|p| p.file_name() == Some("updates".as_ref()))
    else {
        return;
    };
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };
    for entry in entries.flatten() {
        let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());
        if is_dir && entry.path() != dir {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_names() {
        assert!(name_is_plain("sbarhive-0.4.0-1.x86_64.rpm"));
        assert!(name_is_plain("sbarhive_0.4.0~beta.1_amd64.deb"));
        assert!(!name_is_plain(""));
        assert!(!name_is_plain("a/b.rpm"));
        assert!(!name_is_plain("..x"));
        assert!(!name_is_plain("C:evil.msi"));
        assert!(!name_is_plain("a\\b"));
    }

    #[test]
    fn platform_follows_the_package_manager() {
        let both = |_: &str| true;
        let none = |_: &str| false;
        let only_apt = |p: &str| p == "/usr/bin/apt-get";
        assert_eq!(platform_with(Os::Linux, "x86_64", &both).pkg, "rpm");
        assert_eq!(platform_with(Os::Linux, "x86_64", &only_apt).pkg, "deb");
        assert_eq!(platform_with(Os::Linux, "x86_64", &none).pkg, "rpm");
        let win = platform_with(Os::Windows, "x86_64", &none);
        assert_eq!((win.os.as_str(), win.pkg.as_str()), ("windows", "msi"));
        let lin = platform_with(Os::Linux, "aarch64", &both);
        assert_eq!((lin.os.as_str(), lin.arch.as_str()), ("linux", "aarch64"));
    }

    #[test]
    fn other_versions_are_removed_only_under_updates() {
        let t = tempfile::tempdir().unwrap();
        let updates = t.path().join("updates");
        for d in ["0.3.9", "0.4.0"] {
            fs::create_dir_all(updates.join(d)).unwrap();
        }
        fs::write(updates.join("stray.txt"), b"x").unwrap();
        remove_other_versions(&updates.join("0.4.0"));
        assert!(updates.join("0.4.0").is_dir());
        assert!(!updates.join("0.3.9").exists());
        assert!(updates.join("stray.txt").is_file());

        fs::create_dir_all(t.path().join("sibling")).unwrap();
        remove_other_versions(&t.path().join("updates2"));
        remove_other_versions(&t.path().join("sibling"));
        assert!(t.path().join("sibling").is_dir());
        assert!(updates.is_dir());
    }
}
