//! Ghostscript engine tests (S-010): compress and to-images. Real runs need
//! `gs`; without it those tests print a notice and return early.

use lopdf::{Document, Object, Stream, dictionary};
#[cfg_attr(not(unix), allow(unused_imports))]
use sbarhive_core::{CancelToken, Engine, EngineError, MediaKind, Task};
use sbarhive_engines::pdf::GhostscriptEngine;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// `count` pages of `w` x `h` points, each with a small filled square.
/// Vector shapes only: an unembedded font makes Ghostscript search the
/// system fonts, which took 15 to 40 s per run on a busy machine.
fn text_pdf(path: &Path, count: usize, w: i64, h: i64) {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let mut kids = Vec::new();
    for i in 1..=count {
        let content = format!("0 0 0 rg {i} {i} 20 20 re f");
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.into_bytes()));
        kids.push(Object::Reference(doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "MediaBox" => vec![0.into(), 0.into(), w.into(), h.into()],
        })));
    }
    doc.objects.insert(
        pages_id,
        Object::Dictionary(
            dictionary! { "Type" => "Pages", "Count" => count as i64, "Kids" => kids },
        ),
    );
    let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    doc.trailer.set("Root", catalog_id);
    doc.save(path).unwrap();
}

/// One 200 x 200 pt page showing an uncompressed 600 x 600 RGB gradient.
fn image_pdf(path: &Path) {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let side = 600usize;
    let mut pixels = Vec::with_capacity(side * side * 3);
    for y in 0..side {
        for x in 0..side {
            pixels.extend_from_slice(&[(x * 255 / side) as u8, (y * 255 / side) as u8, 128]);
        }
    }
    let image_id = doc.add_object(Stream::new(
        dictionary! {
            "Type" => "XObject", "Subtype" => "Image",
            "Width" => side as i64, "Height" => side as i64,
            "ColorSpace" => "DeviceRGB", "BitsPerComponent" => 8,
        },
        pixels,
    ));
    let content_id = doc.add_object(Stream::new(
        dictionary! {},
        b"q 200 0 0 200 0 0 cm /Im1 Do Q".to_vec(),
    ));
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages_id, "Contents" => content_id,
        "MediaBox" => vec![0.into(), 0.into(), 200.into(), 200.into()],
        "Resources" => dictionary! { "XObject" => dictionary! { "Im1" => image_id } },
    });
    doc.objects.insert(
        pages_id,
        Object::Dictionary(
            dictionary! { "Type" => "Pages", "Count" => 1, "Kids" => vec![page_id.into()] },
        ),
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

fn engine(root: &Path) -> Option<GhostscriptEngine> {
    let engine = GhostscriptEngine::default().temp_root(root);
    if engine.available() {
        Some(engine)
    } else {
        eprintln!("Ghostscript not installed, skipping");
        None
    }
}

fn run(
    engine: &GhostscriptEngine,
    task: &Task,
    cancel: &CancelToken,
) -> (Result<(), EngineError>, Vec<f32>) {
    let seen = Mutex::new(Vec::new());
    let progress = |f: f32| seen.lock().unwrap().push(f);
    let result = engine.run(task, &progress, cancel);
    (result, seen.into_inner().unwrap())
}

fn pages(path: &Path) -> usize {
    let bytes = std::fs::read(path).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
    Document::load_mem(&bytes).unwrap().get_pages().len()
}

fn len(path: &Path) -> u64 {
    std::fs::metadata(path).unwrap().len()
}

fn work_root(dir: &Path) -> PathBuf {
    let root = dir.join("work");
    std::fs::create_dir(&root).unwrap();
    root
}

fn is_empty_dir(dir: &Path) -> bool {
    std::fs::read_dir(dir).unwrap().next().is_none()
}

#[test]
fn pdf_compress_shrinks_an_image_heavy_pdf() {
    let dir = tempfile::tempdir().unwrap();
    let root = work_root(dir.path());
    let Some(engine) = engine(&root) else { return };
    let input = dir.path().join("photo.pdf");
    image_pdf(&input);
    for preset in ["", "preset = \"screen\"", "preset = \"printer\""] {
        let out = dir.path().join("photo-compressed.pdf");
        let (result, progress) = run(
            &engine,
            &task("compress", &input, &out, preset),
            &CancelToken::new(),
        );
        result.unwrap();
        assert_eq!(progress.first(), Some(&0.0));
        assert_eq!(progress.last(), Some(&1.0));
        assert_eq!(pages(&out), 1);
        assert!(
            len(&out) < len(&input) / 2,
            "{preset}: {} vs {}",
            len(&out),
            len(&input)
        );
    }
    assert!(is_empty_dir(&root));
}

#[test]
fn pdf_compress_never_returns_a_larger_file() {
    let dir = tempfile::tempdir().unwrap();
    let root = work_root(dir.path());
    let Some(engine) = engine(&root) else { return };
    // A tiny text PDF: Ghostscript's rewrite adds metadata and comes out larger.
    let input = dir.path().join("tiny.pdf");
    text_pdf(&input, 1, 100, 100);
    let out = dir.path().join("tiny-compressed.pdf");
    run(
        &engine,
        &task("compress", &input, &out, ""),
        &CancelToken::new(),
    )
    .0
    .unwrap();
    assert!(len(&out) <= len(&input));
    assert_eq!(pages(&out), 1);
    assert_eq!(
        std::fs::read(&out).unwrap(),
        std::fs::read(&input).unwrap(),
        "original kept"
    );
    assert!(is_empty_dir(&root));
}

fn png_size(bytes: &[u8]) -> (u32, u32) {
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    let w = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let h = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    (w, h)
}

fn zip_pngs(path: &Path) -> Vec<(String, (u32, u32))> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    (0..archive.len())
        .map(|i| {
            let mut entry = archive.by_index(i).unwrap();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            (entry.name().unwrap().to_string(), png_size(&bytes))
        })
        .collect()
}

#[test]
fn pdf_to_images_zip_holds_one_png_per_page_sized_by_dpi() {
    let dir = tempfile::tempdir().unwrap();
    let root = work_root(dir.path());
    let Some(engine) = engine(&root) else { return };
    let input = dir.path().join("doc.pdf");
    text_pdf(&input, 3, 72, 144);

    let out = dir.path().join("doc-pages.zip");
    let (result, progress) = run(
        &engine,
        &task("to-images", &input, &out, ""),
        &CancelToken::new(),
    );
    result.unwrap();
    assert_eq!(progress.first(), Some(&0.0));
    assert_eq!(progress.last(), Some(&1.0));
    assert!(progress.windows(2).all(|w| w[0] <= w[1]), "{progress:?}");
    assert!(
        progress.len() >= 4,
        "expected per page progress, got {progress:?}"
    );
    assert_eq!(
        zip_pngs(&out),
        [
            ("page-001.png".to_string(), (150, 300)),
            ("page-002.png".to_string(), (150, 300)),
            ("page-003.png".to_string(), (150, 300)),
        ]
    );

    let out = dir.path().join("doc-72.zip");
    run(
        &engine,
        &task("to-images", &input, &out, "dpi = 72"),
        &CancelToken::new(),
    )
    .0
    .unwrap();
    assert!(zip_pngs(&out).iter().all(|(_, size)| *size == (72, 144)));
    assert!(is_empty_dir(&root));
}

// Unix only: uses /bin/false as a fake program.

#[cfg(unix)]
#[test]
fn pdf_ghostscript_rejects_bad_params_and_unknown_op() {
    let dir = tempfile::tempdir().unwrap();
    let engine = GhostscriptEngine::with_program("/bin/false").temp_root(dir.path());
    let input = dir.path().join("a.pdf");
    text_pdf(&input, 1, 72, 72);
    let cases = [
        ("compress", "preset = \"tiny\""),
        ("compress", "preset = 3"),
        ("to-images", "dpi = 0"),
        ("to-images", "dpi = 20000"),
        ("to-images", "timeout_s = 0"),
    ];
    for (op, params) in cases {
        let r = run(
            &engine,
            &task(op, &input, &dir.path().join("x.out"), params),
            &CancelToken::new(),
        )
        .0;
        assert!(
            matches!(r, Err(EngineError::BadParam { .. })),
            "{op} {params}: {r:?}"
        );
    }
    let r = run(
        &engine,
        &task("explode", &input, &dir.path().join("x.pdf"), ""),
        &CancelToken::new(),
    )
    .0;
    assert!(matches!(r, Err(EngineError::UnsupportedOp(_))), "{r:?}");
    let r = run(
        &engine,
        &task("compress", &input, &dir.path().join("x.pdf"), ""),
        &CancelToken::new(),
    )
    .0;
    assert!(
        matches!(r, Err(EngineError::Failed(_))),
        "a failing gs must fail the task: {r:?}"
    );
    let missing = dir.path().join("missing.pdf");
    let r = run(
        &engine,
        &task("compress", &missing, &dir.path().join("x.pdf"), ""),
        &CancelToken::new(),
    )
    .0;
    assert!(matches!(r, Err(EngineError::Input { .. })), "{r:?}");
    let r = run(
        &GhostscriptEngine::with_program("/nonexistent/gs").temp_root(dir.path()),
        &task("compress", &input, &dir.path().join("x.pdf"), ""),
        &CancelToken::new(),
    )
    .0;
    assert!(matches!(r, Err(EngineError::Unavailable(_))), "{r:?}");
    let not_exec = dir.path().join("gs");
    std::fs::write(&not_exec, "not a program").unwrap();
    let r = run(
        &GhostscriptEngine::with_program(&not_exec).temp_root(dir.path()),
        &task("compress", &input, &dir.path().join("x.pdf"), ""),
        &CancelToken::new(),
    )
    .0;
    assert!(matches!(r, Err(EngineError::Io(_))), "{r:?}");
}

/// Any live process whose command line mentions `needle`.
fn processes_mentioning(needle: &str) -> Vec<String> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir("/proc").unwrap().flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.chars().all(|c| c.is_ascii_digit()) || name == std::process::id().to_string() {
            continue;
        }
        let Ok(cmdline) = std::fs::read(entry.path().join("cmdline")) else {
            continue;
        };
        let stat = std::fs::read_to_string(entry.path().join("stat")).unwrap_or_default();
        let zombie = stat
            .rsplit(')')
            .next()
            .unwrap_or("")
            .trim_start()
            .starts_with('Z');
        let cmdline = String::from_utf8_lossy(&cmdline).replace('\0', " ");
        if cmdline.contains(needle) && !zombie {
            found.push(format!("{name}: {cmdline}"));
        }
    }
    found
}

#[test]
fn pdf_ghostscript_cancel_kills_gs_and_cleans_up() {
    let dir = tempfile::tempdir().unwrap();
    let root = work_root(dir.path());
    let Some(engine) = engine(&root) else { return };
    let input = dir.path().join("big.pdf");
    text_pdf(&input, 60, 595, 842);
    let cancel = CancelToken::new();
    let trigger = cancel.clone();
    let canceller = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        trigger.cancel();
    });
    let started = Instant::now();
    let r = run(
        &engine,
        &task(
            "to-images",
            &input,
            &dir.path().join("big.zip"),
            "dpi = 600",
        ),
        &cancel,
    )
    .0;
    canceller.join().unwrap();
    assert!(matches!(r, Err(EngineError::Cancelled)), "{r:?}");
    assert!(started.elapsed() < Duration::from_secs(3));
    let left = processes_mentioning(&root.to_string_lossy());
    assert!(left.is_empty(), "gs survived: {left:?}");
    assert!(is_empty_dir(&root));

    let cancelled = CancelToken::new();
    cancelled.cancel();
    let r = run(
        &engine,
        &task("compress", &input, &dir.path().join("c.pdf"), ""),
        &cancelled,
    )
    .0;
    assert!(matches!(r, Err(EngineError::Cancelled)), "{r:?}");
}

#[cfg(unix)]
#[test]
fn pdf_ghostscript_identity_and_capabilities() {
    let engine = GhostscriptEngine::default();
    assert_eq!(engine.id(), "ghostscript");
    let caps = engine.capabilities();
    let ops: Vec<_> = caps.iter().map(|(op, _)| *op).collect();
    assert_eq!(ops, ["compress", "to-images"]);
    assert!(caps.iter().all(|(_, kinds)| kinds == &[MediaKind::Pdf]));
    assert!(!GhostscriptEngine::with_program("/nonexistent/gs").available());
    assert!(GhostscriptEngine::with_program("/bin/sh").available());
}
