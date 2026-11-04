//! Protocol constants: revisions, server identity and the instructions line.

use serde_json::{Value, json};

/// The instruction line every host shows to the model.
pub(crate) const INSTRUCTIONS: &str =
    "Local file conversion and PDF tools. Nothing leaves the machine.";

/// The revisions this server accepts in `initialize`, newest first.
pub(crate) const PROTOCOL_VERSIONS: [&str; 5] = [
    "2026-07-28",
    "2025-11-25",
    "2025-06-18",
    "2025-03-26",
    "2024-11-05",
];

/// The revisions `server/discover` advertises.
pub(crate) const DISCOVER_VERSIONS: [&str; 3] = ["2026-07-28", "2025-11-25", "2025-06-18"];

/// The revision used when the host asks for one this build does not know.
pub(crate) const FALLBACK_VERSION: &str = "2025-11-25";

/// `serverInfo`: the name and the Kawaro version.
pub(crate) fn server_info() -> Value {
    json!({
        "name": "kawaro",
        "version": env!("CARGO_PKG_VERSION"),
    })
}

/// The capabilities object. Tools only: no resources, no prompts, no logging.
pub(crate) fn capabilities() -> Value {
    json!({"tools": {}})
}
