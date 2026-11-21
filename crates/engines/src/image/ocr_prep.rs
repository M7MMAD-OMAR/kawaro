//! S-146: the private PNG an OCR child reads.
//!
//! The OCR adapter does not decode pictures itself. This helper reuses the
//! image engine's own loader, so the picture the child sees carries the
//! same EXIF orientation and the same white flattening as an export. The
//! orientation is never recomputed here.
//!
//! Only JPEG, PNG and BMP are accepted. Every other format, including the
//! containers that hold several frames, is refused instead of being
//! reduced to one frame.
//!
//! The limits are the OCR admission, and they are the limits the image
//! source already uses: 64 MiB of source bytes and 16,000,000 source
//! pixels. The pixel limit is checked from the header, before any pixel is
//! decoded.
//!
//! Nothing here writes next to the source. The caller gives the path of
//! the private PNG, which lives in the private work directory of the task.

use super::encode::Target;
use super::source;
use image::{DynamicImage, ImageFormat};
use kawaro_core::{CancelToken, EngineError};
use std::path::{Path, PathBuf};

/// The formats the OCR reads.
pub const SUPPORTED_FORMATS: [ImageFormat; 3] =
    [ImageFormat::Jpeg, ImageFormat::Png, ImageFormat::Bmp];

/// The prepared source: the private PNG and the size of the oriented
/// picture.
#[derive(Clone, Debug)]
pub struct Prepared {
    /// The private PNG the child reads.
    pub path: PathBuf,
    /// The width of the oriented picture.
    pub width: u32,
    /// The height of the oriented picture.
    pub height: u32,
}

/// Reads `input`, applies its orientation, flattens its alpha onto white
/// and writes one private PNG at `out`.
///
/// The source is read once, under the byte limit, and its size and
/// modification time are compared around the read. A source that changed
/// while it was read is refused.
pub fn prepare(input: &Path, out: &Path, cancel: &CancelToken) -> Result<Prepared, EngineError> {
    cancel.check()?;
    let snapshot = source::acquire(input).map_err(|e| map_error(input, e))?;
    if !SUPPORTED_FORMATS.contains(&snapshot.format) {
        return Err(EngineError::Input {
            path: input.to_path_buf(),
            reason: format!(
                "the OCR reads JPEG, PNG and BMP pictures only, not {}",
                format_name(snapshot.format)
            ),
        });
    }
    if snapshot.format == ImageFormat::Png && is_apng(&snapshot.bytes) {
        return Err(EngineError::Input {
            path: input.to_path_buf(),
            reason: "the OCR reads still JPEG, PNG and BMP pictures only, not animated PNG (APNG)"
                .into(),
        });
    }
    // The admission runs on the header, before a pixel is decoded.
    let pixels = snapshot.pixels().ok_or_else(|| EngineError::Input {
        path: input.to_path_buf(),
        reason: "the source has an unusable size".into(),
    })?;
    if pixels > source::MAX_SOURCE_PIXELS {
        return Err(EngineError::Input {
            path: input.to_path_buf(),
            reason: format!(
                "the source has {pixels} pixels, above the limit of {} pixels",
                source::MAX_SOURCE_PIXELS
            ),
        });
    }
    // The size the decode must give, read before the bytes move into the
    // loader.
    let oriented = snapshot.oriented_size();
    cancel.check()?;
    // The shared loader: the same decode and the same orientation the
    // export uses, with the limits of the header.
    let loaded = super::load_bytes(
        snapshot.bytes,
        input,
        Some(source::limits(snapshot.width, snapshot.height)),
    )?;
    cancel.check()?;
    let (width, height) = (loaded.image.width(), loaded.image.height());
    if (width, height) != oriented {
        return Err(EngineError::Input {
            path: input.to_path_buf(),
            reason: "the decoded source does not match its header".into(),
        });
    }
    // A transparent picture is flattened onto white, like a JPEG or a BMP
    // export does.
    let picture = DynamicImage::ImageRgb8(super::encode::flatten(&loaded.image));
    let bytes = super::encode::encode(&picture, Target::Png, 90, false)?;
    cancel.check()?;
    std::fs::write(out, &bytes)
        .map_err(|e| EngineError::Failed(format!("cannot write {}: {e}", out.display())))?;
    Ok(Prepared {
        path: out.to_path_buf(),
        width,
        height,
    })
}

/// True when a PNG byte stream declares an APNG animation control (`acTL`)
/// chunk before the first `IDAT` chunk.
fn is_apng(bytes: &[u8]) -> bool {
    const PNG_SIG: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if bytes.len() < 8 || &bytes[..8] != PNG_SIG {
        return false;
    }
    let mut pos = 8usize;
    while let Some(header) = pos.checked_add(8).and_then(|end| bytes.get(pos..end)) {
        let len = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as usize;
        let kind = &header[4..8];
        if kind == b"acTL" {
            return true;
        }
        if kind == b"IDAT" || kind == b"IEND" {
            return false;
        }
        let Some(next) = pos.checked_add(12).and_then(|p| p.checked_add(len)) else {
            return false;
        };
        pos = next;
    }
    false
}

/// The name of a format, for the refusal of an unsupported one.
fn format_name(format: ImageFormat) -> &'static str {
    match format {
        ImageFormat::Jpeg => "JPEG",
        ImageFormat::Png => "PNG",
        ImageFormat::Bmp => "BMP",
        ImageFormat::Gif => "GIF",
        ImageFormat::Tiff => "TIFF",
        ImageFormat::WebP => "WebP",
        ImageFormat::Avif => "AVIF",
        ImageFormat::Tga => "TGA",
        ImageFormat::Ico => "ICO",
        ImageFormat::Qoi => "QOI",
        ImageFormat::Dds => "DDS",
        ImageFormat::Hdr => "HDR",
        ImageFormat::Farbfeld => "Farbfeld",
        ImageFormat::OpenExr => "OpenEXR",
        _ => "this format",
    }
}

/// A source failure as an engine failure of the same source.
fn map_error(input: &Path, error: kawaro_core::image_preview::ImagePreviewError) -> EngineError {
    match error {
        kawaro_core::image_preview::ImagePreviewError::Cancelled => EngineError::Cancelled,
        kawaro_core::image_preview::ImagePreviewError::SourceChanged => EngineError::Input {
            path: input.to_path_buf(),
            reason: "the source changed while it was read".into(),
        },
        kawaro_core::image_preview::ImagePreviewError::PolicyLimit => EngineError::Input {
            path: input.to_path_buf(),
            reason: format!(
                "the source is above the OCR limits ({} bytes or {} pixels)",
                source::MAX_INPUT_BYTES,
                source::MAX_SOURCE_PIXELS
            ),
        },
        kawaro_core::image_preview::ImagePreviewError::Unsupported(reason) => EngineError::Input {
            path: input.to_path_buf(),
            reason,
        },
        kawaro_core::image_preview::ImagePreviewError::Failed(reason) => EngineError::Input {
            path: input.to_path_buf(),
            reason,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The formats the OCR reads, and the ones it refuses.
    #[test]
    fn s146_only_three_formats_are_read() {
        assert!(SUPPORTED_FORMATS.contains(&ImageFormat::Jpeg));
        assert!(SUPPORTED_FORMATS.contains(&ImageFormat::Png));
        assert!(SUPPORTED_FORMATS.contains(&ImageFormat::Bmp));
        for refused in [
            ImageFormat::Tiff,
            ImageFormat::Gif,
            ImageFormat::WebP,
            ImageFormat::Avif,
            ImageFormat::Ico,
        ] {
            assert!(!SUPPORTED_FORMATS.contains(&refused), "{refused:?}");
        }
    }

    /// The format names of the refusal message.
    #[test]
    fn s146_the_format_names_read_as_expected() {
        assert_eq!(format_name(ImageFormat::Tiff), "TIFF");
        assert_eq!(format_name(ImageFormat::WebP), "WebP");
        assert_eq!(format_name(ImageFormat::Jpeg), "JPEG");
    }

    #[test]
    fn s146_animated_png_is_detected_by_actl_chunk_before_idat() {
        fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
            let mut out = Vec::with_capacity(12 + data.len());
            out.extend_from_slice(&(data.len() as u32).to_be_bytes());
            out.extend_from_slice(kind);
            out.extend_from_slice(data);
            out.extend_from_slice(&[0, 0, 0, 0]);
            out
        }
        let mut still = b"\x89PNG\r\n\x1a\n".to_vec();
        still.extend_from_slice(&chunk(b"IHDR", &[0; 13]));
        still.extend_from_slice(&chunk(b"IDAT", &[1, 2]));
        assert!(!is_apng(&still));

        let mut apng = b"\x89PNG\r\n\x1a\n".to_vec();
        apng.extend_from_slice(&chunk(b"IHDR", &[0; 13]));
        apng.extend_from_slice(&chunk(b"acTL", &[0; 8]));
        apng.extend_from_slice(&chunk(b"IDAT", &[1, 2]));
        assert!(is_apng(&apng));
    }
}
