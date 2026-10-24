//! Tray icon. Linux shows a StatusNotifierItem (`tray/linux.rs`); Windows
//! has the shell thread's tray (`tray/windows.rs`, `shell.rs`, S-048). Both give
//! `start`, `set_active` and `set_lang` with the same signatures.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
pub use windows::*;
