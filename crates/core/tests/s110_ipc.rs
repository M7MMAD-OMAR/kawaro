//! S-110 (backlog/S-110-open-at-position.md) AC2 and AC3: `Request::Open`
//! carries the pointer position.
//!
//! Contract (tester, do not edit): `Request::Open` gains
//! `at: Option<(f64, f64)>`, marked `#[serde(default)]`, serialized as
//! `"at": [X, Y]` like `Request::Drop`. The tag shape is `{"cmd":"open",..}`
//! from `ipc.rs`.

use kawaro_core::ipc::Request;
use std::path::PathBuf;

fn open(at: Option<(f64, f64)>) -> Request {
    Request::Open {
        files: vec![PathBuf::from("/a")],
        token: None,
        at,
    }
}

#[test]
fn ac3_an_old_client_line_without_at_parses_as_none() {
    let r: Request = serde_json::from_str(r#"{"cmd":"open","files":["/a"],"token":null}"#).unwrap();
    assert_eq!(r, open(None));
    let r: Request = serde_json::from_str(r#"{"cmd":"open","files":["/a"]}"#).unwrap();
    assert_eq!(r, open(None));
}

#[test]
fn ac3_a_line_with_at_parses_as_some() {
    let r: Request =
        serde_json::from_str(r#"{"cmd":"open","files":["/a"],"token":"T","at":[400.0,500.0]}"#)
            .unwrap();
    assert_eq!(
        r,
        Request::Open {
            files: vec![PathBuf::from("/a")],
            token: Some("T".into()),
            at: Some((400.0, 500.0)),
        }
    );
}

#[test]
fn ac3_a_new_client_line_for_none_parses_with_at_absent_or_null() {
    let line = serde_json::to_string(&open(None)).unwrap();
    assert_eq!(serde_json::from_str::<Request>(&line).unwrap(), open(None));
    let v: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert!(v.get("at").is_none_or(|a| a.is_null()), "{line}");
    let with_null = r#"{"cmd":"open","files":["/a"],"token":null,"at":null}"#;
    assert_eq!(
        serde_json::from_str::<Request>(with_null).unwrap(),
        open(None)
    );
}

#[test]
fn ac2_negative_and_fractional_values_survive_a_round_trip_as_a_pair() {
    let r = open(Some((-1280.5, 300.25)));
    let line = serde_json::to_string(&r).unwrap();
    assert!(!line.contains('\n'));
    let v: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(v["cmd"], "open");
    assert_eq!(v["at"], serde_json::json!([-1280.5, 300.25]), "{line}");
    assert_eq!(serde_json::from_str::<Request>(&line).unwrap(), r);
}

#[test]
fn ac3_the_at_field_is_serde_default_in_the_source() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/ipc.rs")).unwrap();
    let open_at = src.find("Open {").expect("Open variant");
    let end = src[open_at..].find("Drop {").expect("Drop variant");
    let body = &src[open_at..open_at + end];
    let at = body
        .find("at: Option<(f64, f64)>")
        .expect("at field in Open");
    let attr = body[..at]
        .lines()
        .map(str::trim)
        .rfind(|l| !l.is_empty() && !l.starts_with("//"));
    assert_eq!(
        attr,
        Some("#[serde(default)]"),
        "the field must carry #[serde(default)]: {body}"
    );
}
