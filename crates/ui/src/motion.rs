//! Reduced motion preference. Read once when the hive is created, never on
//! the `present` path.
//!
//! Sources, first decisive one wins:
//! 1. `SBARHIVE_REDUCED_MOTION`: `1`, `true`, `yes`, `on` reduce; `0`,
//!    `false`, `no`, `off` animate. Anything else is ignored.
//! 2. KDE: `AnimationDurationFactor=0` in the `[KDE]` group of
//!    `$XDG_CONFIG_HOME/kdeglobals` (a file read, no process).
//! 3. GNOME: `gsettings get org.gnome.desktop.interface enable-animations`,
//!    only when `XDG_CURRENT_DESKTOP` names GNOME, so other desktops never
//!    pay for the process spawn (a few milliseconds, once per daemon).
//!
//! The XDG settings portal has no standard reduced motion key yet, so it is
//! not consulted.

/// Explicit override from the environment variable value.
pub fn parse_env(_value: &str) -> Option<bool> {
    unimplemented!("parse_env")
}

/// `Some(true)` when kdeglobals sets the animation speed to instant.
pub fn parse_kdeglobals(_text: &str) -> Option<bool> {
    unimplemented!("parse_kdeglobals")
}

/// `Some(true)` when gsettings reports animations disabled.
pub fn parse_gsettings(_output: &str) -> Option<bool> {
    unimplemented!("parse_gsettings")
}

/// Combines the sources in priority order. `None` everywhere means animate.
pub fn decide(_env: Option<bool>, _kde: Option<bool>, _gnome: Option<bool>) -> bool {
    unimplemented!("decide")
}

/// Reads the sources listed in the module comment. Blocking, call once.
pub fn detect() -> bool {
    let env = std::env::var("SBARHIVE_REDUCED_MOTION")
        .ok()
        .and_then(|v| parse_env(&v));
    if env.is_some() {
        return decide(env, None, None);
    }
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::Path::new(&h).join(".config")));
    let kde = config
        .and_then(|c| std::fs::read_to_string(c.join("kdeglobals")).ok())
        .and_then(|t| parse_kdeglobals(&t));
    if kde.is_some() {
        return decide(None, kde, None);
    }
    let gnome_session = std::env::var("XDG_CURRENT_DESKTOP")
        .map(|d| d.to_ascii_uppercase().contains("GNOME"))
        .unwrap_or(false);
    let gnome = if gnome_session {
        std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "enable-animations"])
            .stderr(std::process::Stdio::null())
            .output()
            .ok()
            .and_then(|o| parse_gsettings(&String::from_utf8_lossy(&o.stdout)))
    } else {
        None
    };
    decide(None, None, gnome)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_values() {
        for v in ["1", "true", "YES", " on "] {
            assert_eq!(parse_env(v), Some(true), "{v}");
        }
        for v in ["0", "false", "No", "off"] {
            assert_eq!(parse_env(v), Some(false), "{v}");
        }
        assert_eq!(parse_env(""), None);
        assert_eq!(parse_env("maybe"), None);
    }

    #[test]
    fn kdeglobals_factor() {
        let zero = "[General]\nfoo=1\n\n[KDE]\nAnimationDurationFactor=0\n";
        assert_eq!(parse_kdeglobals(zero), Some(true));
        let slow = "[KDE]\nAnimationDurationFactor=0.5\n";
        assert_eq!(parse_kdeglobals(slow), Some(false));
        let other_group = "[Other]\nAnimationDurationFactor=0\n[KDE]\nLookAndFeel=x\n";
        assert_eq!(parse_kdeglobals(other_group), None);
        assert_eq!(parse_kdeglobals("[KDE]\nAnimationDurationFactor = 0 \n"), Some(true));
        assert_eq!(parse_kdeglobals("[KDE]\nAnimationDurationFactor=bad\n"), None);
        assert_eq!(parse_kdeglobals(""), None);
    }

    #[test]
    fn gsettings_output() {
        assert_eq!(parse_gsettings("false\n"), Some(true));
        assert_eq!(parse_gsettings("true\n"), Some(false));
        assert_eq!(parse_gsettings("No such schema"), None);
    }

    #[test]
    fn priority_env_then_kde_then_gnome() {
        assert!(!decide(None, None, None));
        assert!(decide(Some(true), Some(false), Some(false)));
        assert!(!decide(Some(false), Some(true), Some(true)));
        assert!(decide(None, Some(true), Some(false)));
        assert!(!decide(None, Some(false), Some(true)));
        assert!(decide(None, None, Some(true)));
        assert!(!decide(None, None, Some(false)));
    }
}
