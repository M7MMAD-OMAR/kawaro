//! S-106 A1 (backlog/S-106-windows-small-defects.md): the foreground ladder.
//!
//! Hooks pinned here (std only, so this test includes the file by path, like
//! `reopen_logic.rs`): `crates/daemon/src/foreground_logic.rs` with
//! - `enum Step { Plain, Topmost, Flash }` (Debug, Clone, Copy, PartialEq);
//! - `fn foreground_ladder(try_step: impl FnMut(Step) -> bool) -> Step`: tries
//!   Plain, then Topmost, then Flash, stops at the first that returns true,
//!   returns the step that worked or `Flash` last, and calls `try_step` at
//!   most 3 times.
//!
//! `shell_open.rs` must use it and log `kawarod: foreground refused` only
//! when step 1 failed (source scan).

#[allow(dead_code)]
#[path = "../src/foreground_logic.rs"]
mod foreground_logic;

use foreground_logic::{Step, foreground_ladder};

fn run(script: &[bool]) -> (Step, Vec<Step>) {
    let mut seen = Vec::new();
    let mut i = 0;
    let got = foreground_ladder(|s| {
        seen.push(s);
        let ok = script.get(i).copied().unwrap_or(false);
        i += 1;
        ok
    });
    (got, seen)
}

#[test]
fn s106_a1_step_one_passes_after_one_call() {
    let (got, seen) = run(&[true]);
    assert_eq!(got, Step::Plain);
    assert_eq!(seen, [Step::Plain]);
}

#[test]
fn s106_a1_step_one_fails_step_two_passes_after_two_calls() {
    let (got, seen) = run(&[false, true]);
    assert_eq!(got, Step::Topmost);
    assert_eq!(seen, [Step::Plain, Step::Topmost]);
}

#[test]
fn s106_a1_both_fail_ends_with_flash_and_three_calls() {
    let (got, seen) = run(&[false, false]);
    assert_eq!(got, Step::Flash);
    assert_eq!(seen, [Step::Plain, Step::Topmost, Step::Flash]);
}

#[test]
fn s106_a1_calls_never_exceed_three_even_when_flash_fails() {
    let (got, seen) = run(&[false, false, false]);
    assert_eq!(got, Step::Flash);
    assert_eq!(seen.len(), 3);
}

#[test]
fn s106_a1_shell_open_uses_the_ladder_and_logs_refusal_for_step_one_only() {
    let path = format!("{}/src/shell_open.rs", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap();
    let code: String = text
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        code.contains("foreground_ladder"),
        "shell_open must use the ladder"
    );
    assert_eq!(
        code.matches("foreground refused").count(),
        1,
        "one refused log line"
    );
    assert!(
        code.contains("Step::Plain"),
        "the refused line is tied to step 1 (Step::Plain)"
    );
}
