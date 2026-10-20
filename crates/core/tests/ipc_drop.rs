//! S-014a AC4: the `Drop` request on the daemon socket.

use sbarhive_core::ipc::Request;

#[test]
fn ac4_drop_request_is_cmd_drop_with_the_token() {
    let r = Request::Drop {
        token: Some("T".into()),
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
    let r = Request::Drop { token: None };
    let line = serde_json::to_string(&r).unwrap();
    assert_eq!(serde_json::from_str::<Request>(&line).unwrap(), r);
    assert_eq!(
        serde_json::from_str::<Request>(r#"{"cmd":"drop"}"#).unwrap(),
        Request::Drop { token: None }
    );
}
