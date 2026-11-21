//! Maps the cells given to `Hive::present` onto hive slots, and keyboard
//! navigation between them. Pure logic, no Slint, so it is unit tested and
//! mutation tested on its own.
//!
//! Slot rules (the contract leaves the edge cases open, these are decisions):
//! - `ring <= 1` means ring 1, anything above means ring 2.
//! - Ring 1 takes the first 6 ring-1 cells in the given order. Ring-1 cells
//!   beyond 6 move to ring 2, ahead of the ring-2 cells, because they were
//!   ranked higher.
//! - Ring 2 takes the first 12 of that queue. The rest is dropped, so at most
//!   18 cells show.
//! - S-149: a longer outer queue is paged, never dropped. The outer ring
//!   holds 11 actions and the More cell, the last slot of the ring, and every
//!   action of the queue stays reachable, one page after the other. The More
//!   cell is a control of the hive, never an action.
//! - S-149: the page follows the placement. Near a screen edge the outer
//!   ring accepts fewer slots, so a page holds fewer actions and the More
//!   cell takes the last slot the screen accepts. Every action of the queue
//!   stays reachable either way.
//! - S-149: a paged hive keeps two rings on every page. The compact
//!   one-ring rule never applies to it, so the More cell stays in the outer
//!   ring and the hive never resizes under the pointer.
//! - S-149: a placement whose outer ring accepts one slot or less gives
//!   that slot to the More cell, or the last ring-1 slot when the outer
//!   ring has none at all, and the page holds one action at a time.
//! - A ring-2 cell never moves inwards to fill a short ring 1: ring 1 is for
//!   the common actions and an empty slot there reads better than a rare
//!   action in it. Exception: when every cell fits in ring 1 (6 or fewer in
//!   total), all of them go there, ring-1 cells first, so the hive is one
//!   compact ring instead of a lopsided two-ring window.
//! - A ring that is the only ring and is not full spreads its cells evenly
//!   over the six slots, so three actions sit at alternating slots.
//! - Each ring fills its slots from the top-right cell clockwise (LTR), or
//!   from the top-left cell counterclockwise (Arabic), see `geometry::ring`.

use crate::HiveCell;
use crate::Lang;
use crate::geometry::{Axial, Layout, ring};
use crate::tokens::{HEX_GAP, HEX_SIZE_MD, PAD};
// The size of the two rings, the count the hive is built around.
pub use crate::tokens::{RING1, RING2};

/// The id of the More cell (S-149). It is a control of the hive and never an
/// action, so no action id can ever be this one.
pub const MORE_ID: &str = "kawaro.more";

/// The action cells one page of the outer queue holds (S-149). The last slot
/// of the outer ring belongs to the More cell.
pub const PAGE_SIZE: usize = RING2 - 1;

/// The action cells one page of the outer queue holds for a hive placed
/// with `slots` accepted slots of the outer ring (S-149). The More cell
/// takes the last of them, so a page holds `slots - 1` actions.
///
/// A placement with one accepted outer slot or less holds one action per
/// page: the More cell takes the only outer slot, or the last ring-1 slot
/// when the outer ring accepts none at all, and the action of the page
/// takes the ring-1 slot the More cell leaves free.
pub fn page_size(slots: usize) -> usize {
    match slots {
        0 | 1 => 1,
        n => n - 1,
    }
}

/// The ring-1 slots a paged hive reserves for the More cell and the action
/// of the page when the outer ring has no room (S-149). With one accepted
/// outer slot the More cell takes it and the action takes a ring-1 slot.
/// With no accepted outer slot at all both take a ring-1 slot.
pub fn reserved_ring1_slots(outer_slots: usize) -> usize {
    match outer_slots {
        0 => 2,
        1 => 1,
        _ => 0,
    }
}

/// A cell with its slot and its pixel centre inside the hive window.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub cell: HiveCell,
    pub axial: Axial,
    pub x: f32,
    pub y: f32,
}

/// Everything the window needs for one `present` call.
#[derive(Clone, Debug)]
pub struct Plan {
    pub cells: Vec<Placed>,
    pub width: f32,
    pub height: f32,
    pub layout: Layout,
}

/// Splits cells into ring 1 and the whole outer queue, following the slot
/// rules above, without dropping anything: the ring-1 overflow first, then
/// the ring-2 cells, in the given order.
pub fn outer_queue(cells: Vec<HiveCell>) -> (Vec<HiveCell>, Vec<HiveCell>) {
    let (mut ring1, ring2): (Vec<_>, Vec<_>) = cells.into_iter().partition(|c| c.ring <= 1);
    if ring1.len() + ring2.len() <= RING1 {
        ring1.extend(ring2);
        return (ring1, Vec::new());
    }
    let mut outer: Vec<HiveCell> = if ring1.len() > RING1 {
        ring1.split_off(RING1)
    } else {
        Vec::new()
    };
    outer.extend(ring2);
    (ring1, outer)
}

/// Splits cells into ring 1 and ring 2 following the slot rules above.
pub fn assign(cells: Vec<HiveCell>) -> (Vec<HiveCell>, Vec<HiveCell>) {
    let (ring1, mut outer) = outer_queue(cells);
    outer.truncate(RING2);
    (ring1, outer)
}

/// Splits the cells by their ring, without the compact one-ring rule (S-149).
/// The ring-1 cells keep their place and every other cell is an outer cell.
fn split_rings(cells: Vec<HiveCell>) -> (Vec<HiveCell>, Vec<HiveCell>) {
    cells.into_iter().partition(|c| c.ring <= 1)
}

/// Splits cells into ring 1 and the outer ring without the compact
/// one-ring rule (S-149). A paged hive keeps two rings on every page, so
/// the More cell stays in the outer ring and never joins ring 1.
pub fn assign_paged(cells: Vec<HiveCell>) -> (Vec<HiveCell>, Vec<HiveCell>) {
    let (mut ring1, ring2) = split_rings(cells);
    if ring1.len() > RING1 {
        let mut outer = ring1.split_off(RING1);
        outer.extend(ring2);
        return (ring1, outer);
    }
    (ring1, ring2)
}

/// True when the cells hold the More cell, so the hive pages (S-149). The
/// More cell is a control of the hive and never an action, so only a
/// paged hive has one.
pub fn is_paged(cells: &[HiveCell]) -> bool {
    cells.iter().any(|cell| cell.id == MORE_ID)
}

/// The number of pages the outer queue needs, with `size` action cells on a
/// page (S-149). The More cell takes one slot of the outer ring, so a
/// queue of at most `size + 1` cells fits in the ring without it: one page
/// and no More cell.
pub fn pages(outer: &[HiveCell], size: usize) -> usize {
    if size == 0 || outer.len() <= size + 1 {
        return 1;
    }
    outer.len().div_ceil(size)
}

/// The cells of page `page` (0 based) of the outer queue (S-149): `size`
/// action cells and the More cell, which leads to the next page. Every
/// outer cell carries ring 2, so the layout keeps it in the outer ring. A
/// queue that fits in the outer ring is one page without a More cell.
pub fn page(outer: &[HiveCell], page: usize, size: usize, lang: Lang) -> Vec<HiveCell> {
    if pages(outer, size) <= 1 {
        return outer.iter().map(|c| outer_cell(c.clone())).collect();
    }
    let pages = pages(outer, size);
    let page = page.min(pages - 1);
    let start = page * size;
    let mut cells: Vec<HiveCell> = outer[start..(start + size).min(outer.len())]
        .iter()
        .map(|c| outer_cell(c.clone()))
        .collect();
    cells.push(more_cell(lang, (page + 1) % pages, pages));
    cells
}

/// The cells of ring 1 and the whole outer queue for a placement that holds
/// `ring1_slots` cells in ring 1 (S-149). The ring 1 cells the placement
/// does not hold move to the front of the outer queue, as they were ranked
/// higher, and the queue keeps every action.
///
/// The split follows the ring of every cell, never the compact one-ring
/// rule: a paged hive shows its ring-1 cells in ring 1 and its outer
/// queue in the outer ring.
pub fn queue_at(cells: &[HiveCell], ring1_slots: usize) -> (Vec<HiveCell>, Vec<HiveCell>) {
    let (mut ring1, outer) = split_rings(cells.to_vec());
    let moved = ring1.split_off(ring1.len().min(ring1_slots));
    let mut queue = moved;
    queue.extend(outer);
    (ring1, queue)
}

/// An outer cell of a page: the same action, in ring 2.
fn outer_cell(mut cell: HiveCell) -> HiveCell {
    cell.ring = 2;
    cell
}

/// The More cell of a page (S-149): it leads to the page `next` (0 based)
/// of `pages`. The code is the short text of the cell, the label the text
/// the centre cell shows while the cell is highlighted and the screen reader
/// reads. The page number in the label counts from one, as a reader counts.
pub fn more_cell(lang: Lang, next: usize, pages: usize) -> HiveCell {
    let (code, label) = match lang {
        Lang::Ar => (
            "المزيد",
            format!("إجراءات أخرى، الصفحة {} من {pages}", next + 1),
        ),
        Lang::En => (
            "MORE",
            format!("More actions, page {} of {pages}", next + 1),
        ),
    };
    HiveCell {
        id: MORE_ID.into(),
        code: code.into(),
        label,
        icon: String::new(),
        ring: 2,
        enabled: true,
        reason: String::new(),
        customize: false,
    }
}

/// What activating a cell means (S-149).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Activation {
    /// Run the action with this id.
    Action(String),
    /// The More cell: show the next page of the outer queue.
    More,
}

/// What activating the cell with the id `id` means. The More cell is a
/// control of the hive, so it never runs an action.
pub fn activation(id: &str) -> Activation {
    if id == MORE_ID {
        Activation::More
    } else {
        Activation::Action(id.to_string())
    }
}

/// The action cells of `cells` and the More cell, when it is among them.
fn split_more(cells: Vec<HiveCell>) -> (Vec<HiveCell>, Option<HiveCell>) {
    let mut more = None;
    let mut actions = Vec::with_capacity(cells.len());
    for cell in cells {
        if cell.id == MORE_ID {
            more = Some(cell);
        } else {
            actions.push(cell);
        }
    }
    (actions, more)
}

/// Lays out the cells for a window, using the token sizes. The window holds
/// one ring when ring 2 is empty, two otherwise. The hive shows go through
/// [`plan_rings`] with one ring, which is this; the tests use it.
#[cfg_attr(not(test), allow(dead_code))]
pub fn plan(cells: Vec<HiveCell>, rtl: bool) -> Plan {
    plan_rings(cells, rtl, 1)
}

/// As [`plan`], with a window of at least `min_rings` rings. The drop hive
/// uses 2, so its size and centre stay the same when its cells arrive.
pub fn plan_rings(cells: Vec<HiveCell>, rtl: bool, min_rings: i32) -> Plan {
    let paged = is_paged(&cells);
    let (ring1, ring2) = if paged {
        assign_paged(cells)
    } else {
        assign(cells)
    };
    // S-149: a paged hive keeps two rings, so the More cell never reaches
    // ring 1 and the window never resizes under the pointer.
    let paging_rings = if paged { 2 } else { 1 };
    let rings = if ring2.is_empty() { 1 } else { 2 }
        .max(min_rings)
        .max(paging_rings);
    let radius = HEX_SIZE_MD / 3f32.sqrt();
    let (width, height) = Layout::extent(radius, HEX_GAP, rings, PAD);
    let layout = Layout {
        radius,
        gap: HEX_GAP,
        rtl,
        origin: (width / 2.0, height / 2.0),
    };
    let inner = ring(1);
    let n = ring1.len();
    let ring1_slots: Vec<Axial> = if rings == 1 && n > 0 && n < RING1 {
        (0..n).map(|i| inner[i * RING1 / n]).collect()
    } else {
        inner.clone()
    };
    let slots: Vec<(Axial, HiveCell)> = if paged {
        place_paged(&ring1, &ring2, &ring1_slots, &ring(2), RING2)
    } else {
        ring1_slots
            .into_iter()
            .zip(ring1)
            .chain(place_outer(ring2))
            .collect()
    };
    let cells = slots
        .into_iter()
        .map(|(axial, cell)| {
            let (x, y) = layout.center(axial);
            Placed { cell, axial, x, y }
        })
        .collect();
    Plan {
        cells,
        width,
        height,
        layout,
    }
}

/// The outer cells in the slots of ring 2 (S-149): the action cells in the
/// ring order and the More cell in the last slot of the ring.
fn place_outer(cells: Vec<HiveCell>) -> Vec<(Axial, HiveCell)> {
    let (actions, more) = split_more(cells);
    let slots = ring(2);
    let mut outer: Vec<(Axial, HiveCell)> = slots.iter().copied().zip(actions).collect();
    // The More cell takes the last slot of the ring, when one is free.
    if outer.len() < slots.len()
        && let Some(more) = more
        && let Some(slot) = slots.last()
    {
        outer.push((*slot, more));
    }
    outer
}

/// The cells of the outer queue in the accepted slots of the outer rings
/// (S-149). The More cell takes the last accepted slot of the outer ring,
/// the action cells the slots before it, and the slots of the rings after
/// it take the actions that are left.
fn place_outer_at(
    outer: Vec<HiveCell>,
    slots: &[Axial],
    ring2_slots: usize,
) -> Vec<(Axial, HiveCell)> {
    let (actions, more) = split_more(outer);
    let more_index = match &more {
        Some(_) if ring2_slots > 0 => Some(ring2_slots - 1),
        Some(_) => Some(slots.len().saturating_sub(1)),
        None => None,
    };
    let mut placed: Vec<(Axial, HiveCell)> = Vec::new();
    let mut actions = actions.into_iter();
    for (i, slot) in slots.iter().copied().enumerate() {
        if Some(i) == more_index {
            if let Some(more) = more.clone() {
                placed.push((slot, more));
            }
            continue;
        }
        if let Some(cell) = actions.next() {
            placed.push((slot, cell));
        }
    }
    placed
}

/// The cells of a paged hive in their slots (S-149).
///
/// The ring-1 cells take the first accepted ring-1 slots. The actions of
/// the page take the accepted slots of the outer ring and the More cell
/// the last of them, so the More cell never reaches ring 1 and the hive
/// keeps its two rings on every page.
///
/// A placement whose outer ring accepts one slot gives it to the More
/// cell, and the action of the page takes the last free ring-1 slot. A
/// placement whose outer ring accepts no slot at all gives the last two
/// ring-1 slots to the More cell and to the action.
fn place_paged(
    ring1: &[HiveCell],
    outer: &[HiveCell],
    inner: &[Axial],
    outer_slots: &[Axial],
    ring2_slots: usize,
) -> Vec<(Axial, HiveCell)> {
    let (actions, more) = split_more(outer.to_vec());
    let mut placed: Vec<(Axial, HiveCell)> =
        inner.iter().copied().zip(ring1.iter().cloned()).collect();
    // The ring-1 slots the ring-1 cells did not take.
    let mut free: Vec<Axial> = inner.iter().copied().skip(ring1.len()).collect();
    // The accepted slots of the outer ring.
    let ring2: Vec<Axial> = outer_slots
        .iter()
        .copied()
        .take(ring2_slots.min(outer_slots.len()))
        .collect();
    // The More cell takes the last accepted slot of the outer ring. With
    // no accepted outer slot it takes the last free ring-1 slot.
    let more_slot = match (&more, ring2.len()) {
        (Some(_), 0) => free.pop(),
        (Some(_), n) => ring2.get(n - 1).copied(),
        (None, _) => None,
    };
    // The actions of the page take the outer slots before the More cell.
    let mut action_slots: Vec<Axial> = match (&more, ring2.len()) {
        (Some(_), 0) => Vec::new(),
        (Some(_), n) => ring2[..n - 1].to_vec(),
        (None, _) => ring2.clone(),
    };
    if action_slots.len() < actions.len() {
        // The outer ring has no room for the rest of the page: they take
        // the ring-1 slots the ring-1 cells left free.
        action_slots.append(&mut free);
    }
    for (slot, cell) in action_slots.into_iter().zip(actions) {
        placed.push((slot, cell));
    }
    if let (Some(slot), Some(more)) = (more_slot, more) {
        placed.push((slot, more));
    }
    placed
}

/// The outermost ring the hive at an anchor may use (S-019 decision 5).
pub const MAX_RINGS: i32 = 4;

/// The centre of a hive at `anchor` on a `screen` sized output: the anchor
/// clamped so the centre cell plus `PAD` is on screen.
fn clamped_centre(screen: (u32, u32), anchor: (f32, f32)) -> (f32, f32) {
    let radius = HEX_SIZE_MD / 3f32.sqrt();
    let (mx, my) = (radius * 3f32.sqrt() / 2.0 + PAD, radius + PAD);
    let (w, h) = (screen.0 as f32, screen.1 as f32);
    (anchor.0.max(mx).min(w - mx), anchor.1.max(my).min(h - my))
}

/// The slots of rings 1 to [`MAX_RINGS`] that fit on a `screen` sized
/// output for a hive centred at `anchor` (S-019 decision 5). A slot is
/// used only when its hexagon plus `PAD` is on screen.
fn accepted_rings(rtl: bool, screen: (u32, u32), anchor: (f32, f32)) -> Vec<Vec<Axial>> {
    const EPS: f32 = 0.01;
    let radius = HEX_SIZE_MD / 3f32.sqrt();
    let (mx, my) = (radius * 3f32.sqrt() / 2.0 + PAD, radius + PAD);
    let (w, h) = (screen.0 as f32, screen.1 as f32);
    let centre = clamped_centre(screen, anchor);
    let offsets = Layout {
        radius,
        gap: HEX_GAP,
        rtl,
        origin: (0.0, 0.0),
    };
    let fits = |a: &Axial| {
        let (x, y) = offsets.center(*a);
        let (x, y) = (centre.0 + x, centre.1 + y);
        // The centre of every slot ranges where the clamped anchor may.
        (mx - EPS..=(w - mx) + EPS).contains(&x) && (my - EPS..=(h - my) + EPS).contains(&y)
    };
    (1..=MAX_RINGS)
        .map(|k| ring(k).into_iter().filter(fits).collect())
        .collect()
}

/// The slots of ring 1 and of the outer ring a hive placed at `anchor` on a
/// `screen` sized output accepts (S-149). Without an anchor the hive is
/// centred and every slot of the two rings is accepted.
pub fn accepted_slots(rtl: bool, screen: (u32, u32), anchor: Option<(f32, f32)>) -> (usize, usize) {
    let Some(anchor) = anchor else {
        return (RING1, RING2);
    };
    let accepted = accepted_rings(rtl, screen, anchor);
    (accepted[0].len(), accepted[1].len())
}

/// The slots of the outer ring a hive placed at `anchor` on a `screen`
/// sized output accepts (S-149). The page of the outer queue holds one
/// action less, because the More cell takes the last of them.
pub fn accepted_outer_slots(rtl: bool, screen: (u32, u32), anchor: Option<(f32, f32)>) -> usize {
    accepted_slots(rtl, screen, anchor).1
}

/// The hive with its centre at `anchor` on a `screen` sized output (S-019
/// AC3), and where it sits there at scale `scale120`/120. Without an anchor
/// it is [`plan_rings`] centred by [`place`](crate::overlay::place).
///
/// The anchor is clamped so the centre cell plus `PAD` is on screen. A slot
/// is used only when its hexagon plus `PAD` is on screen too. Ring-1 cells
/// take the accepted ring-1 slots; the rest, ring-1 overflow first, fill the
/// accepted slots of rings 2 to [`MAX_RINGS`] in spiral order. The surface
/// is the bounding box of the centre and every accepted slot of the rings
/// it sizes for.
pub fn plan_at(
    cells: Vec<HiveCell>,
    rtl: bool,
    min_rings: i32,
    screen: (u32, u32),
    anchor: Option<(f32, f32)>,
    scale120: u32,
) -> (Plan, crate::overlay::Placement) {
    let Some(anchor) = anchor else {
        let plan = plan_rings(cells, rtl, min_rings);
        let size = (plan.width.ceil() as u32, plan.height.ceil() as u32);
        let placement = crate::overlay::place(screen, size, scale120);
        return (plan, placement);
    };
    let radius = HEX_SIZE_MD / 3f32.sqrt();
    // The same half width `Layout::extent` uses, so a hive with every slot
    // accepted has the size of `plan_rings`.
    let (mx, my) = (radius * 3f32.sqrt() / 2.0 + PAD, radius + PAD);
    let centre = clamped_centre(screen, anchor);
    let accepted = accepted_rings(rtl, screen, anchor);
    let offsets = Layout {
        radius,
        gap: HEX_GAP,
        rtl,
        origin: (0.0, 0.0),
    };

    // S-149: a paged hive never collapses into the compact one-ring
    // window, so the More cell stays in the outer ring.
    let paged = is_paged(&cells);
    let (ring1, ring2) = if paged {
        assign_paged(cells)
    } else {
        assign(cells)
    };
    let inner = &accepted[0];
    let n1 = ring1.len().min(inner.len());
    let mut ring1 = ring1;
    let mut outer = ring1.split_off(n1);
    outer.extend(ring2);
    let outer_slots: Vec<Axial> = (2..=MAX_RINGS)
        .zip(&accepted[1..])
        .flat_map(|(_, v)| v.iter().copied())
        .collect();
    // S-149: the More cell takes the last slot of ring 2, the action cells
    // the slots before it and the outer rings after it. Without a slot of
    // ring 2, it takes the last slot the screen has.
    let ring2_slots = accepted[1].len();
    let placed = if paged {
        place_paged(&ring1, &outer, inner, &outer_slots, ring2_slots)
    } else {
        place_outer_at(outer, &outer_slots, ring2_slots)
    };
    let used = placed.last().map_or(1, |(a, _)| a.distance());
    let rings = used.max(min_rings).max(if paged { 2 } else { 1 });
    // A lone ring spreads its cells; with every slot filled this is the
    // slots in order, and with no cell it is empty.
    let ring1_slots: Vec<Axial> = if rings == 1 {
        (0..n1).map(|i| inner[i * inner.len() / n1]).collect()
    } else {
        inner.clone()
    };

    let mut b = (0f32, 0f32, 0f32, 0f32);
    for a in accepted.iter().take(rings as usize).flatten() {
        let (x, y) = offsets.center(*a);
        b = (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y));
    }
    let (width, height) = (b.2 - b.0 + 2.0 * mx, b.3 - b.1 + 2.0 * my);
    let layout = Layout {
        origin: (mx - b.0, my - b.1),
        ..offsets
    };
    let slots: Vec<(Axial, HiveCell)> = if paged {
        placed
    } else {
        ring1_slots.into_iter().zip(ring1).chain(placed).collect()
    };
    let cells = slots
        .into_iter()
        .map(|(axial, cell)| {
            let (x, y) = layout.center(axial);
            Placed { cell, axial, x, y }
        })
        .collect();
    let size = (width.ceil() as u32, height.ceil() as u32);
    let placement = crate::overlay::Placement {
        x: (centre.0 - layout.origin.0).round() as i32,
        y: (centre.1 - layout.origin.1).round() as i32,
        ..crate::overlay::place(screen, size, scale120)
    };
    let plan = Plan {
        cells,
        width,
        height,
        layout,
    };
    (plan, placement)
}

impl Plan {
    /// Index into `cells` of the action under the point, or -1 for the
    /// centre, a gap, an empty slot or the transparent corners.
    pub fn pick(&self, x: f32, y: f32) -> i32 {
        let slots: Vec<Axial> = self.cells.iter().map(|c| c.axial).collect();
        self.layout.pick(&slots, x, y).map_or(-1, |i| i as i32)
    }
}

/// A navigation key, as sent by the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nav {
    Next,
    Prev,
    Left,
    Right,
    Up,
    Down,
}

impl Nav {
    /// The integer code the .slint side sends: 0 Tab, 1 Shift+Tab, 2 Left,
    /// 3 Right, 4 Up, 5 Down.
    pub fn from_code(code: i32) -> Option<Nav> {
        match code {
            0 => Some(Nav::Next),
            1 => Some(Nav::Prev),
            2 => Some(Nav::Left),
            3 => Some(Nav::Right),
            4 => Some(Nav::Up),
            5 => Some(Nav::Down),
            _ => None,
        }
    }
}

/// Moves the keyboard highlight. `current` is -1 when nothing is highlighted.
/// Next, Down and the arrow pointing along the reading direction (Right in
/// LTR, Left in Arabic) go forward through the ring order; the others go
/// back. Wraps at both ends. Returns -1 when there are no cells.
pub fn step(current: i32, nav: Nav, count: usize, rtl: bool) -> i32 {
    if count == 0 {
        return -1;
    }
    let forward = match nav {
        Nav::Next | Nav::Down => true,
        Nav::Prev | Nav::Up => false,
        Nav::Right => !rtl,
        Nav::Left => rtl,
    };
    let n = count as i32;
    if current < 0 {
        return if forward { 0 } else { n - 1 };
    }
    let delta = if forward { 1 } else { -1 };
    (current + delta).rem_euclid(n)
}

/// Where the hive sits in choose-during-drag mode (S-135 AC3): offset from
/// the pointer by the clearance toward a screen side with room, or centred on
/// the pointer as a dead zone when no side can hold the clearance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClearancePlacement {
    pub x: f32,
    pub y: f32,
    pub dead_zone: bool,
}

/// The hive centre for a pointer on a screen of the given logical size.
pub fn plan_clear_at(
    pointer: (f32, f32),
    screen: (f32, f32),
    clearance: f32,
) -> ClearancePlacement {
    let (px, py) = pointer;
    let (sw, sh) = screen;
    let dead = ClearancePlacement {
        x: px,
        y: py,
        dead_zone: true,
    };
    let near = px < clearance || px > sw - clearance || py < clearance || py > sh - clearance;
    if !near {
        return dead;
    }
    let mut best: Option<(f32, f32, f32)> = None;
    let mut consider = |x: f32, y: f32, room: f32| {
        if (0.0..=sw).contains(&x) && (0.0..=sh).contains(&y) && room >= clearance {
            match best {
                Some((_, _, most)) if most >= room => {}
                _ => best = Some((x, y, room)),
            }
        }
    };
    consider(px - clearance, py, px);
    consider(px + clearance, py, sw - px);
    consider(px, py - clearance, py);
    consider(px, py + clearance, sh - py);
    match best {
        Some((x, y, _)) => ClearancePlacement {
            x,
            y,
            dead_zone: false,
        },
        None => dead,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn ids(v: &[HiveCell]) -> Vec<&str> {
        v.iter().map(|c| c.id.as_str()).collect()
    }

    fn many(prefix: &str, n: usize, ring: u8) -> Vec<HiveCell> {
        (0..n)
            .map(|i| cell(&format!("{prefix}{i}"), ring))
            .collect()
    }

    #[test]
    fn ring_one_and_two_keep_their_order() {
        let mut cells = vec![cell("webp", 1), cell("gray", 2), cell("png", 1)];
        cells.extend(many("x", 5, 2));
        let (r1, r2) = assign(cells);
        assert_eq!(ids(&r1), ["webp", "png"]);
        assert_eq!(ids(&r2), ["gray", "x0", "x1", "x2", "x3", "x4"]);
    }

    #[test]
    fn ring_zero_counts_as_ring_one_and_above_two_as_ring_two() {
        let mut cells = vec![cell("a", 0), cell("b", 3), cell("c", 255)];
        cells.extend(many("x", 4, 2));
        let (r1, r2) = assign(cells);
        assert_eq!(ids(&r1), ["a"]);
        assert_eq!(ids(&r2), ["b", "c", "x0", "x1", "x2", "x3"]);
    }

    #[test]
    fn ring_one_overflow_moves_to_the_front_of_ring_two() {
        let mut cells = many("r1-", 8, 1);
        cells.insert(0, cell("late", 2));
        let (r1, r2) = assign(cells);
        assert_eq!(ids(&r1), ["r1-0", "r1-1", "r1-2", "r1-3", "r1-4", "r1-5"]);
        assert_eq!(ids(&r2), ["r1-6", "r1-7", "late"]);
    }

    #[test]
    fn more_than_eighteen_cells_are_dropped() {
        let mut cells = many("a", 10, 1);
        cells.extend(many("b", 10, 2));
        let (r1, r2) = assign(cells);
        assert_eq!(r1.len(), 6);
        assert_eq!(r2.len(), 12);
        assert_eq!(ids(&r2)[..4], ["a6", "a7", "a8", "a9"]);
        assert_eq!(ids(&r2)[11], "b7");
    }

    /// The cells of one page, as the layout takes them.
    fn page_cells(outer: &[HiveCell], index: usize) -> Vec<HiveCell> {
        page(outer, index, PAGE_SIZE, Lang::En)
    }

    /// The cells of one page of a hive placed with `slots` accepted slots
    /// of the outer ring.
    fn page_at(outer: &[HiveCell], index: usize, slots: usize) -> Vec<HiveCell> {
        page(outer, index, page_size(slots), Lang::En)
    }

    /// `n` actions of the outer queue, behind six common actions, so the
    /// queue is not the compact single ring.
    fn queue(n: usize) -> Vec<HiveCell> {
        let mut cells = many("r", 6, 1);
        cells.extend(many("a", n, 2));
        let (_, outer) = outer_queue(cells);
        outer
    }

    #[test]
    fn the_outer_queue_keeps_every_action() {
        let (r1, outer) = outer_queue(many("a", 19, 2));
        assert!(r1.is_empty());
        assert_eq!(outer.len(), 19);
        assert_eq!(ids(&outer)[..3], ["a0", "a1", "a2"]);
        assert_eq!(ids(&outer)[18], "a18");
        // The ring-1 overflow comes before the ring-2 cells.
        let mut cells = many("r1-", 8, 1);
        cells.insert(0, cell("late", 2));
        let (r1, outer) = outer_queue(cells);
        assert_eq!(ids(&r1), ["r1-0", "r1-1", "r1-2", "r1-3", "r1-4", "r1-5"]);
        assert_eq!(ids(&outer), ["r1-6", "r1-7", "late"]);
    }

    #[test]
    fn the_page_count_follows_the_queue_length() {
        for (n, want) in [
            (0, 1),
            (12, 1),
            (13, 2),
            (18, 2),
            (19, 2),
            (22, 2),
            (23, 3),
            (30, 3),
        ] {
            let outer = many("a", n, 2);
            assert_eq!(pages(&outer, PAGE_SIZE), want, "{n}");
        }
    }

    #[test]
    fn a_page_holds_eleven_actions_and_the_more_cell() {
        let outer = many("a", 19, 2);
        let first = page_cells(&outer, 0);
        assert_eq!(first.len(), 12);
        assert_eq!(
            ids(&first)[..11],
            [
                "a0", "a1", "a2", "a3", "a4", "a5", "a6", "a7", "a8", "a9", "a10"
            ]
        );
        assert_eq!(first[11].id, MORE_ID);
        let last = page_cells(&outer, 1);
        assert_eq!(last.len(), 9);
        assert_eq!(
            ids(&last)[..8],
            ["a11", "a12", "a13", "a14", "a15", "a16", "a17", "a18"]
        );
        assert_eq!(last[8].id, MORE_ID);
        // A page past the last one shows the last one.
        assert_eq!(page_cells(&outer, 9), last);
    }

    #[test]
    fn every_action_is_reachable_exactly_once_across_the_pages() {
        for n in [13, 18, 19, 30] {
            let outer = many("a", n, 2);
            let mut seen: Vec<String> = Vec::new();
            for p in 0..pages(&outer, PAGE_SIZE) {
                for cell in page_cells(&outer, p) {
                    if cell.id != MORE_ID {
                        seen.push(cell.id.clone());
                    }
                }
            }
            assert_eq!(seen.len(), n, "{n}");
            let want: Vec<String> = outer.iter().map(|c| c.id.clone()).collect();
            assert_eq!(seen, want, "{n}");
        }
    }

    #[test]
    fn the_page_size_follows_the_accepted_slots() {
        // The unconstrained placement: the whole outer ring.
        assert_eq!(page_size(RING2), PAGE_SIZE);
        assert_eq!(page_size(12), 11);
        assert_eq!(page_size(8), 7);
        assert_eq!(page_size(5), 4);
        assert_eq!(page_size(2), 1);
        // A placement with one slot or less holds one action per page: the
        // More cell takes the only outer slot, or a ring-1 slot when there
        // is no outer slot at all.
        assert_eq!(page_size(1), 1);
        assert_eq!(page_size(0), 1);
    }

    #[test]
    fn the_reserved_ring_one_slots_follow_the_accepted_outer_slots() {
        assert_eq!(reserved_ring1_slots(RING2), 0);
        assert_eq!(reserved_ring1_slots(2), 0);
        assert_eq!(reserved_ring1_slots(1), 1);
        assert_eq!(reserved_ring1_slots(0), 2);
    }

    /// The cells of a page and the slots they take, for a placement with
    /// `ring1_slots` accepted ring-1 slots and `outer_slots` accepted
    /// outer slots.
    fn paged_placement(
        cells: &[HiveCell],
        ring1_slots: usize,
        outer_slots: usize,
        index: usize,
    ) -> Vec<(Axial, HiveCell)> {
        let reserved = reserved_ring1_slots(outer_slots);
        let (ring1, outer) = queue_at(cells, ring1_slots.saturating_sub(reserved));
        let size = page_size(outer_slots);
        let page_cells = page(&outer, index, size, Lang::En);
        let inner: Vec<Axial> = ring(1).into_iter().take(ring1_slots).collect();
        let outer_axial: Vec<Axial> = ring(2).into_iter().take(outer_slots).collect();
        place_paged(&ring1, &page_cells, &inner, &outer_axial, outer_slots)
    }

    #[test]
    fn a_page_with_two_accepted_outer_slots_holds_one_action_and_the_more_cell() {
        let mut cells = many("r", 6, 1);
        cells.extend(many("a", 5, 2));
        let placed = paged_placement(&cells, 6, 2, 0);
        // The ring-1 cells keep their slots, the page adds two cells.
        assert_eq!(placed.len(), 8);
        // The action takes the outer slot before the More cell.
        assert_eq!(placed[6].0, ring(2)[0]);
        assert_eq!(placed[6].1.id, "a0");
        // The More cell takes the last accepted outer slot.
        let more = placed
            .iter()
            .position(|(_, c)| c.id == MORE_ID)
            .expect("the More cell");
        assert_eq!(placed[more].0, ring(2)[1]);
    }

    #[test]
    fn a_page_with_one_accepted_outer_slot_gives_it_to_the_more_cell() {
        let mut cells = many("r", 6, 1);
        cells.extend(many("a", 5, 2));
        let placed = paged_placement(&cells, 6, 1, 0);
        // One ring-1 slot is reserved for the action of the page.
        assert_eq!(placed.len(), 7);
        // The More cell takes the only accepted outer slot.
        let more = placed
            .iter()
            .position(|(_, c)| c.id == MORE_ID)
            .expect("the More cell");
        assert_eq!(placed[more].0, ring(2)[0]);
        // The action of the page takes the ring-1 slot the More cell
        // leaves free. It is the ring-1 cell that did not fit.
        let action = placed
            .iter()
            .position(|(_, c)| c.id == "r5")
            .expect("the action of the page");
        assert_eq!(placed[action].0, ring(1)[5]);
    }

    #[test]
    fn a_page_without_any_accepted_outer_slot_keeps_both_cells_in_ring_one() {
        let mut cells = many("r", 6, 1);
        cells.extend(many("a", 5, 2));
        let placed = paged_placement(&cells, 6, 0, 0);
        // Two ring-1 slots are reserved for the More cell and the action.
        assert_eq!(placed.len(), 6);
        // The action of the page takes the ring-1 slot before the More
        // cell, and the More cell the last one.
        let action = placed
            .iter()
            .position(|(_, c)| c.id == "r4")
            .expect("the action of the page");
        assert_eq!(placed[action].0, ring(1)[4]);
        let more = placed
            .iter()
            .position(|(_, c)| c.id == MORE_ID)
            .expect("the More cell");
        assert_eq!(placed[more].0, ring(1)[5]);
    }

    #[test]
    fn every_action_is_reachable_with_few_accepted_outer_slots() {
        for outer_slots in [0, 1, 2] {
            let mut cells = many("r", 6, 1);
            cells.extend(many("a", 7, 2));
            let reserved = reserved_ring1_slots(outer_slots);
            let (ring1, outer) = queue_at(&cells, 6usize.saturating_sub(reserved));
            let size = page_size(outer_slots);
            let mut seen: Vec<String> = Vec::new();
            for p in 0..pages(&outer, size) {
                let page_cells = page(&outer, p, size, Lang::En);
                for cell in &page_cells {
                    if cell.id != MORE_ID {
                        seen.push(cell.id.clone());
                    }
                }
                // Every cell of the page is placed: none is dropped.
                let inner: Vec<Axial> = ring(1).into_iter().take(6).collect();
                let outer_axial: Vec<Axial> = ring(2).into_iter().take(outer_slots).collect();
                let placed = place_paged(&ring1, &page_cells, &inner, &outer_axial, outer_slots);
                assert_eq!(
                    placed.len(),
                    ring1.len() + page_cells.len(),
                    "{outer_slots} outer slots, page {p}"
                );
            }
            // Every action of the queue is on exactly one page.
            let want: Vec<String> = outer.iter().map(|c| c.id.clone()).collect();
            assert_eq!(seen, want, "{outer_slots} outer slots");
        }
    }

    #[test]
    fn a_paged_hive_keeps_two_rings_and_the_more_cell_out_of_ring_one() {
        // Two ring-1 cells, one action and the More cell: five cells in
        // all, few enough for the compact one-ring rule. A paged hive never
        // takes it.
        let mut cells = many("r", 2, 1);
        cells.extend(page(&many("a", 3, 2), 0, 1, Lang::En));
        assert!(is_paged(&cells));
        let plan = plan_rings(cells, false, 1);
        // The window keeps the two rings of the hive.
        let two = plan_rings(many("x", 18, 1), false, 2);
        assert_eq!((plan.width, plan.height), (two.width, two.height));
        // The More cell is in the outer ring.
        let more = plan
            .cells
            .iter()
            .find(|c| c.cell.id == MORE_ID)
            .expect("the More cell");
        assert_eq!(more.axial.distance(), 2);
    }

    #[test]
    fn the_more_cell_takes_the_last_accepted_outer_slot_of_the_plan() {
        let screen = (1920, 1080);
        let anchor = (960.0, 540.0);
        let mut cells = many("r", 6, 1);
        cells.extend(page(&many("a", 20, 2), 0, PAGE_SIZE, Lang::En));
        let (plan, _) = plan_at(cells, false, 2, screen, Some(anchor), 120);
        let accepted = accepted_rings(false, screen, anchor);
        let more = plan
            .cells
            .iter()
            .find(|c| c.cell.id == MORE_ID)
            .expect("the More cell");
        assert_eq!(more.axial, *accepted[1].last().unwrap());
        // The actions of the page take the slots before it.
        assert_eq!(plan.cells.len(), 6 + PAGE_SIZE + 1);
    }

    #[test]
    fn every_action_is_reachable_with_fewer_accepted_slots() {
        for slots in [12, 8, 5] {
            for n in [13, 19, 30] {
                let outer = queue(n);
                let size = page_size(slots);
                let mut seen: Vec<String> = Vec::new();
                for p in 0..pages(&outer, size) {
                    let cells = page_at(&outer, p, slots);
                    // The More cell is the last one of the page.
                    assert_eq!(
                        cells.last().map(|c| c.id.as_str()),
                        Some(MORE_ID),
                        "{slots} slots, {n} actions, page {p}"
                    );
                    // The page holds the actions of the page and the More cell.
                    assert!(cells.len() <= size + 1, "{slots} slots, {n} actions");
                    for cell in cells {
                        if cell.id != MORE_ID {
                            seen.push(cell.id.clone());
                        }
                    }
                }
                assert_eq!(seen.len(), n, "{slots} slots, {n} actions");
                let want: Vec<String> = outer.iter().map(|c| c.id.clone()).collect();
                assert_eq!(seen, want, "{slots} slots, {n} actions");
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
                let cells = page_at(&outer, 0, slots);
                assert_eq!(cells.len(), n, "{slots} slots, {n} actions");
                assert!(
                    cells.iter().all(|c| c.id != MORE_ID),
                    "{slots} slots, {n} actions"
                );
            }
        }
    }

    #[test]
    fn the_more_cell_takes_the_last_accepted_slot() {
        for slots in [12, 8, 5] {
            let outer = queue(20);
            let cells = page_at(&outer, 0, slots);
            let accepted: Vec<Axial> = ring(2).into_iter().take(slots).collect();
            let placed = place_outer_at(cells, &accepted, slots);
            // Every cell of the page is placed.
            assert_eq!(placed.len(), slots, "{slots} slots");
            let more = placed
                .iter()
                .position(|(_, c)| c.id == MORE_ID)
                .expect("the More cell");
            assert_eq!(placed[more].0, *accepted.last().unwrap(), "{slots} slots");
        }
    }

    #[test]
    fn a_page_fits_the_slots_the_screen_accepts() {
        for (screen, anchor) in [
            ((1920, 1080), (960.0, 540.0)),
            ((300, 800), (170.0, 400.0)),
            ((300, 800), (40.0, 700.0)),
        ] {
            let (ring1_slots, outer_slots) = accepted_slots(false, screen, Some(anchor));
            assert_eq!(
                accepted_outer_slots(false, screen, Some(anchor)),
                outer_slots,
                "{screen:?} at {anchor:?}"
            );
            let mut cells = many("r", 6, 1);
            cells.extend(many("a", 20, 2));
            let (ring1, outer) = queue_at(&cells, ring1_slots);
            let mut page_cells = ring1;
            page_cells.extend(page(&outer, 0, page_size(outer_slots), Lang::En));
            let (plan, _) = plan_at(page_cells.clone(), false, 2, screen, Some(anchor), 120);
            // No cell of the page is dropped.
            assert_eq!(
                plan.cells.len(),
                page_cells.len(),
                "{screen:?} at {anchor:?}"
            );
            // The More cell takes the last accepted slot of the outer ring.
            if let Some(more) = plan.cells.iter().find(|c| c.cell.id == MORE_ID) {
                let accepted = accepted_rings(false, screen, anchor);
                assert_eq!(
                    more.axial,
                    *accepted[1].last().unwrap(),
                    "{screen:?} at {anchor:?}"
                );
            }
        }
    }

    #[test]
    fn the_more_cell_is_a_control_and_never_an_action() {
        assert_eq!(activation(MORE_ID), Activation::More);
        assert_eq!(
            activation("image.to-text"),
            Activation::Action("image.to-text".into())
        );
        let more = more_cell(Lang::En, 1, 3);
        assert_eq!(more.id, MORE_ID);
        assert_eq!(more.code, "MORE");
        assert_eq!(more.label, "More actions, page 2 of 3");
        assert!(more.enabled);
        assert!(!more.customize);
        assert!(more.reason.is_empty());
        let arabic = more_cell(Lang::Ar, 1, 3);
        assert_eq!(arabic.code, "المزيد");
        assert_eq!(arabic.label, "إجراءات أخرى، الصفحة 2 من 3");
    }

    #[test]
    fn the_more_cell_sits_in_the_last_outer_slot() {
        let (ring1, outer) = outer_queue(many("a", 19, 2));
        let mut cells = ring1;
        cells.extend(page_cells(&outer, 0));
        let p = plan_rings(cells, false, 1);
        let more = p
            .cells
            .iter()
            .position(|c| c.cell.id == MORE_ID)
            .expect("the More cell");
        assert_eq!(p.cells[more].axial, ring(2)[11]);
        // The action cells fill the slots before it.
        assert_eq!(p.cells[0].axial, ring(2)[0]);
        assert_eq!(p.cells[10].axial, ring(2)[10]);
        // The More cell is the last one of the model, so the keyboard order
        // ends with it.
        assert_eq!(more, p.cells.len() - 1);
    }

    #[test]
    fn the_more_cell_mirrors_in_arabic() {
        let (ring1, outer) = outer_queue(many("a", 19, 2));
        let mut cells = ring1;
        cells.extend(page_cells(&outer, 0));
        let ltr = plan_rings(cells.clone(), false, 1);
        let rtl = plan_rings(cells, true, 1);
        let more_ltr = ltr.cells.iter().find(|c| c.cell.id == MORE_ID).unwrap();
        let more_rtl = rtl.cells.iter().find(|c| c.cell.id == MORE_ID).unwrap();
        assert_eq!(more_ltr.axial, more_rtl.axial);
        assert!(
            (more_ltr.x - ltr.layout.origin.0 + (more_rtl.x - rtl.layout.origin.0)).abs() < 1e-3
        );
        assert_eq!(more_ltr.y, more_rtl.y);
    }

    #[test]
    fn every_page_keeps_the_window_size() {
        let (ring1, outer) = outer_queue(many("a", 30, 2));
        let first = plan_rings([ring1.clone(), page_cells(&outer, 0)].concat(), false, 1);
        let last = plan_rings([ring1, page_cells(&outer, 2)].concat(), false, 1);
        assert_eq!((first.width, first.height), (last.width, last.height));
        assert_eq!(first.cells.len(), 12);
        assert_eq!(last.cells.len(), 9);
    }

    #[test]
    fn a_queue_that_fits_keeps_the_old_plan() {
        // Twelve outer cells: one page, no More cell, the same slots.
        let p = plan(many("a", 12, 2), false);
        assert_eq!(p.cells.len(), 12);
        assert!(p.cells.iter().all(|c| c.cell.id != MORE_ID));
        let slots: Vec<Axial> = p.cells.iter().map(|c| c.axial).collect();
        assert_eq!(slots, ring(2));
        // Eighteen cells: six in ring 1 and twelve in ring 2, as before.
        let mut cells = many("a", 6, 1);
        cells.extend(many("b", 12, 2));
        let p = plan(cells, false);
        assert_eq!(p.cells.len(), 18);
        assert!(p.cells.iter().all(|c| c.cell.id != MORE_ID));
        let slots: Vec<Axial> = p.cells.iter().map(|c| c.axial).collect();
        let mut want = ring(1);
        want.extend(ring(2));
        assert_eq!(slots, want);
    }

    #[test]
    fn short_ring_one_is_not_filled_from_ring_two() {
        let mut cells = many("a", 2, 1);
        cells.extend(many("b", 14, 2));
        let (r1, r2) = assign(cells);
        assert_eq!(ids(&r1), ["a0", "a1"]);
        assert_eq!(r2.len(), 12);
        assert_eq!(ids(&r2)[11], "b11");
    }

    #[test]
    fn six_or_fewer_cells_all_go_to_ring_one() {
        let (r1, r2) = assign(vec![cell("gray", 2), cell("webp", 1), cell("png", 1)]);
        assert_eq!(ids(&r1), ["webp", "png", "gray"]);
        assert!(r2.is_empty());
        let mut six = many("a", 4, 1);
        six.extend(many("b", 2, 2));
        let (r1, r2) = assign(six);
        assert_eq!(r1.len(), 6);
        assert!(r2.is_empty());
        let mut seven = many("a", 4, 1);
        seven.extend(many("b", 3, 2));
        let (r1, r2) = assign(seven);
        assert_eq!((r1.len(), r2.len()), (4, 3));
    }

    #[test]
    fn a_partial_single_ring_spreads_evenly() {
        let r1 = ring(1);
        let slots = |n: usize| -> Vec<Axial> {
            plan(many("a", n, 1), false)
                .cells
                .iter()
                .map(|c| c.axial)
                .collect()
        };
        assert_eq!(slots(1), [r1[0]]);
        assert_eq!(slots(2), [r1[0], r1[3]]);
        assert_eq!(slots(3), [r1[0], r1[2], r1[4]]);
        assert_eq!(slots(4), [r1[0], r1[1], r1[3], r1[4]]);
        assert_eq!(slots(6), r1);
        // With a second ring the inner ring keeps consecutive slots.
        let mut cells = many("a", 2, 1);
        cells.extend(many("b", 5, 2));
        let p = plan(cells, false);
        assert_eq!(p.cells[0].axial, r1[0]);
        assert_eq!(p.cells[1].axial, r1[1]);
    }

    #[test]
    fn plan_puts_cells_on_ring_slots_in_order() {
        let mut cells = many("a", 3, 1);
        cells.extend(many("b", 4, 2));
        let p = plan(cells, false);
        let slots: Vec<Axial> = p.cells.iter().map(|c| c.axial).collect();
        let r1 = ring(1);
        let r2 = ring(2);
        assert_eq!(slots, [r1[0], r1[1], r1[2], r2[0], r2[1], r2[2], r2[3]]);
        assert_eq!(
            ids(&p.cells.iter().map(|c| c.cell.clone()).collect::<Vec<_>>()),
            ["a0", "a1", "a2", "b0", "b1", "b2", "b3"]
        );
        for c in &p.cells {
            assert_eq!((c.x, c.y), p.layout.center(c.axial));
        }
    }

    #[test]
    fn plan_centres_the_hive_in_a_window_sized_for_the_rings_used() {
        let one = plan(many("a", 3, 1), false);
        let mut seven = many("a", 1, 1);
        seven.extend(many("b", 6, 2));
        let two = plan(seven, false);
        let empty = plan(Vec::new(), false);
        assert!(one.width < two.width && one.height < two.height);
        assert_eq!((empty.width, empty.height), (one.width, one.height));
        for p in [&one, &two, &empty] {
            assert_eq!(p.layout.origin, (p.width / 2.0, p.height / 2.0));
        }
        let radius = crate::tokens::HEX_SIZE_MD / 3f32.sqrt();
        let (w, h) = Layout::extent(radius, crate::tokens::HEX_GAP, 2, crate::tokens::PAD);
        assert!((two.width - w).abs() < 1e-3 && (two.height - h).abs() < 1e-3);
        assert!((two.layout.radius - radius).abs() < 1e-3);
        assert_eq!(two.layout.gap, crate::tokens::HEX_GAP);
    }

    #[test]
    fn plan_mirrors_for_arabic() {
        let cells = vec![cell("a", 1), cell("b", 2)];
        let ltr = plan(cells.clone(), false);
        let rtl = plan(cells, true);
        assert!(rtl.layout.rtl && !ltr.layout.rtl);
        for (l, r) in ltr.cells.iter().zip(&rtl.cells) {
            assert_eq!(l.axial, r.axial);
            assert!((l.x - ltr.layout.origin.0 + (r.x - rtl.layout.origin.0)).abs() < 1e-3);
            assert_eq!(l.y, r.y);
        }
        // First ring-1 cell: top right in LTR, top left in Arabic.
        assert!(ltr.cells[0].x > ltr.layout.origin.0);
        assert!(rtl.cells[0].x < rtl.layout.origin.0);
    }

    #[test]
    fn pick_returns_action_index_and_minus_one_elsewhere() {
        let p = plan(vec![cell("a", 1), cell("b", 1), cell("c", 2)], true);
        for (i, c) in p.cells.iter().enumerate() {
            assert_eq!(p.pick(c.x, c.y), i as i32);
        }
        let (ox, oy) = p.layout.origin;
        assert_eq!(p.pick(ox, oy), -1, "centre");
        assert_eq!(p.pick(0.0, 0.0), -1, "corner");
        // Empty ring-1 slot: three cells spread over slots 0, 2 and 4.
        let (ex, ey) = p.layout.center(ring(1)[1]);
        assert_eq!(p.pick(ex, ey), -1, "empty slot");
    }

    #[test]
    fn nav_codes_match_the_slint_side() {
        let want = [
            Nav::Next,
            Nav::Prev,
            Nav::Left,
            Nav::Right,
            Nav::Up,
            Nav::Down,
        ];
        for (code, nav) in want.iter().enumerate() {
            assert_eq!(Nav::from_code(code as i32), Some(*nav));
        }
        assert_eq!(Nav::from_code(6), None);
        assert_eq!(Nav::from_code(-1), None);
    }

    #[test]
    fn step_moves_forward_and_back_with_wrap() {
        assert_eq!(step(-1, Nav::Next, 4, false), 0);
        assert_eq!(step(-1, Nav::Prev, 4, false), 3);
        assert_eq!(step(0, Nav::Next, 4, false), 1);
        assert_eq!(step(3, Nav::Next, 4, false), 0);
        assert_eq!(step(0, Nav::Prev, 4, false), 3);
        assert_eq!(step(2, Nav::Prev, 4, false), 1);
        assert_eq!(step(1, Nav::Down, 4, false), 2);
        assert_eq!(step(1, Nav::Up, 4, false), 0);
        assert_eq!(step(-1, Nav::Next, 0, false), -1);
        assert_eq!(
            step(5, Nav::Next, 3, false),
            0,
            "stale index wraps into range"
        );
    }

    #[test]
    fn arrows_follow_the_reading_direction() {
        assert_eq!(step(1, Nav::Right, 4, false), 2);
        assert_eq!(step(1, Nav::Left, 4, false), 0);
        assert_eq!(step(1, Nav::Right, 4, true), 0);
        assert_eq!(step(1, Nav::Left, 4, true), 2);
        // Tab and vertical arrows do not depend on the direction.
        assert_eq!(step(1, Nav::Next, 4, true), 2);
        assert_eq!(step(1, Nav::Down, 4, true), 2);
        assert_eq!(step(1, Nav::Up, 4, true), 0);
    }

    #[test]
    fn a_two_ring_minimum_keeps_the_window_and_the_first_slot() {
        let full = plan(many("x", 18, 1), false);
        for n in [0, 3, 18] {
            let p = plan_rings(many("x", n, 1), false, 2);
            assert_eq!((p.width, p.height), (full.width, full.height), "{n}");
            assert_eq!(p.layout.origin, full.layout.origin, "{n}");
            if n > 0 {
                assert_eq!(
                    (p.cells[0].x, p.cells[0].y),
                    (full.cells[0].x, full.cells[0].y)
                );
            }
        }
        let one = plan_rings(many("x", 3, 1), false, 1);
        assert!(one.width < full.width);
    }
}
