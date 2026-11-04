//! S-130 AC1: the daemon language transition with a window seam.
//!
//! The daemon is a binary crate, so the transition decision moves to a pure
//! module, crates/daemon/src/welcome_transition.rs, included here by path
//! (the same pattern as s101_reopen.rs). Contract pinned by this test
//! (tester), std and kawaro_core only:
//!
//!     pub trait WindowSeam {
//!         fn hide_and_close(&mut self);
//!         fn keep_visible(&mut self);
//!         fn show_error(&mut self, message: &str);
//!         fn build(&mut self, lang: Lang, choose_language: bool) -> Result<(), String>;
//!     }
//!     pub enum Transition { Kept, Rebuilt, RebuildFailed }   // Copy, Debug, PartialEq, Eq
//!     pub fn language_transition(seam: &mut dyn WindowSeam, save: Result<(), String>, lang: Lang) -> Transition
//!
//! Rules: a failed save keeps the visible window with the failure line and
//! builds nothing. A successful save hides and closes the old window first,
//! then builds the new welcome window with the saved language and
//! choose_language false. A failed rebuild leaves the old window hidden and
//! returns RebuildFailed, so the daemon can report it once on stderr; the
//! overlay is never kept.

#[allow(dead_code)]
mod welcome_transition {
    include!("../src/welcome_transition.rs");
}

use kawaro_core::i18n::Lang;
use welcome_transition::{Transition, WindowSeam, language_transition};

struct Fake {
    log: Vec<String>,
    build_result: Result<(), String>,
}

// Result has no Default, so the fixture default is written by hand.
impl Default for Fake {
    fn default() -> Self {
        Self {
            log: Vec::new(),
            build_result: Ok(()),
        }
    }
}

impl WindowSeam for Fake {
    fn hide_and_close(&mut self) {
        self.log.push("hide".to_string());
    }
    fn keep_visible(&mut self) {
        self.log.push("keep".to_string());
    }
    fn show_error(&mut self, message: &str) {
        self.log.push(format!("error {message}"));
    }
    fn build(&mut self, lang: Lang, choose_language: bool) -> Result<(), String> {
        self.log
            .push(format!("build {lang:?} choose_language={choose_language}"));
        self.build_result.clone()
    }
}

fn ok_fake() -> Fake {
    Fake {
        build_result: Ok(()),
        ..Fake::default()
    }
}

fn failing_fake(message: &str) -> Fake {
    Fake {
        build_result: Err(message.to_string()),
        ..Fake::default()
    }
}

#[test]
fn ac1_a_successful_save_hides_the_old_window_then_builds_the_new_language() {
    let mut fake = ok_fake();
    let transition = language_transition(&mut fake, Ok(()), Lang::Ar);
    assert_eq!(transition, Transition::Rebuilt);
    assert_eq!(
        fake.log,
        vec![
            "hide".to_string(),
            "build Ar choose_language=false".to_string(),
        ],
        "the old window is hidden and closed first, then one new window is built in the saved language with choose_language false"
    );
    assert!(
        !fake.log.iter().any(|line| line == "keep"),
        "the successful path never keeps the overlay"
    );
}

#[test]
fn ac1_a_failed_save_keeps_the_window_with_the_error_and_builds_nothing() {
    let mut fake = ok_fake();
    let transition = language_transition(&mut fake, Err("disk full".to_string()), Lang::En);
    assert_eq!(transition, Transition::Kept);
    assert_eq!(
        fake.log,
        vec!["keep".to_string(), "error disk full".to_string()],
        "a failed save keeps the current window, shows the failure line and builds nothing"
    );
    assert!(
        !fake.log.iter().any(|line| line.starts_with("build")),
        "a failed save builds no new window"
    );
}

#[test]
fn ac1_a_failed_rebuild_leaves_no_stale_overlay() {
    let mut fake = failing_fake("no display");
    let transition = language_transition(&mut fake, Ok(()), Lang::En);
    assert_eq!(transition, Transition::RebuildFailed);
    assert_eq!(
        fake.log,
        vec![
            "hide".to_string(),
            "build En choose_language=false".to_string(),
        ],
        "the old window stays hidden and only the failed build is attempted"
    );
    assert!(
        !fake.log.iter().any(|line| line == "keep"),
        "a failed rebuild must not leave the old language overlay visible"
    );
    assert!(
        !fake.log.iter().any(|line| line.starts_with("error ")),
        "the rebuild failure is reported on stderr by the caller, not drawn in the overlay"
    );
}
