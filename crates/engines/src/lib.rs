//! Engine adapters. Each module implements [`kawaro_core::Engine`].

use kawaro_core::Engine;
use std::sync::Arc;

/// The poison ignoring locks of the engine adapters (S-145).
///
/// The values under these locks are program paths and probe results. A
/// holder that panicked is gone, so the lock is taken again and the value
/// is read, never left half written.
pub(crate) mod locks {
    use std::sync::{Mutex, MutexGuard, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

    /// Locks a mutex, ignoring poison.
    pub(crate) fn mutex<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
        m.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Locks a read lock, ignoring poison.
    pub(crate) fn read<T>(l: &RwLock<T>) -> RwLockReadGuard<'_, T> {
        l.read().unwrap_or_else(PoisonError::into_inner)
    }

    /// Locks a write lock, ignoring poison.
    pub(crate) fn write<T>(l: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
        l.write().unwrap_or_else(PoisonError::into_inner)
    }
}

pub mod ffmpeg;
pub mod image;
// S-043b: program lookup (PATH, known dirs, registry).
pub mod locate;
pub mod office;
// S-146: printed text from a picture through the system Tesseract.
pub mod ocr;
// S-150: a searchable PDF through the system OCRmyPDF.
pub mod ocrmypdf;
pub mod pdf;
// S-126: PDF to text through the system poppler `pdftotext`.
pub mod poppler;
// S-128: linearize, cleanup and check through the system `qpdf`.
pub mod qpdf;
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
        Arc::new(poppler::PopplerEngine::default()),
        Arc::new(qpdf::QpdfEngine::default()),
        Arc::new(ocr::OcrEngine::default()),
        Arc::new(ocrmypdf::OcrmypdfEngine::default()),
    ]
}
