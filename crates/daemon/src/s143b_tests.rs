//! S-143-B source tests for the daemon preset writer service.
//!
//! These are source only in this pass. They were never executed. They use
//! temporary paths and real channels, and they never touch the user's store.

use crate::presets::{PresetCommand, PresetError, PresetReply, PresetService};
use kawaro_core::action::ActionDef;
use kawaro_core::presets::{self, PresetStatus};
use std::path::Path;

fn new_service(path: &Path) -> PresetService {
    PresetService::new(Some(path.to_path_buf()))
}

fn webp() -> ActionDef {
    presets::builtin_action("image.to-webp")
        .expect("the built-in WebP action")
        .clone()
}

fn avif() -> ActionDef {
    presets::builtin_action("image.to-avif")
        .expect("the built-in AVIF action")
        .clone()
}

/// Saves under a fresh snapshot revision and gives the preset and revision.
fn save(
    service: &PresetService,
    action: &ActionDef,
    name: &str,
) -> (
    kawaro_core::presets::UserPreset,
    kawaro_core::presets::StoreRevision,
) {
    let expected = service.list(None).expect("list").revision;
    service
        .save_current(action, name, expected)
        .expect("save current")
}

#[test]
fn a_missing_store_lists_as_empty_and_ready() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("presets.toml");
    let service = new_service(&path);

    let list = service.list(None).expect("list");
    assert!(list.entries.is_empty());
    assert!(list.read_only.is_none());
    assert_eq!(list.revision, presets::missing_revision());
}

#[test]
fn save_then_list_is_ready_and_a_fresh_service_reads_it_back() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("presets.toml");
    let service = new_service(&path);
    let action = webp();

    let (preset, revision) = save(&service, &action, "نصف");
    assert_eq!(preset.id, "preset-1");
    assert_eq!(preset.action_id, "image.to-webp");

    let list = service
        .list(Some(String::from("image.to-webp")))
        .expect("list");
    assert_eq!(list.entries.len(), 1);
    assert_eq!(list.entries[0].preset.id, preset.id);
    assert_eq!(list.entries[0].status, PresetStatus::Ready);
    assert_eq!(list.revision, revision);

    // A fresh handle is the daemon restart: the same bytes, the same data.
    let restarted = new_service(&path);
    let list = restarted.list(None).expect("list");
    assert_eq!(list.entries.len(), 1);
    assert_eq!(
        list.entries[0].preset.values.get("quality"),
        action.params.get("quality")
    );
}

#[test]
fn a_corrupt_store_is_read_only_and_keeps_its_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("presets.toml");
    let bytes = b"not a preset store [[[";
    std::fs::write(&path, bytes).unwrap();
    let service = new_service(&path);

    let list = service.list(None).expect("list");
    assert!(matches!(list.read_only, Some(PresetError::Corrupt { .. })));

    let error = service
        .save_current(&webp(), "x", list.revision)
        .expect_err("a corrupt store is read only");
    assert!(matches!(error, PresetError::Corrupt { .. }));
    assert_eq!(std::fs::read(&path).unwrap(), bytes.to_vec());
}

#[test]
fn a_future_version_is_read_only_and_keeps_its_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("presets.toml");
    let bytes = b"version = 2\n";
    std::fs::write(&path, bytes).unwrap();
    let service = new_service(&path);

    let list = service.list(None).expect("list");
    assert!(matches!(
        list.read_only,
        Some(PresetError::UnsupportedVersion { version: 2 })
    ));

    let error = service
        .save_current(&webp(), "x", list.revision)
        .expect_err("a future store is read only");
    assert!(matches!(
        error,
        PresetError::UnsupportedVersion { version: 2 }
    ));
    assert_eq!(std::fs::read(&path).unwrap(), bytes.to_vec());
}

#[test]
fn a_stale_revision_is_refused_without_overwriting() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("presets.toml");
    let service = new_service(&path);
    let action = webp();

    let first = service.list(None).expect("list").revision;
    service
        .save_current(&action, "one", first.clone())
        .expect("first save");

    let error = service
        .save_current(&action, "two", first)
        .expect_err("the old revision is stale");
    assert!(matches!(error, PresetError::Conflict { .. }));

    let list = service.list(None).expect("list");
    assert_eq!(list.entries.len(), 1);
    assert_eq!(list.entries[0].preset.name, "one");
}

#[test]
fn a_rename_keeps_a_stale_preset_and_its_values() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("presets.toml");
    let zero = "0".repeat(64);
    let text = format!(
        "version = 1\nnext_id = 2\n\n[[presets]]\nid = \"preset-1\"\nname = \"قديم\"\naction_id = \"image.no-such-action\"\nschema_fingerprint = \"{zero}-{zero}\"\n\n[presets.values]\nquality = 50\n"
    );
    std::fs::write(&path, text).unwrap();
    let service = new_service(&path);

    let list = service.list(None).expect("list");
    assert_eq!(list.entries.len(), 1);
    assert!(matches!(
        &list.entries[0].status,
        PresetStatus::Unavailable(_)
    ));

    let (preset, _) = service
        .rename("preset-1", "جديد", list.revision)
        .expect("rename");
    assert_eq!(preset.name, "جديد");
    assert_eq!(
        preset.values.get("quality").map(|v| v.to_string()),
        Some(String::from("50"))
    );

    // The preset is still unavailable, and its values were not dropped.
    let list = service.list(None).expect("list");
    assert!(matches!(
        &list.entries[0].status,
        PresetStatus::Unavailable(_)
    ));
    assert_eq!(
        list.entries[0]
            .preset
            .values
            .get("quality")
            .map(|v| v.to_string()),
        Some(String::from("50"))
    );
}

#[test]
fn delete_removes_only_that_preset_and_never_reuses_its_id() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("presets.toml");
    let service = new_service(&path);
    let action = webp();

    let (first, _) = save(&service, &action, "one");
    let (second, _) = save(&service, &action, "two");
    assert_eq!(first.id, "preset-1");
    assert_eq!(second.id, "preset-2");

    let revision = service.list(None).expect("list").revision;
    service.delete(&first.id, revision).expect("delete");

    let (third, _) = save(&service, &action, "three");
    assert_eq!(third.id, "preset-3");
    let list = service.list(None).expect("list");
    assert_eq!(list.entries.len(), 2);
    assert!(list.entries.iter().all(|entry| entry.preset.id != first.id));
}

#[test]
fn apply_returns_the_current_action_with_params_only_for_a_ready_preset() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("presets.toml");
    let service = new_service(&path);
    let action = webp();

    let (preset, _) = save(&service, &action, "ready");
    let applied = service.apply(&preset.id, &action).expect("apply");
    assert_eq!(applied.id, "image.to-webp");
    assert_eq!(applied.params.get("quality"), action.params.get("quality"));

    // A different current action is not the one the preset was saved for.
    assert!(service.apply(&preset.id, &avif()).is_err());
}

#[test]
fn a_failed_save_keeps_the_last_good_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("presets.toml");
    let service = new_service(&path);
    let action = webp();

    let (_, revision) = save(&service, &action, "one");
    std::fs::write(&path, b"broken").unwrap();

    let error = service
        .save_current(&action, "two", revision)
        .expect_err("the store is corrupt");
    assert!(matches!(error, PresetError::Corrupt { .. }));

    // The last good in-memory list is still there, with the read-only mark.
    let list = service.list(None).expect("list");
    assert!(matches!(list.read_only, Some(PresetError::Corrupt { .. })));
    assert_eq!(list.entries.len(), 1);
    assert_eq!(list.entries[0].preset.name, "one");
}

#[test]
fn the_worker_serializes_two_saves_that_share_one_revision() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("presets.toml");
    let service = new_service(&path);
    let action = webp();

    let expected = service.list(None).expect("list").revision;
    let first = service.save_current(&action, "one", expected.clone());
    let second = service.save_current(&action, "two", expected);

    assert!(first.is_ok());
    assert!(matches!(second, Err(PresetError::Conflict { .. })));

    let list = service.list(None).expect("list");
    assert_eq!(list.entries.len(), 1);
    assert_eq!(list.entries[0].preset.name, "one");
}

#[test]
fn submit_returns_a_receiver_the_ui_can_poll() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("presets.toml");
    let service = new_service(&path);

    let receiver = service
        .submit(PresetCommand::List { action_id: None })
        .expect("submit");
    match receiver.recv().expect("reply") {
        PresetReply::List(list) => assert!(list.entries.is_empty()),
        other => panic!("unexpected reply: {other:?}"),
    }
}

#[test]
fn a_service_without_a_path_is_typed_unavailable() {
    let service = PresetService::new(None);
    assert!(matches!(
        service.list(None),
        Err(PresetError::UnavailablePath)
    ));
}
