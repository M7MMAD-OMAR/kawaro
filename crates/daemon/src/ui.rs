//! Bridge to the windows, which live on the main thread with the Slint event
//! loop: the hive and the settings window. Other threads post work with
//! `invoke_from_event_loop`.

use crate::state::{Shared, common_kind, lock};
use crate::{jobs, text};
use sbarhive_core::ActionDef;
use sbarhive_core::config::{Config, Theme};
use sbarhive_core::i18n::Lang;
use sbarhive_core::queue::Queue;
use sbarhive_ui::{Hive, HiveCell, SettingsWindow, thumb};
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

thread_local! {
    static HIVE: RefCell<Option<Hive>> = const { RefCell::new(None) };
    static SETTINGS: RefCell<Option<SettingsWindow>> = const { RefCell::new(None) };
    /// What the settings window needs when it is first opened. Creating it
    /// lazily avoids an empty unmapped toplevel for a window rarely opened.
    static SETTINGS_ARGS: RefCell<Option<(Lang, Shared, Arc<Queue>)>> = const { RefCell::new(None) };
}

/// The colour scheme override the UI crate reads when a window is created.
/// `theme = "auto"` keeps the `SBARHIVE_COLOR_SCHEME` the daemon started
/// with, so it still decides ahead of the portal (S-018 AC15).
fn apply_theme(theme: Theme) {
    static INHERITED: OnceLock<Option<String>> = OnceLock::new();
    let inherited = INHERITED.get_or_init(|| std::env::var("SBARHIVE_COLOR_SCHEME").ok());
    // Only the main thread reads or writes this variable, before creating windows.
    match (theme, inherited) {
        (Theme::Auto, Some(v)) => unsafe { std::env::set_var("SBARHIVE_COLOR_SCHEME", v) },
        (Theme::Auto, None) => unsafe { std::env::remove_var("SBARHIVE_COLOR_SCHEME") },
        (Theme::Light, _) => unsafe { std::env::set_var("SBARHIVE_COLOR_SCHEME", "light") },
        (Theme::Dark, _) => unsafe { std::env::set_var("SBARHIVE_COLOR_SCHEME", "dark") },
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
    let (thumbnails, globs) = preview_source();
    hive.set_preview_source(thumbnails, globs);
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
            if std::env::var_os("SBARHIVE_DEBUG_FOCUS").is_some() {
                eprintln!("{}", drop_cells_line(&actions));
            }
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

/// The `drop cells: ID ID ...` line for the cells of a drag at enter
/// (S-014c AC8), nothing after the colon when no action matches.
fn drop_cells_line(actions: &[ActionDef]) -> String {
    let mut line = String::from("drop cells:");
    for a in actions {
        line.push(' ');
        line.push_str(&a.id);
    }
    line
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

/// Where the hive finds thumbnails and file kinds, from this process's
/// environment (S-018h): the freedesktop thumbnail cache and the
/// shared-mime-info globs of the XDG data directories.
fn preview_source() -> (PathBuf, thumb::Globs) {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let thumbnails = thumb::thumbnails_dir(std::env::var_os("XDG_CACHE_HOME").as_deref(), &home);
    let dirs = thumb::data_dirs(
        std::env::var_os("XDG_DATA_HOME").as_deref(),
        std::env::var_os("XDG_DATA_DIRS").as_deref(),
        &home,
    );
    (thumbnails, thumb::Globs::load(&dirs))
}

/// Shows the hive for `files`, with them in the centre cell.
pub fn present(centre: String, files: Vec<PathBuf>, cells: Vec<HiveCell>, token: Option<String>) {
    let _ = slint::invoke_from_event_loop(move || {
        HIVE.with(|h| {
            if let Some(hive) = h.borrow().as_ref()
                && let Err(e) = hive.present_files(&centre, &files, cells, token)
            {
                eprintln!("sbarhived: cannot show the hive: {e}");
            }
        })
    });
}

/// Shows the hive that waits for a dragged file (`sbarhive drop`), at the
/// pointer position `at` when the caller knows it.
pub fn present_drop(centre: &'static str, token: Option<String>, at: Option<(f64, f64)>) {
    let _ = slint::invoke_from_event_loop(move || {
        HIVE.with(|h| {
            if let Some(hive) = h.borrow().as_ref()
                && let Err(e) = hive.present_drop_at(centre, token, at)
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

    /// S-014c AC8: the ids in cell order, one space apart, and nothing
    /// after the colon when no action matches.
    #[test]
    fn drop_cells_line_lists_the_ids_in_order() {
        let reg = sbarhive_core::Registry::builtin();
        let a = reg.get("image.rotate-90").unwrap().clone();
        let mut b = a.clone();
        b.id = "x.second".into();
        assert_eq!(drop_cells_line(&[]), "drop cells:");
        assert_eq!(
            drop_cells_line(&[a, b]),
            "drop cells: image.rotate-90 x.second"
        );
    }

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

    /// S-018 AC15: the theme setting writes the variable the UI crate reads,
    /// and `auto` puts back the value the daemon started with. One test, so
    /// no other test in this binary touches the variable meanwhile.
    #[test]
    fn ac15_theme_sets_the_colour_scheme_and_auto_restores_the_inherited_value() {
        const VAR: &str = "SBARHIVE_COLOR_SCHEME";
        let original = std::env::var_os(VAR);
        // SAFETY: only this test reads or writes this variable in this
        // binary, and it does so from one thread.
        unsafe { std::env::set_var(VAR, "inherited-by-test") };
        let now = || std::env::var(VAR).ok();
        apply_theme(Theme::Light);
        assert_eq!(now().as_deref(), Some("light"));
        apply_theme(Theme::Dark);
        assert_eq!(now().as_deref(), Some("dark"));
        apply_theme(Theme::Auto);
        assert_eq!(now().as_deref(), Some("inherited-by-test"));
        apply_theme(Theme::Light);
        apply_theme(Theme::Auto);
        assert_eq!(now().as_deref(), Some("inherited-by-test"));
        // SAFETY: as above.
        unsafe {
            match original {
                Some(v) => std::env::set_var(VAR, v),
                None => std::env::remove_var(VAR),
            }
        }
    }
}
