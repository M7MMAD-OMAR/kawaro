//! Drops from a file manager on the hive window on X11 (S-115, ADR 0003).
//! The same shape as `dnd/windows.rs`: winit reports the dragged files but no
//! position, so between the first hover event and the drop, the cancel or
//! the hive's close a timer of `TICK_MS` reads the pointer from the hive's
//! `PointerSource` and feeds the pure `DropTracker`. The hover events of one
//! drag arrive in a burst, one per file, so the cells are asked for once,
//! `2 * TICK_MS` after the last of them. A drop on a cell runs through
//! `Inner::choose`, the path of a click. Nothing here deletes, moves or
//! renames a dropped file.

use crate::dnd::{Event, State};
use crate::drop_track::{DropOutcome, DropTracker, TICK_MS};
use crate::session::Session;
use crate::{DROP_HIVE, Inner};
use slint::winit_030::winit::event::WindowEvent;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

thread_local! {
    static TRACKER: RefCell<DropTracker> = RefCell::new(DropTracker::default());
    /// Runs only during a drag over a drop hive.
    static TICKER: slint::Timer = slint::Timer::default();
    /// Fires once after the last hover event of a burst.
    static BATCH: slint::Timer = slint::Timer::default();
    static BATCH_PENDING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// A drag event of the hive window, from `FocusWatcher::window_event`.
pub fn window_event(event: &WindowEvent) {
    match event {
        WindowEvent::HoveredFile(path) => hovered(path.clone()),
        WindowEvent::DroppedFile(_) => {
            TICKER.with(|t| t.stop());
            flush();
            dropped();
        }
        WindowEvent::HoveredFileCancelled => {
            TICKER.with(|t| t.stop());
            BATCH.with(|t| t.stop());
            BATCH_PENDING.with(|p| p.set(false));
            cancelled();
        }
        _ => {}
    }
}

/// The hive of the current drop show on X11, if it waits for a drag.
fn drop_hive() -> Option<Rc<Inner>> {
    DROP_HIVE
        .with(|h| h.borrow().upgrade())
        .filter(|i| i.x11.session.get() == Session::X11)
        .filter(|i| matches!(i.machine.borrow().state(), State::Waiting | State::Over(_)))
}

/// The pointer in the hive's logical surface coordinates (scale 1).
fn cursor(inner: &Inner) -> Option<(f32, f32)> {
    let p = inner.x11.source.borrow().pointer()?;
    let (x, y) = inner.x11.position.get()?;
    Some(((p.x - x) as f32, (p.y - y) as f32))
}

fn hovered(path: PathBuf) {
    let Some(_inner) = drop_hive() else { return };
    TRACKER.with(|t| {
        let mut t = t.borrow_mut();
        if !t.is_active() {
            *t = DropTracker::default();
        }
        t.on_hover_file(path);
    });
    BATCH_PENDING.with(|p| p.set(true));
    BATCH.with(|t| {
        t.start(
            slint::TimerMode::SingleShot,
            Duration::from_millis(u64::from(TICK_MS) * 2),
            flush,
        )
    });
}

/// Asks for the cells of the files hovered so far, once per burst.
fn flush() {
    if !BATCH_PENDING.with(|p| p.replace(false)) {
        return;
    }
    BATCH.with(|t| t.stop());
    let Some(inner) = drop_hive() else { return };
    let files = TRACKER.with(|t| t.borrow().files().to_vec());
    let event = inner.drop_files(files);
    inner.drag(event);
    // A refused drag may have closed the hive already.
    if TRACKER.with(|t| t.borrow().is_active()) {
        TICKER.with(|t| {
            t.start(
                slint::TimerMode::Repeated,
                Duration::from_millis(u64::from(TICK_MS)),
                tick,
            )
        });
    }
}

/// The hive closed (from `Inner::close`): the drag's files are forgotten
/// and the pointer poll stops.
pub fn hive_closed() {
    TRACKER.with(|t| t.borrow_mut().on_close());
    TICKER.with(|t| t.stop());
    BATCH.with(|t| t.stop());
    BATCH_PENDING.with(|p| p.set(false));
}

fn tick() {
    let Some(inner) = drop_hive() else { return };
    let Some(at) = cursor(&inner) else { return };
    let cell = match inner.plan.borrow().as_ref() {
        Some(plan) => TRACKER.with(|t| t.borrow_mut().on_tick(plan, at)),
        None => None,
    };
    if let Some(cell) = cell {
        inner.drag(Event::Motion(usize::try_from(cell).ok()));
    }
}

fn dropped() {
    let Some(inner) = drop_hive() else {
        TRACKER.with(|t| t.borrow_mut().on_cancel());
        return;
    };
    let outcome = match (cursor(&inner), inner.plan.borrow().as_ref()) {
        (Some(at), Some(plan)) => TRACKER.with(|t| t.borrow_mut().on_drop(plan, at)),
        _ => DropOutcome::Ignored,
    };
    match outcome {
        // The files are the ones `on_drop_files` stored for the run.
        DropOutcome::Run { cell, .. } => inner.choose(cell),
        // A drop off every cell closes the hive, as on Wayland.
        DropOutcome::Ignored => {
            inner.drag(Event::Motion(None));
            inner.drag(Event::Drop);
        }
    }
}

fn cancelled() {
    TRACKER.with(|t| t.borrow_mut().on_cancel());
    if let Some(inner) = drop_hive() {
        inner.drag(Event::Leave);
    }
}
