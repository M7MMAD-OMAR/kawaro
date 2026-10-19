//! Coarse media classification by file extension. Engines check the real
//! content; this only decides which actions the hive offers.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MediaKind {
    Image,
    Audio,
    Video,
    Document,
    Pdf,
    Other,
}

const IMAGE: &[&str] = &[
    "png", "jpg", "jpeg", "jpe", "webp", "avif", "gif", "bmp", "tif", "tiff", "ico", "tga", "qoi",
    "pnm", "pbm", "pgm", "ppm", "heic", "heif", "jxl",
];
const AUDIO: &[&str] = &[
    "mp3", "ogg", "oga", "opus", "flac", "wav", "m4a", "aac", "wma", "aiff", "aif",
];
const VIDEO: &[&str] = &[
    "mp4", "m4v", "mkv", "webm", "mov", "avi", "wmv", "flv", "mpeg", "mpg", "3gp", "ts",
];
const DOCUMENT: &[&str] = &[
    "doc", "docx", "odt", "rtf", "txt", "md", "xls", "xlsx", "ods", "csv", "ppt", "pptx", "odp",
];

/// Lower-case extension without the dot, or an empty string.
pub fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default()
}

pub fn detect(path: &Path) -> MediaKind {
    let ext = extension(path);
    let e = ext.as_str();
    if IMAGE.contains(&e) {
        MediaKind::Image
    } else if AUDIO.contains(&e) {
        MediaKind::Audio
    } else if VIDEO.contains(&e) {
        MediaKind::Video
    } else if DOCUMENT.contains(&e) {
        MediaKind::Document
    } else if e == "pdf" {
        MediaKind::Pdf
    } else {
        MediaKind::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_by_extension_case_insensitively() {
        assert_eq!(detect(Path::new("/a/B.JPG")), MediaKind::Image);
        assert_eq!(detect(Path::new("song.Flac")), MediaKind::Audio);
        assert_eq!(detect(Path::new("clip.mkv")), MediaKind::Video);
        assert_eq!(detect(Path::new("report.docx")), MediaKind::Document);
        assert_eq!(detect(Path::new("scan.pdf")), MediaKind::Pdf);
        assert_eq!(detect(Path::new("archive.zip")), MediaKind::Other);
        assert_eq!(detect(Path::new("noext")), MediaKind::Other);
    }
}
