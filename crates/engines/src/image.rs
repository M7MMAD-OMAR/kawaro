//! Image engine (S-004). Pure Rust decoding and encoding through the `image`
//! crate, lossy WebP through libwebp, PDF through `pdf-writer`. The op table
//! is in `docs/architecture.md`, dependency notes in `docs/engines.md`.
//!
//! Every op decodes the input, applies its EXIF orientation, transforms the
//! pixels and encodes into the format named by the extension of
//! `task.output`. Output bytes are built in memory and written once at the
//! end, so a failure or a cancel never leaves a partial file behind.

mod encode;
mod pdf;

use encode::Target;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, imageops::FilterType};
use kawaro_core::{CancelToken, Engine, EngineError, MediaKind, Progress, Task};
use std::io::Cursor;
use std::path::Path;

/// Operations in the order the hive lists them.
const OPS: [&str; 8] = [
    "convert",
    "resize",
    "rotate",
    "flip",
    "grayscale",
    "compress",
    "strip",
    "to-pdf",
];

/// Lossy quality when the action does not set one.
const DEFAULT_QUALITY: u8 = 90;
/// Lossy quality for `compress` when the action does not set one.
const DEFAULT_COMPRESS_QUALITY: u8 = 75;

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
    ) -> Result<(), EngineError> {
        progress(0.0);
        cancel.check()?;
        match task.op.as_str() {
            "to-pdf" => pdf::run(task, progress, cancel),
            op => {
                let op = Op::parse(op, task)?;
                run_single(task, op, progress, cancel)
            }
        }
    }
}

/// A per-file operation with its validated parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Op {
    Convert,
    Resize(Size),
    Rotate(u32),
    Flip(Axis),
    Grayscale,
    Compress,
    Strip,
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
            Op::Convert | Op::Compress | Op::Strip => img,
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
    fn target(self, w: u32, h: u32) -> (u32, u32) {
        let factor = match self {
            Size::Scale(percent) => percent / 100.0,
            Size::MaxSide(max) => {
                let longest = w.max(h);
                if longest <= max {
                    return (w, h);
                }
                f64::from(max) / f64::from(longest)
            }
        };
        let side = |v: u32| ((f64::from(v) * factor).round() as u32).max(1);
        (side(w), side(h))
    }
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
    let default = if op == Op::Compress {
        DEFAULT_COMPRESS_QUALITY
    } else {
        DEFAULT_QUALITY
    };
    let quality = quality(task, default)?;

    let img = load(input)?.image;
    progress(0.4);
    cancel.check()?;

    let img = op.apply(img);
    progress(0.7);
    cancel.check()?;

    let bytes = encode::encode(&img, target, quality, op == Op::Compress)?;
    cancel.check()?;
    std::fs::write(&task.output, bytes)?;
    progress(1.0);
    Ok(())
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
