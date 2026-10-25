//! S-014a AC4: the `Drop` request on the daemon socket.
//! S-019d, manager decision 1 (backlog/S-019-hive-at-pointer.md): `drop`
//! may carry the pointer position, `kawaro drop --at X,Y`, in global
//! logical coordinates as `hyprctl cursorpos` prints them.
//!
//! Contract (tester, from decision 1; do not edit): `Request::Drop` gains a
//! field `at`, serialized as `"at": [X, Y]` (two JSON numbers), `None`
//! absent or null, and `#[serde(default)]` so a client without it still
//! parses. The field's Rust type is not fixed here beyond that JSON form.
//!
//! S-019e (AC11, amended 2026-10-21): `kawaro drop --hold` is the press of
//! a held key and `kawaro drop --release` its release.
//! Contract (tester, from AC11; do not edit): `Request::Drop` gains
//! `hold: bool`, `"hold": true` in JSON, with `#[serde(default)]` so a line
//! without it (an older client) parses as `false`. A new unit variant
//! `Request::DropRelease` is `{"cmd":"drop-release"}` (the enum's kebab case
//! tag), with no other field.

use kawaro_core::ipc::Request;

#[test]
fn ac4_drop_request_is_cmd_drop_with_the_token() {
    let r = Request::Drop {
        token: Some("T".into()),
        at: None,
        hold: false,
    };
    let line = serde_json::to_string(&r).unwrap();
    assert!(!line.contains('\n'));
    let v: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(v["cmd"], "drop");
    assert_eq!(v["token"], "T");
    assert!(v.get("files").is_none(), "drop carries no files: {line}");
    assert_eq!(serde_json::from_str::<Request>(&line).unwrap(), r);
}

#[test]
fn ac4_drop_without_a_token_round_trips_and_the_token_may_be_omitted() {
    let r = Request::Drop {
        token: None,
        at: None,
        hold: false,
    };
    let line = serde_json::to_string(&r).unwrap();
    assert_eq!(serde_json::from_str::<Request>(&line).unwrap(), r);
    assert_eq!(
        serde_json::from_str::<Request>(r#"{"cmd":"drop"}"#).unwrap(),
        Request::Drop {
            token: None,
            at: None,
            hold: false,
        }
    );
}

/// Parses a line as a `Request` and serializes it back to JSON.
fn round_trip(line: &str) -> serde_json::Value {
    let r: Request = serde_json::from_str(line).unwrap_or_else(|e| panic!("{line}: {e}"));
    let back = serde_json::to_string(&r).unwrap();
    assert!(!back.contains('\n'));
    let again: Request = serde_json::from_str(&back).unwrap();
    assert_eq!(again, r, "{back}");
    serde_json::from_str(&back).unwrap()
}

#[test]
fn ac8_s019_decision1_drop_carries_the_pointer_position_as_at_x_y() {
    let v = round_trip(r#"{"cmd":"drop","token":"T","at":[1198.0,1800.0]}"#);
    assert_eq!(v["cmd"], "drop");
    assert_eq!(v["token"], "T");
    assert_eq!(v["at"], serde_json::json!([1198.0, 1800.0]), "{v}");
    assert!(v.get("files").is_none(), "drop carries no files: {v}");
}

#[test]
fn ac8_s019_decision1_at_keeps_fractions_and_negative_global_coordinates() {
    // Global logical coordinates: an output left of or above the origin has
    // negative ones, and a fractional scale gives fractions.
    let v = round_trip(r#"{"cmd":"drop","at":[-1280.5,300.25]}"#);
    assert_eq!(v["at"], serde_json::json!([-1280.5, 300.25]), "{v}");
}

#[test]
fn ac8_s019_decision1_drop_without_at_still_parses_and_sends_no_position() {
    for line in [
        r#"{"cmd":"drop"}"#,
        r#"{"cmd":"drop","token":"T"}"#,
        r#"{"cmd":"drop","at":null}"#,
    ] {
        let v = round_trip(line);
        assert!(v.get("at").is_none_or(|a| a.is_null()), "{line}: {v}");
    }
    assert_eq!(
        serde_json::from_str::<Request>(r#"{"cmd":"drop"}"#).unwrap(),
        Request::Drop {
            token: None,
            at: None,
            hold: false,
        }
    );
}

#[test]
fn ac8_s019_decision1_at_is_two_numbers() {
    for bad in [
        r#"{"cmd":"drop","at":[1]}"#,
        r#"{"cmd":"drop","at":[1,2,3]}"#,
        r#"{"cmd":"drop","at":"1,2"}"#,
    ] {
        assert!(serde_json::from_str::<Request>(bad).is_err(), "{bad}");
    }
}

#[test]
fn ac11_s019e_drop_hold_carries_hold_true() {
    let r = Request::Drop {
        token: Some("T".into()),
        at: Some((400.0, 500.0)),
        hold: true,
    };
    let line = serde_json::to_string(&r).unwrap();
    assert!(!line.contains('\n'));
    let v: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(v["cmd"], "drop", "{line}");
    assert_eq!(v["hold"], true, "{line}");
    assert_eq!(v["token"], "T", "{line}");
    assert_eq!(v["at"], serde_json::json!([400.0, 500.0]), "{line}");
    assert_eq!(serde_json::from_str::<Request>(&line).unwrap(), r);
    let v = round_trip(r#"{"cmd":"drop","hold":true}"#);
    assert_eq!(v["hold"], true, "{v}");
}

#[test]
fn ac11_s019e_an_older_client_without_hold_is_an_immediate_drop() {
    for (line, token, at) in [
        (r#"{"cmd":"drop"}"#, None, None),
        (r#"{"cmd":"drop","token":"T"}"#, Some("T"), None),
        (r#"{"cmd":"drop","at":[1.0,2.0]}"#, None, Some((1.0, 2.0))),
    ] {
        assert_eq!(
            serde_json::from_str::<Request>(line).unwrap(),
            Request::Drop {
                token: token.map(String::from),
                at,
                hold: false,
            },
            "{line}"
        );
    }
    let v = round_trip(r#"{"cmd":"drop","hold":false}"#);
    assert!(
        v.get("hold")
            .is_none_or(|h| *h == serde_json::Value::Bool(false)),
        "{v}"
    );
}

#[test]
fn ac11_s019e_drop_release_is_its_own_request() {
    let line = serde_json::to_string(&Request::DropRelease).unwrap();
    let v: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(v, serde_json::json!({"cmd": "drop-release"}), "{line}");
    assert_eq!(
        serde_json::from_str::<Request>(r#"{"cmd":"drop-release"}"#).unwrap(),
        Request::DropRelease
    );
    assert_ne!(
        serde_json::from_str::<Request>(r#"{"cmd":"drop-release"}"#).unwrap(),
        Request::Drop {
            token: None,
            at: None,
            hold: false,
        }
    );
}
