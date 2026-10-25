//! S-067 (backlog/S-067-review-fixes-update-drop-notify.md): text tests over
//! the daemon sources for AC5 (the Windows probe moved out of server.rs),
//! AC6 (one open folder path, one log line format) and AC8 (`LOG_PREFIX` is
//! private). Test code (`*tests.rs` files, `#[cfg(test)]` items) and `//`
//! comment lines are left out.
//!
//! Contract (tester): `crates/daemon/src/notify.rs` has `fn open_folder(`
//! (any arguments), the one function that spawns the open folder command:
//! its body has exactly one `.spawn()`, waits on the child on a thread
//! (`thread::spawn` and `.wait()`), and writes a spawn failure with
//! `eprintln!`. `jobs.rs` and `finished` in `notify.rs` call it; both still
//! build the command with `open_folder_command(` (the S-049 test reads that).

use std::path::{Path, PathBuf};

fn src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `text` without its `#[cfg(test)]` items and `//` comment lines.
fn production(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find("#[cfg(test)]") {
        out.push_str(&rest[..i]);
        let after = &rest[i + "#[cfg(test)]".len()..];
        let Some(open) = after.find(['{', ';']) else {
            rest = "";
            break;
        };
        if after.as_bytes()[open] == b';' {
            rest = &after[open + 1..];
            continue;
        }
        let mut depth = 0usize;
        let mut end = after.len();
        for (j, c) in after[open..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = open + j + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        rest = &after[end..];
    }
    out.push_str(rest);
    out.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn read(rel: &str) -> String {
    production(&std::fs::read_to_string(src().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}")))
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// (path under src, production text) of every non test source file.
fn sources() -> Vec<(String, String)> {
    let mut files = Vec::new();
    rust_files(&src(), &mut files);
    files.sort();
    files
        .into_iter()
        .filter(|p| !p.to_string_lossy().ends_with("tests.rs"))
        .map(|p| {
            let rel = p.strip_prefix(src()).unwrap().display().to_string();
            (rel, production(&std::fs::read_to_string(&p).unwrap()))
        })
        .collect()
}

/// The body of the first `fn name(`, to the first closing brace in column 0.
fn body_of<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let start = text.find(&format!("fn {name}("))?;
    let rest = &text[start..];
    Some(&rest[..rest.find("\n}").unwrap_or(rest.len())])
}

// ---------------------------------------------------------------- AC5

#[test]
fn ac5_server_keeps_only_the_pure_rule_and_shell_reads_the_swap_setting() {
    let server = read("server.rs");
    assert!(
        server.contains("fn drop_refused("),
        "server.rs has drop_refused"
    );
    for word in ["GetAsyncKeyState", "shortcut_drop_allowed"] {
        assert!(!server.contains(word), "server.rs still has {word}");
    }
    let shell = read("shell.rs");
    for word in ["GetSystemMetrics", "SM_SWAPBUTTON", "primary_button_vk("] {
        assert!(shell.contains(word), "shell.rs uses {word}");
    }
    let found: Vec<String> = sources()
        .into_iter()
        .filter(|(_, t)| t.contains("shortcut_drop_allowed"))
        .map(|(rel, _)| rel)
        .collect();
    assert!(
        found.is_empty(),
        "shortcut_drop_allowed is still in {found:?}"
    );
}

#[test]
fn ac5_mutants_toml_drops_the_line_anchored_exclusion() {
    let toml = std::fs::read_to_string(repo().join(".cargo/mutants.toml")).unwrap();
    assert!(
        !toml.contains("server.rs:106"),
        "no line anchored exclusion"
    );
    assert!(
        !toml.contains("shortcut_drop_allowed"),
        "no shortcut_drop_allowed entry"
    );
}

// ---------------------------------------------------------------- AC6

#[test]
fn ac6_one_function_spawns_the_open_folder_command() {
    let notify = read("notify.rs");
    let body = body_of(&notify, "open_folder").expect("notify.rs has fn open_folder");
    assert_eq!(body.matches(".spawn()").count(), 1, "one spawn: {body}");
    assert!(
        body.contains("thread::spawn"),
        "the child is waited on a thread"
    );
    assert!(
        body.contains(".wait()"),
        "no zombie: the child is waited on"
    );
    assert!(
        body.contains("eprintln!"),
        "a spawn failure is one stderr line"
    );

    // Nowhere else is the command spawned right where it is built.
    let mut spawns = Vec::new();
    for (rel, text) in sources() {
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let builds = line.contains("open_folder_command(")
                && !line.contains("fn open_folder_command(")
                && !line.trim_start().starts_with("use ");
            let window = lines[i..(i + 3).min(lines.len())].join("\n");
            if builds && window.contains(".spawn()") {
                spawns.push(format!("{rel}:{}", i + 1));
            }
        }
    }
    assert!(
        spawns.is_empty(),
        "open folder spawned outside open_folder: {spawns:?}"
    );
    assert!(read("jobs.rs").contains("open_folder("), "jobs.rs uses it");
    let finished = body_of(&notify, "finished").expect("notify::finished");
    assert!(finished.contains("open_folder("), "finished uses it");
}

#[test]
fn ac6_the_notify_prefix_is_spelled_only_in_notify_logic() {
    let found: Vec<String> = sources()
        .into_iter()
        .filter(|(rel, _)| rel != "notify_logic.rs")
        .filter(|(_, t)| t.contains("kawarod: notify:"))
        .map(|(rel, _)| rel)
        .collect();
    assert!(found.is_empty(), "kawarod: notify: is spelled in {found:?}");
    let notify = read("notify.rs");
    let body = body_of(&notify, "notify_update_with").expect("notify_update_with");
    assert!(
        body.contains("log_line("),
        "notify_update_with uses log_line"
    );
}

// ---------------------------------------------------------------- AC8

#[test]
fn ac8_log_prefix_is_private() {
    let logic = read("notify_logic.rs");
    assert!(
        logic.contains("const LOG_PREFIX"),
        "the prefix stays a constant"
    );
    assert!(
        !logic.contains("pub const LOG_PREFIX"),
        "LOG_PREFIX is private"
    );
    assert!(
        !logic.contains("pub(crate) const LOG_PREFIX"),
        "LOG_PREFIX is private"
    );
}
