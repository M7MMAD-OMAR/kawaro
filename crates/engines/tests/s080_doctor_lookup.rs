//! S-080 (backlog/S-080-windows-doctor-engine-discovery.md), AC1 and AC2 on
//! the one lookup both the engines and `kawaro doctor` use: the Windows
//! known dirs and the registry resolve `ffmpeg`, `ffprobe`, `soffice` and
//! `gs` (the 64-bit Ghostscript family `gswin64c` and `gswin64`) to their
//! absolute paths even when they are absent from `PATH`, and an absent
//! engine resolves to `None`, so its doctor lines say `missing`. The known
//! dir and registry fixtures run on Linux through the injected seam of
//! `locate::search`, the same pure function `locate::find` builds on.
//!
//! Contract (tester): `kawaro_engines::locate` is public with the S-043b
//! signatures unchanged (`enum Os`, `enum Program`, `trait Registry`,
//! `fn names`, `fn search`, `fn find`), so `doctor` can call it and this
//! file can prove the shared resolution. The S-043b name list stays the
//! engines' contract (`gswin64c.exe` then `gswin32c.exe` on Windows); the
//! fixture ships both 64-bit Ghostscript names and asserts the resolution
//! the engines get, so a green doctor line and a runnable engine can never
//! diverge. How `doctor` prints the resolved path is covered in
//! `crates/cli/tests/s080_doctor_engine_tools.rs`.

use kawaro_engines::locate::{Os, Program, Registry, search};
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// An executable file at `dir/name`, returning its path.
fn program(dir: &Path, name: &str) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, "binary").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    path
}

/// Environment lookup over fixed pairs, names compared without case.
fn env(pairs: Vec<(&'static str, PathBuf)>) -> impl Fn(&str) -> Option<OsString> {
    move |key: &str| {
        pairs
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.clone().into_os_string())
    }
}

fn no_env() -> impl Fn(&str) -> Option<OsString> {
    env(vec![])
}

fn norm(s: &str) -> String {
    let s = s.to_ascii_lowercase();
    let s = s
        .strip_prefix("hkey_local_machine\\")
        .or_else(|| s.strip_prefix("hklm\\"))
        .unwrap_or(&s);
    s.to_string()
}

/// Fake registry: values by (key, name) and subkeys by key.
#[derive(Default)]
struct FakeRegistry {
    values: HashMap<(String, String), String>,
    subkeys: HashMap<String, Vec<String>>,
}

impl FakeRegistry {
    fn value(mut self, key: &str, name: &str, data: &Path) -> Self {
        self.values
            .insert((norm(key), norm(name)), data.to_string_lossy().into_owned());
        self
    }

    fn subkeys(mut self, key: &str, names: &[&str]) -> Self {
        self.subkeys
            .insert(norm(key), names.iter().map(|n| n.to_string()).collect());
        self
    }
}

impl Registry for FakeRegistry {
    fn value(&self, key: &str, name: &str) -> Option<String> {
        self.values.get(&(norm(key), norm(name))).cloned()
    }

    fn subkeys(&self, key: &str) -> Vec<String> {
        self.subkeys.get(&norm(key)).cloned().unwrap_or_default()
    }
}

const LO_KEY: &str = "SOFTWARE\\LibreOffice\\UNO\\InstallPath";
const GS_KEY: &str = "SOFTWARE\\GPL Ghostscript";

fn win(
    program_id: Program,
    path_dirs: &[PathBuf],
    env: &dyn Fn(&str) -> Option<OsString>,
    exe_dir: Option<&Path>,
    registry: &dyn Registry,
) -> Option<PathBuf> {
    search(Os::Windows, program_id, path_dirs, env, exe_dir, registry)
}

/// The result is the fixture file's absolute path.
fn assert_resolved(found: Option<PathBuf>, at: &Path) {
    let found = found.unwrap_or_else(|| panic!("no program found at {}", at.display()));
    assert!(found.is_absolute(), "{} must be absolute", found.display());
    assert_eq!(found, at);
}

#[test]
fn ac2_windows_office_known_dir_hit_resolves_the_absolute_path() {
    let t = tempfile::tempdir().unwrap();
    let pf = t.path().join("pf");
    let at = program(&pf.join("LibreOffice").join("program"), "soffice.com");
    let e = env(vec![("ProgramFiles", pf)]);
    let found = win(Program::Office, &[], &e, None, &FakeRegistry::default());
    assert_resolved(found, &at);
}

#[test]
fn ac2_windows_office_registry_hit_resolves_the_absolute_path() {
    let t = tempfile::tempdir().unwrap();
    let at = program(
        &t.path().join("Libre Office 24").join("program"),
        "soffice.exe",
    );
    let reg = FakeRegistry::default().value(
        LO_KEY,
        "",
        &t.path().join("Libre Office 24").join("program"),
    );
    let e = env(vec![("ProgramFiles", t.path().join("pf"))]);
    let found = win(Program::Office, &[], &e, None, &reg);
    assert_resolved(found, &at);
}

#[test]
fn ac2_windows_ghostscript_known_dir_hit_resolves_gswin64c_and_gswin64() {
    let t = tempfile::tempdir().unwrap();
    let pf = t.path().join("pf");
    let bin = pf.join("gs").join("gs10.04.0").join("bin");
    // The 64-bit Ghostscript family named in AC1, console and GUI.
    program(&bin, "gswin64c.exe");
    program(&bin, "gswin64.exe");
    let e = env(vec![("ProgramFiles", pf)]);
    let found = win(
        Program::Ghostscript,
        &[],
        &e,
        None,
        &FakeRegistry::default(),
    );
    let found = found.expect("Ghostscript resolves in the known dir");
    assert!(found.is_absolute(), "{} must be absolute", found.display());
    assert_eq!(found.parent(), Some(bin.as_path()));
    assert!(
        found.file_name().is_some_and(|n| {
            let n = n.to_string_lossy();
            n == "gswin64c.exe" || n == "gswin64.exe"
        }),
        "the resolved program is one of the 64-bit Ghostscript names: {}",
        found.display()
    );
}

#[test]
fn ac2_windows_ghostscript_registry_hit_resolves_the_absolute_path() {
    let t = tempfile::tempdir().unwrap();
    let bin = t.path().join("gs").join("bin");
    let at = program(&bin, "gswin64c.exe");
    let reg = FakeRegistry::default().subkeys(GS_KEY, &["10.04.0"]).value(
        &format!("{GS_KEY}\\10.04.0"),
        "GS_DLL",
        &bin.join("gsdll64.dll"),
    );
    let e = env(vec![("ProgramFiles", t.path().join("pf"))]);
    let found = win(Program::Ghostscript, &[], &e, None, &reg);
    assert_resolved(found, &at);
}

#[test]
fn ac2_windows_ffmpeg_known_dir_hit_resolves_ffmpeg_and_ffprobe() {
    let t = tempfile::tempdir().unwrap();
    let exe_dir = t.path().join("app");
    let bin = exe_dir.join("ffmpeg").join("bin");
    let ffmpeg = program(&bin, "ffmpeg.exe");
    let ffprobe = program(&bin, "ffprobe.exe");
    let found = win(
        Program::Ffmpeg,
        &[],
        &no_env(),
        Some(&exe_dir),
        &FakeRegistry::default(),
    );
    assert_resolved(found, &ffmpeg);
    let found = win(
        Program::Ffprobe,
        &[],
        &no_env(),
        Some(&exe_dir),
        &FakeRegistry::default(),
    );
    assert_resolved(found, &ffprobe);
}

#[test]
fn ac1_an_absent_engine_resolves_to_none_so_the_doctor_lines_say_missing() {
    let t = tempfile::tempdir().unwrap();
    let e = env(vec![("ProgramFiles", t.path().join("pf"))]);
    for program_id in [
        Program::Ffmpeg,
        Program::Ffprobe,
        Program::Office,
        Program::Ghostscript,
    ] {
        let found = win(
            program_id,
            &[t.path().join("empty")],
            &e,
            Some(&t.path().join("app")),
            &FakeRegistry::default(),
        );
        assert_eq!(found, None, "{program_id:?} must resolve to None");
    }
}
