//! S-014b: the drop hive on the main thread, on Slint's headless testing
//! platform. `on_note` stands in for the Wayland receiver thread. One
//! `#[test]` because the platform is installed once per thread.

use super::*;
use receiver::Note;

#[derive(Default)]
struct Log {
    chosen: RefCell<Vec<String>>,
    dismissed: Cell<u32>,
    entered: RefCell<Vec<Vec<PathBuf>>>,
}

fn cells() -> Vec<HiveCell> {
    ["webp", "jpg", "gray"]
        .iter()
        .map(|id| HiveCell {
            id: (*id).into(),
            code: id.to_uppercase(),
            label: format!("label {id}"),
            icon: String::new(),
            ring: 1,
        })
        .collect()
}

fn size(hive: &Hive) -> (f32, f32) {
    let ui = hive.inner.ui.borrow();
    (ui.get_win_w(), ui.get_win_h())
}

fn state(hive: &Hive) -> dnd::State {
    hive.inner.machine.borrow().state()
}

#[test]
fn drop_hive_behaviour() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = Hive::new(Lang::En).unwrap();
    let log = Rc::new(Log::default());
    let l = log.clone();
    hive.on_choose(move |id| l.chosen.borrow_mut().push(id));
    let l = log.clone();
    hive.on_dismiss(move || l.dismissed.set(l.dismissed.get() + 1));
    let l = log.clone();
    hive.on_drop_files(move |files| {
        l.entered.borrow_mut().push(files.clone());
        if files.is_empty() {
            (String::new(), Vec::new())
        } else {
            ("1 image".into(), cells())
        }
    });
    let png = PathBuf::from("/tmp/a.png");

    // Empty two-ring hive; the cells arrive on enter without a resize.
    hive.present_drop("", None).unwrap();
    assert!(hive.inner.visible());
    assert_eq!(state(&hive), dnd::State::Waiting);
    assert_eq!(hive.inner.ui.borrow().get_cells().row_count(), 0);
    let before = size(&hive);
    on_note(Note::Enter(vec![png.clone()]));
    assert_eq!(log.entered.borrow().as_slice(), [vec![png.clone()]]);
    assert_eq!(size(&hive), before);
    assert_eq!(hive.inner.ui.borrow().get_cells().row_count(), 3);
    assert_eq!(hive.inner.ui.borrow().get_centre_text().as_str(), "1 image");

    // Focus loss does not close a waiting hive.
    focus_changed(true);
    focus_changed(false);
    assert!(hive.inner.visible());
    assert_eq!(log.dismissed.get(), 0);

    // Hover highlights, a drop on a cell runs its action only.
    on_note(Note::Motion(Some(1)));
    assert_eq!(hive.inner.ui.borrow().get_highlight(), 1);
    assert_eq!(state(&hive), dnd::State::Over(1));
    on_note(Note::Motion(None));
    assert_eq!(hive.inner.ui.borrow().get_highlight(), -1);
    on_note(Note::Motion(Some(0)));
    on_note(Note::Drop);
    assert_eq!(log.chosen.borrow().as_slice(), ["webp"]);
    assert_eq!(log.dismissed.get(), 0);
    assert!(!hive.inner.visible());
    // A leave after the drop changes nothing.
    on_note(Note::Leave);
    assert_eq!(log.dismissed.get(), 0);

    // A refused drop (off the cells or on the centre) closes and runs nothing.
    hive.present_drop("", None).unwrap();
    on_note(Note::Enter(vec![png.clone()]));
    on_note(Note::Motion(Some(2)));
    on_note(Note::Motion(None));
    on_note(Note::Drop);
    assert_eq!(log.chosen.borrow().len(), 1);
    assert_eq!(log.dismissed.get(), 1);
    assert!(!hive.inner.visible());

    // Leave after enter closes; leave before enter does not.
    hive.present_drop("", None).unwrap();
    on_note(Note::Leave);
    assert!(hive.inner.visible());
    on_note(Note::Enter(Vec::new()));
    assert_eq!(hive.inner.ui.borrow().get_cells().row_count(), 0);
    on_note(Note::Leave);
    assert_eq!(log.dismissed.get(), 2);
    assert!(!hive.inner.visible());

    // A normal hive ignores the receiver and still closes on focus loss.
    let entered = log.entered.borrow().len();
    hive.present("3 images", cells(), None).unwrap();
    on_note(Note::Enter(vec![png.clone()]));
    on_note(Note::Motion(Some(0)));
    on_note(Note::Drop);
    assert_eq!(log.entered.borrow().len(), entered);
    assert_eq!(log.chosen.borrow().len(), 1);
    assert!(hive.inner.visible());
    focus_changed(true);
    focus_changed(false);
    assert_eq!(log.dismissed.get(), 3);
    assert!(!hive.inner.visible());

    // The enter timeout closes a hive the drag never reached.
    hive.present_drop("", None).unwrap();
    hive.inner.drag(dnd::Event::Tick);
    assert!(hive.inner.visible());
    let late = Instant::now() + dnd::ENTER_TIMEOUT;
    let s = hive
        .inner
        .machine
        .borrow_mut()
        .handle(dnd::Event::Tick, late);
    assert_eq!(s, dnd::State::Hidden);
}
