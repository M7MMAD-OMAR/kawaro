//! S-054a AC8 (backlog/S-054a-update-apply.md): `sbarhive update
//! --install-privileged <file> --sha256 <hex> --kind <rpm|deb>` refuses to run
//! unless the effective user is root: exit 77 and nothing copied. The tests run
//! as the ordinary user (they are skipped as root, where the command would
//! really install). The real `/var/tmp` is only listed, never written. The
//! copy folder is named `sbarhive-update-*` (see `s054a_apply_privileged.rs`).
//! No package manager is started: the refusal comes first.
#![cfg(unix)]

use std::fs;
use std::path::Path;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_sbarhive");
const SHA: &str = "bff5b3ca49e18ea61c9f21a39f95922bcf75f2e0f8ffedf46e07c7cfd70d5d64";

fn euid() -> u32 {
    let out = Command::new("id").arg("-u").output().unwrap();
    String::from_utf8_lossy(&out.stdout).trim().parse().unwrap()
}

fn copies() -> Vec<String> {
    let Ok(rd) = fs::read_dir("/var/tmp") else {
        return Vec::new();
    };
    let mut v: Vec<String> = rd
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("sbarhive-update-"))
        .collect();
    v.sort();
    v
}

fn run(file: &Path, kind: &str) -> std::process::Output {
    Command::new(BIN)
        .args(["update", "--install-privileged"])
        .arg(file)
        .args(["--sha256", SHA, "--kind", kind])
        .output()
        .unwrap()
}

#[test]
fn ac8_a_non_root_user_gets_exit_77_and_nothing_is_copied() {
    if euid() == 0 {
        eprintln!("skipped: running as root");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("sbarhive-0.4.0-1.x86_64.rpm");
    fs::write(&file, b"first-asset-bytes").unwrap();
    for kind in ["rpm", "deb"] {
        let before = copies();
        let out = run(&file, kind);
        assert_eq!(out.status.code(), Some(77), "{kind}: {out:?}");
        assert_eq!(copies(), before, "nothing was copied to /var/tmp");
        assert_eq!(fs::read(&file).unwrap(), b"first-asset-bytes");
    }
}

#[test]
fn ac8_the_refusal_comes_before_the_file_is_looked_at() {
    if euid() == 0 {
        eprintln!("skipped: running as root");
        return;
    }
    let out = run(Path::new("/no/such/sbarhive.rpm"), "rpm");
    assert_eq!(out.status.code(), Some(77), "{out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!err.trim().is_empty(), "a reason is printed on stderr");
}
