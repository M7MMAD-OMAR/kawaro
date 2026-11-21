//! S-150 at the engine boundary: action metadata, discovery, version floor,
//! argv, document safety, process-tree cancellation, temp isolation, and queue
//! atomic output of the OCRmyPDF adapter, from outside the crate.
//!
//! Uses fake `ocrmypdf` and `tesseract` shell scripts on Unix so the contract
//! can be checked without requiring real OCRmyPDF or traineddata on the host.

use kawaro_core::action::Registry;
#[cfg(unix)]
use kawaro_core::queue;
#[cfg(unix)]
use kawaro_core::unsupported::{ProbeOutcome, UnavailableReason};
use kawaro_core::{CancelToken, Engine, EngineError, MediaKind, OutputMode, Task};
use kawaro_engines::locate::{self, Os, Program, Registry as LocateRegistry};
use kawaro_engines::ocrmypdf::{
    JOB_TIMEOUT, MAX_INPUT_BYTES, MAX_INPUT_PAGES, MAX_OUTPUT_BYTES, MIN_VERSION, OCRMYPDF_URL,
    OcrmypdfEngine, PER_PAGE_TIMEOUT_SECS, PROBE_TIMEOUT, SUPPORTED_LANGUAGES,
};
#[cfg(unix)]
use lopdf::{Document, Object, Stream, dictionary};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::time::{Duration, Instant};

struct EmptyRegistry;

impl LocateRegistry for EmptyRegistry {
    fn value(&self, _key: &str, _name: &str) -> Option<String> {
        None
    }

    fn subkeys(&self, _key: &str) -> Vec<String> {
        Vec::new()
    }
}

fn task(params: &str, inputs: Vec<PathBuf>, output: PathBuf) -> Task {
    Task {
        action_id: "pdf.make-searchable".into(),
        op: "make-searchable".into(),
        params: toml::from_str(params).unwrap(),
        inputs,
        output,
    }
}

/// A fake executable whose invocation leaves a marker file.
fn fake_marker_program(dir: &Path, name: &str, marker: &Path) -> PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let script = format!("#!/bin/sh\ntouch '{}'\n", marker.display());
        let path = dir.join(name);
        std::fs::write(&path, script).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }
    #[cfg(not(unix))]
    {
        let _ = marker;
        let path = dir.join(format!("{name}.exe"));
        std::fs::write(&path, "").unwrap();
        path
    }
}

/// Writes an executable shell script at `dir/name`.
#[cfg(unix)]
fn write_script(dir: &Path, name: &str, script: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join(name);
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// A fake `tesseract` that answers `--list-langs` with `langs`.
#[cfg(unix)]
fn fake_tesseract(dir: &Path, langs: &[&str]) -> PathBuf {
    let listing = langs.join("\\n");
    let script = format!(
        "#!/bin/sh\nif [ \"$1\" = \"--list-langs\" ]; then printf 'List of available languages:\\n{listing}\\n'; exit 0; fi\nexit 1\n"
    );
    write_script(dir, "tesseract", &script)
}

/// A fake `ocrmypdf` answering `--version` with `version`, and otherwise
/// executing `body`.
#[cfg(unix)]
fn fake_ocrmypdf(dir: &Path, version: &str, body: &str) -> PathBuf {
    let script = format!(
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then printf '%s\\n' '{version}'; exit 0; fi\n{body}\n"
    );
    write_script(dir, "ocrmypdf", &script)
}

/// Generates a minimal valid multi-page PDF using `lopdf`.
#[cfg(unix)]
fn sample_pdf(dir: &Path, name: &str, page_count: usize) -> PathBuf {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let mut kids = Vec::with_capacity(page_count);
    for _ in 0..page_count {
        let content_id = doc.add_object(Stream::new(dictionary! {}, Vec::new()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            "Contents" => content_id,
        });
        kids.push(Object::Reference(page_id));
    }
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Count" => page_count as i64,
            "Kids" => kids,
        }),
    );
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);
    let path = dir.join(name);
    doc.save(&path).unwrap();
    path
}

#[test]
fn s150_engine_id_and_capability_match_action() {
    let engine = OcrmypdfEngine::with_programs("/nonexistent/ocrmypdf", "/nonexistent/tesseract");
    assert_eq!(engine.id(), "ocrmypdf");
    let capabilities = engine.capabilities();
    assert_eq!(capabilities.len(), 1);
    assert_eq!(capabilities[0].0, "make-searchable");
    assert_eq!(capabilities[0].1, vec![MediaKind::Pdf]);
}

#[test]
fn s150_availability_is_a_lookup_and_never_runs_program() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("ran");
    let program = fake_marker_program(dir.path(), "ocrmypdf", &marker);
    let engine = OcrmypdfEngine::with_programs(&program, dir.path().join("no-tesseract"));
    assert!(engine.available(), "existing file counts as available");
    assert_eq!(engine.program(), Some(program.as_path()));
    assert!(
        !marker.exists(),
        "availability check must never execute ocrmypdf"
    );
    let missing = OcrmypdfEngine::with_programs(
        dir.path().join("no-such-ocrmypdf"),
        dir.path().join("no-such-tesseract"),
    );
    assert!(!missing.available());
    assert_eq!(missing.program(), None);
}

#[test]
fn s150_locate_finds_ocrmypdf_on_unix_and_windows_path() {
    let dir = tempfile::tempdir().unwrap();
    let unix_bin = dir.path().join("unix-bin");
    let win_bin = dir.path().join("win-bin");
    std::fs::create_dir_all(&unix_bin).unwrap();
    std::fs::create_dir_all(&win_bin).unwrap();
    let unix_prog = unix_bin.join("ocrmypdf");
    std::fs::write(&unix_prog, "#!/bin/sh\n").unwrap();
    let win_prog = win_bin.join("ocrmypdf.exe");
    std::fs::write(&win_prog, "").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&unix_prog, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::set_permissions(&win_prog, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let no_env = |_: &str| -> Option<OsString> { None };
    // The Unix lookup checks the executable bit, which only a Unix host can
    // set, so that half runs on Unix; the Windows half runs everywhere.
    #[cfg(unix)]
    assert_eq!(
        locate::search(
            Os::Unix,
            Program::Ocrmypdf,
            std::slice::from_ref(&unix_bin),
            &no_env,
            None,
            &EmptyRegistry,
        ),
        Some(unix_prog)
    );
    #[cfg(not(unix))]
    let _ = unix_prog;
    assert_eq!(
        locate::search(
            Os::Windows,
            Program::Ocrmypdf,
            std::slice::from_ref(&win_bin),
            &no_env,
            None,
            &EmptyRegistry,
        ),
        Some(win_prog)
    );
    assert_eq!(
        locate::search(
            Os::Unix,
            Program::Ocrmypdf,
            &[dir.path().join("empty")],
            &no_env,
            None,
            &EmptyRegistry,
        ),
        None
    );
}

#[test]
fn s150_action_metadata_and_language_enum() {
    let registry = Registry::builtin();
    let action = registry
        .get("pdf.make-searchable")
        .expect("pdf.make-searchable action");
    assert_eq!(action.engine, "ocrmypdf");
    assert_eq!(action.op, "make-searchable");
    assert_eq!(action.accepts, vec![MediaKind::Pdf]);
    assert_eq!(action.min_files, 1);
    assert_eq!(action.output.mode, OutputMode::PerFile);
    assert_eq!(action.output.ext, "pdf");
    assert_eq!(action.output.suffix, "-searchable");
    for text in [
        &action.code.ar,
        &action.code.en,
        &action.label.ar,
        &action.label.en,
    ] {
        assert!(!text.is_empty());
        assert!(
            !text.contains('\u{2013}') && !text.contains('\u{2014}'),
            "{text}"
        );
    }

    let options = action.options.as_ref().expect("language option");
    assert_eq!(options.fields.len(), 1);
    let field = &options.fields[0];
    assert_eq!(field.id, "lang");
    let ids: Vec<&str> = field.values.iter().map(|v| v.id.as_str()).collect();
    assert_eq!(ids, SUPPORTED_LANGUAGES.to_vec());
    assert_eq!(
        action.params.get("lang").and_then(|v| v.as_str()),
        Some("ara+eng")
    );
    assert_eq!(
        field.default.as_ref().and_then(|v| v.as_str()),
        Some("ara+eng")
    );
    for id in SUPPORTED_LANGUAGES {
        let explicit: toml::Table = toml::from_str(&format!("lang = \"{id}\"")).unwrap();
        let resolved = action.with_params(&explicit).expect("valid lang resolves");
        assert_eq!(
            resolved.params.get("lang").and_then(|v| v.as_str()),
            Some(id)
        );
    }
    let bad: toml::Table = toml::from_str("lang = \"deu\"").unwrap();
    assert!(action.with_params(&bad).is_err());
}

#[test]
fn s150_limits_are_policy_values() {
    assert_eq!(MIN_VERSION, (14, 4, 0));
    assert_eq!(PROBE_TIMEOUT.as_secs(), 5);
    assert_eq!(PER_PAGE_TIMEOUT_SECS, 180);
    assert_eq!(JOB_TIMEOUT.as_secs(), 600);
    assert_eq!(MAX_INPUT_BYTES, 128 * 1024 * 1024);
    assert_eq!(MAX_INPUT_PAGES, 250);
    assert_eq!(MAX_OUTPUT_BYTES, 256 * 1024 * 1024);
    assert_eq!(SUPPORTED_LANGUAGES, ["eng", "ara", "ara+eng"]);
}

#[test]
fn s150_cancel_before_spawn_never_runs_child() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("spawned");
    let program = fake_marker_program(dir.path(), "ocrmypdf", &marker);
    let input = dir.path().join("a.pdf");
    std::fs::write(&input, b"%PDF-1.5\n%%EOF\n").unwrap();
    let output = dir.path().join("a-searchable.pdf");
    let t = task("lang = \"eng\"", vec![input], output.clone());
    let engine = OcrmypdfEngine::with_programs(&program, dir.path().join("no-tesseract"));

    let cancel = CancelToken::new();
    cancel.cancel();
    match engine.run(&t, &|_| {}, &cancel) {
        Err(EngineError::Cancelled) => {}
        other => panic!("expected Cancelled, got {other:?}"),
    }
    assert!(!marker.exists(), "cancelled task must not execute ocrmypdf");
    assert!(!output.exists());
}

#[test]
fn s150_missing_executable_unknown_op_and_bad_lang() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.pdf");
    std::fs::write(&input, b"%PDF-1.5\n%%EOF\n").unwrap();
    let output = dir.path().join("a-searchable.pdf");
    let engine = OcrmypdfEngine::with_programs(
        dir.path().join("no-ocrmypdf"),
        dir.path().join("no-tesseract"),
    );

    let t = task("lang = \"eng\"", vec![input.clone()], output.clone());
    match engine.run(&t, &|_| {}, &CancelToken::new()) {
        Err(EngineError::Unavailable(msg)) => {
            assert!(msg.contains("OCRmyPDF"), "{msg}");
            assert!(msg.contains(OCRMYPDF_URL), "{msg}");
            assert!(!msg.contains("sudo"), "{msg}");
            assert!(!msg.contains("apt install"), "{msg}");
            assert!(!msg.contains("dnf install"), "{msg}");
        }
        other => panic!("expected Unavailable, got {other:?}"),
    }

    let bad_lang = task("lang = \"deu\"", vec![input.clone()], output.clone());
    match engine.run(&bad_lang, &|_| {}, &CancelToken::new()) {
        Err(EngineError::BadParam { name, .. }) => assert_eq!(name, "lang"),
        other => panic!("expected BadParam for lang, got {other:?}"),
    }

    let bad_op = Task {
        action_id: "pdf.make-searchable".into(),
        op: "to-text".into(),
        params: toml::from_str("lang = \"eng\"").unwrap(),
        inputs: vec![input],
        output,
    };
    match engine.run(&bad_op, &|_| {}, &CancelToken::new()) {
        Err(EngineError::UnsupportedOp(op)) => assert_eq!(op, "to-text"),
        other => panic!("expected UnsupportedOp, got {other:?}"),
    }
}

#[cfg(unix)]
#[test]
fn s150_version_probe_enforces_floor_caches_and_reports_missing_language() {
    let dir = tempfile::tempdir().unwrap();
    let ocr_marker = dir.path().join("ocr-ran");
    let tesseract = fake_tesseract(dir.path(), &["eng"]);

    // Missing binary -> MissingExecutable { program: "ocrmypdf" }.
    let missing_engine = OcrmypdfEngine::with_programs(dir.path().join("no-ocrmypdf"), &tesseract);
    assert_eq!(
        missing_engine.probe_version(&CancelToken::new()),
        ProbeOutcome::Missing
    );
    assert_eq!(
        missing_engine.check_ready("eng", &CancelToken::new()),
        Err(UnavailableReason::MissingExecutable {
            program: "ocrmypdf".to_string()
        })
    );

    // Below floor (14.0.1) -> ProbeOutcome::Unknown -> UnavailableReason::ProbeUnknown.
    let body = format!("touch '{}'; exit 0", ocr_marker.display());
    let ocrmypdf = fake_ocrmypdf(dir.path(), "14.0.1+dfsg1-1", &body);
    let engine = OcrmypdfEngine::with_programs(&ocrmypdf, &tesseract);
    assert_eq!(
        engine.probe_version(&CancelToken::new()),
        ProbeOutcome::Unknown
    );
    assert_eq!(
        engine.check_ready("eng", &CancelToken::new()),
        Err(UnavailableReason::ProbeUnknown)
    );
    let input = sample_pdf(dir.path(), "in.pdf", 1);
    let output = dir.path().join("out.pdf");
    let t = task("lang = \"eng\"", vec![input], output.clone());
    match engine.run(&t, &|_| {}, &CancelToken::new()) {
        Err(EngineError::Unavailable(msg)) => assert!(msg.contains("14.4.0"), "{msg}"),
        other => panic!("expected Unavailable below 14.4.0, got {other:?}"),
    }
    assert!(!ocr_marker.exists(), "must not run OCR below version floor");
    assert!(!output.exists());

    // Upgrade in place to 15.2.0: cached result persists until explicit refresh.
    let _ = fake_ocrmypdf(dir.path(), "15.2.0", "exit 0");
    assert_eq!(
        engine.probe_version(&CancelToken::new()),
        ProbeOutcome::Unknown
    );
    assert_eq!(
        engine.refresh_version_probe(&CancelToken::new()),
        ProbeOutcome::Ready {
            items: vec!["15.2.0".to_string()]
        }
    );
    assert_eq!(engine.check_ready("eng", &CancelToken::new()), Ok(()));
    assert_eq!(
        engine.check_ready("ara", &CancelToken::new()),
        Err(UnavailableReason::MissingLanguageData {
            language: "ara".to_string()
        })
    );
    assert_eq!(
        engine.check_ready("ara+eng", &CancelToken::new()),
        Err(UnavailableReason::MissingLanguageData {
            language: "ara".to_string()
        })
    );

    // Malformed version string -> ProbeOutcome::Unknown after cache invalidation.
    let _ = fake_ocrmypdf(dir.path(), "not-a-version", "exit 0");
    OcrmypdfEngine::invalidate_version_cache_for(&ocrmypdf);
    assert_eq!(
        engine.probe_version(&CancelToken::new()),
        ProbeOutcome::Unknown
    );
}

#[cfg(unix)]
#[test]
fn s150_argv_and_temp_isolation_with_spaces_and_leading_dash() {
    let dir = tempfile::tempdir().unwrap();
    let temp_root = dir.path().join("work-root");
    std::fs::create_dir_all(&temp_root).unwrap();
    let argv_log = dir.path().join("argv.txt");
    let env_log = dir.path().join("env.txt");
    let tesseract = fake_tesseract(dir.path(), &["ara", "eng"]);
    let body = format!(
        "printf '%s\\n' \"$@\" > '{argv}'\n\
         printf '%s\\n%s\\n%s\\n' \"$TMPDIR\" \"$TEMP\" \"$TMP\" > '{env}'\n\
         # Copy penultimate arg (input) to last arg (output)\n\
         prev=''; last=''\n\
         for a in \"$@\"; do prev=\"$last\"; last=\"$a\"; done\n\
         cp \"$prev\" \"$last\"\n",
        argv = argv_log.display(),
        env = env_log.display(),
    );
    let ocrmypdf = fake_ocrmypdf(dir.path(), "16.7.0", &body);
    let input = sample_pdf(dir.path(), "-scan with spaces.pdf", 1);
    let original_bytes = std::fs::read(&input).unwrap();
    let output = dir.path().join("out.pdf");
    let t = task("lang = \"ara+eng\"", vec![input.clone()], output.clone());
    let engine = OcrmypdfEngine::with_programs(&ocrmypdf, &tesseract).temp_root(&temp_root);

    engine
        .run(&t, &|_| {}, &CancelToken::new())
        .expect("searchable PDF succeeds");
    assert!(output.is_file());
    assert_eq!(std::fs::read(&input).unwrap(), original_bytes);

    let recorded: Vec<String> = std::fs::read_to_string(&argv_log)
        .unwrap()
        .lines()
        .map(String::from)
        .collect();
    let abs_input = std::path::absolute(&input).unwrap();
    assert_eq!(recorded.len(), 15, "{recorded:?}");
    assert_eq!(
        &recorded[..13],
        [
            "--language",
            "ara+eng",
            "--skip-text",
            "--output-type",
            "pdf",
            "--optimize",
            "0",
            "--jobs",
            "1",
            "--tesseract-timeout",
            "180",
            "--quiet",
            "--",
        ]
    );
    assert_eq!(PathBuf::from(&recorded[13]), abs_input);
    assert!(
        recorded[14].ends_with("/searchable.pdf"),
        "{}",
        recorded[14]
    );
    for forbidden in [
        "--force-ocr",
        "--redo-ocr",
        "--invalidate-digital-signatures",
        "--sidecar",
    ] {
        assert!(!recorded.iter().any(|arg| arg == forbidden), "{recorded:?}");
    }

    let envs: Vec<String> = std::fs::read_to_string(&env_log)
        .unwrap()
        .lines()
        .map(String::from)
        .collect();
    assert_eq!(envs.len(), 3);
    assert_eq!(envs[0], envs[1]);
    assert_eq!(envs[1], envs[2]);
    assert!(
        Path::new(&envs[0]).starts_with(&temp_root),
        "TMPDIR {} must live inside temp_root {}",
        envs[0],
        temp_root.display()
    );
    assert!(
        !Path::new(&envs[0]).exists(),
        "private work dir must be deleted after completion"
    );
}

#[cfg(unix)]
#[test]
fn s150_refuses_signed_encrypted_and_corrupt_pdfs_without_output() {
    let dir = tempfile::tempdir().unwrap();
    let tesseract = fake_tesseract(dir.path(), &["eng"]);
    let input = sample_pdf(dir.path(), "doc.pdf", 1);
    let original_bytes = std::fs::read(&input).unwrap();

    // Digitally signed PDF (exit 2 / DigitalSignatureError).
    let signed_bin = fake_ocrmypdf(
        dir.path(),
        "15.2.0",
        "echo 'DigitalSignatureError: Input file has a digital signature' >&2; exit 2",
    );
    let signed_out = dir.path().join("signed-out.pdf");
    let engine = OcrmypdfEngine::with_programs(&signed_bin, &tesseract);
    match engine.run(
        &task("lang = \"eng\"", vec![input.clone()], signed_out.clone()),
        &|_| {},
        &CancelToken::new(),
    ) {
        Err(EngineError::Input { reason, .. }) => {
            assert!(reason.contains("digitally signed"), "{reason}");
        }
        other => panic!("expected Input for signed PDF, got {other:?}"),
    }
    assert!(!signed_out.exists());
    assert_eq!(std::fs::read(&input).unwrap(), original_bytes);

    // Encrypted PDF (exit 8 / EncryptedPdfError).
    let enc_dir = tempfile::tempdir().unwrap();
    let enc_bin = fake_ocrmypdf(
        enc_dir.path(),
        "15.2.0",
        "echo 'EncryptedPdfError: Input PDF is encrypted' >&2; exit 8",
    );
    let enc_out = dir.path().join("enc-out.pdf");
    let engine = OcrmypdfEngine::with_programs(&enc_bin, &tesseract);
    match engine.run(
        &task("lang = \"eng\"", vec![input.clone()], enc_out.clone()),
        &|_| {},
        &CancelToken::new(),
    ) {
        Err(EngineError::Input { reason, .. }) => {
            assert!(reason.contains("encrypted"), "{reason}");
        }
        other => panic!("expected Input for encrypted PDF, got {other:?}"),
    }
    assert!(!enc_out.exists());
    assert_eq!(std::fs::read(&input).unwrap(), original_bytes);

    // Corrupt non-PDF input: rejected before invoking child.
    let corrupt_dir = tempfile::tempdir().unwrap();
    let marker = corrupt_dir.path().join("child-ran");
    let corrupt_bin = fake_ocrmypdf(
        corrupt_dir.path(),
        "15.2.0",
        &format!("touch '{}'; exit 0", marker.display()),
    );
    let corrupt_in = dir.path().join("corrupt.pdf");
    std::fs::write(&corrupt_in, b"not a pdf at all").unwrap();
    let corrupt_out = dir.path().join("corrupt-out.pdf");
    let engine = OcrmypdfEngine::with_programs(&corrupt_bin, &tesseract);
    match engine.run(
        &task(
            "lang = \"eng\"",
            vec![corrupt_in.clone()],
            corrupt_out.clone(),
        ),
        &|_| {},
        &CancelToken::new(),
    ) {
        Err(EngineError::Input { .. }) => {}
        other => panic!("expected Input for non-PDF, got {other:?}"),
    }
    assert!(!marker.exists(), "non-PDF must fail before spawning child");
    assert!(!corrupt_out.exists());
    assert_eq!(std::fs::read(&corrupt_in).unwrap(), b"not a pdf at all");
}

#[cfg(unix)]
#[test]
fn s150_per_page_timeout_warning_fails_even_on_exit_zero() {
    let dir = tempfile::tempdir().unwrap();
    let tesseract = fake_tesseract(dir.path(), &["eng"]);
    let input = sample_pdf(dir.path(), "mixed.pdf", 2);

    let warn_bin = fake_ocrmypdf(
        dir.path(),
        "15.2.0",
        "prev=''; last=''\n\
         for a in \"$@\"; do prev=\"$last\"; last=\"$a\"; done\n\
         cp \"$prev\" \"$last\"\n\
         echo 'WARNING - Page 2: Tesseract timed out' >&2\n\
         exit 0",
    );
    let output = dir.path().join("partial.pdf");
    let engine = OcrmypdfEngine::with_programs(&warn_bin, &tesseract);
    match engine.run(
        &task("lang = \"eng\"", vec![input.clone()], output.clone()),
        &|_| {},
        &CancelToken::new(),
    ) {
        Err(EngineError::Failed(msg)) => assert!(msg.contains("per-page OCR timeout"), "{msg}"),
        other => panic!("expected Failed on per-page timeout warning, got {other:?}"),
    }
    assert!(
        !output.exists(),
        "incomplete per-page timeout run must commit no output"
    );

    // Normal `--skip-text` info line on born-digital pages succeeds.
    let ok_dir = tempfile::tempdir().unwrap();
    let ok_bin = fake_ocrmypdf(
        ok_dir.path(),
        "15.2.0",
        "prev=''; last=''\n\
         for a in \"$@\"; do prev=\"$last\"; last=\"$a\"; done\n\
         cp \"$prev\" \"$last\"\n\
         echo '1: page already has text! - skipping all processing on this page' >&2\n\
         exit 0",
    );
    let ok_out = dir.path().join("ok.pdf");
    let ok_engine = OcrmypdfEngine::with_programs(&ok_bin, &tesseract);
    ok_engine
        .run(
            &task("lang = \"eng\"", vec![input], ok_out.clone()),
            &|_| {},
            &CancelToken::new(),
        )
        .expect("normal skip-text warning must not fail the task");
    assert!(ok_out.is_file());
}

#[cfg(unix)]
#[test]
fn s150_cancel_and_timeout_reap_grandchild_and_leave_no_output() {
    let dir = tempfile::tempdir().unwrap();
    let temp_root = dir.path().join("work-root");
    std::fs::create_dir_all(&temp_root).unwrap();
    let tesseract = fake_tesseract(dir.path(), &["eng"]);
    let input = sample_pdf(dir.path(), "slow.pdf", 1);

    // Mid-run cancellation kills and reaps a sleeper grandchild.
    let pidfile = dir.path().join("grandchild.pid");
    let sleep_bin = fake_ocrmypdf(
        dir.path(),
        "15.2.0",
        &format!("sleep 30 & echo $! > '{}'; wait", pidfile.display()),
    );
    let output = dir.path().join("cancelled.pdf");
    let engine = OcrmypdfEngine::with_programs(&sleep_bin, &tesseract).temp_root(&temp_root);
    let cancel = CancelToken::new();
    let watcher = {
        let cancel = cancel.clone();
        let pidfile = pidfile.clone();
        std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(3);
            loop {
                if let Ok(text) = std::fs::read_to_string(&pidfile)
                    && let Ok(pid) = text.trim().parse::<i32>()
                {
                    cancel.cancel();
                    return Some(pid);
                }
                if Instant::now() >= deadline {
                    return None;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        })
    };
    let r = engine.run(
        &task("lang = \"eng\"", vec![input.clone()], output.clone()),
        &|_| {},
        &cancel,
    );
    let grandchild = watcher.join().unwrap().expect("grandchild pid recorded");
    assert!(matches!(r, Err(EngineError::Cancelled)), "{r:?}");
    assert!(!output.exists());
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let gone = match std::fs::read_to_string(format!("/proc/{grandchild}/stat")) {
            Err(_) => true,
            Ok(stat) => stat
                .rsplit_once(')')
                .and_then(|(_, rest)| rest.split_whitespace().next())
                .is_some_and(|state| state == "Z"),
        };
        if gone {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "grandchild {grandchild} survived"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(std::fs::read_dir(&temp_root).unwrap().count(), 0);

    // Whole-job timeout kills the child and leaves no output or temp dir.
    let timeout_out = dir.path().join("timed-out.pdf");
    let timeout_engine = OcrmypdfEngine::with_programs(&sleep_bin, &tesseract)
        .temp_root(&temp_root)
        .with_job_timeout(Duration::from_millis(150));
    match timeout_engine.run(
        &task("lang = \"eng\"", vec![input], timeout_out.clone()),
        &|_| {},
        &CancelToken::new(),
    ) {
        Err(EngineError::Failed(msg)) => assert!(msg.contains("timed out"), "{msg}"),
        other => panic!("expected timeout failure, got {other:?}"),
    }
    assert!(!timeout_out.exists());
    assert_eq!(std::fs::read_dir(&temp_root).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn s150_oversized_output_and_page_limit_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let tesseract = fake_tesseract(dir.path(), &["eng"]);
    let input = sample_pdf(dir.path(), "one.pdf", 1);

    // Output PDF exceeding configured byte cap is refused with no committed file.
    let big_bin = fake_ocrmypdf(
        dir.path(),
        "15.2.0",
        "last=''\n\
         for a in \"$@\"; do last=\"$a\"; done\n\
         printf '%%PDF-1.5\\n' > \"$last\"\n\
         head -c 2048 /dev/zero >> \"$last\"\n",
    );
    let output = dir.path().join("big.pdf");
    let engine = OcrmypdfEngine::with_programs(&big_bin, &tesseract).with_max_output_bytes(1024);
    match engine.run(
        &task("lang = \"eng\"", vec![input], output.clone()),
        &|_| {},
        &CancelToken::new(),
    ) {
        Err(EngineError::Failed(msg)) => assert!(msg.contains("1024"), "{msg}"),
        other => panic!("expected Failed on oversized output, got {other:?}"),
    }
    assert!(!output.exists());

    // Input PDF exceeding MAX_INPUT_PAGES (250) is refused before spawning.
    let too_many_pages = sample_pdf(dir.path(), "251.pdf", MAX_INPUT_PAGES + 1);
    let pages_out = dir.path().join("251-out.pdf");
    match engine.run(
        &task("lang = \"eng\"", vec![too_many_pages], pages_out.clone()),
        &|_| {},
        &CancelToken::new(),
    ) {
        Err(EngineError::Input { reason, .. }) => {
            assert!(reason.contains(&MAX_INPUT_PAGES.to_string()), "{reason}");
        }
        other => panic!("expected Input for > 250 pages, got {other:?}"),
    }
    assert!(!pages_out.exists());
}

#[cfg(unix)]
#[test]
fn s150_queue_finalizes_single_searchable_pdf_and_keeps_original() {
    let dir = tempfile::tempdir().unwrap();
    let tesseract = fake_tesseract(dir.path(), &["ara", "eng"]);
    let ocrmypdf = fake_ocrmypdf(
        dir.path(),
        "16.7.0",
        "prev=''; last=''\n\
         for a in \"$@\"; do prev=\"$last\"; last=\"$a\"; done\n\
         cp \"$prev\" \"$last\"\n",
    );
    let engine = OcrmypdfEngine::with_programs(&ocrmypdf, &tesseract);
    let input = sample_pdf(dir.path(), "scan.pdf", 2);
    let original_bytes = std::fs::read(&input).unwrap();

    let reg = Registry::builtin();
    let base_action = reg
        .get("pdf.make-searchable")
        .expect("pdf.make-searchable action");
    let explicit: toml::Table = toml::from_str("lang = \"ara+eng\"").unwrap();
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
    let expected = dir.path().join("scan-searchable.pdf");
    assert_eq!(first, vec![expected.clone()]);
    assert!(expected.is_file());
    assert_eq!(std::fs::read(&input).unwrap(), original_bytes);

    for entry in std::fs::read_dir(dir.path()).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        assert!(
            !name.starts_with(".kawaro-"),
            "leftover temporary file in output directory: {name}"
        );
    }

    // Running again must never overwrite `scan-searchable.pdf`.
    let second = queue::run_job(
        2,
        &action,
        std::slice::from_ref(&input),
        &engine,
        &CancelToken::new(),
        &tx,
    );
    let expected_second = dir.path().join("scan-searchable (1).pdf");
    assert_eq!(second, vec![expected_second.clone()]);
    assert!(expected_second.is_file());
    assert_eq!(std::fs::read(&input).unwrap(), original_bytes);
}
