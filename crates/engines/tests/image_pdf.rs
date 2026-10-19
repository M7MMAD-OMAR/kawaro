//! S-004 image engine: `to-pdf`.

mod image_common;
use image_common::*;

use sbarhive_core::EngineError;
use std::path::PathBuf;
use tempfile::tempdir;

/// Number of page objects: `/Type /Page` not followed by `s`.
fn page_count(pdf: &[u8]) -> usize {
    count(pdf, b"/Type /Page") - count(pdf, b"/Type /Pages")
}

/// Every MediaBox `[x0 y0 x1 y1]` in file order.
fn media_boxes(pdf: &[u8]) -> Vec<(f32, f32)> {
    let text = String::from_utf8_lossy(pdf);
    text.match_indices("/MediaBox [")
        .map(|(i, m)| {
            let rest = &text[i + m.len()..];
            let end = rest.find(']').unwrap();
            let v: Vec<f32> = rest[..end]
                .split_whitespace()
                .map(|n| n.parse().unwrap())
                .collect();
            assert_eq!(&v[..2], &[0.0, 0.0]);
            (v[2], v[3])
        })
        .collect()
}

fn run_pdf(inputs: Vec<PathBuf>, params: &str) -> (Vec<u8>, Vec<f32>) {
    let d = tempdir().unwrap();
    let out = d.path().join("out.pdf");
    let t = task("to-pdf", inputs, out.clone(), params);
    let (res, progress) = run_task(&t);
    res.unwrap();
    assert_progress_shape(&progress);
    (std::fs::read(out).unwrap(), progress)
}

// AC13, AC14
#[test]
fn three_images_make_three_pages() {
    let d = tempdir().unwrap();
    let plain = d.path().join("plain.jpg");
    let jpeg = jpeg_bytes(&quadrants(40, 20), 85, None);
    std::fs::write(&plain, &jpeg).unwrap();
    let (pdf, progress) = run_pdf(
        vec![fixture("alpha.png"), plain, fixture("orient6.jpg")],
        "",
    );
    assert!(pdf.starts_with(b"%PDF-"));
    assert!(pdf.ends_with(b"%%EOF") || pdf.ends_with(b"%%EOF\n"));
    assert_eq!(page_count(&pdf), 3);
    assert!(contains(&pdf, b"/Count 3"));
    assert_eq!(
        media_boxes(&pdf),
        [(64.0, 48.0), (40.0, 20.0), (48.0, 64.0)]
    );
    // The upright JPEG is embedded byte for byte.
    assert!(contains(&pdf, b"/DCTDecode"));
    assert!(contains(&pdf, &jpeg), "JPEG was re-encoded");
    let inner = progress.iter().filter(|&&p| p > 0.0 && p < 1.0).count();
    assert!(inner >= 3, "per page progress missing: {progress:?}");
}

#[test]
fn rotated_jpeg_and_alpha_are_handled() {
    let (pdf, _) = run_pdf(vec![fixture("orient6.jpg")], "page = \"fit\"");
    assert_eq!(page_count(&pdf), 1);
    assert_eq!(media_boxes(&pdf), [(48.0, 64.0)]);
    // The stored bytes are sideways, so they cannot be embedded as is.
    let src = std::fs::read(fixture("orient6.jpg")).unwrap();
    assert!(!contains(&pdf, &src[src.len() - 200..]));

    let (pdf, _) = run_pdf(
        vec![
            fixture("alpha.png"),
            fixture("sample.gif"),
            fixture("sample.bmp"),
        ],
        "",
    );
    assert_eq!(page_count(&pdf), 3);
    assert_eq!(
        media_boxes(&pdf),
        [(64.0, 48.0), (32.0, 24.0), (40.0, 30.0)]
    );
    assert!(contains(&pdf, b"/FlateDecode"));
    assert!(contains(&pdf, b"/DeviceRGB"));
}

#[test]
fn grayscale_jpeg_is_embedded_as_is() {
    let d = tempdir().unwrap();
    let gray = d.path().join("gray.jpg");
    let img = image::GrayImage::from_fn(24, 16, |x, _| image::Luma([(x * 10) as u8]));
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 90)
        .encode_image(&img)
        .unwrap();
    std::fs::write(&gray, &bytes).unwrap();
    let (pdf, _) = run_pdf(vec![gray], "");
    assert!(contains(&pdf, &bytes));
    assert!(contains(&pdf, b"/DeviceGray"));
    assert_eq!(media_boxes(&pdf), [(24.0, 16.0)]);
}

#[test]
fn bad_page_mode_is_rejected() {
    let d = tempdir().unwrap();
    let out = d.path().join("o.pdf");
    let t = task(
        "to-pdf",
        vec![fixture("alpha.png")],
        out.clone(),
        "page = \"a4\"",
    );
    assert!(matches!(run_task(&t).0, Err(EngineError::BadParam { name, .. }) if name == "page"));
    assert!(!out.exists());
}

#[test]
fn bad_input_in_the_middle_names_that_file() {
    let d = tempdir().unwrap();
    let bad = d.path().join("broken.png");
    std::fs::write(&bad, b"nope").unwrap();
    let out = d.path().join("o.pdf");
    let t = task(
        "to-pdf",
        vec![fixture("alpha.png"), bad.clone()],
        out.clone(),
        "",
    );
    match run_task(&t).0 {
        Err(EngineError::Input { path, .. }) => assert_eq!(path, bad),
        other => panic!("expected Input, got {other:?}"),
    }
    assert!(!out.exists());
}

#[test]
fn cancel_between_pages() {
    let d = tempdir().unwrap();
    let out = d.path().join("o.pdf");
    let token = sbarhive_core::CancelToken::new();
    let t = task(
        "to-pdf",
        vec![fixture("alpha.png"), fixture("sample.gif")],
        out.clone(),
        "",
    );
    let progress = |f: f32| {
        if f > 0.0 {
            token.cancel();
        }
    };
    let res = sbarhive_core::Engine::run(
        &sbarhive_engines::image::ImageEngine::default(),
        &t,
        &progress,
        &token,
    );
    assert!(matches!(res, Err(EngineError::Cancelled)), "{res:?}");
    assert!(!out.exists());
}
