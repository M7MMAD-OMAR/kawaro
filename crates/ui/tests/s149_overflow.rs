//! S-149 (backlog/S-149-all-actions-reachable.md): every matching action
//! stays reachable when there are more of them than the hive has slots.
//!
//! The outer ring pages: every page holds 11 actions and the More cell, the
//! More cell sits in the last slot of the ring and never runs an action, and
//! every action of the registry is reachable exactly once across the pages.
//! A selection that fits in the outer ring keeps the placement and the
//! geometry of today.
//!
//! The page follows the placement: near a screen edge the outer ring accepts
//! fewer slots, so a page holds fewer actions and the More cell takes the
//! last accepted slot. Every action stays reachable either way.
//!
//! The pure layout is the contract, so these tests need no window and no
//! compositor.

use kawaro_ui::HiveCell;
use kawaro_ui::Lang;
use kawaro_ui::layout::{
    Activation, MORE_ID, PAGE_SIZE, activation, more_cell, outer_queue, page, page_size, pages,
    plan_rings,
};

/// One action cell, as the daemon builds it.
fn cell(id: &str, ring: u8) -> HiveCell {
    HiveCell {
        id: id.into(),
        code: id.to_uppercase(),
        label: format!("label {id}"),
        icon: String::new(),
        ring,
        enabled: true,
        reason: String::new(),
        customize: false,
    }
}

/// `n` actions, the first `common` of them common ones (ring 1).
fn actions(n: usize, common: usize) -> Vec<HiveCell> {
    (0..n)
        .map(|i| {
            let ring = if i < common { 1 } else { 2 };
            cell(&format!("image.act-{i}"), ring)
        })
        .collect()
}

fn ids(cells: &[HiveCell]) -> Vec<String> {
    cells.iter().map(|c| c.id.clone()).collect()
}

/// The cells of the page, as the layout takes them.
fn page_cells(outer: &[HiveCell], index: usize) -> Vec<HiveCell> {
    page(outer, index, PAGE_SIZE, Lang::En)
}

/// `n` actions of the outer queue, behind six common actions, so the queue
/// is not the compact single ring.
fn queue(n: usize) -> Vec<HiveCell> {
    let (_, outer) = outer_queue(actions(n + 6, 6));
    outer
}

// AC: every input action is reachable exactly once ------------------------

#[test]
fn every_action_is_reachable_exactly_once_across_the_pages() {
    for n in [13, 18, 19, 30] {
        let cells = actions(n, 6);
        let (ring1, outer) = outer_queue(cells.clone());
        // The ring 1 cells are on every page, the outer cells once.
        let mut seen: Vec<String> = ring1.iter().map(|c| c.id.clone()).collect();
        for p in 0..pages(&outer, PAGE_SIZE) {
            for c in page_cells(&outer, p) {
                if c.id != MORE_ID {
                    seen.push(c.id.clone());
                }
            }
        }
        assert_eq!(seen.len(), n, "{n} actions");
        let mut unique = seen.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), n, "{n} actions, one page each");
        // The registry order is kept: the pages show the queue in its order.
        let want: Vec<String> = cells.iter().map(|c| c.id.clone()).collect();
        assert_eq!(seen, want, "{n} actions in the registry order");
        // The ring 1 cells are on every page, they never page.
        assert!(ring1.iter().all(|c| c.ring <= 1));
    }
}

#[test]
fn a_page_holds_eleven_actions_and_the_more_cell() {
    let (_, outer) = outer_queue(actions(19, 0));
    let first = page_cells(&outer, 0);
    assert_eq!(first.len(), PAGE_SIZE + 1);
    assert_eq!(first[PAGE_SIZE].id, MORE_ID);
    let last = page_cells(&outer, 1);
    assert_eq!(last.len(), 9, "8 actions and the More cell");
    assert_eq!(last[8].id, MORE_ID);
    // The page count follows the queue length.
    assert_eq!(pages(&outer, PAGE_SIZE), 2);
    let (_, outer) = outer_queue(actions(30, 0));
    assert_eq!(pages(&outer, PAGE_SIZE), 3);
}

// AC: the More cell never routes to an action ------------------------------

#[test]
fn the_more_cell_never_routes_to_an_action() {
    assert_eq!(activation(MORE_ID), Activation::More);
    assert_eq!(
        activation("image.to-text"),
        Activation::Action("image.to-text".into())
    );
    assert_eq!(
        activation("image.compress-to"),
        Activation::Action("image.compress-to".into())
    );
    // The id of the More cell is not an action id of the registry.
    let actions_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../actions");
    for entry in std::fs::read_dir(&actions_dir).expect("the actions directory") {
        let path = entry.expect("entry").path();
        let text = std::fs::read_to_string(&path).expect("the action file");
        assert!(
            !text.contains(&format!("id = \"{MORE_ID}\"")),
            "{path:?} uses the id of the More cell"
        );
    }
}

// AC: empty and single page cases preserve the placement -------------------

#[test]
fn a_selection_that_fits_keeps_the_placement_of_today() {
    // Nothing to show: no cell and no More cell.
    let p = plan_rings(Vec::new(), false, 1);
    assert!(p.cells.is_empty());
    // Up to a full outer ring: one page, no More cell, the same slots.
    for n in [1, 6, 12] {
        let p = plan_rings(actions(n, 0), false, 1);
        assert_eq!(p.cells.len(), n, "{n} actions");
        assert!(
            p.cells.iter().all(|c| c.cell.id != MORE_ID),
            "{n} actions, no More cell"
        );
    }
    // The common actions keep ring 1 and the rest keep ring 2.
    let p = plan_rings(actions(18, 6), false, 1);
    assert_eq!(p.cells.len(), 18);
    assert_eq!(p.cells[0].cell.id, "image.act-0");
    assert_eq!(p.cells[6].cell.id, "image.act-6");
    assert!(p.cells.iter().all(|c| c.cell.id != MORE_ID));
}

// AC: the pages keep the window and the session ----------------------------

#[test]
fn every_page_keeps_the_window_size() {
    let (ring1, outer) = outer_queue(actions(30, 6));
    let first = plan_rings([ring1.clone(), page_cells(&outer, 0)].concat(), false, 1);
    let middle = plan_rings([ring1.clone(), page_cells(&outer, 1)].concat(), false, 1);
    let last = plan_rings([ring1, page_cells(&outer, 2)].concat(), false, 1);
    assert_eq!((first.width, first.height), (middle.width, middle.height));
    assert_eq!((first.width, first.height), (last.width, last.height));
    // The More cell sits in the last slot of the outer ring on every page.
    for p in [&first, &middle, &last] {
        let more = p
            .cells
            .iter()
            .position(|c| c.cell.id == MORE_ID)
            .expect("the More cell");
        assert_eq!(more, p.cells.len() - 1, "the More cell is the last one");
        assert_eq!(p.cells[more].axial.distance(), 2, "the outer ring");
    }
}

// AC: the page follows the slots the placement accepts --------------------

#[test]
fn the_page_size_follows_the_accepted_slots() {
    // The unconstrained placement: the whole outer ring.
    assert_eq!(page_size(12), PAGE_SIZE);
    assert_eq!(page_size(8), 7);
    assert_eq!(page_size(5), 4);
    // A placement with one slot or less keeps the default size: the More
    // cell would leave no room for an action.
    assert_eq!(page_size(1), PAGE_SIZE);
    assert_eq!(page_size(0), PAGE_SIZE);
}

#[test]
fn every_action_stays_reachable_with_fewer_accepted_slots() {
    for slots in [12, 8, 5] {
        for n in [13, 19, 30] {
            let outer = queue(n);
            let size = page_size(slots);
            let mut seen: Vec<String> = Vec::new();
            for p in 0..pages(&outer, size) {
                let cells = page(&outer, p, size, Lang::En);
                // The More cell is the last cell of the page.
                assert_eq!(
                    cells.last().map(|c| c.id.clone()),
                    Some(MORE_ID.to_string()),
                    "{slots} slots, {n} actions, page {p}"
                );
                for c in cells {
                    if c.id != MORE_ID {
                        seen.push(c.id.clone());
                    }
                }
            }
            assert_eq!(seen.len(), n, "{slots} slots, {n} actions");
            let mut unique = seen.clone();
            unique.sort();
            unique.dedup();
            assert_eq!(unique.len(), n, "{slots} slots, {n} actions, one page each");
        }
    }
}

#[test]
fn a_queue_that_fits_in_the_accepted_slots_has_no_more_cell() {
    for slots in [12, 8, 5] {
        for n in 0..=slots {
            let outer = queue(n);
            let size = page_size(slots);
            assert_eq!(pages(&outer, size), 1, "{slots} slots, {n} actions");
            let cells = page(&outer, 0, size, Lang::En);
            assert_eq!(cells.len(), n, "{slots} slots, {n} actions");
            assert!(
                cells.iter().all(|c| c.id != MORE_ID),
                "{slots} slots, {n} actions, no More cell"
            );
        }
    }
}

// AC: LTR and RTL page transitions -----------------------------------------

#[test]
fn the_more_cell_mirrors_in_arabic() {
    let (ring1, outer) = outer_queue(actions(19, 6));
    let cells = [ring1, page_cells(&outer, 0)].concat();
    let ltr = plan_rings(cells.clone(), false, 1);
    let rtl = plan_rings(cells, true, 1);
    let more_ltr = ltr.cells.iter().find(|c| c.cell.id == MORE_ID).unwrap();
    let more_rtl = rtl.cells.iter().find(|c| c.cell.id == MORE_ID).unwrap();
    assert_eq!(more_ltr.axial, more_rtl.axial);
    assert!(
        (more_ltr.x - ltr.layout.origin.0 + (more_rtl.x - rtl.layout.origin.0)).abs() < 1e-3,
        "the More cell mirrors"
    );
    assert_eq!(more_ltr.y, more_rtl.y);
    // The page holds the same actions in Arabic.
    let arabic = page(&outer, 0, PAGE_SIZE, Lang::Ar);
    let english = page(&outer, 0, PAGE_SIZE, Lang::En);
    assert_eq!(ids(&arabic)[..PAGE_SIZE], ids(&english)[..PAGE_SIZE]);
    assert_eq!(arabic[PAGE_SIZE].code, "المزيد");
    assert_eq!(english[PAGE_SIZE].code, "MORE");
}

// AC: the texts of the More cell -------------------------------------------

#[test]
fn the_more_cell_names_the_page_it_leads_to() {
    let more = more_cell(Lang::En, 1, 3);
    assert_eq!(more.label, "More actions, page 2 of 3");
    assert_eq!(more.code, "MORE");
    let arabic = more_cell(Lang::Ar, 1, 3);
    assert_eq!(arabic.label, "إجراءات أخرى، الصفحة 2 من 3");
    assert_eq!(arabic.code, "المزيد");
    // The last page leads back to the first one.
    let (_, outer) = outer_queue(actions(30, 0));
    let last = page_cells(&outer, 2);
    assert_eq!(last[last.len() - 1].label, "More actions, page 1 of 3");
}
