//! S-135 AC1, AC3 and AC4: the drop-first plan, the choose-during-drag
//! clearance and the settings row.
//! Contract pinned here (tester): kawaro_ui::dnd::drop_first_plan returns a
//! DropFirstPlan { hint_visible, cells_visible, hint_interactive,
//! accept_anywhere, close_before_open, open_hive, explain };
//! kawaro_ui::layout::plan_clear_at(pointer, screen, clearance) returns a
//! ClearancePlacement { x, y, dead_zone } with the hive centre offset by the
//! clearance from the pointer, or centred on the pointer as a dead zone; and
//! WelcomeWindow settings save keeps Config.drop_mode.

use kawaro_core::config::{Config, DropMode};
use kawaro_core::i18n::Lang;
use kawaro_core::welcome::{Registration, captions};
use kawaro_ui::dnd::{DropFirstPlan, drop_first_plan};
use kawaro_ui::layout::{ClearancePlacement, plan_clear_at};
use kawaro_ui::welcome::WelcomeWindow;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn ac1_the_drag_over_surface_shows_a_hint_and_no_cells() {
    let plan = drop_first_plan(true, true, false);
    assert!(plan.hint_visible);
    assert!(
        !plan.cells_visible,
        "the drop-first hint shows no action cells"
    );
    assert!(
        !plan.hint_interactive,
        "the hint takes no keyboard or click"
    );
    assert!(plan.accept_anywhere, "the whole surface accepts the drop");
    assert!(!plan.open_hive);
}

#[test]
fn ac1_the_drop_closes_the_surface_before_the_normal_hive_opens() {
    let plan = drop_first_plan(true, true, true);
    assert!(plan.close_before_open, "close the drop surface first");
    assert!(plan.open_hive, "then open the normal action hive");
    assert!(!plan.hint_visible, "the hint is gone when the drag ends");
    assert!(!plan.cells_visible);
}

#[test]
fn ac1_a_hint_on_empty_surface_accepts_and_leave_hides() {
    assert!(drop_first_plan(true, true, false).accept_anywhere);
    let left = drop_first_plan(true, false, false);
    assert!(!left.hint_visible && !left.accept_anywhere && !left.open_hive);
}

#[test]
fn ac2_files_with_no_action_explain_and_open_no_hive() {
    let plan = drop_first_plan(false, true, true);
    assert!(plan.explain, "the existing no-action explanation shows");
    assert!(!plan.open_hive);
    assert!(!plan.cells_visible);
}

fn placement(pointer: (f32, f32), screen: (f32, f32)) -> ClearancePlacement {
    plan_clear_at(pointer, screen, 64.0)
}

#[test]
fn ac3_the_clearance_moves_the_hive_away_from_the_pointer() {
    let screen = (1280.0, 800.0);
    for pointer in [(40.0, 40.0), (1240.0, 40.0), (40.0, 760.0), (1240.0, 760.0)] {
        let placed = placement(pointer, screen);
        assert!(
            !placed.dead_zone,
            "{pointer:?}: a corner has room for a side"
        );
        let dx = (placed.x - pointer.0).abs();
        let dy = (placed.y - pointer.1).abs();
        assert!(
            dx >= 64.0 || dy >= 64.0,
            "{pointer:?}: offset by the token clearance, got {placed:?}"
        );
        assert!(
            (0.0..=screen.0).contains(&placed.x) && (0.0..=screen.1).contains(&placed.y),
            "{pointer:?}: clamped to the screen, got {placed:?}"
        );
    }
}

#[test]
fn ac3_the_centre_is_a_dead_zone() {
    let placed = placement((640.0, 400.0), (1280.0, 800.0));
    assert!(placed.dead_zone, "the pointer sits over the centre cell");
    assert_eq!((placed.x, placed.y), (640.0, 400.0));
    let narrow = placement((50.0, 50.0), (100.0, 80.0));
    assert!(narrow.dead_zone, "no side has room on a narrow screen");
}

#[test]
fn ac4_the_settings_card_offers_both_modes() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/shell.slint");
    let slint = std::fs::read_to_string(path).unwrap();
    assert!(slint.contains("Drop first"), "the English drop-first label");
    assert!(
        slint.contains("Choose while dragging"),
        "the English choose label"
    );
    assert!(
        slint
            .chars()
            .any(|c| ('\u{0600}'..='\u{06FF}').contains(&c)),
        "the Arabic labels live with the card"
    );
    let welcome = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/welcome.rs"),
    )
    .unwrap();
    assert!(
        welcome.contains("drop_mode"),
        "the settings FormState carries the mode"
    );
}

#[test]
fn ac4_a_settings_save_keeps_the_chosen_mode() {
    i_slint_backend_testing::init_no_event_loop();
    let win = WelcomeWindow::new(Lang::En).expect("a welcome window");
    let steps = captions(Lang::En, &Registration::Unbound);
    win.open(&steps, false).unwrap();
    let saved: Rc<RefCell<Option<Config>>> = Rc::new(RefCell::new(None));
    let slot = saved.clone();
    win.on_settings_save(move |config| {
        *slot.borrow_mut() = Some(config);
        true
    });
    win.show_settings(&Config {
        drop_mode: DropMode::ChooseDuringDrag,
        ..Config::default()
    });
    win.press_save();
    let config = saved.borrow().clone().expect("the settings were saved");
    assert_eq!(config.drop_mode, DropMode::ChooseDuringDrag);
}

#[allow(dead_code)]
fn plan_has_cells(plan: &DropFirstPlan) -> bool {
    plan.cells_visible
}
