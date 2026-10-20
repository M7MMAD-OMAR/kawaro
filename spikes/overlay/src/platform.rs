//! A thin Slint `Platform` that forwards everything to the winit backend,
//! except `create_window_adapter` when the next window was marked as an
//! overlay: that one gets our layer-shell adapter (`overlay.rs`). The event
//! loop, timers, the settings window, X11 and accesskit stay winit's.

use i_slint_core::{InternalToken, SlintContextWeak};
use slint::PlatformError;
use slint::platform::{Clipboard, EventLoopProxy, Platform, WindowAdapter};
use std::cell::Cell;
use std::rc::Rc;

thread_local! {
    static NEXT_OVERLAY: Cell<bool> = const { Cell::new(false) };
}

/// The next component created on this thread gets an overlay adapter, if the
/// compositor offers zwlr_layer_shell_v1.
pub fn next_is_overlay() {
    NEXT_OVERLAY.with(|c| c.set(true));
}

pub struct Wrap {
    pub winit: i_slint_backend_winit::Backend,
}

impl Platform for Wrap {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        if NEXT_OVERLAY.with(|c| c.replace(false)) {
            match crate::overlay::OverlayAdapter::new() {
                Ok(a) => return Ok(a),
                Err(e) => eprintln!("overlay: unavailable ({e}), falling back to xdg-toplevel"),
            }
        }
        self.winit.create_window_adapter()
    }
    fn run_event_loop(&self) -> Result<(), PlatformError> {
        self.winit.run_event_loop()
    }
    fn process_events(
        &self,
        timeout: Option<std::time::Duration>,
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
    fn duration_since_start(&self) -> std::time::Duration {
        self.winit.duration_since_start()
    }
    fn click_interval(&self) -> std::time::Duration {
        self.winit.click_interval()
    }
    fn cursor_flash_cycle(&self) -> std::time::Duration {
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
