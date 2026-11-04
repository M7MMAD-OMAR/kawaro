//! PDF to text through poppler `pdftotext` (S-126).
//!
//! The action and engine declarations are checked unconditionally. The tests
//! that run the real tool print a notice and pass when pdftotext is not
//! installed, because the missing tool path has its own test with a fake
//! program path.

use kawaro_core::action::{ActionDef, OutputMode};
use kawaro_core::engine::TaskOutcome;
use kawaro_core::options::{Kind, OptionsKind};
use kawaro_core::{CancelToken, Engine, EngineError, MediaKind, Task};
use kawaro_engines::poppler::PopplerEngine;
use lopdf::{Document, Object, Stream, dictionary};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Writes a two page PDF whose pages show ALPHA and BETA.
fn make_pdf(path: &Path) {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let mut kids = Vec::new();
    for text in ["ALPHA page", "BETA page"] {
        let content = format!("BT /F1 18 Tf 72 700 Td ({text}) Tj ET");
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.into_bytes()));
        let page = dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
        };
        kids.push(Object::Reference(doc.add_object(page)));
    }
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Count" => 2, "Kids" => kids,
        }),
    );
    let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog_id);
    doc.save(path).unwrap();
}

fn task(input: &Path, output: &Path, params: &str) -> Task {
    Task {
        action_id: "pdf.to-text".into(),
        op: "to-text".into(),
        params: toml::from_str(params).unwrap(),
        inputs: vec![input.to_path_buf()],
        output: output.to_path_buf(),
    }
}

fn run(engine: &PopplerEngine, task: &Task) -> Result<TaskOutcome, EngineError> {
    engine.run(task, &|_| {}, &CancelToken::new())
}

fn engine() -> PopplerEngine {
    PopplerEngine::default()
}

/// The repository fixture: one page with no extractable text.
fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/docs/one-page.pdf")
        .canonicalize()
        .expect("the one-page PDF fixture exists")
}

fn expect_bad_param(result: Result<TaskOutcome, EngineError>, name: &str) {
    match result {
        Err(EngineError::BadParam { name: found, .. }) => assert_eq!(found, name),
        other => panic!("expected BadParam {name}, got {other:?}"),
    }
}

/// Parses actions/pdf.toml into action definitions.
fn actions() -> Vec<ActionDef> {
    #[derive(serde::Deserialize)]
    struct File {
        action: Vec<ActionDef>,
    }
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../actions/pdf.toml");
    let text = std::fs::read_to_string(path).unwrap();
    toml::from_str::<File>(&text).unwrap().action
}

#[test]
fn s126_action_matches_the_story() {
    let actions = actions();
    let action = actions
        .iter()
        .find(|action| action.id == "pdf.to-text")
        .expect("pdf.to-text is in actions/pdf.toml");
    assert_eq!(action.engine, "poppler");
    assert_eq!(action.op, "to-text");
    assert_eq!(action.ring, 1);
    assert_eq!(action.order, 50);
    assert_eq!(action.accepts, [MediaKind::Pdf]);
    assert_eq!(action.min_files, 1);
    assert_eq!(action.output.mode, OutputMode::PerFile);
    assert_eq!(action.output.ext, "txt");
    assert_eq!(action.output.suffix, "-text");
    assert_eq!(action.code.en, "TXT");
    assert!(!action.code.ar.is_empty());
    assert!(!action.label.ar.is_empty() && !action.label.en.is_empty());
    assert_eq!(action.icon, "text");

    let options = action.options.as_ref().expect("the action carries options");
    assert_eq!(options.kind, OptionsKind::Form);
    assert!(options.validate().is_ok(), "{:?}", options.validate());
    assert_eq!(options.fields.len(), 1);
    let pages = &options.fields[0];
    assert_eq!(pages.id, "pages");
    assert_eq!(pages.kind, Kind::String);
    assert_eq!(
        pages.default.as_ref().and_then(toml::Value::as_str),
        Some("")
    );
    assert_eq!(pages.param_key(), "pages");
    assert!(!pages.label.ar.is_empty() && !pages.label.en.is_empty());
}

#[test]
fn s126_engine_declares_capabilities_and_unknown_ops() {
    let engine = engine();
    assert_eq!(engine.id(), "poppler");
    let caps = engine.capabilities();
    assert_eq!(caps.len(), 1, "{caps:?}");
    assert_eq!(caps[0].0, "to-text");
    assert_eq!(caps[0].1, [MediaKind::Pdf]);

    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.pdf");
    make_pdf(&input);
    let bogus = Task {
        op: "bogus".into(),
        ..task(&input, &dir.path().join("y.txt"), "")
    };
    let r = run(&engine, &bogus);
    assert!(matches!(r, Err(EngineError::UnsupportedOp(_))), "{r:?}");
}

#[test]
fn s126_missing_tool_is_unavailable_naming_the_program() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.pdf");
    make_pdf(&input);
    let engine = PopplerEngine::with_program("/nonexistent/kawaro-no-pdftotext");
    assert!(!engine.available());
    let out = dir.path().join("a.txt");
    match run(&engine, &task(&input, &out, "")) {
        Err(EngineError::Unavailable(message)) => {
            assert!(message.contains("pdftotext"), "{message}");
            assert!(
                message.contains("https://poppler.freedesktop.org/"),
                "{message}"
            );
            assert!(message.contains("kawaro-no-pdftotext"), "{message}");
        }
        other => panic!("expected Unavailable, got {other:?}"),
    }
    assert!(!out.exists(), "a missing tool wrote a file");
}

#[test]
fn s126_bad_ranges_leave_no_output() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.pdf");
    make_pdf(&input);
    let out = dir.path().join("a.txt");
    for bad in ["0", "3-1", "1,3", "1-", "-2", "a", "1-2-3", "1-2,3"] {
        let params = format!("pages = \"{bad}\"");
        expect_bad_param(run(&engine(), &task(&input, &out, &params)), "pages");
        assert!(!out.exists(), "{bad} wrote a file");
    }
    // An out of range range is refused through the page count.
    expect_bad_param(
        run(&engine(), &task(&input, &out, "pages = \"1-3\"")),
        "pages",
    );
    assert!(!out.exists());
}

#[test]
fn s126_pre_cancelled_token_stops_before_an_output() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.pdf");
    make_pdf(&input);
    let out = dir.path().join("a.txt");
    let cancel = CancelToken::new();
    cancel.cancel();
    let r = engine().run(&task(&input, &out, ""), &|_| {}, &cancel);
    assert!(matches!(r, Err(EngineError::Cancelled)), "{r:?}");
    assert!(!out.exists(), "a cancelled run wrote a file");
}

#[test]
fn s126_writes_utf8_without_bom_or_carriage_return() {
    let engine = engine();
    if !engine.available() {
        eprintln!("skipping the pdftotext run tests: pdftotext is not installed");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.pdf");
    make_pdf(&input);
    let before = std::fs::read(&input).unwrap();
    let out = dir.path().join("a.txt");
    let seen = Mutex::new(Vec::new());
    let progress = |fraction: f32| seen.lock().unwrap().push(fraction);
    engine
        .run(&task(&input, &out, ""), &progress, &CancelToken::new())
        .unwrap();

    let bytes = std::fs::read(&out).unwrap();
    assert!(!bytes.is_empty(), "the output is empty");
    assert!(
        !bytes.starts_with(&[0xEF, 0xBB, 0xBF]),
        "the output has a BOM"
    );
    assert!(!bytes.contains(&b'\r'), "the output has a carriage return");
    let text = String::from_utf8(bytes).expect("the output is UTF-8");
    assert!(text.contains("ALPHA"), "{text:?}");
    assert!(text.contains("BETA"), "{text:?}");
    let seen = seen.into_inner().unwrap();
    assert_eq!(seen.first(), Some(&0.0), "{seen:?}");
    assert_eq!(seen.last(), Some(&1.0), "{seen:?}");
    assert!(seen.windows(2).all(|w| w[0] <= w[1]), "{seen:?}");
    assert_eq!(std::fs::read(&input).unwrap(), before, "the input changed");

    // The repository fixture has no text layer; the output is still a valid
    // UTF-8 file without a BOM and without carriage returns.
    let fixture = fixture();
    let fixture_out = dir.path().join("fixture.txt");
    run(&engine, &task(&fixture, &fixture_out, "")).unwrap();
    let bytes = std::fs::read(&fixture_out).unwrap();
    assert!(
        !bytes.starts_with(&[0xEF, 0xBB, 0xBF]),
        "the fixture output has a BOM"
    );
    assert!(
        !bytes.contains(&b'\r'),
        "the fixture output has a carriage return"
    );
    let text = String::from_utf8(bytes).expect("the fixture output is UTF-8");
    assert!(!text.is_empty(), "the fixture output is empty");
    assert!(
        text.trim().is_empty(),
        "the fixture has no text to extract: {text:?}"
    );
}

#[test]
fn s126_page_ranges_select_the_requested_pages() {
    let engine = engine();
    if !engine.available() {
        eprintln!("skipping the pdftotext range tests: pdftotext is not installed");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.pdf");
    make_pdf(&input);

    let out = dir.path().join("all.txt");
    run(&engine, &task(&input, &out, "")).unwrap();
    let all = std::fs::read_to_string(&out).unwrap();
    assert!(all.contains("ALPHA") && all.contains("BETA"), "{all:?}");

    let out = dir.path().join("one.txt");
    run(&engine, &task(&input, &out, "pages = \"1\"")).unwrap();
    let one = std::fs::read_to_string(&out).unwrap();
    assert!(one.contains("ALPHA"), "{one:?}");
    assert!(!one.contains("BETA"), "{one:?}");

    let out = dir.path().join("two.txt");
    run(&engine, &task(&input, &out, "pages = \"2\"")).unwrap();
    let two = std::fs::read_to_string(&out).unwrap();
    assert!(two.contains("BETA"), "{two:?}");
    assert!(!two.contains("ALPHA"), "{two:?}");

    let out = dir.path().join("both.txt");
    run(&engine, &task(&input, &out, "pages = \"1-2\"")).unwrap();
    let both = std::fs::read_to_string(&out).unwrap();
    assert!(both.contains("ALPHA") && both.contains("BETA"), "{both:?}");

    // One page in the fixture, so a two page range is refused.
    let out = dir.path().join("fixture.txt");
    expect_bad_param(
        run(&engine, &task(&fixture(), &out, "pages = \"1-2\"")),
        "pages",
    );
    assert!(!out.exists(), "an out of range run wrote a file");
}
