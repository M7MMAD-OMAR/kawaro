//! S-115 AC5 (script shape), AC7 (docs parity) and AC8 (records), read from
//! the repository files. The live drop runs by hand (`tests/x11/drop.sh`).

use std::fs;
use std::path::PathBuf;

fn read(rel: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn has_long_dash(s: &str) -> bool {
    s.contains('\u{2014}') || s.contains('\u{2013}')
}

fn x11_row(doc: &str) -> String {
    doc.lines()
        .find(|l| l.starts_with("| X11 |"))
        .expect("an X11 row")
        .to_string()
}

/// The numbered steps of the `## X11` section.
fn x11_steps(doc: &str) -> Vec<String> {
    let section = doc
        .split("\n## X11")
        .nth(1)
        .expect("an X11 section")
        .split("\n## ")
        .next()
        .unwrap();
    section
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            let digits = t.chars().take_while(char::is_ascii_digit).count();
            digits > 0 && t[digits..].starts_with(". ")
        })
        .map(str::to_string)
        .collect()
}

#[test]
fn ac5_the_live_drop_script_checks_copy_only_three_times() {
    let s = read("tests/x11/drop.sh");
    for needle in [
        "GDK_BACKEND=x11",
        "tests/drop/source.py",
        "sha256sum",
        "file --mime-type",
        "deleted=False",
        ".webp",
        "Shift",
    ] {
        assert!(s.contains(needle), "drop.sh names {needle}");
    }
    assert!(
        s.contains("seq 1 3") || s.contains("{1..3}"),
        "drop.sh runs 3 times"
    );
    assert!(s.contains("5"), "a 5 s wait for the result");
}

#[test]
fn ac7_the_x11_rows_say_the_verified_state() {
    let en = x11_row(&read("README.md"));
    let ar = x11_row(&read("README.ar.md"));
    assert!(!en.contains("Not supported yet"), "{en}");
    assert!(!ar.contains("غير مدعوم بعد"), "{ar}");
    let lower = en.to_lowercase();
    assert!(
        lower.contains("tested"),
        "platform plus tested or not: {en}"
    );
    assert_eq!(en.matches('|').count(), ar.matches('|').count());
}

#[test]
fn ac7_both_drop_shortcut_docs_have_an_x11_section_with_three_steps() {
    let en = read("docs/drop-shortcut.md");
    let ar = read("docs/drop-shortcut.ar.md");
    let first = "[English](drop-shortcut.md) | [العربية](drop-shortcut.ar.md)";
    assert_eq!(en.lines().next(), Some(first));
    assert_eq!(ar.lines().next(), Some(first));
    let (se, sa) = (x11_steps(&en), x11_steps(&ar));
    assert_eq!(se.len(), 3, "English steps: {se:?}");
    assert_eq!(sa.len(), 3, "Arabic steps: {sa:?}");
    assert!(en.contains("kawaro drop") && ar.contains("kawaro drop"));
}

#[test]
fn ac8_the_changelog_has_one_line_for_s113_to_s115_in_both_languages() {
    let log = read("CHANGELOG.md");
    let line = log
        .lines()
        .find(|l| l.contains("S-113") && l.contains("S-115"))
        .expect("a line naming S-113 to S-115");
    assert!(line.contains(" | "), "English and Arabic: {line}");
    assert!(line.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c)));
    assert!(!has_long_dash(line));
}

#[test]
fn ac8_adr_0003_closes_the_x11_item_with_the_measured_result() {
    let adr = read("docs/adr/0003-drop-hive.md");
    let line = adr
        .lines()
        .find(|l| l.contains("X11 by running"))
        .expect("the X11 item");
    assert!(line.starts_with("- [x]"), "closed: {line}");
    assert!(!has_long_dash(&adr));
}
