//! S-054a AC3 (backlog/S-054a-update-apply.md): `install_privileged` against
//! fake `dnf` and `apt-get` shell scripts in a temp dir. No real package
//! manager, `pkexec`, `rpm` or `dpkg` is ever started and nothing touches the
//! system: `root_dir` is a temp folder. The contract is in `s054a_apply.rs`.
//!
//! Extra DECISIONs (reported): the copy lives in a new folder named
//! `kawaro-update-*` directly under `root_dir`, keeps the original file name
//! (dnf and apt need the suffix), the folder has mode 0700 and the file no
//! group or other bits; the folder is removed on every path (success, manager
//! failure, mismatch), so `root_dir` is empty afterwards. A manager that exits
//! non zero is `Err(UpdateError::Io)`.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use kawaro_core::update::UpdateError;
use kawaro_core::update::apply::{PkgKind, Programs, install_privileged};

static LOCK: Mutex<()> = Mutex::new(());

/// Serialises writing and running scripts, as in the S-053b tests.
fn lock() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

const GOOD: &[u8] = b"first-asset-bytes";
const SHA_GOOD: &str = "bff5b3ca49e18ea61c9f21a39f95922bcf75f2e0f8ffedf46e07c7cfd70d5d64";
const BAD: &[u8] = b"tampered-bytes!!";

struct Lab {
    _tmp: tempfile::TempDir,
    root_dir: PathBuf,
    cache: PathBuf,
    bin: PathBuf,
    programs: Programs,
}

impl Lab {
    fn log(&self, name: &str) -> PathBuf {
        self.bin.join(format!("{name}.log"))
    }
    /// The recorded invocations of `name`: one `Vec` of argv per call.
    fn calls(&self, name: &str) -> Vec<Vec<String>> {
        let Ok(text) = fs::read_to_string(self.log(name)) else {
            return Vec::new();
        };
        text.split("--call--\n")
            .filter(|c| !c.is_empty())
            .map(|c| c.lines().map(str::to_string).collect())
            .collect()
    }
    fn root_entries(&self) -> Vec<String> {
        fs::read_dir(&self.root_dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect()
    }
}

/// A fake manager that records its argv, then copies the package it was asked
/// to install (to `<name>.seen`) and writes the modes of the copy and of its
/// folder (to `<name>.modes`), then exits with `code`.
fn fake(bin: &Path, name: &str, code: i32) -> PathBuf {
    let path = bin.join(name);
    let log = bin.join(format!("{name}.log"));
    let text = format!(
        "#!/bin/sh\n[ -n \"$FAKE_EXEC_PROBE\" ] && exit 0\n{{ echo '--call--'; printf '%s\\n' \"$@\"; }} >> '{log}'\n\
         pkg=\"\"\nfor a in \"$@\"; do pkg=\"$a\"; done\n\
         if [ -f \"$pkg\" ]; then cp \"$pkg\" '{bin}/{name}.seen'; \
         stat -c '%a' \"$pkg\" > '{bin}/{name}.filemode'; \
         stat -c '%a' \"$(dirname \"$pkg\")\" > '{bin}/{name}.dirmode'; fi\n\
         exit {code}\n",
        log = log.display(),
        bin = bin.display(),
    );
    fs::write(&path, text).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    wait_until_executable(&path);
    path
}

/// Waits until a freshly written fake program can be executed. A fork in
/// another test thread can briefly inherit the write descriptor, and exec then
/// fails with ETXTBSY ("Text file busy"), which the code under test reports as
/// a missing program. The probe run exits at once (`FAKE_EXEC_PROBE`).
fn wait_until_executable(path: &Path) {
    const ETXTBSY: i32 = 26;
    for _ in 0..400 {
        match std::process::Command::new(path)
            .env("FAKE_EXEC_PROBE", "1")
            .status()
        {
            Err(e) if e.raw_os_error() == Some(ETXTBSY) => {
                std::thread::sleep(std::time::Duration::from_millis(5))
            }
            _ => return,
        }
    }
    panic!("{} stayed busy", path.display());
}

fn lab(dnf_code: i32, apt_code: i32) -> Lab {
    let tmp = tempfile::tempdir().unwrap();
    let root_dir = tmp.path().join("root");
    let cache = tmp.path().join("cache");
    let bin = tmp.path().join("bin");
    for d in [&root_dir, &cache, &bin] {
        fs::create_dir(d).unwrap();
    }
    let programs = Programs {
        dnf: fake(&bin, "dnf", dnf_code),
        apt_get: fake(&bin, "apt-get", apt_code),
    };
    Lab {
        _tmp: tmp,
        root_dir,
        cache,
        bin,
        programs,
    }
}

fn pkg(lab: &Lab, name: &str, bytes: &[u8]) -> PathBuf {
    let p = lab.cache.join(name);
    fs::write(&p, bytes).unwrap();
    p
}

// ---------------------------------------------------------------- good file

#[test]
fn ac3_rpm_runs_dnf_install_on_the_root_owned_copy_only() {
    let _g = lock();
    let lab = lab(0, 0);
    let file = pkg(&lab, "kawaro-0.4.0-1.x86_64.rpm", GOOD);
    install_privileged(&file, SHA_GOOD, PkgKind::Rpm, &lab.root_dir, &lab.programs).unwrap();

    let calls = lab.calls("dnf");
    assert_eq!(calls.len(), 1, "one dnf call");
    let argv = &calls[0];
    assert_eq!(argv.len(), 3, "install -y <copy>, nothing else: {argv:?}");
    assert_eq!(argv[0], "install");
    assert_eq!(argv[1], "-y");
    let copy = Path::new(&argv[2]);
    assert!(
        copy.starts_with(&lab.root_dir),
        "copy under root_dir: {copy:?}"
    );
    assert_ne!(copy, file.as_path(), "never the original");
    assert!(!copy.starts_with(&lab.cache));
    assert_eq!(copy.file_name(), file.file_name());
    let folder = copy.parent().unwrap();
    assert_eq!(folder.parent().unwrap(), lab.root_dir.as_path());
    assert!(
        folder
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("kawaro-update-"),
        "{folder:?}"
    );
    assert!(argv.iter().all(|a| !a.contains("nogpgcheck")));
    assert!(
        lab.calls("apt-get").is_empty(),
        "the other manager never runs"
    );

    assert_eq!(fs::read(lab.bin.join("dnf.seen")).unwrap(), GOOD);
    let dir_mode = fs::read_to_string(lab.bin.join("dnf.dirmode")).unwrap();
    assert_eq!(dir_mode.trim(), "700");
    let file_mode = u32::from_str_radix(
        fs::read_to_string(lab.bin.join("dnf.filemode"))
            .unwrap()
            .trim(),
        8,
    )
    .unwrap();
    assert_eq!(
        file_mode & 0o077,
        0,
        "no group or other access: {file_mode:o}"
    );
    assert_ne!(file_mode & 0o400, 0, "the owner can read it");

    assert_eq!(fs::read(&file).unwrap(), GOOD, "the original is untouched");
    assert!(
        lab.root_entries().is_empty(),
        "the copy is removed afterwards"
    );
}

#[test]
fn ac3_deb_runs_apt_get_install_on_the_copy() {
    let _g = lock();
    let lab = lab(0, 0);
    let file = pkg(&lab, "kawaro_0.4.0-1_amd64.deb", GOOD);
    install_privileged(&file, SHA_GOOD, PkgKind::Deb, &lab.root_dir, &lab.programs).unwrap();

    let calls = lab.calls("apt-get");
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].len(), 3, "{:?}", calls[0]);
    assert_eq!(calls[0][0], "install");
    assert_eq!(calls[0][1], "-y");
    let copy = Path::new(&calls[0][2]);
    assert!(copy.starts_with(&lab.root_dir));
    assert_eq!(copy.file_name(), file.file_name());
    assert!(copy.is_absolute());
    assert_eq!(fs::read(lab.bin.join("apt-get.seen")).unwrap(), GOOD);
    assert_eq!(
        fs::read_to_string(lab.bin.join("apt-get.dirmode"))
            .unwrap()
            .trim(),
        "700"
    );
    assert!(lab.calls("dnf").is_empty());
    assert!(lab.root_entries().is_empty());
}

#[test]
fn ac3_two_installs_use_two_different_folders() {
    let _g = lock();
    let lab = lab(0, 0);
    let file = pkg(&lab, "kawaro.rpm", GOOD);
    for _ in 0..2 {
        install_privileged(&file, SHA_GOOD, PkgKind::Rpm, &lab.root_dir, &lab.programs).unwrap();
    }
    let calls = lab.calls("dnf");
    assert_eq!(calls.len(), 2);
    assert_ne!(calls[0][2], calls[1][2]);
}

// ----------------------------------------------------------- tampered files

#[test]
fn ac3_a_file_that_does_not_match_the_hash_runs_nothing() {
    let _g = lock();
    for kind in [PkgKind::Rpm, PkgKind::Deb] {
        let lab = lab(0, 0);
        let file = pkg(&lab, "kawaro.pkg", BAD);
        let out = install_privileged(&file, SHA_GOOD, kind, &lab.root_dir, &lab.programs);
        assert_eq!(out, Err(UpdateError::HashMismatch), "{kind:?}");
        assert!(lab.calls("dnf").is_empty(), "{kind:?}");
        assert!(lab.calls("apt-get").is_empty(), "{kind:?}");
        assert!(
            lab.root_entries().is_empty(),
            "the copy is deleted: {kind:?}"
        );
        assert_eq!(fs::read(&file).unwrap(), BAD);
    }
}

#[test]
fn ac3_a_file_replaced_after_the_caller_checked_it_is_a_mismatch() {
    let _g = lock();
    let lab = lab(0, 0);
    let file = pkg(&lab, "kawaro.rpm", GOOD);
    // The caller verified `GOOD` against SHA_GOOD, then the user-writable
    // file is swapped for other bytes of a different size and for the same
    // size, before the privileged call.
    for swapped in [&b"evil"[..], &b"first-asset-bytez"[..], &b""[..]] {
        fs::write(&file, swapped).unwrap();
        let out = install_privileged(&file, SHA_GOOD, PkgKind::Rpm, &lab.root_dir, &lab.programs);
        assert_eq!(out, Err(UpdateError::HashMismatch), "{swapped:?}");
    }
    assert!(lab.calls("dnf").is_empty());
    assert!(lab.calls("apt-get").is_empty());
    assert!(lab.root_entries().is_empty());
}

#[test]
fn ac3_a_malformed_expected_hash_never_matches() {
    let _g = lock();
    let lab = lab(0, 0);
    let file = pkg(&lab, "kawaro.rpm", GOOD);
    for bad in ["", "abc", &SHA_GOOD.to_uppercase(), &SHA_GOOD[..63]] {
        let out = install_privileged(&file, bad, PkgKind::Rpm, &lab.root_dir, &lab.programs);
        assert!(
            matches!(
                out,
                Err(UpdateError::HashMismatch) | Err(UpdateError::BadSha256)
            ),
            "{bad:?}: {out:?}"
        );
    }
    assert!(lab.calls("dnf").is_empty());
    assert!(lab.root_entries().is_empty());
}

// --------------------------------------------------------- other failures

#[test]
fn ac3_a_missing_file_is_io_and_runs_nothing() {
    let _g = lock();
    let lab = lab(0, 0);
    let out = install_privileged(
        &lab.cache.join("nope.rpm"),
        SHA_GOOD,
        PkgKind::Rpm,
        &lab.root_dir,
        &lab.programs,
    );
    assert_eq!(out, Err(UpdateError::Io));
    assert!(lab.calls("dnf").is_empty());
    assert!(lab.root_entries().is_empty());
}

#[test]
fn ac3_a_manager_that_fails_is_io_and_the_copy_is_still_removed() {
    let _g = lock();
    for kind in [PkgKind::Rpm, PkgKind::Deb] {
        let lab = lab(1, 100);
        let file = pkg(&lab, "kawaro.pkg", GOOD);
        let out = install_privileged(&file, SHA_GOOD, kind, &lab.root_dir, &lab.programs);
        assert_eq!(out, Err(UpdateError::Io), "{kind:?}");
        assert_eq!(lab.calls("dnf").len() + lab.calls("apt-get").len(), 1);
        assert!(lab.root_entries().is_empty(), "{kind:?}");
    }
}

#[test]
fn ac3_a_manager_that_cannot_start_is_io() {
    let _g = lock();
    let mut lab = lab(0, 0);
    lab.programs.dnf = lab.bin.join("no-such-dnf");
    let file = pkg(&lab, "kawaro.rpm", GOOD);
    let out = install_privileged(&file, SHA_GOOD, PkgKind::Rpm, &lab.root_dir, &lab.programs);
    assert_eq!(out, Err(UpdateError::Io));
    assert!(lab.root_entries().is_empty());
}

#[test]
fn ac3_a_root_dir_that_does_not_exist_is_io_and_nothing_is_created() {
    let _g = lock();
    let lab = lab(0, 0);
    let missing = lab.root_dir.join("missing");
    let file = pkg(&lab, "kawaro.rpm", GOOD);
    let out = install_privileged(&file, SHA_GOOD, PkgKind::Rpm, &missing, &lab.programs);
    assert_eq!(out, Err(UpdateError::Io));
    assert!(!missing.exists());
    assert!(lab.calls("dnf").is_empty());
}
