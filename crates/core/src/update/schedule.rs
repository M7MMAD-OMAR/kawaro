//! When the next automatic check is due (ADR 0006 section 5).

use serde::{Deserialize, Serialize};

const HOUR: u64 = 3600;
/// The longest wait after failures.
const MAX_BACKOFF: u64 = 24 * HOUR;

/// What `update-state.json` holds, in seconds since the Unix epoch.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    pub last_check: Option<u64>,
    pub last_failure: Option<u64>,
    pub failures: u32,
}

impl State {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Anything that is not a valid state gives the default.
    pub fn from_json(text: &str) -> State {
        serde_json::from_str(text).unwrap_or_default()
    }
}

/// A clock that went backwards counts as elapsed.
fn elapsed(now: u64, since: u64, wait: u64) -> bool {
    now < since || now - since >= wait
}

/// True when a check should run: after failures, once `min(2^(n-1) h, 24 h)`
/// has passed since the last failure; otherwise once `interval_secs` has
/// passed since the last success, or never checked.
pub fn due(s: &State, now_secs: u64, interval_secs: u64) -> bool {
    if s.failures > 0 {
        let wait = (HOUR << (s.failures - 1).min(5)).min(MAX_BACKOFF);
        return s.last_failure.is_none_or(|t| elapsed(now_secs, t, wait));
    }
    s.last_check
        .is_none_or(|t| elapsed(now_secs, t, interval_secs))
}

/// Automatic checks need `update_check`; a manual check always runs.
pub fn allowed(update_check: bool, manual: bool) -> bool {
    update_check || manual
}
