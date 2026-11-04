//! PDF page operations (S-123): extract-pages, delete-pages, reorder-pages,
//! and the split `every` and `ranges` modes. Fixtures are generated with
//! lopdf: page `n` has a MediaBox of width `100 * n` and 200 high, so a page
//! can be followed by its width alone.

use kawaro_core::engine::TaskOutcome;
use kawaro_core::{CancelToken, Engine, EngineError, Task};
use kawaro_engines::pdf::PdfEngine;
use lopdf::{Document, Object, ObjectId, Stream, dictionary};
use std::io::Read;
use std::path::Path;

/// Writes a `pages` page PDF; page `n` is `100 * n` wide and shows `Pn`.
fn make_pdf(path: &Path, pages: usize) {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let mut kids = Vec::new();
    for number in 1..=pages {
        let content = format!("BT /F1 12 Tf 10 10 Td (P{number}) Tj ET");
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.into_bytes()));
        let width = (number * 100) as i64;
        let page = dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "MediaBox" => vec![0.into(), 0.into(), width.into(), 200.into()],
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
        };
        kids.push(Object::Reference(doc.add_object(page)));
    }
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Count" => pages as i64, "Kids" => kids,
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

/// A page attribute, looked up through the Parent chain like a reader does.
fn effective(doc: &Document, page: ObjectId, key: &[u8]) -> Option<Object> {
    let mut dict = doc.get_dictionary(page).ok()?;
    loop {
        if let Ok(value) = dict.get(key) {
            return Some(value.clone());
        }
        let parent = dict.get(b"Parent").and_then(Object::as_reference).ok()?;
        dict = doc.get_dictionary(parent).ok()?;
    }
}

/// The MediaBox width of every page, in page order.
fn widths(doc: &Document) -> Vec<i64> {
    doc.get_pages()
        .values()
        .map(|id| {
            let obj = effective(doc, *id, b"MediaBox").expect("page has no MediaBox");
            obj.as_array().unwrap()[2].as_i64().unwrap()
        })
        .collect()
}

/// The content stream of every page, in page order.
fn texts(doc: &Document) -> Vec<String> {
    doc.get_pages()
        .values()
        .map(|id| String::from_utf8_lossy(&doc.get_page_content(*id)).into_owned())
        .collect()
}

fn marker(document: &Document) -> Vec<String> {
    texts(document)
        .iter()
        .map(|text| {
            let start = text.find("(P").expect("page has no marker") + 1;
            let end = text[start..].find(')').unwrap() + start;
            text[start..end].to_string()
        })
        .collect()
}

/// The PDFs of a split ZIP, entry name and document, in archive order.
fn zip_pdfs(path: &Path) -> Vec<(String, Document)> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    let mut out = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).unwrap();
        let name = entry.name().unwrap().to_string();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        out.push((name, Document::load_mem(&bytes).unwrap()));
    }
    out
}

fn expect_bad_param(result: Result<TaskOutcome, EngineError>, name: &str) {
    match result {
        Err(EngineError::BadParam { name: found, .. }) => assert_eq!(found, name),
        other => panic!("expected BadParam `{name}`, got {other:?}"),
    }
}

#[test]
fn pdf_extract_pages_keeps_the_selection_in_document_order() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.pdf");
    make_pdf(&input, 5);

    // A comma list, in document order even when written out of order.
    let out = dir.path().join("a-pages.pdf");
    run(&task("extract-pages", &input, &out, "pages = \"4,2\"")).unwrap();
    let doc = load(&out);
    assert_eq!(widths(&doc), [200, 400]);
    assert_eq!(marker(&doc), ["P2", "P4"]);

    // A range.
    let out = dir.path().join("a-range.pdf");
    run(&task("extract-pages", &input, &out, "pages = \"1-3\"")).unwrap();
    let doc = load(&out);
    assert_eq!(widths(&doc), [100, 200, 300]);
    assert_eq!(marker(&doc), ["P1", "P2", "P3"]);

    // A single page, and the default: page 1.
    let out = dir.path().join("a-one.pdf");
    run(&task("extract-pages", &input, &out, "pages = \"3\"")).unwrap();
    assert_eq!(widths(&load(&out)), [300]);
    let out = dir.path().join("a-default.pdf");
    run(&task("extract-pages", &input, &out, "")).unwrap();
    assert_eq!(widths(&load(&out)), [100]);

    // The input is untouched.
    assert_eq!(widths(&load(&input)), [100, 200, 300, 400, 500]);
}

#[test]
fn pdf_delete_pages_leaves_the_other_pages() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.pdf");
    make_pdf(&input, 5);

    let out = dir.path().join("a-deleted.pdf");
    run(&task("delete-pages", &input, &out, "pages = \"2,4\"")).unwrap();
    let doc = load(&out);
    assert_eq!(widths(&doc), [100, 300, 500]);
    assert_eq!(marker(&doc), ["P1", "P3", "P5"]);

    // A range, and the last page.
    let out = dir.path().join("a-range.pdf");
    run(&task("delete-pages", &input, &out, "pages = \"1-3\"")).unwrap();
    assert_eq!(widths(&load(&out)), [400, 500]);
    let out = dir.path().join("a-last.pdf");
    run(&task("delete-pages", &input, &out, "pages = \"5\"")).unwrap();
    assert_eq!(widths(&load(&out)), [100, 200, 300, 400]);

    // Deleting every page is refused.
    let out = dir.path().join("a-empty.pdf");
    expect_bad_param(
        run(&task("delete-pages", &input, &out, "pages = \"1-5\"")),
        "pages",
    );
    assert!(!out.exists());
}

#[test]
fn pdf_reorder_pages_follows_the_order_param() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.pdf");
    make_pdf(&input, 5);

    // A permutation in a mixed order.
    let out = dir.path().join("a-ordered.pdf");
    run(&task(
        "reorder-pages",
        &input,
        &out,
        "order = \"3,1,5,2,4\"",
    ))
    .unwrap();
    let doc = load(&out);
    assert_eq!(widths(&doc), [300, 100, 500, 200, 400]);
    assert_eq!(marker(&doc), ["P3", "P1", "P5", "P2", "P4"]);

    // Every page once, in reverse.
    let out = dir.path().join("a-reverse.pdf");
    run(&task(
        "reorder-pages",
        &input,
        &out,
        "order = \"5,4,3,2,1\"",
    ))
    .unwrap();
    assert_eq!(widths(&load(&out)), [500, 400, 300, 200, 100]);

    // A partial list, duplicates, pages outside the document and empty
    // input are refused: reorder must name every page exactly once.
    for bad in [
        "order = \"5,1,2\"",
        "order = \"4,2\"",
        "order = \"1,1,2,3,4\"",
        "order = \"6,1,2,3,4\"",
        "order = \"3-1\"",
        "order = \"\"",
    ] {
        let out = dir.path().join("a-bad.pdf");
        expect_bad_param(run(&task("reorder-pages", &input, &out, bad)), "order");
        assert!(!out.exists());
    }
}

#[test]
fn pdf_split_every_two_writes_three_parts() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.pdf");
    make_pdf(&input, 5);
    let out = dir.path().join("a-split.zip");
    run(&task("split", &input, &out, "every = 2")).unwrap();

    let parts = zip_pdfs(&out);
    let names: Vec<&str> = parts.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["page-001.pdf", "page-002.pdf", "page-003.pdf"]);
    let counts: Vec<usize> = parts.iter().map(|(_, doc)| doc.get_pages().len()).collect();
    assert_eq!(counts, [2, 2, 1]);
    assert_eq!(widths(&parts[0].1), [100, 200]);
    assert_eq!(widths(&parts[1].1), [300, 400]);
    assert_eq!(widths(&parts[2].1), [500]);
}

#[test]
fn pdf_split_ranges_writes_one_part_per_range() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.pdf");
    make_pdf(&input, 5);
    let out = dir.path().join("a-split.zip");
    run(&task("split", &input, &out, "ranges = \"1-2,3-5\"")).unwrap();

    let parts = zip_pdfs(&out);
    assert_eq!(parts.len(), 2);
    assert_eq!(widths(&parts[0].1), [100, 200]);
    assert_eq!(widths(&parts[1].1), [300, 400, 500]);
    assert_eq!(marker(&parts[1].1), ["P3", "P4", "P5"]);

    // Single page ranges work too.
    let out = dir.path().join("a-parts.zip");
    run(&task("split", &input, &out, "ranges = \"1,3-4\"")).unwrap();
    let parts = zip_pdfs(&out);
    assert_eq!(parts.len(), 2);
    assert_eq!(widths(&parts[0].1), [100]);
    assert_eq!(widths(&parts[1].1), [300, 400]);

    // An empty ranges string falls back to every, here the default 1.
    let out = dir.path().join("a-pages.zip");
    run(&task("split", &input, &out, "ranges = \"\"")).unwrap();
    assert_eq!(zip_pdfs(&out).len(), 5);
}

#[test]
fn pdf_split_default_is_one_pdf_per_page() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.pdf");
    make_pdf(&input, 5);
    let out = dir.path().join("a-split.zip");
    run(&task("split", &input, &out, "")).unwrap();
    let parts = zip_pdfs(&out);
    let names: Vec<&str> = parts.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        [
            "page-001.pdf",
            "page-002.pdf",
            "page-003.pdf",
            "page-004.pdf",
            "page-005.pdf",
        ]
    );
    for (index, (_, doc)) in parts.iter().enumerate() {
        assert_eq!(doc.get_pages().len(), 1);
        assert_eq!(widths(doc), [(index + 1) as i64 * 100]);
    }
}

#[test]
fn pdf_page_selection_errors_name_the_param() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.pdf");
    make_pdf(&input, 5);
    let out = dir.path().join("out");
    for bad in [
        "pages = \"\"",
        "pages = \"0\"",
        "pages = \"6\"",
        "pages = \"3-1\"",
        "pages = \"1,1\"",
        "pages = \"1,,2\"",
    ] {
        expect_bad_param(run(&task("extract-pages", &input, &out, bad)), "pages");
        expect_bad_param(run(&task("delete-pages", &input, &out, bad)), "pages");
    }
    for bad in ["order = \"1,1\"", "order = \"6,1\"", "order = \"\""] {
        expect_bad_param(run(&task("reorder-pages", &input, &out, bad)), "order");
    }
    expect_bad_param(
        run(&task("split", &input, &out, "ranges = \"1-9\"")),
        "ranges",
    );
    expect_bad_param(
        run(&task("split", &input, &out, "ranges = \"1-2,2-3\"")),
        "ranges",
    );
    expect_bad_param(run(&task("split", &input, &out, "every = 0")), "every");
    expect_bad_param(run(&task("split", &input, &out, "every = 2.5")), "every");
    assert!(!out.exists(), "a refused call wrote a file");
}
