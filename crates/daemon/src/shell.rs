//! The Windows shell thread (S-048, ADR 0005 section 5): one hidden
//! top-level window (a message-only window gets no broadcasts) that owns
//! the tray icon, the global shortcut and the tray menu. All of it runs on
//! this thread's message loop; no COM is used here, so nothing initialises
//! COM on the main thread. Left out of the mutants run, the live check is
//! S-052; the decisions are in `shell_logic.rs` and `core::hotkey`.

use crate::shell_logic::{
    Command, HoldPoll, POLL_MS, Poll, fit_tip, menu_right_to_left, shortcut_error_line, tooltip,
};
use crate::state::{Shared, lock};
use crate::tray::windows::{self as tray, WM_REFRESH};
use kawaro_core::hotkey::{self, Hotkey};
use kawaro_core::ipc::Request;
use kawaro_core::queue::Queue;
use kawaro_core::{hold, win::wide};
use std::cell::{OnceCell, RefCell};
use std::sync::Arc;
use std::time::Instant;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey,
};
use windows_sys::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
    Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyWindow, DispatchMessageW, GetMessageW,
    HICON, KillTimer, MSG, PostQuitMessage, RegisterClassExW, RegisterWindowMessageW, SetTimer,
    TranslateMessage, WM_CLOSE, WM_DESTROY, WM_HOTKEY, WM_RBUTTONUP, WM_TIMER, WNDCLASSEXW,
};

const HOTKEY_ID: i32 = 1;
const TIMER_ID: usize = 1;

/// What the window procedure needs; set once, before the first message.
struct Ctx {
    queue: Arc<Queue>,
    state: Shared,
    taskbar_created: u32,
    vk: i32,
}

thread_local! {
    static CTX: OnceCell<Ctx> = const { OnceCell::new() };
    /// The tray icon, loaded once: handle and whether we must destroy it.
    static ICON: OnceCell<(HICON, bool)> = const { OnceCell::new() };
    static HOLD: RefCell<HoldPoll> = RefCell::new(HoldPoll::default());
}

/// Starts the shell thread and gives back the window handle once it exists.
pub fn spawn(queue: Arc<Queue>, state: Shared) -> Result<isize, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("shell".into())
        .spawn(move || run(queue, state, tx))
        .map_err(|e| e.to_string())?;
    rx.recv().map_err(|e| e.to_string())?
}

fn run(queue: Arc<Queue>, state: Shared, ready: std::sync::mpsc::Sender<Result<isize, String>>) {
    let shortcut = lock(&state).config.shortcut.clone();
    unsafe {
        let hinst = GetModuleHandleW(std::ptr::null());
        let class = wide("KawaroShell");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(window_proc),
            hInstance: hinst,
            lpszClassName: class.as_ptr(),
            ..std::mem::zeroed()
        };
        if RegisterClassExW(&wc) == 0 {
            let _ = ready.send(Err("cannot register the shell window class".into()));
            return;
        }
        let taskbar_created = RegisterWindowMessageW(wide("TaskbarCreated").as_ptr());
        let parsed = hotkey::parse(&shortcut);
        let vk = parsed.as_ref().map_or(0, |h| h.vk as i32);
        let _ = CTX.with(|c| {
            c.set(Ctx {
                queue,
                state,
                taskbar_created,
                vk,
            })
        });
        // Hidden and never shown: a top-level window, so it gets the
        // TaskbarCreated broadcast after Explorer restarts.
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            wide("Kawaro").as_ptr(),
            0,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            hinst,
            std::ptr::null(),
        );
        if hwnd.is_null() {
            let _ = ready.send(Err("cannot create the shell window".into()));
            return;
        }
        notify_icon(hwnd, NIM_ADD);
        register_shortcut(hwnd, &shortcut, parsed);
        let _ = ready.send(Ok(hwnd as isize));
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

/// Registers the shortcut with MOD_NOREPEAT. A string that does not parse or
/// a combination another program holds is logged once; the daemon runs on.
unsafe fn register_shortcut(hwnd: HWND, shortcut: &str, parsed: Result<Hotkey, String>) {
    let ok = match parsed {
        Ok(h) => unsafe { RegisterHotKey(hwnd, HOTKEY_ID, MOD_NOREPEAT | h.mods, h.vk) != 0 },
        Err(e) => {
            eprintln!("kawarod: shortcut {shortcut}: {e}");
            false
        }
    };
    if let Some(line) = shortcut_error_line(shortcut, ok) {
        eprintln!("{line}");
    }
}

/// Removes the tray icon from outside the shell thread, before the process
/// ends (the system keeps a dead icon until the pointer passes over it).
pub fn close(hwnd: isize) {
    use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;
    unsafe { SendMessageW(hwnd as _, WM_CLOSE, 0, 0) };
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let Some((taskbar_created, vk)) = CTX.with(|c| c.get().map(|c| (c.taskbar_created, c.vk)))
    else {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    };
    if msg == taskbar_created {
        // Explorer restarted: its notification area is empty again.
        notify_icon(hwnd, NIM_ADD);
        return 0;
    }
    match msg {
        WM_REFRESH => notify_icon(hwnd, NIM_MODIFY),
        tray::WM_TRAY if lparam as u32 == WM_RBUTTONUP => show_menu(hwnd),
        // The S-021 rule (S-047 AC6): the shortcut shows no hive without a drag.
        WM_HOTKEY if !crate::server::shortcut_drop_allowed() => {}
        WM_HOTKEY => {
            // The same as `kawaro drop --hold`: the long press gate shows
            // the hive after hold_ms; the release comes from the poll below.
            let press = Request::Drop {
                token: None,
                at: None,
                hold: true,
            };
            signal(&press);
            HOLD.with(|h| h.borrow_mut().press());
            unsafe { SetTimer(hwnd, TIMER_ID, POLL_MS as u32, None) };
        }
        WM_TIMER if wparam == TIMER_ID => {
            let down = unsafe { GetAsyncKeyState(vk) } as u16 & 0x8000 != 0;
            match HOLD.with(|h| h.borrow_mut().tick(down)) {
                Poll::Held => {}
                Poll::Released => {
                    unsafe { KillTimer(hwnd, TIMER_ID) };
                    signal(&Request::DropRelease);
                }
                Poll::Idle => {
                    unsafe { KillTimer(hwnd, TIMER_ID) };
                }
            }
        }
        WM_CLOSE => {
            unsafe { DestroyWindow(hwnd) };
        }
        WM_DESTROY => {
            notify_icon(hwnd, NIM_DELETE);
            if let Some(&(icon, true)) = ICON.with(|i| i.get().copied()).as_ref() {
                unsafe { DestroyIcon(icon) };
            }
            unsafe {
                UnregisterHotKey(hwnd, HOTKEY_ID);
                PostQuitMessage(0);
            }
        }
        _ => return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
    0
}

/// Feeds a request to the long press gate, as the socket does.
fn signal(req: &Request) {
    let state = CTX.with(|c| c.get().map(|c| c.state.clone()));
    if let (Some(s), Some(state)) = (hold::from_request(req), state) {
        crate::ui::hold_signal(s, None, Instant::now(), state);
    }
}

/// The right click menu, right to left for Arabic.
fn show_menu(hwnd: HWND) {
    let Some((queue, state)) = CTX.with(|c| c.get().map(|c| (c.queue.clone(), c.state.clone())))
    else {
        return;
    };
    let lang = tray::lang();
    let update_ready = crate::update_sys::update_ready();
    let items = crate::shell_logic::menu(lang, tray::active().len(), update_ready);
    let rtl = menu_right_to_left(lang);
    match tray::track_menu(hwnd, &items, rtl) {
        Some(Command::UpdateNow) => crate::update_sys::apply_from_menu(state.clone()),
        Some(Command::CancelAll) => queue.cancel_all(),
        Some(Command::Settings) => {
            let config = lock(&state).config.clone();
            crate::ui::open_settings(config);
        }
        Some(Command::Quit) => crate::ui::quit(),
        None => {}
    }
}

/// The tooltip as the fixed 128 unit array of the notification data.
pub fn tip_units() -> [u16; 128] {
    let mut tip = [0u16; 128];
    let (active, lang) = (tray::active(), tray::lang());
    // A running download shows its percentage (S-054b), read without the
    // update lock.
    let text = match crate::progress::shared().snapshot(&crate::update_sys::part_len) {
        Some(phase) => fit_tip(crate::text::tooltip_from(lang, None, Some(&phase), &active)),
        None => tooltip(&active, lang),
    };
    for (slot, u) in tip.iter_mut().zip(text.encode_utf16().take(127)) {
        *slot = u;
    }
    tip
}

/// Fills the parts of the notification data every call shares.
pub fn icon_data(hwnd: HWND) -> NOTIFYICONDATAW {
    let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = tray::WM_TRAY;
    nid.hIcon = ICON.with(|i| i.get_or_init(tray::icon).0);
    nid.szTip = tip_units();
    nid
}

/// `Shell_NotifyIconW` with the current icon and tooltip.
pub fn notify_icon(hwnd: HWND, action: u32) {
    let nid = icon_data(hwnd);
    if unsafe { Shell_NotifyIconW(action, &nid) } == 0 && action == NIM_ADD {
        eprintln!("kawarod: no tray icon");
    }
}
