//! S-136 (backlog/S-136-hive-session-lifecycle.md): one drag owns one
//! selection. These in-crate tests drive the real production dispatch path
//! from receiver notes and wire both on_choose and on_drop_first at once.
//! Contracts pinned here (tester, from the acceptance criteria):
//! - choose-during-drag: one enabled cell drop gives exactly one on_choose,
//!   never on_drop_first; late and duplicate notes add nothing;
//! - drop-first: enter shows the hint with zero action cells, a drop closes
//!   the surface before the handoff callback and hands off once, and no
//!   on_choose runs until the normal hive's enabled cell is chosen;
//! - a committed or dismissed session is terminal: duplicate and late
//!   Enter/Motion/Drop/Leave/release/tick/choose events add no submission,
//!   handoff or show and cannot reuse the old files or preview;
//! - a handoff transfers its files once to a normal hive that is already
//!   visible inside the handoff callback; late drop-session events and a key
//!   release cannot close it;
//! - the mode is captured by the show, so a later config change does not
//!   change the running session.

use super::*;
use kawaro_core::config::DropMode;
use receiver::Note;
use std::rc::Rc;

#[derive(Default)]
struct Log {
    chosen: RefCell<Vec<String>>,
    handoffs: RefCell<Vec<Vec<PathBuf>>>,
    entered: RefCell<Vec<Vec<PathBuf>>>,
    dismissed: Cell<u32>,
    handoff_visible: RefCell<Vec<bool>>,
    presented: Cell<u32>,
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
            enabled: true,
            reason: String::new(),
            customize: false,
        })
        .collect()
}

fn wired(hive: &Rc<Hive>) -> Rc<Log> {
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
    let l = log.clone();
    let h = hive.clone();
    hive.on_drop_first(move |files, _at| {
        l.handoffs.borrow_mut().push(files);
        l.handoff_visible.borrow_mut().push(h.inner.visible());
        l.presented.set(l.presented.get() + 1);
        h.present("1 image", cells(), None).unwrap();
    });
    log
}

fn hive() -> Rc<Hive> {
    Rc::new(test_env::build(&[], Lang::En))
}

fn finish_close(hive: &Hive) {
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(120));
    assert!(!hive.inner.visible(), "the drop surface is closed");
}

#[test]
fn ac1_choose_mode_submits_once_and_never_hands_off() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = hive();
    hive.set_drop_mode(DropMode::ChooseDuringDrag);
    let log = wired(&hive);
    let png = PathBuf::from("/tmp/a.png");
    hive.present_drop("", None).unwrap();
    on_note(Note::Enter(vec![png.clone()]));
    on_note(Note::Motion(Some(0)));
    on_note(Note::Drop);
    assert_eq!(log.chosen.borrow().as_slice(), ["webp"]);
    assert!(log.handoffs.borrow().is_empty());
    assert_eq!(log.presented.get(), 0);
    finish_close(&hive);

    for _ in 0..2 {
        on_note(Note::Enter(vec![png.clone()]));
        on_note(Note::Motion(Some(1)));
        on_note(Note::Drop);
        on_note(Note::Leave);
        hive.key_released();
    }
    assert_eq!(log.chosen.borrow().len(), 1);
    assert!(log.handoffs.borrow().is_empty());
    assert_eq!(log.presented.get(), 0);
    assert!(!hive.inner.visible());
    assert!(hive.inner.dragged.borrow().is_empty());
}

#[test]
fn ac1_drop_first_closes_then_hands_off_once_and_never_chooses() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = hive();
    hive.set_drop_mode(DropMode::DropFirst);
    let log = wired(&hive);
    let png = PathBuf::from("/tmp/a.png");
    hive.present_drop("", None).unwrap();
    on_note(Note::Enter(vec![png.clone()]));
    assert_eq!(
        hive.inner.ui.borrow().get_cells().row_count(),
        0,
        "drop-first enter shows the hint with zero action cells"
    );
    on_note(Note::Motion(Some(0)));
    on_note(Note::Drop);

    assert_eq!(
        log.handoffs.borrow().len(),
        1,
        "the selection hands off once"
    );
    assert_eq!(
        log.handoff_visible.borrow().as_slice(),
        [false],
        "the drop surface is closed before the handoff callback"
    );
    assert!(
        log.chosen.borrow().is_empty(),
        "a drop anywhere submits no job before the choice"
    );
    assert!(hive.inner.visible(), "the normal hive is visible");

    hive.inner.choose(0);
    hive.inner.choose(0);
    assert_eq!(log.chosen.borrow().as_slice(), ["webp"]);

    for _ in 0..2 {
        on_note(Note::Enter(vec![png.clone()]));
        on_note(Note::Motion(Some(1)));
        on_note(Note::Drop);
        on_note(Note::Leave);
        hive.key_released();
    }
    assert_eq!(log.handoffs.borrow().len(), 1);
    assert_eq!(log.chosen.borrow().as_slice(), ["webp"]);
    assert_eq!(log.presented.get(), 1);
}

#[test]
fn ac1_unsupported_and_empty_drops_submit_nothing() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = hive();
    hive.set_drop_mode(DropMode::DropFirst);
    let log = wired(&hive);
    hive.present_drop("", None).unwrap();
    on_note(Note::Enter(Vec::new()));
    on_note(Note::Drop);
    assert!(log.handoffs.borrow().is_empty());
    assert!(log.chosen.borrow().is_empty());
    assert_eq!(log.presented.get(), 0);
    finish_close(&hive);

    hive.present_drop("", None).unwrap();
    on_note(Note::Drop);
    assert!(log.handoffs.borrow().is_empty());
    assert!(hive.inner.dragged.borrow().is_empty());
}

#[test]
fn ac2_a_consumed_session_rejects_each_late_event_twice() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = hive();
    hive.set_drop_mode(DropMode::ChooseDuringDrag);
    let log = wired(&hive);
    let png = PathBuf::from("/tmp/a.png");
    hive.present_drop("", None).unwrap();
    on_note(Note::Enter(vec![png.clone()]));
    on_note(Note::Motion(Some(0)));
    on_note(Note::Drop);
    assert_eq!(log.chosen.borrow().len(), 1);
    finish_close(&hive);

    for _ in 0..2 {
        hive.inner.drag(dnd::Event::Drop);
        hive.inner.drag(dnd::Event::Enter);
        hive.inner.drag(dnd::Event::Motion(Some(0)));
        hive.inner.drag(dnd::Event::Leave);
        hive.inner.drag(dnd::Event::Release);
        hive.inner.drag(dnd::Event::Cancel);
        hive.inner.choose(0);
        hive.inner.drag(dnd::Event::Tick);
    }
    i_slint_backend_testing::mock_elapsed_time(Duration::from_secs(2));
    assert_eq!(log.chosen.borrow().len(), 1, "one submission total");
    assert!(log.handoffs.borrow().is_empty());
    assert_eq!(log.presented.get(), 0);
    assert!(!hive.inner.visible(), "a late event cannot reopen it");
}

#[test]
fn ac2_ac3_a_handoff_owns_the_next_hive_and_late_a_events_are_inert() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = hive();
    hive.set_drop_mode(DropMode::DropFirst);
    let log = wired(&hive);
    let a = PathBuf::from("/tmp/a.png");
    hive.present_drop("", None).unwrap();
    on_note(Note::Enter(vec![a.clone()]));
    on_note(Note::Drop);
    assert_eq!(
        log.handoffs.borrow().as_slice(),
        [vec![a.clone()]],
        "the handoff carries A's payload identity"
    );
    assert!(hive.inner.visible(), "the normal hive B is visible");
    let cells_before = hive.inner.ui.borrow().get_cells().row_count();
    let centre_before = hive.inner.ui.borrow().get_centre_text().to_string();

    let b = PathBuf::from("/tmp/b.png");
    for _ in 0..2 {
        on_note(Note::Enter(vec![b.clone()]));
        on_note(Note::Motion(Some(2)));
        on_note(Note::Drop);
        hive.key_released();
        on_note(Note::Leave);
    }
    assert!(hive.inner.visible(), "B remains visible");
    assert_eq!(
        log.handoffs.borrow().as_slice(),
        [vec![a.clone()]],
        "A's stale generation cannot replace the handoff payload"
    );
    assert_eq!(
        hive.inner.ui.borrow().get_cells().row_count(),
        cells_before,
        "B's model is unchanged by late A events"
    );
    assert_eq!(
        hive.inner.ui.borrow().get_centre_text().to_string(),
        centre_before
    );
    assert!(hive.inner.dragged.borrow().is_empty());
    assert!(log.chosen.borrow().is_empty());
    hive.inner.choose(1);
    assert_eq!(log.chosen.borrow().as_slice(), ["jpg"]);

    hive.present_files("1 image", &[a], cells(), None).unwrap();
    assert!(hive.inner.visible());
    hive.inner.choose(0);
    assert_eq!(log.chosen.borrow().as_slice(), ["jpg", "webp"]);
}

#[test]
fn ac3_the_mode_is_captured_by_the_show() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = hive();
    hive.set_drop_mode(DropMode::ChooseDuringDrag);
    let log = wired(&hive);
    hive.present_drop("", None).unwrap();
    hive.set_drop_mode(DropMode::DropFirst);
    let png = PathBuf::from("/tmp/a.png");
    on_note(Note::Enter(vec![png.clone()]));
    on_note(Note::Motion(Some(0)));
    on_note(Note::Drop);
    assert_eq!(log.chosen.borrow().as_slice(), ["webp"]);
    assert!(
        log.handoffs.borrow().is_empty(),
        "the old show keeps its mode"
    );
}

#[test]
fn ac4_a_hold_release_cannot_dismiss_the_handoff_and_a_fresh_press_works() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = hive();
    hive.set_drop_mode(DropMode::DropFirst);
    let log = wired(&hive);
    let png = PathBuf::from("/tmp/a.png");
    hive.present_drop_hold("", None, Some((400.0, 500.0)))
        .unwrap();
    on_note(Note::Enter(vec![png.clone()]));
    on_note(Note::Drop);
    assert_eq!(log.handoffs.borrow().len(), 1);
    assert!(hive.inner.visible(), "the normal hive is open");
    hive.key_released();
    hive.key_released();
    assert!(
        hive.inner.visible(),
        "the old release cannot dismiss the handoff"
    );
    assert_eq!(log.dismissed.get(), 0);

    hive.present_drop_hold("", None, None).unwrap();
    hive.key_released();
    finish_close(&hive);
    assert!(log.chosen.borrow().is_empty());
    assert_eq!(log.handoffs.borrow().len(), 1);

    hive.present_drop_hold("", None, None).unwrap();
    on_note(Note::Enter(vec![png.clone()]));
    on_note(Note::Drop);
    assert_eq!(log.handoffs.borrow().len(), 2, "a fresh press works once");
}
#[test]
fn ac4_a_held_drop_first_show_matches_the_non_held_show() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = hive();
    hive.set_drop_mode(DropMode::DropFirst);
    let log = wired(&hive);
    let png = PathBuf::from("/tmp/a.png");
    hive.present_drop_hold("", None, None).unwrap();
    on_note(Note::Enter(vec![png.clone()]));
    assert_eq!(
        hive.inner.ui.borrow().get_cells().row_count(),
        0,
        "a held drop-first show shows the hint with no cells"
    );
    on_note(Note::Motion(Some(0)));
    on_note(Note::Drop);
    assert_eq!(log.handoffs.borrow().len(), 1);
    assert_eq!(log.handoff_visible.borrow().as_slice(), [false]);
    assert!(log.chosen.borrow().is_empty());
}

#[test]
fn ac4_a_held_choose_show_never_hands_off() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = hive();
    hive.set_drop_mode(DropMode::ChooseDuringDrag);
    let log = wired(&hive);
    let png = PathBuf::from("/tmp/a.png");
    hive.present_drop_hold("", None, None).unwrap();
    on_note(Note::Enter(vec![png.clone()]));
    assert_eq!(hive.inner.ui.borrow().get_cells().row_count(), 3);
    on_note(Note::Motion(Some(0)));
    on_note(Note::Drop);
    assert_eq!(log.chosen.borrow().as_slice(), ["webp"]);
    assert!(log.handoffs.borrow().is_empty());
}

#[test]
fn ac1_unsupported_nonempty_drag_hands_off_nothing() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = hive();
    hive.set_drop_mode(DropMode::DropFirst);
    let log = Rc::new(Log::default());
    let l = log.clone();
    hive.on_choose(move |id| l.chosen.borrow_mut().push(id));
    let l = log.clone();
    hive.on_drop_files(move |_files| {
        let _ = &l;
        (String::new(), Vec::new())
    });
    let l = log.clone();
    hive.on_drop_first(move |files, _at| l.handoffs.borrow_mut().push(files));
    let png = PathBuf::from("/tmp/no-action.xyz");
    hive.present_drop("", None).unwrap();
    on_note(Note::Enter(vec![png]));
    assert_eq!(
        hive.inner.ui.borrow().get_cells().row_count(),
        0,
        "a nonempty drag with no actions has no cells"
    );
    on_note(Note::Drop);
    assert!(
        log.handoffs.borrow().is_empty(),
        "no handoff without actions"
    );
    assert!(log.chosen.borrow().is_empty());
}

#[test]
fn ac1_a_disabled_cell_submits_nothing() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = hive();
    hive.set_drop_mode(DropMode::ChooseDuringDrag);
    let log = Rc::new(Log::default());
    let l = log.clone();
    hive.on_choose(move |id| l.chosen.borrow_mut().push(id));
    let l = log.clone();
    hive.on_drop_files(move |_files| {
        let mut cells = cells();
        cells[0].enabled = false;
        let _ = &l;
        ("1 image".into(), cells)
    });
    let png = PathBuf::from("/tmp/a.png");
    hive.present_drop("", None).unwrap();
    on_note(Note::Enter(vec![png]));
    on_note(Note::Motion(Some(0)));
    on_note(Note::Drop);
    assert!(
        log.chosen.borrow().is_empty(),
        "a disabled cell chooses nothing"
    );
    assert!(log.handoffs.borrow().is_empty());
}

#[test]
fn ac2_terminal_semantics_hold_under_reduced_motion() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = Rc::new(test_env::build(&[("KAWARO_REDUCED_MOTION", "1")], Lang::En));
    hive.set_drop_mode(DropMode::ChooseDuringDrag);
    let log = wired(&hive);
    let png = PathBuf::from("/tmp/a.png");
    hive.present_drop("", None).unwrap();
    on_note(Note::Enter(vec![png.clone()]));
    on_note(Note::Motion(Some(0)));
    on_note(Note::Drop);
    assert_eq!(log.chosen.borrow().as_slice(), ["webp"]);
    finish_close(&hive);
    for _ in 0..2 {
        on_note(Note::Enter(vec![png.clone()]));
        on_note(Note::Motion(Some(0)));
        on_note(Note::Drop);
        hive.key_released();
    }
    assert_eq!(log.chosen.borrow().len(), 1);
    assert!(log.handoffs.borrow().is_empty());
    assert!(!hive.inner.visible());
}

#[test]
fn ac1_the_receiver_negotiation_only_hands_off_an_accepted_copy() {
    use crate::receiver;
    assert!(
        !receiver::handoff_allowed(false, true, None, DropMode::DropFirst),
        "an invalid accepted must not hand off drop-first"
    );
    assert!(
        !receiver::handoff_allowed(true, false, None, DropMode::DropFirst),
        "a non-copy negotiation must not hand off"
    );
    assert!(
        receiver::handoff_allowed(true, true, None, DropMode::DropFirst),
        "a valid accepted drop off-cell hands off in drop-first"
    );
    assert!(
        !receiver::handoff_allowed(true, true, None, DropMode::ChooseDuringDrag),
        "choose-during-drag needs a cell"
    );
    assert!(receiver::handoff_allowed(
        true,
        true,
        Some(0),
        DropMode::ChooseDuringDrag
    ));
}

#[test]
fn ac2_a_queued_generation_cannot_mutate_the_new_session() {
    let old = crate::session_generation();
    let new = crate::bump_generation_for_test();
    assert!(new > old);
    assert!(
        !crate::generation_is_current(old),
        "the old generation is stale"
    );
    assert!(
        crate::generation_is_current(new),
        "the new generation owns the session"
    );
}

#[test]
fn ac2_a_queued_stage_reset_cannot_reopen_after_terminal_close() {
    assert!(crate::stage_reopen_allowed(false, 7, 7));
    assert!(
        !crate::stage_reopen_allowed(true, 7, 7),
        "a terminal session never rebuilds the panel"
    );
    assert!(
        !crate::stage_reopen_allowed(false, 8, 7),
        "a queued reset from another session is stale"
    );
}

fn stage_hive() -> Rc<Hive> {
    let hive = hive();
    let action = kawaro_core::action::Registry::builtin()
        .get("image.resize-50")
        .expect("the resize action")
        .clone();
    let state =
        stage_two::StageTwoState::build(&action, &[], Lang::En, false).expect("the panel model");
    let cells = vec![HiveCell {
        id: "image.resize-50".to_string(),
        code: "50%".to_string(),
        label: "Resize".to_string(),
        icon: "resize".to_string(),
        ring: 2,
        enabled: true,
        reason: String::new(),
        customize: true,
    }];
    hive.present_options(cells, Rc::new(state)).unwrap();
    hive
}

fn press_escape(hive: &Hive) {
    use slint::platform::{Key, WindowEvent};
    let text: slint::SharedString = Key::Escape.into();
    let ui = hive.inner.ui.borrow();
    let w = ui.window();
    w.dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    w.dispatch_event(WindowEvent::KeyReleased { text });
}

#[test]
fn ac2_a_queued_stage_reset_cannot_revive_a_public_closed_hive() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = stage_hive();
    assert!(hive.inner.ui.borrow().get_stage_two());
    let stamp = crate::session_generation();
    // Queue the real production Reset before the close: reopen_stage's
    // Timer::single_shot is what an old session leaves behind.
    hive.inner.ui.borrow().invoke_stage_reset();
    hive.close();
    assert!(!hive.inner.visible(), "the public close hides at once");
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(5));
    assert!(
        !hive.inner.ui.borrow().get_stage_two(),
        "a queued reset must not rebuild the panel of a closed session"
    );
    assert!(!hive.inner.visible());

    // The active A stamp is stale after the close: the production dispatcher
    // must reject it before it touches the hidden session.
    let point_before = hive.inner.last_point.get();
    crate::dispatch_stamped_note(stamp, receiver::Note::Position(10.0, 20.0), on_note);
    assert_eq!(hive.inner.last_point.get(), point_before);
    assert!(!hive.inner.visible());
    // Source contract: the immutable render geometry is still there for the
    // existing click_cell contract, but the terminal session takes no action.
    let chosen = Rc::new(std::cell::Cell::new(0u32));
    let c = chosen.clone();
    hive.on_choose(move |_| c.set(c.get() + 1));
    assert!(
        hive.click_cell("image.resize-50"),
        "click_cell still finds the rendered cell after the close"
    );
    assert_eq!(
        chosen.get(),
        0,
        "no second callback after the session ended"
    );
}

#[test]
fn ac2_dismiss_now_hides_immediately_and_no_queued_reset_revives_it() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = stage_hive();
    hive.inner.ui.borrow().invoke_stage_reset();
    hive.dismiss_now();
    assert!(
        !hive.inner.ui.borrow().window().is_visible(),
        "dismiss_now hides at once, not after the 90 ms animation"
    );
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(5));
    assert!(!hive.inner.ui.borrow().get_stage_two());
    assert!(!hive.inner.visible());
}

#[test]
fn ac2_a_queued_reset_after_run_or_s139_return_keeps_the_panel_closed() {
    i_slint_backend_testing::init_no_event_loop();
    // Options Run.
    let hive = stage_hive();
    let runs = Rc::new(std::cell::Cell::new(0u32));
    let r = runs.clone();
    hive.on_options_run(move |_, _| r.set(r.get() + 1));
    hive.inner.ui.borrow().invoke_stage_reset();
    hive.inner.ui.borrow().invoke_stage_run();
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(5));
    assert!(
        !hive.inner.ui.borrow().get_stage_two(),
        "Run keeps it closed"
    );
    assert_eq!(runs.get(), 1);

    // S-139: Options Back is a non-terminal return to actions.
    let hive = stage_hive();
    let cancels = Rc::new(std::cell::Cell::new(0u32));
    let c = cancels.clone();
    hive.on_options_cancel(move || c.set(c.get() + 1));
    hive.inner.ui.borrow().invoke_stage_reset();
    hive.inner.ui.borrow().invoke_stage_cancel();
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(5));
    assert!(
        !hive.inner.ui.borrow().get_stage_two(),
        "Cancel keeps it closed"
    );
    assert_eq!(cancels.get(), 1);
    assert!(hive.inner.visible(), "S-139 Back retains the parent hive");

    // S-139: first Escape returns, second Escape remains terminal.
    let hive = stage_hive();
    let cancels = Rc::new(std::cell::Cell::new(0u32));
    let c = cancels.clone();
    hive.on_options_cancel(move || c.set(c.get() + 1));
    hive.inner.ui.borrow().invoke_stage_reset();
    press_escape(&hive);
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(5));
    assert!(
        !hive.inner.ui.borrow().get_stage_two(),
        "Escape keeps it closed"
    );
    assert_eq!(
        cancels.get(),
        1,
        "S-139 first Escape returns through options Back"
    );
    assert!(
        hive.inner.visible(),
        "S-139 first Escape retains the parent hive"
    );
    press_escape(&hive);
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(120));
    assert!(!hive.inner.visible(), "second Escape is terminal");
}

#[test]
fn ac2_a_duplicate_options_run_calls_the_queue_callback_once() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = stage_hive();
    let runs = Rc::new(std::cell::Cell::new(0u32));
    let r = runs.clone();
    hive.on_options_run(move |_id, _params| r.set(r.get() + 1));
    hive.inner.ui.borrow().invoke_stage_run();
    hive.inner.ui.borrow().invoke_stage_run();
    assert_eq!(runs.get(), 1, "one options Run reaches the queue");
    assert!(!hive.inner.ui.borrow().get_stage_two());
}

#[test]
fn ac2_ac3_a_stale_stamped_dispatch_cannot_mutate_the_replacement_session() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = hive();
    hive.set_drop_mode(DropMode::ChooseDuringDrag);
    let a = vec![PathBuf::from("/tmp/a.png")];
    let b = vec![PathBuf::from("/tmp/b.png")];
    let payload = Rc::new(std::cell::RefCell::new(Vec::<PathBuf>::new()));
    let chosen = Rc::new(std::cell::RefCell::new(Vec::<(String, Vec<PathBuf>)>::new()));
    let p = payload.clone();
    hive.on_drop_files(move |files| {
        let mut slot = p.borrow_mut();
        slot.clear();
        slot.extend(files);
        ("1 image".into(), cells())
    });
    let p = payload.clone();
    let c = chosen.clone();
    hive.on_choose(move |id| c.borrow_mut().push((id, p.borrow().clone())));
    let handoffs = Rc::new(std::cell::Cell::new(0u32));
    let h = handoffs.clone();
    hive.on_drop_first(move |_, _| h.set(h.get() + 1));

    // Drop A enters with A, then B replaces it before A is dropped.
    hive.present_drop("", None).unwrap();
    hive.feed_enter(a.clone());
    let stamp_a = crate::session_generation();
    hive.present_drop("", None).unwrap();
    hive.feed_enter(b.clone());
    hive.inner.drag(dnd::Event::Motion(Some(0)));
    assert_eq!(hive.inner.machine.borrow().state(), dnd::State::Over(0));
    assert_eq!(hive.inner.dragged.borrow().as_slice(), b.as_slice());
    let cells_before = hive.inner.ui.borrow().get_cells().row_count();

    // Every late note from A's stamped session is dropped before the sink.
    for note in [
        receiver::Note::Enter(a.clone()),
        receiver::Note::Motion(Some(0)),
        receiver::Note::Drop,
        receiver::Note::Position(10.0, 20.0),
        receiver::Note::Leave,
    ] {
        crate::dispatch_stamped_note(stamp_a, note, on_note);
    }
    assert_eq!(hive.inner.machine.borrow().state(), dnd::State::Over(0));
    assert_eq!(
        hive.inner.dragged.borrow().as_slice(),
        b.as_slice(),
        "B keeps exactly B's payload, not A's"
    );
    assert_eq!(payload.borrow().as_slice(), b.as_slice());
    assert_eq!(hive.inner.ui.borrow().get_cells().row_count(), cells_before);
    assert!(hive.inner.visible(), "B stays visible");

    // B's own drop still submits once, with B's payload identity.
    hive.feed_drop(Some(0));
    assert_eq!(
        chosen.borrow().as_slice(),
        [("webp".to_string(), b.clone())],
        "B's payload is the one consumed"
    );
    assert_eq!(handoffs.get(), 0);
    hive.feed_drop(Some(0));
    assert_eq!(chosen.borrow().len(), 1, "a late second drop adds nothing");
}

#[test]
fn ac2_a_queued_reset_from_a_cannot_rebuild_the_panel_on_replacement_b() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = stage_hive();
    assert!(hive.inner.ui.borrow().get_stage_two());
    hive.inner.ui.borrow().invoke_stage_reset();
    let b = vec![PathBuf::from("/tmp/b.png")];
    hive.present_files("1 image", &b, cells(), None).unwrap();
    let chosen = Rc::new(std::cell::Cell::new(0u32));
    let c = chosen.clone();
    hive.on_choose(move |_| c.set(c.get() + 1));
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(5));
    assert!(
        !hive.inner.ui.borrow().get_stage_two(),
        "A's queued reset must not rebuild the panel on B"
    );
    assert!(hive.inner.visible(), "B stays visible");
    assert!(hive.click_cell("webp"));
    hive.click_cell("webp");
    assert_eq!(chosen.get(), 1, "B's cell choice runs exactly once");
}
