//! S-124 image engine: `to-pdf` page size, orientation, margin and dpi.

mod image_common;
use image_common::*;

use kawaro_core::options::{Kind, OptionsKind};
use kawaro_core::{ActionDef, EngineError};
use lopdf::{Document, Object};
use std::path::{Path, PathBuf};
use tempfile::tempdir;

/// A4 and Letter in points, portrait.
const A4: (f64, f64) = (595.276, 841.89);
const LETTER: (f64, f64) = (612.0, 792.0);
/// Millimetres to points.
const MM_TO_PT: f64 = 72.0 / 25.4;

fn number(o: &Object) -> f64 {
    match o {
        Object::Integer(i) => *i as f64,
        Object::Real(r) => f64::from(*r),
        other => panic!("expected a number, got {other:?}"),
    }
}

fn pages(doc: &Document) -> Vec<lopdf::ObjectId> {
    doc.get_pages().values().copied().collect()
}

/// The page MediaBox as `[x0, y0, x1, y1]`.
fn media_box(doc: &Document, page: usize) -> [f64; 4] {
    let id = pages(doc)[page];
    let values = doc
        .get_dictionary(id)
        .unwrap()
        .get(b"MediaBox")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(values.len(), 4);
    [
        number(&values[0]),
        number(&values[1]),
        number(&values[2]),
        number(&values[3]),
    ]
}

/// The image placement matrix `[a b c d e f]` from the content stream.
fn placement(doc: &Document, page: usize) -> [f64; 6] {
    let id = pages(doc)[page];
    let content = doc.get_page_content(id);
    let text = String::from_utf8_lossy(&content);
    let line = text
        .lines()
        .find(|line| line.trim_end().ends_with(" cm"))
        .unwrap_or_else(|| panic!("no cm operator in {text}"));
    let values: Vec<f64> = line
        .split_whitespace()
        .filter_map(|token| token.parse().ok())
        .collect();
    assert_eq!(values.len(), 6, "{line}");
    [
        values[0], values[1], values[2], values[3], values[4], values[5],
    ]
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() < 0.01
}

fn write_png(dir: &Path, name: &str, w: u32, h: u32) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, png_bytes(&quadrants(w, h))).unwrap();
    path
}

fn run_pdf_bytes(inputs: Vec<PathBuf>, params: &str) -> (Document, Vec<u8>) {
    let dir = tempdir().unwrap();
    let out = dir.path().join("out.pdf");
    let t = task("to-pdf", inputs, out.clone(), params);
    let (res, progress) = run_task(&t);
    res.unwrap_or_else(|e| panic!("to-pdf failed with {params:?}: {e}"));
    assert_progress_shape(&progress);
    let bytes = std::fs::read(&out).unwrap();
    (Document::load_mem(&bytes).unwrap(), bytes)
}

fn run_pdf(inputs: Vec<PathBuf>, params: &str) -> Document {
    run_pdf_bytes(inputs, params).0
}

#[test]
fn fit_keeps_one_point_per_pixel_by_default() {
    let dir = tempdir().unwrap();
    let wide = write_png(dir.path(), "wide.png", 64, 48);
    let tall = write_png(dir.path(), "tall.png", 48, 64);

    let doc = run_pdf(vec![wide.clone(), tall.clone()], "");
    assert_eq!(doc.get_pages().len(), 2);
    assert_eq!(media_box(&doc, 0), [0.0, 0.0, 64.0, 48.0]);
    assert_eq!(media_box(&doc, 1), [0.0, 0.0, 48.0, 64.0]);
    assert_eq!(placement(&doc, 0), [64.0, 0.0, 0.0, 48.0, 0.0, 0.0]);
    assert_eq!(placement(&doc, 1), [48.0, 0.0, 0.0, 64.0, 0.0, 0.0]);

    // The explicit defaults are the same page.
    let doc = run_pdf(
        vec![wide],
        "page = \"fit\"\npage_size = \"fit\"\norientation = \"auto\"\nmargin_mm = 0.0\ndpi = 72",
    );
    assert_eq!(media_box(&doc, 0), [0.0, 0.0, 64.0, 48.0]);
}

#[test]
fn a4_portrait_keeps_the_image_inside_the_margin() {
    let dir = tempdir().unwrap();
    let wide = write_png(dir.path(), "wide.png", 64, 48);
    let doc = run_pdf(
        vec![wide],
        "page_size = \"a4\"\norientation = \"portrait\"\nmargin_mm = 10.0",
    );

    let [x0, y0, x1, y1] = media_box(&doc, 0);
    assert_eq!((x0, y0), (0.0, 0.0));
    assert!(close(x1, A4.0), "A4 width {x1}");
    assert!(close(y1, A4.1), "A4 height {y1}");

    let margin = 10.0 * MM_TO_PT;
    let [w, _, _, h, x, y] = placement(&doc, 0);
    assert!(close(w, 64.0), "the image was enlarged: {w}");
    assert!(close(h, 48.0), "the image was enlarged: {h}");
    assert!(x >= margin, "left {x} is inside the margin {margin}");
    assert!(y >= margin, "bottom {y} is inside the margin {margin}");
    assert!(x + w <= x1 - margin, "right {} passes the margin", x + w);
    assert!(y + h <= y1 - margin, "top {} passes the margin", y + h);
    assert!(close(x, (A4.0 - 64.0) / 2.0), "not centred: {x}");
    assert!(close(y, (A4.1 - 48.0) / 2.0), "not centred: {y}");
}

#[test]
fn orientation_auto_and_forced_swap_the_page() {
    let dir = tempdir().unwrap();
    let wide = write_png(dir.path(), "wide.png", 64, 48);
    let tall = write_png(dir.path(), "tall.png", 48, 64);

    let doc = run_pdf(
        vec![wide.clone()],
        "page_size = \"a4\"\norientation = \"auto\"",
    );
    let [_, _, w, h, ..] = media_box(&doc, 0);
    assert!(close(w, A4.1), "auto on a wide image: {w} x {h}");
    assert!(close(h, A4.0), "auto on a wide image: {w} x {h}");

    let doc = run_pdf(vec![tall], "page_size = \"a4\"\norientation = \"auto\"");
    let [_, _, w, h, ..] = media_box(&doc, 0);
    assert!(close(w, A4.0), "auto on a tall image: {w} x {h}");
    assert!(close(h, A4.1), "auto on a tall image: {w} x {h}");

    let doc = run_pdf(
        vec![wide.clone()],
        "page_size = \"a4\"\norientation = \"landscape\"",
    );
    let [_, _, w, h, ..] = media_box(&doc, 0);
    assert!(close(w, A4.1), "forced landscape: {w} x {h}");
    assert!(close(h, A4.0), "forced landscape: {w} x {h}");

    let doc = run_pdf(
        vec![wide.clone()],
        "page_size = \"a4\"\norientation = \"portrait\"",
    );
    let [_, _, w, h, ..] = media_box(&doc, 0);
    assert!(close(w, A4.0), "forced portrait: {w} x {h}");
    assert!(close(h, A4.1), "forced portrait: {w} x {h}");

    let doc = run_pdf(vec![wide], "page_size = \"letter\"\norientation = \"auto\"");
    let [_, _, w, h, ..] = media_box(&doc, 0);
    assert!(close(w, LETTER.1), "letter landscape: {w} x {h}");
    assert!(close(h, LETTER.0), "letter landscape: {w} x {h}");
}

#[test]
fn dpi_144_halves_the_physical_image() {
    let dir = tempdir().unwrap();
    let wide = write_png(dir.path(), "wide.png", 64, 48);
    let doc = run_pdf(vec![wide], "page_size = \"fit\"\ndpi = 144");
    assert_eq!(media_box(&doc, 0), [0.0, 0.0, 32.0, 24.0]);
    assert_eq!(placement(&doc, 0), [32.0, 0.0, 0.0, 24.0, 0.0, 0.0]);
}

#[test]
fn jpeg_bytes_survive_a4_page_sizing() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("photo.jpg");
    let jpeg = jpeg_bytes(&quadrants(40, 20), 85, None);
    std::fs::write(&path, &jpeg).unwrap();

    let (doc, pdf) = run_pdf_bytes(
        vec![path],
        "page_size = \"a4\"\norientation = \"landscape\"\nmargin_mm = 5.0",
    );
    assert!(contains(&pdf, b"/DCTDecode"));
    assert!(contains(&pdf, &jpeg), "the JPEG was re-encoded");
    let [_, _, w, h, ..] = media_box(&doc, 0);
    assert!(close(w, A4.1), "landscape A4 width {w}");
    assert!(close(h, A4.0), "landscape A4 height {h}");
}

#[test]
fn bad_page_options_are_bad_params() {
    let dir = tempdir().unwrap();
    let wide = write_png(dir.path(), "wide.png", 64, 48);
    for (params, name) in [
        ("page_size = \"a3\"", "page_size"),
        ("page_size = \"A4\"", "page_size"),
        ("page_size = 5", "page_size"),
        ("orientation = \"sideways\"", "orientation"),
        ("orientation = \"Landscape\"", "orientation"),
        ("margin_mm = 60.0", "margin_mm"),
        ("margin_mm = 60", "margin_mm"),
        ("margin_mm = -1.0", "margin_mm"),
        ("dpi = 71", "dpi"),
        ("dpi = 601", "dpi"),
        ("dpi = 144.0", "dpi"),
    ] {
        let out = dir.path().join("out.pdf");
        let t = task("to-pdf", vec![wide.clone()], out.clone(), params);
        match run_task(&t).0 {
            Err(EngineError::BadParam { name: got, reason }) => {
                assert_eq!(got, name, "{params}");
                assert!(!reason.is_empty(), "{params}");
            }
            other => panic!("{params}: expected BadParam {name}, got {other:?}"),
        }
        assert!(!out.exists(), "{params} left an output");
    }
}

#[derive(serde::Deserialize)]
struct ActionsFile {
    action: Vec<ActionDef>,
}

fn image_actions() -> Vec<ActionDef> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../actions/images.toml");
    let text = std::fs::read_to_string(path).unwrap();
    toml::from_str::<ActionsFile>(&text).unwrap().action
}

#[test]
fn action_declares_the_four_pdf_options() {
    let actions = image_actions();
    let action = actions
        .iter()
        .find(|a| a.id == "image.to-pdf")
        .expect("image.to-pdf is missing");

    let text = |s: &str| toml::Value::String(s.to_string());
    assert_eq!(action.params.get("page"), Some(&text("fit")));
    assert_eq!(action.params.get("page_size"), Some(&text("fit")));
    assert_eq!(action.params.get("orientation"), Some(&text("auto")));
    assert_eq!(
        action.params.get("margin_mm"),
        Some(&toml::Value::Float(0.0))
    );
    assert_eq!(action.params.get("dpi"), Some(&toml::Value::Integer(72)));

    let options = action.options.as_ref().expect("to-pdf has no options");
    assert_eq!(options.kind, OptionsKind::Form);
    assert_eq!(options.fields.len(), 4);
    let field = |id: &str| {
        options
            .fields
            .iter()
            .find(|f| f.id == id)
            .unwrap_or_else(|| panic!("missing option {id}"))
    };

    let page_size = field("page_size");
    assert_eq!(page_size.kind, Kind::Enum);
    assert_eq!(page_size.param_key(), "page_size");
    assert_eq!(page_size.default, Some(text("fit")));
    assert_eq!(
        page_size
            .values
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>(),
        ["fit", "a4", "letter"]
    );

    let orientation = field("orientation");
    assert_eq!(orientation.kind, Kind::Enum);
    assert_eq!(orientation.param_key(), "orientation");
    assert_eq!(orientation.default, Some(text("auto")));
    assert_eq!(
        orientation
            .values
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>(),
        ["auto", "portrait", "landscape"]
    );

    let margin = field("margin_mm");
    assert_eq!(margin.kind, Kind::Float);
    assert_eq!(margin.param_key(), "margin_mm");
    assert_eq!(margin.default, Some(toml::Value::Float(0.0)));
    assert_eq!((margin.min, margin.max), (Some(0.0), Some(50.0)));
    assert_eq!(margin.unit.as_deref(), Some("mm"));

    let dpi = field("dpi");
    assert_eq!(dpi.kind, Kind::Int);
    assert_eq!(dpi.param_key(), "dpi");
    assert_eq!(dpi.default, Some(toml::Value::Integer(72)));
    assert_eq!((dpi.min, dpi.max), (Some(72.0), Some(600.0)));

    for f in &options.fields {
        assert!(!f.label.ar.is_empty(), "{} has no Arabic label", f.id);
        assert!(!f.label.en.is_empty(), "{} has no English label", f.id);
        for choice in &f.values {
            assert!(
                !choice.label.ar.is_empty(),
                "{} choice {} has no Arabic label",
                f.id,
                choice.id
            );
            assert!(
                !choice.label.en.is_empty(),
                "{} choice {} has no English label",
                f.id,
                choice.id
            );
        }
    }
}
