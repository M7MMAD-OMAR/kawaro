//! S-004 image engine: geometry, colour, orientation, errors and cancel.

mod image_common;
use image_common::*;

use sbarhive_core::{CancelToken, Engine, EngineError, MediaKind};
use sbarhive_engines::image::ImageEngine;
use tempfile::tempdir;

// AC1
#[test]
fn id_and_capabilities() {
    let e = ImageEngine::default();
    assert_eq!(e.id(), "image");
    assert!(e.available());
    let caps = e.capabilities();
    let ops: Vec<&str> = caps.iter().map(|(op, _)| *op).collect();
    assert_eq!(
        ops,
        [
            "convert",
            "resize",
            "rotate",
            "flip",
            "grayscale",
            "compress",
            "strip",
            "to-pdf"
        ]
    );
    for (op, kinds) in &caps {
        assert_eq!(kinds, &vec![MediaKind::Image], "kinds of {op}");
    }
    assert!(sbarhive_engines::all().iter().any(|e| e.id() == "image"));
}

// AC6
#[test]
fn resize_by_scale_percent() {
    let d = tempdir().unwrap();
    let out = decode(&run_one(
        "resize",
        &fixture("alpha.png"),
        d.path(),
        "png",
        "scale = 50",
    ));
    assert_eq!((out.width(), out.height()), (32, 24));
    // Lanczos3 on flat quadrants keeps the colours at quadrant centres.
    assert!(near(rgb_at(&out, 24, 6), GREEN, 8));
    assert!(near(rgb_at(&out, 8, 18), BLUE, 8));
}

#[test]
fn resize_by_float_scale_and_upscale() {
    let d = tempdir().unwrap();
    let out = decode(&run_one(
        "resize",
        &fixture("alpha.png"),
        d.path(),
        "png",
        "scale = 150.0",
    ));
    assert_eq!((out.width(), out.height()), (96, 72));
}

#[test]
fn resize_by_max_side() {
    let d = tempdir().unwrap();
    let out = decode(&run_one(
        "resize",
        &fixture("alpha.png"),
        d.path(),
        "png",
        "max_side = 32",
    ));
    assert_eq!((out.width(), out.height()), (32, 24));
    let tall = decode(&run_one(
        "resize",
        &fixture("orient6.jpg"),
        d.path(),
        "png",
        "max_side = 32",
    ));
    assert_eq!((tall.width(), tall.height()), (24, 32));
}

#[test]
fn resize_never_upscales_with_max_side() {
    let d = tempdir().unwrap();
    let out = decode(&run_one(
        "resize",
        &fixture("alpha.png"),
        d.path(),
        "png",
        "max_side = 64",
    ));
    assert_eq!((out.width(), out.height()), (64, 48));
    let out = decode(&run_one(
        "resize",
        &fixture("alpha.png"),
        d.path(),
        "png",
        "max_side = 500",
    ));
    assert_eq!((out.width(), out.height()), (64, 48));
}

#[test]
fn resize_keeps_at_least_one_pixel() {
    let d = tempdir().unwrap();
    let out = decode(&run_one(
        "resize",
        &fixture("alpha.png"),
        d.path(),
        "png",
        "scale = 1",
    ));
    assert_eq!((out.width(), out.height()), (1, 1));
    let out = decode(&run_one(
        "resize",
        &fixture("alpha.png"),
        d.path(),
        "png",
        "max_side = 1",
    ));
    assert_eq!((out.width(), out.height()), (1, 1));
}

fn expect_bad_param(op: &str, params: &str, name: &str) {
    let d = tempdir().unwrap();
    let out = d.path().join("out.png");
    let t = task(op, vec![fixture("alpha.png")], out.clone(), params);
    match run_task(&t).0 {
        Err(EngineError::BadParam { name: n, reason }) => {
            assert_eq!(n, name, "{op} {params}");
            assert!(!reason.is_empty());
        }
        other => panic!("{op} {params}: expected BadParam {name}, got {other:?}"),
    }
    assert!(!out.exists(), "{op} {params} left an output");
}

#[test]
fn resize_rejects_bad_params() {
    expect_bad_param("resize", "", "scale");
    expect_bad_param("resize", "scale = 0", "scale");
    expect_bad_param("resize", "scale = -5", "scale");
    expect_bad_param("resize", "scale = 1001", "scale");
    expect_bad_param("resize", "scale = 0.5", "scale");
    expect_bad_param("resize", "max_side = 0", "max_side");
    expect_bad_param("resize", "max_side = -3", "max_side");
}

#[test]
fn resize_accepts_bounds() {
    let d = tempdir().unwrap();
    let out = decode(&run_one(
        "resize",
        &fixture("sample.gif"),
        d.path(),
        "png",
        "scale = 1000",
    ));
    assert_eq!((out.width(), out.height()), (320, 240));
}

// AC7
#[test]
fn rotate_90_180_270() {
    let d = tempdir().unwrap();
    let src = fixture("alpha.png");
    let r90 = decode(&run_one("rotate", &src, d.path(), "png", "degrees = 90"));
    assert_eq!((r90.width(), r90.height()), (48, 64));
    // Clockwise: the red top left quadrant moves to the top right.
    assert!(near(rgb_at(&r90, 40, 24), RED, 0));
    assert!(near(rgb_at(&r90, 40, 40), GREEN, 0));

    let r180 = decode(&run_one("rotate", &src, d.path(), "png", "degrees = 180"));
    assert_eq!((r180.width(), r180.height()), (64, 48));
    assert!(near(rgb_at(&r180, 40, 30), RED, 0));
    assert!(near(rgb_at(&r180, 8, 6), YELLOW, 0));

    let r270 = decode(&run_one("rotate", &src, d.path(), "png", "degrees = 270"));
    assert_eq!((r270.width(), r270.height()), (48, 64));
    assert!(near(rgb_at(&r270, 8, 40), RED, 0));
    assert!(near(rgb_at(&r270, 8, 8), GREEN, 0));
}

#[test]
fn rotate_rejects_other_angles() {
    expect_bad_param("rotate", "", "degrees");
    expect_bad_param("rotate", "degrees = 45", "degrees");
    expect_bad_param("rotate", "degrees = 0", "degrees");
    expect_bad_param("rotate", "degrees = 360", "degrees");
}

// AC8
#[test]
fn flip_both_axes() {
    let d = tempdir().unwrap();
    let src = fixture("alpha.png");
    let h = decode(&run_one(
        "flip",
        &src,
        d.path(),
        "png",
        "axis = \"horizontal\"",
    ));
    assert_eq!((h.width(), h.height()), (64, 48));
    assert!(near(rgb_at(&h, 8, 6), GREEN, 0));
    assert!(near(rgb_at(&h, 40, 20), RED, 0));
    let v = decode(&run_one(
        "flip",
        &src,
        d.path(),
        "png",
        "axis = \"vertical\"",
    ));
    assert!(near(rgb_at(&v, 20, 6), BLUE, 0));
    assert!(near(rgb_at(&v, 20, 40), RED, 0));
}

#[test]
fn flip_rejects_other_axes() {
    expect_bad_param("flip", "", "axis");
    expect_bad_param("flip", "axis = \"diagonal\"", "axis");
}

// AC9
#[test]
fn grayscale_has_equal_channels() {
    let d = tempdir().unwrap();
    for ext in ["png", "jpg", "bmp", "webp"] {
        let out = decode(&run_one(
            "grayscale",
            &fixture("alpha.png"),
            d.path(),
            ext,
            "",
        ));
        assert_eq!((out.width(), out.height()), (64, 48), "{ext}");
        let rgba = out.to_rgba8();
        let tol = if ext == "png" || ext == "bmp" { 0 } else { 3 };
        for p in rgba.pixels() {
            assert!(
                p[0].abs_diff(p[1]) <= tol && p[1].abs_diff(p[2]) <= tol,
                "{ext}: {p:?}"
            );
        }
        // Red, green and blue quadrants map to distinct luma, so this is not
        // a blank image.
        let a = rgb_at(&out, 24, 18)[0];
        let b = rgb_at(&out, 40, 6)[0];
        assert!(a.abs_diff(b) > 40, "{ext}: {a} vs {b}");
    }
}

// AC10
#[test]
fn exif_orientation_is_applied_first() {
    let d = tempdir().unwrap();
    let out = decode(&run_one(
        "convert",
        &fixture("orient6.jpg"),
        d.path(),
        "png",
        "",
    ));
    assert_eq!((out.width(), out.height()), (48, 64));
    assert!(
        near(rgb_at(&out, 40, 8), RED, 40),
        "{:?}",
        rgb_at(&out, 40, 8)
    );
    assert!(
        near(rgb_at(&out, 8, 8), BLUE, 40),
        "{:?}",
        rgb_at(&out, 8, 8)
    );

    // Orientation applies before the op: rotating 90 more gives 180 total.
    let r = decode(&run_one(
        "rotate",
        &fixture("orient6.jpg"),
        d.path(),
        "png",
        "degrees = 90",
    ));
    assert_eq!((r.width(), r.height()), (64, 48));
    assert!(
        near(rgb_at(&r, 48, 36), RED, 40),
        "{:?}",
        rgb_at(&r, 48, 36)
    );
}

// AC14
#[test]
fn progress_starts_at_zero_and_ends_at_one() {
    let d = tempdir().unwrap();
    for (op, params) in [
        ("convert", ""),
        ("resize", "scale = 50"),
        ("rotate", "degrees = 90"),
        ("flip", "axis = \"vertical\""),
        ("grayscale", ""),
        ("compress", ""),
        ("strip", ""),
    ] {
        // run_one checks the progress shape.
        run_one(op, &fixture("alpha.png"), d.path(), "png", params);
    }
}

// AC15
#[test]
fn cancel_before_start_writes_nothing() {
    let d = tempdir().unwrap();
    let token = CancelToken::new();
    token.cancel();
    for (op, ext, params) in [
        ("convert", "webp", ""),
        ("resize", "png", "scale = 50"),
        ("rotate", "png", "degrees = 90"),
        ("flip", "png", "axis = \"vertical\""),
        ("grayscale", "png", ""),
        ("compress", "png", ""),
        ("strip", "png", ""),
        ("to-pdf", "pdf", ""),
    ] {
        let out = d.path().join(format!("{op}.{ext}"));
        let t = task(op, vec![fixture("alpha.png")], out.clone(), params);
        let res = ImageEngine::default().run(&t, &|_| {}, &token);
        assert!(matches!(res, Err(EngineError::Cancelled)), "{op}: {res:?}");
        assert!(!out.exists(), "{op} wrote output after cancel");
    }
}

#[test]
fn cancel_during_run_returns_cancelled() {
    let d = tempdir().unwrap();
    let token = CancelToken::new();
    let out = d.path().join("out.png");
    let t = task("convert", vec![fixture("alpha.png")], out.clone(), "");
    // Fires on the first progress report, after the start check.
    let progress = |_: f32| token.cancel();
    let res = ImageEngine::default().run(&t, &progress, &token);
    assert!(matches!(res, Err(EngineError::Cancelled)), "{res:?}");
    assert!(!out.exists());
}

// AC16
#[test]
fn missing_input_reports_its_path() {
    let d = tempdir().unwrap();
    let missing = d.path().join("nope.png");
    let t = task("convert", vec![missing.clone()], d.path().join("o.jpg"), "");
    match run_task(&t).0 {
        Err(EngineError::Input { path, reason }) => {
            assert_eq!(path, missing);
            assert!(!reason.is_empty());
        }
        other => panic!("expected Input, got {other:?}"),
    }
}

#[test]
fn corrupt_input_reports_its_path() {
    let d = tempdir().unwrap();
    let bad = d.path().join("bad.png");
    std::fs::write(&bad, b"this is not an image").unwrap();
    let t = task("grayscale", vec![bad.clone()], d.path().join("o.png"), "");
    match run_task(&t).0 {
        Err(EngineError::Input { path, .. }) => assert_eq!(path, bad),
        other => panic!("expected Input, got {other:?}"),
    }
    assert!(!d.path().join("o.png").exists());
}

#[test]
fn avif_input_is_reported_as_unsupported() {
    let d = tempdir().unwrap();
    let avif = d.path().join("in.avif");
    let mut bytes = Vec::new();
    image::codecs::avif::AvifEncoder::new_with_speed_quality(&mut bytes, 10, 50)
        .write_image(
            quadrants(16, 16).as_raw(),
            16,
            16,
            image::ExtendedColorType::Rgba8,
        )
        .unwrap();
    std::fs::write(&avif, bytes).unwrap();
    let t = task("convert", vec![avif.clone()], d.path().join("o.png"), "");
    match run_task(&t).0 {
        Err(EngineError::Input { path, reason }) => {
            assert_eq!(path, avif);
            assert!(reason.contains("AVIF"), "{reason}");
        }
        other => panic!("expected Input, got {other:?}"),
    }
}

#[test]
fn wrong_input_count() {
    let d = tempdir().unwrap();
    let out = d.path().join("o.png");
    let none = task("convert", vec![], out.clone(), "");
    assert!(
        matches!(run_task(&none).0, Err(EngineError::BadParam { name, .. }) if name == "inputs")
    );
    let two = task(
        "rotate",
        vec![fixture("alpha.png"), fixture("sample.gif")],
        out.clone(),
        "degrees = 90",
    );
    assert!(
        matches!(run_task(&two).0, Err(EngineError::BadParam { name, .. }) if name == "inputs")
    );
    let pdf = task("to-pdf", vec![], d.path().join("o.pdf"), "");
    assert!(
        matches!(run_task(&pdf).0, Err(EngineError::BadParam { name, .. }) if name == "inputs")
    );
    assert!(!out.exists());
}

#[test]
fn unknown_op() {
    let d = tempdir().unwrap();
    let t = task(
        "sharpen",
        vec![fixture("alpha.png")],
        d.path().join("o.png"),
        "",
    );
    assert!(matches!(run_task(&t).0, Err(EngineError::UnsupportedOp(op)) if op == "sharpen"));
}

use image::ImageEncoder;
