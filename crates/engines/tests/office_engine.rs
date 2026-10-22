//! Office engine tests (S-009). Real conversions run LibreOffice; when it is
//! not installed those tests print a notice and return early.

use sbarhive_core::{CancelToken, Engine, EngineError, Task};
use sbarhive_engines::office::OfficeEngine;
#[cfg(unix)]
use std::ffi::OsString;
use std::io::{Read, Write};
#[cfg_attr(not(unix), allow(unused_imports))]
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

fn task(op: &str, input: &Path, output: &Path, params: &str) -> Task {
    Task {
        action_id: format!("test.{op}"),
        op: op.into(),
        params: toml::from_str(params).unwrap(),
        inputs: vec![input.to_path_buf()],
        output: output.to_path_buf(),
    }
}

/// Runs the task and returns the result plus every progress value reported.
fn run(
    engine: &OfficeEngine,
    task: &Task,
    cancel: &CancelToken,
) -> (Result<(), EngineError>, Vec<f32>) {
    let seen = Mutex::new(Vec::new());
    let progress = |f: f32| seen.lock().unwrap().push(f);
    let result = engine.run(task, &progress, cancel);
    (result, seen.into_inner().unwrap())
}

/// The real engine with its temporary files under `root`, or None to skip.
fn real_engine(root: &Path) -> Option<OfficeEngine> {
    let engine = OfficeEngine::default().temp_root(root);
    if engine.available() {
        Some(engine)
    } else {
        eprintln!("LibreOffice not installed, skipping");
        None
    }
}

fn pdf_pages(path: &Path) -> usize {
    let bytes = std::fs::read(path).unwrap();
    assert!(
        bytes.starts_with(b"%PDF-"),
        "{} is not a PDF",
        path.display()
    );
    lopdf::Document::load_mem(&bytes).unwrap().get_pages().len()
}

fn zip_entry(path: &Path, name: &str) -> Option<String> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    let mut entry = archive.by_name(name).ok()?;
    let mut text = String::new();
    entry.read_to_string(&mut text).ok()?;
    Some(text)
}

fn is_empty_dir(dir: &Path) -> bool {
    std::fs::read_dir(dir).unwrap().next().is_none()
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
        let cmdline = String::from_utf8_lossy(&cmdline).replace('\0', " ");
        if cmdline.contains(needle) && !is_zombie(&name) {
            found.push(format!("{name}: {cmdline}"));
        }
    }
    found
}

fn is_zombie(pid: &str) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .map(|s| {
            s.rsplit(')')
                .next()
                .unwrap_or("")
                .trim_start()
                .starts_with('Z')
        })
        .unwrap_or(true)
}

/// Waits up to 3 s for `pid` to disappear or become a zombie.
#[cfg(unix)]
fn wait_gone(pid: &str) -> bool {
    let end = Instant::now() + Duration::from_secs(3);
    while Instant::now() < end {
        if is_zombie(pid) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

fn wait_no_process(needle: &str) -> Vec<String> {
    let end = Instant::now() + Duration::from_secs(3);
    loop {
        let found = processes_mentioning(needle);
        if found.is_empty() || Instant::now() > end {
            return found;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

const TWO_PAGE_RTF: &str = "{\\rtf1\\ansi Page one\\page Page two}";

#[test]
fn office_rtf_to_docx_then_pdf_keeps_two_pages_and_removes_profile() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("tmp");
    std::fs::create_dir(&root).unwrap();
    let Some(engine) = real_engine(&root) else {
        return;
    };

    let rtf = dir.path().join("report.rtf");
    std::fs::write(&rtf, TWO_PAGE_RTF).unwrap();
    let docx = dir.path().join("out.docx.part.docx");
    let (result, progress) = run(
        &engine,
        &task("convert-doc", &rtf, &docx, ""),
        &CancelToken::new(),
    );
    result.unwrap();
    assert_eq!(progress.first(), Some(&0.0));
    assert_eq!(progress.last(), Some(&1.0));
    assert!(
        zip_entry(&docx, "word/document.xml").is_some(),
        "not a DOCX"
    );
    assert!(
        is_empty_dir(&root),
        "private profile or work dir left behind"
    );

    let pdf = dir.path().join("out.pdf");
    let (result, _) = run(
        &engine,
        &task("to-pdf", &docx, &pdf, ""),
        &CancelToken::new(),
    );
    result.unwrap();
    assert_eq!(pdf_pages(&pdf), 2);
    assert!(
        is_empty_dir(&root),
        "private profile or work dir left behind"
    );
    // The input is untouched and no lock file appears next to it.
    assert_eq!(std::fs::read_to_string(&rtf).unwrap(), TWO_PAGE_RTF);
    let names: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .flatten()
        .map(|e| e.file_name())
        .collect();
    assert_eq!(names.len(), 4, "unexpected files: {names:?}");
}

#[test]
fn office_text_to_odt_and_markdown_to_pdf() {
    let dir = tempfile::tempdir().unwrap();
    let Some(engine) = real_engine(dir.path()) else {
        return;
    };
    let txt = dir.path().join("-notes.txt");
    std::fs::write(&txt, "Hello from SbarHive\n").unwrap();
    let odt = dir.path().join("notes.odt");
    run(
        &engine,
        &task("convert-doc", &txt, &odt, ""),
        &CancelToken::new(),
    )
    .0
    .unwrap();
    assert_eq!(
        zip_entry(&odt, "mimetype").as_deref(),
        Some("application/vnd.oasis.opendocument.text")
    );

    let md = dir.path().join("readme.md");
    std::fs::write(&md, "# Title\n\nSome **bold** text.\n").unwrap();
    let pdf = dir.path().join("readme.pdf");
    run(&engine, &task("to-pdf", &md, &pdf, ""), &CancelToken::new())
        .0
        .unwrap();
    assert_eq!(pdf_pages(&pdf), 1);
}

#[test]
fn office_csv_to_xlsx_and_ods_then_pdf() {
    let dir = tempfile::tempdir().unwrap();
    let Some(engine) = real_engine(dir.path()) else {
        return;
    };
    let csv = dir.path().join("table.csv");
    std::fs::write(&csv, "name,count\nalpha,1\nbeta,2\n").unwrap();

    let xlsx = dir.path().join("table.xlsx");
    run(
        &engine,
        &task("convert-sheet", &csv, &xlsx, ""),
        &CancelToken::new(),
    )
    .0
    .unwrap();
    assert!(zip_entry(&xlsx, "xl/workbook.xml").is_some(), "not an XLSX");

    let ods = dir.path().join("table.ods");
    run(
        &engine,
        &task("convert-sheet", &xlsx, &ods, ""),
        &CancelToken::new(),
    )
    .0
    .unwrap();
    assert_eq!(
        zip_entry(&ods, "mimetype").as_deref(),
        Some("application/vnd.oasis.opendocument.spreadsheet")
    );

    let pdf = dir.path().join("table.pdf");
    run(
        &engine,
        &task("to-pdf", &ods, &pdf, ""),
        &CancelToken::new(),
    )
    .0
    .unwrap();
    assert_eq!(pdf_pages(&pdf), 1);
}

/// A two slide ODP written by hand, so the test needs no fixture file.
fn write_odp(path: &Path) {
    use zip::write::SimpleFileOptions;
    let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    let stored = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip.start_file("mimetype", stored).unwrap();
    zip.write_all(b"application/vnd.oasis.opendocument.presentation")
        .unwrap();
    zip.start_file("META-INF/manifest.xml", stored).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0" manifest:version="1.2">
 <manifest:file-entry manifest:full-path="/" manifest:media-type="application/vnd.oasis.opendocument.presentation"/>
 <manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>
</manifest:manifest>"#,
    )
    .unwrap();
    zip.start_file("content.xml", stored).unwrap();
    let mut content = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0" office:version="1.2"><office:body><office:presentation>"#,
    );
    for n in 1..=2 {
        content.push_str(&format!(
            r#"<draw:page draw:name="s{n}"><draw:frame svg:x="2cm" svg:y="2cm" svg:width="10cm" svg:height="3cm"><draw:text-box><text:p>Slide {n}</text:p></draw:text-box></draw:frame></draw:page>"#
        ));
    }
    content.push_str("</office:presentation></office:body></office:document-content>");
    zip.write_all(content.as_bytes()).unwrap();
    zip.finish().unwrap();
}

#[test]
fn office_odp_to_pptx_then_pdf_keeps_two_slides() {
    let dir = tempfile::tempdir().unwrap();
    let Some(engine) = real_engine(dir.path()) else {
        return;
    };
    let odp = dir.path().join("deck.odp");
    write_odp(&odp);
    let pptx = dir.path().join("deck.pptx");
    run(
        &engine,
        &task("convert-slides", &odp, &pptx, ""),
        &CancelToken::new(),
    )
    .0
    .unwrap();
    assert!(
        zip_entry(&pptx, "ppt/presentation.xml").is_some(),
        "not a PPTX"
    );
    let pdf = dir.path().join("deck.pdf");
    run(
        &engine,
        &task("to-pdf", &pptx, &pdf, ""),
        &CancelToken::new(),
    )
    .0
    .unwrap();
    assert_eq!(pdf_pages(&pdf), 2);
}

#[test]
fn office_rejects_wrong_family_bad_target_and_unknown_op() {
    let dir = tempfile::tempdir().unwrap();
    let engine = OfficeEngine::with_program("/bin/false", vec![]).temp_root(dir.path());
    let docx = dir.path().join("a.docx");
    std::fs::write(&docx, b"x").unwrap();
    let cancel = CancelToken::new();

    let r = run(
        &engine,
        &task("convert-sheet", &docx, &dir.path().join("a.xlsx"), ""),
        &cancel,
    )
    .0;
    assert!(matches!(r, Err(EngineError::Input { .. })), "{r:?}");
    let r = run(
        &engine,
        &task("convert-doc", &docx, &dir.path().join("a.pdf"), ""),
        &cancel,
    )
    .0;
    assert!(matches!(r, Err(EngineError::BadParam { .. })), "{r:?}");
    let r = run(
        &engine,
        &task("to-pdf", &docx, &dir.path().join("a.odt"), ""),
        &cancel,
    )
    .0;
    assert!(matches!(r, Err(EngineError::BadParam { .. })), "{r:?}");
    let r = run(
        &engine,
        &task("explode", &docx, &dir.path().join("a.pdf"), ""),
        &cancel,
    )
    .0;
    assert!(matches!(r, Err(EngineError::UnsupportedOp(_))), "{r:?}");
    let r = run(
        &engine,
        &task("to-pdf", &docx, &dir.path().join("a.pdf"), "timeout_s = 0"),
        &cancel,
    )
    .0;
    assert!(matches!(r, Err(EngineError::BadParam { .. })), "{r:?}");
    let r = run(
        &engine,
        &task("to-pdf", &docx, &dir.path().join("a.pdf"), "timeout_s = -3"),
        &cancel,
    )
    .0;
    assert!(matches!(r, Err(EngineError::BadParam { .. })), "{r:?}");
    let mut two = task("to-pdf", &docx, &dir.path().join("a.pdf"), "");
    two.inputs.push(docx.clone());
    let r = run(&engine, &two, &cancel).0;
    assert!(matches!(r, Err(EngineError::BadParam { .. })), "{r:?}");
    // Validation happens before any work directory is created.
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn office_reports_failure_when_no_output_is_produced() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("tmp");
    std::fs::create_dir(&root).unwrap();
    // `true` exits 0 without writing anything, like LibreOffice on a load failure.
    let engine = OfficeEngine::with_program("/bin/true", vec![]).temp_root(&root);
    let txt = dir.path().join("a.txt");
    std::fs::write(&txt, "x").unwrap();
    let out = dir.path().join("a.pdf");
    let r = run(
        &engine,
        &task("to-pdf", &txt, &out, ""),
        &CancelToken::new(),
    )
    .0;
    assert!(matches!(r, Err(EngineError::Failed(_))), "{r:?}");
    assert!(!out.exists());
    assert!(is_empty_dir(&root));
}

#[cfg(unix)]
#[test]
fn office_missing_program_is_unavailable_error() {
    let dir = tempfile::tempdir().unwrap();
    let engine = OfficeEngine::with_program("/nonexistent/soffice", vec![]).temp_root(dir.path());
    let txt = dir.path().join("a.txt");
    std::fs::write(&txt, "x").unwrap();
    let r = run(
        &engine,
        &task("to-pdf", &txt, &dir.path().join("a.pdf"), ""),
        &CancelToken::new(),
    )
    .0;
    assert!(matches!(r, Err(EngineError::Unavailable(_))), "{r:?}");
    assert!(!engine.available());
    assert!(OfficeEngine::with_program("/bin/sh", vec![]).available());

    // A program that exists but cannot run is an I/O error, not "missing".
    let not_exec = dir.path().join("soffice");
    std::fs::write(&not_exec, "not a program").unwrap();
    let engine = OfficeEngine::with_program(&not_exec, vec![]).temp_root(dir.path());
    let r = run(
        &engine,
        &task("to-pdf", &txt, &dir.path().join("a.pdf"), ""),
        &CancelToken::new(),
    )
    .0;
    assert!(matches!(r, Err(EngineError::Io(_))), "{r:?}");
}

/// A fake LibreOffice: a shell script that starts a grandchild `sleep`,
/// records its pid and waits, so only a process group kill stops both.
#[cfg(unix)]
fn fake_engine(dir: &Path, root: &Path) -> (OfficeEngine, PathBuf) {
    let script = dir.join("fake-soffice.sh");
    let pidfile = dir.join("sleep.pid");
    std::fs::write(&script, "sleep 60 &\necho $! > \"$1\"\nwait\n").unwrap();
    let engine = OfficeEngine::with_program(
        "/bin/sh",
        vec![OsString::from(&script), OsString::from(&pidfile)],
    )
    .temp_root(root);
    (engine, pidfile)
}

#[cfg(unix)]
fn read_pid(pidfile: &Path) -> String {
    let end = Instant::now() + Duration::from_secs(3);
    loop {
        if let Ok(pid) = std::fs::read_to_string(pidfile)
            && !pid.trim().is_empty()
        {
            return pid.trim().to_string();
        }
        assert!(Instant::now() < end, "fake soffice never started");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[cfg(unix)]
#[test]
fn office_timeout_kills_the_process_group() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("tmp");
    std::fs::create_dir(&root).unwrap();
    let (engine, pidfile) = fake_engine(dir.path(), &root);
    let txt = dir.path().join("a.txt");
    std::fs::write(&txt, "x").unwrap();
    let started = Instant::now();
    let r = run(
        &engine,
        &task("to-pdf", &txt, &dir.path().join("a.pdf"), "timeout_s = 0.5"),
        &CancelToken::new(),
    )
    .0;
    let elapsed = started.elapsed();
    match r {
        Err(EngineError::Failed(msg)) => assert!(msg.contains("timed out"), "{msg}"),
        other => panic!("expected a timeout, got {other:?}"),
    }
    assert!(elapsed < Duration::from_secs(5), "took {elapsed:?}");
    assert!(
        elapsed >= Duration::from_millis(450),
        "returned before the timeout: {elapsed:?}"
    );
    assert!(
        wait_gone(&read_pid(&pidfile)),
        "grandchild survived the timeout"
    );
    assert!(is_empty_dir(&root));
}

#[cfg(unix)]
#[test]
fn office_cancel_kills_the_process_group() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("tmp");
    std::fs::create_dir(&root).unwrap();
    let (engine, pidfile) = fake_engine(dir.path(), &root);
    let txt = dir.path().join("a.txt");
    std::fs::write(&txt, "x").unwrap();
    let cancel = CancelToken::new();
    let trigger = cancel.clone();
    let canceller = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(400));
        trigger.cancel();
    });
    let started = Instant::now();
    let r = run(
        &engine,
        &task("to-pdf", &txt, &dir.path().join("a.pdf"), ""),
        &cancel,
    )
    .0;
    canceller.join().unwrap();
    assert!(matches!(r, Err(EngineError::Cancelled)), "{r:?}");
    assert!(started.elapsed() < Duration::from_secs(3));
    assert!(
        wait_gone(&read_pid(&pidfile)),
        "grandchild survived the cancel"
    );
    assert!(is_empty_dir(&root));
}

#[test]
fn office_cancel_stops_real_libreoffice_and_cleans_up() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("cancel-root");
    std::fs::create_dir(&root).unwrap();
    let Some(engine) = real_engine(&root) else {
        return;
    };
    let rtf = dir.path().join("a.rtf");
    std::fs::write(&rtf, TWO_PAGE_RTF).unwrap();
    let cancel = CancelToken::new();
    let trigger = cancel.clone();
    let canceller = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(700));
        trigger.cancel();
    });
    let r = run(
        &engine,
        &task("to-pdf", &rtf, &dir.path().join("a.pdf"), ""),
        &cancel,
    )
    .0;
    canceller.join().unwrap();
    assert!(matches!(r, Err(EngineError::Cancelled)), "{r:?}");
    let left = wait_no_process(&root.to_string_lossy());
    assert!(left.is_empty(), "LibreOffice processes survived: {left:?}");
    assert!(is_empty_dir(&root));
}

#[cfg(unix)]
#[test]
fn office_already_cancelled_starts_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let (engine, pidfile) = fake_engine(dir.path(), dir.path());
    let txt = dir.path().join("a.txt");
    std::fs::write(&txt, "x").unwrap();
    let cancel = CancelToken::new();
    cancel.cancel();
    let r = run(
        &engine,
        &task("to-pdf", &txt, &dir.path().join("a.pdf"), ""),
        &cancel,
    )
    .0;
    assert!(matches!(r, Err(EngineError::Cancelled)), "{r:?}");
    assert!(!pidfile.exists());
}

#[test]
fn office_capabilities_list_the_four_ops_for_documents() {
    let engine = OfficeEngine::default();
    assert_eq!(engine.id(), "office");
    let ops: Vec<_> = engine
        .capabilities()
        .into_iter()
        .map(|(op, _)| op)
        .collect();
    assert_eq!(
        ops,
        ["to-pdf", "convert-doc", "convert-sheet", "convert-slides"]
    );
    for (_, kinds) in engine.capabilities() {
        assert_eq!(kinds, [sbarhive_core::MediaKind::Document]);
    }
}
