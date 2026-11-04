//! S-135 AC6 and AC7: the docs pairs, the README pairs, one bilingual
//! CHANGELOG line and the shared clipboard reader used by CLI and daemon.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    let path = root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn no_long_dash(text: &str, name: &str) {
    assert!(
        !text.contains('\u{2013}') && !text.contains('\u{2014}'),
        "{name} has no en or em dash"
    );
}

#[test]
fn ac6_the_english_guide_describes_the_three_flows() {
    let text = read("docs/drop-shortcut.md");
    for needle in ["drop-first", "clipboard", "choose-during-drag"] {
        assert!(
            text.contains(needle),
            "docs/drop-shortcut.md must mention {needle}"
        );
    }
    assert!(
        text.contains("[English](drop-shortcut.md) | [العربية](drop-shortcut.ar.md)"),
        "the language switch line stays"
    );
    no_long_dash(&text, "docs/drop-shortcut.md");
}

#[test]
fn ac6_the_arabic_guide_describes_the_three_flows() {
    let text = read("docs/drop-shortcut.ar.md");
    for needle in ["drop-first", "clipboard"] {
        assert!(
            text.contains(needle),
            "docs/drop-shortcut.ar.md must mention {needle}"
        );
    }
    assert!(
        text.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c)),
        "the Arabic guide is in Arabic"
    );
    assert!(
        text.contains("[English](drop-shortcut.md) | [العربية](drop-shortcut.ar.md)"),
        "the language switch line stays"
    );
    no_long_dash(&text, "docs/drop-shortcut.ar.md");
}

#[test]
fn ac6_the_readme_pairs_point_at_the_updated_guides() {
    let en = read("README.md");
    let ar = read("README.ar.md");
    assert!(
        en.contains("drop-first"),
        "README.md mentions the default flow"
    );
    assert!(en.contains("docs/drop-shortcut.md"));
    assert!(
        ar.contains("drop-first"),
        "README.ar.md mentions the default flow"
    );
    assert!(ar.contains("docs/drop-shortcut.ar.md"));
    no_long_dash(&en, "README.md");
    no_long_dash(&ar, "README.ar.md");
}

#[test]
fn ac7_one_bilingual_changelog_line_names_the_story() {
    let text = read("CHANGELOG.md");
    let line = text
        .lines()
        .find(|line| line.contains("S-135"))
        .expect("a CHANGELOG line naming S-135");
    assert!(
        line.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c)),
        "the S-135 line carries Arabic as well as English: {line}"
    );
    assert!(
        line.contains("drop-first") || line.contains("drop first"),
        "{line}"
    );
    no_long_dash(&text, "CHANGELOG.md");
}

#[test]
fn ac5_the_cli_and_daemon_use_the_shared_reader() {
    let cli = read("crates/cli/src/clipboard.rs");
    assert!(
        cli.contains("kawaro_core::clipboard"),
        "the CLI clipboard module delegates to the shared reader"
    );
    let daemon = std::fs::read_dir(root().join("crates/daemon/src"))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| std::fs::read_to_string(entry.path()).unwrap_or_default())
        .collect::<String>();
    assert!(
        daemon.contains("kawaro_core::clipboard"),
        "the daemon reads the clipboard through the shared module"
    );
}
