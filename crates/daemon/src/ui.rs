//! Bridge to the windows, which live on the main thread with the Slint event
//! loop: the hive and the settings window. Other threads post work with
//! `invoke_from_event_loop`.

use crate::state::{Shared, common_kind, lock};
use crate::{jobs, text};
use sbarhive_core::ActionDef;
use sbarhive_core::config::{Config, Theme};
use sbarhive_core::i18n::Lang;
use sbarhive_core::queue::Queue;
use sbarhive_ui::{Hive, HiveCell, SettingsWindow};
use std::cell::RefCell;
use std::sync::Arc;

thread_local! {
    static HIVE: RefCell<Option<Hive>> = const { RefCell::new(None) };
    static SETTINGS: RefCell<Option<SettingsWindow>> = const { RefCell::new(None) };
    /// What the settings window needs when it is first opened. Creating it
    /// lazily avoids an empty unmapped toplevel for a window rarely opened.
    static SETTINGS_ARGS: RefCell<Option<(Lang, Shared, Arc<Queue>)>> = const { RefCell::new(None) };
}

/// The colour scheme override the UI crate reads when a window is created.
fn apply_theme(theme: Theme) {
    // Only the main thread reads or writes this variable, before creating windows.
    match theme {
        Theme::Auto => unsafe { std::env::remove_var("SBARHIVE_COLOR_SCHEME") },
        Theme::Light => unsafe { std::env::set_var("SBARHIVE_COLOR_SCHEME", "light") },
        Theme::Dark => unsafe { std::env::set_var("SBARHIVE_COLOR_SCHEME", "dark") },
    }
}

/// Creates the hive and the settings window for `lang` and `theme` and wires
/// them to the queue. Called at start and again when either setting changes.
pub fn build(
    lang: Lang,
    theme: Theme,
    state: &Shared,
    queue: &Arc<Queue>,
) -> Result<(), slint::PlatformError> {
    apply_theme(theme);
    let hive = Hive::new(lang)?;
    {
        let (state, queue) = (state.clone(), queue.clone());
        hive.on_choose(move |id| {
            let mut st = lock(&state);
            let files = st.pending.take().unwrap_or_default();
            match st.registry.get(&id).cloned() {
                Some(action) if !files.is_empty() => {
                    jobs::submit(&mut st, &queue, action, files);
                }
                _ => eprintln!("sbarhived: nothing to run for `{id}`"),
            }
        });
    }
    {
        let state = state.clone();
        hive.on_dismiss(move || lock(&state).pending = None);
    }
    {
        // A drag entered the drop hive: its files wait for the chosen cell,
        // which runs through `on_choose` like a click.
        let state = state.clone();
        hive.on_drop_files(move |files| {
            let mut st = lock(&state);
            let lang = st.lang;
            let actions = st.matching(&files);
            if actions.is_empty() {
                st.pending = None;
                return (text::no_actions(lang).to_string(), Vec::new());
            }
            let centre = text::selection(lang, files.len(), common_kind(&files));
            let cells = actions.iter().map(|a| cell(a, lang)).collect();
            st.pending = Some(files);
            (centre, cells)
        });
    }

    HIVE.with(|h| *h.borrow_mut() = Some(hive));
    // A language or theme change drops the old settings window; the next
    // open builds one with the new settings.
    SETTINGS.with(|s| *s.borrow_mut() = None);
    SETTINGS_ARGS.with(|a| *a.borrow_mut() = Some((lang, state.clone(), queue.clone())));
    Ok(())
}

fn settings_window() -> Result<(), slint::PlatformError> {
    if SETTINGS.with(|s| s.borrow().is_some()) {
        return Ok(());
    }
    let Some((lang, state, queue)) = SETTINGS_ARGS.with(|a| a.borrow().clone()) else {
        return Ok(());
    };
    let settings = SettingsWindow::new(lang)?;
    settings.on_save(move |config| save_settings(config, &state, &queue));
    SETTINGS.with(|s| *s.borrow_mut() = Some(settings));
    Ok(())
}

/// Writes the settings and reloads them. Runs on the event loop thread
/// (settings callback).
fn save_settings(config: Config, state: &Shared, queue: &Arc<Queue>) {
    let Some(path) = Config::path() else {
        eprintln!("sbarhived: no HOME, settings not saved");
        return;
    };
    if let Err(e) = config.save_to(&path) {
        eprintln!("sbarhived: cannot save {}: {e}", path.display());
        return;
    }
    reload(state, queue);
}

/// Re-reads settings and actions, then follows a language or theme change in
/// the tray and the windows. Runs on the event loop thread.
pub fn reload(state: &Shared, queue: &Arc<Queue>) {
    let (before, after) = {
        let mut st = lock(state);
        let before = (st.lang, st.config.theme);
        st.reload();
        (before, (st.lang, st.config.theme))
    };
    if before.0 != after.0 {
        crate::tray::set_lang(after.0);
    }
    if before != after {
        // Rebuild after the settings window's own callback has returned.
        let (state, queue) = (state.clone(), queue.clone());
        slint::Timer::single_shot(std::time::Duration::ZERO, move || {
            if let Err(e) = build(after.0, after.1, &state, &queue) {
                eprintln!("sbarhived: cannot rebuild windows: {e}");
            }
        });
    }
}

pub fn cell(a: &ActionDef, lang: Lang) -> HiveCell {
    HiveCell {
        id: a.id.clone(),
        code: a.code.get(lang).to_string(),
        label: a.label.get(lang).to_string(),
        icon: a.icon.clone(),
        ring: a.ring,
    }
}

pub fn present(centre: String, cells: Vec<HiveCell>, token: Option<String>) {
    let _ = slint::invoke_from_event_loop(move || {
        HIVE.with(|h| {
            if let Some(hive) = h.borrow().as_ref()
                && let Err(e) = hive.present(&centre, cells, token)
            {
                eprintln!("sbarhived: cannot show the hive: {e}");
            }
        })
    });
}

/// Shows the hive that waits for a dragged file (`sbarhive drop`).
pub fn present_drop(token: Option<String>) {
    let _ = slint::invoke_from_event_loop(move || {
        HIVE.with(|h| {
            if let Some(hive) = h.borrow().as_ref()
                && let Err(e) = hive.present_drop("", token)
            {
                eprintln!("sbarhived: cannot show the drop hive: {e}");
            }
        })
    });
}

pub fn open_settings(config: Config) {
    let _ = slint::invoke_from_event_loop(move || {
        if let Err(e) = settings_window() {
            eprintln!("sbarhived: cannot create settings: {e}");
            return;
        }
        SETTINGS.with(|s| {
            if let Some(w) = s.borrow().as_ref()
                && let Err(e) = w.open(&config)
            {
                eprintln!("sbarhived: cannot open settings: {e}");
            }
        })
    });
}

pub fn quit() {
    let _ = slint::invoke_from_event_loop(|| {
        let _ = slint::quit_event_loop();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_carries_the_action_fields_in_the_chosen_language() {
        let reg = sbarhive_core::Registry::builtin();
        let a = reg.get("image.rotate-90").unwrap();
        assert_eq!(
            cell(a, Lang::Ar),
            HiveCell {
                id: "image.rotate-90".into(),
                code: "تدوير".into(),
                label: "تدوير 90 درجة".into(),
                icon: "rotate".into(),
                ring: a.ring,
            }
        );
        let en = cell(a, Lang::En);
        assert_eq!(
            (en.code.as_str(), en.label.as_str(), en.icon.as_str()),
            ("ROT", "Rotate 90°", "rotate")
        );
    }
}
