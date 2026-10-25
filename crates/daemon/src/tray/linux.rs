//! Tray icon (StatusNotifierItem). Shows progress in the tooltip while jobs
//! run; the menu cancels jobs or quits. Missing tray support is not an error.

use crate::progress;
use crate::state::{Shared, lock};
use crate::text;
use kawaro_core::i18n::Lang;
use kawaro_core::queue::Queue;
use ksni::blocking::{Handle, TrayMethods};
use ksni::menu::{MenuItem, StandardItem};
use std::sync::{Arc, OnceLock, TryLockError};

/// A second handle for updates that do not come from the job pump.
static TRAY: OnceLock<Handle<SbarTray>> = OnceLock::new();

/// Runs `f` on the update state; while a check or download holds it, on an
/// idle one, so the tray never waits on the network.
fn with_update<R>(f: impl FnOnce(Option<&crate::update::UpdateState>) -> R) -> R {
    match crate::update_sys::shared().try_lock() {
        Ok(update) => f(Some(&update)),
        Err(TryLockError::Poisoned(p)) => f(Some(&p.into_inner())),
        Err(TryLockError::WouldBlock) => f(None),
    }
}

pub struct SbarTray {
    lang: Lang,
    queue: Arc<Queue>,
    state: Shared,
    active: Vec<(String, f32)>,
}

pub type TrayHandle = Option<Handle<SbarTray>>;

impl ksni::Tray for SbarTray {
    fn id(&self) -> String {
        "kawaro".into()
    }
    fn title(&self) -> String {
        text::app_name(self.lang).into()
    }
    fn icon_name(&self) -> String {
        "kawaro-symbolic".into()
    }
    fn status(&self) -> ksni::Status {
        if self.active.is_empty() {
            ksni::Status::Passive
        } else {
            ksni::Status::Active
        }
    }
    fn tool_tip(&self) -> ksni::ToolTip {
        // The download progress comes from its own holder, so it shows while
        // the update lock is held by the download itself.
        let progress = progress::shared().snapshot(&crate::update_sys::part_len);
        let description = with_update(|update| {
            let mut description =
                text::tooltip_from(self.lang, update, progress.as_ref(), &self.active);
            if update.is_some_and(text::update_item_enabled) {
                description = format!("{description}\n{}", text::update_now_hint(self.lang));
            }
            description
        });
        ksni::ToolTip {
            title: text::app_name(self.lang).into(),
            description,
            icon_name: "kawaro-symbolic".into(),
            icon_pixmap: vec![],
        }
    }
    fn text_direction(&self) -> ksni::TextDirection {
        if self.lang.rtl() {
            ksni::TextDirection::RightToLeft
        } else {
            ksni::TextDirection::LeftToRight
        }
    }
    fn menu(&self) -> Vec<MenuItem<Self>> {
        let (labels, update_enabled) = with_update(|update| {
            let update = update.unwrap_or_else(|| crate::update_sys::idle_view());
            (
                text::menu_labels(self.lang, update),
                text::update_item_enabled(update),
            )
        });
        vec![
            StandardItem {
                label: labels[0].clone(),
                enabled: update_enabled,
                icon_name: "software-update-available".into(),
                activate: Box::new(|t: &mut Self| {
                    crate::update_sys::apply_from_menu(t.state.clone())
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: labels[1].clone(),
                enabled: !self.active.is_empty(),
                icon_name: "process-stop".into(),
                activate: Box::new(|t: &mut Self| t.queue.cancel_all()),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: labels[2].clone(),
                icon_name: "preferences-system".into(),
                activate: Box::new(|t: &mut Self| {
                    let config = lock(&t.state).config.clone();
                    crate::ui::open_settings(config);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: labels[3].clone(),
                icon_name: "application-exit".into(),
                activate: Box::new(|_| crate::ui::quit()),
                ..Default::default()
            }
            .into(),
        ]
    }
}

pub fn start(lang: Lang, queue: Arc<Queue>, state: Shared) -> TrayHandle {
    match (SbarTray {
        lang,
        queue,
        state,
        active: vec![],
    })
    .spawn()
    {
        Ok(h) => {
            let _ = TRAY.set(h.clone());
            Some(h)
        }
        Err(e) => {
            eprintln!("kawarod: no tray icon ({e})");
            None
        }
    }
}

pub fn set_active(tray: &TrayHandle, active: Vec<(String, f32)>) {
    if let Some(h) = tray {
        h.update(|t| t.active = active);
    }
}

/// Switches the tray menu and tooltip to `lang` (after a settings change).
pub fn set_lang(lang: Lang) {
    if let Some(h) = TRAY.get() {
        h.update(|t| t.lang = lang);
    }
}

/// Redraws the menu and tooltip after the update state changed.
pub fn refresh() {
    if let Some(h) = TRAY.get() {
        h.update(|_| {});
    }
}
