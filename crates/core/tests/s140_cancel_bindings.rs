//! S-140 AC1/AC2/AC3: generated commands, not a replacement gate or Lua runtime.
//! Actual callback execution is recorded separately by the external LuaJIT fixture.
use kawaro_core::hyprland_bind::{Syntax, hyprland_bind_text};

#[test]
fn ac2_ac3_lua_release_survives_another_bind_without_consuming_keys() {
    for key in ["Alt", "Super+Z"] {
        let text = hyprland_bind_text(key, Syntax::Lua, "/usr/bin/kawaro");
        // Optional evidence export, never a prerequisite or a skipped semantic test.
        if let Some(dir) = std::env::var_os("S140_BINDING_CAPTURE_DIR") {
            std::fs::write(std::path::Path::new(&dir).join(format!("{key}.lua")), &text).unwrap();
        }
        let release = text
            .lines()
            .find(|s| s.contains("hl.bind(") && s.contains("drop --release"))
            .unwrap();
        for option in [
            "release = true",
            "non_consuming = true",
            "ignore_mods = true",
            "transparent = true",
        ] {
            assert!(
                release.contains(option),
                "{key}: release must retain {option}: {release}"
            );
        }
        let press = text
            .lines()
            .find(|s| s.contains("hl.bind(") && s.contains("drop --hold"))
            .unwrap();
        assert!(press.contains("non_consuming = true"));
        if key == "Alt" {
            assert!(press.contains("ignore_mods = true"));
        }
        assert!(
            !press.contains("transparent = true"),
            "release-only shadow protection"
        );
    }
}

#[test]
fn ac2_ac3_hyprlang_release_is_transparent_nonconsuming_and_ignores_mods() {
    for key in ["Alt", "Super+Z"] {
        let text = hyprland_bind_text(key, Syntax::Hyprlang, "/usr/bin/kawaro");
        let release = text.lines().find(|s| s.contains("drop --release")).unwrap();
        let flags = release
            .split('=')
            .next()
            .unwrap()
            .trim()
            .strip_prefix("bind")
            .unwrap();
        for flag in ['r', 'n', 'i', 't'] {
            assert!(
                flags.contains(flag),
                "{key}: missing release flag {flag}: {release}"
            );
        }
        assert_eq!(
            text.lines().filter(|s| s.starts_with("bind")).count(),
            2,
            "no global foreign Escape bind"
        );
    }
}

#[test]
fn ac1_ac2_escape_has_an_ordered_terminal_route_without_changing_generic_other_key() {
    let text = hyprland_bind_text("Alt", Syntax::Lua, "/usr/bin/kawaro");
    // Pin semantics of the existing code/state callback, not a new public API.
    let escape = text
        .find("code == 9")
        .expect("Escape (XKB code 9) needs its own held-Alt route");
    let route = &text[escape..];
    let other = route
        .find("drop --other-key")
        .expect("Escape preserves Timing chord suppression");
    let release = route
        .find("drop --release")
        .expect("Escape cancels Shown independently of Alt-up");
    assert!(
        other < release,
        "other-key must be dispatched before release"
    );
    assert!(
        !route[..release].contains("sleep "),
        "Escape cannot wait on the generic chord delay"
    );
    assert!(
        text.contains("sleep 0.05; /usr/bin/kawaro drop --other-key"),
        "ordinary keys keep the existing delayed route"
    );
    assert_eq!(
        text.matches("hl.bind(").count(),
        2,
        "observe Escape only in the existing Alt hook"
    );
    let super_z = hyprland_bind_text("Super+Z", Syntax::Lua, "/usr/bin/kawaro");
    assert!(!super_z.contains("input.keyboard.key"));
}
