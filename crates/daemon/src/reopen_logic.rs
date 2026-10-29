//! The pure part of the restart and reopen after an update (S-101 B2): the
//! marker the old daemon leaves, and what the new one does with it. Std and
//! `serde_json` only, so a test can include this file by path.

/// A marker older than this many seconds is ignored.
pub const MAX_AGE_SECS: u64 = 3600;

/// What the old daemon leaves for the new one.
#[derive(Clone, Debug, PartialEq)]
pub struct Marker {
    pub from: String,
    pub to: String,
    pub at: u64,
}

/// What a start does with the marker.
#[derive(Clone, Debug, PartialEq)]
pub enum Reopen {
    /// The new version runs: open the page with `Updated to X`.
    Updated(String),
    /// The install failed and the old version still runs.
    Failed(String),
}

impl Marker {
    /// The text of the marker file.
    pub fn to_text(&self) -> String {
        serde_json::json!({ "from": self.from, "to": self.to, "at": self.at }).to_string()
    }

    /// The marker in `text`; `None` when a key is missing or has the wrong
    /// type.
    pub fn parse(text: &str) -> Option<Marker> {
        let v: serde_json::Value = serde_json::from_str(text).ok()?;
        Some(Marker {
            from: v.get("from")?.as_str()?.to_string(),
            to: v.get("to")?.as_str()?.to_string(),
            at: v.get("at")?.as_u64()?,
        })
    }
}

/// True when `result` (the text of `update-result.json`) reports a failed
/// install of `to`: the status word `failed`, or the `outcome` the helper
/// writes. A result that names another version is not about this install.
fn failed_result(result: &str, to: &str) -> bool {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(result) else {
        return false;
    };
    if v.get("version")
        .and_then(|x| x.as_str())
        .is_some_and(|x| x != to)
    {
        return false;
    }
    v.get("status").and_then(|x| x.as_str()) == Some("failed")
        || v.get("outcome").and_then(|o| o.get("failed")).is_some()
}

/// What to do with the marker at start, and whether to delete it. The
/// marker is used once, so every present marker is deleted. The new version
/// reopens as Updated; the old one with a failed result reopens as Failed.
pub fn reopen_decision(
    marker: Option<&Marker>,
    result: Option<&str>,
    installed: &str,
    now: u64,
) -> (Option<Reopen>, bool) {
    let Some(m) = marker else {
        return (None, false);
    };
    if now.saturating_sub(m.at) > MAX_AGE_SECS {
        return (None, true);
    }
    if m.to == installed {
        return (Some(Reopen::Updated(m.to.clone())), true);
    }
    if m.from == installed && result.is_some_and(|r| failed_result(r, &m.to)) {
        return (Some(Reopen::Failed(m.to.clone())), true);
    }
    (None, true)
}

/// The marker in the file text: `(None, true)` for corrupt text, which is
/// deleted, and `(None, false)` when there is no file.
pub fn read_marker_or_delete(text: Option<&str>) -> (Option<Marker>, bool) {
    match text {
        None => (None, false),
        Some(t) => match Marker::parse(t) {
            Some(m) => (Some(m), false),
            None => (None, true),
        },
    }
}
