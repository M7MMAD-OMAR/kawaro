//! The shared preparation of the image engine (S-142).
//!
//! The export and the preview run the same code here: the operation is
//! parsed once, the source is decoded and oriented once, the operation
//! is applied once, the bytes are encoded once and the metadata is
//! injected once. The export writes the result to the temporary path the
//! queue gave; the preview stops before the write and hands the bytes
//! back in memory.
//!
//! The two modes of the preview are different on purpose. The encoded
//! preview encodes the result, like the export does, and reports the
//! length of those bytes. The transformed draft never encodes: it decodes
//! a bounded proxy of the source, applies the operation to that proxy and
//! reports the geometry of the operation on the oriented original. The
//! draft makes no claim about bytes, quality or codec artifacts.
//!
//! Nothing here writes a file except the export path in `image.rs`.

use super::{
    DEFAULT_COMPRESS_QUALITY, DEFAULT_QUALITY, ImageEngine, Loaded, Op, Target, encode_to_target,
    keep_metadata, load_bytes, quality, single_input, source,
};
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, imageops::FilterType};
use kawaro_core::image_geom::Crop;
use kawaro_core::image_preview::{
    DisplayPixels, ImagePreviewArtifact, ImagePreviewError, ImagePreviewMode, ImagePreviewProvider,
    ImagePreviewRequest,
};
use kawaro_core::media::extension;
use kawaro_core::{ActionDef, CancelToken, EngineError, OutputMode, Task};
use std::io::Cursor;
use std::path::{Path, PathBuf};

/// One prepared image: the transformed pixels and everything the encoder
/// needs, so the export and the preview encode the same bytes under the
/// same input and the same options.
pub(super) struct Prepared {
    /// The transformed pixels, already oriented.
    image: DynamicImage,
    /// The output format.
    target: Target,
    /// The lossy quality.
    quality: u8,
    /// True when the source metadata is copied into the output.
    keep_metadata: bool,
    /// The raw source bytes, for the metadata copy.
    source_bytes: Vec<u8>,
    /// The byte target of `compress-to`, when the operation has one.
    target_bytes: Option<u64>,
    /// True for `compress`, which asks for the strongest PNG compression.
    best: bool,
}

/// Prepares one image for the encoder.
///
/// The target is given by the caller, because the export takes it from the
/// output path and the preview takes it from the action and the source.
/// The decoded source is given by the caller too, because the export reads
/// the path and the preview reads one bounded snapshot of it. Everything
/// else is the same for both.
pub(super) fn prepare(
    task: &Task,
    op: Op,
    target: Target,
    decoded: Loaded,
    cancel: &CancelToken,
) -> Result<Prepared, EngineError> {
    // Only the lossy formats can trade quality for a byte target.
    if matches!(op, Op::CompressTo(_))
        && !matches!(target, Target::Jpeg | Target::Webp | Target::Avif)
    {
        return Err(EngineError::UnsupportedOp(format!(
            "compress-to needs a lossy target: {target:?} has no quality control"
        )));
    }
    let default = if op == Op::Compress {
        DEFAULT_COMPRESS_QUALITY
    } else {
        DEFAULT_QUALITY
    };
    let quality = quality(task, default)?;
    cancel.check()?;

    single_input(task)?;
    let Loaded {
        image,
        bytes,
        format,
        ..
    } = decoded;
    cancel.check()?;

    // S-129: the metadata copy needs a JPEG on both sides and an op that
    // re-encodes the pixels.
    let keep = keep_metadata(task)?
        && matches!(
            op,
            Op::Convert
                | Op::Resize(_)
                | Op::ResizeBox { .. }
                | Op::Crop { .. }
                | Op::Compress
                | Op::CompressTo(_)
        )
        && format == ImageFormat::Jpeg
        && target == Target::Jpeg;
    let image = op.apply(image);
    Ok(Prepared {
        image,
        target,
        quality,
        keep_metadata: keep,
        source_bytes: bytes,
        target_bytes: match op {
            Op::CompressTo(target_bytes) => Some(target_bytes),
            _ => None,
        },
        best: op == Op::Compress,
    })
}

/// Encodes the prepared pixels.
///
/// `compress-to` runs the same bounded search as the export, with the same
/// stages and the same cancel checks.
pub(super) fn encode(prepared: &Prepared, cancel: &CancelToken) -> Result<Vec<u8>, EngineError> {
    match prepared.target_bytes {
        Some(target_bytes) => {
            encode_to_target(&prepared.image, prepared.target, target_bytes, cancel)
        }
        None => {
            cancel.check()?;
            super::encode::encode(
                &prepared.image,
                prepared.target,
                prepared.quality,
                prepared.best,
            )
        }
    }
}

/// Puts the source metadata into the encoded bytes, in memory.
///
/// The result is the final content of the file, so the caller writes it
/// once and a failed injection leaves nothing behind (S-129).
pub(super) fn finalize(prepared: &Prepared, encoded: Vec<u8>) -> Result<Vec<u8>, EngineError> {
    if !prepared.keep_metadata {
        return Ok(encoded);
    }
    let segments = super::metadata::extract(&prepared.source_bytes).unwrap_or_default();
    if segments.is_empty() {
        return Ok(encoded);
    }
    let Some(with_metadata) = super::metadata::inject(&encoded, &segments) else {
        return Err(EngineError::Failed("cannot copy the JPEG metadata".into()));
    };
    Ok(with_metadata)
}

/// The image engine as a preview provider (S-142).
impl ImagePreviewProvider for ImageEngine {
    fn preview(
        &self,
        request: &ImagePreviewRequest,
        cancel: &CancelToken,
    ) -> Result<ImagePreviewArtifact, ImagePreviewError> {
        dispatch(request, cancel)
    }
}

/// One preview request, checked and dispatched.
fn dispatch(
    request: &ImagePreviewRequest,
    cancel: &CancelToken,
) -> Result<ImagePreviewArtifact, ImagePreviewError> {
    cancel.check().map_err(engine_error)?;
    let action = &request.resolved_action;
    if action.engine != "image" {
        return Err(ImagePreviewError::Unsupported(
            "only image-engine actions have an image preview".into(),
        ));
    }
    if action.op == "to-pdf" {
        return Err(ImagePreviewError::Unsupported(
            "images to PDF is a document, not a picture".into(),
        ));
    }
    if action.output.mode == OutputMode::Combined {
        return Err(ImagePreviewError::Unsupported(
            "a combined output has no single picture".into(),
        ));
    }
    match request.mode {
        ImagePreviewMode::TransformedDraft => transformed_draft(request, cancel),
        ImagePreviewMode::EncodedResult => encoded_preview(request, cancel),
    }
}

/// The transformed draft: a bounded proxy of the result, without an
/// encode.
///
/// The geometry comes from the oriented original, through the same
/// `image_geom` calls the export uses, so the readout is the size the
/// operation produces. The picture is a proxy of the source: it is exact
/// for a source that fits the proxy and approximate for a larger one. No
/// byte count, no quality claim and no codec artifact claim is made.
fn transformed_draft(
    request: &ImagePreviewRequest,
    cancel: &CancelToken,
) -> Result<ImagePreviewArtifact, ImagePreviewError> {
    let action = &request.resolved_action;
    let input = &request.input;
    let task = Task {
        action_id: action.id.clone(),
        op: action.op.clone(),
        params: action.params.clone(),
        inputs: vec![input.clone()],
        output: PathBuf::new(),
    };
    let op = Op::parse(&action.op, &task).map_err(engine_error)?;
    // A byte target is the result of a search over encodes. It cannot be
    // inferred from a proxy, so the draft refuses it.
    if matches!(op, Op::CompressTo(_)) {
        return Err(ImagePreviewError::Unsupported(
            "a byte target needs the encoded result; ask for an encoded preview".into(),
        ));
    }
    let target = preview_target(action, input).map_err(engine_error)?;
    // An icon holds the picture at several sizes. Its geometry is not the
    // geometry of one picture, so a proxy cannot show it truthfully.
    if target == Target::Ico {
        return Err(ImagePreviewError::Unsupported(
            "an icon carries several sizes; ask for an encoded preview".into(),
        ));
    }
    cancel.check().map_err(engine_error)?;

    // One bounded snapshot of the source. Its header, its orientation and
    // its proxy all come from these bytes.
    let snapshot = source::acquire(input)?;
    cancel.check().map_err(engine_error)?;

    let (source_width, source_height) = snapshot.oriented_size();
    // The requested result, on the oriented original. The admission runs
    // before the operation is applied and before anything is allocated.
    let intended = op.geometry((source_width, source_height));
    admit_result(intended.width, intended.height)?;
    cancel.check().map_err(engine_error)?;

    let proxy = source::proxy(&snapshot)?;
    cancel.check().map_err(engine_error)?;

    // The crop of the operation, in the coordinates of the oriented
    // original, mapped into the proxy.
    let cropped = match op.crop_window((source_width, source_height)) {
        Some(window) => crop_proxy(&proxy, window, (source_width, source_height)),
        None => proxy,
    };
    // The operations whose pixels do not depend on the absolute size are
    // the existing transforms.
    let transformed = match op {
        Op::Rotate(_) | Op::Flip(_) | Op::Grayscale => op.apply(cropped),
        _ => cropped,
    };
    cancel.check().map_err(engine_error)?;

    // The result is shown at the size of the result, bounded for the
    // display. The absolute target of the operation is never allocated.
    let (display_width, display_height) = source::display_size(intended.width, intended.height);
    let scaled = transformed.resize_exact(display_width, display_height, FilterType::Lanczos3);
    cancel.check().map_err(engine_error)?;

    // A JPEG and a BMP flatten a transparent picture onto white. The same
    // helper the encoder uses gives the same picture.
    let rgba = match target {
        Target::Jpeg | Target::Bmp => {
            let rgb = if super::encode::is_gray(scaled.color()) {
                DynamicImage::ImageLuma8(super::encode::gray_on_white(&scaled)).to_rgb8()
            } else {
                super::encode::flatten(&scaled)
            };
            DynamicImage::ImageRgb8(rgb).to_rgba8()
        }
        _ => scaled.to_rgba8(),
    };
    let expected = u64::from(rgba.width())
        .checked_mul(u64::from(rgba.height()))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| ImagePreviewError::Failed("the display buffer is too large".into()))?;
    if expected != rgba.as_raw().len() as u64 {
        return Err(ImagePreviewError::Failed(
            "the display buffer does not match its size".into(),
        ));
    }
    Ok(ImagePreviewArtifact {
        stamp: request.stamp,
        pixels: DisplayPixels {
            width: rgba.width(),
            height: rgba.height(),
            rgba8: rgba.into_raw(),
        },
        output_width: intended.width,
        output_height: intended.height,
        output_format: target.ext().to_string(),
        // A draft is not an encoded artifact.
        final_encoded_bytes: None,
        mode: ImagePreviewMode::TransformedDraft,
    })
}

/// The encoded preview: the same bytes the export writes, without a file.
///
/// The bytes are the bytes of the export under the same input, the same
/// options and the same build. A later run may differ, because the input,
/// the options, the codec or the metadata may have changed.
fn encoded_preview(
    request: &ImagePreviewRequest,
    cancel: &CancelToken,
) -> Result<ImagePreviewArtifact, ImagePreviewError> {
    let action = &request.resolved_action;
    let input = &request.input;
    let task = Task {
        action_id: action.id.clone(),
        op: action.op.clone(),
        params: action.params.clone(),
        inputs: vec![input.clone()],
        output: PathBuf::new(),
    };
    let op = Op::parse(&action.op, &task).map_err(engine_error)?;
    let target = preview_target(action, input).map_err(engine_error)?;
    cancel.check().map_err(engine_error)?;

    // One bounded snapshot of the source. The header, the decode and the
    // orientation all use these bytes, so a file that changed after the
    // header cannot be decoded instead of the admitted one.
    let snapshot = source::acquire(input)?;
    cancel.check().map_err(engine_error)?;

    // The admission of the source: it is decoded at full size here.
    let pixels = snapshot.pixels().ok_or(ImagePreviewError::PolicyLimit)?;
    if pixels > source::MAX_SOURCE_PIXELS {
        return Err(ImagePreviewError::PolicyLimit);
    }
    let (source_width, source_height) = snapshot.oriented_size();
    // The admission of the requested result, before the operation is
    // applied and before anything is allocated.
    let intended = op.geometry((source_width, source_height));
    admit_result(intended.width, intended.height)?;
    cancel.check().map_err(engine_error)?;

    let source::Snapshot {
        bytes,
        width,
        height,
        ..
    } = snapshot;
    // The snapshot bytes, decoded with the loader the export uses, with
    // the limits of the admitted size.
    let loaded =
        load_bytes(bytes, input, Some(source::limits(width, height))).map_err(engine_error)?;
    cancel.check().map_err(engine_error)?;
    // The decoded picture must be the picture the header described.
    if (loaded.image.width(), loaded.image.height()) != (source_width, source_height) {
        return Err(ImagePreviewError::Failed(
            "the decoded source does not match its header".into(),
        ));
    }

    let prepared = prepare(&task, op, target, loaded, cancel).map_err(engine_error)?;
    cancel.check().map_err(engine_error)?;

    let encoded = encode(&prepared, cancel).map_err(engine_error)?;
    cancel.check().map_err(engine_error)?;
    let final_bytes = finalize(&prepared, encoded).map_err(engine_error)?;
    cancel.check().map_err(engine_error)?;

    // The size of the artifact, read from its own header, and its
    // admission before it is decoded back for the display. A header that
    // cannot be read is a failure, never a guessed size.
    let (output_width, output_height) = admit_artifact(&final_bytes)?;
    let final_len = final_bytes.len() as u64;
    // The full pixels and the source bytes are not needed any more.
    drop(prepared);

    let pixels = display_pixels(&final_bytes)?;
    drop(final_bytes);

    Ok(ImagePreviewArtifact {
        stamp: request.stamp,
        pixels,
        output_width,
        output_height,
        output_format: target.ext().to_string(),
        final_encoded_bytes: Some(final_len),
        mode: request.mode,
    })
}

/// The admission of a requested result, in pixels.
///
/// The limit is a preview policy. The export has none and keeps its
/// normal behaviour. The check runs before the operation is applied and
/// before anything is allocated, so a small source cannot ask for a huge
/// result.
fn admit_result(width: u32, height: u32) -> Result<(), ImagePreviewError> {
    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .ok_or(ImagePreviewError::PolicyLimit)?;
    if pixels > source::MAX_RESULT_PIXELS {
        return Err(ImagePreviewError::PolicyLimit);
    }
    Ok(())
}

/// The admission of the encoded artifact, from its own header.
///
/// The header of the artifact is the truth about the bytes. It is read
/// before the artifact is decoded back for the display.
fn admit_artifact(bytes: &[u8]) -> Result<(u32, u32), ImagePreviewError> {
    let (width, height) = artifact_size(bytes).ok_or_else(|| {
        ImagePreviewError::Failed("cannot read the header of the encoded artifact".into())
    })?;
    admit_result(width, height)?;
    Ok((width, height))
}

/// Maps a crop window of the oriented original into the proxy.
///
/// The proxy is a scaled copy of the original, so the window is scaled
/// with it and clamped into the proxy. The result is not claimed to be
/// pixel exact for a source larger than the proxy.
fn crop_proxy(proxy: &DynamicImage, window: Crop, source_size: (u32, u32)) -> DynamicImage {
    let (proxy_width, proxy_height) = (proxy.width(), proxy.height());
    let (source_width, source_height) = source_size;
    if proxy_width == 0 || proxy_height == 0 {
        return proxy.clone();
    }
    let x = map_axis(u64::from(window.x), source_width, proxy_width);
    let y = map_axis(u64::from(window.y), source_height, proxy_height);
    let right = map_edge(
        u64::from(window.x) + u64::from(window.width),
        source_width,
        proxy_width,
    );
    let bottom = map_edge(
        u64::from(window.y) + u64::from(window.height),
        source_height,
        proxy_height,
    );
    let width = right.saturating_sub(x).max(1).min(proxy_width - x);
    let height = bottom.saturating_sub(y).max(1).min(proxy_height - y);
    proxy.crop_imm(x, y, width, height)
}

/// The start of a window, in the coordinates of the proxy.
fn map_axis(value: u64, source: u32, proxy: u32) -> u32 {
    if source == 0 || proxy == 0 {
        return 0;
    }
    let mapped = value.saturating_mul(u64::from(proxy)) / u64::from(source);
    mapped.min(u64::from(proxy - 1)) as u32
}

/// The end of a window, in the coordinates of the proxy.
///
/// The end is rounded up, so a window of the proxy keeps its width.
fn map_edge(value: u64, source: u32, proxy: u32) -> u32 {
    if source == 0 || proxy == 0 {
        return 0;
    }
    let mapped = value
        .saturating_mul(u64::from(proxy))
        .div_ceil(u64::from(source));
    mapped.min(u64::from(proxy)) as u32
}

/// The format the action writes: its own extension, or the extension of
/// the source when the action keeps the input format.
fn preview_target(action: &ActionDef, input: &Path) -> Result<Target, EngineError> {
    let ext = if action.output.ext.is_empty() {
        extension(input)
    } else {
        action.output.ext.to_ascii_lowercase()
    };
    Target::from_extension(&ext)
}

/// The size of the encoded artifact, read from its header.
fn artifact_size(bytes: &[u8]) -> Option<(u32, u32)> {
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    reader.into_dimensions().ok()
}

/// The display buffer: the encoded artifact decoded back, at most
/// `DISPLAY_MAX` on its longest side.
///
/// The artifact is decoded, not the transformed image, so the picture
/// carries what the encoder did, the white flattening of a JPEG for
/// example.
fn display_pixels(bytes: &[u8]) -> Result<DisplayPixels, ImagePreviewError> {
    let decoded = decode(bytes)?;
    let rgba = decoded.to_rgba8();
    drop(decoded);
    let (width, height) = source::display_size(rgba.width(), rgba.height());
    let small = if (width, height) == (rgba.width(), rgba.height()) {
        rgba
    } else {
        image::imageops::resize(&rgba, width, height, FilterType::Lanczos3)
    };
    let expected = u64::from(small.width())
        .checked_mul(u64::from(small.height()))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| ImagePreviewError::Failed("the display buffer is too large".into()))?;
    if expected != small.as_raw().len() as u64 {
        return Err(ImagePreviewError::Failed(
            "the display buffer does not match its size".into(),
        ));
    }
    Ok(DisplayPixels {
        width: small.width(),
        height: small.height(),
        rgba8: small.into_raw(),
    })
}

/// Decodes image bytes into pixels.
///
/// The encoded bytes already carry the oriented pixels, so the orientation
/// is not applied a second time.
fn decode(bytes: &[u8]) -> Result<DynamicImage, ImagePreviewError> {
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| ImagePreviewError::Failed(format!("cannot read the encoded image: {e}")))?;
    let mut decoder = reader
        .into_decoder()
        .map_err(|e| ImagePreviewError::Failed(format!("cannot read the encoded image: {e}")))?;
    let (width, height) = decoder.dimensions();
    decoder
        .set_limits(source::limits(width, height))
        .map_err(|_| ImagePreviewError::PolicyLimit)?;
    DynamicImage::from_decoder(decoder)
        .map_err(|e| ImagePreviewError::Failed(format!("cannot read the encoded image: {e}")))
}

/// Maps an engine failure to a preview failure.
fn engine_error(error: EngineError) -> ImagePreviewError {
    match error {
        EngineError::Cancelled => ImagePreviewError::Cancelled,
        EngineError::UnsupportedOp(op) => ImagePreviewError::Unsupported(op),
        EngineError::BadParam { name, reason } => {
            ImagePreviewError::Unsupported(format!("bad parameter `{name}`: {reason}"))
        }
        EngineError::Input { path, reason } => {
            ImagePreviewError::Failed(format!("cannot read {}: {reason}", path.display()))
        }
        EngineError::Unavailable(what) | EngineError::Failed(what) => {
            ImagePreviewError::Failed(what)
        }
        EngineError::Io(e) => ImagePreviewError::Failed(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_huge_requested_result_is_refused() {
        assert!(admit_result(200, 100).is_ok());
        assert!(admit_result(20_000, 20_000).is_err());
        assert!(admit_result(4_000, 4_001).is_err());
    }

    #[test]
    fn an_artifact_without_a_header_is_a_failure() {
        let error = admit_artifact(b"not an image").unwrap_err();
        assert!(matches!(error, ImagePreviewError::Failed(_)), "{error:?}");
    }

    #[test]
    fn a_crop_window_is_mapped_into_the_proxy() {
        // A window at the left of a source twice the size of the proxy.
        let window = Crop {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        };
        let proxy = DynamicImage::ImageRgb8(image::RgbImage::new(200, 100));
        let cropped = crop_proxy(&proxy, window, (400, 200));
        assert_eq!((cropped.width(), cropped.height()), (50, 50));
    }
}
