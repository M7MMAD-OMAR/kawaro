//! S-046 AC7 (backlog/S-046-windows-hive-window.md): the pure collector that
//! merges the paths Explorer sends one process at a time.
//!
//! Contract (tester, from AC7; do not edit). In `kawaro_core::merge`:
//! - `pub enum Push { First, Merged }`, `Debug + PartialEq + Clone + Copy`.
//! - `pub struct Collector`, `Collector::new()`.
//! - `push(&mut self, at_ms: u64, path: PathBuf) -> Push`.
//! - `files(&self) -> Vec<PathBuf>`, first seen order, no duplicates.

use kawaro_core::merge::{Collector, Push};
use std::path::PathBuf;

fn p(s: &str) -> PathBuf {
    PathBuf::from(s)
}

#[test]
fn ac7_first_path_is_first() {
    let mut c = Collector::new();
    assert_eq!(c.push(0, p("a")), Push::First);
}

#[test]
fn ac7_gaps_are_measured_from_the_last_push() {
    let mut c = Collector::new();
    assert_eq!(c.push(0, p("a")), Push::First);
    assert_eq!(c.push(100, p("b")), Push::Merged);
    // 250 is 150 after the last push, though 250 after the first.
    assert_eq!(c.push(250, p("c")), Push::Merged);
}

#[test]
fn ac7_long_chain_keeps_merging_past_300_ms_in_total() {
    let mut c = Collector::new();
    c.push(0, p("a"));
    c.push(299, p("b"));
    assert_eq!(c.push(598, p("c")), Push::Merged);
    assert_eq!(c.push(897, p("d")), Push::Merged);
}

#[test]
fn ac7_a_gap_of_exactly_300_starts_a_new_batch() {
    let mut c = Collector::new();
    assert_eq!(c.push(250, p("a")), Push::First);
    assert_eq!(c.push(550, p("b")), Push::First);
}

#[test]
fn ac7_a_gap_of_299_still_merges() {
    let mut c = Collector::new();
    c.push(250, p("a"));
    assert_eq!(c.push(549, p("b")), Push::Merged);
}

#[test]
fn ac7_files_are_in_first_seen_order_without_duplicates() {
    let mut c = Collector::new();
    c.push(0, p("a"));
    c.push(10, p("b"));
    c.push(20, p("a"));
    assert_eq!(c.files(), vec![p("a"), p("b")]);
}

#[test]
fn ac7_a_new_batch_forgets_the_old_files() {
    let mut c = Collector::new();
    c.push(0, p("a"));
    c.push(10, p("b"));
    assert_eq!(c.push(1000, p("c")), Push::First);
    assert_eq!(c.files(), vec![p("c")]);
}

#[test]
fn ac7_a_duplicate_still_counts_as_a_push_for_the_gap() {
    let mut c = Collector::new();
    c.push(0, p("a"));
    c.push(200, p("a"));
    // 450 is 250 after the duplicate at 200, so it merges.
    assert_eq!(c.push(450, p("b")), Push::Merged);
    assert_eq!(c.files(), vec![p("a"), p("b")]);
}
