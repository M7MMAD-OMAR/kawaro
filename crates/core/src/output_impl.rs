//! Owned by the core track; see `output.rs` for the contract.
use crate::action::OutputSpec;
use std::ffi::OsString;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Upper bound on numbered names tried before giving up.
const MAX_NUMBER: u32 = 100_000;

/// A file name split as `<base><number?>.<ext?>`, where `number` is the
/// value of a trailing ` (n)` on the stem, if any.
struct Name {
    dir: PathBuf,
    base: OsString,
    ext: Option<OsString>,
}

impl Name {
    fn with_number(&self, n: u32) -> PathBuf {
        let mut name = self.base.clone();
        if n > 0 {
            name.push(format!(" ({n})"));
        }
        if let Some(ext) = &self.ext {
            name.push(".");
            name.push(ext);
        }
        self.dir.join(name)
    }
}

fn exists(p: &Path) -> bool {
    // symlink_metadata: a dangling link still occupies the name.
    std::fs::symlink_metadata(p).is_ok()
}

/// `final_path` with an extra test for names already handed out, used by
/// the queue planner so one job never plans the same name twice.
pub(crate) fn final_path_with(
    input: &Path,
    spec: &OutputSpec,
    out_dir: &Path,
    taken: &dyn Fn(&Path) -> bool,
) -> PathBuf {
    let mut base = input.file_stem().map(OsString::from).unwrap_or_default();
    base.push(&spec.suffix);
    let ext = if spec.ext.is_empty() {
        input.extension().map(OsString::from)
    } else {
        Some(OsString::from(&spec.ext))
    };
    let name = Name {
        dir: out_dir.to_path_buf(),
        base,
        ext,
    };
    (0..MAX_NUMBER)
        .map(|n| name.with_number(n))
        .find(|c| c != input && !exists(c) && !taken(c))
        .unwrap_or_else(|| name.with_number(MAX_NUMBER))
}

pub(crate) fn final_path(input: &Path, spec: &OutputSpec, out_dir: &Path) -> PathBuf {
    final_path_with(input, spec, out_dir, &|_| false)
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub(crate) fn temp_path(final_path: &Path) -> PathBuf {
    let dir = final_path.parent().unwrap_or(Path::new(""));
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    // `.kawaro-<pid>-<count>-<nanos>-<final name>`: hidden, and the final
    // name stays last so the extension (or its absence) is unchanged.
    let mut name = OsString::from(format!(".kawaro-{}-{count}-{nanos}-", std::process::id()));
    name.push(final_path.file_name().unwrap_or_default());
    dir.join(name)
}

/// Splits `path` into base and starting number: `a (3).png` gives base
/// `a` and number 3, so the next candidate is `a (4).png`.
fn numbered(path: &Path) -> (Name, u32) {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let parsed = stem
        .strip_suffix(')')
        .and_then(|s| s.rsplit_once(" ("))
        // Digits only: `parse` alone would also accept a leading `+`.
        .filter(|(_, n)| n.bytes().all(|c| c.is_ascii_digit()))
        .and_then(|(b, n)| n.parse::<u32>().ok().map(|n| (b, n)));
    let (base, start) = match parsed {
        Some((b, n)) => (OsString::from(b), n),
        None => (path.file_stem().map(OsString::from).unwrap_or_default(), 0),
    };
    let name = Name {
        dir: path.parent().map(Path::to_path_buf).unwrap_or_default(),
        base,
        ext: path.extension().map(OsString::from),
    };
    (name, start)
}

type LinkFn = dyn Fn(&Path, &Path) -> std::io::Result<()>;
/// Moves a file to a name and fails with `AlreadyExists` when the name is
/// taken, never replacing it.
type PlaceFn = dyn Fn(&Path, &Path) -> std::io::Result<()>;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows::place_no_replace;

/// The place function off Windows: an existence check, then `rename`. A
/// file created between the two would be replaced; Windows has no such
/// race (`output_impl/windows.rs`).
#[cfg(not(windows))]
fn place_no_replace(from: &Path, to: &Path) -> std::io::Result<()> {
    if exists(to) {
        return Err(std::io::Error::new(
            ErrorKind::AlreadyExists,
            "output name taken",
        ));
    }
    std::fs::rename(from, to)
}

pub fn dir_collision(candidate: &Path) -> bool {
    match std::fs::metadata(candidate) {
        Ok(metadata) => metadata.is_dir(),
        Err(_) => false,
    }
}

/// Windows long path helper (S-081, amended by the Windows correctness
/// fixes): the form `MoveFileExW` should receive for `path`, resolved
/// against `cwd` ([`win_long_path`] passes this process's working
/// directory). This is exposed for testing on all platforms; the rules and
/// the boundary cases are pinned in `s081_tests`.
///
/// A path that is not valid UTF-8 answers `InvalidFilename` instead of
/// panicking: this runs inside `place_no_replace` for every job's output
/// placement in the daemon, so one odd name must fail that job cleanly.
///
/// An extended spelling (`\\?\` or `\\.\`) is answered verbatim: `..` and
/// `/` are literal there. Every other spelling is normalized first (`/` is
/// a separator like `\`, `.` components drop, `..` pops the previous
/// component and never crosses the root, repeated and trailing separators
/// drop), and the `\\?\` prefix (`\\?\UNC\` for a UNC spelling) is added
/// only when the normalized form does not fit `MAX_PATH` (260 characters
/// including the terminating NUL), so a short spelling keeps working
/// exactly as it did before S-081.
///
/// A relative or rooted spelling is resolved against `cwd` before that
/// decision and stays as spelled when it fits; a working directory that
/// cannot be read leaves it as spelled. A drive relative spelling (`C:a`)
/// is always answered as spelled: its root is the per drive working
/// directory, which no call can read.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn win_long_path_in(cwd: &Path, path: &Path) -> std::io::Result<PathBuf> {
    let text = path.to_str().ok_or_else(|| {
        std::io::Error::new(
            ErrorKind::InvalidFilename,
            "the output path is not valid UTF-8",
        )
    })?;
    let (norm, root) = normalize(text);
    let resolved = match root {
        Root::Extended | Root::Drive | Root::Unc | Root::DriveRelative => {
            Some((norm.clone(), root))
        }
        Root::Rooted => drive_of(cwd).map(|d| normalize(&format!("{d}{norm}"))),
        Root::Relative => cwd
            .to_str()
            .map(|c| normalize(&format!("{c}\\{norm}")))
            .filter(|(_, r)| matches!(r, Root::Extended | Root::Drive | Root::Unc)),
    };
    match resolved {
        Some((candidate, Root::Extended)) => Ok(PathBuf::from(candidate)),
        Some((candidate, Root::Drive | Root::Unc)) if utf16_len(&candidate) >= MAX_PATH => {
            Ok(PathBuf::from(long_form(&candidate)))
        }
        _ => Ok(PathBuf::from(norm)),
    }
}

/// [`win_long_path_in`] with this process's working directory.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn win_long_path(path: &Path) -> std::io::Result<PathBuf> {
    win_long_path_in(&std::env::current_dir().unwrap_or_default(), path)
}

/// `MAX_PATH` (260) includes the terminating NUL, so a normalized form of
/// 260 characters or more does not fit and needs the extended-length form.
const MAX_PATH: usize = 260;

/// The root of a normalized Windows spelling: what `..` cannot cross and
/// the shape the extended prefix attaches to.
#[derive(Clone, Copy)]
enum Root {
    /// `\\?\...` or `\\.\...`: already extended, answered verbatim.
    Extended,
    /// `C:\`
    Drive,
    /// `\\server\share`
    Unc,
    /// `\`, rooted on the working directory's drive.
    Rooted,
    /// `C:`, relative to drive C's own working directory.
    DriveRelative,
    /// No root, relative to the working directory.
    Relative,
}

/// The canonical spelling of `text` and its root: `/` is a separator like
/// `\`, `.` components drop, `..` pops the previous component and never
/// crosses the root, repeated and trailing separators drop, and a plain
/// relative spelling keeps its leading `..`. An extended spelling is
/// answered as it stands.
fn normalize(text: &str) -> (String, Root) {
    let bytes = text.as_bytes();
    if text.starts_with(r"\\?\") || text.starts_with(r"\\.\") {
        return (text.to_string(), Root::Extended);
    }
    // Drive spellings: `C:\...` rooted, `C:...` relative to drive C.
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        let rooted = bytes.len() >= 3 && is_sep(bytes[2] as char);
        let (head, tail) = if rooted {
            (format!("{}:\\", &text[..1]), &text[3..])
        } else {
            (text[..2].to_string(), &text[2..])
        };
        let root = if rooted {
            Root::Drive
        } else {
            Root::DriveRelative
        };
        let comps = resolve(segs(tail), root);
        return (build(&head, &comps), root);
    }
    // UNC spellings: `\\server\share`, also `//server/share`.
    if bytes.len() >= 2 && is_sep(bytes[0] as char) && is_sep(bytes[1] as char) {
        let mut parts: Vec<&str> = segs(&text[2..]);
        if parts.is_empty() {
            return (text.to_string(), Root::Unc);
        }
        let server = parts.remove(0);
        let share = if parts.is_empty() {
            ""
        } else {
            parts.remove(0)
        };
        let head = if share.is_empty() {
            format!("\\\\{server}")
        } else {
            format!("\\\\{server}\\{share}")
        };
        let comps = resolve(parts, Root::Unc);
        return (build(&head, &comps), Root::Unc);
    }
    // Rooted on the working directory's drive: `\...`.
    if bytes.first().is_some_and(|b| is_sep(*b as char)) {
        let comps = resolve(segs(&text[1..]), Root::Rooted);
        return (build("\\", &comps), Root::Rooted);
    }
    let comps = resolve(segs(text), Root::Relative);
    (build("", &comps), Root::Relative)
}

/// True for the separators Windows accepts outside an extended spelling.
fn is_sep(c: char) -> bool {
    c == '/' || c == '\\'
}

/// The non empty components of a path spelling.
fn segs(s: &str) -> Vec<&str> {
    s.split(is_sep).filter(|p| !p.is_empty()).collect()
}

/// Resolves `.` and `..` in the components after a root. `..` pops the
/// previous component and never crosses a root (`C:\`, `\\server\share`,
/// `\`); a plain relative or drive relative spelling keeps its leading
/// `..`, since its root is a working directory the caller cannot see.
fn resolve<'a>(segs: impl IntoIterator<Item = &'a str>, root: Root) -> Vec<&'a str> {
    let mut comps: Vec<&str> = Vec::new();
    for comp in segs {
        if comp == "." {
            continue;
        }
        if comp == ".." {
            match comps.last().copied() {
                Some(prev) if prev != ".." => {
                    comps.pop();
                }
                _ if matches!(root, Root::Relative | Root::DriveRelative) => comps.push(comp),
                _ => {}
            }
            continue;
        }
        comps.push(comp);
    }
    comps
}

/// Joins a root and its resolved components with `\`; `C:\` and `\` end
/// with their own separator, `C:` and the empty root glue to the first
/// component.
fn build(root: &str, comps: &[&str]) -> String {
    let mut out = String::from(root);
    for (i, comp) in comps.iter().enumerate() {
        let glue = !(i == 0 && (out.is_empty() || out.ends_with(':')));
        if glue && !out.ends_with('\\') {
            out.push('\\');
        }
        out.push_str(comp);
    }
    out
}

/// The drive of `cwd` as `C:`, what a rooted spelling (`\a`) runs on.
fn drive_of(cwd: &Path) -> Option<String> {
    let (norm, root) = normalize(cwd.to_str()?);
    let bytes = norm.as_bytes();
    match root {
        Root::Drive | Root::DriveRelative if bytes.len() >= 2 => Some(norm[..2].to_string()),
        // `\\?\C:\...`: the drive sits after the extended prefix.
        Root::Extended
            if bytes.len() >= 6 && bytes[4].is_ascii_alphabetic() && bytes[5] == b':' =>
        {
            Some(norm[4..6].to_string())
        }
        _ => None,
    }
}

/// The extended-length spelling of a normalized drive or UNC path.
fn long_form(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("\\\\") {
        format!(r"\\?\UNC\{rest}")
    } else {
        format!(r"\\?\{path}")
    }
}

/// `text` in UTF-16 code units, what `MAX_PATH` counts.
fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// The kind of a Win32 error code from `MoveFileExW`: `ERROR_FILE_EXISTS`
/// (80) and `ERROR_ALREADY_EXISTS` (183) mean the name is taken,
/// `ERROR_ACCESS_DENIED` (5) is a refusal, anything else is `Other`.
#[cfg_attr(not(windows), allow(dead_code))]
fn move_error_kind(code: u32) -> ErrorKind {
    match code {
        80 | 183 => ErrorKind::AlreadyExists,
        5 => ErrorKind::PermissionDenied,
        _ => ErrorKind::Other,
    }
}

pub(crate) fn finalize(temp: &Path, final_path: &Path) -> std::io::Result<PathBuf> {
    finalize_with(
        temp,
        final_path,
        &|a, b| std::fs::hard_link(a, b),
        &place_no_replace,
    )
}

/// Tries `final_path`, then the following numbers. `hard_link` fails when
/// the target exists, so it never replaces a file. When the filesystem
/// cannot hard link (vfat, exFAT, some network mounts), falls back to
/// `place`, which also refuses a taken name: on Windows atomically
/// (`MoveFileExW` without replacing), elsewhere with a check then `rename`.
fn finalize_with(
    temp: &Path,
    final_path: &Path,
    link: &LinkFn,
    place: &PlaceFn,
) -> std::io::Result<PathBuf> {
    let (name, start) = numbered(final_path);
    let first = std::iter::once(final_path.to_path_buf());
    let rest =
        (start.saturating_add(1)..=start.saturating_add(MAX_NUMBER)).map(|n| name.with_number(n));
    for candidate in first.chain(rest) {
        match link(temp, &candidate) {
            Ok(()) => {
                std::fs::remove_file(temp)?;
                return Ok(candidate);
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
            Err(e) => {
                if !exists(temp) {
                    return Err(e);
                }
                // Skip directory collisions (ERROR_ACCESS_DENIED on Windows)
                if dir_collision(&candidate) {
                    continue;
                }
                match place(temp, &candidate) {
                    Ok(()) => return Ok(candidate),
                    Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
                    Err(e) => return Err(e),
                }
            }
        }
    }
    Err(std::io::Error::new(
        ErrorKind::AlreadyExists,
        "no free output name",
    ))
}

#[cfg(test)]
mod s081_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn unsupported(_: &Path, _: &Path) -> std::io::Result<()> {
        Err(std::io::Error::new(ErrorKind::Unsupported, "no links"))
    }

    #[test]
    fn ac9_fallback_rename_when_hard_links_are_unsupported() {
        let d = tempfile::tempdir().unwrap();
        let fin = d.path().join("a.webp");
        fs::write(&fin, b"old").unwrap();
        let tmp = temp_path(&fin);
        fs::write(&tmp, b"new").unwrap();
        let used = finalize_with(&tmp, &fin, &unsupported, &place_no_replace).unwrap();
        assert_eq!(used, d.path().join("a (1).webp"));
        assert_eq!(fs::read(&fin).unwrap(), b"old");
        assert_eq!(fs::read(&used).unwrap(), b"new");
        assert!(!tmp.exists());
    }

    #[test]
    fn ac9_fallback_with_missing_temp_reports_the_link_error() {
        let d = tempfile::tempdir().unwrap();
        let fin = d.path().join("a.webp");
        let tmp = temp_path(&fin);
        let e = finalize_with(&tmp, &fin, &unsupported, &place_no_replace).unwrap_err();
        assert_eq!(e.kind(), ErrorKind::Unsupported);
        assert!(!fin.exists());
    }

    #[test]
    fn ac9_gives_up_after_the_number_limit() {
        let d = tempfile::tempdir().unwrap();
        let fin = d.path().join("a.webp");
        let tmp = temp_path(&fin);
        fs::write(&tmp, b"new").unwrap();
        let always_taken = |_: &Path, _: &Path| -> std::io::Result<()> {
            Err(std::io::Error::new(ErrorKind::AlreadyExists, "taken"))
        };
        let e = finalize_with(&tmp, &fin, &always_taken, &place_no_replace).unwrap_err();
        assert_eq!(e.kind(), ErrorKind::AlreadyExists);
        assert!(tmp.exists());
    }

    #[test]
    fn ac9_numbered_parsing() {
        let (n, s) = numbered(Path::new("/d/a (12).png"));
        assert_eq!((n.with_number(13), s), (PathBuf::from("/d/a (13).png"), 12));
        for odd in [
            "/d/a ().png",
            "/d/a (x).png",
            "/d/a(2).png",
            "/d/a (2)b.png",
            "/d/a (+2).png",
            "/d/a (99999999999).png",
        ] {
            let (n, s) = numbered(Path::new(odd));
            assert_eq!(s, 0, "{odd}");
            assert_eq!(n.with_number(0), PathBuf::from(odd));
        }
    }

    #[test]
    fn ac7_taken_names_are_skipped() {
        let d = tempfile::tempdir().unwrap();
        let spec = OutputSpec {
            mode: crate::action::OutputMode::PerFile,
            ext: "webp".into(),
            suffix: String::new(),
            dir: String::new(),
        };
        let first = d.path().join("a.webp");
        let got = final_path_with(&d.path().join("a.png"), &spec, d.path(), &|p| p == first);
        assert_eq!(got, d.path().join("a (1).webp"));
    }

    // S-041 AC6 and AC7 (backlog/S-041-windows-paths.md). Contract fixed by
    // the tester:
    // - `finalize_with(temp, final_path, link: &LinkFn, place: &PlaceFn)`,
    //   where `PlaceFn` is `dyn Fn(&Path, &Path) -> std::io::Result<()>`:
    //   moves `from` to `to` and fails with `AlreadyExists` when `to` is
    //   taken, never replacing it. `finalize_with` itself no longer checks
    //   for existence nor calls `rename` when the link fails: it calls
    //   `place`. When the temp is gone, the link error is returned (ac9).
    // - `place_no_replace(from, to) -> std::io::Result<()>`, visible here:
    //   on Linux today's check then `rename`; on Windows `MoveFileExW`
    //   from `output_impl/windows.rs`.
    // - `move_error_kind(code: u32) -> ErrorKind`: the pure map of Win32
    //   codes used by `output_impl/windows.rs`.

    // The fakes capture nothing, so they fit a `'static` `PlaceFn` alias
    // like `LinkFn`; what they were asked is recorded per test thread.
    std::thread_local! {
        static ASKED: std::cell::RefCell<Vec<PathBuf>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    fn asked() -> Vec<PathBuf> {
        ASKED.with(|a| a.borrow().clone())
    }

    /// Refuses `a.webp` and `a (1).webp` and really renames otherwise,
    /// recording every name it was asked for.
    fn refusing(from: &Path, to: &Path) -> std::io::Result<()> {
        ASKED.with(|a| a.borrow_mut().push(to.to_path_buf()));
        let name = to.file_name().unwrap_or_default();
        if name == "a.webp" || name == "a (1).webp" {
            return Err(std::io::Error::new(ErrorKind::AlreadyExists, "taken"));
        }
        fs::rename(from, to)
    }

    fn denied(_: &Path, to: &Path) -> std::io::Result<()> {
        ASKED.with(|a| a.borrow_mut().push(to.to_path_buf()));
        Err(std::io::Error::new(ErrorKind::PermissionDenied, "denied"))
    }

    fn never(_: &Path, _: &Path) -> std::io::Result<()> {
        panic!("place must not run when the link succeeds")
    }

    #[test]
    fn ac6_place_already_exists_moves_on_to_the_next_number() {
        let d = tempfile::tempdir().unwrap();
        let fin = d.path().join("a.webp");
        let one = d.path().join("a (1).webp");
        fs::write(&fin, b"zero").unwrap();
        fs::write(&one, b"one").unwrap();
        let tmp = temp_path(&fin);
        fs::write(&tmp, b"new").unwrap();
        let used = finalize_with(&tmp, &fin, &unsupported, &refusing).unwrap();
        let two = d.path().join("a (2).webp");
        assert_eq!(used, two);
        assert_eq!(asked(), vec![fin.clone(), one.clone(), two.clone()]);
        assert_eq!(fs::read(&fin).unwrap(), b"zero");
        assert_eq!(fs::read(&one).unwrap(), b"one");
        assert_eq!(fs::read(&two).unwrap(), b"new");
        assert!(!tmp.exists(), "the temp must be gone");
    }

    #[test]
    fn ac6_finalize_trusts_the_place_function_not_its_own_check() {
        // Nothing on disk: only the injected function says the two names
        // are taken, so a check then rename left inside finalize_with
        // would answer `a.webp`.
        let d = tempfile::tempdir().unwrap();
        let fin = d.path().join("a.webp");
        let one = d.path().join("a (1).webp");
        let tmp = temp_path(&fin);
        fs::write(&tmp, b"new").unwrap();
        let used = finalize_with(&tmp, &fin, &unsupported, &refusing).unwrap();
        assert_eq!(used, d.path().join("a (2).webp"));
        assert_eq!(asked(), vec![fin.clone(), one.clone(), used.clone()]);
        assert!(!fin.exists() && !one.exists());
        assert_eq!(fs::read(&used).unwrap(), b"new");
        assert!(!tmp.exists());
    }

    #[test]
    fn ac6_place_permission_denied_is_returned_and_keeps_the_temp() {
        let d = tempfile::tempdir().unwrap();
        let fin = d.path().join("a.webp");
        let tmp = temp_path(&fin);
        fs::write(&tmp, b"new").unwrap();
        let e = finalize_with(&tmp, &fin, &unsupported, &denied).unwrap_err();
        assert_eq!(e.kind(), ErrorKind::PermissionDenied);
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
    fn ac6_a_successful_link_does_not_call_place() {
        let d = tempfile::tempdir().unwrap();
        let fin = d.path().join("a.webp");
        let tmp = temp_path(&fin);
        fs::write(&tmp, b"new").unwrap();
        let used = finalize_with(&tmp, &fin, &|a, b| fs::hard_link(a, b), &never).unwrap();
        assert_eq!(used, fin);
        assert_eq!(fs::read(&fin).unwrap(), b"new");
        assert!(!tmp.exists());
    }

    #[test]
    fn ac7_linux_place_refuses_a_taken_name_and_keeps_both_files() {
        let d = tempfile::tempdir().unwrap();
        let from = d.path().join("from");
        let to = d.path().join("to");
        fs::write(&from, b"new").unwrap();
        fs::write(&to, b"old").unwrap();
        let e = place_no_replace(&from, &to).unwrap_err();
        assert_eq!(e.kind(), ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&to).unwrap(), b"old");
        assert_eq!(fs::read(&from).unwrap(), b"new");
    }

    #[cfg(unix)]
    #[test]
    fn ac7_linux_place_refuses_a_dangling_link_name() {
        let d = tempfile::tempdir().unwrap();
        let from = d.path().join("from");
        let to = d.path().join("to");
        fs::write(&from, b"new").unwrap();
        std::os::unix::fs::symlink(d.path().join("nowhere"), &to).unwrap();
        let e = place_no_replace(&from, &to).unwrap_err();
        assert_eq!(e.kind(), ErrorKind::AlreadyExists);
        assert!(fs::symlink_metadata(&to).unwrap().file_type().is_symlink());
        assert_eq!(fs::read(&from).unwrap(), b"new");
    }

    #[test]
    fn ac7_linux_place_moves_to_a_free_name() {
        let d = tempfile::tempdir().unwrap();
        let from = d.path().join("from");
        let to = d.path().join("to");
        fs::write(&from, b"new").unwrap();
        place_no_replace(&from, &to).unwrap();
        assert_eq!(fs::read(&to).unwrap(), b"new");
        assert!(!from.exists());
    }

    #[test]
    fn ac7_move_error_codes_80_and_183_are_already_exists() {
        assert_eq!(move_error_kind(80), ErrorKind::AlreadyExists);
        assert_eq!(move_error_kind(183), ErrorKind::AlreadyExists);
    }

    #[test]
    fn ac7_move_error_code_5_is_permission_denied() {
        // On Linux errno 5 is EIO, so only an explicit map gives this.
        assert_eq!(move_error_kind(5), ErrorKind::PermissionDenied);
    }

    #[test]
    fn ac7_move_error_other_codes_are_neither() {
        for code in [0, 1, 2, 3, 87, 79, 81, 182, 184] {
            let k = move_error_kind(code);
            assert_ne!(k, ErrorKind::AlreadyExists, "code {code}");
            assert_ne!(k, ErrorKind::PermissionDenied, "code {code}");
        }
    }
}
