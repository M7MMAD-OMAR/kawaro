//! S-067 (backlog/S-067-review-fixes-update-drop-notify.md): text tests over
//! the CLI and daemon sources for the shared update code (AC1, AC2, AC3,
//! AC8). The files are read at test time; test code (`*_tests.rs` files and
//! `#[cfg(test)]` items) is left out, because a fake runner in a test is fine.
//!
//! Contract (tester): both `crates/cli/src/update_sys.rs` and
//! `crates/daemon/src/update_sys.rs` use `SystemRunner` and call
//! `probe_facts(`; `crates/cli/src/update.rs` calls `ready_on_disk(` and
//! `Channel::parse(`; `crates/core/src/config.rs` calls `Channel::parse(`;
//! `crates/cli/src/main.rs` names `INSTALL_PRIVILEGED_FLAG`.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// `text` without its `#[cfg(test)]` items (a module, a function or a `mod
/// x;` line).
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
    out
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

/// (path relative to the repo, production text) of every source file of the
/// CLI and the daemon that is not a test file.
fn cli_and_daemon_sources() -> Vec<(String, String)> {
    let mut files = Vec::new();
    for dir in ["crates/cli/src", "crates/daemon/src"] {
        rust_files(&root().join(dir), &mut files);
    }
    files.sort();
    files
        .into_iter()
        .filter(|p| !p.to_string_lossy().ends_with("tests.rs"))
        .map(|p| {
            let rel = p.strip_prefix(root()).unwrap().display().to_string();
            (rel, production(&std::fs::read_to_string(&p).unwrap()))
        })
        .collect()
}

fn offenders(word: &str) -> Vec<String> {
    cli_and_daemon_sources()
        .into_iter()
        .filter(|(_, text)| text.contains(word))
        .map(|(rel, _)| rel)
        .collect()
}

#[test]
fn ac1_impl_command_runner_is_in_core_only_and_both_update_sys_use_system_runner() {
    let found = offenders("impl CommandRunner");
    assert!(
        found.is_empty(),
        "impl CommandRunner outside tests in: {found:?}"
    );
    let apply = production(&read("crates/core/src/update/apply.rs"));
    assert!(
        apply.contains("impl CommandRunner for SystemRunner"),
        "core apply.rs implements CommandRunner for SystemRunner"
    );
    for rel in [
        "crates/cli/src/update_sys.rs",
        "crates/daemon/src/update_sys.rs",
    ] {
        let text = production(&read(rel));
        assert!(text.contains("SystemRunner"), "{rel} uses SystemRunner");
    }
}

#[test]
fn ac2_no_copied_probe_in_the_cli_or_the_daemon() {
    for word in [
        "fn repo_configured",
        "fn clean_exe_path",
        "\"/etc/yum.repos.d\"",
        "\" (deleted)\"",
    ] {
        let found = offenders(word);
        assert!(found.is_empty(), "{word} is still in: {found:?}");
    }
    for rel in [
        "crates/cli/src/update_sys.rs",
        "crates/daemon/src/update_sys.rs",
    ] {
        assert!(
            production(&read(rel)).contains("probe_facts("),
            "{rel} calls the one probe_facts"
        );
    }
}

#[test]
fn ac3_the_cli_status_uses_ready_on_disk_and_spells_no_part_suffix() {
    let mut files = Vec::new();
    rust_files(&root().join("crates/cli/src"), &mut files);
    let found: Vec<String> = files
        .iter()
        .filter(|p| std::fs::read_to_string(p).unwrap().contains("\".part\""))
        .map(|p| p.strip_prefix(root()).unwrap().display().to_string())
        .collect();
    assert!(
        found.is_empty(),
        "a \".part\" literal is still in: {found:?}"
    );
    assert!(
        production(&read("crates/cli/src/update.rs")).contains("ready_on_disk("),
        "kawaro update --status reads core's ready_on_disk"
    );
    let flow = production(&read("crates/core/src/update/flow.rs"));
    let body = &flow[flow
        .find("pub fn download_verified")
        .expect("download_verified")..];
    // The function ends at the first closing brace in column 0.
    let body = &body[..body.find("\n}\n").expect("end of download_verified")];
    assert!(
        body.contains("part_path("),
        "download_verified names its part file through part_path"
    );
}

#[test]
fn ac8_one_channel_parser_for_the_cli_and_config() {
    let update = production(&read("crates/cli/src/update.rs"));
    assert!(
        update.contains("Channel::parse("),
        "the CLI uses Channel::parse"
    );
    for word in ["Some(\"stable\")", "Some(\"beta\")"] {
        assert!(!update.contains(word), "the CLI still matches {word}");
    }
    let config = production(&read("crates/core/src/config.rs"));
    assert!(
        config.contains("Channel::parse("),
        "Config uses Channel::parse"
    );
    for word in ["\"stable\" => Some(Channel", "\"beta\" => Some(Channel"] {
        assert!(!config.contains(word), "config.rs still matches {word}");
    }
}

#[test]
fn ac8_install_privileged_is_one_core_constant() {
    let apply = production(&read("crates/core/src/update/apply.rs"));
    assert_eq!(
        apply.matches("\"--install-privileged\"").count(),
        1,
        "the literal appears once in apply.rs, in the constant"
    );
    assert!(apply.contains("pub const INSTALL_PRIVILEGED_FLAG"));
    let found = offenders("\"--install-privileged\"");
    assert!(
        found.is_empty(),
        "the CLI or daemon spells the flag itself in: {found:?}"
    );
    assert!(
        production(&read("crates/cli/src/main.rs")).contains("INSTALL_PRIVILEGED_FLAG"),
        "the CLI dispatch uses the core constant"
    );
}
