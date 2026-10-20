//! Pure Rust PDF engine tests (S-010): merge, rotate, split. Fixtures are
//! generated with lopdf inside each test.

use lopdf::{Document, Object, ObjectId, Stream, dictionary};
use sbarhive_core::{CancelToken, Engine, EngineError, MediaKind, Task};
use sbarhive_engines::pdf::PdfEngine;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Page tree layout of a generated fixture.
#[derive(Clone, Copy, Default)]
struct Layout {
    /// MediaBox written on each page.
    page_box: Option<(i64, i64)>,
    /// MediaBox written only on the root Pages node, inherited by pages.
    root_box: Option<(i64, i64)>,
    /// Rotate written only on the root Pages node.
    root_rotate: Option<i64>,
}

/// Writes a PDF whose page `i` shows the text `texts[i]`. Fonts live in
/// Resources on the root Pages node, so pages inherit them.
fn make_pdf(path: &Path, texts: &[&str], layout: Layout) {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let mut kids = Vec::new();
    for text in texts {
        let content = format!("BT /F1 12 Tf 10 10 Td ({text}) Tj ET");
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.into_bytes()));
        let mut page =
            dictionary! { "Type" => "Page", "Parent" => pages_id, "Contents" => content_id };
        if let Some((w, h)) = layout.page_box {
            page.set("MediaBox", vec![0.into(), 0.into(), w.into(), h.into()]);
        }
        kids.push(Object::Reference(doc.add_object(page)));
    }
    let mut pages = dictionary! {
        "Type" => "Pages",
        "Count" => texts.len() as i64,
        "Kids" => kids,
        "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
    };
    if let Some((w, h)) = layout.root_box {
        pages.set("MediaBox", vec![0.into(), 0.into(), w.into(), h.into()]);
    }
    if let Some(r) = layout.root_rotate {
        pages.set("Rotate", r);
    }
    doc.objects.insert(pages_id, Object::Dictionary(pages));
    let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog_id);
    doc.save(path).unwrap();
}

fn task(op: &str, inputs: &[PathBuf], output: &Path, params: &str) -> Task {
    Task {
        action_id: format!("test.{op}"),
        op: op.into(),
        params: toml::from_str(params).unwrap(),
        inputs: inputs.to_vec(),
        output: output.to_path_buf(),
    }
}

fn run(task: &Task, cancel: &CancelToken) -> (Result<(), EngineError>, Vec<f32>) {
    let seen = Mutex::new(Vec::new());
    let progress = |f: f32| seen.lock().unwrap().push(f);
    let result = PdfEngine.run(task, &progress, cancel);
    (result, seen.into_inner().unwrap())
}

fn load(path: &Path) -> Document {
    let bytes = std::fs::read(path).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
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

fn page_texts(doc: &Document) -> Vec<String> {
    doc.get_pages()
        .values()
        .map(|id| String::from_utf8_lossy(&doc.get_page_content(*id)).into_owned())
        .collect()
}

fn media_box(doc: &Document, page: ObjectId) -> Vec<i64> {
    let obj = effective(doc, page, b"MediaBox").expect("page has no MediaBox");
    obj.as_array()
        .unwrap()
        .iter()
        .map(|o| o.as_i64().unwrap())
        .collect()
}

fn rotations(doc: &Document) -> Vec<i64> {
    doc.get_pages()
        .values()
        .map(|id| effective(doc, *id, b"Rotate").map_or(0, |o| o.as_i64().unwrap()))
        .collect()
}

#[test]
fn pdf_merge_two_and_three_pages_gives_five_in_selection_order() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.pdf");
    let b = dir.path().join("b.pdf");
    make_pdf(
        &a,
        &["A1", "A2"],
        Layout {
            page_box: Some((200, 100)),
            ..Default::default()
        },
    );
    make_pdf(
        &b,
        &["B1", "B2", "B3"],
        Layout {
            root_box: Some((300, 400)),
            ..Default::default()
        },
    );
    let out = dir.path().join("a-merged.pdf");
    let (result, progress) = run(
        &task("merge", &[b.clone(), a.clone()], &out, ""),
        &CancelToken::new(),
    );
    result.unwrap();
    assert_eq!(progress.last(), Some(&1.0));

    let doc = load(&out);
    let texts = page_texts(&doc);
    assert_eq!(texts.len(), 5);
    for (text, want) in texts.iter().zip(["B1", "B2", "B3", "A1", "A2"]) {
        assert!(
            text.contains(&format!("({want})")),
            "{text} should show {want}"
        );
    }
    let ids: Vec<_> = doc.get_pages().values().copied().collect();
    // Attributes inherited from each source page tree survive the merge.
    for id in &ids[..3] {
        assert_eq!(media_box(&doc, *id), [0, 0, 300, 400]);
    }
    for id in &ids[3..] {
        assert_eq!(media_box(&doc, *id), [0, 0, 200, 100]);
    }
    for id in &ids {
        let res = effective(&doc, *id, b"Resources").expect("page lost its Resources");
        let res = match res {
            Object::Reference(r) => doc.get_dictionary(r).unwrap().clone(),
            Object::Dictionary(d) => d,
            other => panic!("bad Resources {other:?}"),
        };
        assert!(res.get(b"Font").is_ok());
    }
}

#[test]
fn pdf_merge_same_file_twice_and_rejects_bad_input() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.pdf");
    make_pdf(
        &a,
        &["A1", "A2"],
        Layout {
            page_box: Some((200, 100)),
            ..Default::default()
        },
    );
    let out = dir.path().join("twice.pdf");
    run(
        &task("merge", &[a.clone(), a.clone()], &out, ""),
        &CancelToken::new(),
    )
    .0
    .unwrap();
    assert_eq!(load(&out).get_pages().len(), 4);

    let junk = dir.path().join("junk.pdf");
    std::fs::write(&junk, b"not a pdf at all").unwrap();
    let r = run(
        &task(
            "merge",
            &[a.clone(), junk.clone()],
            &dir.path().join("x.pdf"),
            "",
        ),
        &CancelToken::new(),
    )
    .0;
    match r {
        Err(EngineError::Input { path, .. }) => assert_eq!(path, junk),
        other => panic!("expected an input error, got {other:?}"),
    }
    let r = run(
        &task("merge", &[], &dir.path().join("y.pdf"), ""),
        &CancelToken::new(),
    )
    .0;
    assert!(matches!(r, Err(EngineError::BadParam { .. })), "{r:?}");
}

#[test]
fn pdf_rotate_sets_rotate_90_on_every_page() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.pdf");
    make_pdf(
        &a,
        &["P1", "P2", "P3"],
        Layout {
            page_box: Some((200, 100)),
            ..Default::default()
        },
    );
    let out = dir.path().join("a-rotated.pdf");
    let (result, progress) = run(
        &task("rotate", std::slice::from_ref(&a), &out, ""),
        &CancelToken::new(),
    );
    result.unwrap();
    assert_eq!(progress.last(), Some(&1.0));
    let doc = load(&out);
    assert_eq!(rotations(&doc), [90, 90, 90]);
    // Every page carries its own /Rotate.
    for id in doc.get_pages().values() {
        assert_eq!(
            doc.get_dictionary(*id)
                .unwrap()
                .get(b"Rotate")
                .unwrap()
                .as_i64()
                .unwrap(),
            90
        );
    }
    assert_eq!(page_texts(&doc).len(), 3);
}

#[test]
fn pdf_rotate_adds_to_inherited_rotation_and_honours_degrees() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.pdf");
    make_pdf(
        &a,
        &["P1", "P2"],
        Layout {
            root_box: Some((200, 100)),
            root_rotate: Some(270),
            ..Default::default()
        },
    );
    let out = dir.path().join("r90.pdf");
    run(
        &task("rotate", std::slice::from_ref(&a), &out, ""),
        &CancelToken::new(),
    )
    .0
    .unwrap();
    assert_eq!(rotations(&load(&out)), [0, 0]);

    let out = dir.path().join("r180.pdf");
    run(
        &task("rotate", std::slice::from_ref(&a), &out, "degrees = 180"),
        &CancelToken::new(),
    )
    .0
    .unwrap();
    assert_eq!(rotations(&load(&out)), [90, 90]);

    for bad in ["degrees = 45", "degrees = 0", "degrees = \"ninety\""] {
        let r = run(
            &task(
                "rotate",
                std::slice::from_ref(&a),
                &dir.path().join("bad.pdf"),
                bad,
            ),
            &CancelToken::new(),
        )
        .0;
        assert!(
            matches!(r, Err(EngineError::BadParam { .. })),
            "{bad}: {r:?}"
        );
    }
}

#[test]
fn pdf_split_zip_holds_one_single_page_pdf_per_page() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.pdf");
    make_pdf(
        &a,
        &["S1", "S2", "S3"],
        Layout {
            root_box: Some((300, 400)),
            ..Default::default()
        },
    );
    let out = dir.path().join("a-split.zip");
    let (result, progress) = run(
        &task("split", std::slice::from_ref(&a), &out, ""),
        &CancelToken::new(),
    );
    result.unwrap();
    assert_eq!(progress.first(), Some(&0.0));
    assert_eq!(progress.last(), Some(&1.0));
    assert!(
        progress.windows(2).all(|w| w[0] <= w[1]),
        "progress went backwards: {progress:?}"
    );
    assert!(
        progress.len() >= 4,
        "expected per page progress, got {progress:?}"
    );

    let mut archive = zip::ZipArchive::new(std::fs::File::open(&out).unwrap()).unwrap();
    let names: Vec<_> = (0..archive.len())
        .map(|i| archive.by_index(i).unwrap().name().unwrap().to_string())
        .collect();
    assert_eq!(names, ["page-001.pdf", "page-002.pdf", "page-003.pdf"]);
    for (i, want) in ["S1", "S2", "S3"].iter().enumerate() {
        let mut bytes = Vec::new();
        archive
            .by_index(i)
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        let doc = Document::load_mem(&bytes).unwrap();
        let texts = page_texts(&doc);
        assert_eq!(texts.len(), 1);
        assert!(texts[0].contains(&format!("({want})")));
        let id = *doc.get_pages().values().next().unwrap();
        assert_eq!(media_box(&doc, id), [0, 0, 300, 400]);
        // Other pages are not carried along.
        let text_streams = doc
            .objects
            .values()
            .filter_map(|o| o.as_stream().ok())
            .filter(|s| s.content.windows(2).any(|w| w == b"(S"))
            .count();
        assert_eq!(text_streams, 1, "page {} carries other pages", i + 1);
    }
}

#[test]
fn pdf_engine_cancel_unknown_op_and_capabilities() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.pdf");
    make_pdf(
        &a,
        &["P1"],
        Layout {
            page_box: Some((100, 100)),
            ..Default::default()
        },
    );
    let cancel = CancelToken::new();
    cancel.cancel();
    for op in ["merge", "rotate", "split"] {
        let out = dir.path().join(format!("{op}.out"));
        let r = run(&task(op, &[a.clone(), a.clone()], &out, ""), &cancel).0;
        assert!(matches!(r, Err(EngineError::Cancelled)), "{op}: {r:?}");
    }
    let r = run(
        &task(
            "explode",
            std::slice::from_ref(&a),
            &dir.path().join("x.pdf"),
            "",
        ),
        &CancelToken::new(),
    )
    .0;
    assert!(matches!(r, Err(EngineError::UnsupportedOp(_))), "{r:?}");

    assert_eq!(PdfEngine.id(), "pdf");
    assert!(PdfEngine.available());
    let caps = PdfEngine.capabilities();
    let ops: Vec<_> = caps.iter().map(|(op, _)| *op).collect();
    assert_eq!(ops, ["merge", "rotate", "split"]);
    assert!(caps.iter().all(|(_, kinds)| kinds == &[MediaKind::Pdf]));
}

fn assert_progress(got: &[f32], want: &[f32]) {
    assert_eq!(got.len(), want.len(), "progress {got:?}, want {want:?}");
    for (g, w) in got.iter().zip(want) {
        assert!((g - w).abs() < 1e-5, "progress {got:?}, want {want:?}");
    }
}

fn count_type(doc: &Document, name: &[u8]) -> usize {
    doc.objects
        .values()
        .filter(|o| o.type_name().is_ok_and(|t| t == name))
        .count()
}

/// Like `make_pdf`, but the catalog and page tree take the lowest ids, a
/// page is the highest id, and the file has a classic xref table. So an
/// input's highest id belongs to a live object, and an id collision in the
/// merged output loses a page.
fn make_pdf_pages_last(path: &Path, texts: &[&str]) {
    let mut doc = Document::with_version("1.5");
    doc.reference_table.cross_reference_type = lopdf::xref::XrefType::CrossReferenceTable;
    let pages_id = doc.new_object_id();
    let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
    });
    let mut kids = Vec::new();
    for text in texts {
        let content = format!("BT /F1 12 Tf 10 10 Td ({text}) Tj ET");
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.into_bytes()));
        let page = dictionary! {
            "Type" => "Page", "Parent" => pages_id, "Contents" => content_id,
            "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
        };
        kids.push(Object::Reference(doc.add_object(page)));
    }
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Count" => texts.len() as i64, "Kids" => kids,
        }),
    );
    doc.trailer.set("Root", catalog_id);
    doc.save(path).unwrap();
}

#[test]
fn pdf_merge_numbers_objects_tightly_and_drops_the_old_trees() {
    let dir = tempfile::tempdir().unwrap();
    let mut inputs = Vec::new();
    for (name, texts) in [
        ("a.pdf", &["A1", "A2"][..]),
        ("b.pdf", &["B1"][..]),
        ("c.pdf", &["C1", "C2", "C3"][..]),
    ] {
        let path = dir.path().join(name);
        make_pdf_pages_last(&path, texts);
        let input = Document::load(&path).unwrap();
        assert_eq!(count_type(&input, b"XRef"), 0, "fixture has an xref stream");
        let last = *input.objects.keys().next_back().unwrap();
        assert_eq!(input.objects[&last].type_name().unwrap(), b"Page");
        inputs.push(path);
    }
    let out = dir.path().join("merged.pdf");
    let (result, progress) = run(&task("merge", &inputs, &out, ""), &CancelToken::new());
    result.unwrap();
    assert_progress(&progress, &[0.0, 0.3, 0.6, 0.9, 1.0]);

    let doc = load(&out);
    let texts = page_texts(&doc);
    assert_eq!(texts.len(), 6);
    for (text, want) in texts.iter().zip(["A1", "A2", "B1", "C1", "C2", "C3"]) {
        assert!(
            text.contains(&format!("({want})")),
            "{text} should show {want}"
        );
    }
    // The output's own cross-reference stream comes back as an object too.
    let (xref_ids, ids): (Vec<_>, Vec<_>) = doc
        .objects
        .iter()
        .map(|(id, o)| (id.0, o.type_name().is_ok_and(|t| t == b"XRef")))
        .partition(|(_, is_xref)| *is_xref);
    // Per input: one font plus a page and a content stream per page. Then
    // the new catalog and page tree. A collision would lose objects.
    assert_eq!(ids.len(), 5 + 3 + 7 + 2);
    // Only the new catalog and page tree, not the inputs' own.
    assert_eq!(count_type(&doc, b"Catalog"), 1);
    assert_eq!(count_type(&doc, b"Pages"), 1);
    // The xref stream takes the next free id: no ids are left reserved for
    // the dropped catalogs and page trees.
    let highest = ids.iter().map(|(id, _)| *id).max().unwrap();
    assert_eq!(xref_ids.len(), 1);
    assert_eq!(xref_ids[0].0, highest + 1);
}

#[test]
fn pdf_split_reports_each_page_and_stores_entries_uncompressed() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a.pdf");
    make_pdf(&a, &["S1", "S2", "S3"], Layout::default());
    let out = dir.path().join("a-split.zip");
    let (result, progress) = run(
        &task("split", std::slice::from_ref(&a), &out, ""),
        &CancelToken::new(),
    );
    result.unwrap();
    assert_progress(&progress, &[0.0, 0.33, 0.66, 0.99, 1.0]);

    let mut archive = zip::ZipArchive::new(std::fs::File::open(&out).unwrap()).unwrap();
    assert_eq!(archive.len(), 3);
    for i in 0..archive.len() {
        assert_eq!(
            archive.by_index(i).unwrap().compression(),
            zip::CompressionMethod::Stored
        );
    }
}
