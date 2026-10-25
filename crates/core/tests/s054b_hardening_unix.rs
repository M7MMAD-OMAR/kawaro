//! S-054b review rulings that need unix files: the `updates` folder that is a
//! symlink (S-053b), the root-owned exe check (S-054a) and the minimal curl
//! environment (S-053b). The contract is in `s054b_core.rs`.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::Path;

use kawaro_core::update::UpdateError;
use kawaro_core::update::apply::{PkgKind, privileged_argv_checked};
use kawaro_core::update::fetch::{CurlFetcher, FetchError, Fetcher};
use kawaro_core::update::flow::{Update, download_verified};
use kawaro_core::update::manifest::Asset;

const FIRST: &[u8] = b"first-asset-bytes";
const SHA_FIRST: &str = "bff5b3ca49e18ea61c9f21a39f95922bcf75f2e0f8ffedf46e07c7cfd70d5d64";
const NAME: &str = "kawaro-0.4.0-1.x86_64.rpm";

struct Serve(&'static [u8]);

impl Fetcher for Serve {
    fn get(&self, _url: &str, dest: &Path, _max: u64) -> Result<(), FetchError> {
        fs::write(dest, self.0).unwrap();
        Ok(())
    }
}

fn update() -> Update {
    Update {
        version: "0.4.0".to_string(),
        notes_url: "https://github.com/M7MMAD-OMAR/kawaro/releases/tag/v0.4.0".to_string(),
        asset: Asset {
            kind: "rpm".to_string(),
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            name: NAME.to_string(),
            url: format!("https://github.com/M7MMAD-OMAR/kawaro/releases/download/v0.4.0/{NAME}"),
            size: 17,
            sha256: SHA_FIRST.to_string(),
        },
    }
}

#[test]
fn a_symlinked_updates_folder_never_loses_the_folders_of_its_target() {
    let t = tempfile::tempdir().unwrap();
    // Control: a real `updates` folder still loses its other versions.
    let plain = t.path().join("plain/updates");
    fs::create_dir_all(plain.join("0.3.9")).unwrap();
    download_verified(&update(), &plain.join("0.4.0"), &Serve(FIRST)).unwrap();
    assert!(plain.join("0.4.0").join(NAME).is_file());
    assert!(!plain.join("0.3.9").exists(), "the old version is removed");

    // The symlink: the folders it points at are not ours to delete.
    let real = t.path().join("precious");
    for d in ["0.3.9", "documents"] {
        fs::create_dir_all(real.join(d)).unwrap();
        fs::write(real.join(d).join("keep.txt"), b"keep").unwrap();
    }
    let cache = t.path().join("cache");
    fs::create_dir_all(&cache).unwrap();
    symlink(&real, cache.join("updates")).unwrap();
    let _ = download_verified(&update(), &cache.join("updates/0.4.0"), &Serve(FIRST));
    for d in ["0.3.9", "documents"] {
        assert!(
            real.join(d).join("keep.txt").is_file(),
            "{d} in the symlink target must survive"
        );
    }
}

fn euid() -> u32 {
    let status = fs::read_to_string("/proc/self/status").unwrap();
    kawaro_core::update::apply::effective_uid(&status).unwrap()
}

#[test]
fn privileged_argv_checked_refuses_an_exe_that_is_not_root_owned() {
    let t = tempfile::tempdir().unwrap();
    let mine = t.path().join("kawaro");
    fs::write(&mine, b"#!/bin/sh\n").unwrap();
    fs::set_permissions(&mine, fs::Permissions::from_mode(0o755)).unwrap();
    let file = Path::new("/c/kawaro.rpm");
    if euid() != 0 {
        assert_eq!(
            privileged_argv_checked(&mine, file, "ab", PkgKind::Rpm),
            Err(UpdateError::Io)
        );
    }
    // A missing exe is refused too.
    assert_eq!(
        privileged_argv_checked(&t.path().join("none"), file, "ab", PkgKind::Rpm),
        Err(UpdateError::Io)
    );
    // A system program owned by root passes and is re-run as given.
    let argv = privileged_argv_checked(Path::new("/usr/bin/env"), file, "ab", PkgKind::Deb)
        .expect("/usr/bin/env is root owned");
    assert_eq!(argv[0], "/usr/bin/pkexec");
    assert_eq!(argv[1], "/usr/bin/env");
    assert_eq!(argv.last().map(String::as_str), Some("deb"));
}

#[test]
fn curl_runs_with_a_cleared_environment() {
    let t = tempfile::tempdir().unwrap();
    let curl = t.path().join("curl");
    // Shell builtins only: PATH is gone. The answer lands in the --output file.
    fs::write(
        &curl,
        "#!/bin/sh\nout=\"\"; prev=\"\"\n\
         for a in \"$@\"; do if [ \"$prev\" = \"--output\" ]; then out=\"$a\"; fi; prev=\"$a\"; done\n\
         printf '%s|%s|%s|%s|%s|%s|%s' \"${https_proxy-unset}\" \"${HTTPS_PROXY-unset}\" \
         \"${CURL_CA_BUNDLE-unset}\" \"${SSL_CERT_FILE-unset}\" \"${HOME-unset}\" \
         \"${LD_PRELOAD-unset}\" \"${LC_ALL-unset}\" > \"$out\"\n",
    )
    .unwrap();
    fs::set_permissions(&curl, fs::Permissions::from_mode(0o755)).unwrap();
    // Only test in this binary, so no other thread reads the environment.
    for (k, v) in [
        ("https_proxy", "http://evil.example:3128"),
        ("HTTPS_PROXY", "http://evil.example:3128"),
        ("CURL_CA_BUNDLE", "/tmp/evil-ca.pem"),
        ("SSL_CERT_FILE", "/tmp/evil-ca.pem"),
        ("LD_PRELOAD", "/tmp/evil.so"),
    ] {
        unsafe { std::env::set_var(k, v) };
    }
    let dest = t.path().join("out.txt");
    CurlFetcher::with_program(&curl)
        .get(
            "https://github.com/M7MMAD-OMAR/kawaro/releases/download/updates/stable.json",
            &dest,
            1000,
        )
        .unwrap();
    assert_eq!(
        fs::read_to_string(&dest).unwrap(),
        "unset|unset|unset|unset|unset|unset|C"
    );
}
