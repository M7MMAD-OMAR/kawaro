//! S-131 AC1: the settings activation token on the wire and off the disk.
//! Contract pinned by this test (tester): Request::Settings gains
//! token: Option<String> with #[serde(default, skip_serializing_if = "Option::is_none")].
//! The token is never part of config.toml.

use kawaro_core::config::Config;
use kawaro_core::ipc::Request;

#[test]
fn settings_without_a_token_omits_the_key_and_round_trips() {
    let request = Request::Settings { token: None };
    let line = serde_json::to_string(&request).unwrap();
    assert!(line.contains("\"cmd\":\"settings\""), "{line}");
    assert!(!line.contains("token"), "no token key: {line}");
    assert_eq!(serde_json::from_str::<Request>(&line).unwrap(), request);
}

#[test]
fn settings_with_a_token_round_trips() {
    let request = Request::Settings {
        token: Some("tok-123".to_string()),
    };
    let line = serde_json::to_string(&request).unwrap();
    assert!(line.contains("\"token\":\"tok-123\""), "{line}");
    assert_eq!(serde_json::from_str::<Request>(&line).unwrap(), request);
}

#[test]
fn an_old_settings_request_without_the_key_still_parses() {
    let old = r#"{"cmd":"settings"}"#;
    let request: Request = serde_json::from_str(old).unwrap();
    assert_eq!(request, Request::Settings { token: None });
}

#[test]
fn the_token_is_never_written_to_config_toml() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    Config::default().save_to(&path).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(
        !text.to_lowercase().contains("token"),
        "config.toml must not carry the activation token: {text}"
    );
    let table: toml::Table = toml::from_str(&text).unwrap();
    assert!(
        !table.keys().any(|key| key.to_lowercase().contains("token")),
        "no token key in the saved settings: {:?}",
        table.keys().collect::<Vec<_>>()
    );
}
