// The settings window focus plan (S-131), pure so the decision is tested
// without a window or a compositor: a token opens a fresh window, a visible
// window with a token is replaced once, a visible window without a token is
// reused, and a hidden window without a token opens plainly. On Wayland and
// X11 the plain open says once that it has no activation token and claims
// nothing about focus. The daemon calls this from the real show_settings path
// through SettingsWindowSeam; the test includes this file by path.

/// The desktop session the daemon runs in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Session {
    Wayland,
    X11,
    Windows,
    None,
}

/// What one settings request does to the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plan {
    OpenWithToken,
    RebuildWithToken,
    Reuse,
    OpenPlain,
}

/// The decision (S-131 AC2), the same for every session.
pub fn settings_focus_plan(
    session: Session,
    has_token: bool,
    window_visible: bool,
    settings_visible: bool,
) -> Plan {
    let _ = (session, settings_visible);
    match (has_token, window_visible) {
        (true, false) => Plan::OpenWithToken,
        (true, true) => Plan::RebuildWithToken,
        (false, true) => Plan::Reuse,
        (false, false) => Plan::OpenPlain,
    }
}

/// The window one settings request acts on.
pub trait SettingsWindowSeam {
    /// Hides and closes the visible window and drops it.
    fn hide_and_close(&mut self);
    /// Builds and presents a fresh window, with the activation token set
    /// around it when there is one.
    fn build(&mut self, token: Option<String>);
    /// Raises the visible window without building anything.
    fn raise(&mut self);
    /// Puts the token in the slot the next window adapter consumes.
    fn set_pending_token(&mut self, token: Option<String>);
    /// Clears that slot after the build, so the token cannot leak.
    fn clear_pending_token(&mut self);
    /// One diagnostic line; a token never appears in it.
    fn log(&mut self, line: &str);
}

/// Decides and performs the transition (S-131 AC2 and AC4).
pub fn apply_settings_focus(
    seam: &mut dyn SettingsWindowSeam,
    session: Session,
    window_visible: bool,
    settings_visible: bool,
    token: Option<String>,
) -> Plan {
    let plan = settings_focus_plan(session, token.is_some(), window_visible, settings_visible);
    match plan {
        Plan::OpenWithToken => {
            seam.set_pending_token(token.clone());
            seam.build(token);
            seam.clear_pending_token();
        }
        Plan::RebuildWithToken => {
            seam.hide_and_close();
            seam.set_pending_token(token.clone());
            seam.build(token);
            seam.clear_pending_token();
        }
        Plan::Reuse => seam.raise(),
        Plan::OpenPlain => {
            seam.build(None);
            if matches!(session, Session::Wayland | Session::X11) {
                seam.log("settings focus: no activation token");
            }
        }
    }
    plan
}
