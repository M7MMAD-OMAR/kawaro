//! S-129: image metadata preservation, the keep_metadata option.
//!
//! Contract (tester): keep_metadata copies the Exif, XMP and ICC APP
//! segments of the input into the output once; without it the output has no
//! metadata; a PNG source keeps working; corrupt metadata is ignored without
//! a panic; and every metadata-capable action declares the option.

mod image_common;
use image_common::*;

use kawaro_core::ActionDef;
use kawaro_core::EngineError;
use kawaro_core::options::Kind;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

const EXIF: &[u8] = b"Exif\0\0";
const XMP: &[u8] = b"KAWARO-XMP";
const ICC_ONE: &[u8] = b"KAWARO-ICC-PROFILE-ONE";
const ICC_TWO: &[u8] = b"KAWARO-ICC-PROFILE-TWO";
const ICC_PREFIX: &[u8] = b"ICC_PROFILE\0";

fn joined(parts: &[&[u8]]) -> Vec<u8> {
    let mut payload = Vec::new();
    for part in parts {
        payload.extend_from_slice(part);
    }
    payload
}

/// One JPEG APP segment: marker, big endian length that counts itself, payload.
fn app_segment(marker: u8, payload: &[u8]) -> Vec<u8> {
    let mut segment = vec![0xFF, marker];
    segment.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
    segment.extend_from_slice(payload);
    segment
}

/// The JPEG with the given segments inserted right after the SOI marker.
fn with_segments(image: &[u8], segments: &[Vec<u8>]) -> Vec<u8> {
    let mut out = image[..2].to_vec();
    for segment in segments {
        out.extend_from_slice(segment);
    }
    out.extend_from_slice(&image[2..]);
    out
}

/// A JPEG carrying Exif, XMP and two ICC APP2 segments with unique payloads.
fn metadata_jpeg() -> Vec<u8> {
    let base = jpeg_bytes(&quadrants(64, 48), 92, None);
    let xmp = app_segment(0xE1, &joined(&[b"http://ns.adobe.com/xap/1.0/\0", XMP]));
    let icc_one = app_segment(0xE2, &joined(&[ICC_PREFIX, &[1, 2], ICC_ONE]));
    let icc_two = app_segment(0xE2, &joined(&[ICC_PREFIX, &[2, 2], ICC_TWO]));
    with_segments(&base, &[exif_orientation_segment(1), xmp, icc_one, icc_two])
}

fn written(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
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

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn read_u16(bytes: &[u8], at: usize, little: bool) -> Option<u16> {
    let raw = [*bytes.get(at)?, *bytes.get(at + 1)?];
    Some(if little {
        u16::from_le_bytes(raw)
    } else {
        u16::from_be_bytes(raw)
    })
}

fn read_u32(bytes: &[u8], at: usize, little: bool) -> Option<u32> {
    let raw = [
        *bytes.get(at)?,
        *bytes.get(at + 1)?,
        *bytes.get(at + 2)?,
        *bytes.get(at + 3)?,
    ];
    Some(if little {
        u32::from_le_bytes(raw)
    } else {
        u32::from_be_bytes(raw)
    })
}

/// The Orientation tag (0x0112) of the first Exif APP1 segment.
fn exif_orientation(jpeg: &[u8]) -> Option<u16> {
    let tiff = &jpeg[find_bytes(jpeg, EXIF)? + EXIF.len()..];
    let little = match tiff.get(..4)? {
        b"II\x2a\x00" => true,
        b"MM\x00\x2a" => false,
        _ => return None,
    };
    let ifd = read_u32(tiff, 4, little)? as usize;
    let entries = read_u16(tiff, ifd, little)? as usize;
    for index in 0..entries {
        let entry = ifd + 2 + index * 12;
        if read_u16(tiff, entry, little)? == 0x0112 {
            return read_u16(tiff, entry + 8, little);
        }
    }
    None
}

#[test]
fn keep_metadata_copies_exif_xmp_and_both_icc_segments_once() {
    let dir = tempdir().unwrap();
    let input = written(dir.path(), "metadata.jpg", &metadata_jpeg());
    let out = run_one(
        "resize",
        &input,
        dir.path(),
        "jpg",
        "scale = 50\nkeep_metadata = true",
    );
    let image = decode(&out);
    assert_eq!((image.width(), image.height()), (32, 24));
    for (name, needle) in [
        ("Exif", EXIF),
        ("XMP", XMP),
        ("ICC one", ICC_ONE),
        ("ICC two", ICC_TWO),
    ] {
        assert_eq!(
            count(&out, needle),
            1,
            "{name} must appear once in the output"
        );
    }
    assert_eq!(
        count(&out, ICC_PREFIX),
        2,
        "both ICC APP2 segments are kept"
    );
}

#[test]
fn metadata_is_dropped_by_default() {
    let dir = tempdir().unwrap();
    let input = written(dir.path(), "metadata.jpg", &metadata_jpeg());
    let out = run_one("resize", &input, dir.path(), "jpg", "scale = 50");
    let image = decode(&out);
    assert_eq!((image.width(), image.height()), (32, 24));
    for (name, needle) in [
        ("Exif", EXIF),
        ("XMP", XMP),
        ("ICC one", ICC_ONE),
        ("ICC two", ICC_TWO),
        ("ICC prefix", ICC_PREFIX),
    ] {
        assert_eq!(count(&out, needle), 0, "{name} must not be copied");
    }
}

#[test]
fn a_png_source_with_keep_metadata_succeeds_without_segments() {
    let dir = tempdir().unwrap();
    let input = written(dir.path(), "source.png", &png_bytes(&quadrants(64, 48)));
    let out = run_one("convert", &input, dir.path(), "jpg", "keep_metadata = true");
    let image = decode(&out);
    assert_eq!((image.width(), image.height()), (64, 48));
    for needle in [EXIF, XMP, ICC_ONE, ICC_TWO, ICC_PREFIX] {
        assert_eq!(count(&out, needle), 0, "a PNG carries no JPEG metadata");
    }
}

#[test]
fn corrupt_metadata_is_ignored_without_a_panic() {
    let dir = tempdir().unwrap();
    let base = jpeg_bytes(&quadrants(64, 48), 92, None);
    let damaged = app_segment(0xE1, b"Exif\0\0not a tiff header");
    let input = written(dir.path(), "damaged.jpg", &with_segments(&base, &[damaged]));
    let out = run_one(
        "resize",
        &input,
        dir.path(),
        "jpg",
        "scale = 50\nkeep_metadata = true",
    );
    let image = decode(&out);
    assert_eq!((image.width(), image.height()), (32, 24));
}

#[test]
fn every_metadata_capable_action_declares_keep_metadata() {
    let actions = image_actions();
    for id in [
        "image.to-jpg",
        "image.resize-50",
        "image.compress",
        "image.crop",
        "image.resize-to",
        "image.compress-to",
    ] {
        let action = actions
            .iter()
            .find(|action| action.id == id)
            .unwrap_or_else(|| panic!("missing {id}"));
        let options = action
            .options
            .as_ref()
            .unwrap_or_else(|| panic!("{id} carries options"));
        let field = options
            .fields
            .iter()
            .find(|field| field.id == "keep_metadata")
            .unwrap_or_else(|| panic!("{id}: missing keep_metadata"));
        assert_eq!(field.kind, Kind::Bool, "{id}");
        assert_eq!(
            field.default.as_ref().and_then(toml::Value::as_bool),
            Some(false),
            "{id}"
        );
        assert_eq!(field.maps_to.as_deref(), Some("keep_metadata"), "{id}");
        assert!(
            !field.label.ar.is_empty() && !field.label.en.is_empty(),
            "{id}"
        );
        options
            .validate()
            .unwrap_or_else(|error| panic!("{id}: {error}"));
    }
}

#[test]
fn orientation_six_metadata_is_kept_and_normalised_to_one() {
    let dir = tempdir().unwrap();
    let base = jpeg_bytes(&quadrants(64, 48), 92, Some(6));
    let xmp = app_segment(0xE1, &joined(&[b"http://ns.adobe.com/xap/1.0/\0", XMP]));
    let icc_one = app_segment(0xE2, &joined(&[ICC_PREFIX, &[1, 2], ICC_ONE]));
    let icc_two = app_segment(0xE2, &joined(&[ICC_PREFIX, &[2, 2], ICC_TWO]));
    let input = written(
        dir.path(),
        "camera.jpg",
        &with_segments(&base, &[xmp, icc_one, icc_two]),
    );
    let out = run_one(
        "resize",
        &input,
        dir.path(),
        "jpg",
        "scale = 50\nkeep_metadata = true",
    );
    let image = decode(&out);
    // Orientation 6 turns 64x48 into 48x64 before the 50 percent resize.
    assert_eq!((image.width(), image.height()), (24, 32));
    assert_eq!(count(&out, EXIF), 1, "the Exif APP1 is kept, not dropped");
    assert_eq!(
        exif_orientation(&out),
        Some(1),
        "the copied Exif carries the normalised orientation 1"
    );
    for (name, needle) in [("XMP", XMP), ("ICC one", ICC_ONE), ("ICC two", ICC_TWO)] {
        assert_eq!(count(&out, needle), 1, "{name} is kept once");
    }
    assert_eq!(count(&out, ICC_PREFIX), 2, "both ICC segments are kept");
}

#[test]
fn keep_metadata_must_be_a_boolean() {
    let dir = tempdir().unwrap();
    let input = written(dir.path(), "metadata.jpg", &metadata_jpeg());
    let output = dir.path().join("bad.jpg");
    let task = task(
        "resize",
        vec![input],
        output.clone(),
        "scale = 50\nkeep_metadata = \"yes\"",
    );
    let (result, _) = run_task(&task);
    match result {
        Err(EngineError::BadParam { name, .. }) => assert_eq!(name, "keep_metadata"),
        other => panic!("expected BadParam keep_metadata, got {other:?}"),
    }
    assert!(!output.exists(), "a refused value writes no output");
}
