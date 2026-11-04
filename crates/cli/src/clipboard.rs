//! Files copied in a file manager arrive as text/uri-list (S-135): the
//! reader is shared with the daemon through kawaro_core::clipboard, so both
//! read the same commands and the same Windows CF_HDROP list.

pub use kawaro_core::clipboard::read_files;
