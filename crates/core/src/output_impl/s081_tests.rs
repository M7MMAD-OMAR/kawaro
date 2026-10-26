//! S-081 unit tests (backlog/S-081-windows-long-path-and-dir-candidates.md):
//! the Windows long-path form and the directory-name output candidates.
//!
//! Contract fixed by the tester:
//! - `win_long_path(path: &Path) -> PathBuf` in `output_impl.rs`, beside the
//!   pure `move_error_kind`, unit tested on every platform. It answers the
//!   `\\?\` extended-length form: a drive path `C:\x` becomes `\\?\C:\x`, a
//!   UNC path `\\server\share\x` becomes `\\?\UNC\server\share\x`, and a path
//!   already extended (`\\?\` or `\\.\`) or not absolute (no drive, no UNC)
//!   stays unchanged. `place_no_replace` in `output_impl/windows.rs` sends
//!   BOTH its arguments through it before `wide`, so the temp file and the
//!   final name use the long form.
//! - `dir_collision(candidate: &Path) -> bool` in `output_impl.rs`: true only
//!   when the candidate name is occupied by a directory. The rule lives in
//!   `finalize_with`, where the candidate is inspected: when `place` fails
//!   with anything other than `AlreadyExists` and `dir_collision(&candidate)`
//!   is true, the loop continues to the next numbered candidate instead of
//!   returning the error. `move_error_kind` keeps its plain map (5 stays
//!   `PermissionDenied`, no filesystem checks).
//! - Nothing else changes: `MAX_NUMBER`, the never replace rule and the
//!   `AlreadyExists` continuation stay as they are (S-079 exFAT).

use super::{dir_collision, finalize_with, move_error_kind, temp_path, win_long_path};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

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

fn asked() -> Vec<PathBuf> {
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
fn ac1_win_long_path_adds_the_extended_length_prefix() {
    assert_eq!(
        win_long_path(Path::new("C:\\Users\\u\\a.webp")),
        PathBuf::from("\\\\?\\C:\\Users\\u\\a.webp")
    );
    assert_eq!(
        win_long_path(Path::new("\\\\server\\share\\a.webp")),
        PathBuf::from("\\\\?\\UNC\\server\\share\\a.webp")
    );
}

#[test]
fn ac1_win_long_path_leaves_extended_and_relative_paths_unchanged() {
    for p in [
        "\\\\?\\C:\\Users\\u\\a.webp",
        "\\\\?\\UNC\\server\\share\\a.webp",
        "\\\\.\\C:\\a.webp",
        "a.webp",
        "out\\a.webp",
    ] {
        assert_eq!(win_long_path(Path::new(p)), PathBuf::from(p), "{p}");
    }
}

#[test]
fn ac1_win_long_path_prefixes_the_temp_file_and_the_final_name() {
    // The two shapes `place_no_replace` receives: the temp file and the
    // final name beside it.
    for p in [
        "C:\\Users\\u\\Pictures\\a.webp",
        "C:\\Users\\u\\Pictures\\.kawaro-1-2-3-a.webp",
    ] {
        assert_eq!(
            win_long_path(Path::new(p)),
            PathBuf::from(format!("\\\\?\\{p}")),
            "{p}"
        );
    }
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
        asked(),
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
