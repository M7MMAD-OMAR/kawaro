//! Drops from Explorer on the hive window (S-047, ADR 0005 section 5). winit
//! reports the dragged files but no position, so between the first hover
//! event and the drop or cancel a timer of `TICK_MS` polls `GetCursorPos`
//! and feeds the pure `DropTracker`. A drop on a cell runs through
//! `Inner::choose`, the path of a click. Nothing here deletes, moves or
//! renames a dropped file. Windows only, left out of the mutants run; the
//! live check is S-052.

use crate::dnd::{Event, State};
use crate::drop_track::{DropOutcome, DropTracker, TICK_MS};
use crate::{DROP_HIVE, Inner};
use slint::ComponentHandle;
use slint::winit_030::winit::event::WindowEvent;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;
use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

thread_local! {
    static TRACKER: RefCell<DropTracker> = RefCell::new(DropTracker::default());
    /// winit sends one hover event per file: the files of this drag so far.
    static HOVERED: RefCell<Vec<PathBuf>> = const { RefCell::new(Vec::new()) };
    /// Runs only during a drag over a drop hive.
    static TICKER: slint::Timer = slint::Timer::default();
}

/// A drag event of the hive window, from `FocusWatcher::window_event`.
pub fn window_event(event: &WindowEvent) {
    match event {
        WindowEvent::HoveredFile(path) => hovered(path.clone()),
        WindowEvent::DroppedFile(_) => {
            TICKER.with(|t| t.stop());
            dropped();
        }
        WindowEvent::HoveredFileCancelled => {
            TICKER.with(|t| t.stop());
            cancelled();
        }
        _ => {}
    }
}

/// The hive of the current drop show, if it waits for a drag.
fn drop_hive() -> Option<Rc<Inner>> {
    DROP_HIVE
        .with(|h| h.borrow().upgrade())
        .filter(|i| matches!(i.machine.borrow().state(), State::Waiting | State::Over(_)))
}

/// The cursor in the hive's logical surface coordinates.
fn cursor(inner: &Inner) -> (f32, f32) {
    let mut p = POINT { x: 0, y: 0 };
    unsafe { GetCursorPos(&mut p) };
    let ui = inner.ui.borrow();
    let window = ui.window();
    let at = window.position();
    let scale = window.scale_factor();
    ((p.x - at.x) as f32 / scale, (p.y - at.y) as f32 / scale)
}

fn hovered(path: PathBuf) {
    let Some(inner) = drop_hive() else { return };
    let fresh = !TICKER.with(|t| t.running());
    let files = HOVERED.with(|h| {
        let mut h = h.borrow_mut();
        if fresh {
            h.clear();
        }
        h.push(path);
        h.clone()
    });
    // The cells for the dragged files, laid out like a Wayland enter.
    let event = inner.drop_files(files.clone());
    inner.drag(event);
    TRACKER.with(|t| {
        let mut t = t.borrow_mut();
        if fresh {
            *t = DropTracker::default();
        }
        t.on_hover(files);
    });
    if fresh {
        TICKER.with(|t| {
            t.start(
                slint::TimerMode::Repeated,
                Duration::from_millis(u64::from(TICK_MS)),
                tick,
            )
        });
    }
}

fn tick() {
    let Some(inner) = drop_hive() else { return };
    let at = cursor(&inner);
    let cell = match inner.plan.borrow().as_ref() {
        Some(plan) => TRACKER.with(|t| t.borrow_mut().on_tick(plan, at)),
        None => None,
    };
    if let Some(cell) = cell {
        inner.drag(Event::Motion(usize::try_from(cell).ok()));
    }
}

fn dropped() {
    HOVERED.with(|h| h.borrow_mut().clear());
    let Some(inner) = drop_hive() else {
        TRACKER.with(|t| t.borrow_mut().on_cancel());
        return;
    };
    let at = cursor(&inner);
    let outcome = match inner.plan.borrow().as_ref() {
        Some(plan) => TRACKER.with(|t| t.borrow_mut().on_drop(plan, at)),
        None => DropOutcome::Ignored,
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
    HOVERED.with(|h| h.borrow_mut().clear());
    TRACKER.with(|t| t.borrow_mut().on_cancel());
    if let Some(inner) = drop_hive() {
        inner.drag(Event::Leave);
    }
}
