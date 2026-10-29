//! S-099a A4 (manifest notes) and A5 (bundled notes).
//!
//! Test hooks (pinned in backlog/S-099-update-experience.md):
//! - `update::manifest::Notes { en, ar }`; `Manifest.notes: Notes`; empty
//!   `Notes::default()` when absent or invalid. DECISION: one invalid language
//!   empties both, so the two lists always describe the same changes.
//! - `policy::Update.notes` and `flow::Update.notes`: `Notes`.
//! - `update::whats_new::installed(Lang) -> Vec<String>`: the lines of the
//!   embedded `docs/whats-new/<CARGO_PKG_VERSION>.json`, no argument that can
//!   fetch anything.
//! - `update::whats_new::validate_dir(dir: &Path, version: &str) ->
//!   Result<Notes, String>`: reads `<dir>/<version>.json`, Err when the file
//!   is missing, is not `{"en":[..],"ar":[..]}` or breaks the A4 rule.

use std::fs;
use std::path::{Path, PathBuf};

use kawaro_core::i18n::Lang;
use kawaro_core::update::policy::{Decision, evaluate};
use kawaro_core::update::Channel;
use kawaro_core::update::UpdateError;
use kawaro_core::update::manifest::{Manifest, Notes};
use kawaro_core::update::verify::verify_manifest;
use kawaro_core::update::whats_new;
use serde_json::{Value, json};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/update/s054b")
}

fn base() -> Value {
    let bytes = fs::read(fixtures().join("manifest-stable-0.4.0.json")).unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn lines(prefix: &str, n: usize) -> Vec<String> {
    (1..=n).map(|i| format!("{prefix} line {i}")).collect()
}

fn with_notes(notes: Option<Value>) -> Vec<u8> {
    let mut v = base();
    if let Some(n) = notes {
        v["notes"] = n;
    }
    serde_json::to_vec(&v).unwrap()
}

fn parsed(notes: Option<Value>) -> Manifest {
    Manifest::parse(&with_notes(notes)).expect("the manifest stays valid")
}

fn assert_empty_and_available(notes: Option<Value>, why: &str) {
    let m = parsed(notes);
    assert_eq!(m.notes, Notes::default(), "{why}");
    match evaluate(&m, "0.3.3", Channel::Stable) {
        Ok(Decision::Available(u)) => assert_eq!(u.notes, Notes::default(), "{why}"),
        other => panic!("{why}: the update must stay available, got {other:?}"),
    }
}

// ------------------------------------------------------------ A4

#[test]
fn ac_a4_four_and_four_lines_parse_and_reach_the_update() {
    let want = Notes {
        en: lines("en", 4),
        ar: lines("ar", 4),
    };
    let m = parsed(Some(json!({"en": want.en, "ar": want.ar})));
    assert_eq!(m.notes, want);
    match evaluate(&m, "0.3.3", Channel::Stable) {
        Ok(Decision::Available(u)) => assert_eq!(u.notes, want),
        other => panic!("{other:?}"),
    }
}

#[test]
fn ac_a4_the_limits_are_inclusive_3_to_5_lines_and_80_characters() {
    for n in [3usize, 5] {
        let want = Notes {
            en: lines("en", n),
            ar: lines("ar", n),
        };
        let m = parsed(Some(json!({"en": want.en, "ar": want.ar})));
        assert_eq!(m.notes, want, "{n} lines are valid");
    }
    let mut en = lines("en", 3);
    en[1] = "x".repeat(80);
    let mut ar = lines("ar", 3);
    ar[2] = "ع".repeat(80);
    let m = parsed(Some(json!({"en": en, "ar": ar})));
    assert_eq!(m.notes, Notes { en, ar }, "80 characters are valid, counted as characters");
}

#[test]
fn ac_a4_two_lines_six_lines_a_long_line_an_empty_line_or_a_missing_key_give_empty_notes() {
    let ok_ar = lines("ar", 3);
    let mut long = lines("en", 3);
    long[0] = "x".repeat(81);
    let mut long_ar = lines("ar", 3);
    long_ar[0] = "ع".repeat(81);
    let mut empty = lines("en", 3);
    empty[2] = String::new();
    let mut two_lines_in_one = lines("en", 3);
    two_lines_in_one[1] = "first\nsecond".into();
    let cases: Vec<(Value, &str)> = vec![
        (json!({"en": lines("en", 2), "ar": ok_ar}), "2 lines"),
        (json!({"en": lines("en", 6), "ar": ok_ar}), "6 lines"),
        (json!({"en": long, "ar": ok_ar}), "81 characters"),
        (json!({"en": lines("en", 3), "ar": long_ar}), "81 Arabic characters"),
        (json!({"en": empty, "ar": ok_ar}), "an empty line"),
        (json!({"en": two_lines_in_one, "ar": ok_ar}), "a line with a line break"),
        (json!({"en": lines("en", 3)}), "a missing key"),
        (json!({"en": lines("en", 3), "ar": "text"}), "a language that is not a list"),
        (json!({"en": [1, 2, 3], "ar": ok_ar}), "lines that are not strings"),
        (json!("just text"), "notes that are not an object"),
    ];
    for (notes, why) in cases {
        assert_empty_and_available(Some(notes), why);
    }
}

#[test]
fn ac_a4_absent_notes_are_empty_and_the_update_is_available() {
    assert_empty_and_available(None, "no notes key");
}

#[test]
fn ac_a4_schema_2_is_still_unsupported_even_with_valid_notes() {
    let mut v = base();
    v["schema"] = json!(2);
    v["notes"] = json!({"en": lines("en", 3), "ar": lines("ar", 3)});
    assert_eq!(
        Manifest::parse(&serde_json::to_vec(&v).unwrap()).unwrap_err(),
        UpdateError::UnsupportedSchema
    );
}

#[test]
fn ac_a4_notes_are_covered_by_the_signature() {
    let key = fs::read_to_string(fixtures().join("key-d.pub")).unwrap();
    let sig = fs::read_to_string(fixtures().join("manifest-stable-0.4.0.json.minisig")).unwrap();
    let original = fs::read(fixtures().join("manifest-stable-0.4.0.json")).unwrap();
    assert_eq!(verify_manifest(&original, &sig, &[&key]), Ok(()));
    let forged = with_notes(Some(
        json!({"en": lines("en", 3), "ar": lines("ar", 3)}),
    ));
    assert_eq!(
        verify_manifest(&forged, &sig, &[&key]),
        Err(UpdateError::BadSignature),
        "notes added after signing must not verify"
    );
}

// ------------------------------------------------------------ A5

fn repo_notes_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/whats-new")
}

fn bundled_file() -> Value {
    let p = repo_notes_dir().join(format!("{}.json", env!("CARGO_PKG_VERSION")));
    let text = fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("{} must exist for the workspace version: {e}", p.display()));
    serde_json::from_str(&text).unwrap()
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn ac_a5_installed_returns_the_3_to_5_lines_of_the_versions_file_in_both_languages() {
    let file = bundled_file();
    let en = whats_new::installed(Lang::En);
    let ar = whats_new::installed(Lang::Ar);
    assert_eq!(en, strings(&file["en"]));
    assert_eq!(ar, strings(&file["ar"]));
    for l in [&en, &ar] {
        assert!((3..=5).contains(&l.len()), "{l:?}");
        assert!(l.iter().all(|s| !s.is_empty() && s.chars().count() <= 80));
    }
    assert_ne!(en, ar, "the two languages differ");
}

#[test]
fn ac_a5_the_gate_accepts_the_real_tree_and_every_file_in_it() {
    let notes = whats_new::validate_dir(&repo_notes_dir(), env!("CARGO_PKG_VERSION"))
        .expect("docs/whats-new/<workspace version>.json passes the A4 rule");
    assert_eq!(notes.en, whats_new::installed(Lang::En));
    for entry in fs::read_dir(repo_notes_dir()).unwrap() {
        let p = entry.unwrap().path();
        if p.extension().is_some_and(|e| e == "json") {
            let version = p.file_stem().unwrap().to_string_lossy().into_owned();
            assert!(
                whats_new::validate_dir(&repo_notes_dir(), &version).is_ok(),
                "{version}"
            );
        }
    }
}

#[test]
fn ac_a5_the_gate_fails_without_the_versions_file_or_with_a_broken_one() {
    let dir = tempfile::tempdir().unwrap();
    assert!(
        whats_new::validate_dir(dir.path(), "9.9.9").is_err(),
        "no file for the version"
    );
    let write = |text: &str| fs::write(dir.path().join("9.9.9.json"), text).unwrap();
    write("not json");
    assert!(whats_new::validate_dir(dir.path(), "9.9.9").is_err());
    write(&json!({"en": lines("en", 2), "ar": lines("ar", 3)}).to_string());
    assert!(whats_new::validate_dir(dir.path(), "9.9.9").is_err(), "2 lines");
    let mut long = lines("en", 3);
    long[0] = "x".repeat(81);
    write(&json!({"en": long, "ar": lines("ar", 3)}).to_string());
    assert!(whats_new::validate_dir(dir.path(), "9.9.9").is_err(), "81 chars");
    write(&json!({"en": lines("en", 3)}).to_string());
    assert!(whats_new::validate_dir(dir.path(), "9.9.9").is_err(), "no ar");
    write(&json!({"en": lines("en", 3), "ar": lines("ar", 5)}).to_string());
    let ok = whats_new::validate_dir(dir.path(), "9.9.9").unwrap();
    assert_eq!((ok.en.len(), ok.ar.len()), (3, 5));
}
