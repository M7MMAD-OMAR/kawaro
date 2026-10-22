//! Engine adapters. Each module implements [`sbarhive_core::Engine`].

use sbarhive_core::Engine;
use std::sync::Arc;

pub mod ffmpeg;
pub mod image;
// S-025b: program lookup (PATH, known dirs, registry).
mod locate;
pub mod office;
pub mod pdf;

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
