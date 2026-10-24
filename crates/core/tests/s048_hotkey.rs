//! S-048 AC1 (backlog/S-048-windows-tray-shortcut.md): the shortcut string
//! parser and the default shortcut in `Config`.
//!
//! Contract (tester, from AC1; do not edit):
//! - `sbarhive_core::hotkey::parse(&str) -> Result<Hotkey, String>`, pure.
//! - `#[derive(Debug, PartialEq)] pub struct Hotkey { pub mods: u32, pub vk: u32 }`.
//! - `pub const MOD_ALT: u32 = 1; MOD_CONTROL = 2; MOD_SHIFT = 4; MOD_WIN = 8`.
//! - Tokens split on `+`, case-insensitive. Modifier names: `Ctrl`, `Alt`,
//!   `Shift`, `Win`. The last token is the key: `A`..`Z` (0x41..), `0`..`9`
//!   (0x30..), `F1`..`F24` (0x70..0x87). At least one modifier is required.
//! - `Config::shortcut: String`, default `"Ctrl+Alt+H"`, key `shortcut`.

use sbarhive_core::config::Config;
use sbarhive_core::hotkey::{Hotkey, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN, parse};

fn ok(s: &str) -> (u32, u32) {
    let h = parse(s).unwrap_or_else(|e| panic!("{s:?} must parse: {e}"));
    (h.mods, h.vk)
}

#[test]
fn ac1_modifier_constants_are_the_win32_values() {
    assert_eq!(MOD_ALT, 1);
    assert_eq!(MOD_CONTROL, 2);
    assert_eq!(MOD_SHIFT, 4);
    assert_eq!(MOD_WIN, 8);
}

#[test]
fn ac1_default_ctrl_alt_h() {
    assert_eq!(parse("Ctrl+Alt+H"), Ok(Hotkey { mods: 3, vk: 0x48 }));
}

#[test]
fn ac1_case_insensitive_shift_f9() {
    assert_eq!(ok("shift+f9"), (4, 0x78));
    assert_eq!(ok("SHIFT+F9"), (4, 0x78));
    assert_eq!(ok("ctrl+alt+h"), (3, 0x48));
}

#[test]
fn ac1_digit_key() {
    assert_eq!(ok("Ctrl+Alt+7"), (3, 0x37));
    assert_eq!(ok("Ctrl+Alt+0"), (3, 0x30));
    assert_eq!(ok("Ctrl+Alt+9"), (3, 0x39));
}

#[test]
fn ac1_letter_range_ends() {
    assert_eq!(ok("Alt+A"), (1, 0x41));
    assert_eq!(ok("Alt+Z"), (1, 0x5A));
}

#[test]
fn ac1_function_key_range() {
    assert_eq!(ok("Ctrl+F1"), (2, 0x70));
    assert_eq!(ok("Ctrl+F12"), (2, 0x7B));
    assert_eq!(ok("Ctrl+F24"), (2, 0x87));
}

#[test]
fn ac1_each_modifier_and_their_union() {
    assert_eq!(ok("Alt+H").0, 1);
    assert_eq!(ok("Ctrl+H").0, 2);
    assert_eq!(ok("Shift+H").0, 4);
    assert_eq!(ok("Win+H").0, 8);
    assert_eq!(ok("Ctrl+Shift+Win+F12"), (14, 0x7B));
    assert_eq!(ok("Alt+Ctrl+Shift+H"), (7, 0x48));
    assert_eq!(ok("Win+Shift+A"), (12, 0x41));
}

#[test]
fn ac1_no_modifier_is_an_error() {
    let e = parse("H").unwrap_err();
    assert!(!e.is_empty());
    assert!(parse("F9").is_err());
}

#[test]
fn ac1_missing_or_unknown_key_is_an_error() {
    assert!(!parse("Ctrl+Alt+").unwrap_err().is_empty());
    assert!(!parse("Ctrl+Alt+Nope").unwrap_err().is_empty());
    assert!(parse("").is_err());
    assert!(parse("Ctrl+Alt").is_err(), "a modifier is not a key");
    assert!(parse("Ctrl+Alt+HH").is_err());
    assert!(parse("Ctrl+F0").is_err());
    assert!(parse("Ctrl+F25").is_err());
}

#[test]
fn ac1_default_string_in_config() {
    assert_eq!(Config::default().shortcut, "Ctrl+Alt+H");
    assert_eq!(ok(&Config::default().shortcut), (3, 0x48));
}

#[test]
fn ac1_config_reads_the_shortcut_key() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("config.toml");
    std::fs::write(&p, "shortcut = \"Ctrl+Shift+F9\"\n").unwrap();
    let (c, w) = Config::load_from(&p);
    assert_eq!(w, None);
    assert_eq!(c.shortcut, "Ctrl+Shift+F9");
    std::fs::write(&p, "hold_ms = 500\n").unwrap();
    let (c, w) = Config::load_from(&p);
    assert_eq!(w, None);
    assert_eq!(c.shortcut, "Ctrl+Alt+H");
}

#[test]
fn ac1_constants_are_pinned_against_windows_sys_at_compile_time() {
    let s = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/hotkey.rs"))
        .expect("crates/core/src/hotkey.rs must exist");
    assert!(s.contains("windows_sys"));
    assert!(s.contains("cfg(windows)"));
    assert!(
        s.matches("const _: () = assert!(").count() >= 4,
        "four pinned constants"
    );
    for name in ["MOD_ALT", "MOD_CONTROL", "MOD_SHIFT", "MOD_WIN"] {
        assert!(
            s.lines()
                .any(|l| l.contains("assert!") && l.contains(name) && l.contains("windows_sys")),
            "{name} pinned against windows_sys"
        );
    }
}
