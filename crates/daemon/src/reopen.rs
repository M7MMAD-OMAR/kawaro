//! Restart and reopen after an update (S-099 B5, B6).
//!
//! When the user presses Update now in the visible settings window, a small
//! marker `update-reopen.json` is written next to `config.toml` before the
//! install. The next daemon start reads and deletes it: if the installed
//! version is the one the marker names and the marker is fresh, the settings
//! window opens on the Updates page with "Updated to X". Any other path to an
//! install (the tray, an automatic install, a background tick) writes no
//! marker, so no window ever opens by itself in the background.

use std::io;
use std::path::Path;

use kawaro_core::config::Config;

/// The marker file, in the folder of `config.toml`.
pub const MARKER_FILE: &str = "update-reopen.json";
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
    let text =
        serde_json::json!({ "from": marker.from, "to": marker.to, "at": marker.at }).to_string();
    crate::update::write_atomic(&dir.join(MARKER_FILE), &text)
}

/// Reopen only for the version the marker names, while it is fresh.
pub fn reopen_decision(marker: Option<&Marker>, installed: &str, now: u64) -> Reopen {
    match marker {
        Some(m) if m.to == installed && now.saturating_sub(m.at) <= MAX_AGE_SECS => {
            Reopen::Updated(m.to.clone())
        }
        _ => Reopen::None,
    }
}

/// The marker in `text`; `None` when a key is missing or has the wrong type.
fn parse(text: &str) -> Option<Marker> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    Some(Marker {
        from: v.get("from")?.as_str()?.to_string(),
        to: v.get("to")?.as_str()?.to_string(),
        at: v.get("at")?.as_u64()?,
    })
}

/// Reads the marker and always deletes it, so it is used once.
pub fn take(dir: &Path, installed: &str, now: u64) -> Reopen {
    let path = dir.join(MARKER_FILE);
    let marker = std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| parse(&text));
    let _ = std::fs::remove_file(&path);
    reopen_decision(marker.as_ref(), installed, now)
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

/// Called once at start: opens the settings window on the Updates page with
/// the banner when a fresh marker names the installed version. True when it
/// opened the window.
pub fn on_start(dir: &Path, installed: &str, now: u64, config: &Config) -> bool {
    match take(dir, installed, now) {
        Reopen::Updated(version) => {
            crate::ui::open_settings_updates(config.clone(), Some(version));
            true
        }
        Reopen::None => false,
    }
}
