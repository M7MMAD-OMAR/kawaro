//! The app shell shared by the settings window and the welcome window
//! (S-094, `ui/shell.slint`): the page memory, the About privacy sentence,
//! and, behind the `testing-hooks` feature, the geometry and state probes
//! the behaviour tests read (`tests/s094_shell.rs`).

use crate::Lang;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A rectangle in logical px, in window coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// The page the settings window last showed, in memory only (never written
/// to `config.toml`): the daemon keeps one process, so a new window opens on
/// it. 0 is General, which is also the first ever open.
static PAGE: AtomicUsize = AtomicUsize::new(0);

/// The number of settings pages, General to About.
pub(crate) const SETTINGS_PAGES: usize = 5;

pub(crate) fn remembered_page() -> usize {
    PAGE.load(Ordering::Relaxed)
}

pub(crate) fn remember_page(page: usize) {
    PAGE.store(page.min(SETTINGS_PAGES - 1), Ordering::Relaxed);
}

/// Forgets the remembered page, so the next window opens on General.
#[doc(hidden)]
pub fn reset_page_memory() {
    PAGE.store(0, Ordering::Relaxed);
}

/// The privacy sentence of the About page.
pub fn privacy_statement(lang: Lang) -> String {
    match lang {
        Lang::En => "No telemetry. No file leaves your computer.".to_string(),
        Lang::Ar => "بلا تتبع. لا يغادر أي ملف جهازك.".to_string(),
    }
}

#[cfg(feature = "testing-hooks")]
pub(crate) mod probe {
    use super::Rect;
    use i_slint_backend_testing::{AccessibleRole, ElementHandle, ElementRoot};

    fn rect(e: &ElementHandle) -> Rect {
        let p = e.absolute_position();
        let s = e.size();
        Rect {
            x: p.x,
            y: p.y,
            w: s.width,
            h: s.height,
        }
    }

    fn by_id(root: &impl ElementRoot, id: &str) -> Option<ElementHandle> {
        ElementHandle::find_by_element_id(root, id).next()
    }

    /// The rectangle of the element `id`; empty when it is not on screen.
    pub(crate) fn rect_of(root: &impl ElementRoot, id: &str) -> Rect {
        by_id(root, id).map(|e| rect(&e)).unwrap_or(Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        })
    }

    pub(crate) fn exists(root: &impl ElementRoot, id: &str) -> bool {
        by_id(root, id).is_some()
    }

    pub(crate) fn label_of(root: &impl ElementRoot, id: &str) -> String {
        by_id(root, id)
            .and_then(|e| e.accessible_label())
            .map(|s| s.to_string())
            .unwrap_or_default()
    }

    fn all(root: &impl ElementRoot) -> Vec<ElementHandle> {
        root.root_element().query_descendants().find_all()
    }

    fn tabs(root: &impl ElementRoot) -> Vec<ElementHandle> {
        all(root)
            .into_iter()
            .filter(|e| e.accessible_role() == Some(AccessibleRole::Tab))
            .collect()
    }

    pub(crate) fn rail_labels(root: &impl ElementRoot) -> Vec<String> {
        tabs(root)
            .iter()
            .map(|e| e.accessible_label().unwrap_or_default().to_string())
            .collect()
    }

    pub(crate) fn rail_item_rect(root: &impl ElementRoot, i: usize) -> Rect {
        tabs(root).get(i).map(rect).unwrap_or(Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        })
    }

    pub(crate) fn select_tab(root: &impl ElementRoot, i: usize) {
        if let Some(tab) = tabs(root).get(i) {
            tab.invoke_accessible_default_action();
        }
    }

    /// The `FormRow` elements of the current page, top to bottom.
    fn rows(root: &impl ElementRoot) -> Vec<ElementHandle> {
        let mut rows: Vec<ElementHandle> =
            ElementHandle::find_by_element_type_name(root, "FormRow").collect();
        rows.sort_by(|a, b| {
            let (a, b) = (rect(a), rect(b));
            a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x))
        });
        rows
    }

    /// The `WhatsNewCard` elements, top to bottom.
    fn cards(root: &impl ElementRoot) -> Vec<ElementHandle> {
        let mut cards: Vec<ElementHandle> =
            ElementHandle::find_by_element_type_name(root, "WhatsNewCard").collect();
        cards.sort_by(|a, b| rect(a).y.total_cmp(&rect(b).y));
        cards
    }

    pub(crate) fn card_labels(root: &impl ElementRoot) -> Vec<String> {
        cards(root)
            .iter()
            .map(|e| e.accessible_label().unwrap_or_default().to_string())
            .collect()
    }

    pub(crate) fn card_rect(root: &impl ElementRoot, i: usize) -> Rect {
        cards(root).get(i).map(rect).unwrap_or(Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        })
    }

    /// The title of card `i`, found by position inside the card.
    pub(crate) fn card_title_rect(root: &impl ElementRoot, i: usize) -> Rect {
        let card = card_rect(root, i);
        ElementHandle::find_by_element_id(root, "WhatsNewCard::title-text")
            .map(|e| rect(&e))
            .find(|r| r.y >= card.y - 0.5 && r.y + r.h <= card.y + card.h + 0.5)
            .unwrap_or(card)
    }

    pub(crate) fn row_ys(root: &impl ElementRoot) -> Vec<f32> {
        rows(root).iter().map(|e| rect(e).y).collect()
    }

    pub(crate) fn page_rows(root: &impl ElementRoot) -> Vec<String> {
        rows(root)
            .iter()
            .map(|e| e.accessible_description().unwrap_or_default().to_string())
            .collect()
    }

    pub(crate) fn row_rect(root: &impl ElementRoot, i: usize) -> Rect {
        rows(root).get(i).map(rect).unwrap_or(Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        })
    }

    /// The label or the control slot of row `i`, found by position inside
    /// the row.
    pub(crate) fn row_part(root: &impl ElementRoot, i: usize, part: &str) -> Rect {
        let row = row_rect(root, i);
        let id = format!("FormRow::{part}");
        ElementHandle::find_by_element_id(root, &id)
            .map(|e| rect(&e))
            .find(|r| {
                r.y >= row.y - 0.5
                    && r.y + r.h <= row.y + row.h + 0.5
                    && r.x >= row.x - 0.5
                    && r.x + r.w <= row.x + row.w + 0.5
            })
            .unwrap_or(row)
    }

    /// Accessible labels on screen: every non empty label of an element that
    /// is not plain text, plus the row names.
    pub(crate) fn visible_labels(root: &impl ElementRoot) -> Vec<String> {
        let mut out: Vec<String> = all(root)
            .iter()
            .filter(|e| e.accessible_role() != Some(AccessibleRole::Text))
            .filter_map(|e| e.accessible_label())
            .map(|s| s.to_string())
            .filter(|s| !s.trim().is_empty())
            .collect();
        out.extend(page_rows(root));
        out
    }

    pub(crate) fn visible_texts(root: &impl ElementRoot) -> Vec<String> {
        all(root)
            .iter()
            .filter(|e| e.accessible_role() == Some(AccessibleRole::Text))
            .filter_map(|e| e.accessible_label())
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Interactive elements, rail items and the logo with an empty label.
    pub(crate) fn unlabelled_controls(root: &impl ElementRoot) -> usize {
        use AccessibleRole::*;
        let empty = |e: &ElementHandle| e.accessible_label().is_none_or(|l| l.trim().is_empty());
        let controls = all(root)
            .iter()
            .filter(|e| {
                matches!(
                    e.accessible_role(),
                    Some(Button | Switch | Tab | ListItem | TextInput | Checkbox | RadioButton)
                )
            })
            .filter(|e| empty(e))
            .count();
        let logos = ElementHandle::find_by_element_id(root, "AppShell::logo")
            .filter(|e| empty(e))
            .count();
        controls + logos
    }
}

/// The probes both windows expose, with the same names. `$set_theme` sets
/// the theme index on the generated window.
#[cfg(feature = "testing-hooks")]
macro_rules! shell_probes {
    ($t:ty, $set_theme:expr) => {
        impl $t {
            #[doc(hidden)]
            pub fn set_window_size(&self, w: f32, h: f32) {
                self.inner
                    .ui
                    .borrow()
                    .window()
                    .set_size(slint::LogicalSize::new(w, h));
            }

            #[doc(hidden)]
            pub fn window_size(&self) -> (f32, f32) {
                let ui = self.inner.ui.borrow();
                let w = ui.window();
                let s = w.size().to_logical(w.scale_factor());
                (s.width, s.height)
            }

            #[doc(hidden)]
            pub fn min_window_size(&self) -> (f32, f32) {
                let ui = self.inner.ui.borrow();
                (ui.get_window_min_width(), ui.get_window_min_height())
            }

            #[doc(hidden)]
            pub fn page(&self) -> usize {
                usize::try_from(self.inner.ui.borrow().get_page()).unwrap_or(0)
            }

            #[doc(hidden)]
            pub fn select_page(&self, i: usize) {
                crate::shell::probe::select_tab(&*self.inner.ui.borrow(), i);
            }

            #[doc(hidden)]
            pub fn rail_labels(&self) -> Vec<String> {
                crate::shell::probe::rail_labels(&*self.inner.ui.borrow())
            }

            #[doc(hidden)]
            pub fn rail_focus(&self) -> usize {
                usize::try_from(self.inner.ui.borrow().get_rail_focus()).unwrap_or(0)
            }

            #[doc(hidden)]
            pub fn focus_rail(&self) {
                self.inner.ui.borrow().invoke_focus_rail();
            }

            #[doc(hidden)]
            pub fn page_rows(&self) -> Vec<String> {
                crate::shell::probe::page_rows(&*self.inner.ui.borrow())
            }

            #[doc(hidden)]
            pub fn visible_labels(&self) -> Vec<String> {
                crate::shell::probe::visible_labels(&*self.inner.ui.borrow())
            }

            #[doc(hidden)]
            pub fn visible_texts(&self) -> Vec<String> {
                crate::shell::probe::visible_texts(&*self.inner.ui.borrow())
            }

            #[doc(hidden)]
            pub fn unlabelled_controls(&self) -> usize {
                crate::shell::probe::unlabelled_controls(&*self.inner.ui.borrow())
            }

            #[doc(hidden)]
            pub fn header_rect(&self) -> crate::Rect {
                crate::shell::probe::rect_of(&*self.inner.ui.borrow(), "AppShell::header")
            }

            #[doc(hidden)]
            pub fn logo_rect(&self) -> crate::Rect {
                crate::shell::probe::rect_of(&*self.inner.ui.borrow(), "AppShell::logo")
            }

            #[doc(hidden)]
            pub fn name_rect(&self) -> crate::Rect {
                crate::shell::probe::rect_of(&*self.inner.ui.borrow(), "AppShell::name")
            }

            #[doc(hidden)]
            pub fn version_rect(&self) -> crate::Rect {
                crate::shell::probe::rect_of(&*self.inner.ui.borrow(), "AppShell::version")
            }

            #[doc(hidden)]
            pub fn rail_rect(&self) -> crate::Rect {
                crate::shell::probe::rect_of(&*self.inner.ui.borrow(), "AppShell::rail")
            }

            #[doc(hidden)]
            pub fn rail_item_rect(&self, i: usize) -> crate::Rect {
                crate::shell::probe::rail_item_rect(&*self.inner.ui.borrow(), i)
            }

            #[doc(hidden)]
            pub fn page_area_rect(&self) -> crate::Rect {
                crate::shell::probe::rect_of(&*self.inner.ui.borrow(), "AppShell::scroll")
            }

            #[doc(hidden)]
            pub fn content_rect(&self) -> crate::Rect {
                crate::shell::probe::rect_of(&*self.inner.ui.borrow(), "AppShell::column")
            }

            #[doc(hidden)]
            pub fn footer_rect(&self) -> crate::Rect {
                crate::shell::probe::rect_of(&*self.inner.ui.borrow(), "AppShell::footer")
            }

            #[doc(hidden)]
            pub fn row_rect(&self, i: usize) -> crate::Rect {
                crate::shell::probe::row_rect(&*self.inner.ui.borrow(), i)
            }

            #[doc(hidden)]
            pub fn row_label_rect(&self, i: usize) -> crate::Rect {
                crate::shell::probe::row_part(&*self.inner.ui.borrow(), i, "lab")
            }

            #[doc(hidden)]
            pub fn row_control_rect(&self, i: usize) -> crate::Rect {
                crate::shell::probe::row_part(&*self.inner.ui.borrow(), i, "slot")
            }

            #[doc(hidden)]
            pub fn film_rect(&self) -> Option<crate::Rect> {
                let ui = self.inner.ui.borrow();
                crate::shell::probe::exists(&*ui, "FilmCard::film")
                    .then(|| crate::shell::probe::rect_of(&*ui, "FilmCard::film"))
            }

            #[doc(hidden)]
            pub fn scroll_extent(&self) -> (f32, f32) {
                let ui = self.inner.ui.borrow();
                (
                    ui.get_scroll_content_height(),
                    ui.get_scroll_visible_height(),
                )
            }

            #[doc(hidden)]
            pub fn header_version_text(&self) -> String {
                crate::shell::probe::label_of(&*self.inner.ui.borrow(), "AppShell::version-text")
            }

            #[doc(hidden)]
            pub fn header_name_text(&self) -> String {
                crate::shell::probe::label_of(&*self.inner.ui.borrow(), "AppShell::name")
            }

            #[doc(hidden)]
            pub fn logo_label(&self) -> String {
                crate::shell::probe::label_of(&*self.inner.ui.borrow(), "AppShell::logo")
            }

            #[doc(hidden)]
            pub fn unsaved_marker_visible(&self) -> bool {
                crate::shell::probe::exists(&*self.inner.ui.borrow(), "AppShell::unsaved")
            }

            #[doc(hidden)]
            pub fn save_row_visible(&self) -> bool {
                crate::shell::probe::exists(&*self.inner.ui.borrow(), "AppShell::footer")
            }

            #[doc(hidden)]
            pub fn set_theme(&self, theme: kawaro_core::config::Theme) {
                let rtl = self.inner.is_rtl();
                let i = crate::settings::to_visual(&crate::settings::THEMES, &theme, rtl);
                let set: fn(&Self, i32) = $set_theme;
                set(self, i);
            }

            /// Sends a key as the real keyboard does: the modifier keys go
            /// down first and come up last.
            #[doc(hidden)]
            pub fn send_key(&self, text: &str, ctrl: bool, shift: bool) {
                use slint::platform::{Key, WindowEvent};
                let ui = self.inner.ui.borrow();
                let w = ui.window();
                let modifier = |key: Key| slint::SharedString::from(key);
                if ctrl {
                    w.dispatch_event(WindowEvent::KeyPressed {
                        text: modifier(Key::Control),
                    });
                }
                if shift {
                    w.dispatch_event(WindowEvent::KeyPressed {
                        text: modifier(Key::Shift),
                    });
                }
                let text: slint::SharedString = text.into();
                w.dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
                w.dispatch_event(WindowEvent::KeyReleased { text });
                if shift {
                    w.dispatch_event(WindowEvent::KeyReleased {
                        text: modifier(Key::Shift),
                    });
                }
                if ctrl {
                    w.dispatch_event(WindowEvent::KeyReleased {
                        text: modifier(Key::Control),
                    });
                }
            }
        }
    };
}

#[cfg(feature = "testing-hooks")]
pub(crate) use shell_probes;

#[cfg(test)]
mod tests {
    use super::*;

    /// A remembered page never leaves the five settings pages.
    #[test]
    fn the_page_memory_is_clamped_to_the_last_settings_page() {
        remember_page(99);
        assert_eq!(remembered_page(), SETTINGS_PAGES - 1);
        remember_page(SETTINGS_PAGES - 1);
        assert_eq!(remembered_page(), 4);
        remember_page(0);
        assert_eq!(remembered_page(), 0);
    }
}
