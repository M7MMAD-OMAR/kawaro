//! Tray icon (StatusNotifierItem). Shows progress in the tooltip while jobs
//! run; the menu cancels jobs or quits. Missing tray support is not an error.

use crate::text;
use ksni::blocking::{Handle, TrayMethods};
use ksni::menu::{MenuItem, StandardItem};
use sbarhive_core::i18n::Lang;
use sbarhive_core::queue::Queue;
use std::sync::Arc;

pub struct SbarTray {
    lang: Lang,
    queue: Arc<Queue>,
    active: Vec<(String, f32)>,
}

pub type TrayHandle = Option<Handle<SbarTray>>;

impl ksni::Tray for SbarTray {
    fn id(&self) -> String {
        "sbarhive".into()
    }
    fn title(&self) -> String {
        text::app_name(self.lang).into()
    }
    fn icon_name(&self) -> String {
        "sbarhive-symbolic".into()
    }
    fn status(&self) -> ksni::Status {
        if self.active.is_empty() {
            ksni::Status::Passive
        } else {
            ksni::Status::Active
        }
    }
    fn tool_tip(&self) -> ksni::ToolTip {
        let description = if self.active.is_empty() {
            text::idle(self.lang).to_string()
        } else {
            self.active
                .iter()
                .map(|(label, f)| format!("{label}: {}%", (f * 100.0).round() as i32))
                .collect::<Vec<_>>()
                .join("\n")
        };
        ksni::ToolTip {
            title: text::app_name(self.lang).into(),
            description,
            icon_name: "sbarhive-symbolic".into(),
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
        vec![
            StandardItem {
                label: text::cancel_all(self.lang).into(),
                enabled: !self.active.is_empty(),
                icon_name: "process-stop".into(),
                activate: Box::new(|t: &mut Self| t.queue.cancel_all()),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: text::quit(self.lang).into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|_| crate::ui::quit()),
                ..Default::default()
            }
            .into(),
        ]
    }
}

pub fn start(lang: Lang, queue: Arc<Queue>) -> TrayHandle {
    match (SbarTray {
        lang,
        queue,
        active: vec![],
    })
    .spawn()
    {
        Ok(h) => Some(h),
        Err(e) => {
            eprintln!("sbarhived: no tray icon ({e})");
            None
        }
    }
}

pub fn set_active(tray: &TrayHandle, active: Vec<(String, f32)>) {
    if let Some(h) = tray {
        h.update(|t| t.active = active);
    }
}
