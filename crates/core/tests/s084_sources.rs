//! S-084 (backlog/S-084-windows-drag-trigger.md) AC1 and AC5: the spike
//! report with the five measurements, the manager decision line that names
//! the Windows trigger, and the doc checks. They cannot run on Windows here,
//! so they check the files themselves, like `s081_sources.rs`.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    let path = root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} must exist: {e}", path.display()))
}

/// The Windows section of a doc: from the heading that names Windows (in
/// either language) to the next `## ` heading, or the end.
fn windows_section(s: &str) -> &str {
    let mut from = 0;
    let head = loop {
        let rest = &s[from..];
        let Some(i) = rest.find("\n## ") else {
            panic!("the doc has no Windows section: {s}");
        };
        let line_start = from + i + 1;
        let line_end = s[line_start..]
            .find('\n')
            .map(|e| line_start + e)
            .unwrap_or(s.len());
        let line = &s[line_start..line_end];
        if line.starts_with("## ") && (line.contains("Windows") || line.contains("ويندوز")) {
            break line_start;
        }
        from = line_end;
    };
    let body = &s[head + 3..];
    match body.find("\n## ") {
        Some(end) => &body[..end],
        None => body,
    }
}

#[test]
fn ac1_the_spike_report_records_the_five_measurements() {
    let s = read("spikes/windows-drag-trigger/out/report.md");
    for m in ["## M1", "## M2", "## M3", "## M4", "## M5"] {
        assert!(s.contains(m), "the spike report must record {m}: {s}");
    }
}

#[test]
fn ac1_the_manager_decision_line_names_the_windows_trigger() {
    // AC1: the default is named in a `## Manager decision` line before any
    // Windows trigger constant exists, and the code constant follows it.
    let s = read("backlog/S-084-windows-drag-trigger.md");
    let at = s.find("## Manager decision").expect(
        "AC1: a `## Manager decision` line must name the Windows trigger before any Windows trigger constant",
    );
    assert!(
        s[at..].contains("right-button drag"),
        "the manager decision names the trigger: {}",
        &s[at..]
    );
    assert!(
        read("crates/core/src/drop_key.rs").contains("RightDrag"),
        "the Windows trigger constant follows the manager decision"
    );
}

#[test]
fn ac5_the_docs_describe_only_the_measured_windows_trigger() {
    for (rel, phrase) in [
        ("docs/drop-shortcut.md", "right mouse button"),
        ("docs/drop-shortcut.ar.md", "زر الفأرة الأيمن"),
        ("README.md", "right mouse button"),
        ("README.ar.md", "زر الفأرة الأيمن"),
    ] {
        let s = read(rel);
        let win = windows_section(&s);
        assert!(
            win.contains(phrase),
            "{rel} must describe the measured Windows trigger with `{phrase}`: {win}"
        );
    }
}

#[test]
fn ac5_no_doc_tells_a_windows_user_to_hold_alt() {
    // The S-075 rule and the owner's bug report: never ship hold-Alt wording
    // for Windows. The Windows section of every named doc stays free of Alt;
    // the Linux sections may keep their own wording.
    for rel in [
        "docs/drop-shortcut.md",
        "docs/drop-shortcut.ar.md",
        "README.md",
        "README.ar.md",
        "docs/guide/shortcuts.md",
        "docs/guide/shortcuts.ar.md",
    ] {
        let s = read(rel);
        let win = windows_section(&s);
        assert!(
            !win.contains("Alt"),
            "{rel} names Alt in its Windows section: {win}"
        );
    }
}

#[test]
fn ac5_every_chord_mention_carries_the_bilingual_warning() {
    // D5: the S-048 chord stays an undocumented internal path and every
    // mention carries a plain bilingual warning that holding it during a
    // drag changes what Explorer does with the drag.
    const WARN_EN: &str = "changes what Explorer does with the drag";
    const WARN_AR: &str = "يغيّر ما يفعله Explorer مع السحب";
    for (rel, warn) in [
        ("docs/drop-shortcut.md", WARN_EN),
        ("docs/drop-shortcut.ar.md", WARN_AR),
        ("docs/guide/shortcuts.md", WARN_EN),
        ("docs/guide/shortcuts.ar.md", WARN_AR),
        ("docs/guide/settings.md", WARN_EN),
        ("docs/guide/settings.ar.md", WARN_AR),
        ("README.md", WARN_EN),
        ("README.ar.md", WARN_AR),
    ] {
        let s = read(rel);
        for block in s.split("\n\n") {
            if block.contains("Ctrl+Alt+H") {
                assert!(
                    block.contains(warn),
                    "{rel} mentions the chord without the bilingual warning: {block}"
                );
            }
        }
    }
}

#[test]
fn ac5_the_changelog_has_one_s084_line_in_english_and_arabic() {
    let s = read("CHANGELOG.md");
    let lines: Vec<&str> = s.lines().filter(|l| l.contains("[S-084]")).collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].contains('«') && lines[0].contains('»'),
        "the S-084 line carries its Arabic part in guillemets like the other entries: {}",
        lines[0]
    );
    assert!(
        !lines[0].contains("hold Alt"),
        "no hold-Alt wording ships for Windows: {}",
        lines[0]
    );
}
