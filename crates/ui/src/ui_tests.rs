//! Behaviour of the real hive window on Slint's headless testing platform:
//! no compositor, no display. One `#[test]` because the platform is
//! installed once per thread and the focus state is thread local.

use super::*;
use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{LogicalPosition, SharedString};

#[derive(Default)]
struct Log {
    chosen: RefCell<Vec<String>>,
    dismissed: Cell<u32>,
}

fn cells() -> Vec<HiveCell> {
    let c = |id: &str, ring| HiveCell {
        id: id.into(),
        code: id.to_uppercase(),
        label: format!("label {id}"),
        ring,
    };
    vec![c("webp", 1), c("png", 1), c("jpg", 1), c("gray", 2)]
}

fn key(hive: &Hive, k: Key) {
    let text: SharedString = k.into();
    let w = hive.inner.ui.window();
    w.dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    w.dispatch_event(WindowEvent::KeyReleased { text });
}

fn click(hive: &Hive, x: f32, y: f32) {
    let w = hive.inner.ui.window();
    let position = LogicalPosition::new(x, y);
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

fn centre_of(hive: &Hive, index: usize) -> (f32, f32) {
    let plan = hive.inner.plan.borrow();
    let p = &plan.as_ref().unwrap().cells[index];
    (p.x, p.y)
}

fn origin(hive: &Hive) -> (f32, f32) {
    hive.inner.plan.borrow().as_ref().unwrap().layout.origin
}

fn wired(lang: Lang) -> (Hive, Rc<Log>) {
    let hive = Hive::new(lang).unwrap();
    let log = Rc::new(Log::default());
    let l = log.clone();
    hive.on_choose(move |id| l.chosen.borrow_mut().push(id));
    let l = log.clone();
    hive.on_dismiss(move || l.dismissed.set(l.dismissed.get() + 1));
    (hive, log)
}

#[test]
fn hive_behaviour() {
    i_slint_backend_testing::init_no_event_loop();
    let (hive, log) = wired(Lang::En);
    let ui = &hive.inner.ui;

    // present maps cells to the model and shows the window.
    hive.present("3 images", cells(), None).unwrap();
    assert!(hive.inner.visible());
    assert_eq!(ui.get_cells().row_count(), 4);
    assert_eq!(ui.get_centre_text(), "3 images");
    let first = ui.get_cells().row_data(0).unwrap();
    assert_eq!(
        (first.code.as_str(), first.label.as_str()),
        ("WEBP", "label webp")
    );
    assert!(
        first.x > ui.get_centre_x(),
        "LTR ring 1 starts right of the centre"
    );
    assert!(
        first.y < ui.get_centre_y(),
        "ring 1 starts above the centre"
    );

    // Esc closes and dismisses.
    key(&hive, Key::Escape);
    assert!(!hive.inner.visible());
    assert_eq!(log.dismissed.get(), 1);

    // Keyboard: Tab twice, then Enter chooses the second ring-1 cell.
    hive.present("3 images", cells(), None).unwrap();
    key(&hive, Key::Tab);
    assert_eq!(ui.get_highlight(), 0);
    key(&hive, Key::Tab);
    assert_eq!(ui.get_highlight(), 1);
    key(&hive, Key::Return);
    assert!(!hive.inner.visible());
    assert_eq!(*log.chosen.borrow(), ["png"]);

    // Arrows wrap; a fresh present clears the highlight.
    hive.present("3 images", cells(), None).unwrap();
    assert_eq!(ui.get_highlight(), -1);
    key(&hive, Key::LeftArrow);
    assert_eq!(ui.get_highlight(), 3);
    key(&hive, Key::Escape);
    assert_eq!(log.dismissed.get(), 2);

    // Mouse: hover highlights, click on a cell chooses it.
    hive.present("3 images", cells(), None).unwrap();
    let (x, y) = centre_of(&hive, 2);
    hive.inner
        .ui
        .window()
        .dispatch_event(WindowEvent::PointerMoved {
            position: LogicalPosition::new(x, y),
        });
    assert_eq!(ui.get_highlight(), 2);
    click(&hive, x, y);
    assert_eq!(*log.chosen.borrow(), ["png", "jpg"]);
    assert!(!hive.inner.visible());

    // Click on the centre dismisses.
    hive.present("3 images", cells(), None).unwrap();
    let (ox, oy) = origin(&hive);
    click(&hive, ox, oy);
    assert_eq!(log.dismissed.get(), 3);
    assert!(!hive.inner.visible());

    // Click in the gap between the centre and the first cell dismisses.
    hive.present("3 images", cells(), None).unwrap();
    let (x, y) = centre_of(&hive, 0);
    click(&hive, (ox + x) / 2.0, (oy + y) / 2.0);
    assert_eq!(log.dismissed.get(), 4);

    // Focus loss dismisses only after the window had focus.
    hive.present("3 images", cells(), None).unwrap();
    focus_changed(false);
    assert!(
        hive.inner.visible(),
        "focus loss without prior focus keeps the hive"
    );
    focus_changed(true);
    focus_changed(false);
    assert!(!hive.inner.visible());
    assert_eq!(log.dismissed.get(), 5);

    // close() hides without callbacks, and a later focus loss is ignored.
    hive.present("3 images", cells(), None).unwrap();
    focus_changed(true);
    hive.close();
    focus_changed(false);
    assert!(!hive.inner.visible());
    assert_eq!(log.dismissed.get(), 5);
    assert_eq!(log.chosen.borrow().len(), 2);

    // Accessibility: each cell is a button labelled with its full name, and
    // its default action chooses it.
    hive.present("3 images", cells(), None).unwrap();
    let found: Vec<ElementHandle> =
        ElementHandle::find_by_accessible_label(ui, "label gray").collect();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].accessible_role(), Some(AccessibleRole::Button));
    found[0].invoke_accessible_default_action();
    assert_eq!(log.chosen.borrow().last().unwrap(), "gray");
    assert!(!hive.inner.visible());

    // More than 18 cells: 18 shown.
    let many: Vec<HiveCell> = (0..25)
        .map(|i| HiveCell {
            id: format!("a{i}"),
            code: format!("A{i}"),
            label: format!("action {i}"),
            ring: if i < 10 { 1 } else { 2 },
        })
        .collect();
    hive.present("25", many, None).unwrap();
    assert_eq!(ui.get_cells().row_count(), 18);
    hive.close();
    drop(hive);

    // Arabic mirrors: ring 1 starts left of the centre.
    let (ar, _log) = wired(Lang::Ar);
    ar.present("3 صور", cells(), None).unwrap();
    let ui = &ar.inner.ui;
    let first = ui.get_cells().row_data(0).unwrap();
    assert!(
        first.x < ui.get_centre_x(),
        "Arabic ring 1 starts left of the centre"
    );
    // Right arrow goes back in Arabic.
    key(&ar, Key::Tab);
    key(&ar, Key::RightArrow);
    assert_eq!(ui.get_highlight(), 3);
    ar.close();

    // Focus events of another window (the settings window in the daemon)
    // never count as the hive's: only the latest hive window does.
    let other = HiveWindow::new().unwrap();
    assert!(is_hive_window(Some(ar.inner.ui.window())));
    assert!(!is_hive_window(Some(other.window())));
    assert!(is_hive_window(None));
}
