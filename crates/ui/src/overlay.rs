//! The pure parts of the layer-shell overlay (S-018a, ADR 0004): which
//! surface the hive uses, where it sits and at what buffer size, the evdev
//! keys it reads, the colour scheme and the cursor shape request. No Wayland
//! types and no environment reads: the caller passes every input, so this is
//! testable without a compositor (`crates/ui/tests/overlay_pure.rs`).
//!
//! The maths follows `spikes/overlay/src/overlay.rs`.

use kawaro_core::config::HiveSurface;
use slint::platform::Key;

/// Globals the layer path needs, all at once: the overlay itself, the hive
/// subsurface over the input catcher, and a one pixel catcher scaled to the
/// whole output.
const REQUIRED: [&str; 4] = [
    "zwlr_layer_shell_v1",
    "wl_subcompositor",
    "wp_viewporter",
    "wp_single_pixel_buffer_manager_v1",
];

/// Where the hive is drawn: a layer-shell overlay or an ordinary window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    Layer,
    Window,
}

/// Chooses the hive surface from the advertised `globals`. `forced` is the
/// raw `KAWARO_SURFACE` value: `window` always wins, `layer` overrides the
/// screen reader but still needs the globals, any other value is ignored.
/// The second element is a log line for the caller to print.
pub fn choose_surface(
    globals: &[&str],
    screen_reader: bool,
    wayland: bool,
    forced: Option<&str>,
) -> (Surface, Option<String>) {
    let missing: Vec<&str> = REQUIRED
        .into_iter()
        .filter(|r| !globals.contains(r))
        .collect();
    // A forced layer without the globals falls through to the window with
    // what is missing.
    match forced {
        Some("window") => (Surface::Window, None),
        Some("layer") if missing.is_empty() => (Surface::Layer, None),
        Some("layer") => (
            Surface::Window,
            Some(format!(
                "KAWARO_SURFACE=layer but the compositor lacks {}, using a window",
                missing.join(", ")
            )),
        ),
        _ if missing.is_empty() && !screen_reader && wayland => (Surface::Layer, None),
        _ => (Surface::Window, None),
    }
}

/// A screen reader runs (S-019 AC1): the `ScreenReaderEnabled` flag and a
/// reader owning `org.gnome.Orca.Service` together, with `hive_surface` at
/// `auto`. The flag alone is often left over from a GNOME setting.
pub fn screen_reader_running(flag: bool, reader: bool, config: HiveSurface) -> bool {
    flag && reader && config == HiveSurface::Auto
}

/// The hive surface and the reason for it (S-019 AC1, AC2). `forced` is the
/// raw `KAWARO_SURFACE` and beats everything as in [`choose_surface`];
/// then `hive_surface` from config.toml; then the screen reader rule.
pub fn surface_choice(
    globals: &[&str],
    wayland: bool,
    flag: bool,
    reader: bool,
    config: HiveSurface,
    forced: Option<&str>,
) -> (Surface, String) {
    let missing: Vec<&str> = REQUIRED
        .into_iter()
        .filter(|r| !globals.contains(r))
        .collect();
    let lacking = || format!("missing {}", missing.join(", "));
    let window = |why: &str| (Surface::Window, why.to_string());
    let layer = |why: &str| (Surface::Layer, why.to_string());
    // A forced layer without the globals falls through to the window with
    // what is missing.
    match forced {
        Some("window") => return window("KAWARO_SURFACE window"),
        Some("layer") if missing.is_empty() => return layer("KAWARO_SURFACE layer"),
        _ => {}
    }
    if config == HiveSurface::Window {
        return window("config window");
    }
    if !wayland {
        return window("not a Wayland session");
    }
    if !missing.is_empty() {
        return window(&lacking());
    }
    if config == HiveSurface::Layer {
        return layer("config layer");
    }
    if screen_reader_running(flag, reader, config) {
        return window("screen reader running");
    }
    if flag {
        return layer("screen reader flag set, no reader running");
    }
    layer("no screen reader")
}

/// The line printed on every change of the choice, with no environment
/// variable: `hive surface choice: KIND, REASON`. Not `hive surface: `,
/// which stays the S-018 AC11 geometry line.
pub fn choice_line(surface: Surface, reason: &str) -> String {
    let kind = match surface {
        Surface::Layer => "layer",
        Surface::Window => "window",
    };
    format!("hive surface choice: {kind}, {reason}")
}

/// The hive on the overlay, in logical pixels except `buffer`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    /// Subsurface position on the output.
    pub x: i32,
    pub y: i32,
    /// Hive buffer size in physical pixels.
    pub buffer: (u32, u32),
    /// Viewport destination of the hive buffer.
    pub destination: (u32, u32),
}

/// Centres a `cell` sized hive on a `screen` sized output at scale
/// `scale120`/120 (the fractional-scale-v1 unit).
pub fn place(screen: (u32, u32), cell: (u32, u32), scale120: u32) -> Placement {
    let centre = |s: u32, c: u32| ((f64::from(s) - f64::from(c)) / 2.0).round() as i32;
    let physical = |c: u32| (u64::from(c) * u64::from(scale120)).div_ceil(120) as u32;
    Placement {
        x: centre(screen.0, cell.0),
        y: centre(screen.1, cell.1),
        buffer: (physical(cell.0), physical(cell.1)),
        destination: cell,
    }
}

/// The layout independent keys the hive uses, from evdev codes.
pub fn key_for_evdev(code: u32) -> Option<Key> {
    Some(match code {
        1 => Key::Escape,
        28 | 96 => Key::Return,
        57 => Key::Space,
        15 => Key::Tab,
        103 => Key::UpArrow,
        108 => Key::DownArrow,
        105 => Key::LeftArrow,
        106 => Key::RightArrow,
        _ => return None,
    })
}

/// Dark or light: `env` is the raw `KAWARO_COLOR_SCHEME` and wins when it
/// is `light` or `dark`; otherwise the portal `color-scheme` value, where 1
/// is dark and 0, 2, absent or an error are light.
pub fn scheme_is_dark(env: Option<&str>, portal: Option<u32>) -> bool {
    crate::forced_dark(env).unwrap_or(portal == Some(1))
}

/// Whether to ask the compositor for cursor shapes.
pub fn wants_cursor_shape(globals: &[&str]) -> bool {
    globals.contains(&"wp_cursor_shape_manager_v1")
}
