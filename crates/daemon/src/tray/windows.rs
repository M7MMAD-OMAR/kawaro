//! The Windows tray (S-048): the data the shell thread shows, the icon file
//! and the popup menu. The window, the shortcut and `Shell_NotifyIconW` are
//! in `shell.rs`; the choices (tooltip, menu items, right to left) are in
//! `shell_logic.rs`. Left out of the mutants run, the live check is S-052.

use crate::shell_logic::{Command, Item};
use crate::state::{Shared, lock};
use sbarhive_core::i18n::Lang;
use sbarhive_core::queue::Queue;
use sbarhive_core::win::wide;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use windows_sys::Win32::Foundation::{HWND, POINT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, DestroyMenu, GetCursorPos, GetSystemMetrics, HICON, IDI_APPLICATION,
    IMAGE_ICON, InsertMenuItemW, LR_LOADFROMFILE, LoadIconW, LoadImageW, MENUITEMINFOW,
    MFS_DISABLED, MFS_ENABLED, MFT_RIGHTORDER, MFT_SEPARATOR, MFT_STRING, MIIM_FTYPE, MIIM_ID,
    MIIM_STATE, MIIM_STRING, PostMessageW, SM_CXSMICON, SetForegroundWindow, TPM_BOTTOMALIGN,
    TPM_LAYOUTRTL, TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenu, WM_APP, WM_NULL,
};

/// The notification area's callback message.
pub const WM_TRAY: u32 = WM_APP + 1;
/// Posted to the shell window when the tooltip changed.
pub const WM_REFRESH: u32 = WM_APP + 2;

struct Live {
    lang: Lang,
    active: Vec<(String, f32)>,
}

static LIVE: Mutex<Live> = Mutex::new(Live {
    lang: Lang::En,
    active: Vec::new(),
});
static HWND_SHELL: AtomicIsize = AtomicIsize::new(0);

fn live() -> MutexGuard<'static, Live> {
    lock(&LIVE)
}

/// The language of the menu and tooltip.
pub fn lang() -> Lang {
    live().lang
}

/// The running jobs and their progress.
pub fn active() -> Vec<(String, f32)> {
    live().active.clone()
}

/// The shell thread is running; the handle only proves it.
#[derive(Clone)]
pub struct Handle;

pub type TrayHandle = Option<Handle>;

pub fn start(lang: Lang, queue: Arc<Queue>, state: Shared) -> TrayHandle {
    live().lang = lang;
    match crate::shell::spawn(queue, state) {
        Ok(hwnd) => {
            HWND_SHELL.store(hwnd, Ordering::SeqCst);
            Some(Handle)
        }
        Err(e) => {
            eprintln!("sbarhived: no tray icon ({e})");
            None
        }
    }
}

fn refresh() {
    let hwnd = HWND_SHELL.load(Ordering::SeqCst);
    if hwnd != 0 {
        unsafe { PostMessageW(hwnd as _, WM_REFRESH, 0, 0) };
    }
}

pub fn set_active(tray: &TrayHandle, active: Vec<(String, f32)>) {
    if tray.is_some() {
        live().active = active;
        refresh();
    }
}

/// Switches the tray menu and tooltip to `lang` (after a settings change).
pub fn set_lang(lang: Lang) {
    live().lang = lang;
    refresh();
}

/// Removes the icon before the process ends.
pub fn stop() {
    let hwnd = HWND_SHELL.swap(0, Ordering::SeqCst);
    if hwnd != 0 {
        crate::shell::close(hwnd);
    }
}

/// The icon: `sbarhive.ico` from the folder of the executable, at the size
/// the notification area wants. The stock application icon stands in when
/// the file is missing, so the tray still shows something.
pub fn icon() -> HICON {
    let path = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("sbarhive.ico")));
    if let Some(path) = path {
        let size = unsafe { GetSystemMetrics(SM_CXSMICON) };
        let h = unsafe {
            LoadImageW(
                std::ptr::null_mut(),
                wide(&path).as_ptr(),
                IMAGE_ICON,
                size,
                size,
                LR_LOADFROMFILE,
            )
        };
        if !h.is_null() {
            return h as HICON;
        }
        eprintln!("sbarhived: cannot load {}", path.display());
    }
    unsafe { LoadIconW(std::ptr::null_mut(), IDI_APPLICATION) }
}

/// Shows the popup menu at the pointer and gives the chosen command.
/// `rtl` sets `MFT_RIGHTORDER` on each item and mirrors the popup.
pub fn track_menu(hwnd: HWND, items: &[Item], rtl: bool) -> Option<Command> {
    let order = if rtl { MFT_RIGHTORDER } else { 0 };
    let menu = unsafe { CreatePopupMenu() };
    let mut commands = Vec::new();
    for (position, item) in items.iter().enumerate() {
        let mut info: MENUITEMINFOW = unsafe { std::mem::zeroed() };
        info.cbSize = std::mem::size_of::<MENUITEMINFOW>() as u32;
        let mut label = match item {
            Item::Action { label, .. } => wide(label),
            Item::Separator => Vec::new(),
        };
        match item {
            Item::Separator => {
                info.fMask = MIIM_FTYPE;
                info.fType = MFT_SEPARATOR;
            }
            Item::Action {
                command, enabled, ..
            } => {
                commands.push(command.clone());
                info.fMask = MIIM_FTYPE | MIIM_STATE | MIIM_ID | MIIM_STRING;
                info.fType = MFT_STRING | order;
                info.fState = if *enabled { MFS_ENABLED } else { MFS_DISABLED };
                info.wID = commands.len() as u32;
                info.dwTypeData = label.as_mut_ptr();
            }
        }
        unsafe { InsertMenuItemW(menu, position as u32, 1, &info) };
    }
    let mut at = POINT { x: 0, y: 0 };
    unsafe { GetCursorPos(&mut at) };
    // The window must be in front, or the menu would not close on a click
    // elsewhere.
    unsafe { SetForegroundWindow(hwnd) };
    let flags =
        TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | if rtl { TPM_LAYOUTRTL } else { 0 };
    let chosen = unsafe { TrackPopupMenu(menu, flags, at.x, at.y, 0, hwnd, std::ptr::null()) };
    unsafe {
        PostMessageW(hwnd, WM_NULL, 0, 0);
        DestroyMenu(menu);
    }
    usize::try_from(chosen)
        .ok()
        .and_then(|id| id.checked_sub(1))
        .and_then(|i| commands.get(i).cloned())
}
