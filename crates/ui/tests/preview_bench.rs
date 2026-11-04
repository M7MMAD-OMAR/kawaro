//! S-118 preview budget bench (AC4).
//!
//! Prints probe_ms, draft_ms, mask_ms and draft_bytes for a generated 24 MP
//! JPEG and fails when a budget is missed. Run it through
//! tests/preview/bench.sh, which puts it in a systemd scope with --nocapture.
//! The first run also pays for encoding the fixture, so the fixture is built
//! before the timers start.
#![cfg(unix)]

use kawaro_ui::preview_worker::{decode_draft, mask_hexagon, probe_dimensions};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Budgets of S-118 AC4.
const PROBE_MS: f64 = 5.0;
const DRAFT_MS: f64 = 500.0;
const MASK_MS: f64 = 50.0;
const DRAFT_BYTES: usize = 4 * 1024 * 1024;

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A 6000 x 4000 (24 MP) baseline JPEG, the bench photo.
fn photo(dir: &Path) -> PathBuf {
    let path = dir.join("photo.jpg");
    let image = image::RgbImage::from_fn(6000, 4000, |x, y| {
        image::Rgb([
            (x % 256) as u8,
            (y % 256) as u8,
            ((x / 3 + y / 2) % 256) as u8,
        ])
    });
    let file = std::fs::File::create(&path).unwrap();
    let mut encoder =
        image::codecs::jpeg::JpegEncoder::new_with_quality(std::io::BufWriter::new(file), 80);
    encoder
        .encode_image(&image::DynamicImage::ImageRgb8(image))
        .unwrap();
    path
}

/// The median of the runs, so one scheduler hiccup does not fail the bench.
fn median_ms(runs: &mut [f64]) -> f64 {
    runs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    runs[runs.len() / 2]
}

/// The budgets are release numbers: an unoptimised decode of the 24 MP photo
/// is roughly ten times slower, so this test stays ignored for the debug
/// suite and tests/preview/bench.sh runs it with --release --ignored.
#[test]
#[ignore = "release budget bench, run through tests/preview/bench.sh"]
fn preview_budgets_hold_for_a_24_mp_photo() {
    let dir = scratch("s118-bench");
    let path = photo(&dir);

    let mut runs = Vec::new();
    for _ in 0..7 {
        let start = Instant::now();
        let dimensions = probe_dimensions(&path).expect("the header probe");
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(dimensions, (6000, 4000));
        runs.push(ms);
    }
    let probe_ms = median_ms(&mut runs);

    let mut runs = Vec::new();
    for _ in 0..3 {
        let start = Instant::now();
        let draft = decode_draft(&path, 1000).expect("a draft");
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        assert!(draft.width().max(draft.height()) <= 1000);
        runs.push(ms);
    }
    let draft_ms = median_ms(&mut runs);

    let mut draft = decode_draft(&path, 1000).expect("a draft");
    let circumradius = draft.height() as f32 / 2.0;
    let mut runs = Vec::new();
    for _ in 0..5 {
        let start = Instant::now();
        mask_hexagon(&mut draft, circumradius).expect("the mask");
        runs.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    let mask_ms = median_ms(&mut runs);

    let draft_bytes = draft.width() as usize * draft.height() as usize * 4;
    println!("probe_ms: {probe_ms:.2}");
    println!("draft_ms: {draft_ms:.2}");
    println!("mask_ms: {mask_ms:.2}");
    println!("draft_bytes: {draft_bytes}");
    println!("draft_size: {}x{}", draft.width(), draft.height());

    assert!(
        probe_ms < PROBE_MS,
        "probe_ms {probe_ms:.2} over {PROBE_MS}"
    );
    assert!(
        draft_ms < DRAFT_MS,
        "draft_ms {draft_ms:.2} over {DRAFT_MS}"
    );
    assert!(mask_ms < MASK_MS, "mask_ms {mask_ms:.2} over {MASK_MS}");
    assert!(
        draft_bytes < DRAFT_BYTES,
        "draft_bytes {draft_bytes} over {DRAFT_BYTES}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
