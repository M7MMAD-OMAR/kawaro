//! S-018g: the pure lookups behind the centre cell picture
//! (backlog/S-018-hive-overlay.md, AC21 to AC23).
//!
//! Contract these tests fix, in `kawaro_ui::thumb` (a `pub mod`):
//! - `pub fn file_uri(path: &Path) -> String`: the URI exactly as
//!   `Gio.File.get_uri()` writes it.
//! - `pub fn kio_uri(path: &Path) -> String`: the form Qt and KIO write, the
//!   same except that `;` stays literal.
//! - `pub fn thumbnails_dir(xdg_cache_home: Option<&OsStr>, home: &Path) -> PathBuf`:
//!   `$XDG_CACHE_HOME/thumbnails`, or `~/.cache/thumbnails` when the variable
//!   is unset, empty or relative (XDG Base Directory).
//! - `pub enum Size { Normal, Large, XLarge, XxLarge }` (`Clone, Copy, Debug,
//!   PartialEq, Eq`) with `pub fn dir(self) -> &'static str` and
//!   `pub const SEARCH_ORDER: [Size; 4]`.
//! - `pub fn thumbnail_path(thumbnails: &Path, size: Size, uri: &str) -> PathBuf`:
//!   `<thumbnails>/<size>/MD5(uri).png`, lower case hex.
//! - `pub fn find_thumbnail(thumbnails: &Path, file: &Path) -> Option<PathBuf>`:
//!   the first valid cached thumbnail in `SEARCH_ORDER`, never writing.
//! - `pub fn data_dirs(xdg_data_home: Option<&OsStr>, xdg_data_dirs: Option<&OsStr>,
//!   home: &Path) -> Vec<PathBuf>`: XDG_DATA_HOME first, then XDG_DATA_DIRS.
//! - `pub struct Globs` with `pub fn load(dirs: &[PathBuf]) -> Globs` (reads
//!   `<dir>/mime/globs2` of each) and `pub fn mime_of(&self, file_name: &str)
//!   -> Option<String>`.
//! - `pub enum Kind { Image, Video, Audio, Pdf, Document, Sheet, Slides, Text,
//!   File }` (`Clone, Copy, Debug, PartialEq, Eq`) with `pub fn icon(self)
//!   -> &'static str`, an icon name of `kawaro_ui::icon_names()`.
//! - `pub fn kind_of(mime: &str) -> Kind`, `pub fn kind_for(globs: &Globs,
//!   file_name: &str) -> Kind`, `pub fn common_kind<I: IntoIterator<Item = Kind>>(kinds: I)
//!   -> Kind` and `pub fn format_code(file_name: &str) -> Option<String>`.
//!
//! Every fixture is built here, in `CARGO_TARGET_TMPDIR`; the owner's real
//! thumbnail cache and data directories are never read.

use kawaro_ui::thumb::{
    Globs, Kind, SEARCH_ORDER, Size, common_kind, data_dirs, file_uri, find_thumbnail, format_code,
    kind_for, kind_of, kio_uri, thumbnail_path, thumbnails_dir,
};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------- helpers

/// A fresh, empty directory for one test.
fn scratch(test: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("s018g")
        .join(test);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    // The expected URIs below are "file://" + this path + an encoded name,
    // which holds only while the path itself needs no escaping.
    let s = dir.to_str().unwrap();
    assert!(
        s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/-_.".contains(&b)),
        "scratch path needs escaping: {s}"
    );
    dir
}

/// A file with some content, returned with its mtime in seconds and size.
fn source(dir: &Path, name: &str) -> (PathBuf, u64, u64) {
    let p = dir.join(name);
    fs::write(&p, b"not really a picture").unwrap();
    let m = fs::metadata(&p).unwrap();
    let secs = m
        .modified()
        .unwrap()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    (p, secs, m.len())
}

/// A 1 x 1 RGBA PNG with the given tEXt chunks, as a thumbnailer writes it.
fn write_thumb(path: &Path, text: &[(&str, String)]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let f = fs::File::create(path).unwrap();
    let mut e = png::Encoder::new(std::io::BufWriter::new(f), 1, 1);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    for (k, v) in text {
        e.add_text_chunk((*k).to_string(), v.clone()).unwrap();
    }
    let mut w = e.write_header().unwrap();
    w.write_image_data(&[200, 100, 50, 255]).unwrap();
}

fn meta(uri: &str, mtime: u64) -> Vec<(&'static str, String)> {
    vec![
        ("Thumb::URI", uri.to_string()),
        ("Thumb::MTime", mtime.to_string()),
    ]
}

/// Every entry under `root`: relative path to (is dir, length, mtime).
fn tree(root: &Path) -> BTreeMap<PathBuf, (bool, u64, SystemTime)> {
    let mut out = BTreeMap::new();
    let mut todo = vec![root.to_path_buf()];
    while let Some(d) = todo.pop() {
        let Ok(rd) = fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            let m = fs::symlink_metadata(&p).unwrap();
            out.insert(
                p.strip_prefix(root).unwrap().to_path_buf(),
                (m.is_dir(), m.len(), m.modified().unwrap()),
            );
            if m.is_dir() {
                todo.push(p);
            }
        }
    }
    out
}

fn md5_name(uri: &str) -> String {
    thumbnail_path(Path::new("/t"), Size::Normal, uri)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/mime")
}

fn globs() -> Globs {
    Globs::load(&[fixtures().join("home"), fixtures().join("system")])
}

// ---------------------------------------------------------------- AC21

/// `Gio.File.get_uri()` for `/home/u/<name>`, recorded with PyGObject
/// (GLib 2.86) on 2026-10-20.
const GIO: [(&str, &str); 9] = [
    ("صورة.png", "file:///home/u/%D8%B5%D9%88%D8%B1%D8%A9.png"),
    ("my file.png", "file:///home/u/my%20file.png"),
    ("a;b.png", "file:///home/u/a%3Bb.png"),
    ("a#b.png", "file:///home/u/a%23b.png"),
    ("a?b.png", "file:///home/u/a%3Fb.png"),
    ("a[1].png", "file:///home/u/a%5B1%5D.png"),
    (
        "صورة 1+&~,(x).png",
        "file:///home/u/%D8%B5%D9%88%D8%B1%D8%A9%201+&~,(x).png",
    ),
    ("100%.png", "file:///home/u/100%25.png"),
    // The rest of GLib's path-safe set, kept literal.
    ("x'!*=:@$-_.png", "file:///home/u/x'!*=:@$-_.png"),
];

#[test]
fn ac21_file_uri_matches_gio_get_uri_byte_for_byte() {
    for (name, want) in GIO {
        let path = PathBuf::from("/home/u").join(name);
        assert_eq!(file_uri(&path), want, "name {name:?}");
    }
}

#[test]
fn ac21_file_uri_escapes_non_utf8_bytes_upper_case() {
    use std::os::unix::ffi::OsStrExt;
    let path = Path::new(OsStr::from_bytes(b"/tmp/a\xff\x0ab"));
    assert_eq!(file_uri(path), "file:///tmp/a%FF%0Ab");
}

#[test]
fn ac21_kio_uri_keeps_semicolon_literal_and_matches_glib_otherwise() {
    assert_eq!(
        kio_uri(Path::new("/home/u/a;b.png")),
        "file:///home/u/a;b.png"
    );
    for (name, want) in GIO {
        if name.contains(';') {
            continue;
        }
        let path = PathBuf::from("/home/u").join(name);
        assert_eq!(kio_uri(&path), want, "name {name:?}");
    }
}

#[test]
fn ac21_thumbnail_path_is_size_dir_and_md5_of_uri() {
    let root = Path::new("/c/thumbnails");
    let uri = "file:///home/u/%D8%B5%D9%88%D8%B1%D8%A9%201+&~,(x).png";
    assert_eq!(
        thumbnail_path(root, Size::Large, uri),
        Path::new("/c/thumbnails/large/31255e64a30fefe4324248e86008c87c.png")
    );
    let uri = "file:///home/u/my%20file.png";
    assert_eq!(
        thumbnail_path(root, Size::XxLarge, uri),
        Path::new("/c/thumbnails/xx-large/db5f14e12bfc3a9766e36018ecf0879a.png")
    );
}

#[test]
fn ac21_size_dirs_are_the_freedesktop_names() {
    assert_eq!(Size::Normal.dir(), "normal");
    assert_eq!(Size::Large.dir(), "large");
    assert_eq!(Size::XLarge.dir(), "x-large");
    assert_eq!(Size::XxLarge.dir(), "xx-large");
}

#[test]
fn ac21_thumbnails_dir_from_xdg_cache_home_else_home_cache() {
    let home = Path::new("/h");
    assert_eq!(
        thumbnails_dir(Some(OsStr::new("/c")), home),
        Path::new("/c/thumbnails")
    );
    assert_eq!(
        thumbnails_dir(None, home),
        Path::new("/h/.cache/thumbnails")
    );
    assert_eq!(
        thumbnails_dir(Some(OsStr::new("")), home),
        Path::new("/h/.cache/thumbnails")
    );
    assert_eq!(
        thumbnails_dir(Some(OsStr::new("rel/cache")), home),
        Path::new("/h/.cache/thumbnails")
    );
}

// ---------------------------------------------------------------- AC22

#[test]
fn ac22_search_order_is_large_x_large_xx_large_normal() {
    assert_eq!(
        SEARCH_ORDER,
        [Size::Large, Size::XLarge, Size::XxLarge, Size::Normal]
    );
}

#[test]
fn ac22_valid_thumbnail_is_found_for_the_arabic_name() {
    let dir = scratch("valid_arabic");
    let cache = dir.join("thumbnails");
    let (file, mtime, _) = source(&dir, "صورة 1+&~,(x).png");
    let uri = format!(
        "file://{}/%D8%B5%D9%88%D8%B1%D8%A9%201+&~,(x).png",
        dir.display()
    );
    assert_eq!(file_uri(&file), uri);
    let t = thumbnail_path(&cache, Size::Large, &uri);
    write_thumb(&t, &meta(&uri, mtime));
    assert_eq!(find_thumbnail(&cache, &file), Some(t));
}

#[test]
fn ac22_each_size_alone_is_found() {
    for size in SEARCH_ORDER {
        let dir = scratch(&format!("alone_{}", size.dir()));
        let cache = dir.join("thumbnails");
        let (file, mtime, _) = source(&dir, "a.png");
        let uri = format!("file://{}/a.png", dir.display());
        let t = thumbnail_path(&cache, size, &uri);
        write_thumb(&t, &meta(&uri, mtime));
        assert_eq!(find_thumbnail(&cache, &file), Some(t), "size {size:?}");
    }
}

#[test]
fn ac22_order_prefers_earlier_sizes() {
    let dir = scratch("order");
    let cache = dir.join("thumbnails");
    let (file, mtime, _) = source(&dir, "a.png");
    let uri = format!("file://{}/a.png", dir.display());
    let path = |s| thumbnail_path(&cache, s, &uri);
    // All four valid: large wins.
    for s in SEARCH_ORDER {
        write_thumb(&path(s), &meta(&uri, mtime));
    }
    assert_eq!(find_thumbnail(&cache, &file), Some(path(Size::Large)));
    // Without large: x-large.
    fs::remove_file(path(Size::Large)).unwrap();
    assert_eq!(find_thumbnail(&cache, &file), Some(path(Size::XLarge)));
    // Without x-large: xx-large, before normal.
    fs::remove_file(path(Size::XLarge)).unwrap();
    assert_eq!(find_thumbnail(&cache, &file), Some(path(Size::XxLarge)));
    fs::remove_file(path(Size::XxLarge)).unwrap();
    assert_eq!(find_thumbnail(&cache, &file), Some(path(Size::Normal)));
}

#[test]
fn ac22_stale_earlier_size_falls_through_to_a_valid_later_one() {
    let dir = scratch("stale_then_valid");
    let cache = dir.join("thumbnails");
    let (file, mtime, _) = source(&dir, "a.png");
    let uri = format!("file://{}/a.png", dir.display());
    write_thumb(
        &thumbnail_path(&cache, Size::Large, &uri),
        &meta(&uri, mtime - 5),
    );
    let normal = thumbnail_path(&cache, Size::Normal, &uri);
    write_thumb(&normal, &meta(&uri, mtime));
    assert_eq!(find_thumbnail(&cache, &file), Some(normal));
}

#[test]
fn ac22_uri_mismatch_is_rejected() {
    let dir = scratch("uri_mismatch");
    let cache = dir.join("thumbnails");
    let (file, mtime, _) = source(&dir, "a.png");
    let uri = format!("file://{}/a.png", dir.display());
    let t = thumbnail_path(&cache, Size::Large, &uri);
    write_thumb(&t, &meta(&format!("file://{}/b.png", dir.display()), mtime));
    assert_eq!(find_thumbnail(&cache, &file), None);
}

#[test]
fn ac22_missing_uri_or_mtime_is_rejected() {
    let dir = scratch("missing_keys");
    let cache = dir.join("thumbnails");
    let (file, mtime, _) = source(&dir, "a.png");
    let uri = format!("file://{}/a.png", dir.display());
    let t = thumbnail_path(&cache, Size::Large, &uri);
    write_thumb(&t, &[("Thumb::MTime", mtime.to_string())]);
    assert_eq!(find_thumbnail(&cache, &file), None, "no Thumb::URI");
    write_thumb(&t, &[("Thumb::URI", uri.clone())]);
    assert_eq!(find_thumbnail(&cache, &file), None, "no Thumb::MTime");
    write_thumb(&t, &[]);
    assert_eq!(find_thumbnail(&cache, &file), None, "no text at all");
}

#[test]
fn ac22_mtime_in_seconds_must_be_equal() {
    let dir = scratch("mtime");
    let cache = dir.join("thumbnails");
    let (file, mtime, _) = source(&dir, "a.png");
    let uri = format!("file://{}/a.png", dir.display());
    let t = thumbnail_path(&cache, Size::Large, &uri);
    for wrong in [mtime - 1, mtime + 1, 0] {
        write_thumb(&t, &meta(&uri, wrong));
        assert_eq!(find_thumbnail(&cache, &file), None, "mtime {wrong}");
    }
    // Sub-second parts of the file time do not matter.
    write_thumb(&t, &meta(&uri, mtime));
    assert_eq!(find_thumbnail(&cache, &file), Some(t));
}

#[test]
fn ac22_thumb_size_when_present_must_equal_the_file_size() {
    let dir = scratch("size");
    let cache = dir.join("thumbnails");
    let (file, mtime, len) = source(&dir, "a.png");
    let uri = format!("file://{}/a.png", dir.display());
    let t = thumbnail_path(&cache, Size::Large, &uri);
    let mut m = meta(&uri, mtime);
    m.push(("Thumb::Size", len.to_string()));
    write_thumb(&t, &m);
    assert_eq!(find_thumbnail(&cache, &file), Some(t.clone()), "equal size");
    let mut m = meta(&uri, mtime);
    m.push(("Thumb::Size", (len + 1).to_string()));
    write_thumb(&t, &m);
    assert_eq!(find_thumbnail(&cache, &file), None, "different size");
}

#[test]
fn ac22_touch_makes_the_thumbnail_stale() {
    let dir = scratch("touch");
    let cache = dir.join("thumbnails");
    let (file, mtime, _) = source(&dir, "a b.png");
    let uri = format!("file://{}/a%20b.png", dir.display());
    let t = thumbnail_path(&cache, Size::Large, &uri);
    write_thumb(&t, &meta(&uri, mtime));
    assert_eq!(find_thumbnail(&cache, &file), Some(t));
    let later = UNIX_EPOCH + Duration::from_secs(mtime + 30);
    fs::File::options()
        .write(true)
        .open(&file)
        .unwrap()
        .set_modified(later)
        .unwrap();
    assert_eq!(find_thumbnail(&cache, &file), None);
}

#[test]
fn ac22_failed_record_means_no_thumbnail() {
    let dir = scratch("failed");
    let cache = dir.join("thumbnails");
    let (file, mtime, _) = source(&dir, "a.png");
    let uri = format!("file://{}/a.png", dir.display());
    let name = md5_name(&uri);
    write_thumb(
        &cache.join("fail/gnome-thumbnail-factory").join(&name),
        &meta(&uri, mtime),
    );
    assert_eq!(find_thumbnail(&cache, &file), None);
}

#[test]
fn ac22_semicolon_name_finds_the_glib_form() {
    let dir = scratch("semicolon_glib");
    let cache = dir.join("thumbnails");
    let (file, mtime, _) = source(&dir, "a;b.png");
    let uri = format!("file://{}/a%3Bb.png", dir.display());
    let t = thumbnail_path(&cache, Size::Large, &uri);
    write_thumb(&t, &meta(&uri, mtime));
    assert_eq!(find_thumbnail(&cache, &file), Some(t));
}

#[test]
fn ac22_semicolon_name_also_finds_the_qt_literal_form() {
    let dir = scratch("semicolon_qt");
    let cache = dir.join("thumbnails");
    let (file, mtime, _) = source(&dir, "a;b.png");
    let uri = format!("file://{}/a;b.png", dir.display());
    assert_eq!(kio_uri(&file), uri);
    // Dolphin with default icon sizes writes normal only.
    let t = thumbnail_path(&cache, Size::Normal, &uri);
    write_thumb(&t, &meta(&uri, mtime));
    assert_eq!(find_thumbnail(&cache, &file), Some(t));
}

#[test]
fn ac22_qt_form_still_needs_its_own_uri_in_the_chunk() {
    let dir = scratch("semicolon_qt_mismatch");
    let cache = dir.join("thumbnails");
    let (file, mtime, _) = source(&dir, "a;b.png");
    let qt = format!("file://{}/a;b.png", dir.display());
    let glib = format!("file://{}/a%3Bb.png", dir.display());
    // Hashed by the Qt form but claiming the GLib one: not a match.
    write_thumb(
        &thumbnail_path(&cache, Size::Normal, &qt),
        &meta(&glib, mtime),
    );
    assert_eq!(find_thumbnail(&cache, &file), None);
}

#[test]
fn ac22_symlink_finds_its_own_uri() {
    let dir = scratch("symlink_own");
    let cache = dir.join("thumbnails");
    let (target, mtime, _) = source(&dir, "real.png");
    let link = dir.join("link.png");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let uri = format!("file://{}/link.png", dir.display());
    let t = thumbnail_path(&cache, Size::Large, &uri);
    write_thumb(&t, &meta(&uri, mtime));
    assert_eq!(find_thumbnail(&cache, &link), Some(t));
}

#[test]
fn ac22_symlink_also_finds_its_target_like_kio() {
    let dir = scratch("symlink_target");
    let cache = dir.join("thumbnails");
    let (target, mtime, _) = source(&dir, "real file.png");
    let link = dir.join("link.png");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let uri = format!("file://{}/real%20file.png", dir.display());
    let t = thumbnail_path(&cache, Size::Normal, &uri);
    write_thumb(&t, &meta(&uri, mtime));
    assert_eq!(find_thumbnail(&cache, &link), Some(t));
}

#[test]
fn ac22_missing_file_or_cache_gives_none() {
    let dir = scratch("missing");
    let cache = dir.join("thumbnails");
    assert_eq!(find_thumbnail(&cache, &dir.join("absent.png")), None);
    let (file, _, _) = source(&dir, "a.png");
    assert_eq!(find_thumbnail(&cache, &file), None);
    assert!(!cache.exists(), "the lookup created {}", cache.display());
}

#[test]
fn ac22_lookup_writes_nothing_under_thumbnails() {
    let dir = scratch("read_only");
    let cache = dir.join("thumbnails");
    let (a, mtime, _) = source(&dir, "a.png");
    let (b, _, _) = source(&dir, "b;c.png");
    let (c, _, _) = source(&dir, "c.png");
    let ua = format!("file://{}/a.png", dir.display());
    let ub = format!("file://{}/b%3Bc.png", dir.display());
    write_thumb(&thumbnail_path(&cache, Size::Large, &ua), &meta(&ua, mtime));
    write_thumb(
        &thumbnail_path(&cache, Size::XLarge, &ub),
        &meta(&ub, mtime - 9),
    );
    fs::create_dir_all(cache.join("fail/gnome-thumbnail-factory")).unwrap();
    let before = tree(&cache);
    for f in [&a, &b, &c, &dir.join("absent.png")] {
        let _ = find_thumbnail(&cache, f);
    }
    assert_eq!(tree(&cache), before);
}

// ---------------------------------------------------------------- AC23

#[test]
fn ac23_data_dirs_put_xdg_data_home_before_xdg_data_dirs() {
    let home = Path::new("/h");
    assert_eq!(
        data_dirs(Some(OsStr::new("/dh")), Some(OsStr::new("/a:/b")), home),
        vec![PathBuf::from("/dh"), "/a".into(), "/b".into()]
    );
    assert_eq!(
        data_dirs(Some(OsStr::new("/dh")), Some(OsStr::new("/a::/b:")), home),
        vec![PathBuf::from("/dh"), "/a".into(), "/b".into()],
        "empty entries are skipped"
    );
}

#[test]
fn ac23_data_dirs_defaults_when_unset_or_empty() {
    let home = Path::new("/h");
    let want = vec![
        PathBuf::from("/h/.local/share"),
        "/usr/local/share".into(),
        "/usr/share".into(),
    ];
    assert_eq!(data_dirs(None, None, home), want);
    assert_eq!(
        data_dirs(Some(OsStr::new("")), Some(OsStr::new("")), home),
        want
    );
    assert_eq!(
        data_dirs(Some(OsStr::new("relative")), None, home),
        want,
        "a relative XDG_DATA_HOME is ignored"
    );
}

#[test]
fn ac23_mime_by_extension_case_insensitive() {
    let g = globs();
    assert_eq!(g.mime_of("photo.png").as_deref(), Some("image/png"));
    assert_eq!(g.mime_of("PHOTO.PNG").as_deref(), Some("image/png"));
    assert_eq!(g.mime_of("صورة 1+&~,(x).png").as_deref(), Some("image/png"));
    assert_eq!(g.mime_of("scan.Pdf").as_deref(), Some("application/pdf"));
    assert_eq!(
        g.mime_of("report.odt").as_deref(),
        Some("application/vnd.oasis.opendocument.text")
    );
}

#[test]
fn ac23_mime_literal_glob_matches_the_whole_name() {
    let g = globs();
    assert_eq!(g.mime_of("Makefile").as_deref(), Some("text/x-makefile"));
    assert_eq!(g.mime_of("my.makefile"), None);
}

#[test]
fn ac23_mime_longest_pattern_wins_at_equal_weight() {
    let g = globs();
    assert_eq!(
        g.mime_of("backup.tar.gz").as_deref(),
        Some("application/x-compressed-tar")
    );
    assert_eq!(g.mime_of("notes.gz").as_deref(), Some("application/gzip"));
}

#[test]
fn ac23_mime_highest_weight_beats_longer_pattern() {
    let g = globs();
    assert_eq!(g.mime_of("f.x.ab").as_deref(), Some("application/x-short"));
}

#[test]
fn ac23_mime_cs_flag_is_case_sensitive() {
    let g = globs();
    assert_eq!(g.mime_of("data.Foo").as_deref(), Some("application/x-foo"));
    assert_eq!(g.mime_of("data.foo"), None);
    assert_eq!(g.mime_of("data.FOO"), None);
}

#[test]
fn ac23_mime_xdg_data_home_wins_a_tie_and_system_still_counts() {
    let g = globs();
    assert_eq!(
        g.mime_of("x.tie").as_deref(),
        Some("application/x-tie-home")
    );
    // Only the system file knows png: the files are merged, not replaced.
    assert_eq!(g.mime_of("x.png").as_deref(), Some("image/png"));
    // Reversed order, the system entry wins the tie.
    let rev = Globs::load(&[fixtures().join("system"), fixtures().join("home")]);
    assert_eq!(
        rev.mime_of("x.tie").as_deref(),
        Some("application/x-tie-system")
    );
}

#[test]
fn ac23_mime_unknown_or_missing_dirs_give_none() {
    let g = globs();
    assert_eq!(g.mime_of("archive.unknownext"), None);
    assert_eq!(g.mime_of("noext"), None);
    let empty = Globs::load(&[PathBuf::from("/nonexistent/s018g")]);
    assert_eq!(empty.mime_of("photo.png"), None);
}

#[test]
fn ac23_kind_of_maps_mime_types_to_our_categories() {
    let cases: &[(&str, Kind)] = &[
        ("image/png", Kind::Image),
        ("image/svg+xml", Kind::Image),
        ("video/mp4", Kind::Video),
        ("video/x-matroska", Kind::Video),
        ("audio/mpeg", Kind::Audio),
        ("audio/flac", Kind::Audio),
        ("application/pdf", Kind::Pdf),
        ("application/vnd.oasis.opendocument.text", Kind::Document),
        ("application/msword", Kind::Document),
        (
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            Kind::Document,
        ),
        ("application/rtf", Kind::Document),
        (
            "application/vnd.oasis.opendocument.spreadsheet",
            Kind::Sheet,
        ),
        ("application/vnd.ms-excel", Kind::Sheet),
        (
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            Kind::Sheet,
        ),
        ("text/csv", Kind::Sheet),
        (
            "application/vnd.oasis.opendocument.presentation",
            Kind::Slides,
        ),
        ("application/vnd.ms-powerpoint", Kind::Slides),
        (
            "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            Kind::Slides,
        ),
        ("text/plain", Kind::Text),
        ("text/markdown", Kind::Text),
        ("text/x-makefile", Kind::Text),
        ("application/zip", Kind::File),
        ("application/octet-stream", Kind::File),
        ("application/x-compressed-tar", Kind::File),
        ("inode/directory", Kind::File),
        ("", Kind::File),
        ("imagepng", Kind::File),
    ];
    for (mime, want) in cases {
        assert_eq!(kind_of(mime), *want, "mime {mime:?}");
    }
}

#[test]
fn ac23_kind_icons_are_our_monoline_set() {
    let want = [
        (Kind::Image, "image"),
        (Kind::Video, "video"),
        (Kind::Audio, "audio"),
        (Kind::Pdf, "pdf"),
        (Kind::Document, "document"),
        (Kind::Sheet, "sheet"),
        (Kind::Slides, "slides"),
        (Kind::Text, "text"),
        (Kind::File, "file"),
    ];
    let ours: Vec<&str> = kawaro_ui::icon_names().collect();
    for (kind, name) in want {
        assert_eq!(kind.icon(), name, "{kind:?}");
        assert!(
            ours.contains(&name),
            "icon {name:?} is not in crates/ui/icons"
        );
    }
}

#[test]
fn ac23_kind_for_uses_globs_then_the_map() {
    let g = globs();
    assert_eq!(kind_for(&g, "a.png"), Kind::Image);
    assert_eq!(kind_for(&g, "a.mp4"), Kind::Video);
    assert_eq!(kind_for(&g, "a.mp3"), Kind::Audio);
    assert_eq!(kind_for(&g, "a.pdf"), Kind::Pdf);
    assert_eq!(kind_for(&g, "a.odt"), Kind::Document);
    assert_eq!(kind_for(&g, "a.ods"), Kind::Sheet);
    assert_eq!(kind_for(&g, "a.csv"), Kind::Sheet);
    assert_eq!(kind_for(&g, "a.odp"), Kind::Slides);
    assert_eq!(kind_for(&g, "a.txt"), Kind::Text);
    assert_eq!(kind_for(&g, "a.zip"), Kind::File);
    assert_eq!(kind_for(&g, "a.unknownext"), Kind::File);
}

#[test]
fn ac23_format_code_is_the_upper_case_extension_up_to_four() {
    assert_eq!(format_code("photo.webp").as_deref(), Some("WEBP"));
    assert_eq!(format_code("scan.pdf").as_deref(), Some("PDF"));
    assert_eq!(format_code("a.JPEG").as_deref(), Some("JPEG"));
    assert_eq!(format_code("a.tiff").as_deref(), Some("TIFF"));
    assert_eq!(format_code("a.7z").as_deref(), Some("7Z"));
    assert_eq!(format_code("backup.tar.gz").as_deref(), Some("GZ"));
    assert_eq!(format_code("صورة 1+&~,(x).png").as_deref(), Some("PNG"));
    assert_eq!(format_code("page.xhtml"), None, "five letters");
    assert_eq!(format_code("notes.markdown"), None);
    assert_eq!(format_code("Makefile"), None);
    assert_eq!(format_code(".bashrc"), None);
    assert_eq!(format_code("trailing."), None);
}

#[test]
fn ac23_common_kind_of_several_files() {
    assert_eq!(
        common_kind([Kind::Image, Kind::Image, Kind::Image]),
        Kind::Image
    );
    assert_eq!(common_kind([Kind::Pdf]), Kind::Pdf);
    assert_eq!(common_kind([Kind::Image, Kind::Video]), Kind::File);
    assert_eq!(
        common_kind([Kind::Sheet, Kind::Sheet, Kind::Text]),
        Kind::File
    );
    assert_eq!(common_kind(Vec::<Kind>::new()), Kind::File);
}
