//! S-128: PDF compress dpi, and the qpdf child engine.
//!
//! Contract (tester): Ghostscript compress takes an optional dpi target and
//! keeps its preset behaviour at 0; qpdf is a child engine with the three
//! structural ops; a missing program is a clear Unavailable, and the three
//! new actions parse. Live qpdf runs skip with a notice when it is absent.

use kawaro_core::engine::TaskOutcome;
use kawaro_core::{ActionDef, CancelToken, Engine, EngineError, MediaKind, OutputMode, Task};
#[cfg(unix)]
use kawaro_engines::locate::{self, Os, Program};
use lopdf::{Document, Object, Stream, dictionary};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tempfile::tempdir;

/// The engine program lookup and the tests that exercise it share the live
/// PATH, so the tests of this file run one at a time.
static PATH_LOCK: Mutex<()> = Mutex::new(());

fn lock() -> std::sync::MutexGuard<'static, ()> {
    PATH_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name)
}

fn task(op: &str, input: &Path, output: PathBuf, params: &str) -> Task {
    Task {
        action_id: format!("test-{op}"),
        op: op.to_string(),
        params: toml::from_str(params).unwrap(),
        inputs: vec![input.to_path_buf()],
        output,
    }
}

fn run(engine: &dyn Engine, task: &Task) -> Result<TaskOutcome, EngineError> {
    let progress = |_fraction: f32| {};
    engine.run(task, &progress, &CancelToken::new())
}

fn engine(id: &str) -> Option<std::sync::Arc<dyn Engine>> {
    kawaro_engines::all()
        .into_iter()
        .find(|engine| engine.id() == id)
}

fn is_pdf(path: &Path) -> bool {
    lopdf::Document::load(path).is_ok()
}

#[derive(serde::Deserialize)]
struct File {
    action: Vec<ActionDef>,
}

fn pdf_actions() -> Vec<ActionDef> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../actions/pdf.toml");
    let text = std::fs::read_to_string(path).unwrap();
    toml::from_str::<File>(&text).unwrap().action
}

/// One 200 x 200 pt page with an uncompressed 600 x 600 RGB image, so a
/// Ghostscript rewrite always has something to shrink (S-133 skips when the
/// rewrite is not smaller, which the tiny one-page fixture would do).
fn compressible_pdf(path: &Path) {
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

#[test]
fn ghostscript_compress_takes_a_dpi_target_and_keeps_the_preset_at_zero() {
    let _guard = lock();
    let Some(engine) = engine("ghostscript") else {
        panic!("the ghostscript engine is registered");
    };
    if !engine.available() {
        eprintln!("skipped: Ghostscript is not installed");
        return;
    }
    let dir = tempdir().unwrap();
    let input = dir.path().join("photo.pdf");
    compressible_pdf(&input);
    for (index, params) in [
        "preset = \"ebook\"\ndpi = 300",
        "preset = \"ebook\"\ndpi = 0",
        "preset = \"ebook\"",
        "preset = \"prepress\"\ndpi = 150",
    ]
    .into_iter()
    .enumerate()
    {
        let output = dir.path().join(format!("out-{index}.pdf"));
        let task = task("compress", &input, output.clone(), params);
        let result = run(engine.as_ref(), &task);
        assert!(result.is_ok(), "{params}: {result:?}");
        assert!(is_pdf(&output), "{params}: the output is not a valid PDF");
        let document = lopdf::Document::load(&output).unwrap();
        assert_eq!(document.get_pages().len(), 1, "{params}: page count");
    }
}

#[test]
fn qpdf_engine_is_registered_with_the_three_structural_ops() {
    let _guard = lock();
    let engine = engine("qpdf").expect("the qpdf engine is registered");
    assert_eq!(engine.id(), "qpdf");
    let caps = engine.capabilities();
    let ops: Vec<&str> = caps.iter().map(|(op, _)| *op).collect();
    for op in ["linearize", "cleanup", "check"] {
        assert!(ops.contains(&op), "{op} missing from {ops:?}");
    }
    for (op, kinds) in &caps {
        assert_eq!(kinds, &vec![MediaKind::Pdf], "kinds of {op}");
    }
}

#[test]
fn a_missing_qpdf_is_unavailable_and_names_qpdf() {
    let _guard = lock();
    let Some(engine) = engine("qpdf") else {
        panic!("the qpdf engine is registered");
    };
    if engine.available() {
        eprintln!("skipped: qpdf is installed, the unavailable path cannot be exercised here");
        return;
    }
    let dir = tempdir().unwrap();
    let output = dir.path().join("out.pdf");
    let task = task(
        "linearize",
        &fixture("docs/one-page.pdf"),
        output.clone(),
        "",
    );
    let error = run(engine.as_ref(), &task).expect_err("an unavailable qpdf must refuse");
    assert!(matches!(error, EngineError::Unavailable(_)), "{error:?}");
    assert!(
        error.to_string().to_lowercase().contains("qpdf"),
        "the message names qpdf: {error}"
    );
    assert!(!output.exists(), "an unavailable engine writes no output");
}

#[cfg(unix)]
#[test]
fn locate_finds_qpdf_only_on_the_given_path_dirs() {
    use std::ffi::OsString;
    use std::os::unix::fs::PermissionsExt;

    struct NoRegistry;
    impl locate::Registry for NoRegistry {
        fn value(&self, _key: &str, _name: &str) -> Option<String> {
            None
        }
        fn subkeys(&self, _key: &str) -> Vec<String> {
            Vec::new()
        }
    }

    let _guard = lock();
    let dir = tempdir().unwrap();
    let found = dir.path().join("qpdf");
    std::fs::write(&found, "#!/bin/sh\n").unwrap();
    std::fs::set_permissions(&found, std::fs::Permissions::from_mode(0o755)).unwrap();
    let env = |_key: &str| -> Option<OsString> { None };
    let registry = NoRegistry;
    assert_eq!(
        locate::search(
            Os::Unix,
            Program::Qpdf,
            &[dir.path().to_path_buf()],
            &env,
            None,
            &registry,
        ),
        Some(found)
    );
    let empty = tempdir().unwrap();
    assert_eq!(
        locate::search(
            Os::Unix,
            Program::Qpdf,
            &[empty.path().to_path_buf()],
            &env,
            None,
            &registry,
        ),
        None
    );
}

#[test]
fn the_three_qpdf_actions_parse_with_their_outputs() {
    let actions = pdf_actions();
    let rows = [
        ("pdf.linearize", "linearize", "", "-linearized"),
        ("pdf.cleanup", "cleanup", "", "-cleaned"),
        ("pdf.check", "check", "txt", "-check"),
    ];
    for (id, op, ext, suffix) in rows {
        let action = actions
            .iter()
            .find(|action| action.id == id)
            .unwrap_or_else(|| panic!("missing {id}"));
        assert_eq!(action.engine, "qpdf", "{id}");
        assert_eq!(action.op, op, "{id}");
        assert_eq!(action.ring, 2, "{id}");
        assert_eq!(action.accepts, [MediaKind::Pdf], "{id}");
        assert_eq!(action.output.mode, OutputMode::PerFile, "{id}");
        assert_eq!(action.output.ext, ext, "{id}");
        assert_eq!(action.output.suffix, suffix, "{id}");
        assert_eq!(action.output.dir, "", "{id}");
        assert!(!action.code.en.is_empty(), "{id} code en");
        assert!(!action.code.ar.is_empty(), "{id} code ar");
        assert!(!action.label.en.is_empty(), "{id} label en");
        assert!(!action.label.ar.is_empty(), "{id} label ar");
        if let Some(options) = &action.options {
            options
                .validate()
                .unwrap_or_else(|error| panic!("{id}: {error}"));
        }
    }
    let qpdf_actions = actions
        .iter()
        .filter(|action| action.engine == "qpdf")
        .count();
    assert_eq!(qpdf_actions, 3, "three qpdf actions");
}

#[test]
fn live_qpdf_runs_skip_with_a_notice_when_qpdf_is_absent() {
    let _guard = lock();
    let Some(engine) = engine("qpdf") else {
        panic!("the qpdf engine is registered");
    };
    if !engine.available() {
        eprintln!("skipped: qpdf is not installed");
        return;
    }
    let dir = tempdir().unwrap();
    let input = fixture("docs/one-page.pdf");
    let output = dir.path().join("linearized.pdf");
    let task = task("linearize", &input, output.clone(), "");
    let result = run(engine.as_ref(), &task);
    assert!(result.is_ok(), "{result:?}");
    assert!(is_pdf(&output), "the linearized output loads");
    let bytes = std::fs::read(&output).unwrap();
    let head = &bytes[..bytes.len().min(1024)];
    assert!(
        head.windows(b"/Linearized".len())
            .any(|window| window == b"/Linearized"),
        "the first kilobyte carries the linearized marker"
    );
}

#[test]
fn qpdf_flag_unit_check_is_skipped_when_the_engine_keeps_its_argv_private() {
    // The engine exposes with_program and the three ops, but not the argv
    // builders: rewrite_args and check_args are private, and the exact flags
    // (--linearize, the cleanup flags, --check --show-encryption) are pinned
    // by the crate unit test s128_args_build_the_three_commands inside
    // crates/engines/src/qpdf.rs. If the engine ever exposes the argv, this
    // test should assert it here instead of skipping.
    eprintln!(
        "skipped: QpdfEngine keeps its argv private, so the flag unit check is unavailable here"
    );
}
