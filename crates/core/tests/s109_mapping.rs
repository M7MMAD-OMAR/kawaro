//! S-109 AC5 and AC6 (backlog/S-109-first-run-remainder.md): the S-089 test
//! map is exact, and S-089 AC3 and the live Windows run are recorded
//! honestly. Source scans only. Written from the acceptance criteria only.
//!
//! Contract: in the S-109 story, the section that starts at the heading
//! `## S-089 test map` and ends at the next `## ` heading is a table with one
//! row per S-089 criterion (nine rows, AC1 to AC9). Every entry written as
//! `path::test_name` in backticks must resolve: the file exists under the
//! repository root and contains `fn test_name`. The AC7 row must be mapped to
//! at least one entry (the implementer fills it with the S-109 tests:
//! `crates/core/tests/s109_notify_mode.rs`,
//! `crates/ui/tests/s109_notify_page.rs` and the daemon cases in
//! `crates/daemon/src/s109_tests.rs`).
//! S-089 itself records AC3 as "superseded by S-094c" and the live Windows
//! run as "not run".

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    let p = root().join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} must exist: {e}", p.display()))
}

const HEAD: &str = concat!("## S-089 test", " map");

fn map_section(story: &str) -> String {
    let start = story.find(HEAD).expect("the S-089 test map heading");
    let rest = &story[start + HEAD.len()..];
    let end = rest.find("\n## ").unwrap_or(rest.len());
    rest[..end].to_string()
}

/// Every backticked `path::name` in the text, as (path, name).
fn entries(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (i, part) in text.split('`').enumerate() {
        if i % 2 == 0 {
            continue;
        }
        if let Some((path, name)) = part.split_once("::")
            && path.ends_with(".rs")
            && !name.is_empty()
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            out.push((path.to_string(), name.to_string()));
        }
    }
    out
}

/// The entries that do not resolve, as `path::name` strings.
fn missing(entries: &[(String, String)]) -> Vec<String> {
    entries
        .iter()
        .filter(|(path, name)| {
            std::fs::read_to_string(root().join(path))
                .map(|t| !t.contains(&format!("fn {name}")))
                .unwrap_or(true)
        })
        .map(|(p, n)| format!("{p}::{n}"))
        .collect()
}

#[test]
fn ac5_every_entry_of_the_map_resolves_to_a_real_test() {
    let section = map_section(&read("backlog/S-109-first-run-remainder.md"));
    let all = entries(&section);
    assert!(all.len() >= 20, "the map names {} entries", all.len());
    let gone = missing(&all);
    assert!(gone.is_empty(), "0 missing expected, got {gone:?}");
}

#[test]
fn ac5_the_check_reports_a_mutated_name() {
    let section = map_section(&read("backlog/S-109-first-run-remainder.md"));
    let mut all = entries(&section);
    assert!(!all.is_empty());
    all[0].1.push_str("_mutated");
    let gone = missing(&all);
    assert_eq!(gone.len(), 1, "{gone:?}");
    assert!(gone[0].ends_with("_mutated"));
}

#[test]
fn ac5_the_map_has_nine_rows_and_ac7_is_mapped_to_real_tests() {
    let section = map_section(&read("backlog/S-109-first-run-remainder.md"));
    let rows: Vec<&str> = section.lines().filter(|l| l.starts_with("| AC")).collect();
    assert_eq!(rows.len(), 9, "{rows:?}");
    for (i, row) in rows.iter().enumerate() {
        assert!(row.starts_with(&format!("| AC{} ", i + 1)), "{row}");
    }
    let ac7 = rows[6];
    assert!(
        !entries(ac7).is_empty(),
        "AC7 must name its tests, not 'this story': {ac7}"
    );
    assert!(
        !ac7.contains("this story"),
        "AC7 must not point at the story itself: {ac7}"
    );
}

#[test]
fn ac6_s089_records_the_supersession_and_the_manual_run() {
    let s089 = read("backlog/S-089-first-run-and-settings-recovery.md");
    assert!(
        s089.contains("superseded by S-094c"),
        "S-089 must record AC3 as superseded by S-094c"
    );
    assert!(
        s089.contains("not run"),
        "S-089 must record the live Windows run as manual and not run"
    );
    assert!(
        !s089.to_lowercase().contains("verified on windows"),
        "no criterion may be marked verified on Windows"
    );
}

#[test]
fn ac6_the_story_text_carries_the_same_two_records() {
    let story = read("backlog/S-109-first-run-remainder.md");
    assert!(story.contains("superseded by S-094c"));
    assert!(story.contains("not run"));
}
