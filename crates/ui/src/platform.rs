//! A thin Slint `Platform` over the winit backend (S-018b, ADR 0004). It
//! forwards everything to winit, except the window adapter of a hive build
//! when the overlay decision says `Layer`: that one draws on a layer-shell
//! overlay (`layer.rs`). The event loop, timers, the settings window and
//! accesskit stay winit's.

use i_slint_core::{InternalToken, SlintContextWeak};
use slint::PlatformError;
use slint::platform::{Clipboard, EventLoopProxy, Platform, WindowAdapter};
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

thread_local! {
    /// A hive component is being created on this thread.
    static HIVE_BUILD: Cell<bool> = const { Cell::new(false) };
    /// The last hive build got the overlay adapter.
    static LAST_LAYER: Cell<bool> = const { Cell::new(false) };
}

/// Whether the last hive build got the overlay adapter.
pub(crate) fn last_hive_on_layer() -> bool {
    LAST_LAYER.with(Cell::get)
}

/// Marks the window adapters created until `building_hive(false)` as the
/// hive's.
pub(crate) fn building_hive(on: bool) {
    HIVE_BUILD.with(|c| c.set(on));
}

/// The window being made is the hive and the session is X11 (S-113).
pub(crate) fn hive_window_on_x11() -> bool {
    HIVE_BUILD.with(Cell::get) && crate::session::current() == crate::session::Session::X11
}

pub(crate) struct Wrap {
    pub winit: i_slint_backend_winit::Backend,
}

impl Platform for Wrap {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        if HIVE_BUILD.with(Cell::get) {
            let layer = crate::layer::wanted();
            LAST_LAYER.with(|c| c.set(layer));
            if layer {
                return Ok(crate::layer::LayerAdapter::create());
            }
        }
        self.winit.create_window_adapter()
    }
    fn run_event_loop(&self) -> Result<(), PlatformError> {
        self.winit.run_event_loop()
    }
    fn process_events(
        &self,
        timeout: Option<Duration>,
        t: InternalToken,
    ) -> Result<core::ops::ControlFlow<()>, PlatformError> {
        self.winit.process_events(timeout, t)
    }
    fn bind_context(&self, ctx: SlintContextWeak, t: InternalToken) {
        self.winit.bind_context(ctx, t)
    }
    #[allow(deprecated)]
    fn set_event_loop_quit_on_last_window_closed(&self, q: bool) {
        self.winit.set_event_loop_quit_on_last_window_closed(q)
    }
    fn new_event_loop_proxy(&self) -> Option<Box<dyn EventLoopProxy>> {
        self.winit.new_event_loop_proxy()
    }
    fn duration_since_start(&self) -> Duration {
        self.winit.duration_since_start()
    }
    fn click_interval(&self) -> Duration {
        self.winit.click_interval()
    }
    fn cursor_flash_cycle(&self) -> Duration {
        self.winit.cursor_flash_cycle()
    }
    fn set_clipboard_text(&self, text: &str, c: Clipboard) {
        self.winit.set_clipboard_text(text, c)
    }
    fn clipboard_text(&self, c: Clipboard) -> Option<String> {
        self.winit.clipboard_text(c)
    }
    fn debug_log(&self, a: core::fmt::Arguments) {
        self.winit.debug_log(a)
    }
    fn open_url(&self, url: &str) -> Result<(), PlatformError> {
        self.winit.open_url(url)
    }
}
