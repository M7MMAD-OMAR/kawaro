//! Bridge to the windows, which live on the main thread with the Slint event
//! loop: the hive and the settings window. Other threads post work with
//! `invoke_from_event_loop`.

use crate::jobs;
use crate::state::{Shared, lock};
use sbarhive_core::ActionDef;
use sbarhive_core::config::Config;
use sbarhive_core::i18n::Lang;
use sbarhive_core::queue::Queue;
use sbarhive_ui::{Hive, HiveCell, SettingsWindow};
use std::cell::RefCell;
use std::sync::Arc;

thread_local! {
    static HIVE: RefCell<Option<Hive>> = const { RefCell::new(None) };
    static SETTINGS: RefCell<Option<SettingsWindow>> = const { RefCell::new(None) };
}

/// Creates the hive and the settings window for `lang` and wires them to the
/// queue. Called at start and again when the language setting changes.
pub fn build(lang: Lang, state: &Shared, queue: &Arc<Queue>) -> Result<(), slint::PlatformError> {
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

    let settings = SettingsWindow::new(lang)?;
    {
        let (state, queue) = (state.clone(), queue.clone());
        settings.on_save(move |config| save_settings(config, &state, &queue));
    }
    HIVE.with(|h| *h.borrow_mut() = Some(hive));
    SETTINGS.with(|s| *s.borrow_mut() = Some(settings));
    Ok(())
}

/// Writes the settings, reloads them and rebuilds the windows when the
/// language changed. Runs on the event loop thread (settings callback).
fn save_settings(config: Config, state: &Shared, queue: &Arc<Queue>) {
    let Some(path) = Config::path() else {
        eprintln!("sbarhived: no HOME, settings not saved");
        return;
    };
    if let Err(e) = config.save_to(&path) {
        eprintln!("sbarhived: cannot save {}: {e}", path.display());
        return;
    }
    let (before, after) = {
        let mut st = lock(state);
        let before = st.lang;
        st.reload();
        (before, st.lang)
    };
    if before != after {
        // Rebuild after the settings window's own callback has returned.
        let (state, queue) = (state.clone(), queue.clone());
        slint::Timer::single_shot(std::time::Duration::ZERO, move || {
            if let Err(e) = build(after, &state, &queue) {
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

pub fn open_settings(config: Config) {
    let _ = slint::invoke_from_event_loop(move || {
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
