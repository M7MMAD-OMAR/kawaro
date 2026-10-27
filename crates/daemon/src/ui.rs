//! Bridge to the windows, which live on the main thread with the Slint event
//! loop: the hive and the settings window. Other threads post work with
//! `invoke_from_event_loop`.

use crate::portal::{self, TokenStep};
use crate::server;
use crate::state::{Shared, common_kind, lock};
use crate::{jobs, text};
use kawaro_core::ActionDef;
use kawaro_core::config::{Config, LangSetting, Theme};
use kawaro_core::hold::{Effect, Gate, Signal};
use kawaro_core::i18n::Lang;
use kawaro_core::queue::Queue;
#[cfg(unix)]
use kawaro_ui::thumb;
use kawaro_ui::welcome::WelcomeWindow;
use kawaro_ui::{Hive, HiveCell};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

thread_local! {
    static HIVE: RefCell<Option<Hive>> = const { RefCell::new(None) };
    /// What the settings window needs when it is first opened. Creating it
    /// lazily avoids an empty unmapped toplevel for a window rarely opened.
    static SETTINGS_ARGS: RefCell<Option<(Lang, Shared, Arc<Queue>)>> = const { RefCell::new(None) };
    /// The first-run welcome window (S-087), opened on demand like the
    /// settings window.
    static WELCOME: RefCell<Option<WelcomeWindow>> = const { RefCell::new(None) };
    static WELCOME_ARGS: RefCell<Option<(Lang, Shared)>> = const { RefCell::new(None) };
    /// The long press gate of `kawaro drop --hold` (S-019e AC17), on the
    /// event loop thread like the hive it shows.
    static GATE: RefCell<Option<Gate>> = const { RefCell::new(None) };
    /// The activation token of the press that started the gate's timer.
    static HOLD_TOKEN: RefCell<Option<String>> = const { RefCell::new(None) };
    /// Ticks the gate while its threshold timer runs.
    static GATE_TIMER: slint::Timer = slint::Timer::default();
}

/// How often the gate's timer is checked while it runs.
const GATE_TICK: Duration = Duration::from_millis(5);

/// The colour scheme override the UI crate reads when a window is created.
/// `theme = "auto"` keeps the `KAWARO_COLOR_SCHEME` the daemon started
/// with, so it still decides ahead of the portal (S-018 AC15).
fn apply_theme(theme: Theme) {
    static INHERITED: OnceLock<Option<String>> = OnceLock::new();
    let inherited = INHERITED.get_or_init(|| std::env::var("KAWARO_COLOR_SCHEME").ok());
    // Only the main thread reads or writes this variable, before creating windows.
    match (theme, inherited) {
        (Theme::Auto, Some(v)) => unsafe { std::env::set_var("KAWARO_COLOR_SCHEME", v) },
        (Theme::Auto, None) => unsafe { std::env::remove_var("KAWARO_COLOR_SCHEME") },
        (Theme::Light, _) => unsafe { std::env::set_var("KAWARO_COLOR_SCHEME", "light") },
        (Theme::Dark, _) => unsafe { std::env::set_var("KAWARO_COLOR_SCHEME", "dark") },
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
    #[cfg(unix)]
    {
        let (thumbnails, globs) = preview_source();
        hive.set_preview_source(thumbnails, globs);
    }
    {
        let (state, queue) = (state.clone(), queue.clone());
        hive.on_choose(move |id| {
            let mut st = lock(&state);
            let files = st.pending.take().unwrap_or_default();
            match st.registry.get(&id).cloned() {
                Some(action) if !files.is_empty() => {
                    jobs::submit(&mut st, &queue, action, files);
                }
                _ => eprintln!("kawarod: nothing to run for `{id}`"),
            }
            drop(st);
            gate_hive_closed();
        });
    }
    {
        let state = state.clone();
        hive.on_dismiss(move || {
            lock(&state).pending = None;
            gate_hive_closed();
        });
    }
    {
        // A drag entered the drop hive: its files wait for the chosen cell,
        // which runs through `on_choose` like a click.
        let state = state.clone();
        hive.on_drop_files(move |files| {
            let mut st = lock(&state);
            let lang = st.lang;
            let actions = st.matching(&files);
            if std::env::var_os("KAWARO_DEBUG_FOCUS").is_some() {
                kawaro_ui::stderr_line(&drop_cells_line(&actions));
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
    SETTINGS_ARGS.with(|a| *a.borrow_mut() = Some((lang, state.clone(), queue.clone())));
    // The welcome window follows the same rule (S-087).
    WELCOME.with(|w| *w.borrow_mut() = None);
    WELCOME_ARGS.with(|a| *a.borrow_mut() = Some((lang, state.clone())));
    Ok(())
}

/// Writes the settings and reloads them. Runs on the event loop thread
/// (settings callback).
fn save_settings(config: Config, state: &Shared, queue: &Arc<Queue>) -> bool {
    let Some(path) = Config::path() else {
        eprintln!("kawarod: no HOME, settings not saved");
        return false;
    };
    if let Err(e) = config.save_to(&path) {
        eprintln!("kawarod: cannot save {}: {e}", path.display());
        return false;
    }
    reload(state, queue);
    true
}

/// Re-reads settings and actions, then follows a language or theme change in
/// the tray and the windows, and a `portal_shortcut` change in the portal
/// client (S-014f2). Runs on the event loop thread.
pub fn reload(state: &Shared, queue: &Arc<Queue>) {
    let (was_lang, was_theme, was_portal, lang, theme, portal) = {
        let mut st = lock(state);
        let (was_lang, was_theme) = (st.lang, st.config.theme);
        let was_portal = st.config.portal_shortcut;
        st.reload();
        let (lang, theme) = (st.lang, st.config.theme);
        (
            was_lang,
            was_theme,
            was_portal,
            lang,
            theme,
            st.config.portal_shortcut,
        )
    };
    #[cfg(unix)]
    crate::portal_bus::follow(was_portal, portal, lang, state);
    #[cfg(not(unix))]
    let _ = (was_portal, portal);
    if was_lang != lang {
        crate::tray::set_lang(lang);
    }
    if (was_lang, was_theme) != (lang, theme) {
        // Rebuild after the settings window's own callback has returned.
        let was_visible = WELCOME.with(|w| w.borrow().as_ref().is_some_and(|w| w.is_visible()));
        let (state, queue) = (state.clone(), queue.clone());
        slint::Timer::single_shot(std::time::Duration::ZERO, move || {
            if let Err(e) = build(lang, theme, &state, &queue) {
                eprintln!("kawarod: cannot rebuild windows: {e}");
            } else if was_visible {
                present_welcome(&state);
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
#[cfg(unix)]
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
            #[cfg(unix)]
            let shown = h
                .borrow()
                .as_ref()
                .map(|hive| hive.present_files(&centre, &files, cells, token));
            #[cfg(not(unix))]
            let shown = {
                let _ = &files;
                h.borrow()
                    .as_ref()
                    .map(|hive| hive.present(&centre, cells, token))
            };
            if let Some(Err(e)) = shown {
                eprintln!("kawarod: cannot show the hive: {e}");
            }
            // The hive takes keyboard focus at once (S-046 AC4).
            #[cfg(windows)]
            if let Some(hwnd) = h.borrow().as_ref().and_then(|hive| hive.hwnd()) {
                crate::shell_open::foreground(hwnd);
            }
        })
    });
}

/// Shows the hive that waits for a dragged file (`kawaro drop`), at the
/// pointer position `at` when the caller knows it.
pub fn present_drop(centre: &'static str, token: Option<String>, at: Option<(f64, f64)>) {
    let _ = slint::invoke_from_event_loop(move || {
        HIVE.with(|h| {
            if let Some(hive) = h.borrow().as_ref()
                && let Err(e) = hive.present_drop_at(centre, token, at)
            {
                eprintln!("kawarod: cannot show the drop hive: {e}");
            }
        })
    });
}

/// A press or release of the held drop key, received at `now` (S-019e):
/// fed to the long press gate on the event loop thread. The gate shows the
/// drop hive from its timer, and passes a release on to the hive it showed.
pub fn hold_signal(signal: Signal, token: Option<String>, now: Instant, state: Shared) {
    let _ = slint::invoke_from_event_loop(move || {
        let (effect, before, after) = GATE.with(|g| {
            let mut g = g.borrow_mut();
            // hold_ms is read from config.toml at every press (AC18).
            let gate = g.get_or_insert_with(|| Gate::new(|| u64::from(Config::load().0.hold_ms)));
            let before = gate.live_timers();
            let effect = gate.signal(signal, now);
            (effect, before, gate.live_timers())
        });
        // The kept token follows the timer (AC15): a key repeat after the
        // show stores nothing.
        let step = match (before, after) {
            (0, 1) => Some(TokenStep::Start(token.as_deref())),
            (1, 1) => Some(TokenStep::Repeat(token.as_deref())),
            (1, 0) => Some(TokenStep::End),
            _ => None,
        };
        let start = matches!(step, Some(TokenStep::Start(_)));
        if let Some(step) = step {
            token_step(step);
        }
        if start {
            GATE_TIMER.with(|t| {
                t.start(slint::TimerMode::Repeated, GATE_TICK, move || {
                    gate_tick(&state)
                })
            });
        }
        // The gate's borrow is over: the release may close the hive, whose
        // dismiss callback calls the gate again.
        if effect == Some(Effect::Release) {
            HIVE.with(|h| {
                if let Some(hive) = h.borrow().as_ref() {
                    hive.key_released();
                }
            });
        }
    });
}

/// One step of the kept activation token of this thread (AC15): stores the
/// token kept after `step` and returns the one a show activates with.
fn token_step(step: TokenStep<'_>) -> Option<String> {
    HOLD_TOKEN.with(|t| {
        let (kept, shown) = portal::kept_token(t.borrow().as_deref(), step);
        *t.borrow_mut() = kept;
        shown
    })
}

/// The gate's timer: shows the drop hive in hold mode once the threshold
/// has passed with the key still held, and stops when no timer runs.
fn gate_tick(state: &Shared) {
    let (effect, live) = GATE.with(|g| match g.borrow_mut().as_mut() {
        Some(gate) => (gate.tick(Instant::now()), gate.live_timers()),
        None => (None, 0),
    });
    if live == 0 {
        GATE_TIMER.with(|t| t.stop());
    }
    if let Some(Effect::Show { at }) = effect {
        // The files come with the drag, when it enters the hive.
        let lang = {
            let mut st = lock(state);
            st.pending = None;
            st.lang
        };
        let token = token_step(TokenStep::Show);
        HIVE.with(|h| {
            if let Some(hive) = h.borrow().as_ref()
                && let Err(e) = hive.present_drop_hold(text::drop_waiting(lang), token, at)
            {
                eprintln!("kawarod: cannot show the drop hive: {e}");
            }
        });
    }
}

/// Manager decision 14: the hive closed (a drop on a cell, the 10 s
/// timeout, Esc, a click), so the gate forgets the hive it showed and the
/// next press is never ignored.
fn gate_hive_closed() {
    GATE.with(|g| {
        if let Some(gate) = g.borrow_mut().as_mut() {
            gate.hive_closed();
        }
    });
}

/// Counts `open_settings` calls (S-086 AC1): the routing test proves
/// `Request::Settings` reaches this function, whose window work is posted to
/// an event loop no test owns.
#[cfg(test)]
pub fn settings_opens() -> u32 {
    SETTINGS_OPENS.with(|c| c.get())
}

/// Counts `present_welcome` calls (S-087 AC1), the same seam for the
/// first-run route.
#[cfg(test)]
pub fn welcome_opens() -> u32 {
    WELCOME_OPENS.with(|c| c.get())
}

#[cfg(test)]
thread_local! {
    static SETTINGS_OPENS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
    static WELCOME_OPENS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

pub fn open_settings(config: Config) {
    // S-086 AC1: counted before the post, so the routing test sees it
    // without running the event loop.
    #[cfg(test)]
    SETTINGS_OPENS.with(|c| c.set(c.get() + 1));
    let _ = slint::invoke_from_event_loop(move || {
        if let Err(e) = welcome_window() {
            eprintln!("kawarod: cannot create the main window: {e}");
            return;
        }
        WELCOME.with(|slot| {
            let slot = slot.borrow();
            let Some(win) = slot.as_ref() else { return };
            if !win.is_visible() {
                let captions =
                    kawaro_core::welcome::captions(config.lang(), &crate::registration::current());
                if let Err(e) = win.open(&captions, kawaro_ui::welcome::demo_hive_centred()) {
                    eprintln!("kawarod: cannot open the main window: {e}");
                    return;
                }
            }
            win.apply_theme(config.theme);
            if !win.settings_visible() {
                win.show_settings(&config);
            }
            raise_welcome(win);
        });
    });
}

/// Shows the first-run welcome window (S-087). The `settings` entry and the
/// tray item both come through here. The marker is written when the user
/// chooses an action in the introduction.
pub fn present_welcome(state: &Shared) {
    // S-087 AC1: counted before the post, so the routing test sees it
    // without running the event loop.
    #[cfg(test)]
    WELCOME_OPENS.with(|c| c.set(c.get() + 1));
    let state = state.clone();
    let _ = slint::invoke_from_event_loop(move || {
        if let Err(e) = welcome_window() {
            eprintln!("kawarod: cannot create the welcome window: {e}");
            return;
        }
        WELCOME.with(|w| {
            let slot = w.borrow();
            let Some(win) = slot.as_ref() else {
                return;
            };
            if !win.is_visible() {
                let choose_language = {
                    let st = lock(&state);
                    !st.config.welcome_seen && st.config.language == LangSetting::Auto
                };
                if choose_language {
                    win.choose_language();
                }
                let lang = lock(&state).lang;
                let captions =
                    kawaro_core::welcome::captions(lang, &crate::registration::current());
                let centred = kawaro_ui::welcome::demo_hive_centred();
                if let Err(e) = win.open(&captions, centred) {
                    eprintln!("kawarod: cannot open the welcome window: {e}");
                    return;
                }
            }
            win.apply_theme(lock(&state).config.theme);
            raise_welcome(win);
        });
    });
}

/// Creates the welcome window lazily and wires its two buttons.
fn welcome_window() -> Result<(), slint::PlatformError> {
    if WELCOME.with(|w| w.borrow().is_some()) {
        return Ok(());
    }
    let Some((lang, state)) = WELCOME_ARGS.with(|a| a.borrow().clone()) else {
        return Ok(());
    };
    let win = WelcomeWindow::new(lang)?;
    if let Some((_, settings_state, queue)) = SETTINGS_ARGS.with(|a| a.borrow().clone()) {
        win.on_settings_save(move |config| save_settings(config, &settings_state, &queue));
    }
    {
        let state = state.clone();
        win.on_try(move || {
            mark_welcome_complete(&state);
            let dir = std::env::temp_dir().join("kawaro-welcome");
            if let Err(e) = try_sample(&state, &dir) {
                eprintln!("kawarod: {e}");
            }
        });
    }
    {
        let state = state.clone();
        win.on_settings(move || {
            mark_welcome_complete(&state);
            let config = lock(&state).config.clone();
            open_settings(config);
        });
    }
    {
        let state = state.clone();
        win.on_language(move |lang| {
            let Some(path) = Config::path() else {
                return false;
            };
            if let Err(e) = save_language_choice(&state, &path, lang) {
                eprintln!("kawarod: cannot save {}: {e}", path.display());
                return false;
            }
            crate::tray::set_lang(lang);
            let state = state.clone();
            slint::Timer::single_shot(Duration::ZERO, move || {
                let args = SETTINGS_ARGS.with(|a| a.borrow().clone());
                if let Some((_, _, queue)) = args {
                    let theme = lock(&state).config.theme;
                    if let Err(e) = build(lang, theme, &state, &queue) {
                        eprintln!("kawarod: cannot rebuild windows: {e}");
                        WELCOME.with(|slot| {
                            if let Some(win) = slot.borrow().as_ref() {
                                win.language_reload_failed();
                            }
                        });
                        return;
                    }
                    present_welcome(&state);
                }
            });
            true
        });
    }
    WELCOME.with(|w| *w.borrow_mut() = Some(win));
    Ok(())
}

/// Try on an image (S-087 AC4): copies the bundled sample image into the
/// writable folder `dir` and opens the hive for exactly that one file
/// through `server::open`, the same request path the Explorer menu entry
/// takes.
pub(crate) fn try_sample(state: &Shared, dir: &Path) -> Result<PathBuf, String> {
    let sample = kawaro_ui::welcome::install_sample(dir)
        .map_err(|e| format!("cannot write the sample: {e}"))?;
    server::open(state, vec![sample.clone()], None)?;
    Ok(sample)
}

/// Persist the language before updating the running state. Failed writes
/// leave the choice pending and preserve all previous settings.
pub(crate) fn save_language_choice(state: &Shared, path: &Path, lang: Lang) -> std::io::Result<()> {
    let mut st = lock(state);
    let mut config = st.config.clone();
    config.language = match lang {
        Lang::Ar => LangSetting::Ar,
        Lang::En => LangSetting::En,
    };
    config.save_to(path)?;
    st.config = config;
    st.lang = lang;
    Ok(())
}

/// Mark onboarding complete when the user chooses an action in the introduction.
fn mark_welcome_complete(state: &Shared) {
    if let Some(path) = Config::path()
        && let Err(e) = save_welcome_seen(state, &path)
    {
        eprintln!("kawarod: cannot save {}: {e}", path.display());
    }
}

/// Persist the welcome marker after an introduction action.
pub(crate) fn save_welcome_seen(state: &Shared, path: &Path) -> std::io::Result<()> {
    let mut st = lock(state);
    let mut config = st.config.clone();
    config.welcome_seen = true;
    config.save_to(path)?;
    st.config = config;
    Ok(())
}

/// Takes the foreground for the shown welcome window, like the settings
/// window (S-086 AC2).
#[cfg(windows)]
fn raise_welcome(w: &WelcomeWindow) {
    if let Some(hwnd) = w.hwnd() {
        crate::shell_open::foreground(hwnd);
    }
}

#[cfg(not(windows))]
fn raise_welcome(_w: &WelcomeWindow) {}

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
        let reg = kawaro_core::Registry::builtin();
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
        let reg = kawaro_core::Registry::builtin();
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

    /// S-029b AC8 (backlog/S-029-review-cleanup.md): `token_step` keeps the
    /// hold token of this thread. A test runs on its own thread, so the
    /// kept token starts empty.
    #[test]
    fn s029_ac8_token_step_keeps_the_token_of_the_timer() {
        assert_eq!(token_step(TokenStep::Start(Some("t1"))), None);
        assert_eq!(token_step(TokenStep::Show).as_deref(), Some("t1"));
        assert_eq!(token_step(TokenStep::Show), None);

        assert_eq!(token_step(TokenStep::Start(None)), None);
        assert_eq!(token_step(TokenStep::Repeat(Some("t2"))), None);
        assert_eq!(token_step(TokenStep::Show).as_deref(), Some("t2"));

        assert_eq!(token_step(TokenStep::Start(Some("t3"))), None);
        assert_eq!(token_step(TokenStep::End), None);
        assert_eq!(token_step(TokenStep::Show), None);
    }

    /// S-018 AC15: the theme setting writes the variable the UI crate reads,
    /// and `auto` puts back the value the daemon started with. One test, so
    /// no other test in this binary touches the variable meanwhile.
    #[test]
    fn ac15_theme_sets_the_colour_scheme_and_auto_restores_the_inherited_value() {
        const VAR: &str = "KAWARO_COLOR_SCHEME";
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
