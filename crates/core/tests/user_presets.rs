//! S-143: named user presets (`crates/core/src/presets.rs`).
//!
//! The store keeps the option values of one action under a name the user
//! gives, in its own file next to the settings. These cases cover the
//! rules of the store: the round trip through the file, the names, the
//! ids, the mapping of the values, the fingerprint and the write
//! contract.
//!
//! These source-only cases have not been executed. The module is exported
//! for later daemon/UI integration; compilation remains unverified.

use kawaro_core::action::{ActionDef, Registry};
use kawaro_core::options::OptionsSpec;
use kawaro_core::presets::{
    self, PresetDocument, PresetError, PresetStatus, StoreRevision, UserPreset,
};

/// The built-in definition of an action.
fn action(id: &str) -> ActionDef {
    Registry::builtin()
        .get(id)
        .unwrap_or_else(|| panic!("no built-in action `{id}`"))
        .clone()
}

/// A whole number, as a TOML value.
fn int(n: i64) -> toml::Value {
    toml::Value::Integer(n)
}

/// A table of values.
fn table(pairs: &[(&str, toml::Value)]) -> toml::Table {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), value.clone()))
        .collect()
}

/// The names of the files in a folder, sorted.
fn names_in(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    names
}

#[test]
fn a_saved_preset_survives_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("presets.toml");
    let act = action("image.resize-50");
    let mut doc = PresetDocument::empty();
    let preset = doc
        .insert(&act, "نصف الحجم", &table(&[("scale", int(25))]))
        .unwrap();
    assert_eq!(preset.id, "preset-1");
    assert_eq!(preset.action_id, "image.resize-50");

    let revision = presets::save(&path, &presets::missing_revision(), &doc).unwrap();
    let (loaded, loaded_revision) = presets::load(&path).unwrap();
    assert_eq!(loaded, doc);
    assert_eq!(loaded_revision, revision);
    assert_eq!(loaded.presets.len(), 1);
    assert_eq!(loaded.presets[0].name, "نصف الحجم");
    assert_eq!(
        loaded.presets[0]
            .values
            .get("scale")
            .and_then(toml::Value::as_integer),
        Some(25)
    );
}

#[test]
fn a_missing_store_is_an_empty_store() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("presets.toml");
    let (doc, revision) = presets::load(&path).unwrap();
    assert_eq!(doc, PresetDocument::empty());
    assert_eq!(doc.version, 1);
    assert_eq!(doc.next_id, 1);
    // The revision of a missing store is not the revision of an empty file.
    // An existing empty file is a corrupt store and not a missing one, so
    // it is read through its revision and not as a document.
    let empty = dir.path().join("empty.toml");
    std::fs::write(&empty, "").unwrap();
    let empty_revision = presets::revision_of(&empty).unwrap();
    assert_ne!(revision, empty_revision);
}

#[test]
fn a_name_is_display_text_and_unique_inside_one_action() {
    let act = action("image.resize-50");
    let other = action("image.compress");
    let mut doc = PresetDocument::empty();
    doc.insert(&act, "  نصف الحجم  ", &table(&[("scale", int(25))]))
        .unwrap();
    // The same name in another action is fine.
    doc.insert(&other, "نصف الحجم", &table(&[("quality", int(60))]))
        .unwrap();
    // The same name in the same action is refused.
    let err = doc
        .insert(&act, "نصف الحجم", &table(&[("scale", int(30))]))
        .unwrap_err();
    assert!(matches!(err, PresetError::InvalidName { .. }), "{err:?}");
    // A name that looks like a path stays text.
    let preset = doc
        .insert(&act, "../../etc/passwd", &table(&[("scale", int(40))]))
        .unwrap();
    assert_eq!(preset.name, "../../etc/passwd");
    // An empty name and a name with a control character are refused.
    assert!(matches!(
        doc.insert(&act, "   ", &table(&[("scale", int(40))])),
        Err(PresetError::InvalidName { .. })
    ));
    assert!(matches!(
        doc.insert(&act, "half\u{7}size", &table(&[("scale", int(40))])),
        Err(PresetError::InvalidName { .. })
    ));
}

#[test]
fn a_deleted_id_is_never_given_out_again() {
    let act = action("image.resize-50");
    let mut doc = PresetDocument::empty();
    let first = doc
        .insert(&act, "first", &table(&[("scale", int(25))]))
        .unwrap();
    doc.remove(&first.id).unwrap();
    let second = doc
        .insert(&act, "second", &table(&[("scale", int(30))]))
        .unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(second.id, "preset-2");
    assert_eq!(doc.next_id, 3);
}

#[test]
fn the_id_counter_refuses_to_run_out() {
    let act = action("image.resize-50");
    let mut doc = PresetDocument::empty();
    doc.next_id = u64::MAX;
    let err = doc
        .insert(&act, "last", &table(&[("scale", int(25))]))
        .unwrap_err();
    assert!(matches!(err, PresetError::Exhausted { .. }), "{err:?}");
    // Nothing was added.
    assert!(doc.presets.is_empty());
    assert_eq!(doc.next_id, u64::MAX);
}

/// A spec whose field ids are not the param keys they set.
fn spec_with_maps() -> OptionsSpec {
    toml::from_str(
        r#"
kind = "form"
[[field]]
id = "size_option"
type = "int"
label = { ar = "ع", en = "Size" }
maps_to = "size"
clears = ["other"]
[[field]]
id = "other"
type = "int"
label = { ar = "ع", en = "Other" }
maps_to = "other"
clears = ["size"]
"#,
    )
    .expect("the spec parses")
}

#[test]
fn the_saved_values_are_keyed_by_field_id() {
    let spec = spec_with_maps();
    let resolved = table(&[("size", int(800))]);
    let values = presets::field_values(&spec, &resolved);
    assert_eq!(values.len(), 1);
    assert_eq!(
        values.get("size_option").and_then(toml::Value::as_integer),
        Some(800)
    );
    // The param key is not a field id here, so it is not the saved key.
    assert!(values.get("size").is_none());
}

#[test]
fn a_cleared_field_is_not_saved() {
    let act = action("image.resize-50");
    let spec = act.options.as_ref().unwrap();
    let resolved = spec
        .resolve(&act.params, &table(&[("scale", int(25))]))
        .unwrap();
    assert!(resolved.get("max_side").is_none());
    let saved = presets::canonical_values(&act, &resolved).unwrap();
    assert_eq!(
        saved.get("scale").and_then(toml::Value::as_integer),
        Some(25)
    );
    assert!(saved.get("max_side").is_none());
    assert!(saved.get("keep_metadata").is_some());
}

#[test]
fn two_options_that_clear_each_other_cannot_be_saved_together() {
    let act = action("image.resize-50");
    let err =
        presets::canonical_values(&act, &table(&[("scale", int(25)), ("max_side", int(800))]))
            .unwrap_err();
    assert!(matches!(err, PresetError::InvalidValues { .. }), "{err:?}");
}

#[test]
fn a_preset_is_ready_while_the_action_stands_still() {
    let act = action("image.to-webp");
    let mut doc = PresetDocument::empty();
    let preset = doc
        .insert(&act, "small", &table(&[("quality", int(60))]))
        .unwrap();
    assert_eq!(
        presets::validate_preset(&preset, Some(&act)),
        PresetStatus::Ready
    );
    // The settings may move the base quality: the fingerprint stays.
    let mut configured = act.clone();
    configured.params.insert("quality".into(), int(95));
    assert_eq!(
        presets::fingerprint(&configured),
        presets::fingerprint(&act)
    );
    assert_eq!(
        presets::validate_preset(&preset, Some(&configured)),
        PresetStatus::Ready
    );
}

#[test]
fn a_changed_schema_asks_for_a_review_and_a_broken_value_is_unavailable() {
    let act = action("image.resize-50");
    let mut doc = PresetDocument::empty();
    let preset = doc
        .insert(&act, "half", &table(&[("scale", int(25))]))
        .unwrap();

    // The schema of the action changed, but the value still fits.
    let mut changed = act.clone();
    changed.id = String::from("image.resize-50-other");
    changed.options.as_mut().unwrap().fields[0].max = Some(500.0);
    let mut reviewed = preset.clone();
    reviewed.schema_fingerprint = presets::fingerprint(&changed);
    assert!(
        matches!(
            presets::validate_preset(&reviewed, Some(&act)),
            PresetStatus::NeedsReview(_)
        ),
        "{:?}",
        presets::validate_preset(&reviewed, Some(&act))
    );

    // The operation of the action changed: the preset is not for this
    // action any more.
    let mut other_op = act.clone();
    other_op.id = String::from("image.resize-50-other");
    other_op.op = String::from("resize-box");
    let mut broken = preset.clone();
    broken.schema_fingerprint = presets::fingerprint(&other_op);
    assert!(
        matches!(
            presets::validate_preset(&broken, Some(&act)),
            PresetStatus::Unavailable(_)
        ),
        "{:?}",
        presets::validate_preset(&broken, Some(&act))
    );

    // A value that does not fit the current range is unavailable.
    let mut out_of_range = preset.clone();
    out_of_range.values = table(&[("scale", int(5000))]);
    assert!(
        matches!(
            presets::validate_preset(&out_of_range, Some(&act)),
            PresetStatus::Unavailable(_)
        ),
        "{:?}",
        presets::validate_preset(&out_of_range, Some(&act))
    );

    // A field the action does not declare is unavailable.
    let mut unknown = preset.clone();
    unknown.values = table(&[("nope", int(1))]);
    assert!(
        matches!(
            presets::validate_preset(&unknown, Some(&act)),
            PresetStatus::Unavailable(_)
        ),
        "{:?}",
        presets::validate_preset(&unknown, Some(&act))
    );

    // An action that is not there is unavailable.
    let mut gone = preset.clone();
    gone.action_id = String::from("image.gone");
    assert!(
        matches!(
            presets::validate_preset(&gone, None),
            PresetStatus::Unavailable(_)
        ),
        "{:?}",
        presets::validate_preset(&gone, None)
    );
}

#[test]
fn a_user_action_that_reuses_a_built_in_id_is_not_trusted() {
    let act = action("image.resize-50");
    let mut doc = PresetDocument::empty();
    let preset = doc
        .insert(&act, "half", &table(&[("scale", int(25))]))
        .unwrap();
    // A user action with the same id and another operation.
    let mut user = act.clone();
    user.op = String::from("resize-box");
    assert!(
        matches!(
            presets::validate_preset(&preset, Some(&user)),
            PresetStatus::Unavailable(_)
        ),
        "{:?}",
        presets::validate_preset(&preset, Some(&user))
    );
    // It cannot be saved under the built-in id either.
    let mut other = PresetDocument::empty();
    let err = other
        .insert(&user, "half", &table(&[("scale", int(25))]))
        .unwrap_err();
    assert!(
        matches!(err, PresetError::UnsupportedScope { .. }),
        "{err:?}"
    );
}

#[test]
fn the_images_to_pdf_action_is_out_of_the_saved_scope() {
    let act = action("image.to-pdf");
    let err = presets::canonical_values(&act, &toml::Table::new()).unwrap_err();
    assert!(
        matches!(err, PresetError::UnsupportedScope { .. }),
        "{err:?}"
    );
    let mut doc = PresetDocument::empty();
    let err = doc.insert(&act, "pages", &toml::Table::new()).unwrap_err();
    assert!(
        matches!(err, PresetError::UnsupportedScope { .. }),
        "{err:?}"
    );
}

#[test]
fn a_future_or_broken_store_is_kept_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("presets.toml");
    let doc = PresetDocument::empty();

    let future = "version = 2\nnext_id = 1\npresets = []\n";
    std::fs::write(&path, future).unwrap();
    match presets::load(&path) {
        Err(PresetError::UnsupportedVersion { version }) => assert_eq!(version, 2),
        other => panic!("expected UnsupportedVersion, got {other:?}"),
    }
    // The file is untouched, and it is not overwritten either.
    let revision = presets::revision_of(&path).unwrap();
    let err = presets::save(&path, &revision, &doc).unwrap_err();
    assert!(
        matches!(err, PresetError::UnsupportedVersion { .. }),
        "{err:?}"
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), future);

    let broken = "this is not = = toml";
    std::fs::write(&path, broken).unwrap();
    let err = presets::load(&path).unwrap_err();
    assert!(matches!(err, PresetError::Corrupt { .. }), "{err:?}");
    let revision = presets::revision_of(&path).unwrap();
    let err = presets::save(&path, &revision, &doc).unwrap_err();
    assert!(matches!(err, PresetError::Corrupt { .. }), "{err:?}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), broken);
}

#[test]
fn a_save_against_an_old_revision_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("presets.toml");
    let act = action("image.resize-50");
    let mut doc = PresetDocument::empty();
    doc.insert(&act, "half", &table(&[("scale", int(25))]))
        .unwrap();
    let revision = presets::save(&path, &presets::missing_revision(), &doc).unwrap();

    // Another writer changed the file.
    let other = "version = 1\nnext_id = 9\npresets = []\n";
    std::fs::write(&path, other).unwrap();
    let err = presets::save(&path, &revision, &doc).unwrap_err();
    assert!(matches!(err, PresetError::Conflict { .. }), "{err:?}");
    // The other file is kept.
    assert_eq!(std::fs::read_to_string(&path).unwrap(), other);

    // A file that was taken away is a conflict as well.
    std::fs::remove_file(&path).unwrap();
    let err = presets::save(&path, &revision, &doc).unwrap_err();
    assert!(matches!(err, PresetError::Conflict { .. }), "{err:?}");
    assert!(!path.exists());
}

#[test]
fn the_ordinary_settings_are_not_touched() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.toml");
    let store = dir.path().join("presets.toml");
    let settings = "language = \"ar\"\nquality = \"max\"\n";
    std::fs::write(&config, settings).unwrap();

    let act = action("image.resize-50");
    let mut doc = PresetDocument::empty();
    doc.insert(&act, "half", &table(&[("scale", int(25))]))
        .unwrap();
    presets::save(&store, &presets::missing_revision(), &doc).unwrap();

    // The settings file is the same, and no temporary file is left behind.
    assert_eq!(std::fs::read_to_string(&config).unwrap(), settings);
    assert_eq!(
        names_in(dir.path()),
        vec![String::from("config.toml"), String::from("presets.toml")]
    );
}

#[test]
fn a_renamed_preset_keeps_its_id_and_its_values() {
    let act = action("image.resize-50");
    let mut doc = PresetDocument::empty();
    let preset = doc
        .insert(&act, "half", &table(&[("scale", int(25))]))
        .unwrap();
    doc.rename(&preset.id, "نصف").unwrap();
    let renamed = doc.get(&preset.id).unwrap();
    assert_eq!(renamed.id, preset.id);
    assert_eq!(renamed.name, "نصف");
    assert_eq!(renamed.values, preset.values);
    // The new name is taken now.
    let err = doc
        .insert(&act, "نصف", &table(&[("scale", int(30))]))
        .unwrap_err();
    assert!(matches!(err, PresetError::InvalidName { .. }), "{err:?}");
}

#[test]
fn a_preset_is_applied_only_when_it_is_ready() {
    let act = action("image.resize-50");
    let mut doc = PresetDocument::empty();
    let preset = doc
        .insert(&act, "half", &table(&[("scale", int(25))]))
        .unwrap();
    let params = presets::apply_preset(&preset, &act).unwrap();
    assert_eq!(
        params.get("scale").and_then(toml::Value::as_integer),
        Some(25)
    );

    // A preset that asks for a review is not applied on trust.
    let mut changed = act.clone();
    changed.id = String::from("image.resize-50-other");
    changed.options.as_mut().unwrap().fields[0].max = Some(500.0);
    let mut reviewed = preset.clone();
    reviewed.schema_fingerprint = presets::fingerprint(&changed);
    assert!(presets::apply_preset(&reviewed, &act).is_err());
}

#[test]
fn a_store_with_a_broken_id_is_not_read() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("presets.toml");
    let fingerprint = format!("{}-{}", "a".repeat(64), "b".repeat(64));
    // The counter is not above the ids in the store.
    let text = format!(
        "version = 1\nnext_id = 1\n\n[[presets]]\nid = \"preset-4\"\nname = \"x\"\naction_id = \"image.resize-50\"\nschema_fingerprint = \"{fingerprint}\"\n\n[presets.values]\nscale = 25\n"
    );
    std::fs::write(&path, &text).unwrap();
    let err = presets::load(&path).unwrap_err();
    assert!(matches!(err, PresetError::Corrupt { .. }), "{err:?}");
    // The file stays as it is.
    assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
}

#[test]
fn a_preset_that_was_never_saved_has_no_path_of_its_own() {
    // The name is text. It never reaches a file name.
    let act = action("image.resize-50");
    let mut doc = PresetDocument::empty();
    let preset = doc
        .insert(&act, "../escape", &table(&[("scale", int(25))]))
        .unwrap();
    assert_eq!(preset.name, "../escape");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("presets.toml");
    presets::save(&path, &presets::missing_revision(), &doc).unwrap();
    assert_eq!(names_in(dir.path()), vec![String::from("presets.toml")]);
    let (loaded, _) = presets::load(&path).unwrap();
    assert_eq!(loaded.presets[0].name, "../escape");
    assert_eq!(loaded.presets[0].id, "preset-1");
}

/// The store never keeps a path, an output name or a command.
#[test]
fn the_saved_values_hold_no_path_and_no_command() {
    let act = action("image.resize-50");
    let spec = act.options.as_ref().unwrap();
    let resolved = spec
        .resolve(&act.params, &table(&[("scale", int(25))]))
        .unwrap();
    let saved = presets::canonical_values(&act, &resolved).unwrap();
    for (key, value) in &saved {
        assert!(matches!(
            key.as_str(),
            "scale" | "max_side" | "keep_metadata"
        ));
        assert!(
            matches!(value, toml::Value::Integer(_) | toml::Value::Boolean(_)),
            "{key} holds {value:?}"
        );
    }
}

/// The revision of a store that was never written is not the revision of
/// the file that was written.
#[test]
fn the_revision_follows_the_content_of_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("presets.toml");
    assert_eq!(
        presets::revision_of(&path).unwrap(),
        presets::missing_revision()
    );
    std::fs::write(&path, "version = 1\nnext_id = 1\npresets = []\n").unwrap();
    let first: StoreRevision = presets::revision_of(&path).unwrap();
    std::fs::write(&path, "version = 1\nnext_id = 2\npresets = []\n").unwrap();
    let second = presets::revision_of(&path).unwrap();
    assert_ne!(first, second);
    let preset = UserPreset {
        id: String::from("preset-1"),
        name: String::from("half"),
        action_id: String::from("image.resize-50"),
        schema_fingerprint: String::from("x"),
        values: table(&[("scale", int(25))]),
    };
    // A fingerprint this build cannot read makes the preset unavailable.
    assert!(matches!(
        presets::validate_preset(&preset, None),
        PresetStatus::Unavailable(_)
    ));
}

#[test]
fn an_existing_empty_store_is_preserved_as_corrupt() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("presets.toml");
    std::fs::write(&path, b"").unwrap();
    assert!(matches!(
        presets::load(&path),
        Err(PresetError::Corrupt { .. })
    ));
    let revision = presets::revision_of(&path).unwrap();
    assert!(presets::save(&path, &revision, &PresetDocument::empty()).is_err());
    assert!(std::fs::read(&path).unwrap().is_empty());
}

#[test]
fn a_missing_current_action_never_marks_a_valid_preset_ready() {
    let act = action("image.resize-50");
    let mut doc = PresetDocument::empty();
    let preset = doc
        .insert(&act, "half", &table(&[("scale", int(25))]))
        .unwrap();
    assert!(matches!(
        presets::validate_preset(&preset, None),
        PresetStatus::Unavailable(_)
    ));
}

#[test]
fn unavailable_saved_payload_does_not_block_other_preset_changes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("presets.toml");
    let act = action("image.resize-50");
    let mut old = PresetDocument::empty();
    let orphan = old
        .insert(&act, "old", &table(&[("scale", int(25))]))
        .unwrap();
    old.presets[0].action_id = "image.removed-in-an-upgrade".to_string();
    std::fs::write(&path, toml::to_string_pretty(&old).unwrap()).unwrap();
    let (mut doc, revision) = presets::load(&path).unwrap();
    doc.rename(&orphan.id, "retained old preset").unwrap();
    doc.insert(&act, "current", &table(&[("scale", int(50))]))
        .unwrap();
    presets::save(&path, &revision, &doc).unwrap();
    let (loaded, _) = presets::load(&path).unwrap();
    assert_eq!(loaded.presets.len(), 2);
    assert_eq!(loaded.presets[0].values, old.presets[0].values);
    assert_eq!(loaded.presets[0].action_id, old.presets[0].action_id);
}

#[test]
fn deleting_every_preset_does_not_allow_counter_rollback() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("presets.toml");
    let act = action("image.resize-50");
    let mut doc = PresetDocument::empty();
    let preset = doc
        .insert(&act, "half", &table(&[("scale", int(25))]))
        .unwrap();
    let revision = presets::save(&path, &presets::missing_revision(), &doc).unwrap();
    doc.remove(&preset.id).unwrap();
    doc.next_id = 1;
    assert!(matches!(
        presets::save(&path, &revision, &doc),
        Err(PresetError::InvalidValues { .. })
    ));
}

#[test]
fn a_store_over_the_admission_limit_is_not_loaded() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("presets.toml");
    std::fs::write(&path, vec![b' '; presets::MAX_STORE_BYTES + 1]).unwrap();
    assert!(matches!(
        presets::load(&path),
        Err(PresetError::Corrupt { .. })
    ));
    assert_eq!(
        std::fs::metadata(&path).unwrap().len(),
        presets::MAX_STORE_BYTES as u64 + 1
    );
}
