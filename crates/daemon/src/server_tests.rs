//! S-046 AC8 (backlog/S-046-windows-hive-window.md): `server::open` refuses
//! a selection no action fits, before any window is built.

use crate::server::open;
use crate::state::State;
use kawaro_core::config::Config;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[test]
fn ac8_open_fails_when_no_action_matches_the_file() {
    let state = Arc::new(Mutex::new(State::new(Config::default(), &[])));
    let result = open(&state, vec![PathBuf::from("unknown.zzz")], None);
    assert!(result.is_err(), "{result:?}");
    assert!(state.lock().unwrap().pending.is_none());
}
