//! S-080 (backlog/S-080-windows-doctor-engine-discovery.md), AC1 and AC4 at
//! source level: the CLI keeps no bare `PATH` probe for the four engine
//! tools (`ffmpeg`, `ffprobe`, `soffice`, `gs`), resolves them through
//! `kawaro_engines::locate::find` or a direct wrapper of it, and keeps the
//! bare probe for the plain `PATH` tools (`wl-paste`, `xclip`, `xdg-open`,
//! `kawarod`) only.
//!
//! Contract (tester): a direct wrapper of `locate::find` is a function that
//! calls `locate::find` with the tool's `Program`, so the tool to `Program`
//! mapping shows in the CLI as the four `Program::` variants, and the table
//! that feeds `on_path` holds the plain tools only. The behavioural proofs
//! that the resolution is the engines' lookup live in
//! `crates/cli/tests/s080_doctor_engine_tools.rs`.
#![cfg(unix)]

use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    let path = root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} must exist: {e}", path.display()))
}

/// Every `.rs` file under `crates/cli/src`, joined. File names are kept as
/// comments would be, since only whole-source needles are matched.
fn cli_sources() -> String {
    fn walk(dir: &std::path::Path, out: &mut String) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push_str(&std::fs::read_to_string(&path).unwrap());
            }
        }
    }
    let mut out = String::new();
    walk(&root().join("crates/cli/src"), &mut out);
    out
}

/// The code part of `crates/cli/src/main.rs`, above the test module.
fn main_source() -> String {
    let s = read("crates/cli/src/main.rs");
    s.split("\n#[cfg(test)]").next().unwrap().to_string()
}

/// Every top level `[...]` span of `text`, skipping comments and strings.
fn array_literals(text: &str) -> Vec<String> {
    let ch: Vec<char> = text.chars().collect();
    let mut spans = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    let mut i = 0;
    while i < ch.len() {
        let c = ch[i];
        if c == '/' && ch.get(i + 1) == Some(&'/') {
            while i < ch.len() && ch[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && ch.get(i + 1) == Some(&'*') {
            i += 2;
            let mut depth = 1;
            while i < ch.len() && depth > 0 {
                if ch[i] == '/' && ch.get(i + 1) == Some(&'*') {
                    depth += 1;
                    i += 2;
                } else if ch[i] == '*' && ch.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
        } else if c == 'r' && ch.get(i + 1).is_some_and(|n| *n == '"' || *n == '#') {
            let mut j = i + 1;
            let mut hashes = 0;
            while ch.get(j) == Some(&'#') {
                hashes += 1;
                j += 1;
            }
            if ch.get(j) == Some(&'"') {
                j += 1;
                while j < ch.len() {
                    if ch[j] == '"' {
                        let mut seen = 0;
                        while ch.get(j + 1 + seen) == Some(&'#') && seen < hashes {
                            seen += 1;
                        }
                        if seen == hashes {
                            j += 1 + hashes;
                            break;
                        }
                    }
                    j += 1;
                }
                i = j;
            } else {
                i += 1;
            }
        } else if c == '"' {
            i += 1;
            while i < ch.len() {
                if ch[i] == '\\' {
                    i += 2;
                } else if ch[i] == '"' {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
        } else if c == '[' {
            stack.push(i);
            i += 1;
        } else if c == ']' {
            let closed = stack.pop();
            if let Some(start) = closed
                && stack.is_empty()
            {
                spans.push(ch[start..=i].iter().collect());
            }
            i += 1;
        } else {
            i += 1;
        }
    }
    spans
}

const ENGINE_TOOLS: [&str; 4] = ["ffmpeg", "ffprobe", "soffice", "gs"];
const PLAIN_TOOLS: [&str; 4] = ["wl-paste", "xclip", "xdg-open", "kawarod"];

#[test]
fn ac1_no_bare_path_probe_remains_for_the_engine_tools() {
    let src = cli_sources();
    for name in ENGINE_TOOLS {
        assert!(
            !src.contains(&format!("on_path(\"{name}\")")),
            "on_path(\"{name}\") is a bare PATH probe for an engine tool"
        );
    }
    // The table that feeds `on_path` must hold the plain tools only: the
    // four engine tools resolve through the engines' lookup instead.
    let arrays = array_literals(&main_source());
    for array in &arrays {
        if array.contains("\"wl-paste\"") {
            for name in ENGINE_TOOLS {
                assert!(
                    !array.contains(&format!("\"{name}\"")),
                    "the bare PATH probe table still lists {name}:\n{array}"
                );
            }
        }
    }
}

#[test]
fn ac1_engine_tools_resolve_through_locate_find_or_a_wrapper() {
    let src = cli_sources();
    assert!(
        src.contains("locate::find") || src.contains("kawaro_engines::locate"),
        "the engine tool resolution must call `locate::find` or a direct \
         wrapper of it"
    );
    for program in [
        "Program::Ffmpeg",
        "Program::Ffprobe",
        "Program::Office",
        "Program::Ghostscript",
    ] {
        assert!(
            src.contains(program),
            "the doctor tool rows must map their tool to {program} and \
             resolve it through the shared lookup"
        );
    }
}

#[test]
fn ac4_on_path_stays_for_the_plain_tools() {
    let main = main_source();
    let src = cli_sources();
    assert!(main.contains("fn on_path"), "the plain PATH probe stays");
    assert!(src.contains("on_path("), "on_path stays in use");
    let arrays = array_literals(&main);
    let plain: Vec<&String> = arrays
        .iter()
        .filter(|a| a.contains("\"wl-paste\""))
        .collect();
    assert!(!plain.is_empty(), "the plain tools table exists");
    for array in &plain {
        for name in PLAIN_TOOLS {
            assert!(
                array.contains(&format!("\"{name}\"")),
                "the plain tools table must keep {name}:\n{array}"
            );
        }
    }
}
