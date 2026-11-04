//! S-131 AC2 and AC4: the pure settings focus plan and the window seam.
//!
//! The daemon is a binary crate, so the decision moves to a pure module,
//! crates/daemon/src/settings_focus.rs, included here (std and kawaro_core
//! only, no crate::). Contract pinned by this test (tester):
//!
//!     pub enum Session { Wayland, X11, Windows, None }        // Copy, Debug, PartialEq, Eq
//!     pub enum Plan { OpenWithToken, RebuildWithToken, Reuse, OpenPlain } // same derives
//!     pub fn settings_focus_plan(session: Session, has_token: bool,
//!         window_visible: bool, settings_visible: bool) -> Plan
//!     pub trait SettingsWindowSeam {
//!         fn hide_and_close(&mut self);
//!         fn build(&mut self, token: Option<String>);
//!         fn raise(&mut self);
//!         fn set_pending_token(&mut self, token: Option<String>);
//!         fn clear_pending_token(&mut self);
//!         fn log(&mut self, line: &str);
//!     }
//!     pub fn apply_settings_focus(seam: &mut dyn SettingsWindowSeam, session: Session,
//!         window_visible: bool, settings_visible: bool, token: Option<String>) -> Plan
//!
//! Rules: hidden + token -> OpenWithToken (pending token, build, clear);
//! visible + token -> RebuildWithToken (hide and close first, then pending,
//! build, clear); visible + no token -> Reuse (raise, no build); hidden + no
//! token -> OpenPlain (build, and on Wayland or X11 exactly one line
//! "settings focus: no activation token"). The token is cleared after the
//! build and never appears in a log line. The plan table is the same for
//! Wayland, X11, Windows and None.

#[allow(dead_code)]
mod settings_focus {
    include!("../src/settings_focus.rs");
}

use settings_focus::{
    Plan, Session, SettingsWindowSeam, apply_settings_focus, settings_focus_plan,
};

#[derive(Default)]
struct Fake {
    events: Vec<String>,
    constructions: u32,
    visible: u32,
    pending: Option<String>,
}

fn token_desc(token: &Option<String>) -> String {
    match token {
        Some(value) => format!("token={value}"),
        None => "none".to_string(),
    }
}

impl SettingsWindowSeam for Fake {
    fn hide_and_close(&mut self) {
        self.events.push("hide".to_string());
        self.visible = self.visible.saturating_sub(1);
    }
    fn build(&mut self, token: Option<String>) {
        self.events.push(format!("build {}", token_desc(&token)));
        self.constructions += 1;
        self.visible += 1;
    }
    fn raise(&mut self) {
        self.events.push("raise".to_string());
    }
    fn set_pending_token(&mut self, token: Option<String>) {
        self.events.push(format!("pending {}", token_desc(&token)));
        self.pending = token;
    }
    fn clear_pending_token(&mut self) {
        self.events.push("clear".to_string());
        self.pending = None;
    }
    fn log(&mut self, line: &str) {
        self.events.push(format!("log: {line}"));
    }
}

fn fake_with_visible(visible: bool) -> Fake {
    Fake {
        visible: u32::from(visible),
        ..Fake::default()
    }
}

#[test]
fn ac2_the_plan_table_covers_every_session_token_and_visibility() {
    let sessions = [
        Session::Wayland,
        Session::X11,
        Session::Windows,
        Session::None,
    ];
    for session in sessions {
        for has_token in [false, true] {
            for window_visible in [false, true] {
                let expected = match (has_token, window_visible) {
                    (true, false) => Plan::OpenWithToken,
                    (true, true) => Plan::RebuildWithToken,
                    (false, true) => Plan::Reuse,
                    (false, false) => Plan::OpenPlain,
                };
                assert_eq!(
                    settings_focus_plan(session, has_token, window_visible, false),
                    expected,
                    "{session:?} token={has_token} visible={window_visible}"
                );
                assert_eq!(
                    settings_focus_plan(session, has_token, window_visible, true),
                    expected,
                    "the settings page does not change the plan ({session:?})"
                );
            }
        }
    }
}

#[test]
fn ac2_open_with_token_sets_and_clears_the_pending_token() {
    let mut fake = fake_with_visible(false);
    let plan = apply_settings_focus(
        &mut fake,
        Session::Wayland,
        false,
        false,
        Some("tok".to_string()),
    );
    assert_eq!(plan, Plan::OpenWithToken);
    assert_eq!(
        fake.events,
        vec![
            "pending token=tok".to_string(),
            "build token=tok".to_string(),
            "clear".to_string(),
        ],
        "the token wraps the fresh build and is cleared after it"
    );
    assert_eq!(
        fake.pending, None,
        "PENDING_TOKEN is cleared after the build"
    );
    assert_eq!(fake.constructions, 1);
    assert_eq!(fake.visible, 1);
    assert!(!fake.events.iter().any(|event| event.starts_with("log:")));
}

#[test]
fn ac2_rebuild_with_token_hides_the_old_window_first() {
    let mut fake = fake_with_visible(true);
    let plan = apply_settings_focus(
        &mut fake,
        Session::Wayland,
        true,
        true,
        Some("tok".to_string()),
    );
    assert_eq!(plan, Plan::RebuildWithToken);
    assert_eq!(
        fake.events,
        vec![
            "hide".to_string(),
            "pending token=tok".to_string(),
            "build token=tok".to_string(),
            "clear".to_string(),
        ],
        "the old window is hidden and closed before the fresh window is built"
    );
    assert_eq!(fake.visible, 1, "exactly one window remains");
    assert_eq!(fake.pending, None);
}

#[test]
fn ac2_reuse_raises_the_visible_window_without_building() {
    let mut fake = fake_with_visible(true);
    let plan = apply_settings_focus(&mut fake, Session::X11, true, true, None);
    assert_eq!(plan, Plan::Reuse);
    assert_eq!(fake.events, vec!["raise".to_string()]);
    assert_eq!(fake.constructions, 0);
    assert_eq!(fake.visible, 1);
}

#[test]
fn ac2_no_token_writes_one_settings_focus_line_on_wayland_and_x11() {
    for session in [Session::Wayland, Session::X11] {
        let mut fake = fake_with_visible(false);
        let plan = apply_settings_focus(&mut fake, session, false, false, None);
        assert_eq!(plan, Plan::OpenPlain);
        let lines: Vec<&String> = fake
            .events
            .iter()
            .filter(|event| event.starts_with("log:"))
            .collect();
        assert_eq!(
            lines.len(),
            1,
            "{session:?}: exactly one line per request, got {:?}",
            fake.events
        );
        assert_eq!(lines[0], "log: settings focus: no activation token");
        assert_eq!(fake.constructions, 1);
        assert_eq!(fake.visible, 1);
    }
}

#[test]
fn ac2_a_token_is_never_written_to_a_log_line() {
    let mut fake = fake_with_visible(false);
    apply_settings_focus(
        &mut fake,
        Session::Wayland,
        false,
        false,
        Some("secret".to_string()),
    );
    assert!(
        !fake
            .events
            .iter()
            .any(|event| { event.starts_with("log:") && event.contains("secret") }),
        "the token must not reach a log line: {:?}",
        fake.events
    );
}

#[test]
fn ac4_two_settings_requests_construct_one_window_and_reuse_it() {
    let mut fake = fake_with_visible(false);
    let first = apply_settings_focus(&mut fake, Session::Wayland, false, false, None);
    let second = apply_settings_focus(&mut fake, Session::Wayland, true, true, None);
    assert_eq!(first, Plan::OpenPlain);
    assert_eq!(second, Plan::Reuse, "the second request reuses the window");
    assert_eq!(fake.constructions, 1, "never a second construction");
    assert_eq!(fake.visible, 1, "exactly one visible settings window");
}

#[test]
fn ac4_a_token_request_replaces_the_visible_window_once() {
    let mut fake = fake_with_visible(false);
    apply_settings_focus(&mut fake, Session::Wayland, false, false, None);
    let plan = apply_settings_focus(
        &mut fake,
        Session::Wayland,
        true,
        true,
        Some("tok".to_string()),
    );
    assert_eq!(plan, Plan::RebuildWithToken);
    assert_eq!(fake.constructions, 2, "one open plus one replacement");
    assert_eq!(fake.visible, 1, "still one visible window");
    let hide = fake.events.iter().position(|event| event == "hide");
    let build = fake
        .events
        .iter()
        .rposition(|event| event.starts_with("build "));
    assert!(
        hide.is_some() && build.is_some() && hide < build,
        "the old window is hidden before the new one is built: {:?}",
        fake.events
    );
}

#[test]
fn ac4_regression_owner_duplicate_settings_window() {
    // The owner once saw two settings windows. Repeated requests must keep
    // one construction and never show more than one window at any step.
    let mut fake = fake_with_visible(false);
    let mut plans = Vec::new();
    for _ in 0..5 {
        let visible = fake.visible > 0;
        plans.push(apply_settings_focus(
            &mut fake,
            Session::Wayland,
            visible,
            true,
            None,
        ));
        assert!(fake.visible <= 1, "more than one visible settings window");
    }
    assert_eq!(plans[0], Plan::OpenPlain);
    assert!(plans[1..].iter().all(|plan| *plan == Plan::Reuse));
    assert_eq!(fake.constructions, 1, "one window for the whole session");
    assert_eq!(fake.visible, 1);
}
