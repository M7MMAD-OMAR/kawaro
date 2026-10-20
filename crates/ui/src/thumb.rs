//! Centre cell picture lookups (S-018g): the thumbnail a file manager already
//! wrote to the freedesktop cache, the MIME type from shared-mime-info
//! `globs2`, and our own icon for that type. Pure functions over the paths
//! and values they are given: nothing here reads the environment, and the
//! thumbnail cache is only read, never written.
//!
//! Thumbnail Managing Standard: `<cache>/thumbnails/<size>/MD5(uri).png`,
//! valid when its `Thumb::URI` equals the URI, its `Thumb::MTime` equals the
//! file's mtime in seconds and its `Thumb::Size`, when present, the size.

use md5::{Digest, Md5};
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// `path` as a `file://` URI: every byte outside `keep` and ASCII
/// alphanumerics becomes `%XX` with upper case hex.
fn encode(path: &Path, keep: &[u8]) -> String {
    let mut s = String::from("file://");
    for &b in path.as_os_str().as_bytes() {
        if b.is_ascii_alphanumeric() || keep.contains(&b) {
            s.push(b as char);
        } else {
            s.push_str(&format!("%{b:02X}"));
        }
    }
    s
}

/// The URI exactly as `Gio.File.get_uri()` writes it (GLib's path-safe set).
pub fn file_uri(path: &Path) -> String {
    encode(path, b"-_.!~*'()/$&+,=:@")
}

/// The URI as Qt and KIO write it: GLib's form with `;` kept literal.
pub fn kio_uri(path: &Path) -> String {
    encode(path, b"-_.!~*'()/$&+,=:@;")
}

/// `$XDG_CACHE_HOME/thumbnails`, or `~/.cache/thumbnails` when the variable is
/// unset, empty or relative.
pub fn thumbnails_dir(xdg_cache_home: Option<&OsStr>, home: &Path) -> PathBuf {
    xdg_cache_home
        .map(Path::new)
        .filter(|p| p.is_absolute())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| home.join(".cache"))
        .join("thumbnails")
}

/// A thumbnail size directory of the freedesktop cache.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Size {
    Normal,
    Large,
    XLarge,
    XxLarge,
}

impl Size {
    /// The directory name under `thumbnails/`.
    pub fn dir(self) -> &'static str {
        match self {
            Size::Normal => "normal",
            Size::Large => "large",
            Size::XLarge => "x-large",
            Size::XxLarge => "xx-large",
        }
    }
}

/// Large first: the centre hexagon is about 190 px wide at scale 1.
pub const SEARCH_ORDER: [Size; 4] = [Size::Large, Size::XLarge, Size::XxLarge, Size::Normal];

/// `<thumbnails>/<size>/MD5(uri).png`, lower case hex.
pub fn thumbnail_path(thumbnails: &Path, size: Size, uri: &str) -> PathBuf {
    let hex: String = Md5::digest(uri.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    thumbnails.join(size.dir()).join(format!("{hex}.png"))
}

/// The `tEXt` and `zTXt` chunks before the image data. KIO writes through Qt,
/// which compresses values of 40 characters or more, so long URIs are `zTXt`.
fn png_text(path: &Path) -> Vec<(String, String)> {
    let Ok(f) = std::fs::File::open(path) else {
        return Vec::new();
    };
    let mut d = png::Decoder::new(std::io::BufReader::new(f));
    d.set_ignore_text_chunk(false);
    let Ok(r) = d.read_info() else {
        return Vec::new();
    };
    let info = r.info();
    let mut v: Vec<(String, String)> = info
        .uncompressed_latin1_text
        .iter()
        .map(|t| (t.keyword.clone(), t.text.clone()))
        .collect();
    for t in &info.compressed_latin1_text {
        let mut t = t.clone();
        if t.decompress_text().is_ok()
            && let Ok(text) = t.get_text()
        {
            v.push((t.keyword, text));
        }
    }
    v
}

/// Whether the PNG at `path` is a thumbnail of `uri` with this mtime and size.
fn valid(path: &Path, uri: &str, mtime: u64, size: u64) -> bool {
    let text = png_text(path);
    let get = |k: &str| text.iter().find(|(kk, _)| kk == k).map(|(_, v)| v.as_str());
    get("Thumb::URI") == Some(uri)
        && get("Thumb::MTime").and_then(|v| v.parse::<u64>().ok()) == Some(mtime)
        && get("Thumb::Size").is_none_or(|v| v.parse::<u64>().ok() == Some(size))
}

/// The first valid cached thumbnail of `file`, trying its GLib URI, its Qt
/// form, then the same two for its symlink target, each in `SEARCH_ORDER`.
pub fn find_thumbnail(thumbnails: &Path, file: &Path) -> Option<PathBuf> {
    let meta = std::fs::metadata(file).ok()?;
    let mtime = meta
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs();
    let mut paths = vec![file.to_path_buf()];
    if let Ok(target) = std::fs::canonicalize(file) {
        paths.push(target);
    }
    let mut uris: Vec<String> = Vec::new();
    for p in &paths {
        for u in [file_uri(p), kio_uri(p)] {
            if !uris.contains(&u) {
                uris.push(u);
            }
        }
    }
    uris.iter().find_map(|uri| {
        SEARCH_ORDER
            .into_iter()
            .map(|s| thumbnail_path(thumbnails, s, uri))
            .find(|t| valid(t, uri, mtime, meta.len()))
    })
}

/// `XDG_DATA_HOME` (else `~/.local/share`) then `XDG_DATA_DIRS` (else
/// `/usr/local/share:/usr/share`); empty entries are skipped.
pub fn data_dirs(
    xdg_data_home: Option<&OsStr>,
    xdg_data_dirs: Option<&OsStr>,
    home: &Path,
) -> Vec<PathBuf> {
    let first = xdg_data_home
        .map(Path::new)
        .filter(|p| p.is_absolute())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| home.join(".local/share"));
    let mut rest: Vec<PathBuf> = xdg_data_dirs
        .map(|v| v.as_bytes())
        .unwrap_or_default()
        .split(|&b| b == b':')
        .filter(|s| !s.is_empty())
        .map(|s| PathBuf::from(OsStr::from_bytes(s)))
        .collect();
    if rest.is_empty() {
        rest = vec!["/usr/local/share".into(), "/usr/share".into()];
    }
    let mut v = vec![first];
    v.extend(rest);
    v
}

/// One `globs2` line.
struct Glob {
    weight: u32,
    mime: String,
    pattern: String,
    case_sensitive: bool,
}

impl Glob {
    /// `*suffix` matches the end of the name, a pattern without wildcards the
    /// whole name; other wildcard patterns are not supported and never match.
    fn matches(&self, name: &str) -> bool {
        let (name, pattern) = if self.case_sensitive {
            (name.to_string(), self.pattern.clone())
        } else {
            (name.to_lowercase(), self.pattern.to_lowercase())
        };
        match pattern.strip_prefix('*') {
            Some(suffix) => name.ends_with(suffix),
            None => name == pattern,
        }
    }
}

/// The name globs of shared-mime-info, merged from several data directories.
pub struct Globs {
    globs: Vec<Glob>,
}

impl Globs {
    /// Reads `<dir>/mime/globs2` of each directory, in order; a missing file
    /// is skipped.
    pub fn load(dirs: &[PathBuf]) -> Globs {
        let mut globs = Vec::new();
        for d in dirs {
            let Ok(text) = std::fs::read_to_string(d.join("mime/globs2")) else {
                continue;
            };
            for line in text.lines().filter(|l| !l.starts_with('#')) {
                let mut it = line.splitn(4, ':');
                let (Some(w), Some(mime), Some(pattern)) = (it.next(), it.next(), it.next()) else {
                    continue;
                };
                let Ok(weight) = w.parse() else { continue };
                let case_sensitive = it.next().is_some_and(|f| f.split(',').any(|f| f == "cs"));
                globs.push(Glob {
                    weight,
                    mime: mime.to_string(),
                    pattern: pattern.to_string(),
                    case_sensitive,
                });
            }
        }
        Globs { globs }
    }

    /// The MIME type of a file name: the highest weight, then the longest
    /// pattern, then the earlier directory.
    pub fn mime_of(&self, file_name: &str) -> Option<String> {
        let mut best: Option<&Glob> = None;
        for g in self.globs.iter().filter(|g| g.matches(file_name)) {
            if best.is_none_or(|b| (g.weight, g.pattern.len()) > (b.weight, b.pattern.len())) {
                best = Some(g);
            }
        }
        best.map(|g| g.mime.clone())
    }
}

/// The category of a file, which picks its icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Image,
    Video,
    Audio,
    Pdf,
    Document,
    Sheet,
    Slides,
    Text,
    File,
}

impl Kind {
    /// The name of our monoline icon for this kind.
    pub fn icon(self) -> &'static str {
        match self {
            Kind::Image => "image",
            Kind::Video => "video",
            Kind::Audio => "audio",
            Kind::Pdf => "pdf",
            Kind::Document => "document",
            Kind::Sheet => "sheet",
            Kind::Slides => "slides",
            Kind::Text => "text",
            Kind::File => "file",
        }
    }
}

/// Maps a MIME type to our categories.
pub fn kind_of(mime: &str) -> Kind {
    match mime {
        "application/pdf" => Kind::Pdf,
        "application/vnd.oasis.opendocument.text"
        | "application/msword"
        | "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        | "application/rtf" => Kind::Document,
        "application/vnd.oasis.opendocument.spreadsheet"
        | "application/vnd.ms-excel"
        | "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        | "text/csv" => Kind::Sheet,
        "application/vnd.oasis.opendocument.presentation"
        | "application/vnd.ms-powerpoint"
        | "application/vnd.openxmlformats-officedocument.presentationml.presentation" => {
            Kind::Slides
        }
        _ => match mime.split_once('/') {
            Some(("image", _)) => Kind::Image,
            Some(("video", _)) => Kind::Video,
            Some(("audio", _)) => Kind::Audio,
            Some(("text", _)) => Kind::Text,
            _ => Kind::File,
        },
    }
}

/// The category of a file name, through its MIME type.
pub fn kind_for(globs: &Globs, file_name: &str) -> Kind {
    globs.mime_of(file_name).map_or(Kind::File, |m| kind_of(&m))
}

/// The kind all files share, else `File` (also for none).
pub fn common_kind<I: IntoIterator<Item = Kind>>(kinds: I) -> Kind {
    let mut it = kinds.into_iter();
    let Some(first) = it.next() else {
        return Kind::File;
    };
    if it.all(|k| k == first) {
        first
    } else {
        Kind::File
    }
}

/// The last extension in upper case when it has one to four characters.
pub fn format_code(file_name: &str) -> Option<String> {
    let (_, ext) = file_name.rsplit_once('.')?;
    (1..=4)
        .contains(&ext.chars().count())
        .then(|| ext.to_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Qt writes values of 40 characters or more as `zTXt`, so a KIO
    /// thumbnail of a long URI carries it compressed.
    #[test]
    fn compressed_text_chunks_are_read() {
        let dir = Path::new(env!("OUT_DIR")).join("s018g-unit-ztxt");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("a long enough name for compression.png");
        std::fs::write(&file, b"x").unwrap();
        let m = std::fs::metadata(&file).unwrap();
        let mtime = m
            .modified()
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let uri = kio_uri(&file);
        let cache = dir.join("thumbnails");
        let t = thumbnail_path(&cache, Size::Normal, &uri);
        std::fs::create_dir_all(t.parent().unwrap()).unwrap();
        let w = std::io::BufWriter::new(std::fs::File::create(&t).unwrap());
        let mut e = png::Encoder::new(w, 1, 1);
        e.set_color(png::ColorType::Rgba);
        e.add_ztxt_chunk("Thumb::URI".into(), uri.clone()).unwrap();
        e.add_ztxt_chunk("Thumb::MTime".into(), mtime.to_string())
            .unwrap();
        let mut w = e.write_header().unwrap();
        w.write_image_data(&[0, 0, 0, 255]).unwrap();
        drop(w);
        assert_eq!(find_thumbnail(&cache, &file), Some(t));
    }
}
