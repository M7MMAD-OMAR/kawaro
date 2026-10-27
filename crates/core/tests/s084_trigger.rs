//! S-084 (backlog/S-084-windows-drag-trigger.md) AC4 and AC6 with the
//! manager decisions D1, D3 and D6: the per-platform `drop_key` default and
//! its read-time resolution, the capture rules of D1 with their bilingual
//! refusal texts, and the S-019e long press gate the key-hold trigger reuses
//! (AC3). Pure logic, so it runs on Linux.

use kawaro_core::config::Config;
use kawaro_core::drop_key::{self, CaptureRefusal, DropTrigger};
use kawaro_core::hold::{Effect, Gate, Signal};
use kawaro_core::i18n::Lang;
use std::time::{Duration, Instant};

/// Loads `text` as `config.toml` and guards D1's "no migration write": a
/// load never rewrites the file.
fn load(text: &str) -> Config {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("config.toml");
    std::fs::write(&p, text).unwrap();
    let before = std::fs::read_to_string(&p).unwrap();
    let (c, w) = Config::load_from(&p);
    assert_eq!(w, None, "the test file must load clean");
    assert_eq!(
        before,
        std::fs::read_to_string(&p).unwrap(),
        "a load must never rewrite the file (D1: no migration write)"
    );
    c
}

#[test]
fn ac4_a_fresh_windows_config_uses_the_right_drag_trigger_and_never_alt() {
    // D1 and D3: a missing `drop_key` resolves to the manager decision's
    // trigger on Windows and to Alt where Alt was measured.
    assert_eq!(drop_key::resolve(None, true), DropTrigger::RightDrag);
    assert_ne!(
        drop_key::resolve(None, true),
        DropTrigger::Key("Alt".into()),
        "Alt is never the Windows default"
    );
    assert_eq!(drop_key::resolve(None, false), DropTrigger::Key("Alt".into()));
}

#[test]
fn ac4_a_stale_alt_drop_key_re_resolves_to_the_windows_default_on_read() {
    // D1: a stored Alt written before this story never resurrects the bug.
    assert_eq!(drop_key::resolve(Some("Alt"), true), DropTrigger::RightDrag);
    assert_eq!(drop_key::resolve(Some("alt"), true), DropTrigger::RightDrag);
    // A chosen chord and the measured Linux default stay as chosen.
    let chosen = DropTrigger::Key("Super+Z".into());
    assert_eq!(drop_key::resolve(Some("Super+Z"), true), chosen);
    assert_eq!(drop_key::resolve(Some("Super+Z"), false), chosen);
}

#[test]
fn ac4_a_stored_drop_key_is_read_from_the_file() {
    let c = load("drop_key = \"Alt\"\n");
    assert_eq!(c.drop_key.as_deref(), Some("Alt"), "the key is read, not dropped");
    assert_eq!(
        drop_key::resolve(c.drop_key.as_deref(), true),
        DropTrigger::RightDrag
    );
    let missing = load("hold_ms = 500\n");
    assert_eq!(missing.drop_key, None, "a missing key stays missing");
    assert_eq!(Config::default().drop_key, None, "a fresh config has no key");
}

#[test]
fn ac4_the_picker_offers_only_options_that_hold_no_explorer_drag_modifier() {
    // D3, D6 and D7: the measured default is the only preset on Windows, and
    // every capture the picker can accept holds none of Alt, Ctrl and Shift
    // while the drag is in progress (AC2a: the invariant covers every
    // shipped trigger).
    assert_eq!(drop_key::preset_options(true), vec![DropTrigger::RightDrag]);
    assert_eq!(
        drop_key::preset_options(false),
        vec![DropTrigger::Key("Alt".into())]
    );
    for chord in ["Super", "Super+Z", "Super+F9", "Win+F9"] {
        let ok = drop_key::check_capture(chord, true)
            .unwrap_or_else(|e| panic!("{chord} must be accepted on Windows: {e:?}"));
        for token in ok.split('+') {
            assert!(
                !["Alt", "Ctrl", "Shift"]
                    .iter()
                    .any(|m| token.eq_ignore_ascii_case(m)),
                "{chord} resolved to {ok}, which holds an Explorer drag modifier"
            );
        }
    }
}

#[test]
fn ac4_capture_refuses_the_explorer_drag_modifiers_on_windows() {
    // D1: bare Alt, Ctrl or Shift and every chord that would hold one during
    // the drag are refused on Windows, each is an Explorer drag modifier
    // there. The refusals cover the single-modifier choices and the chords
    // whose modifiers would all be held during the drag.
    for chord in [
        "Alt",
        "Ctrl",
        "Shift",
        "alt",
        "Ctrl+Alt+H",
        "Alt+K",
        "Ctrl+Shift",
        "Ctrl+Super+Z",
        "Shift+F9",
        "Super+Shift+Z",
    ] {
        assert_eq!(
            drop_key::check_capture(chord, true),
            Err(CaptureRefusal::DragModifier),
            "{chord}"
        );
    }
    // Where Alt was measured, S-076 decision 5 still accepts these.
    assert_eq!(drop_key::check_capture("Alt", false), Ok("Alt".to_string()));
    assert_eq!(
        drop_key::check_capture("Ctrl+Alt+H", false),
        Ok("Ctrl+Alt+H".to_string())
    );
    assert_eq!(
        drop_key::check_capture("Ctrl+Shift", false),
        Ok("Ctrl+Shift".to_string())
    );
    // Captures normalise case and keep the order.
    assert_eq!(
        drop_key::check_capture("shift+f9", false),
        Ok("Shift+F9".to_string())
    );
    assert_eq!(
        drop_key::check_capture("super+z", true),
        Ok("Super+Z".to_string())
    );
    assert_eq!(drop_key::check_capture("Win+F9", true), Ok("Super+F9".to_string()));
}

#[test]
fn ac4_capture_refuses_a_plain_letter_and_unsupported_input() {
    // S-076 decision 5 (both platforms): a plain key without a modifier and
    // unsupported input are refused and replace nothing.
    for chord in ["A", "k", "A+B", "F5"] {
        for windows in [false, true] {
            assert_eq!(
                drop_key::check_capture(chord, windows),
                Err(CaptureRefusal::PlainKey),
                "{chord} windows {windows}"
            );
        }
    }
    for chord in ["", "Foo+Z", "Super+", "+Z", "Super+Esc", "Super+Z+Q"] {
        for windows in [false, true] {
            assert_eq!(
                drop_key::check_capture(chord, windows),
                Err(CaptureRefusal::Unsupported),
                "{chord} windows {windows}"
            );
        }
    }
}

#[test]
fn ac4_the_refusal_text_names_the_windows_reason_in_both_languages() {
    // D1: the refusal text names the Windows reason in both languages.
    let en = drop_key::refusal_text(CaptureRefusal::DragModifier, Lang::En);
    let ar = drop_key::refusal_text(CaptureRefusal::DragModifier, Lang::Ar);
    assert!(en.contains("Windows"), "English text names Windows: {en}");
    assert!(en.contains("Explorer"), "English text names Explorer: {en}");
    assert!(ar.contains("ويندوز"), "Arabic text names Windows: {ar}");
    assert!(ar.contains("Explorer"), "Arabic text names Explorer: {ar}");
    assert_ne!(en, ar);
    // Every refusal has a text in both languages.
    for r in [
        CaptureRefusal::PlainKey,
        CaptureRefusal::DragModifier,
        CaptureRefusal::Unsupported,
    ] {
        for lang in [Lang::En, Lang::Ar] {
            assert!(!drop_key::refusal_text(r, lang).is_empty());
        }
    }
}

#[test]
fn ac3_the_key_hold_trigger_reuses_the_s019e_gate_with_hold_ms() {
    // D8: the key-hold trigger (the internal chord, any captured key) shows
    // through the long press gate with `hold_ms`; the release closes it.
    let mut gate = Gate::new(|| 400);
    let press = Instant::now();
    assert_eq!(
        gate.signal(Signal::Hold { at: Some((320.0, 240.0)) }, press),
        None,
        "no show at the press"
    );
    assert_eq!(
        gate.tick(press + Duration::from_millis(399)),
        None,
        "no show before hold_ms"
    );
    assert_eq!(
        gate.tick(press + Duration::from_millis(400)),
        Some(Effect::Show {
            at: Some((320.0, 240.0))
        }),
        "the show once hold_ms passed with the key held"
    );
    assert_eq!(
        gate.signal(Signal::Release, press + Duration::from_millis(500)),
        Some(Effect::Release),
        "the release passes to the hive the gate showed"
    );
}
