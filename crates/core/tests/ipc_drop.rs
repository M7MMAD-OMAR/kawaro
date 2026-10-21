//! S-014a AC4: the `Drop` request on the daemon socket.
//! S-019d, manager decision 1 (backlog/S-019-hive-at-pointer.md): `drop`
//! may carry the pointer position, `sbarhive drop --at X,Y`, in global
//! logical coordinates as `hyprctl cursorpos` prints them.
//!
//! Contract (tester, from decision 1; do not edit): `Request::Drop` gains a
//! field `at`, serialized as `"at": [X, Y]` (two JSON numbers), `None`
//! absent or null, and `#[serde(default)]` so a client without it still
//! parses. The field's Rust type is not fixed here beyond that JSON form.

use sbarhive_core::ipc::Request;

#[test]
fn ac4_drop_request_is_cmd_drop_with_the_token() {
    let r = Request::Drop {
        token: Some("T".into()),
        at: None,
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
    };
    let line = serde_json::to_string(&r).unwrap();
    assert_eq!(serde_json::from_str::<Request>(&line).unwrap(), r);
    assert_eq!(
        serde_json::from_str::<Request>(r#"{"cmd":"drop"}"#).unwrap(),
        Request::Drop {
            token: None,
            at: None,
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
