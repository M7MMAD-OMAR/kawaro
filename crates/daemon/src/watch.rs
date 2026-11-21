//! Reloads settings and user actions when their files change, so editing
//! `config.toml` by hand needs no `kawaro reload`. Polls file times every
//! two seconds: two small paths, no extra dependency.

use crate::state::Shared;
use kawaro_core::config::Config;
use kawaro_core::queue::Queue;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

const EVERY: Duration = Duration::from_secs(2);

/// What changes when a watched file is written, added or removed.
pub type Stamp = Vec<(PathBuf, Option<SystemTime>, u64)>;

/// One path, its modification time and its length.
fn stamp_entry(path: &Path) -> (PathBuf, Option<SystemTime>, u64) {
    let meta = std::fs::metadata(path).ok();
    let modified = meta.as_ref().and_then(|m| m.modified().ok());
    let len = meta.map_or(0, |m| m.len());
    (path.to_path_buf(), modified, len)
}

/// The config file plus every `*.toml` file in the actions directory.
pub fn stamp(config: &Path, actions: &Path) -> Stamp {
    let mut files = vec![config.to_path_buf()];
    if let Ok(dir) = std::fs::read_dir(actions) {
        let mut tomls: Vec<PathBuf> = dir
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "toml"))
            .collect();
        tomls.sort();
        files.extend(tomls);
    }
    files.iter().map(|p| stamp_entry(p)).collect()
}

/// The preset store alone. It lives outside the actions directory, so it has
/// its own stamp and a preset-only change never reloads the settings.
pub fn preset_stamp(path: &Path) -> Stamp {
    vec![stamp_entry(path)]
}

pub fn run(state: Shared, queue: Arc<Queue>) {
    let (Some(config), Some(actions)) = (Config::path(), Config::actions_dir()) else {
        return;
    };
    let preset_path = kawaro_core::presets::path();
    let mut last_settings = stamp(&config, &actions);
    let mut last_presets = preset_path.as_deref().map(preset_stamp);
    loop {
        std::thread::sleep(EVERY);
        let settings = stamp(&config, &actions);
        let settings_changed = settings != last_settings;
        if settings_changed {
            last_settings = settings;
        }
        let presets = preset_path.as_deref().map(preset_stamp);
        let presets_changed = presets != last_presets;
        if presets_changed {
            last_presets = presets;
        }
        if !settings_changed && !presets_changed {
            continue;
        }
        if settings_changed {
            let (state, queue) = (state.clone(), queue.clone());
            let _ = slint::invoke_from_event_loop(move || crate::ui::reload(&state, &queue));
        }
        if presets_changed {
            // A preset change is independent of a settings change in the
            // same poll interval.
            let presets = crate::state::lock(&state).presets.clone();
            presets.reload_async();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::stamp;

    #[test]
    fn stamp_changes_when_a_watched_file_changes() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("config.toml");
        let actions = dir.path().join("actions");
        let s0 = stamp(&config, &actions);
        assert_eq!(s0, stamp(&config, &actions));

        std::fs::write(&config, "language = \"ar\"\n").unwrap();
        let s1 = stamp(&config, &actions);
        assert_ne!(s0, s1);

        std::fs::write(&config, "language = \"en\"\nnotify = false\n").unwrap();
        let s2 = stamp(&config, &actions);
        assert_ne!(
            s1, s2,
            "a longer file changes the stamp even within one mtime tick"
        );

        std::fs::create_dir(&actions).unwrap();
        std::fs::write(actions.join("notes.txt"), "x").unwrap();
        assert_eq!(s2, stamp(&config, &actions), "only toml files count");
        std::fs::write(actions.join("mine.toml"), "").unwrap();
        let s3 = stamp(&config, &actions);
        assert_ne!(s2, s3);
        assert_eq!(s3.len(), 2);

        std::fs::remove_file(actions.join("mine.toml")).unwrap();
        assert_eq!(stamp(&config, &actions), s2);
    }
}
