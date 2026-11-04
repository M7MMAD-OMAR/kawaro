//! actions/pdf.toml parses and only names ops the PDF engines run (S-010).

use kawaro_core::{ActionDef, Engine, MediaKind, OutputMode};
use kawaro_engines::pdf::{GhostscriptEngine, PdfEngine};
use kawaro_engines::poppler::PopplerEngine;
use kawaro_engines::qpdf::QpdfEngine;

#[derive(serde::Deserialize)]
struct File {
    action: Vec<ActionDef>,
}

fn actions() -> Vec<ActionDef> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../actions/pdf.toml");
    let text = std::fs::read_to_string(path).unwrap();
    toml::from_str::<File>(&text).unwrap().action
}

#[test]
fn pdf_actions_match_engine_capabilities() {
    let engines: [&dyn Engine; 4] = [
        &PdfEngine,
        &GhostscriptEngine::default(),
        &PopplerEngine::default(),
        &QpdfEngine::default(),
    ];
    for a in actions() {
        assert!(a.id.starts_with("pdf."), "{}", a.id);
        let engine = engines
            .iter()
            .find(|e| e.id() == a.engine)
            .unwrap_or_else(|| panic!("{} engine", a.id));
        assert!(
            engine.capabilities().iter().any(|(op, _)| *op == a.op),
            "{} op {}",
            a.id,
            a.op
        );
        assert_eq!(a.accepts, [MediaKind::Pdf], "{}", a.id);
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
fn pdf_actions_have_the_planned_rings_outputs_and_codes() {
    let actions = actions();
    let want = [
        (
            "pdf.compress",
            "ghostscript",
            "compress",
            1,
            OutputMode::PerFile,
            "",
            "-compressed",
            1,
            "SHRINK",
            "ضغط",
        ),
        (
            "pdf.merge",
            "pdf",
            "merge",
            1,
            OutputMode::Combined,
            "pdf",
            "-merged",
            2,
            "MERGE",
            "دمج",
        ),
        (
            "pdf.to-images",
            "ghostscript",
            "to-images",
            1,
            OutputMode::PerFile,
            "zip",
            "-pages",
            1,
            "IMAGES",
            "صور",
        ),
        (
            "pdf.rotate",
            "pdf",
            "rotate",
            1,
            OutputMode::PerFile,
            "",
            "-rotated",
            1,
            "ROTATE",
            "تدوير",
        ),
        (
            "pdf.split",
            "pdf",
            "split",
            2,
            OutputMode::PerFile,
            "zip",
            "-split",
            1,
            "SPLIT",
            "تقسيم",
        ),
        (
            "pdf.extract-pages",
            "pdf",
            "extract-pages",
            2,
            OutputMode::PerFile,
            "",
            "-pages",
            1,
            "EXTRACT",
            "استخراج",
        ),
        (
            "pdf.delete-pages",
            "pdf",
            "delete-pages",
            2,
            OutputMode::PerFile,
            "",
            "-deleted",
            1,
            "REMOVE",
            "حذف",
        ),
        (
            "pdf.reorder-pages",
            "pdf",
            "reorder-pages",
            2,
            OutputMode::PerFile,
            "",
            "-ordered",
            1,
            "ORDER",
            "ترتيب",
        ),
        (
            "pdf.metadata",
            "pdf",
            "metadata",
            2,
            OutputMode::PerFile,
            "",
            "-metadata",
            1,
            "META",
            "بيانات",
        ),
        (
            "pdf.to-text",
            "poppler",
            "to-text",
            1,
            OutputMode::PerFile,
            "txt",
            "-text",
            1,
            "TXT",
            "نص",
        ),
        (
            "pdf.linearize",
            "qpdf",
            "linearize",
            2,
            OutputMode::PerFile,
            "",
            "-linearized",
            1,
            "FAST",
            "سريع",
        ),
        (
            "pdf.cleanup",
            "qpdf",
            "cleanup",
            2,
            OutputMode::PerFile,
            "",
            "-cleaned",
            1,
            "CLEAN",
            "تنظيف",
        ),
        (
            "pdf.check",
            "qpdf",
            "check",
            2,
            OutputMode::PerFile,
            "txt",
            "-check",
            1,
            "CHECK",
            "فحص",
        ),
    ];
    assert_eq!(actions.len(), want.len());
    for (id, engine, op, ring, mode, ext, suffix, min_files, en, ar) in want {
        let a = actions
            .iter()
            .find(|a| a.id == id)
            .unwrap_or_else(|| panic!("missing {id}"));
        assert_eq!(a.engine, engine, "{id}");
        assert_eq!(a.op, op, "{id}");
        assert_eq!(a.ring, ring, "{id}");
        assert_eq!(a.output.mode, mode, "{id}");
        assert_eq!(a.output.ext, ext, "{id}");
        assert_eq!(a.output.suffix, suffix, "{id}");
        assert_eq!(a.min_files, min_files, "{id}");
        assert_eq!((a.code.en.as_str(), a.code.ar.as_str()), (en, ar), "{id}");
    }
}
