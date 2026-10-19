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
    // `.sbarhive-<pid>-<count>-<nanos>-<final name>`: hidden, and the final
    // name stays last so the extension (or its absence) is unchanged.
    let mut name = OsString::from(format!(".sbarhive-{}-{count}-{nanos}-", std::process::id()));
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

pub(crate) fn finalize(temp: &Path, final_path: &Path) -> std::io::Result<PathBuf> {
    finalize_with(temp, final_path, &|a, b| std::fs::hard_link(a, b))
}

/// Tries `final_path`, then the following numbers. `hard_link` fails when
/// the target exists, so it never replaces a file. When the filesystem
/// cannot hard link (vfat, some network mounts), falls back to an
/// existence check followed by `rename`. That fallback has a small race:
/// a file created between the check and the rename would be replaced.
fn finalize_with(temp: &Path, final_path: &Path, link: &LinkFn) -> std::io::Result<PathBuf> {
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
                if exists(&candidate) {
                    continue;
                }
                std::fs::rename(temp, &candidate)?;
                return Ok(candidate);
            }
        }
    }
    Err(std::io::Error::new(
        ErrorKind::AlreadyExists,
        "no free output name",
    ))
}

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
        let used = finalize_with(&tmp, &fin, &unsupported).unwrap();
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
        let e = finalize_with(&tmp, &fin, &unsupported).unwrap_err();
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
        let e = finalize_with(&tmp, &fin, &always_taken).unwrap_err();
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
        };
        let first = d.path().join("a.webp");
        let got = final_path_with(&d.path().join("a.png"), &spec, d.path(), &|p| p == first);
        assert_eq!(got, d.path().join("a (1).webp"));
    }
}
