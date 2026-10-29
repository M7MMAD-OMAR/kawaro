//! S-101b B2 (backlog/S-101-update-ui-defects.md): the reopen decision table.
//! The daemon is a binary crate and `reopen.rs` calls `crate::`, so the pure
//! part moves to `crates/daemon/src/reopen_logic.rs` (std and `kawaro_core`
//! only, never `crate::`), included here by path.
//!
//! Contract (tester, do not edit):
//! - `Marker { from: String, to: String, at: u64 }` with
//!   `Marker::to_text(&self) -> String` and `Marker::parse(&str) ->
//!   Option<Marker>` (round trip; `None` for corrupt text).
//! - `Reopen { Updated(String), Failed(String) }`, `PartialEq + Debug`.
//! - `reopen_decision(marker: Option<&Marker>, result: Option<&str>,
//!   installed: &str, now: u64) -> (Option<Reopen>, bool)` where the bool is
//!   "delete the marker". `result` is the text of `update-result.json`
//!   (a JSON object with `"status": "failed"` for a failed install).
//!   Age is `now - at` in seconds; more than 3600 is stale.
//! - `read_marker_or_delete(text: Option<&str>) -> (Option<Marker>, bool)`:
//!   corrupt or absent text gives `(None, true)` for corrupt, `(None, false)`
//!   for absent.

#[allow(dead_code)]
#[path = "../src/reopen_logic.rs"]
mod reopen_logic;

use reopen_logic::{Marker, Reopen, read_marker_or_delete, reopen_decision};

fn marker(at: u64) -> Marker {
    Marker {
        from: "0.4.2".into(),
        to: "0.4.3".into(),
        at,
    }
}
const FAILED: &str = r#"{"status":"failed","version":"0.4.3"}"#;

#[test]
fn ac_b2_installed_equals_to_is_updated() {
    let m = marker(1000);
    let (d, del) = reopen_decision(Some(&m), None, "0.4.3", 1010);
    assert_eq!(d, Some(Reopen::Updated("0.4.3".into())));
    assert!(del, "a used marker is deleted");
}

#[test]
fn ac_b2_installed_equals_from_with_a_failed_result_is_failed() {
    let m = marker(1000);
    let (d, _) = reopen_decision(Some(&m), Some(FAILED), "0.4.2", 1010);
    assert_eq!(d, Some(Reopen::Failed("0.4.3".into())));
}

#[test]
fn ac_b2_installed_equals_from_without_a_failed_result_is_none() {
    let m = marker(1000);
    let (d, _) = reopen_decision(Some(&m), None, "0.4.2", 1010);
    assert_eq!(d, None);
}

#[test]
fn ac_b2_stale_other_version_and_missing_marker_give_none_and_delete() {
    let m = marker(1000);
    let (d, del) = reopen_decision(Some(&m), Some(FAILED), "0.4.2", 1000 + 3601);
    assert_eq!((d, del), (None, true), "age 3601 s");
    let (d, del) = reopen_decision(Some(&m), None, "0.9.9", 1010);
    assert_eq!((d, del), (None, true), "other version");
    let (d, del) = reopen_decision(None, Some(FAILED), "0.4.2", 1010);
    assert_eq!((d, del), (None, false), "no marker, nothing to delete");
}

#[test]
fn ac_b2_marker_round_trip_and_corrupt_text_is_deleted() {
    let m = marker(1234);
    assert_eq!(Marker::parse(&m.to_text()), Some(m.clone()));
    assert_eq!(read_marker_or_delete(Some(&m.to_text())), (Some(m), false));
    assert_eq!(read_marker_or_delete(Some("{not json")), (None, true));
    assert_eq!(read_marker_or_delete(None), (None, false));
}
