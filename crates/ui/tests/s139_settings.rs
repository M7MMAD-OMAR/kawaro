//! S-139 AC5/AC6. Real Settings controls and actual row containment.
//! Minimal read-only testing-hooks observation contract:
//! SettingsWindow::notification_geometry() -> Vec<(Rect, (f32,f32))> in logical
//! option order, actual segment bounds and rendered label glyph extent after
//! font shaping. Read the real items, never reconstruct a shadow layout.
//! Snapshot capture matrix (no baseline updates during tester phase):
//! options: resize-50, resize-to, crop, compress-to x En/Ar x light/dark
//! at 393x362 and current larger hive size. Capture initial, focused numeric
//! input, invalid draft+error, body scrolled bottom with footer reachable.
//! settings: En/Ar x light/dark at 880x640 on Integration, all three selected
//! notification modes. Native Lead captures every candidate from frozen
//! binaries, before/after SHA256, and inspects images before approval.
use kawaro_core::config::{Config, NotifyMode, Theme};
use kawaro_ui::{Lang, Rect, SettingsWindow};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

fn contains(a: Rect, b: Rect) -> bool {
    b.x >= a.x - 1. && b.y >= a.y - 1. && b.x + b.w <= a.x + a.w + 1. && b.y + b.h <= a.y + a.h + 1.
}

#[test]
fn ac5_ac6_short_truthful_notification_choice_fits_and_saves_all_modes() {
    i_slint_backend_testing::init_no_event_loop();
    for lang in [Lang::En, Lang::Ar] {
        for theme in [Theme::Light, Theme::Dark] {
            let w = SettingsWindow::new(lang).unwrap();
            let saved = Rc::new(RefCell::new(Vec::new()));
            let s = saved.clone();
            w.on_save(move |c| s.borrow_mut().push(c));
            w.open(&Config::default()).unwrap();
            w.set_theme(theme);
            w.set_window_size(880., 640.);
            let mut found = None;
            for page in 0..6 {
                w.select_page(page);
                if let Some(i) = w
                    .page_rows()
                    .iter()
                    .position(|r| r == "Notifications" || r == "الإشعارات")
                {
                    found = Some((page, i));
                    break;
                }
            }
            let (page, row) = found.expect("Integration notification row");
            let options = w.notify_options();
            let long = options
                .iter()
                .find(|(text, _, _)| text.contains("10"))
                .expect("10 second option");
            let expected = if lang == Lang::En {
                "10 seconds or more"
            } else {
                "10 ثوانٍ أو أكثر"
            };
            assert_eq!(
                long.0, expected,
                "AC5: shorten only the label, keep the inclusive threshold"
            );
            assert!(
                contains(w.row_rect(row), w.row_control_rect(row)),
                "complete segmented group fits actual row"
            );
            let geometry = w.notification_geometry();
            assert_eq!(geometry.len(), 3);
            for (bounds, glyphs) in geometry {
                assert!(
                    glyphs.0 <= bounds.w + 1. && glyphs.1 <= bounds.h + 1.,
                    "AC5: actual notification glyph clipping {lang:?}/{theme:?}: {glyphs:?} in {bounds:?}"
                );
            }
            for (logical, mode) in [NotifyMode::Off, NotifyMode::All, NotifyMode::Long]
                .into_iter()
                .enumerate()
            {
                // SettingsWindow::save returns early while hidden: reopen and
                // settle the Integration page before each real selection.
                w.open(&Config::default()).unwrap();
                w.set_theme(theme);
                w.set_window_size(880., 640.);
                w.select_page(page);
                i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(20));
                // Existing probe gives logical list items interleaved with drop mode.
                w.pick_notify_option([0, 2, 4][logical]);
                i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(20));
                w.press_save();
                assert_eq!(saved.borrow().last().unwrap().notify_mode, mode);
            }
        }
    }
}
