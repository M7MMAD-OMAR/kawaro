//! Platform-free records for the transformed image preview (S-142).
//!
//! A preview asks an engine for the result of one operation without
//! writing a file. These types carry the request, the result and the
//! failures. They hold no Slint type, no image type and no platform type,
//! so the core, the engines, the daemon and the UI can share them.
//!
//! The provider is shared between threads, so it is an `Arc`, never an
//! `Rc`: the preview runs on a worker and the UI reads the result on the
//! event loop.

use crate::{ActionDef, CancelToken};
use std::path::PathBuf;
use std::sync::Arc;

/// What the preview shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImagePreviewMode {
    /// A bounded proxy of the transformed picture, without an exact byte
    /// count and without an exact encoded result.
    TransformedDraft,
    /// The final encoded artifact, decoded back for display, with the exact
    /// length of those bytes.
    EncodedResult,
}

/// The identity of one preview request.
///
/// The session, the panel and the request counters let a surface drop a
/// result that belongs to a closed panel or to an older set of options.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreviewStamp {
    /// The session the panel belongs to.
    pub session: u64,
    /// The panel the request was made for.
    pub panel: u64,
    /// The request counter of that panel.
    pub request: u64,
}

/// One preview request: what to show, on which file, with which operation.
#[derive(Clone, Debug)]
pub struct ImagePreviewRequest {
    /// The identity of the request.
    pub stamp: PreviewStamp,
    /// The source file.
    pub input: PathBuf,
    /// The action with its options already resolved. The engine reads the
    /// operation and the parameters from it, so the preview and the run
    /// share one validated description.
    pub resolved_action: ActionDef,
    /// What the caller wants to see.
    pub mode: ImagePreviewMode,
}

/// The pixels the surface shows.
///
/// The buffer is always RGBA, four bytes per pixel, and its length is
/// `width * height * 4`. The display size and the intended output size are
/// different things: the display buffer is bounded, the output is what the
/// operation produces.
#[derive(Clone, Debug)]
pub struct DisplayPixels {
    /// The width of the buffer in pixels.
    pub width: u32,
    /// The height of the buffer in pixels.
    pub height: u32,
    /// The RGBA bytes, `width * height * 4` of them.
    pub rgba8: Vec<u8>,
}

/// The result of one preview.
#[derive(Clone, Debug)]
pub struct ImagePreviewArtifact {
    /// The stamp of the request this result belongs to.
    pub stamp: PreviewStamp,
    /// The picture to show.
    pub pixels: DisplayPixels,
    /// The width of the intended output, in pixels.
    pub output_width: u32,
    /// The height of the intended output, in pixels.
    pub output_height: u32,
    /// The format of the output, as an extension without a dot, for example
    /// `png` or `jpg`.
    pub output_format: String,
    /// The exact length of the encoded artifact, when the mode encodes one.
    /// `None` for a mode that does not encode.
    pub final_encoded_bytes: Option<u64>,
    /// The mode this result belongs to.
    pub mode: ImagePreviewMode,
}

/// Why a preview did not produce a picture.
#[derive(Debug, thiserror::Error)]
pub enum ImagePreviewError {
    /// The request was cancelled before it finished.
    #[error("the preview was cancelled")]
    Cancelled,
    /// The source file changed while it was read, so the bytes describe a
    /// file that is no longer there.
    #[error("the source file changed while it was read")]
    SourceChanged,
    /// The source or requested result exceeds preview admission limits.
    /// Normal export behavior is not changed by this preview policy.
    #[error("the image exceeds the preview policy limits")]
    PolicyLimit,
    /// The operation has no preview, or the request is not a valid one.
    #[error("{0}")]
    Unsupported(String),
    /// The preview failed.
    #[error("{0}")]
    Failed(String),
}

/// A source of preview pictures.
///
/// The engine side of the contract: the request names one file and one
/// validated action, and the result carries the pixels and the facts about
/// the output.
pub trait ImagePreviewProvider: Send + Sync {
    /// Prepares the preview of `request`. Nothing is written to disk and no
    /// job event is emitted.
    fn preview(
        &self,
        request: &ImagePreviewRequest,
        cancel: &CancelToken,
    ) -> Result<ImagePreviewArtifact, ImagePreviewError>;
}

/// A provider shared between threads.
pub type SharedPreviewProvider = Arc<dyn ImagePreviewProvider>;
