//! S-014e1 (backlog/S-014-drop-hive.md, AC13, AC17, AC19 and the manager's
//! answers 4 and 5 of 2026-10-22): the pure part of the daemon's
//! GlobalShortcuts portal client, `crate::portal`. Its D-Bus side runs on a
//! live session bus only and is covered by tests/drop/portal.sh.

use crate::portal::{self, Reason, Reload};
use sbarhive_core::hold::{self, Signal};
use sbarhive_core::i18n::Lang;

const OURS: &str = "/org/freedesktop/portal/desktop/session/1_42/sbarhive";
const OTHER: &str = "/org/freedesktop/portal/desktop/session/1_42/other";

fn arabic(c: char) -> bool {
    ('\u{0600}'..='\u{06FF}').contains(&c)
}

#[test]
fn ac13_registers_as_the_desktop_file_id() {
    // packaging/io.github.m7mmad_omar.SbarHive.desktop
    assert_eq!(portal::APP_ID, "io.github.m7mmad_omar.SbarHive");
}

#[test]
fn ac13_one_shortcut_drop_with_super_z_in_the_shortcuts_spec_form() {
    assert_eq!(portal::SHORTCUT_ID, "drop");
    // shortcuts-spec: modifiers CTRL, ALT, SHIFT, NUM, LOGO joined by "+" to
    // an xkbcommon keysym name without XKB_KEY_ (XKB_KEY_z is "z").
    assert_eq!(portal::PREFERRED_TRIGGER, "LOGO+z");
}

#[test]
fn ac13_description_in_both_languages() {
    let en = portal::description(Lang::En);
    let ar = portal::description(Lang::Ar);
    for d in [en, ar] {
        assert!(!d.is_empty());
        assert_eq!(d, d.trim(), "no outer spaces: {d:?}");
        assert!(
            !d.contains('\u{2014}') && !d.contains('\u{2013}'),
            "no long dash: {d:?}"
        );
    }
    assert!(en.chars().any(|c| c.is_ascii_alphabetic()), "{en:?}");
    assert!(!en.chars().any(arabic), "{en:?}");
    assert!(ar.chars().any(arabic), "{ar:?}");
    assert_ne!(en, ar);
}

#[test]
fn ac13_bound_line_carries_the_trigger_description() {
    assert_eq!(
        portal::bound_line("Fake Super+Z"),
        "portal shortcuts: bound drop (Fake Super+Z)"
    );
    assert_eq!(
        portal::bound_line("Meta+Z"),
        "portal shortcuts: bound drop (Meta+Z)"
    );
}

#[test]
fn ac13_registry_line() {
    assert_eq!(portal::REGISTRY_UNAVAILABLE, "portal registry: unavailable");
}

#[test]
fn ac17_activated_and_deactivated_of_drop_in_our_session_are_the_gate_inputs() {
    assert_eq!(
        portal::signal(OURS, "Activated", OURS, "drop"),
        Some(Signal::Hold { at: None })
    );
    assert_eq!(
        portal::signal(OURS, "Deactivated", OURS, "drop"),
        Some(Signal::Release)
    );
    // The same inputs as hold::from_portal, so one gate and one timer.
    assert_eq!(
        portal::signal(OURS, "Activated", OURS, "drop"),
        hold::from_portal("Activated")
    );
    assert_eq!(
        portal::signal(OURS, "Deactivated", OURS, "drop"),
        hold::from_portal("Deactivated")
    );
}

#[test]
fn ac17_another_shortcut_id_is_ignored() {
    for member in ["Activated", "Deactivated"] {
        assert_eq!(portal::signal(OURS, member, OURS, "other"), None);
        assert_eq!(portal::signal(OURS, member, OURS, ""), None);
        assert_eq!(portal::signal(OURS, member, OURS, "Drop"), None);
    }
}

#[test]
fn ac17_another_session_is_ignored() {
    for member in ["Activated", "Deactivated"] {
        assert_eq!(portal::signal(OURS, member, OTHER, "drop"), None);
        assert_eq!(portal::signal(OTHER, member, OURS, "drop"), None);
    }
}

#[test]
fn ac17_other_signals_are_ignored() {
    assert_eq!(portal::signal(OURS, "ShortcutsChanged", OURS, "drop"), None);
    assert_eq!(portal::signal(OURS, "activated", OURS, "drop"), None);
}

#[test]
fn ac19_unavailable_lines_name_their_reason() {
    let want = [
        (Reason::NoPortal, "no-portal"),
        (Reason::NoInterface, "no-interface"),
        (Reason::Denied, "denied"),
        (Reason::Error, "error"),
        // Manager answer 4: a backend that never sends Deactivated.
        (Reason::NoRelease, "no-release"),
        // Manager answer 5: a remembered denial, portal_shortcut = false.
        (Reason::Off, "off"),
    ];
    for (r, name) in want {
        assert_eq!(
            portal::unavailable_line(r),
            format!("portal shortcuts: unavailable ({name})")
        );
    }
}

#[test]
fn ac19_no_interface_without_globalshortcuts_or_below_version_1() {
    assert_eq!(portal::interface_reason(None), Some(Reason::NoInterface));
    assert_eq!(portal::interface_reason(Some(0)), Some(Reason::NoInterface));
    assert_eq!(portal::interface_reason(Some(1)), None);
    assert_eq!(portal::interface_reason(Some(2)), None);
    assert_eq!(portal::interface_reason(Some(3)), None);
}

#[test]
fn ac19_bind_response_codes() {
    assert_eq!(portal::bind_reason(0, &["drop"]), None);
    // Code 1: the user cancelled the dialog.
    assert_eq!(portal::bind_reason(1, &[]), Some(Reason::Denied));
    assert_eq!(portal::bind_reason(1, &["drop"]), Some(Reason::Denied));
    // Code 0 with an empty list: nothing bound, the same as a denial.
    assert_eq!(portal::bind_reason(0, &[]), Some(Reason::Denied));
    // Code 0 without "drop" among the bound ids: drop is not bound.
    assert_eq!(portal::bind_reason(0, &["other"]), Some(Reason::Denied));
    // Code 2 and every other code.
    assert_eq!(portal::bind_reason(2, &[]), Some(Reason::Error));
    assert_eq!(portal::bind_reason(2, &["drop"]), Some(Reason::Error));
    assert_eq!(portal::bind_reason(3, &["drop"]), Some(Reason::Error));
    assert_eq!(portal::bind_reason(u32::MAX, &[]), Some(Reason::Error));
}

#[test]
fn ac19_only_a_denial_is_remembered() {
    // Manager answer 5: a denial writes portal_shortcut = false; the other
    // reasons ask again at the next start.
    assert!(portal::remembers(Reason::Denied));
    for r in [
        Reason::NoPortal,
        Reason::NoInterface,
        Reason::Error,
        Reason::NoRelease,
        Reason::Off,
    ] {
        assert!(!portal::remembers(r), "{r:?}");
    }
}

/// S-014f2 (AC25, AC26, planner decision 2): the pure decision behind a
/// config reload, from the `portal_shortcut` setting before and after it
/// and whether a client thread is still running (bound, registering or
/// closing). False to true starts a client (the caller waits for a closing
/// one, so at most one runs); true to false stops a running one; nothing
/// else touches the portal.
#[test]
fn ac25_ac26_reload_decision_from_the_setting_and_the_client() {
    let want = [
        // (before, after, running)
        ((false, true, false), Reload::Start),
        // A client still closing after an earlier off: start after it ends.
        ((false, true, true), Reload::Start),
        ((true, false, true), Reload::Stop),
        // Nothing runs (it ended on a fallback reason): nothing to close.
        ((true, false, false), Reload::Nothing),
        // Another key changed (AC26 hold_ms), or a client that ended on
        // no-portal or error is not retried by a poll (AC19's 30 s).
        ((true, true, true), Reload::Nothing),
        ((true, true, false), Reload::Nothing),
        // A denial's own write: the key is false in memory first (decision
        // 3), so the poll sees false to false and prints no second line.
        ((false, false, true), Reload::Nothing),
        ((false, false, false), Reload::Nothing),
    ];
    for ((before, after, running), r) in want {
        assert_eq!(
            portal::reload(before, after, running),
            r,
            "before {before} after {after} running {running}"
        );
    }
}
