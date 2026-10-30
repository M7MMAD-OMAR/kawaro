//! Windows shell glue for `kawarod open` (S-046): the foreground grant, the
//! foreground call after the hive shows, and the merging of the files that
//! Explorer sends one process at a time. Left out of the mutants run, the
//! live check is S-052; the decisions are in `args.rs`, `batch.rs`,
//! `core::merge` and `server::focus_log`.

use crate::batch::{OpenBatch, Step as Batch};
use crate::foreground_logic::{Step, foreground_ladder};
use crate::state::{Shared, lock};
use kawaro_core::ActionDef;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    ASFW_ANY, AllowSetForegroundWindow, FLASHW_ALL, FLASHW_TIMERNOFG, FLASHWINFO, FlashWindowEx,
    GetForegroundWindow, HWND_NOTOPMOST, HWND_TOPMOST, IsIconic, SW_RESTORE, SWP_NOMOVE,
    SWP_NOSIZE, SetForegroundWindow, SetWindowPos, ShowWindow,
};

/// Lets the running service take the foreground: this process was started
/// by Explorer and may hand the right on.
pub fn allow_foreground() {
    unsafe { AllowSetForegroundWindow(ASFW_ANY) };
}

/// Gives the shown window keyboard focus. When Windows refuses (a process
/// without the foreground right, as after an update, S-106 D1), the ladder
/// tries the topmost toggle, then flashes the taskbar button.
pub fn foreground(hwnd: isize) {
    let hwnd = hwnd as _;
    foreground_ladder(|step| match step {
        Step::Plain => {
            // SAFETY: a plain call with a window handle of this process.
            // The return value can be nonzero without the window in front,
            // so the result is what the system reports after a short settle.
            unsafe { SetForegroundWindow(hwnd) };
            std::thread::sleep(Duration::from_millis(30));
            let ok = unsafe { GetForegroundWindow() == hwnd };
            if !ok {
                eprintln!("kawarod: foreground refused");
            }
            ok
        }
        Step::Topmost => {
            // SAFETY: plain calls with a window handle of this process.
            unsafe {
                if IsIconic(hwnd) != 0 {
                    ShowWindow(hwnd, SW_RESTORE);
                }
                let flags = SWP_NOMOVE | SWP_NOSIZE;
                SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, flags);
                SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, flags);
                SetForegroundWindow(hwnd);
                std::thread::sleep(Duration::from_millis(30));
                GetForegroundWindow() == hwnd
            }
        }
        Step::Flash => {
            let info = FLASHWINFO {
                cbSize: std::mem::size_of::<FLASHWINFO>() as u32,
                hwnd,
                dwFlags: FLASHW_ALL | FLASHW_TIMERNOFG,
                uCount: 3,
                dwTimeout: 0,
            };
            // SAFETY: `info` is fully set and lives for the call.
            unsafe { FlashWindowEx(&info) != 0 }
        }
    });
}

/// Whether `hwnd` is the foreground window now.
pub fn is_foreground(hwnd: isize) -> bool {
    // SAFETY: a plain query without arguments.
    unsafe { GetForegroundWindow() == hwnd as _ }
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
    let Some(Batch::Show { files, cells } | Batch::Rebuild { files, cells }) = step else {
        return (Vec::new(), Vec::new());
    };
    let st = lock(state);
    let actions = cells
        .iter()
        .filter_map(|id| st.registry.get(id).cloned())
        .collect();
    (files, actions)
}
