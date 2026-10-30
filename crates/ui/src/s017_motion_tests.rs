//! S-017b and S-017c at the UI level (AC4 to AC9, AC13): the hive really
//! consumes the motion and glass tokens of design/tokens.json. Written from
//! the acceptance criteria of backlog/S-017-glass-hive-motion.md and its
//! manager decision section, before implementation.
//!
//! Contract these tests pin for the implementer (do not edit):
//!
//! - `CellData` gains `ring: int`: the cell's slot ring, the axial distance
//!   of its slot from the centre (1 to 4). The centre and stack hexagons
//!   pass `ring: 0` to `HexCell`.
//! - `HexCell` (crates/ui/components/hex-cell.slint) gains `ring: int`,
//!   `opened: bool`, `pressed: bool`, `glass: bool`, `draw-fill: bool` and
//!   `draw-label: bool`. Its open binds opacity 0 and
//!   `Tokens.hive-scale-from` and, in the `open` state (when `opened`), 1
//!   and `Tokens.hive-scale-to` with `in { animate ... }` over
//!   `Tokens.hive-open` and `Tokens.hive-open-easing`, delayed by
//!   `ring * Tokens.ring-delay`, so a reset to hidden is instant and only
//!   entering open animates. Press scales the cell to
//!   `Tokens.press-scale-to` over `Tokens.press` and back to 1, never under
//!   reduced motion. With `glass` the cell fill is `Tokens.glass-2`
//!   (`Tokens.glass-3` for the centre) and the edge is the fill's own
//!   `border-hex` stroke (an opaque hexagon under a translucent fill would
//!   make the cell opaque); otherwise the opaque hex-fill tokens and the
//!   edge hexagon as today. The press scale lives on an inner
//!   `press-wrap`, so the tests read it there.
//! - `HiveWindow` (crates/ui/ui/hive.slint) gains `closing: bool` (true
//!   during the 90 ms close; the layer fades to
//!   `Tokens.hive-close-opacity-to` and scales to
//!   `Tokens.hive-close-scale-to` over `Tokens.hive-close` with
//!   `Tokens.hive-close-easing`) and `glass: bool` (passed to every cell).
//!   One accent highlight element with id `highlight` slides between cell
//!   centres (`animate x, y` with `Tokens.highlight-move` and
//!   `Tokens.highlight-move-easing`), shown only while `highlight >= 0`.
//!   The per-cell fill and label are separate instances with ids
//!   `cell-hex` and `cell-label`, so the sliding accent hexagon passes
//!   under the labels and over the fills.
//! - `Hive::set_glass(&self, bool)` sets the glass fills on the shown hive,
//!   and `KAWARO_GLASS=1|true|yes|on` sets them at build (`Hive::new`).
//! - The close: `on_choose` and `on_dismiss` run in the same turn as today
//!   and the surface stays for 90 ms (0 with reduced motion), one callback
//!   per show; `present` during the close cancels it; a probing drop show
//!   and `Hive::close()` hide at once.
//!
//! Measured contract (manager decision 2 supersedes AC5's 25 percent line):
//! the highlight centre is on the segment between the two cell centres at
//! 70 ms and within 0.5 px of the new centre at 140 ms plus one frame.

use super::*;
use i_slint_backend_testing::{ElementHandle, ElementRoot};
use slint::LogicalPosition;
use slint::platform::{Key, PointerEventButton, WindowEvent};

/// Test cells: seven actions, so the plan keeps six in ring 1 and puts one
/// in ring 2 (layout.rs: ring-1 overflow moves to ring 2).
fn seven() -> Vec<HiveCell> {
    (0..7)
        .map(|i| HiveCell {
            id: format!("c{i}"),
            code: format!("C{i}"),
            label: format!("label {i}"),
            icon: String::new(),
            ring: 1,
            enabled: true,
            reason: String::new(),
        })
        .collect()
}

fn wired() -> Hive {
    test_env::build(&[], Lang::En)
}

fn plan(hive: &Hive) -> layout::Plan {
    hive.inner.plan.borrow().clone().unwrap()
}

fn turn(ms: u64) {
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(ms));
}

/// All elements whose id contains `needle` (repeated instances may carry an
/// index in their debug id).
fn by_id(hive: &Hive, needle: &str) -> Vec<ElementHandle> {
    let needle = needle.to_string();
    let ui = hive.inner.ui.borrow();
    ui.root_element()
        .query_descendants()
        .match_predicate(move |e| {
            e.id().is_some_and(|i| i.contains(&needle)) && e.size().width > 0.0
        })
        .find_all()
}

/// The top-left corner of `e` in window coordinates.
fn top_left(e: &ElementHandle) -> (f32, f32) {
    let p = e.absolute_position();
    (p.x, p.y)
}

/// Scale of a cell instance from its top-left corner: a scale `s` around the
/// element's own centre moves the corner to `centre - (w/2) * s`.
fn scale_of(e: &ElementHandle, centre: (f32, f32)) -> f32 {
    let (x, _) = top_left(e);
    let half = e.size().width / 2.0;
    ((centre.0 - x) / half).clamp(0.0, 1.5)
}

/// The one instance among `found` whose cell is at `centre`. Scales run
/// around the element centre, so the corner stays within a few pixels of
/// its unscaled place (0.92 to 1.04 of w/2).
fn at(found: &[ElementHandle], centre: (f32, f32)) -> ElementHandle {
    let mut hits: Vec<ElementHandle> = found
        .iter()
        .filter(|e| {
            let (x, y) = top_left(e);
            let (w, h) = (e.size().width / 2.0, e.size().height / 2.0);
            (x - (centre.0 - w)).abs() < 6.0 && (y - (centre.1 - h)).abs() < 6.0
        })
        .cloned()
        .collect();
    assert_eq!(hits.len(), 1, "exactly one instance at {centre:?}");
    hits.pop().unwrap()
}

/// Ends the opening animation so element boxes are where they stay.
fn settle_hive(hive: &Hive) {
    hive.inner.ui.borrow().set_opened(true);
    turn(1000);
}

/// Reads every element's opacity and position, as the first drawn frame
/// would: Slint evaluates animated bindings lazily, so the hidden state of
/// the open must be materialised before `opened` flips again (the real
/// window draws its first frame in between; the layer path waits for it).
fn first_frame(hive: &Hive) {
    let ui = hive.inner.ui.borrow();
    for e in ui
        .root_element()
        .query_descendants()
        .match_predicate(|_| true)
        .find_all()
    {
        e.computed_opacity();
        e.absolute_position();
    }
}

// ------------------------------------------------------------------------
// Harness sanity: the testing platform sees repeated elements and reports
// positions that reflect `transform-scale` (0.92 while the open is pending).

#[test]
fn harness_sees_repeated_cells_and_transforms() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = wired();
    hive.present("x", seven(), None).unwrap();
    first_frame(&hive);
    // The opener has not run: the hive is still closed at hive-scale-from.
    turn(200);
    let hexes: Vec<ElementHandle> = {
        let ui = hive.inner.ui.borrow();
        ui.root_element()
            .query_descendants()
            .match_predicate(|e| e.type_name().is_some_and(|t| t == "HexCell"))
            .find_all()
    };
    assert!(
        hexes.len() >= 8,
        "the harness sees the repeated HexCell instances: {}",
        hexes.len()
    );
    let p = plan(&hive);
    let centre = p.layout.origin;
    let e = at(&by_id(&hive, "centre"), centre);
    let s = scale_of(&e, centre);
    assert!(
        (s - 0.92).abs() < 0.02,
        "transform-scale shows in the position: scale {s}"
    );
    // Settled: full size, corner exactly at centre minus half.
    settle_hive(&hive);
    let e = at(&by_id(&hive, "centre"), centre);
    let (x, y) = top_left(&e);
    let (w, h) = (e.size().width / 2.0, e.size().height / 2.0);
    assert!(
        (x - (centre.0 - w)).abs() < 0.5 && (y - (centre.1 - h)).abs() < 0.5,
        "settled position maps to the plan: ({x}, {y}) for {centre:?}"
    );
}

// ------------------------------------------------------------------------
// The .slint consumes every token in its role, not just in the generated
// Tokens global (which tests/glass_tokens.rs covers).

fn hive_src() -> &'static str {
    include_str!("../ui/hive.slint")
}

fn cell_src() -> &'static str {
    include_str!("../components/hex-cell.slint")
}

/// The `animate` block that mentions `needle`, as tight text.
fn animate_block(src: &str, needle: &str) -> String {
    let mut out = String::new();
    let mut in_block = false;
    let mut depth = 0;
    for line in src.lines() {
        let code = line.split("//").next().unwrap_or("");
        if !in_block && code.contains("animate") {
            in_block = true;
            depth = 0;
            out.clear();
        }
        if in_block {
            out.push_str(code.trim());
            out.push(' ');
            depth += code.matches('{').count() as i32;
            depth -= code.matches('}').count() as i32;
            if depth <= 0 && code.contains('}') {
                if out.contains(needle) {
                    return out;
                }
                in_block = false;
            }
        }
    }
    panic!("no `animate` block containing `{needle}` in the .slint sources");
}

#[test]
fn hive_slint_delays_the_open_of_every_cell_by_its_ring() {
    let block = animate_block(cell_src(), "Tokens.ring-delay");
    assert!(
        block.contains("root.ring * Tokens.ring-delay"),
        "the open delay is the cell ring times Tokens.ring-delay: {block}"
    );
    assert!(
        block.contains("duration: Tokens.hive-open;"),
        "the open runs over Tokens.hive-open: {block}"
    );
    assert!(
        block.contains("easing: Tokens.hive-open-easing;"),
        "the open eases with Tokens.hive-open-easing: {block}"
    );
    // Opacity 0 to 1 and scale hive-scale-from to hive-scale-to per cell.
    let cell = cell_src();
    assert!(cell.contains("opacity: 0"), "cells start at opacity 0");
    assert!(
        cell.contains("root.opacity: 1"),
        "the open state reaches opacity 1"
    );
    assert!(
        cell.contains("Tokens.hive-scale-from"),
        "the cells scale from Tokens.hive-scale-from"
    );
    assert!(
        cell.contains("root.transform-scale: Tokens.hive-scale-to"),
        "the open state scales to Tokens.hive-scale-to"
    );
}

#[test]
fn hive_slint_slides_one_accent_highlight_between_cell_centres() {
    let src = hive_src();
    assert!(
        src.contains("highlight :="),
        "hive.slint has the one sliding highlight element"
    );
    let block = animate_block(src, "Tokens.highlight-move");
    assert!(
        block.contains("duration: Tokens.highlight-move;"),
        "the highlight moves over Tokens.highlight-move: {block}"
    );
    assert!(
        block.contains("easing: Tokens.highlight-move-easing;"),
        "the highlight eases with Tokens.highlight-move-easing: {block}"
    );
    assert!(
        block.contains("animate x, y"),
        "the highlight animates its position: {block}"
    );
    // Exactly one animation uses the highlight duration.
    assert_eq!(
        src.matches("Tokens.highlight-move;").count(),
        1,
        "one highlight animation in hive.slint"
    );
}

#[test]
fn hive_slint_presses_the_cell_with_the_press_tokens() {
    let cell = cell_src();
    let block = animate_block(cell, "Tokens.press;");
    assert!(
        block.contains("duration: Tokens.press;"),
        "the press runs over Tokens.press: {block}"
    );
    assert!(
        cell.contains("Tokens.press-scale-to"),
        "the press scales to Tokens.press-scale-to"
    );
    assert!(
        cell.contains("!Tokens.reduced-motion) ? Tokens.press-scale-to"),
        "the press does not scale under reduced motion"
    );
}

#[test]
fn hive_slint_closes_with_the_hive_close_tokens() {
    let src = hive_src();
    let block = animate_block(src, "Tokens.hive-close-easing");
    assert!(
        block.contains("duration: Tokens.hive-close;"),
        "the close runs over Tokens.hive-close: {block}"
    );
    assert!(
        block.contains("easing: Tokens.hive-close-easing;"),
        "the close eases with Tokens.hive-close-easing: {block}"
    );
    for token in ["Tokens.hive-close-scale-to", "Tokens.hive-close-opacity-to"] {
        assert!(src.contains(token), "hive.slint uses {token}");
    }
}

#[test]
fn hex_cell_slint_fills_the_glass_surfaces_from_the_glass_tokens() {
    let cell = cell_src();
    for token in ["Tokens.glass-2", "Tokens.glass-3"] {
        assert!(cell.contains(token), "hex-cell.slint uses {token}");
    }
    assert!(
        cell.contains("root.glass ? Tokens.glass-2"),
        "a cell is glass-2 only when glass is on"
    );
    assert!(
        cell.contains("root.glass ? Tokens.glass-3"),
        "the centre is glass-3 only when glass is on"
    );
    assert!(
        hive_src().contains("glass: root.glass"),
        "hive.slint passes its glass property to the cells"
    );
}

#[test]
fn the_slint_sources_write_no_literal_duration_or_easing() {
    for (name, src) in [("hive.slint", hive_src()), ("hex-cell.slint", cell_src())] {
        for line in src.lines().map(str::trim) {
            let code = line.split("//").next().unwrap_or("");
            assert!(
                !code.contains("ms;"),
                "{name} writes a literal duration: {line}"
            );
            for easing in [
                "ease-out",
                "ease-in",
                "ease-in-out",
                "linear",
                "cubic-bezier",
            ] {
                assert!(
                    !code.contains(easing),
                    "{name} writes a literal easing: {line}"
                );
            }
        }
    }
}

// ------------------------------------------------------------------------
// Behaviour on the headless platform (mock time).

#[test]
fn open_is_ring_by_ring_and_every_ring_finishes_at_120_plus_40_r_ms() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = wired();
    hive.present("x", seven(), None).unwrap();
    // The first frame draws the hidden hive, then the opener starts the
    // open (window path, one turn).
    first_frame(&hive);
    turn(1);
    let p = plan(&hive);
    assert!(hive.inner.ui.borrow().get_opened(), "the open has started");

    let op = |centre: (f32, f32)| at(&by_id(&hive, "cell-hex"), centre).computed_opacity();
    let ring1 = (p.cells[0].x, p.cells[0].y);
    let ring2 = (p.cells[6].x, p.cells[6].y);
    let centre = p.layout.origin;
    let op_centre = |centre: (f32, f32)| at(&by_id(&hive, "centre"), centre).computed_opacity();

    // At the first frame everything is hidden at hive-scale-from.
    for c in &p.cells {
        let at_ = (c.x, c.y);
        let e = at(&by_id(&hive, "cell-hex"), at_);
        assert_eq!(e.computed_opacity(), 0.0, "cell at {at_:?} starts hidden");
        assert!(
            (scale_of(&e, at_) - 0.92).abs() < 0.01,
            "cell at {at_:?} starts at hive-scale-from"
        );
    }

    // 100 ms in: ring 1 is further along than ring 2, the centre (ring 0)
    // ahead of both.
    turn(100);
    let (o0, o1, o2) = (op_centre(centre), op(ring1), op(ring2));
    assert!(o1 > o2, "ring 1 ({o1}) is ahead of ring 2 ({o2}) at 100 ms");
    assert!(
        o0 > o1,
        "the centre ({o0}) is ahead of ring 1 ({o1}) at 100 ms"
    );

    // At 120 + 40 * 2 ms after the first frame every cell of this two-ring
    // hive is at opacity 1 and scale 1.
    turn(120);
    for c in &p.cells {
        let at_ = (c.x, c.y);
        let e = at(&by_id(&hive, "cell-hex"), at_);
        assert_eq!(e.computed_opacity(), 1.0, "{at_:?} fully visible");
        assert!(
            (scale_of(&e, at_) - 1.0).abs() < 0.01,
            "{at_:?} at full scale"
        );
    }
}

/// The one sliding accent highlight, wherever it currently is.
fn highlight(hive: &Hive) -> ElementHandle {
    let found = by_id(hive, "highlight");
    assert_eq!(found.len(), 1, "exactly one sliding highlight");
    found.into_iter().next().unwrap()
}

/// The centre of an element in window coordinates (the highlight holds no
/// transform of its own: its scale is always 1).
fn centre_of_element(e: &ElementHandle) -> (f32, f32) {
    let (x, y) = top_left(e);
    (x + e.size().width / 2.0, y + e.size().height / 2.0)
}

#[test]
fn the_highlight_slides_between_cell_centres_for_keyboard_and_mouse() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = wired();
    hive.present("x", seven(), None).unwrap();
    settle_hive(&hive);
    let p = plan(&hive);
    let a = (p.cells[0].x, p.cells[0].y);
    let b = (p.cells[1].x, p.cells[1].y);

    // First appearance is in place: no slide from anywhere.
    key(&hive, Key::Tab);
    let hl = highlight(&hive);
    let (x, y) = top_left(&hl);
    let half = (hl.size().width / 2.0, hl.size().height / 2.0);
    assert!(
        (x + half.0 - a.0).abs() < 0.5 && (y + half.1 - a.1).abs() < 0.5,
        "the first highlight is in place at {a:?}"
    );

    // A move to the neighbour slides over Tokens.highlight-move (140 ms):
    // at 70 ms its centre is still on the segment, at 140 ms plus one frame
    // at the new centre (manager decision 2).
    key(&hive, Key::Tab);
    assert_eq!(hive.inner.ui.borrow().get_highlight(), 1);
    turn(70);
    let hl = highlight(&hive);
    let c = centre_of_element(&hl);
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let t = ((c.0 - a.0) * dx + (c.1 - a.1) * dy) / (dx * dx + dy * dy);
    assert!(
        t > 0.0 && t < 1.0,
        "at 70 ms the highlight centre {c:?} is on the segment {a:?} to {b:?} (t = {t})"
    );
    turn(71);
    let c = centre_of_element(&highlight(&hive));
    assert!(
        (c.0 - b.0).abs() < 0.5 && (c.1 - b.1).abs() < 0.5,
        "at 141 ms the highlight centre {c:?} is at the new centre {b:?}"
    );

    // The mouse drives the same sliding element: it slides to the cell and
    // settles there.
    let (cx, cy) = (p.cells[2].x, p.cells[2].y);
    hive.inner
        .ui
        .borrow()
        .window()
        .dispatch_event(WindowEvent::PointerMoved {
            position: LogicalPosition::new(cx, cy),
        });
    turn(141);
    let c = centre_of_element(&highlight(&hive));
    assert!(
        (c.0 - cx).abs() < 0.5 && (c.1 - cy).abs() < 0.5,
        "the mouse slides the highlight to its cell: {c:?}"
    );

    // Leaving all cells hides it in place: it just disappears.
    hive.inner.ui.borrow().set_highlight(-1);
    assert!(
        by_id(&hive, "highlight").is_empty(),
        "no highlight without a highlighted cell"
    );
}

/// The scale of the pressed cell, from the `press-wrap` instances at its
/// centre (the fill and the label move together).
fn press_scale(hive: &Hive, centre: (f32, f32)) -> f32 {
    let found = by_id(hive, "press-wrap");
    let scales: Vec<f32> = found
        .iter()
        .filter(|e| {
            let (x, y) = top_left(e);
            let (w, h) = (e.size().width / 2.0, e.size().height / 2.0);
            (x - (centre.0 - w)).abs() < 6.0 && (y - (centre.1 - h)).abs() < 6.0
        })
        .map(|e| scale_of(e, centre))
        .collect();
    assert!(!scales.is_empty(), "press-wrap instances at {centre:?}");
    for s in &scales {
        assert!(
            (s - scales[0]).abs() < 0.01,
            "the fill and the label of {centre:?} move together: {scales:?}"
        );
    }
    scales[0]
}

#[test]
fn press_scales_the_cell_to_press_scale_to_and_release_returns_it() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = wired();
    hive.present("x", seven(), None).unwrap();
    settle_hive(&hive);
    let p = plan(&hive);
    let c = (p.cells[0].x, p.cells[0].y);
    let want = 0.94_f32; // motion.press.scale-to, pinned by tests/glass_tokens.rs

    assert!(
        (press_scale(&hive, c) - 1.0).abs() < 0.01,
        "at rest the cell is at scale 1"
    );

    let press = |down: bool| {
        let ui = hive.inner.ui.borrow();
        let w = ui.window();
        let position = LogicalPosition::new(c.0, c.1);
        w.dispatch_event(WindowEvent::PointerMoved { position });
        w.dispatch_event(WindowEvent::PointerPressed {
            position,
            button: PointerEventButton::Left,
        });
        if !down {
            w.dispatch_event(WindowEvent::PointerReleased {
                position,
                button: PointerEventButton::Left,
            });
        }
        drop(ui);
        // The pointer move highlights the cell and adds its own (unpressed)
        // instances; the press scale is measured on the pressed cell only.
        hive.inner.ui.borrow().set_highlight(-1);
    };

    press(true);
    turn(70);
    let s = press_scale(&hive, c);
    assert!(
        (s - want).abs() < 0.01,
        "pressed for 70 ms the cell is at {want}: {s}"
    );
    press(false);
    turn(70);
    let s = press_scale(&hive, c);
    assert!(
        (s - 1.0).abs() < 0.01,
        "released the cell is back at 1: {s}"
    );
}

#[test]
fn the_close_keeps_the_surface_90_ms_with_one_callback_per_show() {
    i_slint_backend_testing::init_no_event_loop();
    let (hive, count) = counted();
    hive.present("x", seven(), None).unwrap();
    settle_hive(&hive);

    key(&hive, Key::Escape);
    assert_eq!(count(), 1, "the callback runs in the same turn");
    assert!(
        hive.inner.visible(),
        "the close animation keeps the surface for 90 ms"
    );
    turn(85);
    assert!(hive.inner.visible(), "still closing at 85 ms");
    turn(35);
    assert!(!hive.inner.visible(), "gone by 120 ms (90 ms + a frame)");

    // A second input during the close adds no callback (1 per show).
    hive.present("x", seven(), None).unwrap();
    settle_hive(&hive);
    key(&hive, Key::Escape);
    key(&hive, Key::Escape);
    key(&hive, Key::Tab);
    assert_eq!(count(), 2, "one callback per show");
    // present during the close cancels it and shows the new hive at once.
    hive.present("x", seven(), None).unwrap();
    assert!(hive.inner.visible(), "the new show is up");
    assert!(
        !hive.inner.ui.borrow().get_closing(),
        "the close is cancelled by the new show"
    );
    turn(120);
    assert!(
        hive.inner.visible(),
        "the new show is not closed by the old timer"
    );
}

#[test]
fn reduced_motion_opens_at_once_jumps_the_highlight_and_closes_at_once() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = test_env::build(&[("KAWARO_REDUCED_MOTION", "1")], Lang::En);
    let count = Rc::new(Cell::new(0u32));
    let c = count.clone();
    hive.on_dismiss(move || c.set(c.get() + 1));

    hive.present("x", seven(), None).unwrap();
    turn(1);
    let p = plan(&hive);
    let e = at(&by_id(&hive, "cell-hex"), (p.cells[6].x, p.cells[6].y));
    assert_eq!(
        e.computed_opacity(),
        1.0,
        "with reduced motion every cell is at 1 at 0 ms"
    );
    // The highlight is at its target when it changes.
    hive.inner.ui.borrow().set_highlight(2);
    let c2 = (p.cells[2].x, p.cells[2].y);
    let hl = at(&by_id(&hive, "highlight"), c2);
    let (x, _) = top_left(&hl);
    assert!(
        (x + hl.size().width / 2.0 - c2.0).abs() < 0.5,
        "the highlight jumps to its target"
    );
    // Press does not scale.
    assert!(
        (press_scale(&hive, c2) - 1.0).abs() < 0.01,
        "press does not scale under reduced motion"
    );
    // The close hides at once.
    key(&hive, Key::Escape);
    assert_eq!(count.get(), 1);
    assert!(
        !hive.inner.visible(),
        "with reduced motion the close is instant"
    );
}

// ------------------------------------------------------------------------
// The glass fills, rendered with the software renderer (as AC13 and AC16
// probe them): cells and stacks `glass-2`, the centre `glass-3`, gaps 0.

use slint::platform::software_renderer::{
    MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType, TargetPixel,
};
use slint::platform::{Platform, WindowAdapter};

thread_local! {
    static WINDOW: RefCell<Option<Rc<MinimalSoftwareWindow>>> = const { RefCell::new(None) };
}

/// A platform whose windows draw with the software renderer into a buffer
/// we own (the testing platform cannot take snapshots).
struct SoftPlatform;

impl Platform for SoftPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        let w = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        WINDOW.with(|c| *c.borrow_mut() = Some(w.clone()));
        Ok(w)
    }
}

#[derive(Clone, Copy, Default)]
struct Px {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl TargetPixel for Px {
    fn blend(&mut self, c: PremultipliedRgbaColor) {
        let keep = u16::from(255 - c.alpha);
        let mix = |d: u8, s: u8| (u16::from(d) * keep / 255) as u8 + s;
        self.r = mix(self.r, c.red);
        self.g = mix(self.g, c.green);
        self.b = mix(self.b, c.blue);
        self.a = mix(self.a, c.alpha);
    }
    fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
    fn background() -> Self {
        Self::default()
    }
}

/// Presents `seven()` in `scheme` with the glass fills on or off, renders
/// the settled hive and returns (width, height, RGBA bytes, plan).
fn render_glass(scheme: &str, glass: bool) -> (u32, u32, Vec<u8>, layout::Plan) {
    slint::platform::set_platform(Box::new(SoftPlatform)).ok();
    let hive = test_env::build(&[("KAWARO_COLOR_SCHEME", scheme)], Lang::En);
    hive.set_glass(glass);
    hive.present("X", seven(), None).unwrap();
    // The ring-by-ring open starts hidden; the probes read the settled hive.
    hive.inner.ui.borrow().set_opened(true);
    turn(500);
    let win = WINDOW
        .with(|c| c.borrow().clone())
        .expect("the hive made a window");
    let (lw, lh) = {
        let ui = hive.inner.ui.borrow();
        (ui.get_win_w(), ui.get_win_h())
    };
    win.set_size(slint::PhysicalSize::new(lw as u32, lh as u32));
    win.dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor: 1.0 });
    let (w, h) = (lw as u32, lh as u32);
    let mut buf = vec![Px::default(); (w * h) as usize];
    win.request_redraw();
    let drew = win.draw_if_needed(|r| {
        r.render(&mut buf, w as usize);
    });
    assert!(drew, "the hive window drew nothing");
    let p = plan(&hive);
    let bytes = buf.iter().flat_map(|p| [p.r, p.g, p.b, p.a]).collect();
    (w, h, bytes, p)
}

/// The alpha byte of an 8-digit hex token colour.
fn alpha_of(hex: &str) -> u8 {
    u8::from_str_radix(&hex[7..9], 16).unwrap()
}

fn token_hex(key: &str) -> String {
    let raw = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../design/tokens.json"),
    )
    .unwrap();
    let t: serde_json::Value = serde_json::from_str(&raw).unwrap();
    t["glass"][key]["light"].as_str().unwrap().to_string()
}

fn alpha_at(s: &(u32, u32, Vec<u8>, layout::Plan), x: f32, y: f32) -> u8 {
    let (w, h) = (s.0 as i64, s.1 as i64);
    let (x, y) = (
        (x.round() as i64).clamp(0, w - 1),
        (y.round() as i64).clamp(0, h - 1),
    );
    s.2[((y * w + x) * 4 + 3) as usize]
}

#[test]
fn glass_surfaces_use_glass_2_and_glass_3_with_their_token_alphas() {
    // Sample inside a cell but above its icon and text.
    let s = render_glass("light", true);
    let r = s.3.layout.radius;
    let cell = (s.3.cells[0].x, s.3.cells[0].y - 0.55 * r);
    let centre = (s.3.layout.origin.0, s.3.layout.origin.1 - 0.6 * r);
    // A gap point: the midpoint of two neighbouring ring-1 cells.
    let gap = (
        (s.3.cells[0].x + s.3.cells[1].x) / 2.0,
        (s.3.cells[0].y + s.3.cells[1].y) / 2.0,
    );
    assert_eq!(
        alpha_at(&s, cell.0, cell.1),
        alpha_of(&token_hex("glass-2")),
        "a cell is glass-2 away from its label"
    );
    assert_eq!(
        alpha_at(&s, centre.0, centre.1),
        alpha_of(&token_hex("glass-3")),
        "the centre is glass-3 away from its label"
    );
    assert_eq!(alpha_at(&s, gap.0, gap.1), 0, "gaps stay transparent");
}

#[test]
fn without_glass_the_surfaces_stay_opaque() {
    let s = render_glass("light", false);
    let r = s.3.layout.radius;
    let cell = (s.3.cells[0].x, s.3.cells[0].y - 0.55 * r);
    assert_eq!(
        alpha_at(&s, cell.0, cell.1),
        255,
        "no blur, no transparency (CLAUDE.md design rules)"
    );
}

#[test]
fn kawaro_glass_env_and_set_glass_switch_the_fills() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = test_env::build(&[("KAWARO_GLASS", "1")], Lang::En);
    assert!(
        hive.inner.ui.borrow().get_glass(),
        "KAWARO_GLASS=1 sets glass"
    );
    hive.set_glass(false);
    assert!(
        !hive.inner.ui.borrow().get_glass(),
        "set_glass(false) clears it"
    );
}

fn counted() -> (Hive, impl Fn() -> u32) {
    let hive = wired();
    let count = Rc::new(Cell::new(0u32));
    let c = count.clone();
    hive.on_dismiss(move || c.set(c.get() + 1));
    let c = count.clone();
    hive.on_choose(move |_| c.set(c.get() + 1));
    (hive, move || count.get())
}

fn key(hive: &Hive, k: Key) {
    let text: slint::SharedString = k.into();
    let ui = hive.inner.ui.borrow();
    let w = ui.window();
    w.dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    w.dispatch_event(WindowEvent::KeyReleased { text });
}
