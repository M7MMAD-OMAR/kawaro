//! S-107 A3 (backlog/S-107-windows-foreground-verify.md): the retry gate.
//!
//! Hook pinned here (std only, included by path like `s106_foreground.rs`):
//! `crates/daemon/src/foreground_logic.rs` gains
//! `pub fn should_retry(verified: bool, attempts: u32) -> bool`, true only when
//! `verified` is false and `attempts < 1`.
//!
//! A1 (the ladder is unchanged) is covered by the untouched `s106_foreground`.

#[allow(dead_code)]
#[path = "../src/foreground_logic.rs"]
mod foreground_logic;

use foreground_logic::should_retry;

#[test]
fn s107_a3_unverified_first_attempt_retries() {
    assert!(should_retry(false, 0));
}

#[test]
fn s107_a3_unverified_after_one_attempt_does_not_retry() {
    assert!(!should_retry(false, 1));
}

#[test]
fn s107_a3_verified_never_retries() {
    assert!(!should_retry(true, 0));
    assert!(!should_retry(true, 1));
}

#[test]
fn s107_a3_unverified_after_many_attempts_does_not_retry() {
    assert!(!should_retry(false, 5));
}
