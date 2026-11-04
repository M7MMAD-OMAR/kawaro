//! Image engine (S-004). Pure Rust decoding and encoding through the `image`
//! crate, lossy WebP through libwebp, PDF through `pdf-writer`. The op table
//! is in `docs/architecture.md`, dependency notes in `docs/engines.md`.
//!
//! Every op decodes the input, applies its EXIF orientation, transforms the
//! pixels and encodes into the format named by the extension of
//! `task.output`. Output bytes are built in memory and written once at the
//! end, so a failure or a cancel never leaves a partial file behind.

mod encode;
mod metadata;
mod pdf;

use encode::Target;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, imageops::FilterType};
use kawaro_core::engine::TaskOutcome;
use kawaro_core::image_geom::{self, Anchor, Mode};
use kawaro_core::{CancelToken, Engine, EngineError, MediaKind, Progress, Task};
use std::io::Cursor;
use std::path::{Path, PathBuf};

/// Operations in the order the hive lists them.
const OPS: [&str; 11] = [
    "convert",
    "resize",
    "rotate",
    "flip",
    "grayscale",
    "compress",
    "strip",
    "to-pdf",
    "resize-box",
    "crop",
    "compress-to",
];

/// Lossy quality when the action does not set one.
const DEFAULT_QUALITY: u8 = 90;
/// Lossy quality for `compress` when the action does not set one.
const DEFAULT_COMPRESS_QUALITY: u8 = 75;
/// Largest accepted `width` or `height` of `resize-box`.
const BOX_MAX: i64 = 20_000;
/// Largest accepted `target_bytes` of `compress-to`.
const TARGET_BYTES_MAX: i64 = 20_000_000;
/// Most encodes the `compress-to` quality search may spend on one size.
const TARGET_ENCODES: usize = 6;
/// The fixed downscale step of a byte target (S-132): 4/5.
const TARGET_STEP_NUM: u64 = 4;
const TARGET_STEP_DEN: u64 = 5;
/// The smallest side the target fallback shrinks to (S-132).
const TARGET_MIN_SIDE: u32 = 64;
/// The most downscale rounds one target search may take (S-132).
const TARGET_ROUNDS: u32 = 16;

/// `non_exhaustive` keeps room for settings and lets `lib.rs` build it with
/// `ImageEngine::default()` without a clippy unit struct warning.
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub struct ImageEngine;

impl Engine for ImageEngine {
    fn id(&self) -> &'static str {
        "image"
    }

    fn capabilities(&self) -> Vec<(&'static str, Vec<MediaKind>)> {
        OPS.iter().map(|op| (*op, vec![MediaKind::Image])).collect()
    }

    fn run(
        &self,
        task: &Task,
        progress: Progress,
        cancel: &CancelToken,
    ) -> Result<TaskOutcome, EngineError> {
        progress(0.0);
        cancel.check()?;
        match task.op.as_str() {
            "to-pdf" => pdf::run(task, progress, cancel)?,
            op => {
                let op = Op::parse(op, task)?;
                run_single(task, op, progress, cancel)?;
            }
        }
        Ok(TaskOutcome::Written)
    }

    /// The final and source sizes of a finished `compress-to` (S-132), read
    /// from the output header and the first input, never from state.
    fn result_detail(&self, task: &Task) -> Option<kawaro_core::engine::TaskDetail> {
        if task.op != "compress-to" {
            return None;
        }
        let (width, height) = image_dimensions(&task.output)?;
        let bytes = std::fs::metadata(&task.output).ok()?.len();
        let (source_width, source_height) = image_dimensions(task.inputs.first()?)?;
        Some(kawaro_core::engine::TaskDetail {
            width,
            height,
            source_width,
            source_height,
            bytes,
            downscaled: width < source_width || height < source_height,
        })
    }
}

/// The pixel size from a file header, without decoding the pixels.
fn image_dimensions(path: &Path) -> Option<(u32, u32)> {
    image::ImageReader::open(path).ok()?.into_dimensions().ok()
}

/// A per-file operation with its validated parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Op {
    Convert,
    Resize(Size),
    ResizeBox {
        width: u32,
        height: u32,
        mode: BoxFit,
    },
    Crop {
        aspect: Aspect,
        anchor: Anchor,
    },
    Rotate(u32),
    Flip(Axis),
    Grayscale,
    Compress,
    CompressTo(u64),
    Strip,
}

/// How a `resize-box` target relates to its box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BoxFit {
    /// Contain in the box, never enlarging.
    Fit,
    /// Cover the box, centre cropping the source first.
    Fill,
    /// Exactly the box, enlarging and distorting when needed.
    Exact,
}

/// The aspect presets of `crop`; `original` keeps the whole image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Aspect {
    Original,
    Square,
    FourThree,
    SixteenNine,
    ThreeTwo,
}

impl Aspect {
    /// The `width:height` ratio, `None` for the whole image.
    fn ratio(self) -> Option<(u32, u32)> {
        match self {
            Aspect::Original => None,
            Aspect::Square => Some((1, 1)),
            Aspect::FourThree => Some((4, 3)),
            Aspect::SixteenNine => Some((16, 9)),
            Aspect::ThreeTwo => Some((3, 2)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Size {
    /// Percent of the original size.
    Scale(f64),
    /// Longest side in pixels, never enlarging.
    MaxSide(u32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Axis {
    Horizontal,
    Vertical,
}

fn bad(name: &str, reason: impl Into<String>) -> EngineError {
    EngineError::BadParam {
        name: name.to_string(),
        reason: reason.into(),
    }
}

impl Op {
    fn parse(op: &str, task: &Task) -> Result<Self, EngineError> {
        Ok(match op {
            "convert" => Op::Convert,
            "resize" => Op::Resize(parse_size(task)?),
            "resize-box" => Op::ResizeBox {
                width: box_side(task, "width")?,
                height: box_side(task, "height")?,
                mode: box_mode(task)?,
            },
            "crop" => Op::Crop {
                aspect: aspect_param(task)?,
                anchor: anchor_param(task)?,
            },
            "compress-to" => Op::CompressTo(target_bytes(task)?),
            "rotate" => match task.param_i64("degrees") {
                Some(d @ (90 | 180 | 270)) => Op::Rotate(d as u32),
                _ => return Err(bad("degrees", "must be 90, 180 or 270")),
            },
            "flip" => match task.param_str("axis") {
                Some("horizontal") => Op::Flip(Axis::Horizontal),
                Some("vertical") => Op::Flip(Axis::Vertical),
                _ => return Err(bad("axis", "must be \"horizontal\" or \"vertical\"")),
            },
            "grayscale" => Op::Grayscale,
            "compress" => Op::Compress,
            "strip" => Op::Strip,
            other => return Err(EngineError::UnsupportedOp(other.to_string())),
        })
    }

    fn apply(self, img: DynamicImage) -> DynamicImage {
        match self {
            Op::Convert | Op::Compress | Op::CompressTo(_) | Op::Strip => img,
            Op::ResizeBox {
                width,
                height,
                mode,
            } => resize_box(img, width, height, mode),
            Op::Crop { aspect, anchor } => crop(img, aspect, anchor),
            Op::Resize(size) => {
                let (w, h) = size.target(img.width(), img.height());
                if (w, h) == (img.width(), img.height()) {
                    img
                } else {
                    img.resize_exact(w, h, FilterType::Lanczos3)
                }
            }
            Op::Rotate(90) => img.rotate90(),
            Op::Rotate(180) => img.rotate180(),
            Op::Rotate(_) => img.rotate270(),
            Op::Flip(Axis::Horizontal) => img.fliph(),
            Op::Flip(Axis::Vertical) => img.flipv(),
            Op::Grayscale => {
                if img.color().has_alpha() {
                    DynamicImage::ImageLumaA8(img.to_luma_alpha8())
                } else {
                    DynamicImage::ImageLuma8(img.to_luma8())
                }
            }
        }
    }
}

fn parse_size(task: &Task) -> Result<Size, EngineError> {
    if task.params.contains_key("scale") {
        match task.param_f64("scale") {
            Some(s) if (1.0..=1000.0).contains(&s) => Ok(Size::Scale(s)),
            _ => Err(bad("scale", "must be a percentage from 1 to 1000")),
        }
    } else if task.params.contains_key("max_side") {
        match task.param_i64("max_side") {
            Some(m) if (1..=i64::from(u32::MAX)).contains(&m) => Ok(Size::MaxSide(m as u32)),
            _ => Err(bad(
                "max_side",
                "must be a whole number of pixels, 1 or more",
            )),
        }
    } else {
        Err(bad("scale", "set `scale` (percent) or `max_side` (pixels)"))
    }
}

impl Size {
    /// New dimensions keeping the aspect ratio, at least one pixel each.
    /// The arithmetic is `kawaro_core::image_geom`'s (S-118), so the stage 2
    /// preview and the file this engine writes are never a pixel apart.
    fn target(self, w: u32, h: u32) -> (u32, u32) {
        let target = match self {
            Size::Scale(percent) if percent.fract() == 0.0 => {
                image_geom::target((w, h), Mode::Percent(percent as u32))
            }
            Size::MaxSide(max) => image_geom::target((w, h), Mode::LongestSide(max)),
            // The option schema's percent field is whole. A hand written
            // action file may still carry a fraction, and keeps the dimensions
            // the old float arithmetic gave it.
            Size::Scale(percent) => image_geom::target_percent((w, h), percent),
        };
        match target {
            Some(t) => (t.width, t.height),
            None => (w, h),
        }
    }
}

/// `width` or `height` of `resize-box`: 1 to 20000 pixels.
fn box_side(task: &Task, name: &str) -> Result<u32, EngineError> {
    match task.param_i64(name) {
        Some(side) if (1..=BOX_MAX).contains(&side) => Ok(side as u32),
        _ => Err(bad(
            name,
            format!("must be a whole number of pixels from 1 to {BOX_MAX}"),
        )),
    }
}

/// The `mode` of `resize-box`, `fit` when it is absent.
fn box_mode(task: &Task) -> Result<BoxFit, EngineError> {
    match task.params.get("mode") {
        None => Ok(BoxFit::Fit),
        Some(value) => match value.as_str() {
            Some("fit") => Ok(BoxFit::Fit),
            Some("fill") => Ok(BoxFit::Fill),
            Some("exact") => Ok(BoxFit::Exact),
            _ => Err(bad("mode", "must be fit, fill or exact")),
        },
    }
}

/// The `aspect` of `crop`, the whole image when it is absent.
fn aspect_param(task: &Task) -> Result<Aspect, EngineError> {
    match task.params.get("aspect") {
        None => Ok(Aspect::Original),
        Some(value) => match value.as_str() {
            Some("original") => Ok(Aspect::Original),
            Some("1:1") => Ok(Aspect::Square),
            Some("4:3") => Ok(Aspect::FourThree),
            Some("16:9") => Ok(Aspect::SixteenNine),
            Some("3:2") => Ok(Aspect::ThreeTwo),
            _ => Err(bad("aspect", "must be original, 1:1, 4:3, 16:9 or 3:2")),
        },
    }
}

/// The `anchor` of `crop`, the centre when it is absent.
fn anchor_param(task: &Task) -> Result<Anchor, EngineError> {
    match task.params.get("anchor") {
        None => Ok(Anchor::Center),
        Some(value) => match value.as_str() {
            Some("center") => Ok(Anchor::Center),
            Some("top") => Ok(Anchor::Top),
            Some("bottom") => Ok(Anchor::Bottom),
            Some("left") => Ok(Anchor::Left),
            Some("right") => Ok(Anchor::Right),
            _ => Err(bad("anchor", "must be center, top, bottom, left or right")),
        },
    }
}

/// The `target_bytes` of `compress-to`: 1 to 20000000 bytes.
fn target_bytes(task: &Task) -> Result<u64, EngineError> {
    match task.param_i64("target_bytes") {
        Some(bytes) if (1..=TARGET_BYTES_MAX).contains(&bytes) => Ok(bytes as u64),
        _ => Err(bad(
            "target_bytes",
            format!("must be a whole number of bytes from 1 to {TARGET_BYTES_MAX}"),
        )),
    }
}

/// `resize-box`: the `image_geom` target, with the fill crop applied before
/// the resize.
fn resize_box(img: DynamicImage, width: u32, height: u32, mode: BoxFit) -> DynamicImage {
    let geom = match mode {
        BoxFit::Fit => Mode::Fit { width, height },
        BoxFit::Fill => Mode::Fill { width, height },
        BoxFit::Exact => Mode::Exact { width, height },
    };
    let Some(target) = image_geom::target((img.width(), img.height()), geom) else {
        return img;
    };
    let source = match target.crop {
        Some(crop) => img.crop_imm(crop.x, crop.y, crop.width, crop.height),
        None => img,
    };
    if (source.width(), source.height()) == (target.width, target.height) {
        source
    } else {
        source.resize_exact(target.width, target.height, FilterType::Lanczos3)
    }
}

/// `crop`: the largest box of `aspect` inside the source, at `anchor`. The
/// arithmetic is `kawaro_core::image_geom::window`, shared with the stage 2
/// preview (S-127 AC2).
fn crop(img: DynamicImage, aspect: Aspect, anchor: Anchor) -> DynamicImage {
    let Some(ratio) = aspect.ratio() else {
        return img;
    };
    let Some(window) = image_geom::window((img.width(), img.height()), ratio, anchor) else {
        return img;
    };
    img.crop_imm(window.x, window.y, window.width, window.height)
}

/// `quality` from the action, validated, or `default`.
fn quality(task: &Task, default: u8) -> Result<u8, EngineError> {
    match task.params.get("quality") {
        None => Ok(default),
        Some(v) => match v.as_integer() {
            Some(q @ 1..=100) => Ok(q as u8),
            _ => Err(bad("quality", "must be a whole number from 1 to 100")),
        },
    }
}

/// `keep_metadata` from the action: JPEG metadata is copied when true, and
/// absent keeps the strip-on-re-encode default. Any other TOML type is
/// refused, never read as false.
fn keep_metadata(task: &Task) -> Result<bool, EngineError> {
    match task.params.get("keep_metadata") {
        None => Ok(false),
        Some(value) => match value.as_bool() {
            Some(on) => Ok(on),
            None => Err(bad("keep_metadata", "must be true or false")),
        },
    }
}

/// Rewrites `path` atomically (S-129): the new bytes go to a sibling file
/// that is renamed over it, so a failure never leaves a half written JPEG.
fn write_over(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    static TEMP_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let count = TEMP_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut name = path.as_os_str().to_owned();
    name.push(format!(".{}.{count}.meta.tmp", std::process::id()));
    let temp = PathBuf::from(name);
    std::fs::write(&temp, bytes)?;
    match std::fs::rename(&temp, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(&temp);
            Err(e)
        }
    }
}

/// Copies the JPEG metadata of `source` into `output` (S-129). Any failure
/// removes the output, so a task never leaves a JPEG that lost the metadata
/// it asked to keep.
fn copy_metadata(source: &[u8], output: &Path) -> Result<(), EngineError> {
    let segments = metadata::extract(source).unwrap_or_default();
    if segments.is_empty() {
        return Ok(());
    }
    let discard = |error: EngineError| {
        let _ = std::fs::remove_file(output);
        error
    };
    let encoded = std::fs::read(output).map_err(|e| discard(e.into()))?;
    let Some(with_metadata) = metadata::inject(&encoded, &segments) else {
        return Err(discard(EngineError::Failed(
            "cannot copy the JPEG metadata".into(),
        )));
    };
    write_over(output, &with_metadata).map_err(|e| discard(e.into()))
}

fn single_input(task: &Task) -> Result<&Path, EngineError> {
    match task.inputs.as_slice() {
        [one] => Ok(one),
        other => Err(bad(
            "inputs",
            format!("`{}` takes exactly one image, got {}", task.op, other.len()),
        )),
    }
}

fn run_single(
    task: &Task,
    op: Op,
    progress: Progress,
    cancel: &CancelToken,
) -> Result<(), EngineError> {
    let input = single_input(task)?;
    let target = Target::from_path(&task.output)?;
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

    let Loaded {
        image: img,
        bytes: source,
        format,
        ..
    } = load(input)?;
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
    progress(0.4);
    cancel.check()?;

    let img = op.apply(img);
    progress(0.7);
    cancel.check()?;

    let bytes = match op {
        Op::CompressTo(target_bytes) => encode_to_target(&img, target, target_bytes, cancel)?,
        _ => encode::encode(&img, target, quality, op == Op::Compress)?,
    };
    cancel.check()?;
    std::fs::write(&task.output, &bytes)?;
    if keep {
        // The encode is done; copy the input's metadata into the encoded file.
        copy_metadata(&source, &task.output)?;
    }
    progress(1.0);
    Ok(())
}

/// One \`compress-to\` search: the full quality search at the current size, then
/// a fixed 0.8 downscale when it cannot reach the target, until a size fits or
/// the minimum side or the round limit is reached (S-132 AC1 and AC2). The
/// fallback never enlarges and never shrinks a side already at the minimum.
fn encode_to_target(
    img: &DynamicImage,
    target: Target,
    target_bytes: u64,
    cancel: &CancelToken,
) -> Result<Vec<u8>, EngineError> {
    let mut current = img.clone();
    let mut rounds = 0u32;
    let mut smallest_bytes = u64::MAX;
    loop {
        cancel.check()?;
        if let Some(bytes) =
            quality_under(&current, target, target_bytes, &mut smallest_bytes, cancel)?
        {
            return Ok(bytes);
        }
        let (width, height) = (current.width(), current.height());
        let (next_width, next_height) = (stepped(width), stepped(height));
        if rounds >= TARGET_ROUNDS || (next_width == width && next_height == height) {
            return Err(EngineError::Failed(format!(
                "cannot reach {target_bytes} bytes: the smallest size is {width} x {height} and its smallest encode is {smallest_bytes} bytes"
            )));
        }
        rounds += 1;
        current = current.resize_exact(next_width, next_height, FilterType::Lanczos3);
    }
}

/// One \`4/5\` step, rounded half up, at least one pixel smaller and never
/// below the minimum side (S-132).
fn stepped(side: u32) -> u32 {
    if side <= TARGET_MIN_SIDE {
        return side;
    }
    let next = (u64::from(side) * TARGET_STEP_NUM + TARGET_STEP_DEN / 2) / TARGET_STEP_DEN;
    let next = u32::try_from(next).unwrap_or(u32::MAX);
    next.clamp(TARGET_MIN_SIDE, side - 1)
}

/// The best encode at or under \`target_bytes\` at the current size, spending at
/// most [\`TARGET_ENCODES\`] encodes (S-127 AC4): quality 100 when it already
/// fits, then a binary search over 1 to 99. None when even quality 1 is over
/// the target. \`smallest_bytes\` keeps the smallest encode seen at any size.
fn quality_under(
    img: &DynamicImage,
    target: Target,
    target_bytes: u64,
    smallest_bytes: &mut u64,
    cancel: &CancelToken,
) -> Result<Option<Vec<u8>>, EngineError> {
    cancel.check()?;
    let top = encode::encode(img, target, 100, false)?;
    *smallest_bytes = (*smallest_bytes).min(top.len() as u64);
    if top.len() as u64 <= target_bytes {
        return Ok(Some(top));
    }
    let floor = encode::encode(img, target, 1, false)?;
    *smallest_bytes = (*smallest_bytes).min(floor.len() as u64);
    if floor.len() as u64 > target_bytes {
        return Ok(None);
    }
    let mut best = floor;
    let mut attempts = 2;
    let (mut low, mut high) = (2u8, 99u8);
    while attempts < TARGET_ENCODES && low <= high {
        cancel.check()?;
        let quality = ((u16::from(low) + u16::from(high)) / 2) as u8;
        let bytes = encode::encode(img, target, quality, false)?;
        attempts += 1;
        if bytes.len() as u64 <= target_bytes {
            best = bytes;
            low = quality + 1;
        } else {
            high = quality - 1;
        }
    }
    Ok(Some(best))
}

/// A decoded input with orientation applied.
struct Loaded {
    image: DynamicImage,
    /// Raw file bytes, for embedding without re-encoding.
    bytes: Vec<u8>,
    format: ImageFormat,
    /// True when the EXIF orientation changed the pixels.
    reoriented: bool,
}

fn load(path: &Path) -> Result<Loaded, EngineError> {
    let input_err = |reason: String| EngineError::Input {
        path: path.to_path_buf(),
        reason,
    };
    let bytes = std::fs::read(path).map_err(|e| input_err(e.to_string()))?;
    let reader = ImageReader::new(Cursor::new(bytes.as_slice()))
        .with_guessed_format()
        .map_err(|e| input_err(e.to_string()))?;
    let format = match reader.format() {
        Some(ImageFormat::Avif) => {
            return Err(input_err(
                "AVIF decoding is not supported in this build (it needs the dav1d system library)"
                    .into(),
            ));
        }
        Some(f) => f,
        None => return Err(input_err("not a recognised image format".into())),
    };
    let mut decoder = reader
        .into_decoder()
        .map_err(|e| input_err(e.to_string()))?;
    let orientation = decoder
        .orientation()
        .unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut image = DynamicImage::from_decoder(decoder).map_err(|e| input_err(e.to_string()))?;
    let reoriented = orientation != image::metadata::Orientation::NoTransforms;
    image.apply_orientation(orientation);
    Ok(Loaded {
        image,
        bytes,
        format,
        reoriented,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_task(params: &str) -> Task {
        Task {
            action_id: "test.image".into(),
            op: "x".into(),
            params: toml::from_str(params).unwrap(),
            inputs: vec![],
            output: std::path::PathBuf::from("out.png"),
        }
    }

    #[test]
    fn resize_box_fit_fill_and_exact() {
        let source = DynamicImage::ImageRgb8(image::RgbImage::new(64, 48));
        let fit = resize_box(source.clone(), 32, 32, BoxFit::Fit);
        assert_eq!((fit.width(), fit.height()), (32, 24));
        let fill = resize_box(source.clone(), 32, 32, BoxFit::Fill);
        assert_eq!((fill.width(), fill.height()), (32, 32));
        let exact = resize_box(source, 10, 20, BoxFit::Exact);
        assert_eq!((exact.width(), exact.height()), (10, 20));
        // Fit never enlarges a source smaller than the box.
        let small = DynamicImage::ImageRgb8(image::RgbImage::new(20, 10));
        let fit = resize_box(small, 64, 64, BoxFit::Fit);
        assert_eq!((fit.width(), fit.height()), (20, 10));
    }

    #[test]
    fn s127_params_are_validated() {
        assert_eq!(box_side(&test_task("width = 1920"), "width").unwrap(), 1920);
        assert_eq!(box_side(&test_task("height = 1"), "height").unwrap(), 1);
        assert_eq!(
            box_side(&test_task("width = 20000"), "width").unwrap(),
            20000
        );
        for bad in [
            "width = 0",
            "width = 20001",
            "width = -1",
            "width = \"big\"",
        ] {
            match box_side(&test_task(bad), "width") {
                Err(EngineError::BadParam { name, .. }) => assert_eq!(name, "width", "{bad}"),
                other => panic!("{bad}: expected BadParam, got {other:?}"),
            }
        }
        assert_eq!(box_mode(&test_task("")).unwrap(), BoxFit::Fit);
        assert_eq!(
            box_mode(&test_task("mode = \"fill\"")).unwrap(),
            BoxFit::Fill
        );
        assert_eq!(
            box_mode(&test_task("mode = \"exact\"")).unwrap(),
            BoxFit::Exact
        );
        assert!(box_mode(&test_task("mode = \"cover\"")).is_err());
        assert_eq!(aspect_param(&test_task("")).unwrap(), Aspect::Original);
        assert_eq!(
            aspect_param(&test_task("aspect = \"1:1\"")).unwrap(),
            Aspect::Square
        );
        for bad in ["aspect = \"9:16\"", "aspect = 1", "aspect = \"square\""] {
            assert!(aspect_param(&test_task(bad)).is_err(), "{bad}");
        }
        assert_eq!(anchor_param(&test_task("")).unwrap(), Anchor::Center);
        assert_eq!(
            anchor_param(&test_task("anchor = \"bottom\"")).unwrap(),
            Anchor::Bottom
        );
        for bad in ["anchor = \"middle\"", "anchor = 1"] {
            assert!(anchor_param(&test_task(bad)).is_err(), "{bad}");
        }
        assert_eq!(target_bytes(&test_task("target_bytes = 1")).unwrap(), 1);
        assert_eq!(
            target_bytes(&test_task("target_bytes = 20000000")).unwrap(),
            20_000_000
        );
        for bad in [
            "target_bytes = 0",
            "target_bytes = 20000001",
            "target_bytes = \"500\"",
        ] {
            assert!(target_bytes(&test_task(bad)).is_err(), "{bad}");
        }
    }

    #[test]
    fn s127_compress_to_reaches_the_target_when_it_can() {
        let img = DynamicImage::ImageRgb8(image::RgbImage::from_fn(64, 64, |x, y| {
            image::Rgb([
                (x.wrapping_mul(7)) as u8,
                (y.wrapping_mul(11)) as u8,
                (x.wrapping_add(y).wrapping_mul(5)) as u8,
            ])
        }));
        let bytes = encode_to_target(&img, Target::Jpeg, 900, &CancelToken::new()).unwrap();
        assert!(bytes.len() as u64 <= 900, "{} bytes", bytes.len());
        assert_eq!(image::load_from_memory(&bytes).unwrap().width(), 64);
        // Quality 100 is used as it is when it already fits.
        let huge = encode_to_target(&img, Target::Jpeg, 10_000_000, &CancelToken::new()).unwrap();
        assert_eq!(
            huge.len(),
            encode::encode(&img, Target::Jpeg, 100, false)
                .unwrap()
                .len()
        );
        // A target no quality can reach fails instead of missing it.
        match encode_to_target(&img, Target::Jpeg, 1, &CancelToken::new()) {
            Err(EngineError::Failed(message)) => assert!(message.contains("1"), "{message}"),
            other => panic!("expected Failed, got {other:?}"),
        }
        let cancel = CancelToken::new();
        cancel.cancel();
        assert!(matches!(
            encode_to_target(&img, Target::Jpeg, 900, &cancel),
            Err(EngineError::Cancelled)
        ));
    }

    #[test]
    fn size_target_rounds_and_clamps() {
        assert_eq!(Size::Scale(50.0).target(64, 48), (32, 24));
        assert_eq!(Size::Scale(150.0).target(3, 3), (5, 5));
        assert_eq!(Size::Scale(25.0).target(10, 6), (3, 2));
        assert_eq!(Size::Scale(1.0).target(10, 10), (1, 1));
        assert_eq!(Size::MaxSide(32).target(64, 48), (32, 24));
        assert_eq!(Size::MaxSide(32).target(48, 64), (24, 32));
        assert_eq!(Size::MaxSide(64).target(64, 48), (64, 48));
        assert_eq!(Size::MaxSide(63).target(64, 48), (63, 47));
        assert_eq!(Size::MaxSide(10).target(100, 1), (10, 1));
    }
}
