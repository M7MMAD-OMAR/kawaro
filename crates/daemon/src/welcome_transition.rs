// The language transition of the first-run welcome window (S-130), pure so
// the rule is tested without a compositor: a failed save keeps the visible
// window with its error line and builds nothing; a saved language hides and
// closes the old window, then builds a fresh one in the saved language with
// the language screen gone; a failed rebuild leaves the old window hidden.
//
// The daemon calls this from the real on_language path through WindowSeam.
// The test in crates/daemon/tests/s130_welcome_transition.rs includes this
// file by path, so it stays free of daemon internals.

use kawaro_core::i18n::Lang;

/// The visible window one language transition acts on.
pub trait WindowSeam {
    /// Hides and closes the visible welcome window and drops it.
    fn hide_and_close(&mut self);
    /// Keeps the current window on screen.
    fn keep_visible(&mut self);
    /// Shows the save failure message, for example in the kept window.
    fn show_error(&mut self, message: &str);
    /// Builds and presents a fresh welcome window in lang, with the language
    /// screen already gone unless choose_language asks for it.
    fn build(&mut self, lang: Lang, choose_language: bool) -> Result<(), String>;
}

/// What one language transition did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transition {
    /// The save failed; the window stayed and nothing was built.
    Kept,
    /// The old window was hidden and a new one built in the saved language.
    Rebuilt,
    /// The save succeeded and the old window hid, but the rebuild failed.
    RebuildFailed,
}

/// Decides and performs the transition (S-130 AC1).
pub fn language_transition(
    seam: &mut dyn WindowSeam,
    save: Result<(), String>,
    lang: Lang,
) -> Transition {
    if let Err(message) = save {
        seam.keep_visible();
        seam.show_error(&message);
        return Transition::Kept;
    }
    seam.hide_and_close();
    match seam.build(lang, false) {
        Ok(()) => Transition::Rebuilt,
        Err(_) => Transition::RebuildFailed,
    }
}
