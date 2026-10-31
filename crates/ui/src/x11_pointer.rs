//! The live pointer query on X11 (S-113): `XQueryPointer` on the root window
//! through x11rb. It needs a display, so the live check is S-114 and the
//! file is left out of the mutants run.

use crate::x11::{Pointer, PointerSource};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::ConnectionExt;

/// Asks the X server on every call.
pub struct Live;

impl PointerSource for Live {
    fn pointer(&self) -> Option<Pointer> {
        let (conn, screen) = x11rb::connect(None).ok()?;
        let root = conn.setup().roots.get(screen)?;
        let reply = conn.query_pointer(root.root).ok()?.reply().ok()?;
        Some(Pointer {
            x: i32::from(reply.root_x),
            y: i32::from(reply.root_y),
            root_w: u32::from(root.width_in_pixels),
            root_h: u32::from(root.height_in_pixels),
        })
    }
}
