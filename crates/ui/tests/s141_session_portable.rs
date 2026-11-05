//! S-141: the existing pure session API is available to cross-platform callers.

use kawaro_ui::session::{Session, current, detect};

// Check availability and signature without reading or mutating the environment.
const _: fn() -> Session = current;

#[test]
fn ac1_wayland_takes_precedence_over_display_and_platform() {
    for is_windows in [false, true] {
        assert_eq!(
            detect(Some("wayland-0"), Some(":0"), is_windows),
            Session::Wayland
        );
    }
}

#[test]
fn ac1_empty_wayland_falls_back_to_display() {
    for is_windows in [false, true] {
        for wayland in [None, Some("")] {
            assert_eq!(detect(wayland, Some(":0"), is_windows), Session::X11);
        }
    }
}

#[test]
fn ac1_windows_without_display_is_windows() {
    for wayland in [None, Some("")] {
        for display in [None, Some("")] {
            assert_eq!(detect(wayland, display, true), Session::Windows);
        }
    }
}

#[test]
fn ac1_non_windows_without_any_display_is_none() {
    for wayland in [None, Some("")] {
        for display in [None, Some("")] {
            assert_eq!(detect(wayland, display, false), Session::None);
        }
    }
}
