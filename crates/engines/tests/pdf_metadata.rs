//! PDF metadata (S-125, AC1 to AC9): setting the document information
//! fields, clearing them with the XMP stream, the text string encoding and
//! the action contract. Fixtures are generated with lopdf.

use kawaro_core::action::{ActionDef, OutputMode};
use kawaro_core::engine::TaskOutcome;
use kawaro_core::options::{Kind, OptionsKind};
use kawaro_core::{CancelToken, Engine, EngineError, MediaKind, Registry, Task};
use kawaro_engines::pdf::PdfEngine;
use lopdf::{Document, Object, Stream, StringFormat, dictionary};
use std::path::{Path, PathBuf};

/// Writes a two page PDF with Title, Author, Subject, Keywords, Creator and
/// Producer in its Info dictionary, and an XMP metadata stream in its
/// catalog. Page n shows Pn and is 200 by 200.
fn make_pdf(path: &Path) {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let mut kids = Vec::new();
    for number in 1..=2 {
        let content = format!("BT /F1 12 Tf 10 10 Td (P{number}) Tj ET");
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.into_bytes()));
        let page = dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "MediaBox" => vec![0.into(), 0.into(), 200.into(), 200.into()],
        };
        kids.push(Object::Reference(doc.add_object(page)));
    }
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Count" => 2, "Kids" => kids,
        }),
    );
    let info_id = doc.add_object(dictionary! {
        "Title" => Object::string_literal("Old title"),
        "Author" => Object::string_literal("Old Author"),
        "Subject" => Object::string_literal("Old subject"),
        "Keywords" => Object::string_literal("Old keywords"),
        "Creator" => Object::string_literal("Old creator"),
        "Producer" => Object::string_literal("Old producer"),
    });
    let xmp = doc.add_object(Stream::new(
        dictionary! { "Type" => "Metadata", "Subtype" => "XML" },
        b"<x:xmpmeta></x:xmpmeta>".to_vec(),
    ));
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog", "Pages" => pages_id, "Metadata" => xmp,
    });
    doc.trailer.set("Root", catalog_id);
    doc.trailer.set("Info", info_id);
    doc.save(path).unwrap();
}

/// Writes a one page PDF with no Info dictionary and no XMP stream.
fn make_plain_pdf(path: &Path) {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let content = b"BT /F1 12 Tf 10 10 Td (P1) Tj ET".to_vec();
    let content_id = doc.add_object(Stream::new(dictionary! {}, content));
    let page = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content_id,
        "MediaBox" => vec![0.into(), 0.into(), 200.into(), 200.into()],
    });
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Count" => 1, "Kids" => vec![Object::Reference(page)],
        }),
    );
    let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog_id);
    doc.save(path).unwrap();
}

fn task(op: &str, input: &Path, output: &Path, params: &str) -> Task {
    Task {
        action_id: format!("test.{op}"),
        op: op.into(),
        params: toml::from_str(params).unwrap(),
        inputs: vec![input.to_path_buf()],
        output: output.to_path_buf(),
    }
}

fn run(task: &Task) -> Result<TaskOutcome, EngineError> {
    PdfEngine.run(task, &|_| {}, &CancelToken::new())
}

fn load(path: &Path) -> Document {
    let bytes = std::fs::read(path).unwrap();
    assert!(bytes.starts_with(b"%PDF-"), "{path:?} is not a PDF");
    Document::load_mem(&bytes).unwrap()
}

/// One object of the output Info dictionary, if it is present.
fn info_object(doc: &Document, key: &str) -> Option<Object> {
    let id = doc
        .trailer
        .get(b"Info")
        .and_then(Object::as_reference)
        .ok()?;
    let dict = doc.get_dictionary(id).ok()?;
    dict.get(key.as_bytes()).ok().cloned()
}

/// The decoded text of one Info field, if it is present.
fn info_text(doc: &Document, key: &str) -> Option<String> {
    let object = info_object(doc, key)?;
    let bytes = object.as_str().ok()?;
    Some(decode_text(bytes))
}

/// A PDF text string: literal ASCII, or UTF-16BE with a BOM in hexadecimal.
fn decode_text(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let units: Vec<u16> = bytes[2..]
            .chunks(2)
            .filter(|pair| pair.len() == 2)
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .collect();
        String::from_utf16(&units).unwrap()
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

/// The hexadecimal body lopdf writes for a UTF-16BE text string.
fn utf16be_hex(value: &str) -> String {
    let mut hex = String::from("FEFF");
    for unit in value.encode_utf16() {
        hex.push_str(&format!("{unit:04X}"));
    }
    hex
}

fn has_xmp(doc: &Document) -> bool {
    let referenced = doc
        .catalog()
        .ok()
        .and_then(|catalog| catalog.get(b"Metadata").ok())
        .is_some();
    let stream = doc
        .objects
        .values()
        .any(|object| object.type_name().is_ok_and(|name| name == b"Metadata"));
    referenced || stream
}

fn page_texts(doc: &Document) -> Vec<String> {
    doc.get_pages()
        .values()
        .map(|id| String::from_utf8_lossy(&doc.get_page_content(*id)).into_owned())
        .collect()
}

fn expect_bad_param(result: Result<TaskOutcome, EngineError>, name: &str) {
    match result {
        Err(EngineError::BadParam {
            name: found,
            reason,
        }) => {
            assert_eq!(found, name);
            assert!(!reason.is_empty(), "{name} has an empty reason");
        }
        other => panic!("expected BadParam `{name}`, got {other:?}"),
    }
}

#[test]
fn ac1_metadata_is_dispatched_with_capabilities_and_progress() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.pdf");
    make_pdf(&input);
    let out = dir.path().join("a-metadata.pdf");
    let seen = std::sync::Mutex::new(Vec::new());
    let progress = |fraction: f32| seen.lock().unwrap().push(fraction);
    PdfEngine
        .run(
            &task("metadata", &input, &out, "title = \"New\""),
            &progress,
            &CancelToken::new(),
        )
        .unwrap();
    let seen = seen.into_inner().unwrap();
    assert_eq!(seen.first(), Some(&0.0), "{seen:?}");
    assert_eq!(seen.last(), Some(&1.0), "{seen:?}");
    assert!(seen.windows(2).all(|w| w[0] <= w[1]), "{seen:?}");

    // The op is declared for PDF input and an unknown op is still refused.
    let caps = PdfEngine.capabilities();
    assert!(
        caps.iter().any(|(op, kinds)| {
            *op == "metadata" && kinds.len() == 1 && kinds[0] == MediaKind::Pdf
        }),
        "{caps:?}"
    );
    let r = run(&task("bogus", &input, &dir.path().join("x.pdf"), ""));
    assert!(matches!(r, Err(EngineError::UnsupportedOp(_))), "{r:?}");

    // A pre-cancelled token stops before an output is written.
    let cancel = CancelToken::new();
    cancel.cancel();
    let out = dir.path().join("cancelled.pdf");
    let r = PdfEngine.run(
        &task("metadata", &input, &out, "title = \"New\""),
        &|_| {},
        &cancel,
    );
    assert!(matches!(r, Err(EngineError::Cancelled)), "{r:?}");
    assert!(!out.exists(), "a cancelled run wrote a file");
}

#[test]
fn ac2_sets_all_four_fields_and_keeps_the_other_info_keys() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.pdf");
    make_pdf(&input);
    let before = load(&input);
    let out = dir.path().join("a-metadata.pdf");
    run(&task(
        "metadata",
        &input,
        &out,
        "title = \"Clean title\"\nauthor = \"Clean Author\"\nsubject = \"Clean subject\"\nkeywords = \"one, two\"",
    ))
    .unwrap();
    let doc = load(&out);
    assert_eq!(info_text(&doc, "Title").as_deref(), Some("Clean title"));
    assert_eq!(info_text(&doc, "Author").as_deref(), Some("Clean Author"));
    assert_eq!(info_text(&doc, "Subject").as_deref(), Some("Clean subject"));
    assert_eq!(info_text(&doc, "Keywords").as_deref(), Some("one, two"));
    // Every other Info key keeps its value.
    assert_eq!(info_text(&doc, "Creator").as_deref(), Some("Old creator"));
    assert_eq!(info_text(&doc, "Producer").as_deref(), Some("Old producer"));
    // Pages, page text and MediaBox are unchanged.
    assert_eq!(doc.get_pages().len(), 2);
    assert_eq!(page_texts(&doc), page_texts(&before));
    assert_eq!(doc.get_pages(), before.get_pages());
}

#[test]
fn ac2_creates_the_info_dictionary_when_the_input_has_none() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("plain.pdf");
    make_plain_pdf(&input);
    let out = dir.path().join("plain-metadata.pdf");
    run(&task("metadata", &input, &out, "title = \"Fresh\"")).unwrap();
    let doc = load(&out);
    assert_eq!(info_text(&doc, "Title").as_deref(), Some("Fresh"));
    assert_eq!(doc.get_pages().len(), 1);
}

#[test]
fn ac3_absent_or_empty_fields_leave_the_input_values_alone() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.pdf");
    make_pdf(&input);

    // One field set, the rest kept.
    let out = dir.path().join("a-one.pdf");
    run(&task("metadata", &input, &out, "title = \"Only title\"")).unwrap();
    let doc = load(&out);
    assert_eq!(info_text(&doc, "Title").as_deref(), Some("Only title"));
    assert_eq!(info_text(&doc, "Author").as_deref(), Some("Old Author"));
    assert_eq!(info_text(&doc, "Subject").as_deref(), Some("Old subject"));
    assert_eq!(info_text(&doc, "Keywords").as_deref(), Some("Old keywords"));

    // An empty string does not delete an existing value.
    let out = dir.path().join("a-empty.pdf");
    run(&task("metadata", &input, &out, "title = \"\"")).unwrap();
    assert_eq!(
        info_text(&load(&out), "Title").as_deref(),
        Some("Old title")
    );

    // Nothing set: a rewrite with the same metadata and the same pages.
    let out = dir.path().join("a-copy.pdf");
    run(&task("metadata", &input, &out, "")).unwrap();
    let doc = load(&out);
    assert_eq!(info_text(&doc, "Title").as_deref(), Some("Old title"));
    assert_eq!(info_text(&doc, "Author").as_deref(), Some("Old Author"));
    assert_eq!(doc.get_pages().len(), 2);
    assert!(has_xmp(&doc), "an untouched XMP stream is kept");
}

#[test]
fn ac4_clear_removes_the_info_entry_and_the_xmp_stream() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.pdf");
    make_pdf(&input);
    let before = std::fs::read(&input).unwrap();
    let out = dir.path().join("a-metadata.pdf");
    run(&task("metadata", &input, &out, "clear = true")).unwrap();

    let doc = load(&out);
    assert!(
        doc.trailer.get(b"Info").is_err(),
        "the output still has a trailer Info entry"
    );
    assert!(
        doc.catalog().unwrap().get(b"Metadata").is_err(),
        "the catalog still points at XMP"
    );
    assert!(!has_xmp(&doc), "an XMP object survived the clear");
    let raw = String::from_utf8_lossy(&std::fs::read(&out).unwrap()).into_owned();
    assert!(!raw.contains("x:xmpmeta"), "the XMP bytes survived");
    // No metadata key is left in Info, because Info itself is gone.
    for key in [
        "Title", "Author", "Subject", "Keywords", "Creator", "Producer",
    ] {
        assert_eq!(info_text(&doc, key), None, "{key} was not cleared");
    }
    assert_eq!(doc.get_pages().len(), 2);
    assert_eq!(std::fs::read(&input).unwrap(), before, "the input changed");
}

#[test]
fn ac5_clear_with_a_text_field_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.pdf");
    make_pdf(&input);
    let out = dir.path().join("a-metadata.pdf");
    let r = run(&task(
        "metadata",
        &input,
        &out,
        "clear = true\ntitle = \"Both\"",
    ));
    match r {
        Err(EngineError::BadParam { name, reason }) => {
            assert_eq!(name, "clear");
            assert!(reason.contains("title"), "{reason}");
            assert!(reason.contains("clear"), "{reason}");
        }
        other => panic!("expected BadParam clear, got {other:?}"),
    }
    assert!(!out.exists(), "a refused call wrote a file");

    // clear = false with a text field is the normal set path.
    run(&task(
        "metadata",
        &input,
        &out,
        "clear = false\ntitle = \"Set\"",
    ))
    .unwrap();
    assert_eq!(info_text(&load(&out), "Title").as_deref(), Some("Set"));
}

#[test]
fn ac6_ascii_is_literal_and_non_ascii_is_utf16be_hex() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.pdf");
    make_pdf(&input);

    // ASCII stays a literal string.
    let out = dir.path().join("a-ascii.pdf");
    run(&task("metadata", &input, &out, "title = \"Clean title\"")).unwrap();
    let doc = load(&out);
    match info_object(&doc, "Title") {
        Some(Object::String(bytes, StringFormat::Literal)) => {
            assert_eq!(String::from_utf8_lossy(&bytes), "Clean title");
        }
        other => panic!("expected a literal string, got {other:?}"),
    }
    assert_eq!(info_text(&doc, "Title").as_deref(), Some("Clean title"));
    let raw = String::from_utf8_lossy(&std::fs::read(&out).unwrap()).into_owned();
    assert!(
        raw.contains("(Clean title)"),
        "the literal is not in the file"
    );

    // A non-ASCII value is a hexadecimal UTF-16BE string with a BOM.
    let arabic = "عنوان عربي";
    let accented = "José Álvarez";
    let out = dir.path().join("a-utf16.pdf");
    run(&task(
        "metadata",
        &input,
        &out,
        "title = \"عنوان عربي\"\nauthor = \"José Álvarez\"",
    ))
    .unwrap();
    let doc = load(&out);
    for (key, value) in [("Title", arabic), ("Author", accented)] {
        match info_object(&doc, key) {
            Some(Object::String(bytes, StringFormat::Hexadecimal)) => {
                assert!(bytes.starts_with(&[0xFE, 0xFF]), "{key} has no BOM");
            }
            other => panic!("expected a hexadecimal string for {key}, got {other:?}"),
        }
        assert_eq!(info_text(&doc, key).as_deref(), Some(value), "{key}");
    }
    let raw = String::from_utf8_lossy(&std::fs::read(&out).unwrap()).into_owned();
    for value in [arabic, accented] {
        let hex = utf16be_hex(value);
        assert!(
            raw.contains(&hex),
            "the hex form of {value} is not in the file"
        );
    }
}

#[test]
fn ac7_bad_values_and_missing_inputs_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.pdf");
    make_pdf(&input);
    let out = dir.path().join("a-metadata.pdf");
    expect_bad_param(
        run(&task("metadata", &input, &out, "clear = \"yes\"")),
        "clear",
    );
    expect_bad_param(run(&task("metadata", &input, &out, "clear = 1")), "clear");
    expect_bad_param(run(&task("metadata", &input, &out, "title = 5")), "title");
    expect_bad_param(
        run(&task("metadata", &input, &out, "author = true")),
        "author",
    );
    expect_bad_param(run(&task("metadata", &input, &out, "nope = \"x\"")), "nope");
    assert!(!out.exists(), "a refused call wrote a file");

    // A missing file and a file that is not a PDF take the Input path.
    let missing = dir.path().join("absent.pdf");
    let r = run(&task("metadata", &missing, &dir.path().join("x.pdf"), ""));
    assert!(matches!(r, Err(EngineError::Input { .. })), "{r:?}");
    let junk = dir.path().join("junk.pdf");
    std::fs::write(&junk, b"not a pdf").unwrap();
    let r = run(&task("metadata", &junk, &dir.path().join("y.pdf"), ""));
    assert!(matches!(r, Err(EngineError::Input { .. })), "{r:?}");
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
fn ac8_the_action_matches_the_story() {
    let actions = actions();
    let action = actions
        .iter()
        .find(|action| action.id == "pdf.metadata")
        .expect("pdf.metadata is in actions/pdf.toml");
    assert_eq!(action.engine, "pdf");
    assert_eq!(action.op, "metadata");
    assert_eq!(action.ring, 2);
    assert_eq!(action.order, 50);
    assert_eq!(action.accepts, [MediaKind::Pdf]);
    assert_eq!(action.min_files, 1);
    assert_eq!(action.output.mode, OutputMode::PerFile);
    assert_eq!(action.output.ext, "", "the extension stays PDF");
    assert_eq!(action.output.suffix, "-metadata");
    assert_eq!(
        (action.code.ar.as_str(), action.code.en.as_str()),
        ("بيانات", "META")
    );
    assert!(!action.label.ar.is_empty() && !action.label.en.is_empty());
    assert_eq!(action.icon, "file");

    // Params: the switch and the four empty text fields.
    assert_eq!(
        action.params.get("clear").and_then(toml::Value::as_bool),
        Some(false)
    );
    for name in ["title", "author", "subject", "keywords"] {
        assert_eq!(
            action.params.get(name).and_then(toml::Value::as_str),
            Some(""),
            "{name}"
        );
    }

    let options = action.options.as_ref().expect("the action carries options");
    assert_eq!(options.kind, OptionsKind::Form);
    assert!(options.validate().is_ok(), "{:?}", options.validate());
    let ids: Vec<&str> = options
        .fields
        .iter()
        .map(|field| field.id.as_str())
        .collect();
    assert_eq!(ids, ["clear", "title", "author", "subject", "keywords"]);

    let clear = &options.fields[0];
    assert_eq!(clear.kind, Kind::Bool);
    assert_eq!(
        clear.default.as_ref().and_then(toml::Value::as_bool),
        Some(false)
    );
    assert_eq!(clear.param_key(), "clear");
    assert_eq!(clear.clears, ["title", "author", "subject", "keywords"]);

    for name in ["title", "author", "subject", "keywords"] {
        let field = options
            .fields
            .iter()
            .find(|field| field.id == name)
            .unwrap();
        assert_eq!(field.kind, Kind::String, "{name}");
        assert_eq!(
            field.default.as_ref().and_then(toml::Value::as_str),
            Some(""),
            "{name}"
        );
        assert_eq!(field.param_key(), name, "{name}");
        assert_eq!(field.clears, ["clear"], "{name}");
        assert!(
            !field.label.ar.is_empty() && !field.label.en.is_empty(),
            "{name}"
        );
    }
    for field in &options.fields {
        assert!(
            !field.label.ar.is_empty() && !field.label.en.is_empty(),
            "{}",
            field.id
        );
    }
}

#[test]
fn ac9_the_registry_has_56_actions_and_17_with_options() {
    let dir = tempfile::tempdir().unwrap();
    let pdf = dir.path().join("a.pdf");
    std::fs::write(&pdf, b"%PDF-1.5\n").unwrap();
    let registry = Registry::builtin();
    // 56 since S-128 added pdf.linearize, pdf.cleanup and pdf.check.
    assert_eq!(registry.all().len(), 56, "built-in action count");
    let with_options: Vec<&str> = registry
        .all()
        .iter()
        .filter(|action| action.options.is_some())
        .map(|action| action.id.as_str())
        .collect();
    assert_eq!(with_options.len(), 17, "{with_options:?}");
    assert!(registry.get("pdf.metadata").is_some());

    let paths: Vec<PathBuf> = vec![pdf];
    let refs: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
    let matched: Vec<&str> = registry
        .matching(&refs, &|engine| engine == "pdf")
        .iter()
        .map(|action| action.id.as_str())
        .collect();
    assert!(matched.contains(&"pdf.metadata"), "{matched:?}");
}
