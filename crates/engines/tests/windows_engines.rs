//! S-043b (backlog/S-043-windows-engines.md): engine messages when a program
//! is missing (AC11) and the no-console flag on every ffmpeg and ffprobe
//! command (AC13). These use only the public engine API and the source text,
//! so they run on Linux.

use kawaro_core::{CancelToken, Engine, EngineError, Task};
use kawaro_engines::ffmpeg::FfmpegEngine;
use kawaro_engines::office::OfficeEngine;
use kawaro_engines::pdf::GhostscriptEngine;
use std::path::{Path, PathBuf};

fn task(op: &str, input: &Path, output: PathBuf) -> Task {
    Task {
        action_id: format!("test.{op}"),
        op: op.into(),
        params: toml::from_str("").unwrap(),
        inputs: vec![input.to_path_buf()],
        output,
    }
}

fn unavailable(r: Result<(), EngineError>) -> String {
    match r {
        Err(EngineError::Unavailable(m)) => m,
        other => panic!("expected Unavailable, got {other:?}"),
    }
}

#[test]
fn ac11_office_missing_program_names_libreoffice_and_link() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.txt");
    std::fs::write(&input, "x").unwrap();
    let engine = OfficeEngine::with_program("/nonexistent/soffice", vec![]).temp_root(dir.path());
    let m = unavailable(engine.run(
        &task("to-pdf", &input, dir.path().join("a.pdf")),
        &|_| {},
        &CancelToken::new(),
    ));
    assert!(m.contains("LibreOffice"), "{m}");
    assert!(m.contains("https://www.libreoffice.org/download/"), "{m}");
}

#[test]
fn ac11_ghostscript_missing_program_names_it_and_link() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("a.pdf");
    std::fs::write(&input, "%PDF-1.4\n").unwrap();
    let engine = GhostscriptEngine::with_program("/nonexistent/gs").temp_root(dir.path());
    let m = unavailable(engine.run(
        &task("compress", &input, dir.path().join("b.pdf")),
        &|_| {},
        &CancelToken::new(),
    ));
    assert!(m.contains("Ghostscript"), "{m}");
    assert!(m.contains("https://ghostscript.com/releases/"), "{m}");
}

#[test]
fn ac11_ffmpeg_missing_programs_names_ffmpeg_and_link() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("in.mp4");
    std::fs::write(&input, "x").unwrap();
    let engine = FfmpegEngine::with_binaries("/nonexistent/ffmpeg", "/nonexistent/ffprobe");
    let m = unavailable(engine.run(
        &task("convert-video", &input, dir.path().join("o.mp4")),
        &|_| {},
        &CancelToken::new(),
    ));
    assert!(m.to_ascii_lowercase().contains("ffmpeg"), "{m}");
    assert!(m.contains("https://ffmpeg.org/download.html"), "{m}");
}

/// Text of `fn` items: from the line declaring `fn` to the closing brace at
/// the same indentation.
fn enclosing_fn(lines: &[&str], at: usize) -> String {
    let start = (0..=at)
        .rev()
        .find(|&i| {
            let t = lines[i].trim_start();
            t.starts_with("fn ") || t.starts_with("pub fn ") || t.starts_with("pub(crate) fn ")
        })
        .unwrap_or_else(|| panic!("no fn encloses line {}", at + 1));
    let indent = lines[start].len() - lines[start].trim_start().len();
    let close = format!("{}}}", " ".repeat(indent));
    let end = (start..lines.len())
        .find(|&i| lines[i].trim_end() == close)
        .unwrap_or_else(|| panic!("fn at line {} never closes", start + 1));
    // Attributes right above the fn belong to it too.
    let mut first = start;
    while first > 0 && lines[first - 1].trim_start().starts_with("#[") {
        first -= 1;
    }
    lines[first..=end].join("\n")
}

#[test]
fn ac13_every_ffmpeg_command_goes_through_one_no_window_function() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ffmpeg.rs");
    let source = std::fs::read_to_string(&path).unwrap();
    let code = source.split("\n#[cfg(test)]\nmod tests").next().unwrap();
    let lines: Vec<&str> = code.lines().collect();
    let sites: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.contains("Command::new("))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        sites.len(),
        1,
        "every ffmpeg and ffprobe command must be built in one function; \
         `Command::new(` at lines {:?}",
        sites.iter().map(|i| i + 1).collect::<Vec<_>>()
    );
    let body = enclosing_fn(&lines, sites[0]);
    assert!(body.contains("cfg(windows)"), "no cfg(windows) in:\n{body}");
    assert!(
        body.contains("creation_flags"),
        "no creation_flags in:\n{body}"
    );
    assert!(
        body.contains("CREATE_NO_WINDOW"),
        "no CREATE_NO_WINDOW in:\n{body}"
    );
    assert!(
        !code.contains("const CREATE_NO_WINDOW"),
        "CREATE_NO_WINDOW must come from office::tree, not a copy in ffmpeg.rs"
    );
}
