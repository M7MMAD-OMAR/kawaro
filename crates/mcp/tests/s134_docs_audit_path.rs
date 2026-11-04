//! S-134 AC3: both MCP docs document audit_path, the absolute/~/relative
//! rules and the 1 MiB rotation, without the old always-config-folder claim
//! and without a long dash.

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
fn ac3_the_english_doc_documents_audit_path() {
    let text = read("docs/mcp.md");
    for needle in ["audit_path", "absolute", "~", "relative", "1 MiB"] {
        assert!(text.contains(needle), "docs/mcp.md must mention {needle}");
    }
    assert!(
        !text.contains("stays there even when"),
        "the old always-in-the-config-folder sentence is gone"
    );
    no_long_dash(&text, "docs/mcp.md");
}

#[test]
fn ac3_the_arabic_doc_documents_audit_path() {
    let text = read("docs/mcp.ar.md");
    for needle in ["audit_path", "~", "1 MiB"] {
        assert!(
            text.contains(needle),
            "docs/mcp.ar.md must mention {needle}"
        );
    }
    assert!(
        text.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c)),
        "the Arabic doc is in Arabic"
    );
    no_long_dash(&text, "docs/mcp.ar.md");
}
