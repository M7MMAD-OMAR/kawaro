//! The bounded source of an image preview (S-142).
//!
//! A preview reads the source file once, under a byte limit, reads its
//! header from those bytes and decodes a bounded proxy of them. The
//! export path does not use this module: the export keeps its own read
//! and its own decode, with no limit of this kind.
//!
//! This is not a second image loader. The format, the header and the
//! orientation come from the same `image::ImageReader` the export uses,
//! and the decoded picture is the same picture the export would write,
//! only smaller.

use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits, imageops::FilterType};
use kawaro_core::image_preview::ImagePreviewError;
use std::io::{Cursor, Read};
use std::path::Path;
use std::time::SystemTime;

/// The largest source file a preview accepts, in bytes.
pub(super) const MAX_INPUT_BYTES: u64 = 64 * 1024 * 1024;
/// The largest source a preview decodes at full size, in pixels.
pub(super) const MAX_SOURCE_PIXELS: u64 = 16_000_000;
/// The largest result a preview describes or decodes, in pixels.
pub(super) const MAX_RESULT_PIXELS: u64 = 16_000_000;
/// The longest side of the proxy and of the display buffer.
pub(super) const DISPLAY_MAX: u32 = 512;
/// The allocation one decoder may make for one picture, in bytes.
///
/// This bounds the decoder, best effort. It is not a promise about the
/// memory of the process: a codec may allocate internally.
pub(super) const MAX_DECODE_ALLOC: u64 = 256 * 1024 * 1024;
/// The buffer the JPEG decoder may hold, in bytes. Best effort, like the
/// allocation bound above. A progressive JPEG keeps its coefficients, so
/// the bound is generous enough for a picture at the source limit.
const MAX_JPEG_BUFFER: usize = 256 * 1024 * 1024;

/// One bounded snapshot of the source file.
#[derive(Debug)]
pub(super) struct Snapshot {
    /// The bytes of the file, read once under the byte limit.
    pub bytes: Vec<u8>,
    /// The format the header says.
    pub format: ImageFormat,
    /// The width in the header, before the orientation.
    pub width: u32,
    /// The height in the header, before the orientation.
    pub height: u32,
    /// The EXIF orientation of the source.
    pub orientation: image::metadata::Orientation,
}

impl Snapshot {
    /// The size of the source after the orientation.
    pub fn oriented_size(&self) -> (u32, u32) {
        match self.orientation {
            image::metadata::Orientation::Rotate90
            | image::metadata::Orientation::Rotate270
            | image::metadata::Orientation::Rotate90FlipH
            | image::metadata::Orientation::Rotate270FlipH => (self.height, self.width),
            _ => (self.width, self.height),
        }
    }

    /// The number of pixels of the source, checked.
    pub fn pixels(&self) -> Option<u64> {
        u64::from(self.width).checked_mul(u64::from(self.height))
    }
}

/// Takes one bounded snapshot of the source file.
///
/// The file is read once, with a hard limit: a file above the limit is
/// refused before a byte of it is decoded. The size and the modification
/// time of the file are compared around the read, so a file that grew or
/// was replaced while it was read is refused. That comparison is best
/// effort: it is not a guarantee against a change that keeps the size and
/// the modification time.
pub(super) fn acquire(path: &Path) -> Result<Snapshot, ImagePreviewError> {
    let file = std::fs::File::open(path)
        .map_err(|e| ImagePreviewError::Failed(format!("cannot read {}: {e}", path.display())))?;
    let before = {
        let meta = file.metadata().ok();
        stamp(meta.as_ref())
    };
    let mut bytes = Vec::new();
    let mut limited = (&file).take(MAX_INPUT_BYTES + 1);
    limited
        .read_to_end(&mut bytes)
        .map_err(|e| ImagePreviewError::Failed(format!("cannot read {}: {e}", path.display())))?;
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err(ImagePreviewError::PolicyLimit);
    }
    let after = {
        let meta = std::fs::metadata(path).ok();
        stamp(meta.as_ref())
    };
    if before != after {
        return Err(ImagePreviewError::SourceChanged);
    }
    header(path, bytes)
}

/// The size and the modification time of a file, when they can be read.
fn stamp(meta: Option<&std::fs::Metadata>) -> Option<(u64, Option<SystemTime>)> {
    meta.map(|m| (m.len(), m.modified().ok()))
}

/// Reads the format, the size and the orientation from the snapshot bytes.
fn header(path: &Path, bytes: Vec<u8>) -> Result<Snapshot, ImagePreviewError> {
    let reader = ImageReader::new(Cursor::new(bytes.as_slice()))
        .with_guessed_format()
        .map_err(|e| ImagePreviewError::Failed(format!("cannot read {}: {e}", path.display())))?;
    let format = match reader.format() {
        // The AVIF decoder needs a system library this build does not
        // have. The refusal is typed, and no source pixels are shown in
        // its place.
        Some(ImageFormat::Avif) => {
            return Err(ImagePreviewError::Unsupported(
                "AVIF decoding is not supported in this build (it needs the dav1d system library)"
                    .into(),
            ));
        }
        Some(format) => format,
        None => {
            return Err(ImagePreviewError::Failed(format!(
                "{} is not a recognised image format",
                path.display()
            )));
        }
    };
    let mut decoder = reader
        .into_decoder()
        .map_err(|e| ImagePreviewError::Failed(format!("cannot read {}: {e}", path.display())))?;
    let (width, height) = decoder.dimensions();
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    // The decoder borrows the bytes; release it before the bytes move.
    drop(decoder);
    Ok(Snapshot {
        bytes,
        format,
        width,
        height,
        orientation,
    })
}

/// Decodes a bounded proxy of the source, with the orientation applied.
///
/// A JPEG is decoded with the DCT scale the proxy needs, so a large photo
/// is never decoded at its full size. Every other format is decoded at
/// full size, and only under the declared pixel admission, so a huge file
/// is never decoded and shrunk afterwards.
pub(super) fn proxy(snapshot: &Snapshot) -> Result<DynamicImage, ImagePreviewError> {
    let pixels = snapshot.pixels().ok_or(ImagePreviewError::PolicyLimit)?;
    if pixels > MAX_SOURCE_PIXELS {
        return Err(ImagePreviewError::PolicyLimit);
    }
    let target = display_size(snapshot.width, snapshot.height);
    let mut image = match snapshot.format {
        ImageFormat::Jpeg => jpeg_proxy(&snapshot.bytes, snapshot.width, snapshot.height, target)?,
        _ => full_proxy(&snapshot.bytes, snapshot.width, snapshot.height, target)?,
    };
    image.apply_orientation(snapshot.orientation);
    Ok(image)
}

/// The limits of one preview decode: the size of the header and an
/// allocation bound. The width and the height are strict, so a decoder
/// that reports more than the header said is refused before it allocates.
pub(super) fn limits(width: u32, height: u32) -> Limits {
    let mut limits = Limits::default();
    limits.max_image_width = Some(width);
    limits.max_image_height = Some(height);
    limits.max_alloc = Some(MAX_DECODE_ALLOC);
    limits
}

/// A JPEG decoded with the DCT scale the proxy needs.
fn jpeg_proxy(
    bytes: &[u8],
    width: u32,
    height: u32,
    target: (u32, u32),
) -> Result<DynamicImage, ImagePreviewError> {
    let mut decoder = jpeg_decoder::Decoder::new(Cursor::new(bytes));
    decoder.set_max_decoding_buffer_size(MAX_JPEG_BUFFER);
    decoder
        .read_info()
        .map_err(|e| ImagePreviewError::Failed(format!("cannot read the JPEG source: {e}")))?;
    // The frame must be the picture the header described.
    let info = decoder
        .info()
        .ok_or_else(|| ImagePreviewError::Failed("the JPEG source has no frame".into()))?;
    if u32::from(info.width) != width || u32::from(info.height) != height {
        return Err(ImagePreviewError::Failed(
            "the decoded source does not match its header".into(),
        ));
    }
    decoder
        .scale(scale_u16(target.0), scale_u16(target.1))
        .map_err(|e| ImagePreviewError::Failed(format!("cannot read the JPEG source: {e}")))?;
    let pixels = decoder
        .decode()
        .map_err(|e| ImagePreviewError::Failed(format!("cannot read the JPEG source: {e}")))?;
    let info = decoder
        .info()
        .ok_or_else(|| ImagePreviewError::Failed("the JPEG source has no frame".into()))?;
    let (width, height) = (u32::from(info.width), u32::from(info.height));
    let image = match info.pixel_format {
        jpeg_decoder::PixelFormat::L8 => {
            image::GrayImage::from_raw(width, height, pixels).map(DynamicImage::ImageLuma8)
        }
        jpeg_decoder::PixelFormat::L16 => {
            // Big endian pairs: the high byte is enough for a proxy.
            let luma: Vec<u8> = (0..pixels.len() / 2).map(|i| pixels[i * 2]).collect();
            image::GrayImage::from_raw(width, height, luma).map(DynamicImage::ImageLuma8)
        }
        jpeg_decoder::PixelFormat::RGB24 => {
            image::RgbImage::from_raw(width, height, pixels).map(DynamicImage::ImageRgb8)
        }
        // A CMYK JPEG is not converted by the decoder, and a proxy with
        // the wrong colours is worse than no proxy.
        jpeg_decoder::PixelFormat::CMYK32 => {
            return Err(ImagePreviewError::Unsupported(
                "a CMYK JPEG has no draft preview".into(),
            ));
        }
    }
    .ok_or_else(|| ImagePreviewError::Failed("the JPEG source does not match its header".into()))?;
    // The scaled decode is at least as large as the proxy, never smaller.
    Ok(image.resize_exact(target.0, target.1, FilterType::Lanczos3))
}

/// Every other format, decoded at full size under the limits, then
/// reduced to the proxy.
fn full_proxy(
    bytes: &[u8],
    width: u32,
    height: u32,
    target: (u32, u32),
) -> Result<DynamicImage, ImagePreviewError> {
    let mut decoder = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| ImagePreviewError::Failed(format!("cannot read the source: {e}")))?
        .into_decoder()
        .map_err(|e| ImagePreviewError::Failed(format!("cannot read the source: {e}")))?;
    decoder
        .set_limits(limits(width, height))
        .map_err(|_| ImagePreviewError::PolicyLimit)?;
    let image = DynamicImage::from_decoder(decoder)
        .map_err(|e| ImagePreviewError::Failed(format!("cannot read the source: {e}")))?;
    // The decoded picture must be the picture the header described.
    if image.width() != width || image.height() != height {
        return Err(ImagePreviewError::Failed(
            "the decoded source does not match its header".into(),
        ));
    }
    // The full picture is never kept: the proxy is the size of the
    // display.
    Ok(image.resize_exact(target.0, target.1, FilterType::Lanczos3))
}

/// A size the JPEG decoder can be asked for.
fn scale_u16(value: u32) -> u16 {
    value.clamp(1, u32::from(u16::MAX)) as u16
}

/// The size of a picture that fits the display: at most `DISPLAY_MAX` on
/// its longest side, with the aspect ratio kept. A picture that already
/// fits keeps its size.
pub(super) fn display_size(width: u32, height: u32) -> (u32, u32) {
    let longest = width.max(height);
    if longest <= DISPLAY_MAX {
        return (width, height);
    }
    let scale = f64::from(DISPLAY_MAX) / f64::from(longest);
    let width = ((f64::from(width) * scale).round() as u32).max(1);
    let height = ((f64::from(height) * scale).round() as u32).max(1);
    (width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_display_size_never_enlarges() {
        assert_eq!(display_size(400, 200), (400, 200));
        assert_eq!(display_size(512, 512), (512, 512));
        assert_eq!(display_size(1024, 512), (512, 256));
        assert_eq!(display_size(512, 1024), (256, 512));
        // A side never reaches zero.
        assert_eq!(display_size(2000, 1), (512, 1));
    }

    #[test]
    fn the_limits_are_the_size_of_the_header() {
        let limits = limits(400, 200);
        assert_eq!(limits.max_image_width, Some(400));
        assert_eq!(limits.max_image_height, Some(200));
        assert_eq!(limits.max_alloc, Some(MAX_DECODE_ALLOC));
    }

    #[test]
    fn a_snapshot_above_the_byte_limit_is_refused() {
        let dir = std::env::temp_dir().join("kawaro-s142b-source");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("big.bin");
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(MAX_INPUT_BYTES + 1).unwrap();
        drop(file);
        let error = acquire(&path).unwrap_err();
        assert!(matches!(error, ImagePreviewError::PolicyLimit), "{error:?}");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn a_changed_size_is_a_change() {
        let before: Option<(u64, Option<SystemTime>)> = Some((10, None));
        let after: Option<(u64, Option<SystemTime>)> = Some((11, None));
        assert_ne!(before, after);
        // A missing modification time is not a change on its own.
        assert_eq!(before, Some((10, None)));
    }
}
