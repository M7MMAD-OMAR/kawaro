//! Engine adapters. Each module implements [`sbarhive_core::Engine`].

use sbarhive_core::Engine;
use std::sync::Arc;

pub mod image;

/// Every engine compiled into this build, available or not.
pub fn all() -> Vec<Arc<dyn Engine>> {
    vec![Arc::new(image::ImageEngine::default())]
}
