//! S-081 unit tests (backlog/S-081-windows-long-path-and-dir-candidates.md):
//! the Windows long-path form and the directory-name output candidates.
//!
//! Contract fixed by the tester, amended by the Windows correctness fixes
//! (the first contract answered the prefixed form for every absolute path
//! and panicked on a name that is not valid UTF-8):
//! - `win_long_path(path: &Path) -> std::io::Result<PathBuf>` in
//!   `output_impl.rs`, beside the pure `move_error_kind`, unit tested on
//!   every platform. It answers the form `MoveFileExW` should receive:
//!   - a path that is not valid UTF-8 answers `io::Error` with kind
//!     `InvalidFilename`, never a panic: it runs inside `place_no_replace`
//!     for every job's output placement in the daemon, so one odd name must
//!     fail that job cleanly;
//!   - a spelling that is already extended (`\\?\` or `\\.\`) is answered
//!     verbatim: `..` and `/` are literal there;
//!   - every other spelling is normalized first: `/` is a separator like
//!     `\`, `.` components drop, `..` pops the previous component and never
//!     crosses the root (`C:\`, `\\server\share`, `\`), repeated and
//!     trailing separators drop, and a plain relative spelling keeps its
//!     leading `..` components;
//!   - the `\\?\` prefix (`\\?\UNC\` for a UNC spelling) is added only when
//!     the normalized form does not fit `MAX_PATH` (260 characters
//!     including the terminating NUL): 259 characters answer without the
//!     prefix so `MoveFileExW` sees the spelling that worked before S-081,
//!     260 and more answer the extended form;
//!   - a relative spelling is resolved against the working directory before
//!     the prefix decision: `win_long_path_in(cwd, path)` takes that
//!     directory as an argument and `win_long_path` passes this process's
//!     working directory. A short relative spelling stays relative. A
//!     rooted spelling (`\a`) runs on the working directory's drive when it
//!     needs the prefix, and a drive relative spelling (`C:a`) is answered
//!     as spelled: its root is the per drive working directory, which no
//!     call can read.
//! - `place_no_replace` in `output_impl/windows.rs` sends BOTH its arguments
//!   through `win_long_path` before `wide`, so the temp file and the final
//!   name get the right form (the source check is `tests/s081_sources.rs`).
//! - `dir_collision(candidate: &Path) -> bool` in `output_impl.rs`: true only
//!   when the candidate name is occupied by a directory. The rule lives in
//!   `finalize_with`, where the candidate is inspected: when `place` fails
//!   with anything other than `AlreadyExists` and `dir_collision(&candidate)`
//!   is true, the loop continues to the next numbered candidate instead of
//!   returning the error. `move_error_kind` keeps its plain map (5 stays
//!   `PermissionDenied`, no filesystem checks).
//! - Nothing else changes: `MAX_NUMBER`, the never replace rule and the
//!   `AlreadyExists` continuation stay as they are (S-079 exFAT).

#[cfg(windows)]
use super::place_no_replace;
use super::{
    dir_collision, finalize_with, move_error_kind, temp_path, win_long_path, win_long_path_in,
};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// The answer for `path` when the working directory is `cwd`.
fn asked(cwd: &str, path: &str) -> PathBuf {
    win_long_path_in(Path::new(cwd), Path::new(path)).unwrap()
}

/// A spelling of exactly `units` UTF-16 units that starts with `prefix`.
fn pad(prefix: &str, units: usize) -> String {
    format!(
        "{prefix}{}",
        "x".repeat(units - prefix.encode_utf16().count())
    )
}

#[test]
fn ac1_only_a_form_that_does_not_fit_max_path_gets_the_prefix() {
    // MAX_PATH (260) includes the terminating NUL: 259 characters fit and
    // stay the spelling that worked before, 260 do not and get `\\?\`.
    for units in [3usize, 32, 258, 259] {
        let p = pad("C:\\", units);
        assert_eq!(asked("C:\\work", &p), PathBuf::from(&p), "{units}");
    }
    for units in [260, 261, 400] {
        let p = pad("C:\\", units);
        assert_eq!(
            asked("C:\\work", &p),
            PathBuf::from(format!(r"\\?\{p}")),
            "{units}"
        );
    }
}

#[test]
fn ac1_long_drive_and_unc_forms_get_the_right_extended_form() {
    let p = pad("D:\\", 300);
    assert_eq!(
        asked("C:\\work", &p),
        PathBuf::from(format!(r"\\?\{p}")),
        "the drive letter is not special"
    );
    // A UNC spelling: `\\?\UNC\server\share\...`.
    let u = pad(r"\\server\share\", 300);
    let want = format!(r"\\?\UNC\{}", &u[2..]);
    assert_eq!(asked("C:\\work", &u), PathBuf::from(want));
}

#[test]
fn ac1_forward_slash_spellings_normalize_and_get_the_prefix() {
    // `C:/photos/a.png` worked before S-081 and must keep working: it is
    // normalized before any prefix decision, and a long form is fixed.
    let p = format!("C:/photos/{}", "x".repeat(300));
    let want = format!(r"\\?\C:\photos\{}", "x".repeat(300));
    assert_eq!(asked("C:\\work", &p), PathBuf::from(want));
    let u = format!("//s/sh/{}", "x".repeat(300));
    let want = format!(r"\\?\UNC\s\sh\{}", "x".repeat(300));
    assert_eq!(asked("C:\\work", &u), PathBuf::from(want));
}

#[test]
fn ac1_separators_and_dot_components_normalize_before_any_prefix() {
    let table = [
        ("C:/photos/./a/../b.png", r"C:\photos\b.png"),
        (r"C:\a\..\..\b.png", r"C:\b.png"),
        ("//server/share/a/./..\\b.webp", r"\\server\share\b.webp"),
        (r"\\server\share\a\..\..\..\b", r"\\server\share\b"),
        ("out/./x/../a.webp", r"out\a.webp"),
        (r"..\x\a.webp", r"..\x\a.webp"),
        (r"C:\a\\b\\", r"C:\a\b"),
        (r"\a\..\..\b", r"\b"),
        (r"C:foo\..\x", "C:x"),
        (r"C:foo\..\..\x", r"C:..\x"),
    ];
    for (input, want) in table {
        assert_eq!(asked("C:\\work", input), PathBuf::from(want), "{input}");
    }
}

#[test]
fn ac1_extended_spellings_stay_verbatim() {
    for p in [
        r"\\?\C:\Users\u\a.webp",
        r"\\?\UNC\server\share\a.webp",
        r"\\.\C:\a.webp",
        r"\\?\C:\a\..\b.webp",
    ] {
        assert_eq!(asked("C:\\work", p), PathBuf::from(p), "{p}");
    }
}

#[test]
fn ac1_a_relative_spelling_resolves_against_the_working_directory() {
    let cwd = "C:\\Users\\u";
    // Short: answered relative, normalized, exactly as spelled.
    assert_eq!(asked(cwd, "out/./a.webp"), PathBuf::from(r"out\a.webp"));
    // Long only after the join: the extended form of the resolved path.
    let rel = format!("out/{}", "x".repeat(250));
    let want = format!(r"\\?\C:\Users\u\out\{}", "x".repeat(250));
    assert_eq!(asked(cwd, &rel), PathBuf::from(want));
    // Short by itself but long in a deep working directory.
    let deep = pad("C:\\", 400);
    let want = format!(r"\\?\{deep}\a.webp");
    assert_eq!(asked(&deep, "a.webp"), PathBuf::from(want));
}

#[test]
fn ac1_a_rooted_spelling_runs_on_the_working_directory_drive() {
    // Short: answered as spelled.
    assert_eq!(
        asked("C:\\work", "/photos/./a.png"),
        PathBuf::from(r"\photos\a.png")
    );
    // Long: the drive of the working directory, extended.
    let rooted = format!(r"\{}", "x".repeat(300));
    let want = format!(r"\\?\C:{rooted}");
    assert_eq!(asked("C:\\work", &rooted), PathBuf::from(want));
}

#[test]
fn ac1_win_long_path_passes_the_process_working_directory() {
    assert_eq!(
        win_long_path(Path::new("out/./a.webp")).unwrap(),
        PathBuf::from(r"out\a.webp")
    );
}

/// A path spelling the platform accepts but UTF-8 cannot hold: a stray byte
/// on Unix, a lone UTF-16 surrogate on Windows.
#[cfg(unix)]
fn non_utf8() -> PathBuf {
    use std::os::unix::ffi::OsStringExt;
    PathBuf::from(std::ffi::OsString::from_vec(
        b"out\\name-\xff.webp".to_vec(),
    ))
}

#[cfg(windows)]
fn non_utf8() -> PathBuf {
    use std::os::windows::ffi::OsStringExt;
    let wide: Vec<u16> = "out\\name-"
        .encode_utf16()
        .chain([0xD800])
        .chain(".webp".encode_utf16())
        .collect();
    PathBuf::from(std::ffi::OsString::from_wide(&wide))
}

#[cfg(any(unix, windows))]
#[test]
fn ac1_a_path_that_is_not_valid_utf8_is_an_error_not_a_panic() {
    for cwd in ["C:\\work", ""] {
        let err = win_long_path_in(Path::new(cwd), &non_utf8()).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidFilename, "cwd {cwd:?}");
    }
}

#[test]
fn ac1_win_long_path_prefixes_the_temp_file_and_the_final_name() {
    // The two shapes `place_no_replace` receives in a deep folder: the temp
    // file and the final name beside it.
    let dir = format!("C:\\{}", "s081-segment\\".repeat(24));
    for name in ["a.webp", ".kawaro-1-2-3-a.webp"] {
        let p = format!("{dir}{name}");
        assert_eq!(
            asked("C:\\work", &p),
            PathBuf::from(format!(r"\\?\{p}")),
            "{name}"
        );
    }
}

/// Refuses every name, like a filesystem without hard links (exFAT), so
/// `finalize_with` falls back to `place` for every candidate.
fn unsupported(_: &Path, _: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(ErrorKind::Unsupported, "no links"))
}

/// Mimics `MoveFileExW` with flags 0: an existing file name answers
/// `AlreadyExists` (ERROR_FILE_EXISTS 80, ERROR_ALREADY_EXISTS 183), an
/// existing directory name answers `PermissionDenied` (ERROR_ACCESS_DENIED
/// 5), a free name is renamed.
fn move_file_ex(from: &Path, to: &Path) -> std::io::Result<()> {
    match fs::symlink_metadata(to) {
        Ok(m) if m.is_dir() => Err(std::io::Error::new(
            ErrorKind::PermissionDenied,
            "access denied",
        )),
        Ok(_) => Err(std::io::Error::new(ErrorKind::AlreadyExists, "exists")),
        Err(_) => fs::rename(from, to),
    }
}

// The fakes capture nothing, so they fit a `'static` `PlaceFn` alias like
// `LinkFn`; what they were asked is recorded per test thread.
std::thread_local! {
    static ASKED: std::cell::RefCell<Vec<PathBuf>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn asked_names() -> Vec<PathBuf> {
    ASKED.with(|a| a.borrow().clone())
}

/// A genuine permission failure with the original message, for any name.
fn denied(_: &Path, to: &Path) -> std::io::Result<()> {
    ASKED.with(|a| a.borrow_mut().push(to.to_path_buf()));
    Err(std::io::Error::new(
        ErrorKind::PermissionDenied,
        "denied by policy",
    ))
}

#[test]
fn ac2_a_directory_candidate_is_name_taken_and_the_next_free_name_is_used() {
    let d = tempfile::tempdir().unwrap();
    let fin = d.path().join("a.webp");
    fs::create_dir(&fin).unwrap();
    fs::write(fin.join("marker"), b"keep").unwrap();
    let one = d.path().join("a (1).webp");
    fs::create_dir(&one).unwrap();
    fs::write(one.join("marker"), b"keep").unwrap();
    let tmp = temp_path(&fin);
    fs::write(&tmp, b"new").unwrap();
    let used = finalize_with(&tmp, &fin, &unsupported, &move_file_ex).unwrap();
    assert_eq!(used, d.path().join("a (2).webp"));
    assert_eq!(fs::read(&used).unwrap(), b"new");
    assert!(!tmp.exists(), "the temp must be gone");
    for dir in [&fin, &one] {
        assert!(dir.is_dir(), "{dir:?} must stay a directory");
        assert_eq!(
            fs::read(dir.join("marker")).unwrap(),
            b"keep",
            "{dir:?} must stay untouched"
        );
    }
}

#[test]
fn ac3_a_file_collision_on_the_place_path_still_continues() {
    let d = tempfile::tempdir().unwrap();
    let fin = d.path().join("a.webp");
    fs::write(&fin, b"zero").unwrap();
    let one = d.path().join("a (1).webp");
    fs::write(&one, b"one").unwrap();
    let tmp = temp_path(&fin);
    fs::write(&tmp, b"new").unwrap();
    let used = finalize_with(&tmp, &fin, &unsupported, &move_file_ex).unwrap();
    assert_eq!(used, d.path().join("a (2).webp"));
    assert_eq!(fs::read(&used).unwrap(), b"new");
    assert_eq!(fs::read(&fin).unwrap(), b"zero");
    assert_eq!(fs::read(&one).unwrap(), b"one");
    assert!(!tmp.exists(), "the temp must be gone");
}

#[test]
fn ac3_a_genuine_permission_failure_keeps_its_original_error_and_the_temp() {
    let d = tempfile::tempdir().unwrap();
    let fin = d.path().join("a.webp");
    let tmp = temp_path(&fin);
    fs::write(&tmp, b"new").unwrap();
    let e = finalize_with(&tmp, &fin, &unsupported, &denied).unwrap_err();
    assert_eq!(e.kind(), ErrorKind::PermissionDenied);
    assert!(
        e.to_string().contains("denied by policy"),
        "the job fails with its original error: {e}"
    );
    assert_eq!(
        asked_names(),
        vec![fin.clone()],
        "no retry on another name after a real error"
    );
    assert_eq!(fs::read(&tmp).unwrap(), b"new", "the temp must stay");
    assert!(!fin.exists());
    assert!(!d.path().join("a (1).webp").exists());
}

#[test]
fn ac4_move_error_kind_keeps_its_plain_map() {
    let table = [
        (5, ErrorKind::PermissionDenied), // ERROR_ACCESS_DENIED
        (80, ErrorKind::AlreadyExists),   // ERROR_FILE_EXISTS
        (183, ErrorKind::AlreadyExists),  // ERROR_ALREADY_EXISTS
        (2, ErrorKind::Other),            // ERROR_FILE_NOT_FOUND: unmapped
    ];
    for (code, want) in table {
        assert_eq!(move_error_kind(code), want, "code {code}");
    }
}

#[test]
fn ac4_dir_collision_is_true_only_for_a_directory_name() {
    let d = tempfile::tempdir().unwrap();
    let dir = d.path().join("a.webp");
    fs::create_dir(&dir).unwrap();
    assert!(dir_collision(&dir), "a directory name is the collision");
    let file = d.path().join("b.webp");
    fs::write(&file, b"x").unwrap();
    assert!(!dir_collision(&file), "a file collision is not this rule");
    assert!(
        !dir_collision(&d.path().join("missing.webp")),
        "a free name is not a collision"
    );
}

/// AC1 live proof on Windows: the move places a temp file and a final name
/// over 260 characters and the contents match what the conversion wrote.
#[cfg(windows)]
#[test]
fn ac1_windows_move_places_a_300_character_output_path() {
    let d = tempfile::tempdir().unwrap();
    let mut dir = d.path().to_path_buf();
    for i in 0..8 {
        dir = dir.join(format!("s081-long-segment-{i:02}-padding-padding-padding"));
    }
    fs::create_dir_all(&dir).unwrap();
    let fin = dir.join("a.webp");
    let tmp = temp_path(&fin);
    let final_len = fin.to_string_lossy().len();
    assert!(
        final_len >= 300,
        "the final path is only {final_len} characters"
    );
    assert!(
        tmp.to_string_lossy().len() > 260,
        "the temp path must also pass MAX_PATH"
    );
    fs::write(&tmp, b"converted").unwrap();
    place_no_replace(&tmp, &fin).unwrap();
    assert_eq!(fs::read(&fin).unwrap(), b"converted");
    assert!(!tmp.exists());
}
