//! File URIs, MIME types and the PNG preview of a tool result.

use base64::Engine as _;
use std::path::Path;

/// Extensions the image crate can decode in this build.
const IMAGE_EXT: [&str; 9] = [
    "png", "jpg", "jpeg", "jpe", "webp", "gif", "bmp", "tif", "tiff",
];

fn lower_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
}

/// True when the file is an image this build can decode, so a preview is
/// worth a try.
pub(crate) fn is_image(path: &Path) -> bool {
    lower_extension(path).is_some_and(|ext| IMAGE_EXT.contains(&ext.as_str()))
}

/// The most pixels a preview may decode. A very large scan is skipped
/// instead: the tool still succeeds, it just gets no image block.
const MAX_PREVIEW_PIXELS: u64 = 40_000_000;

/// True when a picture is too big to decode for a preview: over 40 million
/// pixels, which covers a 200 MP scan.
fn too_large(width: u32, height: u32) -> bool {
    u64::from(width) * u64::from(height) > MAX_PREVIEW_PIXELS
}

/// A `content` image block: the file decoded, thumbnailed to `px` on its long
/// side and re-encoded as a PNG, base64 with the standard alphabet.
///
/// The header is read first and a picture over [`MAX_PREVIEW_PIXELS`] is
/// skipped without decoding it, so a 200 MP scan cannot exhaust memory.
pub(crate) fn block(path: &Path, px: u32) -> Option<serde_json::Value> {
    let reader = image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?;
    let (width, height) = reader.into_dimensions().ok()?;
    if too_large(width, height) {
        return None;
    }
    let image = image::open(path).ok()?;
    let side = px.max(1);
    let thumb = image.thumbnail(side, side);
    let mut buffer = std::io::Cursor::new(Vec::new());
    thumb.write_to(&mut buffer, image::ImageFormat::Png).ok()?;
    let data = base64::engine::general_purpose::STANDARD.encode(buffer.into_inner());
    Some(serde_json::json!({
        "type": "image",
        "data": data,
        "mimeType": "image/png",
    }))
}

/// The MIME type of a result file, by extension.
pub(crate) fn mime_type(path: &Path) -> &'static str {
    match lower_extension(path).as_deref() {
        Some("png") => "image/png",
        Some("jpg" | "jpeg" | "jpe") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        Some("bmp") => "image/bmp",
        Some("tif" | "tiff") => "image/tiff",
        Some("avif") => "image/avif",
        Some("ico") => "image/x-icon",
        Some("pdf") => "application/pdf",
        Some("zip") => "application/zip",
        Some("mp4") => "video/mp4",
        Some("mp3") => "audio/mpeg",
        _ => "application/octet-stream",
    }
}

/// A `file://` URI for a path: every byte outside the unreserved set and
/// `/` is percent encoded. A non unix path uses its lossy string.
pub(crate) fn file_uri(path: &Path) -> String {
    #[cfg(unix)]
    let bytes = path.as_os_str().as_encoded_bytes().to_vec();
    #[cfg(not(unix))]
    let bytes = path.to_string_lossy().into_owned().into_bytes();
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut uri = String::from("file://");
    for byte in bytes {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                uri.push(byte as char);
            }
            _ => {
                uri.push('%');
                uri.push(HEX[usize::from(byte >> 4)] as char);
                uri.push(HEX[usize::from(byte & 0x0f)] as char);
            }
        }
    }
    uri
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_over_forty_million_pixels_is_not_previewed() {
        assert!(!too_large(6000, 6000));
        assert!(
            !too_large(8000, 5000),
            "exactly 40 million pixels is allowed"
        );
        assert!(too_large(8001, 5000));
        assert!(too_large(20_000, 10_000));
    }
}
