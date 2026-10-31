//! S-096 (backlog/S-096-unsupported-file-feedback.md), daemon side that
//! needs no window: AC2 `server::open` presents the hive for an unsupported
//! selection instead of failing. The hive contents (centre text, greyed
//! cells, `REQ` cell, opener calls) need seams in `ui` that S-096b adds; they
//! are listed as not covered in the story review log.

use crate::server::open;
use crate::state::State;
use kawaro_core::config::Config;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[test]
fn ac2_open_returns_ok_for_an_unsupported_selection() {
    let state = Arc::new(Mutex::new(State::new(Config::default(), &[])));
    let result = open(&state, vec![PathBuf::from("a.xyz")], None, None);
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn ac2_open_returns_ok_for_several_unsupported_files() {
    let state = Arc::new(Mutex::new(State::new(Config::default(), &[])));
    let files = ["a.xyz", "b.abc", "README"].map(PathBuf::from).to_vec();
    assert!(open(&state, files, None, None).is_ok());
}
