//! Tray icon. Linux shows a StatusNotifierItem (`tray/linux.rs`); Windows
//! has a stub for now (`tray/windows.rs`, the real icon is S-046). Both give
//! `start`, `set_active` and `set_lang` with the same signatures.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;
