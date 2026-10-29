//! S-099a A1: the IPC shape of `Response::Update` with percent, size and notes.
//!
//! Test hooks (pinned in backlog/S-099-update-experience.md):
//! - `kawaro_core::update::manifest::Notes { en: Vec<String>, ar: Vec<String> }`
//!   (`Clone + Debug + Default + PartialEq + Serialize + Deserialize`).
//! - `Response::Update { status, current, latest, percent: Option<u8>,
//!   size: Option<u64>, notes: Option<Notes> }`; the three new fields use
//!   `#[serde(default, skip_serializing_if = "Option::is_none")]`.

use kawaro_core::ipc::Response;
use kawaro_core::update::manifest::Notes;
use serde::Deserialize;

/// The reply struct of a client built before S-099 (no new fields).
#[derive(Debug, Deserialize, PartialEq)]
struct OldReply {
    status: String,
    state: String,
    current: String,
    latest: Option<String>,
}

const DOWNLOADING: &str = r#"{"status":"update","state":"downloading","current":"0.4.2","latest":"0.4.3","percent":42,"size":52428800}"#;
const OLD_READY: &str =
    r#"{"status":"update","state":"ready","current":"0.3.3","latest":"0.4.0"}"#;
const OLD_OFFLINE: &str =
    r#"{"status":"update","state":"offline","current":"0.3.3","latest":null}"#;

fn lines(prefix: &str, n: usize) -> Vec<String> {
    (1..=n).map(|i| format!("{prefix} {i}")).collect()
}

#[test]
fn ac1_a_downloading_reply_is_exactly_the_story_json() {
    let reply = Response::Update {
        status: "downloading".into(),
        current: "0.4.2".into(),
        latest: Some("0.4.3".into()),
        percent: Some(42),
        size: Some(52_428_800),
        notes: None,
    };
    assert_eq!(serde_json::to_string(&reply).unwrap(), DOWNLOADING);
    assert_eq!(serde_json::from_str::<Response>(DOWNLOADING).unwrap(), reply);
}

#[test]
fn ac1_a_reply_with_the_three_new_fields_none_is_byte_for_byte_the_old_json() {
    let ready = Response::Update {
        status: "ready".into(),
        current: "0.3.3".into(),
        latest: Some("0.4.0".into()),
        percent: None,
        size: None,
        notes: None,
    };
    assert_eq!(serde_json::to_string(&ready).unwrap(), OLD_READY);
    let offline = Response::Update {
        status: "offline".into(),
        current: "0.3.3".into(),
        latest: None,
        percent: None,
        size: None,
        notes: None,
    };
    assert_eq!(serde_json::to_string(&offline).unwrap(), OLD_OFFLINE);
}

#[test]
fn ac1_a_new_client_parses_an_old_reply_with_three_nones() {
    for (line, state, latest) in [
        (OLD_READY, "ready", Some("0.4.0")),
        (OLD_OFFLINE, "offline", None),
    ] {
        match serde_json::from_str::<Response>(line).unwrap() {
            Response::Update {
                status,
                latest: l,
                percent,
                size,
                notes,
                ..
            } => {
                assert_eq!(status, state);
                assert_eq!(l.as_deref(), latest);
                assert_eq!((percent, size, notes), (None, None, None));
            }
            other => panic!("{other:?}"),
        }
    }
}

#[test]
fn ac1_an_old_client_parses_a_new_reply_and_ignores_the_new_fields() {
    let old: OldReply = serde_json::from_str(DOWNLOADING).unwrap();
    assert_eq!(
        old,
        OldReply {
            status: "update".into(),
            state: "downloading".into(),
            current: "0.4.2".into(),
            latest: Some("0.4.3".into()),
        }
    );
    // Also with notes present.
    let with_notes = Response::Update {
        status: "ready".into(),
        current: "0.4.2".into(),
        latest: Some("0.4.3".into()),
        percent: None,
        size: None,
        notes: Some(Notes {
            en: lines("en", 3),
            ar: lines("ar", 3),
        }),
    };
    let line = serde_json::to_string(&with_notes).unwrap();
    let old: OldReply = serde_json::from_str(&line).unwrap();
    assert_eq!(old.state, "ready");
}

#[test]
fn ac1_notes_travel_as_an_en_ar_object_and_round_trip() {
    let notes = Notes {
        en: lines("en", 4),
        ar: lines("ar", 5),
    };
    let reply = Response::Update {
        status: "ready".into(),
        current: "0.4.2".into(),
        latest: Some("0.4.3".into()),
        percent: None,
        size: None,
        notes: Some(notes.clone()),
    };
    let line = serde_json::to_string(&reply).unwrap();
    let value: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(value["notes"]["en"], serde_json::json!(notes.en));
    assert_eq!(value["notes"]["ar"], serde_json::json!(notes.ar));
    assert!(value.get("percent").is_none() && value.get("size").is_none());
    assert_eq!(serde_json::from_str::<Response>(&line).unwrap(), reply);
}
