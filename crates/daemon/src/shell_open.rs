//! Windows shell glue for `sbarhived open` (S-046): the foreground grant, the
//! foreground call after the hive shows, and the merging of the files that
//! Explorer sends one process at a time. Left out of the mutants run, the
//! live check is S-052; the decisions are in `args.rs`, `batch.rs`,
//! `core::merge` and `server::focus_log`.

use crate::batch::{OpenBatch, Step};
use crate::server::focus_log;
use crate::state::{Shared, lock};
use sbarhive_core::ActionDef;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    ASFW_ANY, AllowSetForegroundWindow, SetForegroundWindow,
};

/// Lets the running service take the foreground: this process was started
/// by Explorer and may hand the right on.
pub fn allow_foreground() {
    unsafe { AllowSetForegroundWindow(ASFW_ANY) };
}

/// Gives the shown hive window keyboard focus.
pub fn foreground(hwnd: isize) {
    let ok = unsafe { SetForegroundWindow(hwnd as _) } != 0;
    if let Some(line) = focus_log(ok) {
        eprintln!("{line}");
    }
}

/// Adds `files` to the batch in progress and gives the files of the batch
/// with the actions for all of them.
pub fn selection(state: &Shared, files: Vec<PathBuf>) -> (Vec<PathBuf>, Vec<ActionDef>) {
    static BATCH: OnceLock<Mutex<OpenBatch>> = OnceLock::new();
    static START: OnceLock<Instant> = OnceLock::new();
    let batch = BATCH.get_or_init(|| {
        let state = state.clone();
        Mutex::new(OpenBatch::new(move |files| {
            lock(&state)
                .matching(files)
                .into_iter()
                .map(|a| a.id)
                .collect()
        }))
    });
    let now = START.get_or_init(Instant::now).elapsed().as_millis() as u64;
    let mut step = None;
    for file in files {
        step = Some(lock(batch).push(now, file));
    }
    let Some(Step::Show { files, cells } | Step::Rebuild { files, cells }) = step else {
        return (Vec::new(), Vec::new());
    };
    let st = lock(state);
    let actions = cells
        .iter()
        .filter_map(|id| st.registry.get(id).cloned())
        .collect();
    (files, actions)
}
