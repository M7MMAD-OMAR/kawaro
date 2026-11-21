//! S-142: the transformed image preview, the engine side.
//!
//! Contract (tester): the encoded preview reports the intended output size,
//! counts the exact length of the artifact including its metadata, agrees
//! with the bytes the export writes, and writes no file of its own. The
//! source stays as it was. A source above the policy is refused, a
//! cancelled request stops, and the draft mode shows a bounded proxy of
//! the result with the geometry of the operation and no byte claim.
//!
//! This file is source only. It was written during the source
//! implementation and it was never executed.

mod image_common;
use image_common::*;

use kawaro_core::action::{Localized, OutputSpec};
use kawaro_core::image_preview::{
    ImagePreviewError, ImagePreviewMode, ImagePreviewProvider, ImagePreviewRequest, PreviewStamp,
};
use kawaro_core::{ActionDef, CancelToken, MediaKind, OutputMode};
use kawaro_engines::image::ImageEngine;
use std::path::Path;
use tempfile::tempdir;

fn action(op: &str, out_ext: &str, params: &str) -> ActionDef {
    ActionDef {
        id: format!("test.{op}"),
        engine: "image".into(),
        op: op.into(),
        ring: 1,
        order: 0,
        accepts: vec![MediaKind::Image],
        skip_ext: vec![],
        min_files: 1,
        max_files: 0,
        output: OutputSpec {
            mode: OutputMode::PerFile,
            ext: out_ext.into(),
            suffix: String::new(),
            dir: String::new(),
        },
        code: Localized {
            ar: "T".into(),
            en: "T".into(),
        },
        label: Localized {
            ar: "T".into(),
            en: "T".into(),
        },
        icon: String::new(),
        params: toml::from_str(params).unwrap(),
        options: None,
    }
}

/// The same action, on another engine.
fn foreign_action(op: &str, out_ext: &str, params: &str) -> ActionDef {
    let mut action = action(op, out_ext, params);
    action.engine = "pdf".into();
    action
}

fn stamp() -> PreviewStamp {
    PreviewStamp {
        session: 1,
        panel: 2,
        request: 3,
    }
}

fn request(action: ActionDef, input: &Path, mode: ImagePreviewMode) -> ImagePreviewRequest {
    ImagePreviewRequest {
        stamp: stamp(),
        input: input.to_path_buf(),
        resolved_action: action,
        mode,
    }
}

fn preview(
    action: ActionDef,
    input: &Path,
    mode: ImagePreviewMode,
) -> Result<kawaro_core::image_preview::ImagePreviewArtifact, ImagePreviewError> {
    ImageEngine::default().preview(&request(action, input, mode), &CancelToken::new())
}

/// A 400x200 PNG source in a fresh folder.
fn source(dir: &Path, name: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, png_bytes(&quadrants(400, 200))).unwrap();
    path
}

/// Every file name in `dir`, sorted.
fn names(dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    out.sort();
    out
}

/// The colour of a pixel of the display buffer.
fn pixel(artifact: &kawaro_core::image_preview::ImagePreviewArtifact, x: u32, y: u32) -> [u8; 3] {
    let index = (y as usize * artifact.pixels.width as usize + x as usize) * 4;
    [
        artifact.pixels.rgba8[index],
        artifact.pixels.rgba8[index + 1],
        artifact.pixels.rgba8[index + 2],
    ]
}

#[test]
fn the_encoded_preview_reports_the_intended_output_size() {
    let dir = tempdir().unwrap();
    let input = source(dir.path(), "in.png");

    let artifact = preview(
        action("resize", "png", "scale = 50"),
        &input,
        ImagePreviewMode::EncodedResult,
    )
    .unwrap();

    assert_eq!(artifact.output_width, 200);
    assert_eq!(artifact.output_height, 100);
    assert_eq!(artifact.output_format, "png");
    assert_eq!(artifact.mode, ImagePreviewMode::EncodedResult);
    assert_eq!(artifact.stamp, stamp());
    assert!(artifact.final_encoded_bytes.is_some());
    // The display buffer is bounded and matches its own size.
    assert!(artifact.pixels.width.max(artifact.pixels.height) <= 512);
    assert_eq!(
        artifact.pixels.rgba8.len() as u64,
        u64::from(artifact.pixels.width) * u64::from(artifact.pixels.height) * 4
    );
    // The display size and the intended output size are different things.
    assert!(artifact.pixels.width <= artifact.output_width.max(artifact.output_height));
}

#[test]
fn the_preview_writes_no_file() {
    let dir = tempdir().unwrap();
    let input = source(dir.path(), "in.png");

    preview(
        action("resize", "png", "scale = 50"),
        &input,
        ImagePreviewMode::EncodedResult,
    )
    .unwrap();

    assert_eq!(names(dir.path()), vec!["in.png".to_string()]);
}

#[test]
fn the_source_is_not_touched() {
    let dir = tempdir().unwrap();
    let input = source(dir.path(), "in.png");
    let before = std::fs::read(&input).unwrap();

    preview(
        action("resize", "png", "scale = 50"),
        &input,
        ImagePreviewMode::EncodedResult,
    )
    .unwrap();

    assert_eq!(before, std::fs::read(&input).unwrap());
}

#[test]
fn the_counted_bytes_are_the_bytes_the_export_writes() {
    let dir = tempdir().unwrap();
    let input = source(dir.path(), "in.png");

    let artifact = preview(
        action("resize", "png", "scale = 50"),
        &input,
        ImagePreviewMode::EncodedResult,
    )
    .unwrap();

    // The same action, run through the engine, writes the same bytes.
    let out = dir.path().join("out.png");
    let t = task("resize", vec![input.clone()], out.clone(), "scale = 50");
    let (res, _) = run_task(&t);
    res.unwrap();
    let written = std::fs::read(&out).unwrap();
    assert_eq!(artifact.final_encoded_bytes, Some(written.len() as u64));
}

#[test]
fn the_metadata_is_inside_the_counted_bytes() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("in.jpg");
    // A JPEG with an EXIF orientation tag.
    std::fs::write(&input, jpeg_bytes(&quadrants(400, 200), 90, Some(6))).unwrap();

    let artifact = preview(
        action("convert", "jpg", "quality = 80\nkeep_metadata = true"),
        &input,
        ImagePreviewMode::EncodedResult,
    )
    .unwrap();

    let out = dir.path().join("out.jpg");
    let t = task(
        "convert",
        vec![input.clone()],
        out.clone(),
        "quality = 80\nkeep_metadata = true",
    );
    let (res, _) = run_task(&t);
    res.unwrap();
    let written = std::fs::read(&out).unwrap();
    // The metadata is in the file and in the count.
    assert!(contains(&written, b"Exif\0\0"));
    assert_eq!(artifact.final_encoded_bytes, Some(written.len() as u64));
}

#[test]
fn a_source_above_the_policy_is_refused() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("big.png");
    // A sparse file above 64 MiB: the size check refuses it before any
    // decode, and nothing is written.
    let file = std::fs::File::create(&input).unwrap();
    file.set_len(64 * 1024 * 1024 + 1).unwrap();
    drop(file);

    let error = preview(
        action("convert", "png", ""),
        &input,
        ImagePreviewMode::EncodedResult,
    )
    .unwrap_err();
    assert!(matches!(error, ImagePreviewError::PolicyLimit), "{error:?}");
    assert_eq!(names(dir.path()), vec!["big.png".to_string()]);
}

#[test]
fn a_cancelled_request_stops() {
    let dir = tempdir().unwrap();
    let input = source(dir.path(), "in.png");
    let token = CancelToken::new();
    token.cancel();

    let error = ImageEngine::default()
        .preview(
            &request(
                action("resize", "png", "scale = 50"),
                &input,
                ImagePreviewMode::EncodedResult,
            ),
            &token,
        )
        .unwrap_err();
    assert!(matches!(error, ImagePreviewError::Cancelled), "{error:?}");
}

#[test]
fn images_to_pdf_has_no_preview() {
    let dir = tempdir().unwrap();
    let input = source(dir.path(), "in.png");

    let error = preview(
        action("to-pdf", "pdf", ""),
        &input,
        ImagePreviewMode::EncodedResult,
    )
    .unwrap_err();
    assert!(
        matches!(error, ImagePreviewError::Unsupported(_)),
        "{error:?}"
    );
}

#[test]
fn another_engine_action_has_no_image_preview() {
    let dir = tempdir().unwrap();
    let input = source(dir.path(), "in.png");

    for mode in [
        ImagePreviewMode::TransformedDraft,
        ImagePreviewMode::EncodedResult,
    ] {
        let error = preview(foreign_action("convert", "png", ""), &input, mode).unwrap_err();
        assert!(
            matches!(error, ImagePreviewError::Unsupported(_)),
            "{error:?}"
        );
    }
}

#[test]
fn the_draft_reports_the_geometry_of_the_operation() {
    let dir = tempdir().unwrap();
    let input = source(dir.path(), "in.png");

    let artifact = preview(
        action("resize", "png", "scale = 50"),
        &input,
        ImagePreviewMode::TransformedDraft,
    )
    .unwrap();

    // The geometry comes from the oriented original, not from the proxy.
    assert_eq!(artifact.output_width, 200);
    assert_eq!(artifact.output_height, 100);
    assert_eq!(artifact.output_format, "png");
    assert_eq!(artifact.mode, ImagePreviewMode::TransformedDraft);
    assert_eq!(artifact.stamp, stamp());
    // A draft is not an encoded artifact: it makes no byte claim.
    assert_eq!(artifact.final_encoded_bytes, None);
    // The proxy is bounded and matches its own size.
    assert!(artifact.pixels.width.max(artifact.pixels.height) <= 512);
    assert_eq!(
        artifact.pixels.rgba8.len() as u64,
        u64::from(artifact.pixels.width) * u64::from(artifact.pixels.height) * 4
    );
}

#[test]
fn the_draft_writes_no_file() {
    let dir = tempdir().unwrap();
    let input = source(dir.path(), "in.png");
    let before = std::fs::read(&input).unwrap();

    preview(
        action("resize", "png", "scale = 50"),
        &input,
        ImagePreviewMode::TransformedDraft,
    )
    .unwrap();

    assert_eq!(names(dir.path()), vec!["in.png".to_string()]);
    assert_eq!(before, std::fs::read(&input).unwrap());
}

#[test]
fn a_tiny_source_cannot_ask_for_a_huge_result() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("tiny.png");
    std::fs::write(&input, png_bytes(&quadrants(8, 8))).unwrap();

    for mode in [
        ImagePreviewMode::TransformedDraft,
        ImagePreviewMode::EncodedResult,
    ] {
        // 20000 x 20000 is refused before the operation is applied and
        // before anything is allocated.
        let error = preview(
            action(
                "resize-box",
                "png",
                "width = 20000\nheight = 20000\nmode = \"exact\"",
            ),
            &input,
            mode,
        )
        .unwrap_err();
        assert!(matches!(error, ImagePreviewError::PolicyLimit), "{error:?}");
    }
}

#[test]
fn the_export_has_no_preview_limit() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("tiny.png");
    std::fs::write(&input, png_bytes(&quadrants(8, 8))).unwrap();

    // A result above the preview policy is a normal export. The preview
    // refuses it, the export writes it.
    let out = dir.path().join("out.png");
    let t = task(
        "resize-box",
        vec![input.clone()],
        out.clone(),
        "width = 4000\nheight = 4001\nmode = \"exact\"",
    );
    let (res, _) = run_task(&t);
    res.unwrap();
    let written = decode(&std::fs::read(&out).unwrap());
    assert_eq!((written.width(), written.height()), (4000, 4001));

    let error = preview(
        action(
            "resize-box",
            "png",
            "width = 4000\nheight = 4001\nmode = \"exact\"",
        ),
        &input,
        ImagePreviewMode::TransformedDraft,
    )
    .unwrap_err();
    assert!(matches!(error, ImagePreviewError::PolicyLimit), "{error:?}");
}

#[test]
fn the_draft_follows_the_exif_orientation() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("in.jpg");
    // A 400x200 JPEG with the orientation that turns it on its side.
    std::fs::write(&input, jpeg_bytes(&quadrants(400, 200), 90, Some(6))).unwrap();

    let artifact = preview(
        action("convert", "jpg", ""),
        &input,
        ImagePreviewMode::TransformedDraft,
    )
    .unwrap();

    // The oriented original is 200x400, and so is the result.
    assert_eq!(artifact.output_width, 200);
    assert_eq!(artifact.output_height, 400);
    assert_eq!(artifact.pixels.width, 200);
    assert_eq!(artifact.pixels.height, 400);
}

#[test]
fn the_draft_cuts_the_same_window_as_the_export() {
    let dir = tempdir().unwrap();
    let input = source(dir.path(), "in.png");

    // A square window of a 400x200 picture: the centre window starts at
    // x = 100 and reaches the green quadrant, the left window stays in
    // the red one.
    let centre = preview(
        action("crop", "png", "aspect = \"1:1\"\nanchor = \"center\""),
        &input,
        ImagePreviewMode::TransformedDraft,
    )
    .unwrap();
    assert_eq!((centre.output_width, centre.output_height), (200, 200));
    assert_eq!(centre.pixels.width, 200);
    assert_eq!(centre.pixels.height, 200);
    assert!(near(pixel(&centre, 199, 0), GREEN, 4));

    let left = preview(
        action("crop", "png", "aspect = \"1:1\"\nanchor = \"left\""),
        &input,
        ImagePreviewMode::TransformedDraft,
    )
    .unwrap();
    assert_eq!((left.output_width, left.output_height), (200, 200));
    assert!(near(pixel(&left, 199, 0), RED, 4));
}

#[test]
fn a_byte_target_has_no_draft() {
    let dir = tempdir().unwrap();
    let input = source(dir.path(), "in.png");

    // A byte target is the result of a search over encodes. A proxy
    // cannot say what it weighs.
    let error = preview(
        action("compress-to", "jpg", "target_bytes = 1000"),
        &input,
        ImagePreviewMode::TransformedDraft,
    )
    .unwrap_err();
    assert!(
        matches!(error, ImagePreviewError::Unsupported(_)),
        "{error:?}"
    );
    // The encoded preview still runs the bounded search.
    let artifact = preview(
        action("compress-to", "jpg", "target_bytes = 1000"),
        &input,
        ImagePreviewMode::EncodedResult,
    )
    .unwrap();
    assert!(artifact.final_encoded_bytes.unwrap() <= 1000);
    // The reported size is the size of the artifact, not of the source.
    assert!(artifact.output_width <= 400);
    assert!(artifact.output_height <= 200);
}

#[test]
fn an_icon_target_has_no_draft() {
    let dir = tempdir().unwrap();
    let input = source(dir.path(), "in.png");

    // An icon carries the picture at several sizes, so its geometry is
    // not the geometry of one picture.
    let error = preview(
        action("convert", "ico", ""),
        &input,
        ImagePreviewMode::TransformedDraft,
    )
    .unwrap_err();
    assert!(
        matches!(error, ImagePreviewError::Unsupported(_)),
        "{error:?}"
    );
}

#[test]
fn a_cancelled_draft_stops() {
    let dir = tempdir().unwrap();
    let input = source(dir.path(), "in.png");
    let token = CancelToken::new();
    token.cancel();

    let error = ImageEngine::default()
        .preview(
            &request(
                action("resize", "png", "scale = 50"),
                &input,
                ImagePreviewMode::TransformedDraft,
            ),
            &token,
        )
        .unwrap_err();
    assert!(matches!(error, ImagePreviewError::Cancelled), "{error:?}");
}
