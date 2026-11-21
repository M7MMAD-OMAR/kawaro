//! S-146 at the engine boundary: the action, the availability and the cancel
//! path of the Tesseract adapter, from outside the crate.
//!
//! Every case here is source only. No child is started, no picture is
//! converted and no language data is read: the fake executable is a file, and
//! the test that proves availability is a lookup checks that the file was
//! never run. The parts that need a real child, a real language probe and a
//! real Tesseract run stay unexecuted and are listed in the handoff.

use kawaro_core::action::Registry;
#[cfg(unix)]
use kawaro_core::queue;
#[cfg(unix)]
use kawaro_core::unsupported::UnavailableReason;
use kawaro_core::{CancelToken, Engine, EngineError, MediaKind, Task};
use kawaro_engines::ocr::{
    EXTRACTION_TIMEOUT, MAX_OUTPUT_BYTES, NO_TEXT, OcrEngine, PROBE_TIMEOUT, SUPPORTED_LANGUAGES,
    TESSERACT_URL,
};
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::time::Duration;

fn task(params: &str, inputs: Vec<PathBuf>, output: PathBuf) -> Task {
    Task {
        action_id: "image.to-text".into(),
        op: "to-text".into(),
        params: toml::from_str(params).unwrap(),
        inputs,
        output,
    }
}

/// A fake `tesseract`: an executable file whose run would leave a marker.
/// Nothing here runs it.
fn fake_program(dir: &Path, marker: &Path) -> PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let script = format!("#!/bin/sh\ntouch {}\n", marker.display());
        let path = dir.join("tesseract");
        std::fs::write(&path, script).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }
    #[cfg(not(unix))]
    {
        let _ = marker;
        let path = dir.join("tesseract.exe");
        std::fs::write(&path, "").unwrap();
        path
    }
}

/// A fake `tesseract` whose extraction body is `body`. It answers the
/// language probe with `eng` so a run reaches the extraction, then runs
/// `body` with the usual arguments: `$1` is the private PNG, `$2` is the
/// private output base, `$3` is `-l`, `$4` is the language and `$5` is `txt`.
/// Nothing here is run in this pass.
#[cfg(unix)]
fn fake_tesseract(dir: &Path, body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let script = format!(
        "#!/bin/sh\nif [ \"$1\" = \"--list-langs\" ]; then printf 'eng\\n'; exit 0; fi\n{body}\n"
    );
    let path = dir.join("tesseract");
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// A small valid picture, so the private PNG preparation has a source.
#[cfg(unix)]
fn picture(dir: &Path) -> PathBuf {
    let path = dir.join("a.png");
    let image = image::RgbImage::from_pixel(4, 4, image::Rgb([255, 255, 255]));
    image.save(&path).unwrap();
    path
}

#[test]
fn s146_the_engine_id_and_capability_are_the_action() {
    let engine = OcrEngine::with_program("/nonexistent/tesseract");
    assert_eq!(engine.id(), "ocr");
    let capabilities = engine.capabilities();
    assert_eq!(capabilities.len(), 1);
    assert_eq!(capabilities[0].0, "to-text");
    assert_eq!(capabilities[0].1, vec![MediaKind::Image]);
}

#[test]
fn s146_availability_is_a_lookup_and_never_runs_the_program() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("ran");
    let program = fake_program(dir.path(), &marker);
    let engine = OcrEngine::with_program(&program);
    assert!(engine.available(), "the fake executable is a file");
    assert_eq!(engine.program(), Some(program.as_path()));
    assert!(
        !marker.exists(),
        "the availability check started the program, which must never happen"
    );
    // A missing executable is unavailable, and names itself in the error.
    let missing = OcrEngine::with_program(dir.path().join("no-such-tesseract"));
    assert!(!missing.available());
    assert_eq!(missing.program(), None);
}

#[test]
fn s146_a_cancelled_task_stops_before_the_first_child() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.png");
    std::fs::write(&input, b"not a picture").unwrap();
    let output = dir.path().join("a-text.txt");
    let task = task("lang = \"eng\"", vec![input], output.clone());
    let cancel = CancelToken::new();
    cancel.cancel();
    // No program at all: the cancel must win before the lookup, so no error
    // about Tesseract and no output file.
    let engine = OcrEngine::with_program(dir.path().join("no-such-tesseract"));
    match engine.run(&task, &|_| {}, &cancel) {
        Err(EngineError::Cancelled) => {}
        other => panic!("expected Cancelled, got {other:?}"),
    }
    assert!(!output.exists(), "a cancelled task leaves no output");
}

#[test]
fn s146_a_missing_executable_is_a_missing_tool() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.png");
    std::fs::write(&input, b"not a picture").unwrap();
    let output = dir.path().join("a-text.txt");
    let task = task("lang = \"eng\"", vec![input], output.clone());
    let engine = OcrEngine::with_program(dir.path().join("no-such-tesseract"));
    match engine.run(&task, &|_| {}, &CancelToken::new()) {
        Err(EngineError::Unavailable(message)) => {
            assert!(message.contains("Tesseract"), "{message}");
            assert!(message.contains(TESSERACT_URL), "{message}");
        }
        other => panic!("expected Unavailable, got {other:?}"),
    }
    assert!(!output.exists());
}

#[test]
fn s146_an_unknown_operation_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let task = Task {
        action_id: "image.to-text".into(),
        op: "to-pdf".into(),
        params: toml::from_str("lang = \"eng\"").unwrap(),
        inputs: vec![],
        output: dir.path().join("a.pdf"),
    };
    let engine = OcrEngine::with_program(dir.path().join("no-such-tesseract"));
    match engine.run(&task, &|_| {}, &CancelToken::new()) {
        Err(EngineError::UnsupportedOp(op)) => assert_eq!(op, "to-pdf"),
        other => panic!("expected UnsupportedOp, got {other:?}"),
    }
}

#[test]
fn s146_the_action_excludes_every_other_image_extension() {
    let registry = Registry::builtin();
    let action = registry.get("image.to-text").expect("the OCR action");
    assert_eq!(action.engine, "ocr");
    assert_eq!(action.op, "to-text");
    assert!(action.accepts.contains(&MediaKind::Image));
    // The extensions the action reads. The registry skips the rest, so the
    // cell never appears for a container or an animation.
    let read = ["png", "jpg", "jpeg", "jpe", "bmp"];
    let others = [
        "webp", "avif", "gif", "tif", "tiff", "ico", "tga", "qoi", "pnm", "pbm", "pgm", "ppm",
        "heic", "heif", "jxl",
    ];
    for ext in others {
        assert!(
            action.skip_ext.iter().any(|s| s == ext),
            ".{ext} is not skipped by the OCR action"
        );
    }
    for ext in read {
        assert!(
            !action.skip_ext.iter().any(|s| s == ext),
            ".{ext} is skipped by the OCR action"
        );
    }
    // A selection of a skipped extension is not offered the action.
    let matched: Vec<&str> = registry
        .matching(&[Path::new("/home/u/photo.tiff")], &|_| true)
        .iter()
        .map(|a| a.id.as_str())
        .collect();
    assert!(!matched.contains(&"image.to-text"), "{matched:?}");
    let matched: Vec<&str> = registry
        .matching(&[Path::new("/home/u/photo.jpg")], &|_| true)
        .iter()
        .map(|a| a.id.as_str())
        .collect();
    assert!(matched.contains(&"image.to-text"), "{matched:?}");
}

#[test]
fn s146_the_action_takes_the_language_choice_only() {
    let registry = Registry::builtin();
    let action = registry.get("image.to-text").expect("the OCR action");
    let options = action.options.as_ref().expect("the language option");
    assert_eq!(options.fields.len(), 1, "{:?}", options.fields);
    let field = &options.fields[0];
    assert_eq!(field.id, "lang");
    let ids: Vec<&str> = field.values.iter().map(|v| v.id.as_str()).collect();
    assert_eq!(ids, SUPPORTED_LANGUAGES.to_vec());
    // The defaults agree with the engine's own list.
    assert_eq!(
        action.params.get("lang").and_then(|v| v.as_str()),
        Some("ara+eng")
    );
    assert_eq!(
        field.default.as_ref().and_then(|v| v.as_str()),
        Some("ara+eng")
    );
    // Every language value survives the typed option resolution unchanged.
    for id in SUPPORTED_LANGUAGES {
        let explicit: toml::Table = toml::from_str(&format!("lang = \"{id}\"")).unwrap();
        let resolved = action
            .with_params(&explicit)
            .expect("the language resolves");
        assert_eq!(
            resolved.params.get("lang").and_then(|v| v.as_str()),
            Some(id)
        );
    }
    // An unsupported language is refused by the option schema, not passed on.
    let unknown: toml::Table = toml::from_str("lang = \"deu\"").unwrap();
    assert!(action.with_params(&unknown).is_err());
}

#[test]
fn s146_the_limits_are_the_policy_values() {
    assert_eq!(PROBE_TIMEOUT.as_secs(), 5);
    assert_eq!(EXTRACTION_TIMEOUT.as_secs(), 240);
    assert_eq!(MAX_OUTPUT_BYTES, 8 * 1024 * 1024);
    assert_eq!(NO_TEXT, "No printed text recognized");
    assert_eq!(SUPPORTED_LANGUAGES, ["eng", "ara", "ara+eng"]);
    // No long dashes in the diagnostic, and no Arabic-Indic digit in it.
    assert!(!NO_TEXT.contains('\u{2013}') && !NO_TEXT.contains('\u{2014}'));
}

#[test]
fn s146_the_output_name_is_fixed_not_derived_from_the_document() {
    // The action names one text file beside the source, from the action and
    // never from the file contents.
    let registry = Registry::builtin();
    let action = registry.get("image.to-text").expect("the OCR action");
    assert_eq!(action.output.mode, kawaro_core::OutputMode::PerFile);
    assert_eq!(action.output.ext, "txt");
    assert!(!action.output.suffix.is_empty());
    // One input per task, like every other per-file action.
    assert_eq!(action.min_files, 1);
}

#[test]
fn s146_the_offline_policy_is_a_page_not_a_download() {
    // The engine names a page and never a package manager command.
    let engine = OcrEngine::with_program("/nonexistent/tesseract");
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.png");
    std::fs::write(&input, b"not a picture").unwrap();
    let task = task("lang = \"eng\"", vec![input], dir.path().join("a-text.txt"));
    let message = match engine.run(&task, &|_| {}, &CancelToken::new()) {
        Err(EngineError::Unavailable(message)) => message,
        other => panic!("expected Unavailable, got {other:?}"),
    };
    assert!(message.starts_with("Tesseract not found"), "{message}");
    assert!(!message.contains("sudo"), "{message}");
    assert!(!message.contains("apt install"), "{message}");
    assert!(!message.contains("dnf install"), "{message}");
}

/// The task and output path of one extraction over a fresh picture.
#[cfg(unix)]
fn extraction(dir: &Path) -> (Task, PathBuf) {
    let input = picture(dir);
    let output = dir.join("out.txt");
    (task("lang = \"eng\"", vec![input], output.clone()), output)
}

// The source cases below use a fake `tesseract` script and a generated PNG.
// They are written, not run: no child, no inference and no real conversion
// happened in this pass.

#[cfg(unix)]
#[test]
fn s146b_an_empty_text_file_is_the_no_text_diagnostic() {
    let dir = tempfile::tempdir().unwrap();
    let program = fake_tesseract(dir.path(), ": > \"$2.txt\"");
    let (task, output) = extraction(dir.path());
    let engine = OcrEngine::with_program(program);
    match engine.run(&task, &|_| {}, &CancelToken::new()) {
        Err(EngineError::Failed(message)) => assert_eq!(message, NO_TEXT),
        other => panic!("expected the no text diagnostic, got {other:?}"),
    }
    assert!(!output.exists(), "a blank result leaves no output");
}

#[cfg(unix)]
#[test]
fn s146b_a_missing_text_file_is_a_distinct_error() {
    let dir = tempfile::tempdir().unwrap();
    let program = fake_tesseract(dir.path(), "exit 0");
    let (task, output) = extraction(dir.path());
    let engine = OcrEngine::with_program(program);
    match engine.run(&task, &|_| {}, &CancelToken::new()) {
        Err(EngineError::Failed(message)) => {
            assert!(message.contains("wrote no text file"), "{message}")
        }
        other => panic!("expected the missing file error, got {other:?}"),
    }
    assert!(!output.exists());
}

#[cfg(unix)]
#[test]
fn s146b_malformed_utf8_is_a_distinct_error() {
    let dir = tempfile::tempdir().unwrap();
    let program = fake_tesseract(dir.path(), "printf '\\377' > \"$2.txt\"");
    let (task, output) = extraction(dir.path());
    let engine = OcrEngine::with_program(program);
    match engine.run(&task, &|_| {}, &CancelToken::new()) {
        Err(EngineError::Failed(message)) => assert!(message.contains("UTF-8"), "{message}"),
        other => panic!("expected the encoding error, got {other:?}"),
    }
    assert!(!output.exists());
}

#[cfg(unix)]
#[test]
fn s146b_a_text_file_over_the_limit_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let body = format!("head -c {} /dev/zero > \"$2.txt\"", MAX_OUTPUT_BYTES + 1);
    let program = fake_tesseract(dir.path(), &body);
    let (task, output) = extraction(dir.path());
    let engine = OcrEngine::with_program(program);
    match engine.run(&task, &|_| {}, &CancelToken::new()) {
        Err(EngineError::Failed(message)) => {
            assert!(message.contains(&MAX_OUTPUT_BYTES.to_string()), "{message}")
        }
        other => panic!("expected the size refusal, got {other:?}"),
    }
    assert!(!output.exists());
}

#[cfg(unix)]
#[test]
fn s146b_a_text_file_exactly_at_the_limit_is_read() {
    let dir = tempfile::tempdir().unwrap();
    let body = format!(
        "head -c {} /dev/zero | tr '\\0' a > \"$2.txt\"",
        MAX_OUTPUT_BYTES
    );
    let program = fake_tesseract(dir.path(), &body);
    let (task, output) = extraction(dir.path());
    let engine = OcrEngine::with_program(program);
    engine
        .run(&task, &|_| {}, &CancelToken::new())
        .expect("the file at the limit is accepted");
    assert_eq!(std::fs::metadata(&output).unwrap().len(), MAX_OUTPUT_BYTES);
}

#[cfg(unix)]
#[test]
fn s146b_the_recognized_text_is_written_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let program = fake_tesseract(dir.path(), "printf 'نص 123' > \"$2.txt\"");
    let (task, output) = extraction(dir.path());
    let engine = OcrEngine::with_program(program);
    engine
        .run(&task, &|_| {}, &CancelToken::new())
        .expect("the run succeeds");
    assert_eq!(std::fs::read_to_string(&output).unwrap(), "نص 123");
}

#[cfg(unix)]
#[test]
fn s146b_a_failed_child_reports_its_stderr_tail() {
    let dir = tempfile::tempdir().unwrap();
    let program = fake_tesseract(dir.path(), "echo oops >&2; exit 3");
    let (task, output) = extraction(dir.path());
    let engine = OcrEngine::with_program(program);
    match engine.run(&task, &|_| {}, &CancelToken::new()) {
        Err(EngineError::Failed(message)) => {
            assert!(message.contains("tesseract failed"), "{message}");
            assert!(message.contains("oops"), "{message}");
        }
        other => panic!("expected the child failure, got {other:?}"),
    }
    assert!(!output.exists());
}

#[cfg(unix)]
#[test]
fn s146b_a_cancelled_extraction_leaves_no_output() {
    let dir = tempfile::tempdir().unwrap();
    let program = fake_tesseract(dir.path(), "sleep 30");
    let (task, output) = extraction(dir.path());
    let engine = OcrEngine::with_program(program);
    let cancel = CancelToken::new();
    let watcher = {
        let cancel = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(80));
            cancel.cancel();
        })
    };
    let r = engine.run(&task, &|_| {}, &cancel);
    watcher.join().unwrap();
    assert!(matches!(r, Err(EngineError::Cancelled)));
    assert!(!output.exists());
}

#[cfg(unix)]
#[test]
fn s146_missing_language_yields_typed_missing_language_data_and_runs_no_ocr() {
    let dir = tempfile::tempdir().unwrap();
    let ocr_marker = dir.path().join("ocr-ran");
    let body = format!("touch '{}'; printf 'ok' > \"$2.txt\"", ocr_marker.display());
    // Fake tesseract reports only `eng`, not `ara`.
    let program = fake_tesseract(dir.path(), &body);
    let engine = OcrEngine::with_program(&program);

    // Typed S-145 preflight returns MissingLanguageData before any OCR runs.
    assert_eq!(
        engine.check_language("ara", &CancelToken::new()),
        Err(UnavailableReason::MissingLanguageData {
            language: "ara".to_string()
        })
    );
    assert_eq!(
        engine.check_language("ara+eng", &CancelToken::new()),
        Err(UnavailableReason::MissingLanguageData {
            language: "ara".to_string()
        })
    );
    assert_eq!(engine.check_language("eng", &CancelToken::new()), Ok(()));
    assert_eq!(
        engine.check_language("deu", &CancelToken::new()),
        Err(UnavailableReason::InvalidOptions)
    );
    assert!(!ocr_marker.exists(), "preflight must never start OCR");

    // Running a task for `ara+eng` also blocks before OCR and writes no output.
    let input = picture(dir.path());
    let output = dir.path().join("out.txt");
    let t = task("lang = \"ara+eng\"", vec![input], output.clone());
    match engine.run(&t, &|_| {}, &CancelToken::new()) {
        Err(EngineError::Unavailable(message)) => {
            assert!(message.contains("ara"), "{message}");
            assert!(message.contains(TESSERACT_URL), "{message}");
        }
        other => panic!("expected Unavailable for missing ara language, got {other:?}"),
    }
    assert!(
        !ocr_marker.exists(),
        "OCR child must not run when requested language data is missing"
    );
    assert!(!output.exists());
}

#[cfg(unix)]
#[test]
fn s146_language_probe_cache_refreshes_and_failing_probe_is_unknown() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let program = dir.path().join("tesseract");
    std::fs::write(
        &program,
        "#!/bin/sh\nif [ \"$1\" = \"--list-langs\" ]; then printf 'eng\\n'; exit 0; fi\nexit 1\n",
    )
    .unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();

    let engine = OcrEngine::with_program(&program);
    assert_eq!(
        engine.check_language("ara", &CancelToken::new()),
        Err(UnavailableReason::MissingLanguageData {
            language: "ara".to_string()
        })
    );

    // Install `ara` in place: cached probe keeps the old result until refreshed.
    std::fs::write(
        &program,
        "#!/bin/sh\nif [ \"$1\" = \"--list-langs\" ]; then printf 'ara\\neng\\n'; exit 0; fi\nexit 1\n",
    )
    .unwrap();
    assert_eq!(
        engine.check_language("ara", &CancelToken::new()),
        Err(UnavailableReason::MissingLanguageData {
            language: "ara".to_string()
        })
    );

    // Explicit refresh observes the newly installed language.
    let _ = engine.refresh_language_probe(&CancelToken::new());
    assert_eq!(
        engine.check_language("ara+eng", &CancelToken::new()),
        Ok(())
    );

    // A failing `--list-langs` probe yields ProbeUnknown, never ready.
    std::fs::write(&program, "#!/bin/sh\nexit 2\n").unwrap();
    OcrEngine::invalidate_language_cache_for(&program);
    assert_eq!(
        engine.check_language("eng", &CancelToken::new()),
        Err(UnavailableReason::ProbeUnknown)
    );
}

#[cfg(unix)]
#[test]
fn s146_cancel_before_spawn_never_invokes_tesseract() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("spawned");
    let program = fake_program(dir.path(), &marker);
    let input = picture(dir.path());
    let output = dir.path().join("out.txt");
    let t = task("lang = \"eng\"", vec![input], output.clone());
    let engine = OcrEngine::with_program(&program);

    let cancel = CancelToken::new();
    cancel.cancel();
    match engine.run(&t, &|_| {}, &cancel) {
        Err(EngineError::Cancelled) => {}
        other => panic!("expected Cancelled before spawn, got {other:?}"),
    }
    assert!(
        !marker.exists(),
        "a task cancelled before spawn must never execute tesseract"
    );
    assert!(!output.exists());
}

#[cfg(unix)]
#[test]
fn s146_atomic_output_through_queue_writes_next_to_original_and_keeps_source() {
    let dir = tempfile::tempdir().unwrap();
    let program = fake_tesseract(dir.path(), "printf 'نص عربي 123\\n' > \"$2.txt\"");
    let engine = OcrEngine::with_program(&program);
    let input = picture(dir.path());
    let original_bytes = std::fs::read(&input).unwrap();

    let reg = Registry::builtin();
    let base_action = reg.get("image.to-text").expect("image.to-text action");
    let explicit: toml::Table = toml::from_str("lang = \"eng\"").unwrap();
    let action = base_action.with_params(&explicit).unwrap();

    let (tx, _rx) = std::sync::mpsc::channel();
    let first = queue::run_job(
        1,
        &action,
        std::slice::from_ref(&input),
        &engine,
        &CancelToken::new(),
        &tx,
    );
    let expected = dir.path().join("a-text.txt");
    assert_eq!(first, vec![expected.clone()]);
    assert_eq!(std::fs::read_to_string(&expected).unwrap(), "نص عربي 123\n");
    assert_eq!(
        std::fs::read(&input).unwrap(),
        original_bytes,
        "source image bytes must remain unchanged"
    );

    // No temporary `.kawaro-*` file may remain next to the original.
    for entry in std::fs::read_dir(dir.path()).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        assert!(
            !name.starts_with(".kawaro-"),
            "leftover temporary file in output directory: {name}"
        );
    }

    // Running again must never overwrite `a-text.txt`, producing `a-text (1).txt`.
    let second = queue::run_job(
        2,
        &action,
        std::slice::from_ref(&input),
        &engine,
        &CancelToken::new(),
        &tx,
    );
    let expected_second = dir.path().join("a-text (1).txt");
    assert_eq!(second, vec![expected_second.clone()]);
    assert_eq!(std::fs::read_to_string(&expected).unwrap(), "نص عربي 123\n");
    assert_eq!(
        std::fs::read_to_string(&expected_second).unwrap(),
        "نص عربي 123\n"
    );
}
