//! SbarHive core. Everything the daemon, the CLI and the engines share.
//!
//! The contracts in [`engine`], [`action`] and [`ipc`] are the boundary
//! between crates. Change them only with an ADR.

pub mod action;
mod action_impl;
pub mod config;
pub mod engine;
pub mod hold;
pub mod hotkey;
pub mod i18n;
pub mod ipc;
pub mod media;
pub mod merge;
pub mod output;
mod output_impl;
pub mod paths;
pub mod queue;
mod queue_impl;
pub mod registry;
mod sync;
pub mod transport;
pub mod uri_list;
#[cfg(windows)]
#[doc(hidden)]
pub mod win;

pub use action::{ActionDef, OutputMode, Registry};
pub use engine::{CancelToken, Engine, EngineError, Progress, Task};
pub use media::MediaKind;
