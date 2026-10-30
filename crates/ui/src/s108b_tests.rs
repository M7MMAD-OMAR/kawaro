//! S-108 part B (backlog/S-108-s096b-hive-cells.md), the window half: AC1
//! the message element, AC6 the suggestion element, AC7 the token scans of
//! the greyed branch, AC8 the accessibility sweep, AC9 reduced motion.
//! Headless Slint platform, no compositor.
//!
//! Hooks these tests pin:
//! - `Hive::set_notes(&self, message: &str, suggestion: Option<&str>)` sets
//!   the window's `centre-message` and `centre-suggestion` string properties
//!   (`ui.get_centre_message()`, `ui.get_centre_suggestion()`). Every
//!   `present*` call clears both, so a note never outlives its show. The
//!   centre text of the window never holds the message (no newline).
//! - `HiveWindow` draws them as their own elements with the ids
//!   `HiveWindow::message` and `HiveWindow::suggestion`, each present only
//!   while its text is not empty (element count 1, otherwise 0). Their text
//!   is the accessible label. They share the centre's open: ring 0, opacity
//!   0 until `opened`, then 1 over `Tokens.hive-open`; under reduced motion
//!   opacity 1 at once.
//! - `HexCell` gains `enabled: bool` (default true). Its fill and ink are
//!   chosen by a statement that mentions `enabled`: the greyed branch uses
//!   `Tokens.surface-2` and `Tokens.text-secondary` only. `hive.slint` passes
//!   `enabled: cell.enabled` and does not draw the sliding accent highlight
//!   over a disabled cell (the `highlight` index still moves, so the centre
//!   shows the reason).

use super::*;
use i_slint_backend_testing::{AccessibleRole, ElementHandle, ElementRoot};
use kawaro_core::unsupported::{more_label, request_hover, request_label, skipped_label};
use slint::LogicalPosition;
use slint::platform::WindowEvent;

fn cell(id: &str, code: &str, label: &str, ring: u8, enabled: bool, reason: &str) -> HiveCell {
    HiveCell {
        id: id.into(),
        code: code.into(),
        label: label.into(),
        icon: String::new(),
        ring,
        enabled,
        reason: reason.into(),
    }
}

fn turn(ms: u64) {
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(ms));
}

fn count(hive: &Hive, id: &str) -> usize {
    let ui = hive.inner.ui.borrow();
    ElementHandle::find_by_element_id(&*ui, id).count()
}

fn one(hive: &Hive, id: &str) -> ElementHandle {
    let ui = hive.inner.ui.borrow();
    let mut found: Vec<ElementHandle> = ElementHandle::find_by_element_id(&*ui, id).collect();
    assert_eq!(found.len(), 1, "exactly one `{id}` element");
    found.pop().unwrap()
}

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

fn plan(hive: &Hive) -> layout::Plan {
    hive.inner.plan.borrow().clone().unwrap()
}

fn at(found: &[ElementHandle], centre: (f32, f32)) -> ElementHandle {
    let hits: Vec<&ElementHandle> = found
        .iter()
        .filter(|e| {
            let p = e.absolute_position();
            let (w, h) = (e.size().width / 2.0, e.size().height / 2.0);
            (p.x - (centre.0 - w)).abs() < 6.0 && (p.y - (centre.1 - h)).abs() < 6.0
        })
        .collect();
    assert_eq!(hits.len(), 1, "exactly one instance at {centre:?}");
    hits[0].clone()
}

fn hover(hive: &Hive, index: usize) {
    let p = plan(hive);
    hive.inner
        .ui
        .borrow()
        .window()
        .dispatch_event(WindowEvent::PointerMoved {
            position: LogicalPosition::new(p.cells[index].x, p.cells[index].y),
        });
}

// ------------------------------------------------------------------ AC1

#[test]
fn ac1_the_message_has_its_own_element_with_the_message_as_text() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = test_env::build(&[], Lang::En);
    hive.present("File .xyz", Vec::new(), None).unwrap();
    hive.set_notes("This format is not supported yet", None);
    assert_eq!(count(&hive, "HiveWindow::message"), 1);
    let e = one(&hive, "HiveWindow::message");
    assert_eq!(
        e.accessible_label().unwrap().as_str(),
        "This format is not supported yet"
    );
    let ui = hive.inner.ui.borrow();
    assert_eq!(
        ui.get_centre_message().as_str(),
        "This format is not supported yet"
    );
    assert_eq!(ui.get_centre_text().as_str(), "File .xyz");
    assert!(!ui.get_centre_text().contains('\n'));
    assert!(!ui.get_centre_shown().contains('\n'));
}

#[test]
fn ac1_an_empty_message_draws_no_message_element() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = test_env::build(&[], Lang::En);
    hive.present("3 files", Vec::new(), None).unwrap();
    assert_eq!(count(&hive, "HiveWindow::message"), 0);
    hive.set_notes("", None);
    assert_eq!(count(&hive, "HiveWindow::message"), 0);
}

#[test]
fn ac1_the_arabic_message_is_its_own_element_too() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = test_env::build(&[], Lang::Ar);
    hive.present("ملف \u{200e}.xyz", Vec::new(), None).unwrap();
    hive.set_notes("هذه الصيغة غير مدعومة بعد", None);
    assert_eq!(
        one(&hive, "HiveWindow::message")
            .accessible_label()
            .unwrap()
            .as_str(),
        "هذه الصيغة غير مدعومة بعد"
    );
}

#[test]
fn ac1_a_new_show_clears_the_notes() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = test_env::build(&[], Lang::En);
    hive.present("File .heic", Vec::new(), None).unwrap();
    hive.set_notes("This format is not supported yet", Some("Convert to JPG"));
    assert_eq!(count(&hive, "HiveWindow::message"), 1);
    assert_eq!(count(&hive, "HiveWindow::suggestion"), 1);
    hive.present("2 images", Vec::new(), None).unwrap();
    assert_eq!(count(&hive, "HiveWindow::message"), 0);
    assert_eq!(count(&hive, "HiveWindow::suggestion"), 0);
}

// ------------------------------------------------------------------ AC6

#[test]
fn ac6_the_suggestion_has_its_own_element_only_when_there_is_one() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = test_env::build(&[], Lang::En);
    hive.present("File .heic", Vec::new(), None).unwrap();

    // `.xyz`, family present with the engine off, and `Other`: no line.
    hive.set_notes("This format is not supported yet", None);
    assert_eq!(count(&hive, "HiveWindow::suggestion"), 0);
    assert_eq!(count(&hive, "HiveWindow::message"), 1);

    // A family stand-in with the engine on: one line with that label.
    hive.set_notes("This format is not supported yet", Some("Convert to JPG"));
    assert_eq!(count(&hive, "HiveWindow::suggestion"), 1);
    assert_eq!(
        one(&hive, "HiveWindow::suggestion")
            .accessible_label()
            .unwrap()
            .as_str(),
        "Convert to JPG"
    );
    assert_eq!(
        hive.inner.ui.borrow().get_centre_suggestion().as_str(),
        "Convert to JPG"
    );

    // Back to none: the element goes again, never a generic line.
    hive.set_notes("This format is not supported yet", None);
    assert_eq!(count(&hive, "HiveWindow::suggestion"), 0);
}

// ------------------------------------------------------------------ AC7

fn hive_src() -> &'static str {
    include_str!("../ui/hive.slint")
}

fn cell_src() -> &'static str {
    include_str!("../components/hex-cell.slint")
}

/// The code of `src` without `//` comments.
fn code_of(src: &str) -> Vec<&str> {
    src.lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect()
}

/// Every statement of `hex-cell.slint` (from a line that mentions `enabled`
/// to the first line ending in `;`) that is not the property declaration.
fn enabled_statements(src: &str) -> Vec<String> {
    let lines = code_of(src);
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i];
        if l.contains("enabled") && !l.contains("property") {
            let mut s = String::new();
            let mut j = i;
            loop {
                s.push_str(lines[j].trim());
                s.push(' ');
                if lines[j].trim_end().ends_with(';') || j + 1 >= lines.len() {
                    break;
                }
                j += 1;
            }
            out.push(s);
            i = j;
        }
        i += 1;
    }
    out
}

#[test]
fn ac7_the_greyed_branch_uses_only_surface_2_and_text_secondary() {
    let statements = enabled_statements(cell_src());
    let all = statements.join("\n");
    assert!(
        !statements.is_empty(),
        "hex-cell.slint has a binding that depends on `enabled`"
    );
    assert!(
        all.contains("Tokens.surface-2"),
        "the greyed fill is Tokens.surface-2: {all}"
    );
    assert!(
        all.contains("Tokens.text-secondary"),
        "the greyed ink is Tokens.text-secondary: {all}"
    );
    for forbidden in [
        "opacity",
        "#",
        "drop-shadow",
        "@linear-gradient",
        "@radial-gradient",
    ] {
        assert!(
            !all.contains(forbidden),
            "the greyed branch contains `{forbidden}`: {all}"
        );
    }
}

#[test]
fn ac7_a_greyed_cell_never_takes_the_accent_fill_when_highlighted() {
    // The fill statement decides on `enabled` before it looks at
    // `highlighted`, so a disabled cell stays surface-2.
    let fill = enabled_statements(cell_src())
        .into_iter()
        .find(|s| s.contains("Tokens.surface-2"))
        .expect("a fill statement with the greyed surface");
    if let (Some(e), Some(h)) = (fill.find("enabled"), fill.find("highlighted")) {
        assert!(e < h, "enabled is tested before highlighted: {fill}");
    }
    assert!(
        !fill.contains("Tokens.accent"),
        "the greyed branch has no accent: {fill}"
    );
    assert!(
        hive_src().contains("enabled: cell.enabled"),
        "hive.slint passes the cell's enabled flag to the HexCell"
    );
}

#[test]
fn ac7_no_literal_colours_shadows_or_gradients_in_the_two_sources() {
    for (name, src) in [("hive.slint", hive_src()), ("hex-cell.slint", cell_src())] {
        for line in code_of(src) {
            let hex_colour = line.match_indices('#').any(|(i, _)| {
                line[i + 1..]
                    .chars()
                    .take_while(char::is_ascii_hexdigit)
                    .count()
                    >= 3
            });
            assert!(!hex_colour, "{name} writes a hex colour: {line}");
            for forbidden in ["drop-shadow", "box-shadow", "@linear-gradient", "@radial"] {
                assert!(!line.contains(forbidden), "{name}: `{forbidden}` in {line}");
            }
        }
    }
    // The statement count is what makes this test fail before the feature:
    // the scans above are only meaningful once the greyed branch exists.
    assert!(
        !enabled_statements(cell_src()).is_empty(),
        "the greyed branch exists in hex-cell.slint"
    );
}

#[test]
fn ac7_the_accent_highlight_is_not_drawn_over_a_disabled_cell() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = test_env::build(&[], Lang::En);
    let cells = vec![
        cell("real", "PNG", "Real", 1, true, ""),
        cell("skipped:0", "XYZ", "skipped", 1, false, "reason zero"),
        cell("more", "+3", "more", 1, false, "reason more"),
        cell("request:xyz", "REQ", "Request", 1, true, "hover text"),
    ];
    hive.present("3 files", cells, None).unwrap();
    hive.inner.ui.borrow().set_opened(true);
    turn(1000);
    hover(&hive, 0);
    assert_eq!(by_id(&hive, "highlight").len(), 1, "an enabled cell");
    hover(&hive, 1);
    assert_eq!(hive.inner.ui.borrow().get_highlight(), 1);
    assert_eq!(
        by_id(&hive, "highlight").len(),
        0,
        "no accent highlight over a skipped cell"
    );
    hover(&hive, 2);
    assert_eq!(by_id(&hive, "highlight").len(), 0, "nor over the +K cell");
    hover(&hive, 3);
    assert_eq!(
        by_id(&hive, "highlight").len(),
        1,
        "the request cell is live"
    );
}

// ------------------------------------------------------------------ AC8

/// The cells of one outcome in `lang`, built from the core strings.
fn outcome_cells(outcome: &str, lang: Lang) -> Vec<HiveCell> {
    let reason = match outcome {
        "unsupported" => kawaro_core::unsupported::unsupported_message(lang).to_string(),
        _ => kawaro_core::unsupported::missing_engine_message("FFmpeg", lang),
    };
    let mut cells = Vec::new();
    if outcome == "mixed" {
        cells.push(cell("image.rotate-90", "ROT", "Rotate", 1, true, ""));
    }
    cells.push(cell(
        "skipped:0",
        "XYZ",
        &skipped_label("xyz", &reason, lang),
        1,
        false,
        &reason,
    ));
    cells.push(cell(
        "more",
        "+3",
        &more_label(3, lang),
        1,
        false,
        &more_label(3, lang),
    ));
    cells.push(cell(
        "request:xyz",
        "REQ",
        request_label(lang),
        1,
        true,
        request_hover(lang),
    ));
    cells
}

#[test]
fn ac8_every_cell_and_the_message_have_an_accessible_label_in_both_languages() {
    i_slint_backend_testing::init_no_event_loop();
    for lang in [Lang::En, Lang::Ar] {
        for outcome in ["unsupported", "missing-engine", "mixed"] {
            let hive = test_env::build(&[], lang);
            let cells = outcome_cells(outcome, lang);
            hive.present("x", cells.clone(), None).unwrap();
            let message = if outcome == "unsupported" {
                kawaro_core::unsupported::unsupported_message(lang).to_string()
            } else {
                kawaro_core::unsupported::missing_engine_message("FFmpeg", lang)
            };
            hive.set_notes(&message, None);
            for c in &cells {
                let found: Vec<ElementHandle> = {
                    let ui = hive.inner.ui.borrow();
                    ElementHandle::find_by_accessible_label(&*ui, &c.label).collect()
                };
                let buttons = found
                    .iter()
                    .filter(|e| e.accessible_role() == Some(AccessibleRole::Button))
                    .count();
                assert_eq!(
                    buttons, 1,
                    "{outcome} {lang:?}: one button for `{}`",
                    c.label
                );
                assert!(!c.label.is_empty());
            }
            let skipped = &cells.iter().find(|c| c.id == "skipped:0").unwrap().label;
            let marker = match lang {
                Lang::En => "unavailable",
                Lang::Ar => "غير متاح",
            };
            assert!(skipped.contains(marker), "{skipped}");
            assert!(skipped.contains(&message), "{skipped}");
            let label = one(&hive, "HiveWindow::message").accessible_label();
            assert_eq!(label.as_deref(), Some(message.as_str()));
        }
    }
}

// ------------------------------------------------------------------ AC9

fn motion_cells() -> Vec<HiveCell> {
    // Six in ring 1 (a real action, two skipped, +K, the request cell and
    // one more), two in ring 2.
    let mut v = vec![
        cell("real", "PNG", "Real", 1, true, ""),
        cell("skipped:0", "XYZ", "s0", 1, false, "r0"),
        cell("skipped:1", "ABC", "s1", 1, false, "r1"),
        cell("more", "+3", "more", 1, false, "rm"),
        cell("request:xyz", "REQ", "Request", 1, true, "hover"),
        cell("real2", "JPG", "Real 2", 1, true, ""),
    ];
    v.push(cell("skipped:2", "DEF", "s2", 2, false, "r2"));
    v.push(cell("skipped:3", "GHI", "s3", 2, false, "r3"));
    v
}

fn opacity_of(hive: &Hive, index: usize) -> f32 {
    let p = plan(hive);
    at(
        &by_id(hive, "cell-hex"),
        (p.cells[index].x, p.cells[index].y),
    )
    .computed_opacity()
}

#[test]
fn ac9_reduced_motion_shows_every_new_cell_and_note_at_once() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = test_env::build(&[("KAWARO_REDUCED_MOTION", "1")], Lang::En);
    hive.present("File .xyz", motion_cells(), None).unwrap();
    hive.set_notes("This format is not supported yet", Some("Convert to JPG"));
    turn(1);
    for i in 0..8 {
        assert_eq!(opacity_of(&hive, i), 1.0, "cell {i} is at 1 at 0 ms");
    }
    for id in ["HiveWindow::message", "HiveWindow::suggestion"] {
        assert_eq!(
            one(&hive, id).computed_opacity(),
            1.0,
            "{id} is at 1 at 0 ms"
        );
    }
}

#[test]
fn ac9_with_motion_the_new_cells_and_notes_open_ring_by_ring() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = test_env::build(&[("KAWARO_REDUCED_MOTION", "0")], Lang::En);
    hive.present("File .xyz", motion_cells(), None).unwrap();
    hive.set_notes("This format is not supported yet", Some("Convert to JPG"));
    first_frame(&hive);
    turn(1);
    assert!(hive.inner.ui.borrow().get_opened());
    for i in 0..8 {
        assert_eq!(opacity_of(&hive, i), 0.0, "cell {i} starts hidden");
    }
    for id in ["HiveWindow::message", "HiveWindow::suggestion"] {
        assert_eq!(one(&hive, id).computed_opacity(), 0.0, "{id} starts hidden");
    }
    turn(100);
    assert!(
        opacity_of(&hive, 1) > opacity_of(&hive, 6),
        "ring 1 is ahead of ring 2 at 100 ms"
    );
    turn(120);
    for i in 0..8 {
        assert_eq!(opacity_of(&hive, i), 1.0, "cell {i} fully visible");
    }
    for id in ["HiveWindow::message", "HiveWindow::suggestion"] {
        assert_eq!(one(&hive, id).computed_opacity(), 1.0, "{id} fully visible");
    }
}
