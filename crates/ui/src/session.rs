//! Which kind of desktop session the process runs in (S-113 AC1), decided
//! from the environment values the caller passes, so it is testable without
//! touching the environment.

/// The display system the hive appears on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Session {
    Wayland,
    X11,
    Windows,
    None,
}

/// `WAYLAND_DISPLAY` set and not empty is `Wayland`; else `DISPLAY` set and
/// not empty is `X11`; else `Windows` on Windows; else `None`.
pub fn detect(wayland_display: Option<&str>, display: Option<&str>, is_windows: bool) -> Session {
    let set = |v: Option<&str>| v.is_some_and(|s| !s.is_empty());
    if set(wayland_display) {
        Session::Wayland
    } else if set(display) {
        Session::X11
    } else if is_windows {
        Session::Windows
    } else {
        Session::None
    }
}

/// The session of this process, from `WAYLAND_DISPLAY` and `DISPLAY`.
pub fn current() -> Session {
    let var = |k: &str| std::env::var(k).ok();
    detect(
        var("WAYLAND_DISPLAY").as_deref(),
        var("DISPLAY").as_deref(),
        cfg!(windows),
    )
}
