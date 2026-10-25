//! Engine adapters. Each module implements [`kawaro_core::Engine`].

use kawaro_core::Engine;
use std::sync::Arc;

pub mod ffmpeg;
pub mod image;
// S-043b: program lookup (PATH, known dirs, registry).
mod locate;
pub mod office;
pub mod pdf;
// S-030: one killable process tree per engine child.
pub(crate) mod process;

/// Every engine compiled into this build, available or not.
pub fn all() -> Vec<Arc<dyn Engine>> {
    vec![
        Arc::new(image::ImageEngine::default()),
        Arc::new(ffmpeg::FfmpegEngine::default()),
        Arc::new(office::OfficeEngine::default()),
        Arc::new(pdf::PdfEngine),
        Arc::new(pdf::GhostscriptEngine::default()),
    ]
}
