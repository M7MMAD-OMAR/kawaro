//! Centre cell picture: the thumbnail a file manager already wrote to the
//! freedesktop cache, else the file type icon, else a built-in glyph.
//! Nothing is rendered from the file itself, so the cost is one small PNG read.
//!
//! Thumbnail Managing Standard: $XDG_CACHE_HOME/thumbnails/{normal,large,
//! x-large,xx-large}/MD5(uri).png, valid when its Thumb::URI equals the URI
//! and its Thumb::MTime equals the file's mtime in seconds (Thumb::Size, when
//! present, equals the size). A file in fail/<program>/ means "tried, failed".

use md5::{Digest, Md5};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug)]
pub enum Source {
    /// A valid cached thumbnail.
    Thumbnail(PathBuf),
    /// A theme icon for the MIME type.
    Icon { mime: String, path: PathBuf },
    /// No icon found: draw the built-in glyph for this MIME type.
    Glyph { mime: String },
}

/// The file URI exactly as GLib's g_filename_to_uri writes it: bytes outside
/// the allowed set are %XX with upper case hex. The allowed set is the
/// unreserved characters plus the path-safe reserved ones of GLib's
/// UNSAFE_PATH table (checked against Gio by `overlay-spike uri`).
pub fn file_uri(path: &Path) -> String {
    let mut s = String::from("file://");
    for &b in path.as_os_str().as_bytes() {
        let keep = b.is_ascii_alphanumeric() || b"-_.!~*'()/$&+,=:@".contains(&b);
        if keep {
            s.push(b as char);
        } else {
            s.push_str(&format!("%{b:02X}"));
        }
    }
    s
}

pub fn cache_dir() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".cache"))
        .join("thumbnails")
}

fn md5_hex(s: &str) -> String {
    Md5::digest(s.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

/// Text chunks of a PNG (tEXt and iTXt).
fn png_text(path: &Path) -> Option<Vec<(String, String)>> {
    let f = std::fs::File::open(path).ok()?;
    let mut d = png::Decoder::new(std::io::BufReader::new(f));
    d.set_ignore_text_chunk(false);
    let r = d.read_info().ok()?;
    let info = r.info();
    let mut v: Vec<(String, String)> =
        info.uncompressed_latin1_text.iter().map(|t| (t.keyword.clone(), t.text.clone())).collect();
    for t in &info.utf8_text {
        if let Ok(text) = t.get_text() {
            v.push((t.keyword.clone(), text));
        }
    }
    for t in &info.compressed_latin1_text {
        let mut t = t.clone();
        if t.decompress_text().is_ok()
            && let Ok(text) = t.get_text()
        {
            v.push((t.keyword.clone(), text));
        }
    }
    Some(v)
}

/// Why a cached thumbnail was or was not used (printed by the spike).
pub fn check(file: &Path) -> (Option<PathBuf>, Vec<String>) {
    let mut notes = Vec::new();
    let Ok(file) = file.canonicalize() else {
        return (None, vec!["file missing".into()]);
    };
    let uri = file_uri(&file);
    let name = format!("{}.png", md5_hex(&uri));
    notes.push(format!("uri {uri}"));
    notes.push(format!("name {name}"));
    let meta = std::fs::metadata(&file).ok();
    let mtime = meta
        .as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs());
    let size = meta.as_ref().map(|m| m.len());
    let base = cache_dir();
    // Large first: the centre hexagon is about 190 px wide at scale 1.
    for dir in ["large", "x-large", "xx-large", "normal"] {
        let p = base.join(dir).join(&name);
        if !p.exists() {
            continue;
        }
        let text = png_text(&p).unwrap_or_default();
        let get = |k: &str| text.iter().find(|(kk, _)| kk == k).map(|(_, v)| v.clone());
        let t_uri = get("Thumb::URI");
        let t_mtime = get("Thumb::MTime").and_then(|v| v.parse::<u64>().ok());
        let t_size = get("Thumb::Size").and_then(|v| v.parse::<u64>().ok());
        let ok = t_uri.as_deref() == Some(uri.as_str())
            && t_mtime.is_some()
            && t_mtime == mtime
            && (t_size.is_none() || t_size == size);
        notes.push(format!(
            "{dir}: uri {} mtime {:?} (file {:?}) size {:?} => {}",
            if t_uri.as_deref() == Some(uri.as_str()) { "matches" } else { "differs" },
            t_mtime,
            mtime,
            t_size,
            if ok { "valid" } else { "stale" }
        ));
        if ok {
            return (Some(p), notes);
        }
    }
    if let Ok(rd) = std::fs::read_dir(base.join("fail")) {
        for e in rd.flatten() {
            if e.path().join(&name).exists() {
                notes.push(format!("failed before: fail/{}", e.file_name().to_string_lossy()));
            }
        }
    }
    (None, notes)
}

fn data_dirs() -> Vec<PathBuf> {
    let mut v = vec![
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share")),
    ];
    let sys = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    v.extend(sys.split(':').filter(|s| !s.is_empty()).map(PathBuf::from));
    v
}

/// MIME type from shared-mime-info globs2 (name only, highest weight,
/// longest pattern). Magic sniffing is left out: the file manager already
/// knows the type, and a name match is enough for an icon.
pub fn mime_of(file: &Path) -> String {
    let name = file.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    let mut best: Option<(u32, usize, String)> = None;
    for d in data_dirs() {
        let Ok(text) = std::fs::read_to_string(d.join("mime/globs2")) else { continue };
        for line in text.lines().filter(|l| !l.starts_with('#')) {
            let mut it = line.splitn(4, ':');
            let (Some(w), Some(mime), Some(glob)) = (it.next(), it.next(), it.next()) else { continue };
            let w: u32 = w.parse().unwrap_or(50);
            let hit = match glob.strip_prefix('*') {
                Some(suffix) if !suffix.contains(['*', '?', '[']) => name.ends_with(&suffix.to_lowercase()),
                _ => name == glob.to_lowercase(),
            };
            if hit && best.as_ref().is_none_or(|(bw, bl, _)| (w, glob.len()) > (*bw, *bl)) {
                best = Some((w, glob.len(), mime.to_string()));
            }
        }
    }
    best.map(|b| b.2).unwrap_or_else(|| "application/octet-stream".into())
}

/// Icon names for a MIME type: its own (image-png), the generic one from
/// shared-mime-info (image-x-generic), then the media type (image-x-generic).
fn icon_names(mime: &str) -> Vec<String> {
    let mut v = vec![mime.replace('/', "-")];
    for d in data_dirs() {
        if let Ok(text) = std::fs::read_to_string(d.join("mime/generic-icons")) {
            if let Some(g) = text.lines().find_map(|l| l.strip_prefix(&format!("{mime}:"))) {
                v.push(g.to_string());
                break;
            }
        }
    }
    if let Some((media, _)) = mime.split_once('/') {
        v.push(format!("{media}-x-generic"));
    }
    v.push("text-x-generic".into());
    v
}

/// A much reduced icon theme lookup: the theme, then hicolor, preferring
/// scalable, then the largest fixed size, in the mimetypes context only.
pub fn icon_path(mime: &str, theme: &str) -> Option<PathBuf> {
    let names = icon_names(mime);
    let mut roots = Vec::new();
    for d in data_dirs() {
        roots.push(d.join("icons"));
    }
    for name in &names {
        for t in [theme, "hicolor"] {
            for r in &roots {
                let base = r.join(t);
                let mut cands = vec![base.join("scalable/mimetypes").join(format!("{name}.svg"))];
                for s in ["512x512", "256x256", "128x128", "96x96", "64x64", "48x48"] {
                    cands.push(base.join(s).join("mimetypes").join(format!("{name}.png")));
                }
                if let Some(p) = cands.into_iter().find(|p| p.exists()) {
                    return Some(p);
                }
            }
        }
    }
    None
}

pub fn resolve(file: &Path, theme: &str) -> Source {
    if let (Some(p), _) = check(file) {
        return Source::Thumbnail(p);
    }
    let mime = mime_of(file);
    match icon_path(&mime, theme) {
        Some(path) => Source::Icon { mime, path },
        None => Source::Glyph { mime },
    }
}

/// Decodes a PNG to RGBA8 for Slint (the workspace builds Slint without its
/// image decoders).
pub fn load_png(path: &Path) -> Option<slint::Image> {
    let f = std::fs::File::open(path).ok()?;
    let mut d = png::Decoder::new(std::io::BufReader::new(f));
    d.set_transformations(png::Transformations::normalize_to_color8() | png::Transformations::ALPHA);
    let mut r = d.read_info().ok()?;
    let mut buf = vec![0; r.output_buffer_size()];
    let info = r.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width, info.height);
    let mut px = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(w, h);
    let dst = px.make_mut_bytes();
    match info.color_type {
        png::ColorType::Rgba => dst.copy_from_slice(&buf[..dst.len()]),
        png::ColorType::GrayscaleAlpha => {
            for (i, c) in buf.chunks_exact(2).take((w * h) as usize).enumerate() {
                dst[i * 4..i * 4 + 4].copy_from_slice(&[c[0], c[0], c[0], c[1]]);
            }
        }
        _ => return None,
    }
    Some(slint::Image::from_rgba8(px))
}

pub fn load(source: &Source) -> Option<slint::Image> {
    match source {
        Source::Thumbnail(p) => load_png(p),
        Source::Icon { path, .. } if path.extension().is_some_and(|e| e == "svg") => {
            slint::Image::load_from_svg_data(&std::fs::read(path).ok()?).ok()
        }
        Source::Icon { path, .. } => load_png(path),
        Source::Glyph { .. } => None,
    }
}
