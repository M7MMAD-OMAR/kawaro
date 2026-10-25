//! actions/documents.toml parses and only names ops the office engine runs (S-009).

use kawaro_core::{ActionDef, Engine, MediaKind, OutputMode};
use kawaro_engines::office::OfficeEngine;

#[derive(serde::Deserialize)]
struct File {
    action: Vec<ActionDef>,
}

fn actions() -> Vec<ActionDef> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../actions/documents.toml");
    let text = std::fs::read_to_string(path).unwrap();
    toml::from_str::<File>(&text).unwrap().action
}

const WRITER: &[&str] = &["doc", "docx", "odt", "rtf", "txt", "md"];
const CALC: &[&str] = &["xls", "xlsx", "ods", "csv"];
const IMPRESS: &[&str] = &["ppt", "pptx", "odp"];

#[test]
fn office_actions_match_engine_capabilities() {
    let engine = OfficeEngine::default();
    let caps = engine.capabilities();
    let actions = actions();
    let mut ids: Vec<_> = actions.iter().map(|a| a.id.as_str()).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), actions.len(), "duplicate ids");
    for a in &actions {
        assert!(a.id.starts_with("documents."), "{}", a.id);
        assert_eq!(a.engine, "office", "{}", a.id);
        let (_, kinds) = caps
            .iter()
            .find(|(op, _)| *op == a.op)
            .unwrap_or_else(|| panic!("{} op {}", a.id, a.op));
        assert_eq!(a.accepts, [MediaKind::Document], "{}", a.id);
        assert!(kinds.contains(&MediaKind::Document));
        assert!(a.ring == 1 || a.ring == 2, "{}", a.id);
        assert_eq!(a.output.mode, OutputMode::PerFile, "{}", a.id);
        assert_eq!(a.min_files, 1, "{}", a.id);
        for text in [&a.code.ar, &a.code.en, &a.label.ar, &a.label.en] {
            assert!(
                !text.is_empty() && !text.contains(['\u{2014}', '\u{2013}']),
                "{}: {text}",
                a.id
            );
        }
    }
}

#[test]
fn office_actions_cover_pdf_and_each_conversion() {
    let actions = actions();
    let get = |id: &str| {
        actions
            .iter()
            .find(|a| a.id == id)
            .unwrap_or_else(|| panic!("missing {id}"))
    };

    let pdf = get("documents.to-pdf");
    assert_eq!(
        (pdf.op.as_str(), pdf.ring, pdf.output.ext.as_str()),
        ("to-pdf", 1, "pdf")
    );
    assert!(pdf.skip_ext.is_empty());

    // Each conversion hides itself for its own target and for the other
    // families, so a spreadsheet never offers DOCX.
    let families = [
        ("documents.to-docx", "convert-doc", "docx", WRITER),
        ("documents.to-odt", "convert-doc", "odt", WRITER),
        ("documents.to-xlsx", "convert-sheet", "xlsx", CALC),
        ("documents.to-ods", "convert-sheet", "ods", CALC),
        ("documents.to-pptx", "convert-slides", "pptx", IMPRESS),
        ("documents.to-odp", "convert-slides", "odp", IMPRESS),
    ];
    for (id, op, ext, family) in families {
        let a = get(id);
        assert_eq!(a.op, op, "{id}");
        assert_eq!(a.output.ext, ext, "{id}");
        assert!(a.skip_ext.iter().any(|e| e == ext), "{id} must skip {ext}");
        for other in WRITER.iter().chain(CALC).chain(IMPRESS) {
            let skipped = a.skip_ext.iter().any(|e| e == other);
            assert_eq!(
                skipped,
                !family.contains(other) || *other == ext,
                "{id} skip_ext {other}"
            );
        }
    }
}
