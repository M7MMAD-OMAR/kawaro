//! S-120 (backlog/S-120-cli-set-and-schema.md): per-run params reach the
//! action and the wire without a second merge implementation.
//!
//! ActionDef::with_params is the one core entry point the daemon uses: it
//! clones the action, resolves the explicit option values onto its
//! action.params table and refuses an unknown key. Request::Run gains an
//! optional params table that older clients do not send, and Request::Version
//! with Response::Version advertise the capability so a new CLI can fall back
//! to the in process run instead of silently dropping the values.

use kawaro_core::action::{ActionDef, Registry};
use kawaro_core::ipc::{Request, Response};
use kawaro_core::options::OptionsError;
use std::path::PathBuf;

fn table(text: &str) -> toml::Table {
    toml::from_str(text).unwrap()
}

fn action(id: &str) -> ActionDef {
    Registry::builtin()
        .get(id)
        .unwrap_or_else(|| panic!("missing built-in action {id}"))
        .clone()
}

#[test]
fn with_params_resolves_explicit_values_onto_a_copy() {
    let base = action("image.to-webp");
    let resolved = base.with_params(&table("quality = 60")).unwrap();
    assert_eq!(
        resolved
            .params
            .get("quality")
            .and_then(toml::Value::as_integer),
        Some(60)
    );
    assert_eq!(resolved.id, base.id, "the action identity is kept");
    assert_eq!(
        base.params.get("quality").and_then(toml::Value::as_integer),
        Some(85),
        "the action itself is not changed"
    );
}

#[test]
fn with_params_clears_the_sibling_option() {
    let resolved = action("image.resize-50")
        .with_params(&table("max_side = 1280"))
        .unwrap();
    assert_eq!(
        resolved
            .params
            .get("max_side")
            .and_then(toml::Value::as_integer),
        Some(1280)
    );
    assert!(
        !resolved.params.contains_key("scale"),
        "scale is cleared: {:?}",
        resolved.params
    );
}

#[test]
fn with_params_refuses_an_unknown_key() {
    let err = action("image.to-webp")
        .with_params(&table("bogus = 1"))
        .unwrap_err();
    assert!(matches!(err, OptionsError::Unknown(_)), "{err:?}");
    assert!(err.to_string().contains("unknown option"), "{err}");
}

#[test]
fn with_params_on_an_action_without_options_refuses_values() {
    let base = action("image.to-png");
    let err = base.with_params(&table("quality = 50")).unwrap_err();
    assert!(matches!(err, OptionsError::Unknown(_)), "{err:?}");

    // An empty table takes the base params exactly as an old client would.
    let unchanged = base.with_params(&toml::Table::new()).unwrap();
    assert_eq!(unchanged.params, base.params);
}

#[test]
fn request_run_round_trips_with_and_without_params() {
    let files = vec![PathBuf::from("/home/u/photo.jpg")];
    let with = Request::Run {
        action: "image.to-webp".into(),
        files: files.clone(),
        wait: true,
        params: Some(table("quality = 60")),
    };
    let line = serde_json::to_string(&with).unwrap();
    assert!(!line.contains('\n'), "{line}");
    assert!(line.contains(r#""params":{"quality":60}"#), "{line}");
    assert_eq!(serde_json::from_str::<Request>(&line).unwrap(), with);

    let without = Request::Run {
        action: "image.to-webp".into(),
        files,
        wait: false,
        params: None,
    };
    let line = serde_json::to_string(&without).unwrap();
    assert!(!line.contains("params"), "the key is skipped: {line}");
    assert_eq!(serde_json::from_str::<Request>(&line).unwrap(), without);
}

#[test]
fn an_old_run_request_without_params_still_parses() {
    let old = r#"{"cmd":"run","action":"image.to-webp","files":["/a/x.jpg"],"wait":false}"#;
    let request: Request = serde_json::from_str(old).unwrap();
    assert_eq!(
        request,
        Request::Run {
            action: "image.to-webp".into(),
            files: vec![PathBuf::from("/a/x.jpg")],
            wait: false,
            params: None,
        }
    );
}

#[test]
fn version_request_and_response_pin_the_params_capability() {
    let line = serde_json::to_string(&Request::Version).unwrap();
    assert_eq!(line, r#"{"cmd":"version"}"#);
    assert_eq!(
        serde_json::from_str::<Request>(&line).unwrap(),
        Request::Version
    );

    let response = Response::Version {
        version: "0.5.0".into(),
        run_params: true,
    };
    let line = serde_json::to_string(&response).unwrap();
    assert!(line.contains(r#""status":"version""#), "{line}");
    assert!(line.contains(r#""run_params":true"#), "{line}");
    assert_eq!(serde_json::from_str::<Response>(&line).unwrap(), response);
}
