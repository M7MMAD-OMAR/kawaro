//! S-109 AC1 and AC2 (backlog/S-109-first-run-remainder.md): the pure
//! notification decision and the `notify_mode` config key. Written from the
//! acceptance criteria only.
//!
//! Hooks these tests pin (new, in `kawaro_core::config`):
//! - `NotifyMode { Off, All, Long }`: Copy, Debug, PartialEq, serde as the
//!   lowercase words `off`, `all`, `long`.
//! - `LONG_JOB: Duration` is 10 seconds.
//! - `notify_decision(mode: NotifyMode, elapsed: Option<Duration>) -> bool`.
//! - `Config.notify_mode: NotifyMode`, default `All`. The file key is spelled
//!   with an underscore, `notify_mode`, as `hive_surface` is (the planner's
//!   text). The two legacy keys keep their current file spelling, `notify`
//!   and `notify-long-only` (the struct is kebab case), and stay as fields.
//! - The daemon source `crates/daemon/src/jobs.rs` no longer has the old
//!   two boolean function and never reads the legacy fields.
//!
//! Needles are split with `concat!` so this file never matches itself.

use kawaro_core::config::{Config, LONG_JOB, NotifyMode, Quality, notify_decision};
use std::path::{Path, PathBuf};
use std::time::Duration;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn write(dir: &Path, text: &str) -> PathBuf {
    let p = dir.join("config.toml");
    std::fs::write(&p, text).unwrap();
    p
}

// ------------------------------------------------------------ AC1

#[test]
fn ac1_the_long_job_threshold_is_ten_seconds() {
    assert_eq!(LONG_JOB, Duration::from_secs(10));
}

#[test]
fn ac1_the_decision_table_has_eighteen_cells() {
    let inputs: [Option<Duration>; 6] = [
        Some(Duration::ZERO),
        Some(Duration::from_millis(9_999)),
        Some(Duration::from_secs(10)),
        Some(Duration::from_secs(60)),
        None,
        Some(Duration::from_secs(10) - Duration::from_nanos(1)),
    ];
    // Off: false everywhere. All: true everywhere, None included.
    // Long: only 10 s and 60 s.
    let expected = [
        (NotifyMode::Off, [false; 6]),
        (NotifyMode::All, [true; 6]),
        (NotifyMode::Long, [false, false, true, true, false, false]),
    ];
    let mut cells = 0;
    for (mode, row) in expected {
        for (input, want) in inputs.iter().zip(row) {
            assert_eq!(notify_decision(mode, *input), want, "{mode:?} {input:?}");
            cells += 1;
        }
    }
    assert_eq!(cells, 18);
}

#[test]
fn ac1_the_daemon_uses_the_pure_decision_and_not_the_two_booleans() {
    let text = std::fs::read_to_string(root().join("crates/daemon/src/jobs.rs")).unwrap();
    let code: String = text
        .lines()
        .take_while(|l| l.trim() != "#[cfg(test)]")
        .map(|l| format!("{l}\n"))
        .collect();
    assert!(
        code.contains("notify_decision"),
        "jobs.rs must call notify_decision"
    );
    assert!(
        !code.contains(concat!("fn should_", "notify")),
        "the old two boolean function must be gone"
    );
    assert!(
        !code.contains(concat!("notify_long", "_only")),
        "the daemon must not read the legacy long switch"
    );
    assert!(
        !code.contains(concat!("config.", "notify,"))
            && !code.contains(concat!("config.", "notify;")),
        "the daemon must not read the legacy on switch"
    );
}

// ------------------------------------------------------------ AC2

#[test]
fn ac2_old_files_keep_their_behaviour() {
    let cases = [
        (
            "notify = false\nnotify-long-only = false\n",
            NotifyMode::Off,
        ),
        ("notify = false\nnotify-long-only = true\n", NotifyMode::Off),
        ("notify = true\nnotify-long-only = false\n", NotifyMode::All),
        ("notify = true\nnotify-long-only = true\n", NotifyMode::Long),
        ("language = \"en\"\n", NotifyMode::All),
        ("notify = true\n", NotifyMode::All),
        ("notify = false\n", NotifyMode::Off),
        ("notify-long-only = true\n", NotifyMode::Long),
    ];
    for (text, mode) in cases {
        let dir = tempfile::tempdir().unwrap();
        let (c, warn) = Config::load_from(&write(dir.path(), text));
        assert_eq!(c.notify_mode, mode, "{text:?}");
        assert!(warn.is_none(), "{text:?}: {warn:?}");
    }
}

#[test]
fn ac2_the_default_is_all() {
    assert_eq!(Config::default().notify_mode, NotifyMode::All);
}

#[test]
fn ac2_a_file_with_notify_mode_ignores_the_legacy_keys() {
    for (text, mode) in [
        (
            "notify_mode = \"off\"\nnotify = true\nnotify-long-only = true\n",
            NotifyMode::Off,
        ),
        (
            "notify_mode = \"long\"\nnotify = false\nnotify-long-only = false\n",
            NotifyMode::Long,
        ),
        (
            "notify_mode = \"all\"\nnotify = false\nnotify-long-only = true\n",
            NotifyMode::All,
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let (c, warn) = Config::load_from(&write(dir.path(), text));
        assert_eq!(c.notify_mode, mode, "{text:?}");
        assert!(warn.is_none(), "{text:?}: {warn:?}");
    }
}

#[test]
fn ac2_an_unknown_mode_falls_back_to_all_with_a_warning_and_keeps_other_keys() {
    let dir = tempfile::tempdir().unwrap();
    let (c, warn) = Config::load_from(&write(
        dir.path(),
        "quality = \"max\"\nnotify_mode = \"sometimes\"\n",
    ));
    assert_eq!(c.notify_mode, NotifyMode::All);
    assert_eq!(c.quality, Quality::Max, "the load must not fall back whole");
    let warn = warn.expect("an unknown mode warns");
    assert!(warn.contains("notify_mode"), "{warn}");
}

#[test]
fn ac2_saving_writes_the_mode_and_the_two_legacy_keys() {
    for (mode, word, on, long) in [
        (NotifyMode::Off, "off", false, false),
        (NotifyMode::All, "all", true, false),
        (NotifyMode::Long, "long", true, true),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("config.toml");
        Config {
            notify_mode: mode,
            ..Config::default()
        }
        .save_to(&p)
        .unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        let table: toml::Table = toml::from_str(&text).unwrap();
        assert_eq!(table["notify_mode"].as_str(), Some(word), "{text}");
        assert_eq!(table["notify"].as_bool(), Some(on), "{text}");
        assert_eq!(table["notify-long-only"].as_bool(), Some(long), "{text}");
        let (back, warn) = Config::load_from(&p);
        assert!(warn.is_none(), "{warn:?}");
        assert_eq!(back.notify_mode, mode);
    }
}

#[test]
fn ac2_other_keys_round_trip_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let p = write(
        dir.path(),
        "language = \"ar\"\nquality = \"max\"\nfuture-key = 3\nnotify_mode = \"long\"\n",
    );
    let (c, warn) = Config::load_from(&p);
    assert!(warn.is_none(), "{warn:?}");
    let out = dir.path().join("out.toml");
    c.save_to(&out).unwrap();
    let (again, warn) = Config::load_from(&out);
    assert!(warn.is_none(), "{warn:?}");
    assert_eq!(again, c);
    assert_eq!(again.quality, Quality::Max);
    assert_eq!(again.notify_mode, NotifyMode::Long);
}
