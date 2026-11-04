//! S-127: image engine modes, resize-box, crop and compress-to.
//!
//! Contract (tester): resize-box fits, fills then centre crops, or takes the
//! exact box; crop takes the largest window of an aspect at an anchor;
//! compress-to searches the quality for a byte target and refuses lossless
//! formats. The three actions in actions/images.toml carry the options.

mod image_common;
use image_common::*;

use image::{Rgb, RgbImage};
use kawaro_core::options::Field;
use kawaro_core::{ActionDef, EngineError, MediaKind, OutputMode};
use std::path::{Path, PathBuf};
use tempfile::tempdir;

const SHIPPED_IMAGE_ACTIONS: [&str; 16] = [
    "image.to-webp",
    "image.to-png",
    "image.to-jpg",
    "image.to-avif",
    "image.to-pdf",
    "image.to-ico",
    "image.resize-50",
    "image.fit-1920",
    "image.rotate-90",
    "image.flip-h",
    "image.grayscale",
    "image.compress",
    "image.strip",
    "image.to-gif",
    "image.to-tiff",
    "image.to-bmp",
];

fn quadrants_png(dir: &Path) -> PathBuf {
    let path = dir.join("quadrants.png");
    quadrants(400, 300).save(&path).unwrap();
    path
}

/// A 400x300 image whose grey follows y, so a crop row names its source row.
fn vertical_gradient_png(dir: &Path) -> PathBuf {
    let path = dir.join("vertical.png");
    RgbImage::from_fn(400, 300, |_x, y| {
        let v = (y * 255 / 299) as u8;
        Rgb([v, v, v])
    })
    .save(&path)
    .unwrap();
    path
}

/// A 400x300 image whose grey follows x, so a crop column names its source column.
fn horizontal_gradient_png(dir: &Path) -> PathBuf {
    let path = dir.join("horizontal.png");
    RgbImage::from_fn(400, 300, |x, _y| {
        let v = (x * 255 / 399) as u8;
        Rgb([v, v, v])
    })
    .save(&path)
    .unwrap();
    path
}

fn error_of(op: &str, input: &Path, dir: &Path, ext: &str, params: &str) -> EngineError {
    let out = dir.join(format!("error-{op}.{ext}"));
    let t = task(op, vec![input.to_path_buf()], out, params);
    run_task(&t).0.expect_err("the task must fail")
}

fn grey(img: &image::DynamicImage, x: u32, y: u32) -> u8 {
    rgb_at(img, x, y)[0]
}

#[derive(serde::Deserialize)]
struct File {
    action: Vec<ActionDef>,
}

fn image_actions() -> Vec<ActionDef> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../actions/images.toml");
    let text = std::fs::read_to_string(path).unwrap();
    toml::from_str::<File>(&text).unwrap().action
}

fn find<'a>(actions: &'a [ActionDef], id: &str) -> &'a ActionDef {
    actions
        .iter()
        .find(|a| a.id == id)
        .unwrap_or_else(|| panic!("missing {id}"))
}

fn field<'a>(action: &'a ActionDef, id: &str) -> &'a Field {
    action
        .options
        .as_ref()
        .expect("the action carries options")
        .fields
        .iter()
        .find(|f| f.id == id)
        .unwrap_or_else(|| panic!("{}: missing field {id}", action.id))
}

fn enum_ids(field: &Field) -> Vec<&str> {
    field.values.iter().map(|c| c.id.as_str()).collect()
}

#[test]
fn resize_box_fits_fills_and_takes_the_exact_box() {
    let dir = tempdir().unwrap();
    let input = quadrants_png(dir.path());

    let fit = decode(&run_one(
        "resize-box",
        &input,
        dir.path(),
        "png",
        "width = 200\nheight = 200\nmode = \"fit\"",
    ));
    assert_eq!((fit.width(), fit.height()), (200, 150));

    // The default mode is fit.
    let default_fit = decode(&run_one(
        "resize-box",
        &input,
        dir.path(),
        "png",
        "width = 200\nheight = 200",
    ));
    assert_eq!((default_fit.width(), default_fit.height()), (200, 150));

    let fill = decode(&run_one(
        "resize-box",
        &input,
        dir.path(),
        "png",
        "width = 200\nheight = 200\nmode = \"fill\"",
    ));
    assert_eq!((fill.width(), fill.height()), (200, 200));

    let exact = decode(&run_one(
        "resize-box",
        &input,
        dir.path(),
        "png",
        "width = 100\nheight = 50\nmode = \"exact\"",
    ));
    assert_eq!((exact.width(), exact.height()), (100, 50));
}

#[test]
fn resize_box_keeps_the_input_format() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("photo.jpg");
    std::fs::write(&input, jpeg_bytes(&quadrants(400, 300), 92, None)).unwrap();
    let out = run_one(
        "resize-box",
        &input,
        dir.path(),
        "jpg",
        "width = 200\nheight = 200\nmode = \"fit\"",
    );
    assert_eq!(&out[..2], &[0xFF, 0xD8], "the output stays JPEG");
    let image = decode(&out);
    assert_eq!((image.width(), image.height()), (200, 150));
}

#[test]
fn crop_takes_the_largest_window_of_the_aspect() {
    let dir = tempdir().unwrap();
    let input = quadrants_png(dir.path());

    let square = decode(&run_one(
        "crop",
        &input,
        dir.path(),
        "png",
        "aspect = \"1:1\"",
    ));
    assert_eq!((square.width(), square.height()), (300, 300));

    let square_top = decode(&run_one(
        "crop",
        &input,
        dir.path(),
        "png",
        "aspect = \"1:1\"\nanchor = \"top\"",
    ));
    assert_eq!((square_top.width(), square_top.height()), (300, 300));

    let wide = decode(&run_one(
        "crop",
        &input,
        dir.path(),
        "png",
        "aspect = \"16:9\"",
    ));
    assert_eq!((wide.width(), wide.height()), (400, 225));

    // 400x300 is exactly 4:3, so that window is the whole image.
    let four_three = decode(&run_one(
        "crop",
        &input,
        dir.path(),
        "png",
        "aspect = \"4:3\"",
    ));
    assert_eq!((four_three.width(), four_three.height()), (400, 300));

    // original, explicit or by default, keeps the whole image.
    let original = decode(&run_one(
        "crop",
        &input,
        dir.path(),
        "png",
        "aspect = \"original\"",
    ));
    assert_eq!((original.width(), original.height()), (400, 300));
    let default = decode(&run_one("crop", &input, dir.path(), "png", ""));
    assert_eq!((default.width(), default.height()), (400, 300));
}

#[test]
fn crop_anchors_move_the_window() {
    let dir = tempdir().unwrap();
    let vertical = vertical_gradient_png(dir.path());
    // The 16:9 window is 225 rows tall: top keeps row 0, bottom keeps row 75,
    // centre keeps row 37 or 38, so the first output row names the anchor.
    for (anchor, low, high) in [("top", 0, 1), ("center", 30, 33), ("bottom", 62, 65)] {
        let out = decode(&run_one(
            "crop",
            &vertical,
            dir.path(),
            "png",
            &format!("aspect = \"16:9\"\nanchor = \"{anchor}\""),
        ));
        assert_eq!((out.width(), out.height()), (400, 225), "{anchor}");
        let value = grey(&out, 0, 0);
        assert!(
            (low..=high).contains(&value),
            "{anchor}: first row grey {value}, want {low} to {high}"
        );
    }

    let horizontal = horizontal_gradient_png(dir.path());
    // The 1:1 window is 300 columns wide: left keeps column 0, right column
    // 100, centre column 50.
    for (anchor, low, high) in [("left", 0, 1), ("center", 30, 33), ("right", 62, 65)] {
        let out = decode(&run_one(
            "crop",
            &horizontal,
            dir.path(),
            "png",
            &format!("aspect = \"1:1\"\nanchor = \"{anchor}\""),
        ));
        let value = grey(&out, 0, 0);
        assert!(
            (low..=high).contains(&value),
            "{anchor}: first column grey {value}, want {low} to {high}"
        );
    }
}

#[test]
fn bad_parameters_are_refused() {
    let dir = tempdir().unwrap();
    let input = quadrants_png(dir.path());
    for params in [
        "width = 0\nheight = 100",
        "width = 20001\nheight = 100",
        "width = 100\nheight = 0",
        "width = 100\nheight = 20001",
        "width = 100\nheight = 100\nmode = \"stretch\"",
    ] {
        let error = error_of("resize-box", &input, dir.path(), "png", params);
        assert!(
            matches!(error, EngineError::BadParam { .. }),
            "{params}: {error:?}"
        );
    }
    for params in ["aspect = \"5:4\"", "anchor = \"middle\""] {
        let error = error_of("crop", &input, dir.path(), "png", params);
        assert!(
            matches!(error, EngineError::BadParam { .. }),
            "{params}: {error:?}"
        );
    }

    let jpeg = dir.path().join("photo.jpg");
    std::fs::write(&jpeg, jpeg_bytes(&quadrants(80, 60), 80, None)).unwrap();
    for params in ["target_bytes = 0", "target_bytes = 20000001"] {
        let error = error_of("compress-to", &jpeg, dir.path(), "jpg", params);
        assert!(
            matches!(error, EngineError::BadParam { .. }),
            "{params}: {error:?}"
        );
    }
}

#[test]
fn compress_to_reaches_the_byte_target_and_refuses_lossless_formats() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("noisy.jpg");
    RgbImage::from_fn(800, 600, |x, y| {
        Rgb([
            ((x * 37 + y * 17) % 256) as u8,
            ((x * 53 + y * 29) % 256) as u8,
            ((x * 11 + y * 47) % 256) as u8,
        ])
    })
    .save(&input)
    .unwrap();
    let before = std::fs::metadata(&input).unwrap().len();

    let out = run_one(
        "compress-to",
        &input,
        dir.path(),
        "jpg",
        "target_bytes = 60000",
    );
    assert!(
        out.len() as u64 <= 60_000,
        "the output is {} bytes, the target was 60000 (input {before})",
        out.len()
    );
    let image = decode(&out);
    assert_eq!((image.width(), image.height()), (800, 600));

    // Even quality 1 stays over a one byte target: fail and write nothing.
    let impossible = error_of("compress-to", &input, dir.path(), "jpg", "target_bytes = 1");
    assert!(
        matches!(impossible, EngineError::Failed(_)),
        "{impossible:?}"
    );
    assert!(
        !dir.path().join("error-compress-to.jpg").exists(),
        "an unreachable target leaves no output"
    );

    // PNG is lossless, so the op is refused.
    let png = quadrants_png(dir.path());
    let png_out = dir.path().join("refused.png");
    let t = task("compress-to", vec![png], png_out, "target_bytes = 60000");
    let (result, _) = run_task(&t);
    assert!(
        matches!(result, Err(EngineError::UnsupportedOp(_))),
        "{result:?}"
    );
}

#[test]
fn the_three_actions_declare_their_options() {
    let actions = image_actions();
    assert_eq!(
        actions.len(),
        19,
        "the 16 shipped image actions plus the 3 new ones"
    );
    for id in SHIPPED_IMAGE_ACTIONS {
        assert!(
            actions.iter().any(|a| a.id == id),
            "the shipped action {id} is still there"
        );
    }

    let resize = find(&actions, "image.resize-to");
    assert_eq!(resize.engine, "image");
    assert_eq!(resize.op, "resize-box");
    assert_eq!(resize.ring, 2);
    assert_eq!(resize.accepts, [MediaKind::Image]);
    assert_eq!(resize.output.mode, OutputMode::PerFile);
    assert_eq!(resize.output.ext, "");
    assert_eq!(resize.output.suffix, "-resized");
    assert_eq!(resize.output.dir, "");
    assert_eq!(resize.icon, "fit");
    let width = field(resize, "width");
    assert_eq!(width.kind, kawaro_core::options::Kind::Int);
    assert_eq!(width.min, Some(16.0));
    assert_eq!(width.max, Some(20000.0));
    assert_eq!(
        width.default.as_ref().and_then(toml::Value::as_integer),
        Some(1920)
    );
    assert_eq!(width.maps_to.as_deref(), Some("width"));
    let height = field(resize, "height");
    assert_eq!(height.kind, kawaro_core::options::Kind::Int);
    assert_eq!(height.min, Some(16.0));
    assert_eq!(height.max, Some(20000.0));
    assert_eq!(
        height.default.as_ref().and_then(toml::Value::as_integer),
        Some(1080)
    );
    let mode = field(resize, "mode");
    assert_eq!(mode.kind, kawaro_core::options::Kind::Enum);
    assert_eq!(enum_ids(mode), ["fit", "fill", "exact"]);
    assert_eq!(
        mode.default.as_ref().and_then(toml::Value::as_str),
        Some("fit")
    );
    assert_eq!(mode.maps_to.as_deref(), Some("mode"));

    let crop = find(&actions, "image.crop");
    assert_eq!(crop.op, "crop");
    assert_eq!(crop.ring, 2);
    assert_eq!(crop.output.suffix, "-crop");
    assert_eq!(crop.output.ext, "");
    assert_eq!(crop.code.en, "CROP");
    assert_eq!(crop.icon, "resize");
    let aspect = field(crop, "aspect");
    assert_eq!(aspect.kind, kawaro_core::options::Kind::Enum);
    assert_eq!(enum_ids(aspect), ["1:1", "4:3", "16:9", "3:2", "original"]);
    assert_eq!(
        aspect.default.as_ref().and_then(toml::Value::as_str),
        Some("original")
    );
    assert_eq!(aspect.maps_to.as_deref(), Some("aspect"));
    let anchor = field(crop, "anchor");
    assert_eq!(anchor.kind, kawaro_core::options::Kind::Enum);
    assert_eq!(
        enum_ids(anchor),
        ["center", "top", "bottom", "left", "right"]
    );
    assert_eq!(
        anchor.default.as_ref().and_then(toml::Value::as_str),
        Some("center")
    );
    assert_eq!(anchor.maps_to.as_deref(), Some("anchor"));

    let compress = find(&actions, "image.compress-to");
    assert_eq!(compress.op, "compress-to");
    assert_eq!(compress.ring, 2);
    assert_eq!(compress.output.suffix, "-small");
    assert_eq!(compress.output.ext, "");
    assert_eq!(compress.code.en, "SIZE");
    assert_eq!(compress.icon, "compress");
    let target = field(compress, "target_bytes");
    assert_eq!(target.kind, kawaro_core::options::Kind::Int);
    assert_eq!(target.min, Some(1.0));
    assert_eq!(
        compress.options.as_ref().unwrap().kind,
        kawaro_core::options::OptionsKind::Form
    );
    assert_eq!(
        compress
            .options
            .as_ref()
            .unwrap()
            .fields
            .iter()
            .find(|f| f.id == "target_bytes")
            .unwrap()
            .max,
        Some(20_000_000.0)
    );
    assert_eq!(
        target.default.as_ref().and_then(toml::Value::as_integer),
        Some(500_000)
    );
    assert_eq!(target.unit.as_deref(), Some("bytes"));
    assert_eq!(target.maps_to.as_deref(), Some("target_bytes"));

    // Every label and choice has Arabic and English, and every spec validates.
    for action in [resize, crop, compress] {
        let options = action.options.as_ref().expect("options");
        options
            .validate()
            .unwrap_or_else(|error| panic!("{}: {error}", action.id));
        for field in &options.fields {
            assert!(
                !field.label.ar.is_empty() && !field.label.en.is_empty(),
                "{} field {} label",
                action.id,
                field.id
            );
            for choice in &field.values {
                assert!(
                    !choice.label.ar.is_empty() && !choice.label.en.is_empty(),
                    "{} choice {}",
                    action.id,
                    choice.id
                );
            }
        }
    }
}
