//! Shared helpers for the image engine tests (S-004).
//!
//! Fixture layout, used by every test: a 64x48 picture split into four
//! 32x24 quadrants, red top left, green top right, blue bottom left and
//! yellow bottom right. The 16x16 block at the top left corner is fully
//! transparent in `alpha.png`, so flattening onto white is observable.
#![allow(dead_code)]

use image::{ImageFormat, Rgba, RgbaImage};
use kawaro_core::{CancelToken, Engine, EngineError, Task};
use kawaro_engines::image::ImageEngine;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const RED: [u8; 3] = [255, 0, 0];
pub const GREEN: [u8; 3] = [0, 255, 0];
pub const BLUE: [u8; 3] = [0, 0, 255];
pub const YELLOW: [u8; 3] = [255, 255, 0];

pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/images")
        .join(name)
}

/// The quadrant picture, fully opaque.
pub fn quadrants(w: u32, h: u32) -> RgbaImage {
    RgbaImage::from_fn(w, h, |x, y| {
        let c = match (x < w / 2, y < h / 2) {
            (true, true) => RED,
            (false, true) => GREEN,
            (true, false) => BLUE,
            (false, false) => YELLOW,
        };
        Rgba([c[0], c[1], c[2], 255])
    })
}

/// The quadrant picture with the transparent 16x16 corner.
pub fn quadrants_alpha() -> RgbaImage {
    let mut img = quadrants(64, 48);
    for y in 0..16 {
        for x in 0..16 {
            img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
        }
    }
    img
}

/// A minimal big endian EXIF APP1 segment holding only Orientation.
pub fn exif_orientation_segment(orientation: u16) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(b"Exif\0\0");
    body.extend_from_slice(b"MM\0\x2a\0\0\0\x08");
    body.extend_from_slice(&1u16.to_be_bytes());
    body.extend_from_slice(&0x0112u16.to_be_bytes());
    body.extend_from_slice(&3u16.to_be_bytes());
    body.extend_from_slice(&1u32.to_be_bytes());
    body.extend_from_slice(&orientation.to_be_bytes());
    body.extend_from_slice(&[0, 0]);
    body.extend_from_slice(&0u32.to_be_bytes());
    let mut seg = vec![0xFF, 0xE1];
    seg.extend_from_slice(&((body.len() + 2) as u16).to_be_bytes());
    seg.extend_from_slice(&body);
    seg
}

/// Encodes `img` (RGB) as JPEG at `quality` and inserts an EXIF segment
/// right after SOI when `orientation` is given.
pub fn jpeg_bytes(img: &RgbaImage, quality: u8, orientation: Option<u16>) -> Vec<u8> {
    let rgb = image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality)
        .encode_image(&rgb)
        .unwrap();
    if let Some(o) = orientation {
        let seg = exif_orientation_segment(o);
        out.splice(2..2, seg);
    }
    out
}

pub fn png_bytes(img: &RgbaImage) -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, ImageFormat::Png).unwrap();
    out.into_inner()
}

pub fn task(op: &str, inputs: Vec<PathBuf>, output: PathBuf, params: &str) -> Task {
    Task {
        action_id: format!("test-{op}"),
        op: op.to_string(),
        params: toml::from_str(params).unwrap(),
        inputs,
        output,
    }
}

/// Runs a task and returns the result and every progress value reported.
pub fn run_task(t: &Task) -> (Result<(), EngineError>, Vec<f32>) {
    let seen = Mutex::new(Vec::new());
    let progress = |f: f32| seen.lock().unwrap().push(f);
    let res = ImageEngine::default().run(t, &progress, &CancelToken::new());
    (res, seen.into_inner().unwrap())
}

/// Runs `op` on one input into `dir/out.<ext>` and returns the output bytes.
pub fn run_one(op: &str, input: &Path, dir: &Path, ext: &str, params: &str) -> Vec<u8> {
    let out = dir.join(format!("out.{ext}"));
    let t = task(op, vec![input.to_path_buf()], out.clone(), params);
    let (res, progress) = run_task(&t);
    res.unwrap_or_else(|e| panic!("{op} to {ext} failed: {e}"));
    assert_progress_shape(&progress);
    std::fs::read(&out).unwrap()
}

pub fn assert_progress_shape(p: &[f32]) {
    assert!(p.len() >= 2, "progress called {} times", p.len());
    assert_eq!(p[0], 0.0, "first progress value");
    assert_eq!(*p.last().unwrap(), 1.0, "last progress value");
    assert!(
        p.windows(2).all(|w| w[0] <= w[1]),
        "progress decreased: {p:?}"
    );
}

pub fn decode(bytes: &[u8]) -> image::DynamicImage {
    image::load_from_memory(bytes).unwrap()
}

/// True when every channel of `px` is within `tol` of `want`.
pub fn near(px: [u8; 3], want: [u8; 3], tol: u8) -> bool {
    px.iter().zip(want).all(|(a, b)| a.abs_diff(b) <= tol)
}

pub fn rgb_at(img: &image::DynamicImage, x: u32, y: u32) -> [u8; 3] {
    let p = img.to_rgba8().get_pixel(x, y).0;
    [p[0], p[1], p[2]]
}

pub fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

pub fn count(hay: &[u8], needle: &[u8]) -> usize {
    hay.windows(needle.len()).filter(|w| *w == needle).count()
}
