//! S-108 part A (backlog/S-108-s096b-hive-cells.md), the cell model of the
//! window: AC4 a skipped or `+K` cell is not activatable, and hovering it
//! shows its reason. Headless Slint platform, no compositor.
//!
//! Hooks these tests pin:
//! - `HiveCell` gains `pub enabled: bool` and `pub reason: String`.
//! - the window's cell model (`CellData`) carries `enabled` and `reason`
//!   (`ui.get_cells().row_data(i)`).
//! - `HiveWindow` gets `out property <string> centre-shown`
//!   (`ui.get_centre_shown()`): the text the centre cell draws. With no cell
//!   highlighted it is `centre-text`; over a cell with an empty `reason` it
//!   is the cell's `label`; over a cell with a `reason` (skipped, `+K` and
//!   the request cell, S-108 AC4 and AC5) it is that `reason`.
//! - a disabled cell never fires `choose` (click, Return, Space), and the
//!   hive stays open (no dismiss).

use super::*;
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{LogicalPosition, SharedString};

#[derive(Default)]
struct Log {
    chosen: RefCell<Vec<String>>,
    dismissed: Cell<u32>,
}

fn cell(id: &str, enabled: bool, reason: &str) -> HiveCell {
    HiveCell {
        id: id.into(),
        code: id.to_uppercase(),
        label: format!("label {id}"),
        icon: String::new(),
        ring: 1,
        enabled,
        reason: reason.into(),
    }
}

fn cells() -> Vec<HiveCell> {
    vec![
        cell("real", true, ""),
        cell("skipped:0", false, "reason zero"),
        cell("more", false, "reason more"),
        cell("request:xyz", true, "hover text"),
    ]
}

fn wired() -> (Hive, Rc<Log>) {
    let hive = test_env::build(&[], Lang::En);
    let log = Rc::new(Log::default());
    let l = log.clone();
    hive.on_choose(move |id| l.chosen.borrow_mut().push(id));
    let l = log.clone();
    hive.on_dismiss(move || l.dismissed.set(l.dismissed.get() + 1));
    (hive, log)
}

fn key(hive: &Hive, k: Key) {
    let text: SharedString = k.into();
    let ui = hive.inner.ui.borrow();
    let w = ui.window();
    w.dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    w.dispatch_event(WindowEvent::KeyReleased { text });
}

fn centre_of(hive: &Hive, index: usize) -> (f32, f32) {
    let plan = hive.inner.plan.borrow();
    let p = &plan.as_ref().unwrap().cells[index];
    (p.x, p.y)
}

fn hover(hive: &Hive, index: usize) {
    let (x, y) = centre_of(hive, index);
    hive.inner
        .ui
        .borrow()
        .window()
        .dispatch_event(WindowEvent::PointerMoved {
            position: LogicalPosition::new(x, y),
        });
}

fn click(hive: &Hive, index: usize) {
    let (x, y) = centre_of(hive, index);
    let position = LogicalPosition::new(x, y);
    let ui = hive.inner.ui.borrow();
    let w = ui.window();
    w.dispatch_event(WindowEvent::PointerMoved { position });
    w.dispatch_event(WindowEvent::PointerPressed {
        position,
        button: PointerEventButton::Left,
    });
    w.dispatch_event(WindowEvent::PointerReleased {
        position,
        button: PointerEventButton::Left,
    });
}

#[test]
fn ac4_the_window_model_carries_enabled_and_reason() {
    i_slint_backend_testing::init_no_event_loop();
    let (hive, _log) = wired();
    hive.present("3 files", cells(), None).unwrap();
    let ui = hive.inner.ui.borrow();
    let rows: Vec<(bool, String)> = (0..4)
        .map(|i| {
            let c = ui.get_cells().row_data(i).unwrap();
            (c.enabled, c.reason.to_string())
        })
        .collect();
    assert_eq!(
        rows,
        [
            (true, String::new()),
            (false, "reason zero".into()),
            (false, "reason more".into()),
            (true, "hover text".into()),
        ]
    );
}

#[test]
fn ac4_clicking_a_disabled_cell_chooses_nothing() {
    i_slint_backend_testing::init_no_event_loop();
    let (hive, log) = wired();
    hive.present("3 files", cells(), None).unwrap();
    click(&hive, 1);
    click(&hive, 2);
    assert!(log.chosen.borrow().is_empty(), "{:?}", log.chosen.borrow());
    assert_eq!(log.dismissed.get(), 0);
    // An enabled cell next to them still works.
    click(&hive, 3);
    assert_eq!(*log.chosen.borrow(), ["request:xyz"]);
}

#[test]
fn ac4_return_and_space_on_a_disabled_cell_choose_nothing() {
    i_slint_backend_testing::init_no_event_loop();
    let (hive, log) = wired();
    hive.present("3 files", cells(), None).unwrap();
    hover(&hive, 1);
    assert_eq!(hive.inner.ui.borrow().get_highlight(), 1);
    key(&hive, Key::Return);
    key(&hive, Key::Space);
    hover(&hive, 2);
    key(&hive, Key::Return);
    assert!(log.chosen.borrow().is_empty(), "{:?}", log.chosen.borrow());
    assert_eq!(log.dismissed.get(), 0);
    hover(&hive, 0);
    key(&hive, Key::Return);
    assert_eq!(*log.chosen.borrow(), ["real"]);
}

#[test]
fn ac4_hovering_shows_the_reason_for_a_disabled_cell_and_the_label_otherwise() {
    i_slint_backend_testing::init_no_event_loop();
    let (hive, _log) = wired();
    hive.present("3 files", cells(), None).unwrap();
    let shown = |h: &Hive| h.inner.ui.borrow().get_centre_shown().to_string();
    assert_eq!(shown(&hive), "3 files");
    hover(&hive, 0);
    assert_eq!(shown(&hive), "label real");
    hover(&hive, 1);
    assert_eq!(shown(&hive), "reason zero");
    hover(&hive, 2);
    assert_eq!(shown(&hive), "reason more");
    hover(&hive, 3);
    assert_eq!(
        shown(&hive),
        "hover text",
        "the request cell shows its hover text"
    );
}
