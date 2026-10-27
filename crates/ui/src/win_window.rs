//! The hive window on Windows (ADR 0005 sections 5 and 6, S-046): an
//! undecorated, always on top winit window with no taskbar button and no
//! system shadow, square corners, placed at the pointer inside the work area
//! of its monitor. The decisions are pure (`monitor.rs`, `layout::plan_at`);
//! this file only calls Win32. Left out of the mutants run, the live check
//! is S-052.

use crate::HiveCell;
use crate::layout::{self, Plan};
use crate::monitor::{Rect, work_area_for};
use slint::winit_030::{WinitWindowAccessor, winit};
use std::cell::Cell;
use std::ffi::c_void;
use windows_sys::Win32::Foundation::{LPARAM, POINT, RECT};
use windows_sys::Win32::Graphics::Dwm as dwm;
use windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute;
use windows_sys::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
};
use windows_sys::Win32::UI::HiDpi::GetDpiForSystem;
use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

/// `DWMWA_WINDOW_CORNER_PREFERENCE`, the window attribute for the corners.
const DWMWA_WINDOW_CORNER_PREFERENCE: i32 = 33;
/// `DWMWCP_DONOTROUND`: square corners, so the hive's own shape shows.
const DWMWCP_DONOTROUND: i32 = 1;

const _: () = assert!(DWMWCP_DONOTROUND == 1);
const _: () = assert!(DWMWCP_DONOTROUND == dwm::DWMWCP_DONOTROUND);
const _: () = assert!(DWMWA_WINDOW_CORNER_PREFERENCE == dwm::DWMWA_WINDOW_CORNER_PREFERENCE);

thread_local! {
    /// The next window to be created is the hive: the attributes hook runs
    /// for the settings window too, which keeps its frame.
    static BUILDING_HIVE: Cell<bool> = const { Cell::new(false) };
}

pub fn set_building(on: bool) {
    BUILDING_HIVE.with(|c| c.set(on));
}

/// The winit attributes of the hive window.
pub fn attributes(attrs: winit::window::WindowAttributes) -> winit::window::WindowAttributes {
    use winit::platform::windows::WindowAttributesExtWindows;
    use winit::window::WindowLevel;
    let attrs = match app_icon() {
        Some(icon) => attrs.with_window_icon(Some(icon)),
        None => attrs,
    };
    if !BUILDING_HIVE.with(Cell::get) {
        return attrs;
    }
    attrs
        .with_transparent(true)
        .with_decorations(false)
        .with_undecorated_shadow(false)
        .with_skip_taskbar(true)
        .with_window_level(WindowLevel::AlwaysOnTop)
}

/// Decode the bundled brand image for native window and taskbar icons.
fn app_icon() -> Option<winit::window::Icon> {
    let decoder = png::Decoder::new(std::io::Cursor::new(include_bytes!(
        "../../../site/apple-touch-icon.png"
    )));
    let mut reader = decoder.read_info().ok()?;
    let mut pixels = vec![0; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut pixels).ok()?;
    pixels.truncate(frame.buffer_size());
    if frame.color_type != png::ColorType::Rgba || frame.bit_depth != png::BitDepth::Eight {
        return None;
    }
    winit::window::Icon::from_rgba(pixels, frame.width, frame.height).ok()
}

/// The work areas of all monitors, in virtual screen pixels.
fn work_areas() -> Vec<Rect> {
    unsafe extern "system" fn each(
        monitor: HMONITOR,
        _dc: HDC,
        _rect: *mut RECT,
        data: LPARAM,
    ) -> i32 {
        let areas = unsafe { &mut *(data as *mut Vec<Rect>) };
        let mut info: MONITORINFO = unsafe { std::mem::zeroed() };
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if unsafe { GetMonitorInfoW(monitor, &mut info) } != 0 {
            let r = info.rcWork;
            areas.push(Rect {
                x: r.left,
                y: r.top,
                w: r.right - r.left,
                h: r.bottom - r.top,
            });
        }
        1
    }
    let mut areas: Vec<Rect> = Vec::new();
    unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(each),
            &mut areas as *mut Vec<Rect> as LPARAM,
        );
    }
    areas
}

/// The pointer position and the work area it is in.
fn pointer_and_area() -> ((i32, i32), Rect) {
    let mut p = POINT { x: 0, y: 0 };
    let cursor = if unsafe { GetCursorPos(&mut p) } != 0 {
        (p.x, p.y)
    } else {
        (0, 0)
    };
    (cursor, work_area_for(cursor, &work_areas()))
}

/// The hive plan for the pointer, and the physical position of its window.
pub fn plan(cells: Vec<HiveCell>, rtl: bool, min_rings: i32) -> (Plan, slint::PhysicalPosition) {
    let (cursor, area) = pointer_and_area();
    let scale = (unsafe { GetDpiForSystem() }.max(96)) as f32 / 96.0;
    let screen = (
        (area.w as f32 / scale).round() as u32,
        (area.h as f32 / scale).round() as u32,
    );
    let anchor = (
        (cursor.0 - area.x) as f32 / scale,
        (cursor.1 - area.y) as f32 / scale,
    );
    let (plan, place) = layout::plan_at(
        cells,
        rtl,
        min_rings,
        screen,
        Some(anchor),
        (scale * 120.0).round() as u32,
    );
    let at = slint::PhysicalPosition::new(
        area.x + (place.x as f32 * scale).round() as i32,
        area.y + (place.y as f32 * scale).round() as i32,
    );
    (plan, at)
}

/// The window handle, for `SetForegroundWindow`.
pub fn hwnd(window: &slint::Window) -> Option<isize> {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    window
        .with_winit_window(|w| match w.window_handle().map(|h| h.as_raw()) {
            Ok(RawWindowHandle::Win32(h)) => Some(h.hwnd.get()),
            _ => None,
        })
        .flatten()
}

/// Asks the system for square corners on the shown window.
pub fn square_corners(window: &slint::Window) {
    let Some(hwnd) = hwnd(window) else { return };
    let value: i32 = DWMWCP_DONOTROUND;
    unsafe {
        DwmSetWindowAttribute(
            hwnd as *mut c_void,
            DWMWA_WINDOW_CORNER_PREFERENCE as _,
            &value as *const i32 as *const c_void,
            std::mem::size_of::<i32>() as u32,
        );
    }
}
