// S-043b (backlog/S-043-windows-engines.md): where each engine program is
// looked up, as pure functions with the environment and the registry
// injected, so a Linux run tests and mutates the Windows order. The test
// module below is the tester's contract and must not change; the
// implementation goes above it.
//
// Contract pinned by the tests (all items visible to `super::*`):
// - `enum Os { Unix, Windows }`: `Clone`, `Copy`, `Debug`, `PartialEq`.
// - `enum Program { Office, Ghostscript, Ffmpeg, Ffprobe }`: same derives.
//   It fills the `names` slot of AC7's signature: each program has its own
//   name list per system.
// - `fn names(os: Os, program: Program) -> &'static [&'static str]`, in
//   priority order.
// - `trait Registry` (read only, under `HKEY_LOCAL_MACHINE`; key paths are
//   given without the hive and separated by `\`, for example
//   `SOFTWARE\LibreOffice\UNO\InstallPath`; the fakes compare keys and value
//   names without case, as Windows does):
//     fn value(&self, key: &str, name: &str) -> Option<String>;
//         // string value `name` of `key`; `""` is the key's default value
//     fn subkeys(&self, key: &str) -> Vec<String>;
//         // names of the direct subkeys of `key`
// - `fn search(os: Os, program: Program, path_dirs: &[PathBuf],
//       env: &dyn Fn(&str) -> Option<OsString>, exe_dir: Option<&Path>,
//       registry: &dyn Registry) -> Option<PathBuf>`
//   (a generic `impl Fn` for `env` also fits these tests). The first
//   executable file found, in this order:
//     1. `path_dirs`, names first as `office::find_program` does today:
//        the first name in any dir, then the second name in any dir;
//     2. Windows only, the known dirs:
//        Office:      `env("ProgramFiles")\LibreOffice\program`
//        Ghostscript: `env("ProgramFiles")\gs\<newest gs*>\bin`
//        Ffmpeg and Ffprobe: `exe_dir\ffmpeg\bin`
//     3. Windows only, the registry:
//        Office:      the folder in the default value of
//                     `SOFTWARE\LibreOffice\UNO\InstallPath`
//        Ghostscript: the newest version subkey of `SOFTWARE\GPL Ghostscript`
//                     (by number, as `newest_gs`), whose `GS_DLL` value is
//                     the full path of the DLL in the `bin` folder; the
//                     program is looked up in that folder.
//   Within one dir the names keep their order (`soffice.com` before
//   `soffice.exe`). On Unix only step 1 runs.
// - `fn newest_gs<'a>(names: &[&'a str]) -> Option<&'a str>`: the `gs<ver>`
//   name with the highest version compared number by number; names that are
//   not `gs` followed by a version are ignored.

use crate::office::{find_program, path_dirs};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[cfg(windows)]
mod windows;

/// Host operating system family for program name selection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Os {
    Unix,
    Windows,
}

/// The system this build runs on. A value, not a `cfg`, so the Windows
/// order in [`search`] builds and runs everywhere.
const OS: Os = if cfg!(windows) { Os::Windows } else { Os::Unix };

/// A resolved engine program to look up on the system.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Program {
    Office,
    Ghostscript,
    Ffmpeg,
    Ffprobe,
}

/// Program file names in priority order.
pub(crate) fn names(os: Os, program: Program) -> &'static [&'static str] {
    match (os, program) {
        (Os::Unix, Program::Office) => &["soffice", "libreoffice"],
        (Os::Unix, Program::Ghostscript) => &["gs"],
        (Os::Unix, Program::Ffmpeg) => &["ffmpeg"],
        (Os::Unix, Program::Ffprobe) => &["ffprobe"],
        (Os::Windows, Program::Office) => &["soffice.com", "soffice.exe"],
        (Os::Windows, Program::Ghostscript) => &["gswin64c.exe", "gswin32c.exe"],
        (Os::Windows, Program::Ffmpeg) => &["ffmpeg.exe"],
        (Os::Windows, Program::Ffprobe) => &["ffprobe.exe"],
    }
}

/// Read only view of `HKEY_LOCAL_MACHINE`. Keys are given without the hive.
pub trait Registry {
    /// String value `name` of `key`; `""` is the key's default value.
    fn value(&self, key: &str, name: &str) -> Option<String>;
    /// Names of the direct subkeys of `key`.
    fn subkeys(&self, key: &str) -> Vec<String>;
}

// Both keys are assumptions from ADR 0005, to be checked on Windows (S-052).
const LO_KEY: &str = "SOFTWARE\\LibreOffice\\UNO\\InstallPath";
const GS_KEY: &str = "SOFTWARE\\GPL Ghostscript";

/// The first executable `program` found: `path_dirs`, then on Windows the
/// usual install folder, then the folder the registry names.
pub fn search(
    os: Os,
    program: Program,
    path_dirs: &[PathBuf],
    env: &dyn Fn(&str) -> Option<OsString>,
    exe_dir: Option<&Path>,
    registry: &dyn Registry,
) -> Option<PathBuf> {
    let names = names(os, program);
    let found = find_program(names, path_dirs);
    if found.is_some() || os == Os::Unix {
        return found;
    }
    let in_dir = |dir: PathBuf| find_program(names, &[dir]);
    known_dir(program, env, exe_dir)
        .and_then(in_dir)
        .or_else(|| registry_dir(program, registry).and_then(in_dir))
}

/// Where the Windows installers put the program.
fn known_dir(
    program: Program,
    env: &dyn Fn(&str) -> Option<OsString>,
    exe_dir: Option<&Path>,
) -> Option<PathBuf> {
    match program {
        Program::Office => {
            env("ProgramFiles").map(|pf| PathBuf::from(pf).join("LibreOffice").join("program"))
        }
        Program::Ghostscript => {
            let gs = PathBuf::from(env("ProgramFiles")?).join("gs");
            let dirs: Vec<String> = std::fs::read_dir(&gs)
                .ok()?
                .filter_map(|e| e.ok()?.file_name().into_string().ok())
                .collect();
            let dirs: Vec<&str> = dirs.iter().map(String::as_str).collect();
            Some(gs.join(newest_gs(&dirs)?).join("bin"))
        }
        Program::Ffmpeg | Program::Ffprobe => exe_dir.map(|d| d.join("ffmpeg").join("bin")),
    }
}

/// The folder the installer recorded in the registry.
fn registry_dir(program: Program, registry: &dyn Registry) -> Option<PathBuf> {
    match program {
        Program::Office => registry.value(LO_KEY, "").map(PathBuf::from),
        Program::Ghostscript => {
            let versions = registry.subkeys(GS_KEY);
            let versions: Vec<&str> = versions.iter().map(String::as_str).collect();
            let newest = newest(&versions, "")?;
            let dll = registry.value(&format!("{GS_KEY}\\{newest}"), "GS_DLL")?;
            Path::new(&dll).parent().map(Path::to_path_buf)
        }
        Program::Ffmpeg | Program::Ffprobe => None,
    }
}

/// The `gs<version>` name with the highest version, compared number by number.
pub(crate) fn newest_gs<'a>(names: &[&'a str]) -> Option<&'a str> {
    newest(names, "gs")
}

/// The `<prefix><version>` name with the highest version; other names are
/// ignored.
fn newest<'a>(names: &[&'a str], prefix: &str) -> Option<&'a str> {
    names
        .iter()
        .filter_map(|name| Some((version(name.strip_prefix(prefix)?)?, *name)))
        .max_by_key(|(v, _)| v.clone())
        .map(|(_, name)| name)
}

/// `10.04.0` as `[10, 4, 0]`; `None` unless every part is a number.
fn version(text: &str) -> Option<Vec<u32>> {
    text.split('.').map(|part| part.parse().ok()).collect()
}

/// `program` on this system, from the live `PATH`, environment, executable
/// folder and registry.
pub fn find(program: Program) -> Option<PathBuf> {
    let exe = std::env::current_exe().ok();
    search(
        OS,
        program,
        &path_dirs(),
        &|key| std::env::var_os(key),
        exe.as_deref().and_then(Path::parent),
        &SystemRegistry,
    )
}

#[cfg(windows)]
use windows::SystemRegistry;

/// No registry off Windows; [`search`] never asks it there.
#[cfg(not(windows))]
struct SystemRegistry;

#[cfg(not(windows))]
impl Registry for SystemRegistry {
    fn value(&self, _key: &str, _name: &str) -> Option<String> {
        None
    }
    fn subkeys(&self, _key: &str) -> Vec<String> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    /// An executable file at `dir/name` (exec bit set on Unix), returning its path.
    fn program(dir: &Path, name: &str) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, "#!/bin/sh\n").unwrap();
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

    fn lo_registry(dir: &Path) -> FakeRegistry {
        FakeRegistry::default().value(LO_KEY, "", dir)
    }

    fn gs_registry(entries: &[(&str, &Path)]) -> FakeRegistry {
        let versions: Vec<&str> = entries.iter().map(|(v, _)| *v).collect();
        let mut reg = FakeRegistry::default().subkeys(GS_KEY, &versions);
        for (version, bin) in entries {
            reg = reg.value(
                &format!("{GS_KEY}\\{version}"),
                "GS_DLL",
                &bin.join("gsdll64.dll"),
            );
        }
        reg
    }

    fn win(
        program: Program,
        path_dirs: &[PathBuf],
        env: &dyn Fn(&str) -> Option<OsString>,
        exe_dir: Option<&Path>,
        registry: &dyn Registry,
    ) -> Option<PathBuf> {
        search(Os::Windows, program, path_dirs, env, exe_dir, registry)
    }

    // AC7: names

    #[test]
    fn ac7_windows_names_per_program() {
        assert_eq!(
            names(Os::Windows, Program::Office),
            ["soffice.com", "soffice.exe"]
        );
        assert_eq!(
            names(Os::Windows, Program::Ghostscript),
            ["gswin64c.exe", "gswin32c.exe"]
        );
        assert_eq!(names(Os::Windows, Program::Ffmpeg), ["ffmpeg.exe"]);
        assert_eq!(names(Os::Windows, Program::Ffprobe), ["ffprobe.exe"]);
    }

    #[test]
    fn ac7_unix_names_as_today() {
        assert_eq!(names(Os::Unix, Program::Office), ["soffice", "libreoffice"]);
        assert_eq!(names(Os::Unix, Program::Ghostscript), ["gs"]);
        assert_eq!(names(Os::Unix, Program::Ffmpeg), ["ffmpeg"]);
        assert_eq!(names(Os::Unix, Program::Ffprobe), ["ffprobe"]);
    }

    // AC7: LibreOffice on Windows

    #[test]
    fn ac7_windows_office_path_wins_over_program_files_and_registry() {
        let t = tempfile::tempdir().unwrap();
        let bin = t.path().join("bin");
        let found = program(&bin, "soffice.exe");
        let pf = t.path().join("pf");
        program(&pf.join("LibreOffice").join("program"), "soffice.com");
        let reg_dir = t.path().join("reg");
        program(&reg_dir, "soffice.com");
        let e = env(vec![("ProgramFiles", pf)]);
        let r = win(
            Program::Office,
            &[t.path().join("empty"), bin],
            &e,
            Some(t.path()),
            &lo_registry(&reg_dir),
        );
        assert_eq!(r, Some(found));
    }

    #[test]
    fn ac7_windows_office_com_before_exe_in_one_dir() {
        let t = tempfile::tempdir().unwrap();
        let pf = t.path().join("pf");
        let dir = pf.join("LibreOffice").join("program");
        program(&dir, "soffice.exe");
        let com = program(&dir, "soffice.com");
        let e = env(vec![("ProgramFiles", pf)]);
        let r = win(Program::Office, &[], &e, None, &FakeRegistry::default());
        assert_eq!(r, Some(com));

        let bin = t.path().join("bin");
        program(&bin, "soffice.exe");
        let com = program(&bin, "soffice.com");
        let r = win(
            Program::Office,
            std::slice::from_ref(&bin),
            &no_env(),
            None,
            &FakeRegistry::default(),
        );
        assert_eq!(r, Some(com));
    }

    #[test]
    fn ac7_windows_office_program_files_wins_over_registry() {
        let t = tempfile::tempdir().unwrap();
        let pf = t.path().join("pf");
        let found = program(&pf.join("LibreOffice").join("program"), "soffice.exe");
        let reg_dir = t.path().join("reg");
        program(&reg_dir, "soffice.com");
        let e = env(vec![("ProgramFiles", pf)]);
        let r = win(
            Program::Office,
            &[t.path().join("empty")],
            &e,
            None,
            &lo_registry(&reg_dir),
        );
        assert_eq!(r, Some(found));
    }

    #[test]
    fn ac7_windows_office_registry_install_path_last() {
        let t = tempfile::tempdir().unwrap();
        let reg_dir = t.path().join("Libre Office 24").join("program");
        let found = program(&reg_dir, "soffice.exe");
        // ProgramFiles is set but holds no LibreOffice.
        let e = env(vec![("ProgramFiles", t.path().join("pf"))]);
        let r = win(
            Program::Office,
            &[t.path().join("empty")],
            &e,
            None,
            &lo_registry(&reg_dir),
        );
        assert_eq!(r, Some(found));
    }

    #[test]
    fn ac7_windows_office_none_when_missing_everywhere() {
        let t = tempfile::tempdir().unwrap();
        let e = env(vec![("ProgramFiles", t.path().join("pf"))]);
        // The registry names a folder that holds no program.
        let r = win(
            Program::Office,
            &[t.path().to_path_buf()],
            &e,
            Some(t.path()),
            &lo_registry(&t.path().join("gone")),
        );
        assert_eq!(r, None);
        let r = win(
            Program::Office,
            &[],
            &no_env(),
            None,
            &FakeRegistry::default(),
        );
        assert_eq!(r, None);
    }

    // AC7 and AC8: Ghostscript on Windows

    #[test]
    fn ac7_windows_ghostscript_path_wins_and_64_before_32() {
        let t = tempfile::tempdir().unwrap();
        let bin = t.path().join("bin");
        program(&bin, "gswin32c.exe");
        let found = program(&bin, "gswin64c.exe");
        let pf = t.path().join("pf");
        program(&pf.join("gs").join("gs10.04.0").join("bin"), "gswin64c.exe");
        let reg_bin = t.path().join("reg").join("bin");
        program(&reg_bin, "gswin64c.exe");
        let e = env(vec![("ProgramFiles", pf)]);
        let r = win(
            Program::Ghostscript,
            &[bin],
            &e,
            None,
            &gs_registry(&[("10.04.0", &reg_bin)]),
        );
        assert_eq!(r, Some(found));
    }

    #[test]
    fn ac7_ac8_windows_ghostscript_newest_program_files_dir_by_number() {
        let t = tempfile::tempdir().unwrap();
        let pf = t.path().join("pf");
        let gs = pf.join("gs");
        program(&gs.join("gs9.56.1").join("bin"), "gswin64c.exe");
        let found = program(&gs.join("gs10.04.0").join("bin"), "gswin64c.exe");
        std::fs::create_dir_all(gs.join("gsx").join("bin")).unwrap();
        let reg_bin = t.path().join("reg").join("bin");
        program(&reg_bin, "gswin64c.exe");
        let e = env(vec![("ProgramFiles", pf)]);
        let r = win(
            Program::Ghostscript,
            &[t.path().join("empty")],
            &e,
            None,
            &gs_registry(&[("10.10.0", &reg_bin)]),
        );
        assert_eq!(r, Some(found));
    }

    #[test]
    fn ac7_windows_ghostscript_32_bit_name_in_program_files() {
        let t = tempfile::tempdir().unwrap();
        let pf = t.path().join("pf");
        let found = program(&pf.join("gs").join("gs9.56.1").join("bin"), "gswin32c.exe");
        let e = env(vec![("ProgramFiles", pf)]);
        let r = win(
            Program::Ghostscript,
            &[],
            &e,
            None,
            &FakeRegistry::default(),
        );
        assert_eq!(r, Some(found));
    }

    #[test]
    fn ac7_ac8_windows_ghostscript_registry_newest_version_key_by_number() {
        let t = tempfile::tempdir().unwrap();
        let old = t.path().join("gs9.56.1").join("bin");
        program(&old, "gswin64c.exe");
        let mid = t.path().join("gs10.04.0").join("bin");
        program(&mid, "gswin64c.exe");
        let new = t.path().join("gs10.10.0").join("bin");
        let found = program(&new, "gswin64c.exe");
        // "9.56.1" is the largest as text; 10.10.0 is the newest as a number.
        let reg = gs_registry(&[("10.04.0", &mid), ("9.56.1", &old), ("10.10.0", &new)]);
        let e = env(vec![("ProgramFiles", t.path().join("pf"))]);
        let r = win(
            Program::Ghostscript,
            &[t.path().join("empty")],
            &e,
            None,
            &reg,
        );
        assert_eq!(r, Some(found));
    }

    #[test]
    fn ac7_windows_ghostscript_none_when_missing_everywhere() {
        let t = tempfile::tempdir().unwrap();
        let pf = t.path().join("pf");
        std::fs::create_dir_all(pf.join("gs").join("gs10.04.0").join("bin")).unwrap();
        let e = env(vec![("ProgramFiles", pf)]);
        let reg = gs_registry(&[("10.04.0", &t.path().join("gone"))]);
        let r = win(
            Program::Ghostscript,
            &[t.path().to_path_buf()],
            &e,
            None,
            &reg,
        );
        assert_eq!(r, None);
    }

    // AC7: FFmpeg on Windows

    #[test]
    fn ac7_windows_ffmpeg_path_wins_over_exe_dir() {
        let t = tempfile::tempdir().unwrap();
        let bin = t.path().join("bin");
        let ffmpeg = program(&bin, "ffmpeg.exe");
        let ffprobe = program(&bin, "ffprobe.exe");
        let exe_dir = t.path().join("app");
        let side = exe_dir.join("ffmpeg").join("bin");
        program(&side, "ffmpeg.exe");
        program(&side, "ffprobe.exe");
        let reg = FakeRegistry::default();
        let paths = [bin];
        let r = win(Program::Ffmpeg, &paths, &no_env(), Some(&exe_dir), &reg);
        assert_eq!(r, Some(ffmpeg));
        let r = win(Program::Ffprobe, &paths, &no_env(), Some(&exe_dir), &reg);
        assert_eq!(r, Some(ffprobe));
    }

    #[test]
    fn ac7_windows_ffmpeg_and_ffprobe_in_exe_dir_ffmpeg_bin() {
        let t = tempfile::tempdir().unwrap();
        let exe_dir = t.path().join("app");
        let side = exe_dir.join("ffmpeg").join("bin");
        let ffmpeg = program(&side, "ffmpeg.exe");
        let ffprobe = program(&side, "ffprobe.exe");
        let reg = FakeRegistry::default();
        let paths = [t.path().join("empty")];
        let r = win(Program::Ffmpeg, &paths, &no_env(), Some(&exe_dir), &reg);
        assert_eq!(r, Some(ffmpeg));
        let r = win(Program::Ffprobe, &paths, &no_env(), Some(&exe_dir), &reg);
        assert_eq!(r, Some(ffprobe));
        assert_eq!(
            win(Program::Ffmpeg, &paths, &no_env(), None, &reg),
            None,
            "no exe_dir, nothing in PATH"
        );
    }

    // AC7: Unix keeps today's PATH lookup

    // Unix only: exercises the Unix PATH lookup, Windows has its own tests.

    #[cfg(unix)]
    #[test]
    fn ac7_unix_searches_path_only() {
        let t = tempfile::tempdir().unwrap();
        let a = t.path().join("a");
        let b = t.path().join("b");
        let libre = program(&a, "libreoffice");
        let soffice = program(&b, "soffice");
        let gs = program(&b, "gs");
        let ffmpeg = program(&b, "ffmpeg");
        let ffprobe = program(&b, "ffprobe");
        let paths = [a.clone(), b.clone()];
        let reg = FakeRegistry::default();
        let unix = |p: Program, dirs: &[PathBuf]| search(Os::Unix, p, dirs, &no_env(), None, &reg);
        // Name order first, as find_program does today.
        assert_eq!(unix(Program::Office, &paths), Some(soffice));
        assert_eq!(unix(Program::Office, std::slice::from_ref(&a)), Some(libre));
        assert_eq!(unix(Program::Ghostscript, &paths), Some(gs));
        assert_eq!(unix(Program::Ffmpeg, &paths), Some(ffmpeg));
        assert_eq!(unix(Program::Ffprobe, &paths), Some(ffprobe));
    }

    #[test]
    fn ac7_unix_ignores_program_files_exe_dir_and_registry() {
        let t = tempfile::tempdir().unwrap();
        let pf = t.path().join("pf");
        program(&pf.join("LibreOffice").join("program"), "soffice");
        program(&pf.join("gs").join("gs10.04.0").join("bin"), "gs");
        let exe_dir = t.path().join("app");
        program(&exe_dir.join("ffmpeg").join("bin"), "ffmpeg");
        let reg_dir = t.path().join("reg");
        program(&reg_dir, "soffice");
        program(&reg_dir, "gs");
        let e = env(vec![("ProgramFiles", pf)]);
        let empty = [t.path().join("empty")];
        assert_eq!(
            search(
                Os::Unix,
                Program::Office,
                &empty,
                &e,
                Some(&exe_dir),
                &lo_registry(&reg_dir)
            ),
            None
        );
        assert_eq!(
            search(
                Os::Unix,
                Program::Ghostscript,
                &empty,
                &e,
                Some(&exe_dir),
                &gs_registry(&[("10.04.0", &reg_dir)])
            ),
            None
        );
        assert_eq!(
            search(
                Os::Unix,
                Program::Ffmpeg,
                &empty,
                &e,
                Some(&exe_dir),
                &FakeRegistry::default()
            ),
            None
        );
    }

    // AC8: newest_gs

    #[test]
    fn ac8_newest_gs_compares_versions_by_number() {
        assert_eq!(
            newest_gs(&["gs9.56.1", "gs10.04.0", "gs10.10.0", "gsx"]),
            Some("gs10.10.0")
        );
        assert_eq!(newest_gs(&["gs9.56.1", "gs10.04.0"]), Some("gs10.04.0"));
        assert_eq!(newest_gs(&[]), None);
    }
}
