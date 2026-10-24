//! S-029 (backlog/S-029-review-cleanup.md): the structural checks of the
//! cleanup, one test per criterion, each the story's grep with its expected
//! count. Paths are read from the workspace root. Needles that the scan of
//! a test could find in this file are split with `concat!`, so the file does
//! not match itself. Unix only: it reads the source tree and runs bash,
//! neither of which the Windows test ISO carries.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    let path = root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The text above the first `#[cfg(test)]` line.
fn above_test(text: &str) -> String {
    text.lines()
        .take_while(|l| l.trim() != "#[cfg(test)]")
        .map(|l| format!("{l}\n"))
        .collect()
}

/// From the first line containing `header` to the next line that is `}`.
fn body_of(text: &str, header: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for l in text.lines() {
        if !inside && l.contains(header) {
            inside = true;
        }
        if inside {
            out.push_str(l);
            out.push('\n');
            if l == "}" {
                break;
            }
        }
    }
    assert!(inside, "no line with {header:?}");
    out
}

fn lines_with(text: &str, word: &str) -> usize {
    text.lines().filter(|l| l.contains(word)).count()
}

/// Every file under `dir`, recursively.
fn files_under(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            files_under(&p, out);
        } else {
            out.push(p);
        }
    }
}

/// `grep -rn word dir`: the matching lines, as `path: line`.
fn grep_r(rel_dir: &str, word: &str) -> Vec<String> {
    let mut files = Vec::new();
    files_under(&root().join(rel_dir), &mut files);
    let mut hits = Vec::new();
    for f in files {
        let Ok(bytes) = std::fs::read(&f) else {
            continue;
        };
        for l in String::from_utf8_lossy(&bytes).lines() {
            if l.contains(word) {
                hits.push(format!("{}: {l}", f.display()));
            }
        }
    }
    hits
}

/// The poison-ignoring lock of the AC5 and AC9 grep:
/// `unwrap_or_else\((PoisonError::into_inner|\|e\| e\.into_inner\(\))\)`.
fn poison_locks(rel_dir: &str) -> Vec<String> {
    let mut hits = grep_r(rel_dir, "unwrap_or_else(PoisonError::into_inner)");
    hits.extend(grep_r(rel_dir, "unwrap_or_else(|e| e.into_inner())"));
    hits
}

#[test]
fn s029_ac1_one_utf16_helper_in_core_win() {
    let lib = read("crates/core/src/lib.rs");
    let lines: Vec<&str> = lib.lines().map(str::trim).collect();
    let at = lines
        .iter()
        .position(|l| *l == "pub mod win;")
        .expect("lib.rs declares `pub mod win;`");
    let attrs = &lines[at.saturating_sub(2)..at];
    assert!(
        attrs.contains(&"#[cfg(windows)]") && attrs.contains(&"#[doc(hidden)]"),
        "`pub mod win` is #[cfg(windows)] #[doc(hidden)]: {attrs:?}"
    );

    let win = read("crates/core/src/win.rs");
    assert_eq!(
        lines_with(
            &win,
            concat!("pub fn ", "wide(s: impl AsRef<OsStr>) -> Vec<u16>")
        ),
        1,
        "win.rs has the wide helper"
    );
    assert!(win.contains("encode_wide"), "wide uses encode_wide");
    assert_eq!(
        lines_with(
            &win,
            "pub unsafe fn from_wide_ptr(p: *const u16) -> OsString"
        ),
        1,
        "win.rs has from_wide_ptr"
    );
    assert!(
        win.contains("# Safety"),
        "from_wide_ptr has a # Safety section"
    );
    assert!(
        win.contains("unsafe {"),
        "explicit unsafe blocks in the body"
    );

    let needle = concat!("fn ", "wide");
    let hits = grep_r("crates", needle);
    assert_eq!(hits.len(), 1, "one {needle} in crates: {hits:#?}");

    for f in [
        "crates/core/src/paths/windows.rs",
        "crates/core/src/transport/windows.rs",
    ] {
        assert_eq!(lines_with(&read(f), ".add("), 0, "no pointer walk in {f}");
    }
    let sid = body_of(
        &read("crates/core/src/transport/windows.rs"),
        "fn token_sid(",
    );
    assert!(
        sid.contains("to_string_lossy().into_owned()"),
        "token_sid turns the OsString into text: {sid}"
    );
    assert!(
        read("crates/engines/src/locate/windows.rs").contains("fn until_nul"),
        "until_nul stays in locate/windows.rs"
    );
    assert!(
        read(".cargo/mutants.toml").contains("\"crates/core/src/win.rs\","),
        "crates/core/src/win.rs is in exclude_globs"
    );
}

#[test]
fn s029_ac2_non_unix_uses_std_owned_handle() {
    let f = read("crates/engines/src/office/non_unix.rs");
    for word in ["struct Handle", "impl Drop", "CloseHandle"] {
        assert_eq!(lines_with(&f, word), 0, "non_unix.rs has no {word}");
    }
    assert!(
        f.contains("OwnedHandle"),
        "the job, snapshot and thread handles are OwnedHandle"
    );
}

#[test]
fn s029_ac3_bind_matches_on_create_instance() {
    let f = read("crates/core/src/transport/windows.rs");
    assert_eq!(lines_with(&f, "(BindOutcome::"), 0, "no tuple match");
    assert_eq!(lines_with(&f, "from_raw_os_error"), 0, "os_error only");
    let bind = body_of(&f, "pub fn bind(");
    assert!(
        bind.contains("bind_result(Some("),
        "Err(code) goes through bind_result(Some(code)): {bind}"
    );
    assert!(
        bind.contains("os_error("),
        "other arms give os_error: {bind}"
    );
    let t = read("crates/core/src/transport.rs");
    assert_eq!(
        t.lines()
            .filter(|l| l.starts_with("fn os_error(code: u32) -> io::Error"))
            .count(),
        1,
        "os_error keeps its signature and visibility"
    );
    assert_eq!(
        lines_with(
            &t,
            "pub fn bind_result(first_instance_error: Option<u32>) -> BindOutcome"
        ),
        1,
        "bind_result's signature is pinned"
    );
}

#[test]
fn s029_ac4_deadline_flag_is_an_atomic_bool() {
    let f = read("crates/core/src/transport/deadline.rs");
    assert_eq!(lines_with(&f, "Mutex"), 0, "deadline.rs has no Mutex");
    assert!(f.contains("AtomicBool"), "the one-shot flag is AtomicBool");
    assert!(f.contains("Arc"), "shared through Arc");
    let swap =
        |l: &&str| l.contains(".swap(true, SeqCst)") || l.contains(".swap(true, Ordering::SeqCst)");
    let all = f.lines().filter(swap).count();
    let negated = f
        .lines()
        .filter(swap)
        .filter(|l| {
            // The receiver of the swap is negated: `!flag.swap(..)`.
            let before = &l[..l.find(".swap(true, ").unwrap()];
            before
                .trim_end_matches(|c: char| c.is_ascii_alphanumeric() || c == '_')
                .ends_with('!')
        })
        .count();
    assert_eq!(all, 2, "one swap in the watcher, one on the read side");
    assert_eq!(negated, 1, "the watcher cancels only when !flag.swap(..)");
    assert!(
        f.lines().filter(swap).any(|l| l.contains("fired")),
        "the read side takes fired from the swap"
    );
}

#[test]
fn s029_ac5_one_poison_ignoring_lock_in_core() {
    let sync = read("crates/core/src/sync.rs");
    assert_eq!(
        sync.lines()
            .filter(|l| l.contains("pub(crate) fn lock<T>(")
                && l.contains("&Mutex<T>")
                && l.contains("MutexGuard<"))
            .count(),
        1,
        "sync.rs has the lock helper"
    );
    assert_eq!(
        lines_with(&read("crates/core/src/lib.rs"), "mod sync;"),
        1,
        "lib.rs declares sync"
    );
    assert_eq!(
        lines_with(&read("crates/core/src/queue_impl.rs"), "fn lock"),
        0,
        "queue_impl.rs loses its copy"
    );
    let hits = poison_locks("crates/core/src");
    assert_eq!(hits.len(), 1, "one poison-ignoring lock in core: {hits:#?}");
}

#[test]
fn s029_ac6_one_missing_program_helper() {
    let office = above_test(&read("crates/engines/src/office.rs"));
    let pdf = above_test(&read("crates/engines/src/pdf.rs"));
    // rustfmt may wrap the signature: compare with whitespace collapsed.
    let flat = office
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace("( ", "(")
        .replace(", )", ")");
    assert_eq!(
        flat.matches(
            "pub(crate) fn missing_on_not_found(e: EngineError, name: &str, url: &str, program: &Path) -> EngineError"
        )
        .count(),
        1,
        "office.rs defines missing_on_not_found"
    );
    assert!(
        lines_with(&office, "missing_on_not_found(") >= 2,
        "office.rs uses it"
    );
    assert!(
        lines_with(&pdf, "missing_on_not_found(") >= 1,
        "pdf.rs uses it"
    );
    assert_eq!(
        lines_with(&office, "ErrorKind::NotFound") + lines_with(&pdf, "ErrorKind::NotFound"),
        1,
        "one NotFound test in office.rs and pdf.rs together"
    );
}

#[test]
fn s029_ac7_form_state_keeps_the_portal_switch() {
    let f = above_test(&read("crates/ui/src/settings.rs"));
    let form = body_of(&f, "struct FormState {");
    assert_eq!(
        lines_with(&form, "portal_shortcut: bool"),
        1,
        "FormState has portal_shortcut: {form}"
    );
    assert_eq!(lines_with(&f, "portal_shortcut: true"), 0);
    assert_eq!(lines_with(&f, "get_portal_shortcut"), 1);
    assert_eq!(lines_with(&f, "set_portal_shortcut"), 1);
    for (header, word) in [
        ("fn from_config(", "portal_shortcut"),
        ("fn to_config(", "portal_shortcut"),
        ("fn state(", "portal_shortcut"),
        ("fn set_state(", "portal_shortcut"),
    ] {
        let mut body = String::new();
        let mut inside = false;
        for l in f.lines() {
            if !inside && l.contains(header) {
                inside = true;
            }
            if inside {
                body.push_str(l);
                body.push('\n');
                if l == "    }" {
                    break;
                }
            }
        }
        assert!(body.contains(word), "{header} maps {word}: {body}");
    }
}

#[test]
fn s029_ac8_reload_named_fields_and_one_token_step() {
    let full = read("crates/daemon/src/ui.rs");
    let f = above_test(&full);
    let reload = body_of(&f, "pub fn reload(");
    for v in ["before", "after", "portal"] {
        for i in ["0", "1"] {
            let pat = format!("{v}.{i}");
            assert_eq!(
                lines_with(&reload, &pat),
                0,
                "reload has no {pat}: {reload}"
            );
        }
    }
    assert_eq!(
        f.lines()
            .filter(
                |l| l.contains("fn token_step(step: TokenStep") && l.contains("-> Option<String>")
            )
            .count(),
        1,
        "ui.rs has fn token_step(step: TokenStep) -> Option<String>"
    );
    assert_eq!(
        lines_with(&full, "HOLD_TOKEN.with"),
        1,
        "one HOLD_TOKEN.with"
    );
    assert_eq!(lines_with(&full, "kept_token("), 1, "one kept_token( call");
    let step = body_of(&f, "fn token_step(");
    assert!(
        step.contains("HOLD_TOKEN.with") && step.contains("kept_token("),
        "both are inside token_step: {step}"
    );
    let hold = body_of(&f, "pub fn hold_signal(");
    let starts = hold
        .lines()
        .filter(|l| l.contains("(0, 1)") || l.contains("before == 0"))
        .count();
    assert_eq!(starts, 1, "hold_signal tests the start once: {hold}");
    assert!(
        read(".cargo/mutants.toml").contains("\"crates/daemon/src/ui.rs:.*in reload$\""),
        "exclude_re has the reload entry"
    );
}

#[test]
fn s029_ac9_portal_client_stops_with_abortable() {
    let f = read("crates/daemon/src/portal_bus.rs");
    for word in ["struct Stop", "AtomicWaker", "poll_fn", "select("] {
        assert_eq!(lines_with(&f, word), 0, "portal_bus.rs has no {word}");
    }
    assert!(f.contains("AbortHandle"), "uses AbortHandle");
    assert!(f.contains("Abortable"), "uses Abortable");
    let client = body_of(&f, "fn client(");
    assert!(
        client.contains("lock(") && !client.contains("unwrap_or_else"),
        "client() calls state::lock: {client}"
    );
    let state = read("crates/daemon/src/state.rs");
    assert_eq!(
        state.lines().filter(|l| l.contains("fn lock<T>(")).count(),
        1,
        "state::lock is generic"
    );
    let hits = poison_locks("crates/daemon/src");
    assert_eq!(
        hits.len(),
        1,
        "one poison-ignoring lock in the daemon: {hits:#?}"
    );
}

/// Non-comment lines of a shell script.
fn code_lines(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .map(str::to_string)
        .collect()
}

/// Names of the functions a shell script defines (`name() {`).
fn shell_functions(text: &str) -> Vec<String> {
    code_lines(text)
        .iter()
        .filter_map(|l| {
            let t = l.trim();
            let (name, rest) = t.split_once("()")?;
            let name = name.trim().trim_start_matches("function ").trim();
            let ok =
                !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
            (ok && rest.trim_start().starts_with('{')).then(|| name.to_string())
        })
        .collect()
}

fn sources_scope(text: &str) -> bool {
    code_lines(text).iter().any(|l| {
        let t = l.trim_start();
        (t.starts_with("source ") || t.starts_with(". ")) && t.contains("scope.sh")
    })
}

#[test]
fn s029_ac11_one_sourced_scope_script() {
    let dir = root().join("scripts/windows");
    let scope_path = dir.join("scope.sh");
    let scope = read("scripts/windows/scope.sh");
    assert!(
        scope.lines().any(|l| l.trim() == "# shellcheck shell=bash"),
        "scope.sh has # shellcheck shell=bash"
    );
    for dash in ['\u{2014}', '\u{2013}'] {
        assert!(!scope.contains(dash), "scope.sh has no long dash");
    }
    let n = Command::new("bash")
        .arg("-n")
        .arg(&scope_path)
        .status()
        .unwrap();
    assert!(n.success(), "bash -n scope.sh");

    // systemd-run and the default target dir: once each in non-comment
    // lines of scripts/windows/*.sh, both in scope.sh.
    let mut scripts: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "sh"))
        .collect();
    scripts.sort();
    for word in ["systemd-run", "sbarhive-win-target"] {
        let mut hits = Vec::new();
        for s in &scripts {
            let text = std::fs::read_to_string(s).unwrap();
            for l in code_lines(&text) {
                if l.contains(word) {
                    hits.push(format!("{}: {l}", s.display()));
                }
            }
        }
        assert_eq!(hits.len(), 1, "{word} once in scripts/windows: {hits:#?}");
        assert!(hits[0].contains("scope.sh: "), "{word} is in scope.sh");
    }

    // Sourcing has no side effect: nothing printed, no scope entered, the
    // shell goes on, and exactly one function is defined.
    let fake = tempfile::tempdir().unwrap();
    let marker = fake.path().join("ran");
    let bin = fake.path().join("systemd-run");
    std::fs::write(&bin, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path = format!(
        "{}:{}",
        fake.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new("bash")
        .arg("-c")
        .arg("before=$(declare -F | wc -l); source \"$1\"; echo after; echo $(( $(declare -F | wc -l) - before ))")
        .arg("_")
        .arg(&scope_path)
        .env("PATH", &path)
        .env_remove("CI")
        .env_remove("SBARHIVE_WIN_SCOPE")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "sourcing scope.sh: {out:?}");
    assert_eq!(
        stdout, "after\n1\n",
        "nothing printed, one function defined"
    );
    assert!(!marker.exists(), "sourcing scope.sh enters no scope");

    let funcs = shell_functions(&scope);
    assert_eq!(funcs.len(), 1, "scope.sh defines one function: {funcs:?}");
    let reexec = &funcs[0];
    for s in ["build.sh", "check.sh"] {
        let text = read(&format!("scripts/windows/{s}"));
        assert!(sources_scope(&text), "{s} sources scope.sh");
        let lines: Vec<&str> = text.lines().collect();
        let at = lines
            .iter()
            .position(|l| {
                let t = l.trim_start();
                (t.starts_with("source ") || t.starts_with(". ")) && t.contains("scope.sh")
            })
            .unwrap();
        assert!(
            at > 0 && lines[at - 1].trim() == "# shellcheck source=/dev/null",
            "{s}: the source line has # shellcheck source=/dev/null above it"
        );
        assert!(
            code_lines(&text)
                .iter()
                .any(|l| l.contains(reexec.as_str()) && l.contains("\"$@\"")),
            "{s} calls {reexec} with \"$@\""
        );
    }
    let iso = read("scripts/windows/make-iso.sh");
    assert!(
        sources_scope(&iso),
        "make-iso.sh takes the default from scope.sh"
    );
    assert!(
        !code_lines(&iso).iter().any(|l| l.contains(reexec.as_str())),
        "make-iso.sh never enters a scope"
    );

    if Command::new("shellcheck").arg("--version").output().is_ok() {
        for s in &scripts {
            let out = Command::new("shellcheck").arg(s).output().unwrap();
            assert!(
                out.status.success(),
                "shellcheck {}: {}",
                s.display(),
                String::from_utf8_lossy(&out.stdout)
            );
        }
    } else {
        eprintln!("shellcheck not found: the shellcheck part is skipped");
    }
    let lib = read("tests/windows/lib.sh");
    assert!(
        lib.contains("sbarhive-win-target"),
        "tests/windows/lib.sh keeps its own default"
    );
}
