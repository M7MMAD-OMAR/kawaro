//! Restart and reopen after an update (S-099 B5, B6).
//!
//! When the user presses Update now in the visible settings window, a small
//! marker `update-reopen.json` is written next to `config.toml` before the
//! install. The next daemon start reads and deletes it: if the installed
//! version is the one the marker names and the marker is fresh, the settings
//! window opens on the Updates page with "Updated to X". Any other path to an
//! install by the tray or a background tick with no visible window writes no
//! marker, so no window ever opens by itself in the background. S-101: an
//! automatic install with the window visible writes one too, and a start
//! whose old version still runs after a failed install (a `Failed` result
//! file) reopens the page with the failed banner.

use std::io;
use std::path::Path;

use kawaro_core::config::Config;

#[cfg(test)]
pub use crate::reopen_logic::MAX_AGE_SECS;
pub use crate::reopen_logic::{Marker, Reopen as Decision};
use crate::reopen_logic::{read_marker_or_delete, reopen_decision as decide};

/// The marker file, in the folder of `config.toml`.
pub const MARKER_FILE: &str = "update-reopen.json";

/// What a start of the S-099 reopen did: the old two-way answer, kept for
/// its table of tests.
#[derive(Clone, Debug, PartialEq)]
pub enum Reopen {
    None,
    Updated(String),
}

/// Who asked for the install.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ApplyOrigin {
    /// The Update now button of the settings window.
    Settings,
    /// The tray item and an automatic install write no marker; only the
    /// tests name them, to pin that rule.
    #[cfg_attr(not(test), allow(dead_code))]
    Tray,
    #[cfg_attr(not(test), allow(dead_code))]
    Auto,
}

/// Writes the marker atomically.
pub fn write(dir: &Path, marker: &Marker) -> io::Result<()> {
    crate::update::write_atomic(&dir.join(MARKER_FILE), &marker.to_text())
}

/// Reopen only for the version the marker names, while it is fresh.
#[cfg_attr(not(test), allow(dead_code))]
pub fn reopen_decision(marker: Option<&Marker>, installed: &str, now: u64) -> Reopen {
    match decide(marker, None, installed, now).0 {
        Some(Decision::Updated(v)) => Reopen::Updated(v),
        _ => Reopen::None,
    }
}

/// Reads the marker and always deletes it, so it is used once. The result
/// file is read, not deleted: the toast still takes it afterwards.
pub fn take_decision(dir: &Path, installed: &str, now: u64) -> Option<Decision> {
    let path = dir.join(MARKER_FILE);
    let text = std::fs::read_to_string(&path).ok();
    let (marker, corrupt) = read_marker_or_delete(text.as_deref());
    let result = std::fs::read_to_string(dir.join(kawaro_core::update::result::RESULT_FILE)).ok();
    let (decision, delete) = decide(marker.as_ref(), result.as_deref(), installed, now);
    if delete || corrupt {
        let _ = std::fs::remove_file(&path);
    }
    decision
}

/// The old answer of `take_decision`, without a failed result.
#[cfg_attr(not(test), allow(dead_code))]
pub fn take(dir: &Path, installed: &str, now: u64) -> Reopen {
    match take_decision(dir, installed, now) {
        Some(Decision::Updated(v)) => Reopen::Updated(v),
        _ => Reopen::None,
    }
}

/// Called just before an install starts: writes the marker only for the
/// click on Update now in the visible settings window. True when written.
pub fn before_install(
    dir: &Path,
    origin: ApplyOrigin,
    settings_visible: bool,
    from: &str,
    to: &str,
    now: u64,
) -> bool {
    if origin != ApplyOrigin::Settings || !settings_visible {
        return false;
    }
    let marker = Marker {
        from: from.to_string(),
        to: to.to_string(),
        at: now,
    };
    write(dir, &marker).is_ok()
}

/// Called just before an install that did not come from the Update now
/// click (an automatic one, or one that waited for jobs): writes the marker
/// when the settings window is visible, so the new daemon reopens it
/// (S-101 B2). True when written.
pub fn before_auto_install(
    dir: &Path,
    settings_visible: bool,
    from: &str,
    to: &str,
    now: u64,
) -> bool {
    if !settings_visible {
        return false;
    }
    let marker = Marker {
        from: from.to_string(),
        to: to.to_string(),
        at: now,
    };
    write(dir, &marker).is_ok()
}

/// Removes the marker of an install that did not start.
pub fn clear(dir: &Path) {
    let _ = std::fs::remove_file(dir.join(MARKER_FILE));
}

/// Called once at start: opens the settings window on the Updates page with
/// the banner when a fresh marker names the installed version (Updated), or
/// the version that did not install while the old one still runs (Failed,
/// S-101 B3). True when it opened the window.
pub fn on_start(dir: &Path, installed: &str, now: u64, config: &Config) -> bool {
    match take_decision(dir, installed, now) {
        Some(Decision::Updated(version)) => {
            crate::ui::open_settings_updates(config.clone(), Some(version));
            true
        }
        Some(Decision::Failed(version)) => {
            crate::ui::open_settings_failed(config.clone(), version);
            true
        }
        None => false,
    }
}
