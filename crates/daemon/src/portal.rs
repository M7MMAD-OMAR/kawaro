//! The pure part of the daemon's GlobalShortcuts portal client (S-014e1):
//! the shortcut it registers, the lines it prints, why it falls back to the
//! manual bind, and which portal signals reach the long press gate. The
//! D-Bus side is `portal_bus.rs`.

use sbarhive_core::hold::{self, Signal};
use sbarhive_core::i18n::Lang;

/// The id the daemon registers with the portal's Registry: the name of
/// `packaging/io.github.m7mmad_omar.SbarHive.desktop`.
pub const APP_ID: &str = "io.github.m7mmad_omar.SbarHive";
/// The one shortcut the daemon binds.
pub const SHORTCUT_ID: &str = "drop";
/// Super+Z in the shortcuts specification's form: modifiers joined by `+`
/// to an xkbcommon keysym name.
pub const PREFERRED_TRIGGER: &str = "LOGO+z";
/// Printed when `Registry.Register` fails; registration goes on.
pub const REGISTRY_UNAVAILABLE: &str = "portal registry: unavailable";

/// Desktops whose portal backend never sends `Deactivated` (manager
/// answer 4), as `XDG_CURRENT_DESKTOP` names them. Empty until the runs of
/// S-014e2 and S-014e3 show otherwise.
pub const NO_RELEASE: &[&str] = &[];

/// Why the daemon keeps the manual bind instead of the portal shortcut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reason {
    /// Nothing owns `org.freedesktop.portal.Desktop` and it is not
    /// activatable.
    NoPortal,
    /// The portal has no GlobalShortcuts, or its version is below 1.
    NoInterface,
    /// The user cancelled the dialog, or nothing was bound.
    Denied,
    /// Any other failure.
    Error,
    /// The desktop's backend never sends `Deactivated`.
    NoRelease,
    /// A remembered denial: `portal_shortcut = false` in `config.toml`.
    Off,
}

impl Reason {
    fn name(self) -> &'static str {
        match self {
            Reason::NoPortal => "no-portal",
            Reason::NoInterface => "no-interface",
            Reason::Denied => "denied",
            Reason::Error => "error",
            Reason::NoRelease => "no-release",
            Reason::Off => "off",
        }
    }
}

/// The shortcut's description in the portal's dialog and settings.
pub fn description(lang: Lang) -> &'static str {
    match lang {
        Lang::Ar => "إظهار خلية صبار هايف لملف مسحوب",
        Lang::En => "Show the SbarHive hive for a dragged file",
    }
}

/// The line after a successful bind; `trigger` is the reply's
/// `trigger_description`.
pub fn bound_line(trigger: &str) -> String {
    format!("portal shortcuts: bound {SHORTCUT_ID} ({trigger})")
}

/// The one line of a fallback to the manual bind.
pub fn unavailable_line(r: Reason) -> String {
    format!("portal shortcuts: unavailable ({})", r.name())
}

/// The GlobalShortcuts `version` property, `None` when the interface is
/// missing.
pub fn interface_reason(version: Option<u32>) -> Option<Reason> {
    match version {
        Some(v) if v >= 1 => None,
        _ => Some(Reason::NoInterface),
    }
}

/// The BindShortcuts response code and the ids it bound.
pub fn bind_reason(response: u32, bound: &[&str]) -> Option<Reason> {
    match response {
        0 if bound.contains(&SHORTCUT_ID) => None,
        0 | 1 => Some(Reason::Denied),
        _ => Some(Reason::Error),
    }
}

/// Only a denial is written to `config.toml`; every other reason asks
/// again at the next start.
pub fn remembers(r: Reason) -> bool {
    r == Reason::Denied
}

/// What a config reload does to the portal client (S-014f2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reload {
    /// Start a new client, after a closing one has ended.
    Start,
    /// Close the running client's session and end it.
    Stop,
    /// Leave the portal alone.
    Nothing,
}

/// The `portal_shortcut` setting `before` and `after` a reload, and whether
/// a client thread is still running (bound, registering or closing).
pub fn reload(before: bool, after: bool, running: bool) -> Reload {
    match (before, after) {
        (false, true) => Reload::Start,
        (true, false) if running => Reload::Stop,
        _ => Reload::Nothing,
    }
}

/// The in-memory `portal_shortcut` after a denial's save (S-014f3): false
/// when the save succeeded, else the value the file still holds, so a later
/// edit reads no switch change and no second dialog opens.
pub fn after_denial(file: bool, saved: bool) -> bool {
    file && !saved
}

/// The `activation_token` of an `Activated` signal's string valued options
/// (AC15), when it is non-empty.
pub fn activation_token(options: &[(&str, &str)]) -> Option<String> {
    options
        .iter()
        .find(|(k, _)| *k == "activation_token")
        .and_then(|(_, v)| nonempty(Some(v)))
}

fn nonempty(t: Option<&str>) -> Option<String> {
    t.filter(|t| !t.is_empty()).map(str::to_string)
}

/// A step of the long press gate, as the kept activation token sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenStep<'a> {
    /// A press that starts the gate's timer, with its token.
    Start(Option<&'a str>),
    /// A later press of the same timer, with its token.
    Repeat(Option<&'a str>),
    /// The gate's show.
    Show,
    /// The timer ended without a show.
    End,
}

/// The token kept after `step`, and the token the show activates with
/// (`Some` only for [`TokenStep::Show`]). The portal and IPC alike.
pub fn kept_token(kept: Option<&str>, step: TokenStep<'_>) -> (Option<String>, Option<String>) {
    match step {
        TokenStep::Start(t) => (nonempty(t), None),
        TokenStep::Repeat(Some(t)) if !t.is_empty() => (Some(t.to_string()), None),
        TokenStep::Repeat(_) => (kept.map(str::to_string), None),
        TokenStep::Show => (None, kept.map(str::to_string)),
        TokenStep::End => (None, None),
    }
}

/// Whether `XDG_CURRENT_DESKTOP` (a colon separated list) names a desktop
/// of `list`.
pub fn listed(desktops: &str, list: &[&str]) -> bool {
    desktops.split(':').any(|d| list.contains(&d))
}

/// A portal signal as a gate input: `Activated` and `Deactivated` of the
/// drop shortcut in the daemon's own session, nothing else.
pub fn signal(own_session: &str, member: &str, session: &str, shortcut_id: &str) -> Option<Signal> {
    if own_session != session || shortcut_id != SHORTCUT_ID {
        return None;
    }
    hold::from_portal(member)
}

/// The debug line for a signal that reached the gate.
pub fn debug_line(s: Signal) -> &'static str {
    match s {
        Signal::Hold { .. } => "portal: activated drop",
        Signal::Release => "portal: deactivated drop",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listed_matches_one_entry_of_the_colon_list() {
        let list = ["KDE", "Hyprland"];
        assert!(listed("KDE", &list));
        assert!(listed("ubuntu:GNOME:Hyprland", &list));
        assert!(!listed("GNOME", &list));
        assert!(!listed("", &list));
        assert!(!listed("KDE", &[]));
        assert!(!listed("GNOME", NO_RELEASE));
    }

    #[test]
    fn debug_lines_name_the_signal() {
        assert_eq!(
            debug_line(Signal::Hold { at: None }),
            "portal: activated drop"
        );
        assert_eq!(debug_line(Signal::Release), "portal: deactivated drop");
    }
}
