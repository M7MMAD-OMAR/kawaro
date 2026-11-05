//! S-139 production generated-component tests. No shadow form or geometry model.
//! Pointer/key events go to the real window; ElementHandle reads actual items.
//! Additional minimal production contracts (not shadow test implementations):
//! Hive::stage_stamp() -> (u64, u64): session owner and panel-instance epoch.
//! dispatch_stage_update(&Weak<Inner>, stamp, impl FnOnce(&HiveWindow)) runs
//! only for the owning live panel. Actual deferred input, preview results and
//! Reset delivery MUST all use this same dispatcher, including same-session
//! reopen and rebuild. A predicate with no production callsites is not enough.
//! stage_geometry(&Hive) -> Vec<(String, Rect, (f32,f32))> reads actual laid-out
//! stage items, never recomputes a parallel layout. Names: viewport, footer,
//! row:<id>, label:<id>, control:<id>, readout, error. Third tuple is actual
//! wrapped glyph extent for text items, zero for non-text. Offscreen body rows
//! remain observed in window coordinates. This is a read-only observation seam.
use super::*;
use i_slint_backend_testing::{ElementHandle, ElementRoot};
use kawaro_core::options::Kind;
use slint::LogicalPosition;
use slint::platform::WindowAdapter as _;

static TESTS: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn init() -> std::sync::MutexGuard<'static, ()> {
    i_slint_backend_testing::init_no_event_loop();
    TESTS.lock().unwrap_or_else(|e| e.into_inner())
}
use slint::platform::{Key, PointerEventButton, WindowEvent};

fn turn() {
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(250));
}
fn cells() -> Vec<HiveCell> {
    [
        "image.to-webp",
        "image.resize-50",
        "image.resize-to",
        "image.crop",
        "image.compress-to",
    ]
    .iter()
    .enumerate()
    .map(|(i, id)| HiveCell {
        id: (*id).into(),
        code: (*id).into(),
        label: (*id).into(),
        icon: "resize".into(),
        ring: if i == 0 { 1 } else { 2 },
        enabled: true,
        reason: String::new(),
        customize: true,
    })
    .collect()
}
fn state(id: &str, lang: Lang) -> Rc<stage_two::StageTwoState> {
    let action = kawaro_core::action::Registry::builtin()
        .get(id)
        .expect("builtin action")
        .clone();
    Rc::new(stage_two::StageTwoState::build(&action, &[], lang, false).expect("options"))
}
fn open(h: &Hive, id: &str, lang: Lang) {
    h.present_options(cells(), state(id, lang)).unwrap();
    turn();
}

/// The expected schema kinds of the action, independent of how many actual
/// MaterialTextFields the panel happens to build. Crop has enum and bool
/// fields only, so the numeric draft checks must not run for it.
fn schema_kinds(id: &str) -> Vec<Kind> {
    kawaro_core::action::Registry::builtin()
        .get(id)
        .and_then(|action| action.options.as_ref())
        .map(|options| options.fields.iter().map(|field| field.kind).collect())
        .unwrap_or_default()
}

fn is_numeric(kind: Kind) -> bool {
    matches!(kind, Kind::Percent | Kind::Px | Kind::PxPair | Kind::Int)
}
fn event(h: &Hive, e: WindowEvent) {
    h.inner.ui.borrow().window().dispatch_event(e);
}
fn down(h: &Hive, k: Key) {
    event(h, WindowEvent::KeyPressed { text: k.into() });
}
fn up(h: &Hive, k: Key) {
    event(h, WindowEvent::KeyReleased { text: k.into() });
}
fn key(h: &Hive, k: Key) {
    down(h, k);
    up(h, k);
}
fn text(h: &Hive, text: &str) {
    event(h, WindowEvent::KeyPressed { text: text.into() });
    event(h, WindowEvent::KeyReleased { text: text.into() });
}
fn move_to(h: &Hive, p: LogicalPosition) {
    event(h, WindowEvent::PointerMoved { position: p });
}
fn click(h: &Hive, p: LogicalPosition) {
    move_to(h, p);
    event(
        h,
        WindowEvent::PointerPressed {
            position: p,
            button: PointerEventButton::Left,
        },
    );
    event(
        h,
        WindowEvent::PointerReleased {
            position: p,
            button: PointerEventButton::Left,
        },
    );
}
fn all(h: &Hive) -> Vec<ElementHandle> {
    h.inner
        .ui
        .borrow()
        .root_element()
        .query_descendants()
        .match_predicate(|_| true)
        .find_all()
}
fn material_fields(h: &Hive) -> Vec<ElementHandle> {
    h.inner
        .ui
        .borrow()
        .root_element()
        .query_descendants()
        .match_inherits("TextField")
        .find_all()
}
fn centre(e: &ElementHandle) -> LogicalPosition {
    let p = e.absolute_position();
    let s = e.size();
    LogicalPosition::new(p.x + s.width / 2., p.y + s.height / 2.)
}
fn button(h: &Hive, label: &str) -> ElementHandle {
    ElementHandle::find_by_accessible_label(&*h.inner.ui.borrow(), label)
        .find(|e| e.size().width > 0. && e.size().height > 0.)
        .unwrap_or_else(|| panic!("missing real button {label}"))
}
fn field(h: &Hive, index: usize) -> ElementHandle {
    let fields = material_fields(h);
    assert!(
        fields.len() > index,
        "AC3: numeric schema row {index} needs an actual vendored MaterialTextField, got {}",
        fields.len()
    );
    fields[index].clone()
}
fn type_exact(h: &Hive, index: usize, draft: &str) {
    let f = field(h, index);
    click(h, centre(&f));
    down(h, Key::Control);
    text(h, "a");
    up(h, Key::Control);
    key(h, Key::Backspace);
    if !draft.is_empty() {
        text(h, draft);
    }
}
fn field_value(e: &ElementHandle) -> String {
    e.accessible_value()
        .or_else(|| {
            e.query_descendants()
                .match_inherits("TextInput")
                .find_first()
                .and_then(|t| t.accessible_value())
        })
        .unwrap_or_default()
        .to_string()
}
fn bounds(h: &Hive) -> (slint::PhysicalPosition, slint::PhysicalSize) {
    let ui = h.inner.ui.borrow();
    (ui.window().position(), ui.window().size())
}

// REWORK1: reuse the actual generated window, never seed private hint state.
fn rework1_pill_visible(h: &Hive) -> bool {
    ElementHandle::find_by_accessible_label(
        &*h.inner.ui.borrow(),
        stage_two::customize_hint(Lang::En),
    )
    .any(|e| e.size().width > 0. && e.size().height > 0. && e.computed_opacity() > 0.)
}

fn rework1_point(h: &Hive, index: usize) -> LogicalPosition {
    let plan = h.inner.plan.borrow();
    let c = &plan.as_ref().unwrap().cells[index];
    LogicalPosition::new(c.x, c.y)
}

fn rework1_replace(h: &Hive, ending: &str, model: Vec<HiveCell>) {
    match ending {
        "escape" => key(h, Key::Escape),
        "choose" => click(h, rework1_point(h, 1)),
        "close" => h.close(),
        "replace" => (),
        _ => unreachable!(),
    }
    turn();
    if ending != "replace" {
        assert!(!h.inner.visible(), "A must actually be terminal: {ending}");
    }
    h.present_files("B.png", &[PathBuf::from("/selection/B.png")], model, None)
        .unwrap();
    turn();
}

#[test]
fn ac7_ac4_rework1_terminal_or_replacement_has_no_unhovered_stale_pill() {
    let _serial = init();
    let mut stale = Vec::new();
    for ending in ["escape", "choose", "close", "replace"] {
        for variant in ["reordered", "same-index-new-action", "noncustomizable"] {
            let h = test_env::build(&[], Lang::En);
            h.present("A", cells(), None).unwrap();
            turn();
            move_to(&h, rework1_point(&h, 1));
            turn();
            assert!(rework1_pill_visible(&h), "actual A hover must expose pill");
            let mut b = cells();
            match variant {
                "reordered" => b.swap(1, 2),
                "same-index-new-action" => b[1] = b[4].clone(),
                "noncustomizable" => b[1].customize = false,
                _ => unreachable!(),
            }
            rework1_replace(&h, ending, b);
            assert_eq!(h.inner.ui.borrow().get_highlight(), -1);
            // No pointer event has been delivered to B. Inspect real pill items.
            if rework1_pill_visible(&h) {
                stale.push(format!("{ending}/{variant}"));
            }
            h.close();
        }
    }
    assert!(
        stale.is_empty(),
        "AC7/S136 ownership: unhovered B exposes A's stale Customize: {stale:?}"
    );
}

#[test]
fn ac7_ac4_rework1_former_corridor_highlights_actual_new_session_hit() {
    let _serial = init();
    let mut wrong = Vec::new();
    for ending in ["escape", "choose", "close", "replace"] {
        let h = test_env::build(&[], Lang::En);
        h.present("A", cells(), None).unwrap();
        turn();
        let origin = rework1_point(&h, 1);
        move_to(&h, origin);
        turn();
        let target = centre(&button(&h, stage_two::customize_hint(Lang::En)));
        let mut b = cells();
        b.swap(1, 2);
        rework1_replace(&h, ending, b);
        // Find an actual production hit along the former A-to-pill corridor.
        // invoke_pick calls the same wired layout hit detector as PointerMoved.
        let hit = (1..100)
            .find_map(|n| {
                let t = n as f32 / 100.;
                let p = LogicalPosition::new(
                    origin.x + (target.x - origin.x) * t,
                    origin.y + (target.y - origin.y) * t,
                );
                let index = h.inner.ui.borrow().invoke_pick(p.x, p.y);
                (index >= 0 && index != 1).then_some((p, index))
            })
            .expect("fixture corridor must cross a different actual B cell");
        move_to(&h, hit.0);
        let actual = h.inner.ui.borrow().get_highlight();
        if actual != hit.1 {
            wrong.push(format!(
                "{ending}: point={:?}, actual hit={}, highlight={actual}",
                hit.0, hit.1
            ));
        }
        h.close();
    }
    assert!(
        wrong.is_empty(),
        "AC7: retired A corridor steals B's real hit: {wrong:?}"
    );
}

#[test]
fn ac7_ac1_rework1_back_keeps_current_pill_keyboard_and_new_b_click_once() {
    let _serial = init();
    let h = Rc::new(test_env::build(&[], Lang::En));
    let model = Rc::new(RefCell::new(cells()));
    let opened = Rc::new(RefCell::new(Vec::new()));
    let chosen = Rc::new(RefCell::new(Vec::new()));
    let c = chosen.clone();
    h.on_choose(move |id| c.borrow_mut().push(id));
    let weak = Rc::downgrade(&h);
    let o = opened.clone();
    let m = model.clone();
    h.on_customize(move |id| {
        o.borrow_mut().push(id.clone());
        let weak = weak.clone();
        let model = m.borrow().clone();
        slint::Timer::single_shot(Duration::ZERO, move || {
            if let Some(h) = weak.upgrade() {
                h.present_options(model, state(&id, Lang::En)).unwrap();
            }
        });
    });
    h.present("A", model.borrow().clone(), None).unwrap();
    turn();
    let a = rework1_point(&h, 1);
    move_to(&h, a);
    turn();
    let pill = centre(&button(&h, stage_two::customize_hint(Lang::En)));
    for n in 1..=10 {
        let t = n as f32 / 10.;
        move_to(
            &h,
            LogicalPosition::new(a.x + (pill.x - a.x) * t, a.y + (pill.y - a.y) * t),
        );
        assert!(rework1_pill_visible(&h));
    }
    click(&h, pill);
    turn();
    assert!(h.inner.ui.borrow().get_stage_two());
    let back = h.inner.ui.borrow().get_stage_back_text().to_string();
    click(&h, centre(&button(&h, &back)));
    turn();
    assert!(h.inner.visible());
    assert!(!h.inner.ui.borrow().get_stage_two());
    assert_eq!(h.inner.ui.borrow().get_highlight(), 1);
    assert!(
        rework1_pill_visible(&h),
        "same-session Back retains current action's door"
    );
    assert!(chosen.borrow().is_empty());
    // New ownership and reordered actions, then real keyboard navigation.
    model.borrow_mut().swap(1, 2);
    rework1_replace(&h, "escape", model.borrow().clone());
    key(&h, Key::Tab);
    let index = h.inner.ui.borrow().get_highlight() as usize;
    let wanted = model.borrow()[index].id.clone();
    down(&h, Key::Shift);
    key(&h, Key::Return);
    up(&h, Key::Shift);
    turn();
    assert!(h.inner.ui.borrow().get_stage_two());
    assert_eq!(
        opened.borrow().last(),
        Some(&wanted),
        "keyboard must customize B's actual highlight"
    );
    key(&h, Key::Escape);
    turn();
    // Explicit B hover retargets the mouse door. Approach its actual pill.
    let b = rework1_point(&h, 4);
    move_to(&h, b);
    turn();
    let pill = centre(&button(&h, stage_two::customize_hint(Lang::En)));
    for n in 1..=10 {
        let t = n as f32 / 10.;
        move_to(
            &h,
            LogicalPosition::new(b.x + (pill.x - b.x) * t, b.y + (pill.y - b.y) * t),
        );
        assert!(rework1_pill_visible(&h));
    }
    let before = opened.borrow().len();
    click(&h, pill);
    turn();
    assert_eq!(opened.borrow().len(), before + 1);
    assert_eq!(opened.borrow().last(), Some(&model.borrow()[4].id));
    assert!(h.inner.ui.borrow().get_stage_two());
    click(&h, pill);
    turn();
    assert_eq!(
        opened.borrow().len(),
        before + 1,
        "panel click cannot duplicate Customize"
    );
    assert!(
        chosen.borrow().is_empty(),
        "pill and keyboard must never submit defaults"
    );
    key(&h, Key::Escape);
    turn();
    click(&h, b);
    turn();
    click(&h, b);
    turn();
    assert_eq!(
        *chosen.borrow(),
        [model.borrow()[4].id.clone()],
        "plain default click remains exactly once"
    );
}

#[test]
fn ac1_ac4_back_and_first_escape_restore_parent_without_dismissal() {
    let _serial = init();
    for calm in ["0", "1"] {
        for escape in [false, true] {
            let h = test_env::build(&[("KAWARO_REDUCED_MOTION", calm)], Lang::En);
            let dismiss = Rc::new(Cell::new(0));
            let d = dismiss.clone();
            h.on_dismiss(move || d.set(d.get() + 1));
            h.present_files(
                "selected.png",
                &[PathBuf::from("/selection/selected.png")],
                cells(),
                None,
            )
            .unwrap();
            turn();
            h.set_highlight(1);
            let original = bounds(&h);
            open(&h, "image.resize-50", Lang::En);
            assert_eq!(
                bounds(&h),
                original,
                "AC1: panel keeps actual global bounds while open"
            );
            if escape {
                key(&h, Key::Escape);
            } else {
                h.inner.ui.borrow().invoke_stage_cancel();
            }
            turn();
            assert!(
                h.inner.visible(),
                "AC1: Back/first Escape must return to visible actions, not close the hive"
            );
            assert!(!h.inner.ui.borrow().get_stage_two());
            assert_eq!(dismiss.get(), 0, "return is not terminal dismissal");
            assert_eq!(
                h.inner.ui.borrow().get_highlight(),
                1,
                "retain highlighted action"
            );
            assert_eq!(bounds(&h), original, "same global bounds");
            open(&h, "image.resize-50", Lang::En);
            assert_eq!(
                h.inner
                    .ui
                    .borrow()
                    .get_stage_options()
                    .row_data(0)
                    .unwrap()
                    .value,
                50.
            );
            h.inner.ui.borrow().invoke_stage_cancel();
            turn();
            key(&h, Key::Escape);
            turn();
            assert!(!h.inner.visible(), "second Escape is terminal");
            assert_eq!(dismiss.get(), 1);
        }
    }
}

#[test]
fn ac2_ac3_numeric_controls_are_actual_material_fields_at_compact_size() {
    let _serial = init();
    for lang in [Lang::En, Lang::Ar] {
        let h = test_env::build(&[], lang);
        open(&h, "image.resize-50", lang);
        h.inner
            .ui
            .borrow()
            .window()
            .set_size(slint::LogicalSize::new(393., 362.));
        turn();
        let actual_items = all(&h);
        assert!(
            actual_items
                .iter()
                .any(|e| e.type_name().is_some_and(|t| t.contains("ScrollView"))
                    || e.bases()
                        .is_some_and(|mut b| b.any(|t| t.contains("ScrollView")))),
            "AC2: body uses existing Material ScrollView"
        );
        let fields = material_fields(&h);
        assert_eq!(
            fields.len(),
            2,
            "AC3: percent and pixels are native MaterialTextFields, not slider-only numbers"
        );
        for f in &fields {
            assert!(
                f.size().height >= 48.,
                "AC2: Material field intrinsic height: {:?}",
                f.size()
            );
        }
        let a = fields[0].absolute_position();
        let b = fields[1].absolute_position();
        assert!(
            a.y + fields[0].size().height <= b.y || b.y + fields[1].size().height <= a.y,
            "stacked rows never intersect"
        );
    }
}

#[test]
fn ac3_ac4_real_keyboard_exact_values_invalid_drafts_and_return_do_not_run() {
    let _serial = init();
    let h = test_env::build(&[], Lang::En);
    open(&h, "image.resize-50", Lang::En);
    let runs = Rc::new(RefCell::new(Vec::new()));
    let r = runs.clone();
    h.on_options_run(move |id, p| r.borrow_mut().push((id, p)));
    let mut action = kawaro_core::action::Registry::builtin()
        .get("image.resize-50")
        .unwrap()
        .clone();
    action
        .options
        .as_mut()
        .unwrap()
        .presets
        .push(kawaro_core::options::Preset {
            id: "low".into(),
            values: [("scale".into(), toml::Value::Integer(40))]
                .into_iter()
                .collect(),
        });
    h.present_options(
        cells(),
        Rc::new(stage_two::StageTwoState::build(&action, &[], Lang::En, false).unwrap()),
    )
    .unwrap();
    type_exact(&h, 0, "37");
    click(&h, centre(&button(&h, "Reset")));
    turn();
    assert_eq!(
        field_value(&field(&h, 0)),
        "50",
        "Reset synchronizes real control"
    );
    click(&h, centre(&button(&h, "low")));
    turn();
    assert_eq!(
        field_value(&field(&h, 0)),
        "40",
        "preset synchronizes real control"
    );
    assert!(runs.borrow().is_empty());
    type_exact(&h, 0, "37");
    key(&h, Key::Tab);
    down(&h, Key::Control);
    text(&h, "a");
    up(&h, Key::Control);
    text(&h, "641");
    assert_eq!(
        field_value(&field(&h, 1)),
        "641",
        "Tab reaches pixels using real keyboard focus"
    );
    down(&h, Key::Shift);
    key(&h, Key::Tab);
    up(&h, Key::Shift);
    down(&h, Key::Control);
    text(&h, "a");
    up(&h, Key::Control);
    text(&h, "37");
    assert_eq!(
        field_value(&field(&h, 0)),
        "37",
        "Shift+Tab returns to percent"
    );
    for (index, valid, invalid, param) in [
        (
            0,
            vec!["1", "37", "1000"],
            vec!["", "abc", "1.5", "0", "1001"],
            "scale",
        ),
        (
            1,
            vec!["16", "641", "20000"],
            vec!["", "abc", "1.5", "15", "20001"],
            "max_side",
        ),
    ] {
        for v in valid {
            type_exact(&h, index, v);
            key(&h, Key::Return);
            assert_eq!(
                field_value(&field(&h, index)),
                v,
                "exact typed draft retained"
            );
            assert_eq!(h.stage_error(), "");
            assert!(runs.borrow().is_empty(), "Return accepts, never Run");
        }
        for v in invalid {
            type_exact(&h, index, v);
            assert_eq!(
                field_value(&field(&h, index)),
                v,
                "invalid text remains editable, never clamped"
            );
            assert!(
                !h.stage_error().is_empty(),
                "{param}: {v:?} must show resolver error"
            );
            h.inner.ui.borrow().invoke_stage_run();
            assert!(runs.borrow().is_empty(), "invalid Run submits zero");
            assert!(h.inner.visible());
        }
    }
    type_exact(&h, 0, "37");
    click(&h, centre(&button(&h, "Run")));
    assert_eq!(runs.borrow().len(), 1);
    assert_eq!(
        runs.borrow()[0]
            .1
            .get("scale")
            .and_then(toml::Value::as_integer),
        Some(37)
    );
    h.inner.ui.borrow().invoke_stage_run();
    key(&h, Key::Return);
    assert_eq!(runs.borrow().len(), 1);
}

#[test]
fn ac1_ac4_queued_reset_cannot_replace_a_new_panel_in_the_same_session() {
    let _serial = init();
    let h = test_env::build(&[], Lang::En);
    h.present("selected", cells(), None).unwrap();
    let session = crate::session_generation();
    open(&h, "image.resize-50", Lang::En);
    assert_eq!(
        crate::session_generation(),
        session,
        "AC1: opening a panel is an in-place stage, not a new hive session"
    );
    h.inner.ui.borrow().invoke_stage_reset();
    // Queue A's real reset, replace with B before the event loop delivers it.
    h.present_options(cells(), state("image.resize-to", Lang::En))
        .unwrap();
    let before: Vec<_> = h
        .inner
        .ui
        .borrow()
        .get_stage_options()
        .iter()
        .map(|r| r.id)
        .collect();
    turn();
    let after: Vec<_> = h
        .inner
        .ui
        .borrow()
        .get_stage_options()
        .iter()
        .map(|r| r.id)
        .collect();
    assert_eq!(
        after, before,
        "AC4: an old panel Reset cannot overwrite B inside the same session"
    );
}

#[test]
fn ac7_mouse_customization_survives_ten_point_natural_approach() {
    let _serial = init();
    for lang in [Lang::En, Lang::Ar] {
        for index in [1, 0, 2, 3, 4] {
            let h = Rc::new(test_env::build(&[], lang));
            h.present("selected", cells(), None).unwrap();
            turn();
            let chosen = Rc::new(Cell::new(0));
            let c = chosen.clone();
            h.on_choose(move |_| c.set(c.get() + 1));
            let wanted = cells()[index].id.clone();
            let opened = Rc::new(RefCell::new(Vec::<String>::new()));
            let o = opened.clone();
            let weak = Rc::downgrade(&h);
            h.on_customize(move |id| {
                o.borrow_mut().push(id.clone());
                let weak = weak.clone();
                slint::Timer::single_shot(Duration::ZERO, move || {
                    if let Some(h) = weak.upgrade() {
                        h.present_options(cells(), state(&id, lang)).unwrap();
                    }
                });
            });
            let p = {
                let plan = h.inner.plan.borrow();
                let c = &plan.as_ref().unwrap().cells[index];
                LogicalPosition::new(c.x, c.y)
            };
            move_to(&h, p);
            turn();
            let label = stage_two::customize_hint(lang);
            let target = centre(&button(&h, label));
            for n in 1..=10 {
                let t = n as f32 / 10.;
                move_to(
                    &h,
                    LogicalPosition::new(p.x + (target.x - p.x) * t, p.y + (target.y - p.y) * t),
                );
                assert!(
                    ElementHandle::find_by_accessible_label(&*h.inner.ui.borrow(), label)
                        .any(|e| e.size().width > 0. && e.computed_opacity() > 0.),
                    "AC7: Customize disappeared at natural approach point {n} for {wanted}"
                );
            }
            click(&h, target);
            turn();
            assert!(
                h.inner.ui.borrow().get_stage_two(),
                "actual production panel opens after the natural mouse path"
            );
            assert_eq!(
                *opened.borrow(),
                [wanted],
                "Customize keeps intended action"
            );
            assert_eq!(chosen.get(), 0, "no accidental default conversion");
        }
    }
}

#[test]
fn ac1_ac4_old_panel_input_preview_and_reset_dispatch_cannot_touch_return_or_reopen() {
    let _serial = init();
    for replacement in ["return", "same-action", "other-action", "run", "close"] {
        let h = test_env::build(&[], Lang::En);
        h.present("files", cells(), None).unwrap();
        open(&h, "image.resize-50", Lang::En);
        let old = h.stage_stamp();
        dispatch_stage_update(&Rc::downgrade(&h.inner), old, |ui| {
            ui.set_stage_error("live delivery".into())
        });
        assert_eq!(
            h.stage_error(),
            "live delivery",
            "live owner must really reach generated component"
        );
        match replacement {
            "return" => h.inner.ui.borrow().invoke_stage_cancel(),
            "same-action" => {
                h.inner.ui.borrow().invoke_stage_cancel();
                open(&h, "image.resize-50", Lang::En);
            }
            "other-action" => open(&h, "image.resize-to", Lang::En),
            "run" => h.inner.ui.borrow().invoke_stage_run(),
            _ => h.close(),
        }
        turn();
        let before = (
            h.inner.visible(),
            h.inner.ui.borrow().get_stage_two(),
            h.stage_error(),
            h.inner.ui.borrow().get_stage_readout(),
            h.inner
                .ui
                .borrow()
                .get_stage_options()
                .iter()
                .map(|r| (r.id, r.value, r.text))
                .collect::<Vec<_>>(),
        );
        for kind in ["input", "preview", "reset"] {
            dispatch_stage_update(&Rc::downgrade(&h.inner), old, move |ui| match kind {
                "input" => ui.invoke_option_changed(0, 999., "999".into()),
                "preview" => ui.set_stage_readout("STALE PREVIEW RESULT".into()),
                _ => ui.invoke_stage_reset(),
            });
        }
        turn();
        let after = (
            h.inner.visible(),
            h.inner.ui.borrow().get_stage_two(),
            h.stage_error(),
            h.inner.ui.borrow().get_stage_readout(),
            h.inner
                .ui
                .borrow()
                .get_stage_options()
                .iter()
                .map(|r| (r.id, r.value, r.text))
                .collect::<Vec<_>>(),
        );
        assert_eq!(
            after, before,
            "old input/preview/Reset changed {replacement}"
        );
    }
}

#[test]
fn ac2_ac3_production_geometry_matrix_has_readable_stacked_rows_and_fixed_footer() {
    let _serial = init();
    for lang in [Lang::En, Lang::Ar] {
        for dark in [false, true] {
            for id in [
                "image.resize-50",
                "image.resize-to",
                "image.crop",
                "image.compress-to",
            ] {
                let h = test_env::build(&[], lang);
                open(&h, id, lang);
                h.inner.ui.borrow().global::<Tokens>().set_dark(dark);
                h.inner
                    .ui
                    .borrow()
                    .window()
                    .set_size(slint::LogicalSize::new(393., 362.));
                turn();
                let observed = stage_geometry(&h);
                let named = |name: &str| {
                    observed
                        .iter()
                        .find(|(n, _, _)| n == name)
                        .unwrap_or_else(|| panic!("missing real item {name} for {id}"))
                };
                let viewport = named("viewport").1;
                let footer = named("footer").1;
                assert!(
                    footer.y >= viewport.y + viewport.h - 1. && footer.y + footer.h <= 363.,
                    "footer outside scrolling body and in window: {footer:?}"
                );
                let rows: Vec<_> = observed
                    .iter()
                    .filter(|(n, _, _)| n.starts_with("row:"))
                    .collect();
                assert!(!rows.is_empty());
                for pair in rows.windows(2) {
                    assert!(
                        pair[0].1.y + pair[0].1.h <= pair[1].1.y + 1.,
                        "stacked rows intersect: {pair:?}"
                    );
                }
                for (name, bounds, glyphs) in &observed {
                    if name.starts_with("label:") || name == "readout" || name == "error" {
                        assert!(
                            glyphs.0 <= bounds.w + 1. && glyphs.1 <= bounds.h + 1.,
                            "actual glyph clipping {lang:?}/{id}/{name}: {glyphs:?} in {bounds:?}"
                        );
                    }
                    if name.starts_with("control:") {
                        assert!(
                            bounds.h >= 48.,
                            "native Material minimum height {name}: {bounds:?}"
                        );
                    }
                }
                let kinds = schema_kinds(id);
                if kinds.iter().any(|kind| is_numeric(*kind)) {
                    let f = field(&h, 0);
                    click(&h, centre(&f));
                    key(&h, Key::Tab);
                    down(&h, Key::Shift);
                    key(&h, Key::Tab);
                    up(&h, Key::Shift);
                    type_exact(&h, 0, "abc");
                    assert!(!h.stage_error().is_empty());
                    assert_eq!(
                        stage_geometry(&h)
                            .iter()
                            .find(|(n, _, _)| n == "footer")
                            .unwrap()
                            .1,
                        footer,
                        "invalid/focus does not move footer"
                    );
                } else {
                    // Crop: enum and bool controls have no numeric draft.
                    // Exercise their real focus without inventing fields.
                    let geometry = stage_geometry(&h);
                    let control = geometry
                        .iter()
                        .find(|(n, _, _)| n.starts_with("control:"))
                        .unwrap_or_else(|| panic!("expected enum/bool controls for {id}"));
                    let bounds = control.1;
                    click(
                        &h,
                        LogicalPosition::new(bounds.x + bounds.w / 2., bounds.y + bounds.h / 2.),
                    );
                    key(&h, Key::Tab);
                    down(&h, Key::Shift);
                    key(&h, Key::Tab);
                    up(&h, Key::Shift);
                    assert!(
                        h.stage_error().is_empty(),
                        "enum/bool focus makes no draft error"
                    );
                    assert_eq!(
                        stage_geometry(&h)
                            .iter()
                            .find(|(n, _, _)| n == "footer")
                            .unwrap()
                            .1,
                        footer,
                        "enum/bool focus does not move footer"
                    );
                }
            }
        }
    }
}

// Maintainer-approved extra cycle after REWORK2: observe the generated controls,
// not reconstructed RTL flags or a second palette.
fn rework2_enum(h: &Hive) -> (usize, StageOption) {
    h.inner
        .ui
        .borrow()
        .get_stage_options()
        .iter()
        .enumerate()
        .find(|(_, row)| row.kind == StageKind::Enum)
        .expect("real enum schema row")
}

fn rework2_choice(h: &Hive, choice: usize) -> (ElementHandle, ElementHandle) {
    let (_, row) = rework2_enum(h);
    let label = row.choices.row_data(choice).expect("real choice").text;
    // Queries exclude invisible items. Wheel the real ScrollView until both
    // actual items are wholly inside its viewport, rather than inventing bounds.
    for _ in 0..45 {
        turn();
        let viewport = stage_geometry(h)
            .into_iter()
            .find(|(n, _, _)| n == "viewport")
            .unwrap()
            .1;
        let radios = h
            .inner
            .ui
            .borrow()
            .root_element()
            .query_descendants()
            .match_inherits("RadioButton")
            .find_all();
        let labels = h
            .inner
            .ui
            .borrow()
            .root_element()
            .query_descendants()
            .match_inherits("Text")
            .match_predicate({
                let label = label.clone();
                move |e| e.accessible_label().as_ref() == Some(&label)
            })
            .find_all();
        for t in labels {
            for radio in &radios {
                let p = radio.absolute_position();
                let s = radio.size();
                let tp = t.absolute_position();
                let ts = t.size();
                if (p.y + s.height / 2. - tp.y - ts.height / 2.).abs() < 2.
                    && s.width > 0.
                    && s.height > 0.
                    && ts.width > 0.
                    && ts.height > 0.
                    && p.y >= viewport.y
                    && p.y + s.height <= viewport.y + viewport.h
                    && tp.y >= viewport.y
                    && tp.y + ts.height <= viewport.y + viewport.h
                {
                    return (radio.clone(), t);
                }
            }
        }
        event(
            h,
            WindowEvent::PointerScrolled {
                position: LogicalPosition::new(
                    viewport.x + viewport.w / 2.,
                    viewport.y + viewport.h / 2.,
                ),
                delta_x: 0.,
                delta_y: -12.,
            },
        );
    }
    panic!("first enum choice {label} was not brought into the actual viewport");
}

#[test]
fn ac2_rework2_actual_enum_radio_text_geometry_reverses_in_arabic() {
    let _serial = init();
    let mut failures = Vec::new();
    for id in ["image.crop", "image.resize-to"] {
        for lang in [Lang::En, Lang::Ar] {
            let h = test_env::build(&[], lang);
            open(&h, id, lang);
            h.inner
                .ui
                .borrow()
                .window()
                .set_size(slint::LogicalSize::new(393., 362.));
            let (radio, label) = rework2_choice(&h, 0);
            let rx = radio.absolute_position().x;
            let tx = label.absolute_position().x;
            eprintln!("AC2 {id}/{lang:?}: actual RadioButton x={rx}, Text x={tx}");
            let ordered = if lang == Lang::Ar {
                tx + label.size().width <= rx
            } else {
                rx + radio.size().width <= tx
            };
            if !ordered {
                failures.push(format!(
                    "{id}/{lang:?}: RadioButton x={rx}, Text x={tx}, not mirrored"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "AC2 actual enum geometry: {failures:?}"
    );
}

#[test]
fn ac2_ac3_rework2_actual_radio_enabled_click_and_space_update_options_without_run() {
    let _serial = init();
    let mut disabled = Vec::new();
    for id in ["image.crop", "image.resize-to"] {
        for lang in [Lang::En, Lang::Ar] {
            let h = test_env::build(&[], lang);
            open(&h, id, lang);
            h.inner
                .ui
                .borrow()
                .window()
                .set_size(slint::LogicalSize::new(393., 362.));
            let jobs = Rc::new(Cell::new(0));
            let j = jobs.clone();
            h.on_options_run(move |_, _| j.set(j.get() + 1));
            let (index, initial) = rework2_enum(&h);
            let (first, _) = rework2_choice(&h, 0);
            let first_enabled = first.accessible_enabled();
            if first_enabled != Some(true) {
                disabled.push(format!(
                    "{id}/{lang:?}: actual accessible-enabled={:?}",
                    first.accessible_enabled()
                ));
            }
            assert_eq!(first.accessible_checked(), Some(initial.selected == 0));
            #[cfg(target_os = "linux")]
            let updates = Box::pin(i_slint_core::model::ModelChangeListenerContainer::new(
                Rework2ModelUpdates {
                    index,
                    changes: Cell::new(0),
                },
            ));
            #[cfg(target_os = "linux")]
            h.inner
                .ui
                .borrow()
                .get_stage_options()
                .model_tracker()
                .attach_peer(updates.as_ref().model_peer());
            let (second, _) = rework2_choice(&h, 1);
            click(&h, centre(&second));
            turn();
            assert_eq!(
                h.inner
                    .ui
                    .borrow()
                    .get_stage_options()
                    .row_data(index)
                    .unwrap()
                    .selected,
                1,
                "normal pointer event updates real options model"
            );
            assert_eq!(rework2_choice(&h, 1).0.accessible_checked(), Some(true));
            #[cfg(target_os = "linux")]
            assert_eq!(
                updates.changes.get(),
                1,
                "one real options-model update per click"
            );
            eprintln!(
                "AC2 {id}/{lang:?}: actual accessible-enabled={first_enabled:?}; click selected=1, one model update, jobs={}",
                jobs.get()
            );
            assert_eq!(jobs.get(), 0, "pointer choice edit never submits a job");
            // Do not demand keyboard focus from a control already proven disabled.
            // The final assertion still fails every such case; after correction
            // the same test must exercise Space and its second model notification.
            if first_enabled != Some(true) {
                continue;
            }
            // Return to the first, now unselected choice using ordinary focus
            // traversal and Space, without accessibility action or callback calls.
            let (first, _) = rework2_choice(&h, 0);
            #[cfg(target_os = "linux")]
            rework2_focus_radio(&h, &first);
            #[cfg(not(target_os = "linux"))]
            {
                down(&h, Key::Shift);
                key(&h, Key::Tab);
                up(&h, Key::Shift);
            }
            text(&h, " ");
            turn();
            assert_eq!(
                h.inner
                    .ui
                    .borrow()
                    .get_stage_options()
                    .row_data(index)
                    .unwrap()
                    .selected,
                0,
                "Space updates real first choice, not just a checked decoration"
            );
            assert_eq!(rework2_choice(&h, 0).0.accessible_checked(), Some(true));
            assert_eq!(rework2_choice(&h, 1).0.accessible_checked(), Some(false));
            turn();
            assert_eq!(
                h.inner
                    .ui
                    .borrow()
                    .get_stage_options()
                    .row_data(index)
                    .unwrap()
                    .selected,
                0,
                "key release/timer turn do not apply a second choice change"
            );
            #[cfg(target_os = "linux")]
            assert_eq!(
                updates.changes.get(),
                2,
                "one additional real options-model update per Space, no duplicate release"
            );
            assert_eq!(jobs.get(), 0, "editing choices never submits a job");
        }
    }
    assert!(
        disabled.is_empty(),
        "AC2 actual radios must be enabled: {disabled:?}"
    );
}

#[cfg(target_os = "linux")]
fn rework2_focus_radio(h: &Hive, radio: &ElementHandle) {
    for _ in 0..64 {
        let p = radio.absolute_position();
        let s = radio.size();
        let focused = {
            let ui = h.inner.ui.borrow();
            i_slint_core::window::WindowInner::from_pub(ui.window())
                .window_item_rc()
                .unwrap()
                .visit_descendants(|item| {
                    if item
                        .downcast::<i_slint_core::items::FocusScope>()
                        .is_some_and(|scope| scope.as_pin_ref().has_focus())
                    {
                        let origin = item.map_to_window(Default::default());
                        if origin.x >= p.x
                            && origin.x < p.x + s.width
                            && origin.y >= p.y
                            && origin.y < p.y + s.height
                        {
                            return std::ops::ControlFlow::Break(true);
                        }
                    }
                    std::ops::ControlFlow::Continue(())
                })
                .unwrap_or(false)
        };
        if focused {
            return;
        }
        key(h, Key::Tab);
    }
    panic!("Tab did not reach the actual first RadioButton FocusScope");
}

#[cfg(target_os = "linux")]
struct Rework2ModelUpdates {
    index: usize,
    changes: Cell<usize>,
}
#[cfg(target_os = "linux")]
impl i_slint_core::model::ModelChangeListener for Rework2ModelUpdates {
    fn row_changed(self: std::pin::Pin<&Self>, row: usize) {
        if row == self.index {
            self.changes.set(self.changes.get() + 1);
        }
    }
    fn row_added(self: std::pin::Pin<&Self>, _: usize, _: usize) {}
    fn row_removed(self: std::pin::Pin<&Self>, _: usize, _: usize) {}
    fn reset(self: std::pin::Pin<&Self>) {
        panic!("enum edit must update its real row, not reset the model");
    }
}

thread_local! {
    static REWORK2_RENDER_WINDOW: RefCell<Option<Rc<slint::platform::software_renderer::MinimalSoftwareWindow>>> = const { RefCell::new(None) };
}
struct Rework2SoftwarePlatform;
impl slint::platform::Platform for Rework2SoftwarePlatform {
    fn create_window_adapter(
        &self,
    ) -> Result<Rc<dyn slint::platform::WindowAdapter>, slint::PlatformError> {
        let w = slint::platform::software_renderer::MinimalSoftwareWindow::new(
            slint::platform::software_renderer::RepaintBufferType::NewBuffer,
        );
        REWORK2_RENDER_WINDOW.with(|slot| *slot.borrow_mut() = Some(w.clone()));
        Ok(w)
    }
}

#[test]
fn ac2_rework2_forced_scheme_renders_actual_material_colors_from_tokens() {
    // Real software renderer, isolated from the mock renderer used for geometry.
    slint::platform::set_platform(Box::new(Rework2SoftwarePlatform)).unwrap();
    let _serial = TESTS.lock().unwrap_or_else(|e| e.into_inner());
    let tokens: serde_json::Value =
        serde_json::from_str(include_str!("../../../design/tokens.json")).unwrap();
    let mut failures = Vec::new();
    for scheme in ["dark", "light"] {
        let h = test_env::build(
            &[
                ("KAWARO_COLOR_SCHEME", scheme),
                ("KAWARO_REDUCED_MOTION", "1"),
            ],
            Lang::En,
        );
        open(&h, "image.resize-to", Lang::En);
        let ui = h.inner.ui.borrow();
        assert_eq!(
            ui.global::<Tokens>().get_dark(),
            scheme == "dark",
            "existing token override is active"
        );
        // HiveWindow does not export Palette or MaterialPalette getters.
        // Observe their actual consumer pixels, not a parallel palette.
        ui.window().set_size(slint::LogicalSize::new(393., 362.));
        drop(ui);
        turn();
        let f = field(&h, 0);
        let pos = f.absolute_position();
        let size = f.size();
        let win = REWORK2_RENDER_WINDOW
            .with(|slot| slot.borrow().clone())
            .unwrap();
        let dimensions = win.window().size();
        let mut pixels =
            vec![slint::Rgb8Pixel::default(); (dimensions.width * dimensions.height) as usize];
        win.window().request_redraw();
        assert!(win.draw_if_needed(|renderer| {
            renderer.render(&mut pixels, dimensions.width as usize);
        }));
        let rgb = |key: &str| -> [u8; 3] {
            let hex = tokens["color"][scheme][key]
                .as_str()
                .unwrap()
                .trim_start_matches('#');
            [
                u8::from_str_radix(&hex[0..2], 16).unwrap(),
                u8::from_str_radix(&hex[2..4], 16).unwrap(),
                u8::from_str_radix(&hex[4..6], 16).unwrap(),
            ]
        };
        let expected_fill = rgb("surface-3");
        let expected_ink = rgb("text-primary");
        let mut fill = 0;
        let mut ink = 0;
        let mut total = 0;
        // Read the actual TextField interior, excluding its border and corners.
        for y in (pos.y + 6.) as u32..(pos.y + size.height - 6.) as u32 {
            for x in (pos.x + 6.) as u32..(pos.x + size.width - 6.) as u32 {
                let p = pixels[(y * dimensions.width + x) as usize];
                let c = [p.r, p.g, p.b];
                fill += usize::from(c == expected_fill);
                ink += usize::from(c == expected_ink);
                total += 1;
            }
        }
        eprintln!(
            "AC2 {scheme}: real Material TextField token surface-3 pixels={fill}/{total}, text-primary pixels={ink}"
        );
        if fill * 2 <= total || ink == 0 {
            failures.push(format!("{scheme}: rendered Material TextField does not use token surface-3/text-primary: fill={fill}/{total}, ink={ink}"));
        }
    }
    assert!(
        failures.is_empty(),
        "AC2 coherent forced Material theme: {failures:?}"
    );
}
