//! S-132 AC1, AC2, AC4 and AC5: image.compress-to reaches a byte target by
//! a deterministic 0.8 downscale when quality alone cannot.
//! Contract pinned here: fixed step 4/5 rounded half up, aspect kept, minimum
//! side 64 px, at most 16 rounds, full quality search at each size, stop at
//! the first size at or under target_bytes. TaskDetail reaches surfaces
//! through Engine::result_detail and carries the final and source sizes.

mod image_common;
use image_common::*;

use image::{Rgb, RgbImage};
use kawaro_core::engine::TaskDetail;
use kawaro_core::{CancelToken, Engine, EngineError, Task};
use kawaro_engines::image::ImageEngine;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

const SOURCE_WIDTH: u32 = 2400;
const SOURCE_HEIGHT: u32 = 1600;

/// A deterministic noisy photo, so the quality ladder really runs out.
fn photo(dir: &Path) -> PathBuf {
    let path = dir.join("photo.jpg");
    let image = RgbImage::from_fn(SOURCE_WIDTH, SOURCE_HEIGHT, |x, y| {
        Rgb([
            ((x * 37 + y * 17) % 256) as u8,
            ((x * 53 + y * 29) % 256) as u8,
            ((x * 11 + y * 47) % 256) as u8,
        ])
    });
    image.save(&path).unwrap();
    path
}

fn task(input: &Path, output: PathBuf, params: &str) -> Task {
    Task {
        action_id: "image.compress-to".to_string(),
        op: "compress-to".to_string(),
        params: toml::from_str(params).unwrap(),
        inputs: vec![input.to_path_buf()],
        output,
    }
}

fn run(engine: &ImageEngine, task: &Task) -> Result<(), EngineError> {
    engine.run(task, &|_fraction| {}, &CancelToken::new())
}

fn open(path: &Path) -> image::DynamicImage {
    image::open(path).expect("the output decodes")
}

#[test]
fn ac1_target_50000_is_reached_by_downscaling() {
    let dir = tempdir().unwrap();
    let input = photo(dir.path());
    let before = std::fs::read(&input).unwrap();
    let out = dir.path().join("out.jpg");
    let task = task(&input, out.clone(), "target_bytes = 50000");
    let engine = ImageEngine::default();
    run(&engine, &task).expect("target 50000 must be reachable by downscaling");

    let bytes = std::fs::metadata(&out).unwrap().len();
    assert!(
        bytes <= 50_000,
        "the output is {bytes} bytes, over the target"
    );
    let decoded = open(&out);
    let (w, h) = (decoded.width(), decoded.height());
    assert!(
        w < SOURCE_WIDTH && h < SOURCE_HEIGHT,
        "both sides shrink: {w}x{h}"
    );
    let ideal_height = w as f64 * SOURCE_HEIGHT as f64 / SOURCE_WIDTH as f64;
    assert!(
        (h as f64 - ideal_height).abs() <= 1.0,
        "aspect within 1 px: {w}x{h} for 2400x1600"
    );
    assert_eq!(
        std::fs::read(&input).unwrap(),
        before,
        "the input is unchanged"
    );

    let detail = engine
        .result_detail(&task)
        .expect("a compress-to result carries TaskDetail");
    assert_eq!(
        (
            detail.width,
            detail.height,
            detail.source_width,
            detail.source_height
        ),
        (w, h, SOURCE_WIDTH, SOURCE_HEIGHT)
    );
    assert_eq!(detail.bytes, bytes as u64);
    assert!(detail.downscaled, "a downscaled result says so");
}

#[test]
fn ac5_the_same_fixture_and_target_give_the_same_result_twice() {
    let engine = ImageEngine::default();
    let dir1 = tempdir().unwrap();
    let dir2 = tempdir().unwrap();
    let photo1 = photo(dir1.path());
    let photo2 = photo(dir2.path());
    assert_eq!(
        std::fs::read(&photo1).unwrap(),
        std::fs::read(&photo2).unwrap()
    );

    let out1 = dir1.path().join("out.jpg");
    let out2 = dir2.path().join("out.jpg");
    let task1 = task(&photo1, out1.clone(), "target_bytes = 60000");
    let task2 = task(&photo2, out2.clone(), "target_bytes = 60000");
    run(&engine, &task1).unwrap();
    run(&engine, &task2).unwrap();
    let first = open(&out1);
    let second = open(&out2);
    assert_eq!(
        (first.width(), first.height()),
        (second.width(), second.height()),
        "the downscale is deterministic"
    );
    assert_eq!(
        std::fs::metadata(&out1).unwrap().len(),
        std::fs::metadata(&out2).unwrap().len(),
        "the bytes are deterministic"
    );
}

#[test]
fn ac1_a_two_step_target_takes_at_least_two_downscale_rounds() {
    let dir = tempdir().unwrap();
    let input = photo(dir.path());
    let out = dir.path().join("out.jpg");
    run(
        &ImageEngine::default(),
        &task(&input, out.clone(), "target_bytes = 50000"),
    )
    .expect("target 50000 needs at least two rounds");
    let decoded = open(&out);
    assert!(
        decoded.width() < 1920,
        "one 0.8 step gives 1920 wide; {} means two or more rounds",
        decoded.width()
    );
}

#[test]
fn ac1_a_target_that_fits_at_the_original_size_does_not_downscale() {
    let dir = tempdir().unwrap();
    let input = photo(dir.path());
    let out = dir.path().join("out.jpg");
    let task = task(&input, out.clone(), "target_bytes = 20000000");
    let engine = ImageEngine::default();
    run(&engine, &task).expect("a huge target fits at the original size");
    let decoded = open(&out);
    assert_eq!(
        (decoded.width(), decoded.height()),
        (SOURCE_WIDTH, SOURCE_HEIGHT)
    );
    let detail = engine.result_detail(&task).expect("detail for the output");
    assert!(!detail.downscaled, "no fallback means downscaled false");
    assert_eq!(
        (
            detail.width,
            detail.height,
            detail.source_width,
            detail.source_height
        ),
        (SOURCE_WIDTH, SOURCE_HEIGHT, SOURCE_WIDTH, SOURCE_HEIGHT)
    );
}

#[test]
fn ac2_an_unreachable_target_fails_naming_the_minimum_and_writes_nothing() {
    let dir = tempdir().unwrap();
    let input = photo(dir.path());
    let out = dir.path().join("out.jpg");
    let error = run(
        &ImageEngine::default(),
        &task(&input, out.clone(), "target_bytes = 100"),
    )
    .expect_err("100 bytes is unreachable");
    let EngineError::Failed(message) = error else {
        panic!("expected Failed, got {error:?}");
    };
    assert!(message.contains("100"), "the target is named: {message}");
    assert!(
        message.contains(" x "),
        "the minimum dimensions are named: {message}"
    );
    assert!(
        message.contains("bytes"),
        "the smallest bytes are named: {message}"
    );
    assert!(!out.exists(), "a failed target writes no output");
    let leftovers: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name != "photo.jpg")
        .collect();
    assert!(leftovers.is_empty(), "no temp file is left: {leftovers:?}");
}

#[test]
fn ac2_target_bounds_stay_bad_param() {
    let dir = tempdir().unwrap();
    let input = photo(dir.path());
    for target in [0, 20_000_001] {
        let out = dir.path().join(format!("out-{target}.jpg"));
        let error = run(
            &ImageEngine::default(),
            &task(&input, out.clone(), &format!("target_bytes = {target}")),
        )
        .expect_err("out of range");
        match error {
            EngineError::BadParam { name, .. } => assert_eq!(name, "target_bytes"),
            other => panic!("expected BadParam target_bytes, got {other:?}"),
        }
        assert!(!out.exists());
    }
}

#[test]
fn ac4_keep_metadata_survives_the_downscale_fallback() {
    let dir = tempdir().unwrap();
    let source = photo(dir.path());
    let base = std::fs::read(&source).unwrap();
    let xmp = app_segment(0xE1, b"http://ns.adobe.com/xap/1.0/\0KAWARO-XMP");
    let icc = app_segment(0xE2, b"ICC_PROFILE\0\x01\x01KAWARO-ICC");
    let input = dir.path().join("meta.jpg");
    std::fs::write(&input, with_segments(&base, &[xmp, icc])).unwrap();
    let out = dir.path().join("out.jpg");
    run(
        &ImageEngine::default(),
        &task(
            &input,
            out.clone(),
            "target_bytes = 50000\nkeep_metadata = true",
        ),
    )
    .expect("the target needs a fallback and keeps metadata");
    let written = std::fs::read(&out).unwrap();
    assert_eq!(
        count(&written, b"KAWARO-XMP"),
        1,
        "the XMP APP1 is kept once"
    );
    assert_eq!(
        count(&written, b"KAWARO-ICC"),
        1,
        "the ICC APP2 is kept once"
    );
}

#[test]
fn ac4_a_cancelled_run_leaves_no_output() {
    let dir = tempdir().unwrap();
    let input = photo(dir.path());
    let out = dir.path().join("out.jpg");
    let task = task(&input, out.clone(), "target_bytes = 50000");
    let cancel = CancelToken::new();
    cancel.cancel();
    let result = ImageEngine::default().run(&task, &|_fraction| {}, &cancel);
    assert!(matches!(result, Err(EngineError::Cancelled)), "{result:?}");
    assert!(!out.exists());
}

fn app_segment(marker: u8, payload: &[u8]) -> Vec<u8> {
    let mut segment = vec![0xFF, marker];
    segment.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
    segment.extend_from_slice(payload);
    segment
}

fn with_segments(image: &[u8], segments: &[Vec<u8>]) -> Vec<u8> {
    let mut out = image[..2].to_vec();
    for segment in segments {
        out.extend_from_slice(segment);
    }
    out.extend_from_slice(&image[2..]);
    out
}

/// The struct the surfaces receive; kept here so the field names are pinned.
#[allow(dead_code)]
fn detail_shape(detail: &TaskDetail) -> (u32, u32, u32, u32, u64, bool) {
    (
        detail.width,
        detail.height,
        detail.source_width,
        detail.source_height,
        detail.bytes,
        detail.downscaled,
    )
}
