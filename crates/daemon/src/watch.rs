//! Reloads settings and user actions when their files change, so editing
//! `config.toml` by hand needs no `sbarhive reload`. Polls file times every
//! two seconds: two small paths, no extra dependency.

use crate::state::Shared;
use sbarhive_core::config::Config;
use sbarhive_core::queue::Queue;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

const EVERY: Duration = Duration::from_secs(2);

/// What changes when a watched file is written, added or removed.
pub type Stamp = Vec<(PathBuf, Option<SystemTime>, u64)>;

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
    files
        .into_iter()
        .map(|p| {
            let meta = std::fs::metadata(&p).ok();
            let modified = meta.as_ref().and_then(|m| m.modified().ok());
            let len = meta.map_or(0, |m| m.len());
            (p, modified, len)
        })
        .collect()
}

pub fn run(state: Shared, queue: Arc<Queue>) {
    let (Some(config), Some(actions)) = (Config::path(), Config::actions_dir()) else {
        return;
    };
    let mut last = stamp(&config, &actions);
    loop {
        std::thread::sleep(EVERY);
        let now = stamp(&config, &actions);
        if now != last {
            last = now;
            let (state, queue) = (state.clone(), queue.clone());
            let _ = slint::invoke_from_event_loop(move || crate::ui::reload(&state, &queue));
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
