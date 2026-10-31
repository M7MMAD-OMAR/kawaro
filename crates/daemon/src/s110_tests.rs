//! S-110 (backlog/S-110-open-at-position.md) AC4: `server::open` passes the
//! pointer position on to the presenter.
//!
//! Hooks these tests pin (in `server.rs`; the implementer builds them):
//! - `pub fn open(state: &Shared, files: Vec<PathBuf>, token: Option<String>,
//!   at: Option<(f64, f64)>) -> Result<(), String>` (the fourth parameter is
//!   new); production calls `open_with` with the existing `ui::present`.
//! - `pub(crate) struct Presented { pub files: Vec<PathBuf>, pub token:
//!   Option<String>, pub at: Option<(f64, f64)>, .. }`: what the presenter is
//!   given, other fields (centre, message, suggestion, cells) are free.
//! - `pub(crate) fn open_with(state: &Shared, files: Vec<PathBuf>, token:
//!   Option<String>, at: Option<(f64, f64)>, present: &mut dyn
//!   FnMut(Presented)) -> Result<(), String>`: the same body as `open`,
//!   calling `present` exactly once when a hive is shown.
//! - `crate::ui::present` takes the same `at` and hands it to the hive.
//! - `Request::Open { files, token, at }` in `handle` forwards `at`.

use crate::server::{Presented, open_with};
use crate::state::State;
use kawaro_core::config::Config;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn state() -> crate::state::Shared {
    Arc::new(Mutex::new(State::new(Config::default(), &[])))
}

fn run(at: Option<(f64, f64)>) -> Vec<(Vec<PathBuf>, Option<String>, Option<(f64, f64)>)> {
    let mut seen = Vec::new();
    let r = open_with(
        &state(),
        vec![PathBuf::from("unknown.zzz")],
        Some("T".into()),
        at,
        &mut |p: Presented| seen.push((p.files, p.token, p.at)),
    );
    assert!(r.is_ok(), "{r:?}");
    seen
}

#[test]
fn ac4_open_with_some_calls_the_presenter_once_with_that_position() {
    let seen = run(Some((400.0, 500.0)));
    assert_eq!(
        seen,
        vec![(
            vec![PathBuf::from("unknown.zzz")],
            Some("T".to_string()),
            Some((400.0, 500.0))
        )]
    );
}

#[test]
fn ac4_open_with_none_calls_the_presenter_once_with_none() {
    let seen = run(None);
    assert_eq!(
        seen,
        vec![(
            vec![PathBuf::from("unknown.zzz")],
            Some("T".to_string()),
            None
        )]
    );
}

#[test]
fn ac4_negative_and_fractional_positions_are_passed_unchanged() {
    let seen = run(Some((-1280.5, 300.25)));
    assert_eq!(seen[0].2, Some((-1280.5, 300.25)));
}
