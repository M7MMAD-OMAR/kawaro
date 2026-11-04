//! Preview worker for the stage 2 form (S-118, docs/research
//! "image-editing-and-preview.md" section 6).
//!
//! The hive cannot rotate, scale or clip a picture with the software
//! renderer, so the worker hands Slint a ready RGBA buffer, already scaled to
//! the well and masked to the pointy top hexagon. Decoding is bounded: the
//! dimensions come from the header first, a JPEG is decoded with the DCT
//! scale the draft needs, a PNG is read row by row, an SVG is rendered at the
//! target and the other formats decode their first frame.
//!
//! Nothing here runs on its own: there is no thread, timer or channel in this
//! module until S-119 starts one after the first hive frame. Every failure
//! (a corrupt file, an unsupported layout, a size over the cap) returns None,
//! so a preview can never fail a job or block the UI.

use crate::preview;
use slint::{Rgba8Pixel, SharedPixelBuffer};
use std::io::Read;
use std::path::Path;

/// Longest side of one draft, in pixels (the memory cap of AC4).
pub const MAX_DRAFT_SIDE: u32 = 1000;
/// Source pixels above which only the dimensions are shown: a 100 MP file
/// never decodes for a preview.
pub const MAX_DRAFT_PIXELS: u64 = 40_000_000;

const SQRT_3: f32 = 1.732_050_8;

/// The formats the worker treats specially.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Jpeg,
    Png,
    Svg,
    /// GIF, WebP, BMP, TIFF and anything else the image crate reads.
    Other,
}

fn kind(path: &Path) -> Option<Kind> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut head = [0u8; 16];
    let n = file.read(&mut head).ok()?;
    let head = &head[..n];
    if head.starts_with(&[0xFF, 0xD8]) {
        return Some(Kind::Jpeg);
    }
    if head.starts_with(&[0x89, b'P', b'N', b'G']) {
        return Some(Kind::Png);
    }
    let svg_ext = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("svg"));
    if svg_ext || head.starts_with(b"<") {
        return Some(Kind::Svg);
    }
    Some(Kind::Other)
}

/// The dimensions from the header only, with no pixels decoded.
pub fn probe_dimensions(path: &Path) -> Option<(u32, u32)> {
    let (w, h) = match kind(path)? {
        Kind::Jpeg => {
            let file = std::fs::File::open(path).ok()?;
            let mut decoder = jpeg_decoder::Decoder::new(std::io::BufReader::new(file));
            decoder.read_info().ok()?;
            let info = decoder.info()?;
            (u32::from(info.width), u32::from(info.height))
        }
        Kind::Png => {
            let file = std::fs::File::open(path).ok()?;
            let reader = png::Decoder::new(std::io::BufReader::new(file))
                .read_info()
                .ok()?;
            (reader.info().width, reader.info().height)
        }
        Kind::Svg => {
            let tree = svg_tree(path)?;
            let size = tree.size().to_int_size();
            (size.width(), size.height())
        }
        Kind::Other => image::ImageReader::open(path)
            .ok()?
            .with_guessed_format()
            .ok()?
            .into_dimensions()
            .ok()?,
    };
    (w > 0 && h > 0).then_some((w, h))
}

/// A draft no larger than `max_side` on its longest side, capped at
/// `MAX_DRAFT_SIDE`. None when the file cannot be read, when the source is
/// over `MAX_DRAFT_PIXELS`, or when `max_side` is zero.
pub fn decode_draft(path: &Path, max_side: u32) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
    let max = max_side.min(MAX_DRAFT_SIDE);
    if max == 0 {
        return None;
    }
    let (w, h) = probe_dimensions(path)?;
    if u64::from(w) * u64::from(h) > MAX_DRAFT_PIXELS {
        return None;
    }
    let (tw, th) = preview::fit(w, h, max);
    if tw == 0 || th == 0 {
        return None;
    }
    match kind(path)? {
        Kind::Jpeg => jpeg_draft(path, tw, th),
        // The PNG path is the row streaming of the centre cell: one output
        // row is built at a time, the full size image is never held.
        Kind::Png => preview::decode_fit(path, max),
        Kind::Svg => svg_draft(path, tw, th),
        Kind::Other => other_draft(path, tw, th),
    }
}

/// A JPEG decoded at the DCT scale the draft needs, never at full size when a
/// scale is possible.
fn jpeg_draft(path: &Path, tw: u32, th: u32) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
    let file = std::fs::File::open(path).ok()?;
    let mut decoder = jpeg_decoder::Decoder::new(std::io::BufReader::new(file));
    let scale = |v: u32| v.min(u32::from(u16::MAX)) as u16;
    decoder.scale(scale(tw), scale(th)).ok()?;
    let pixels = decoder.decode().ok()?;
    let info = decoder.info()?;
    let (dw, dh) = (u32::from(info.width), u32::from(info.height));
    if dw == 0 || dh == 0 {
        return None;
    }
    let image = match info.pixel_format {
        jpeg_decoder::PixelFormat::L8 => {
            image::GrayImage::from_raw(dw, dh, pixels).map(image::DynamicImage::ImageLuma8)
        }
        jpeg_decoder::PixelFormat::L16 => {
            // Big endian pairs: the high byte is enough for a draft.
            let luma: Vec<u8> = (0..pixels.len() / 2).map(|i| pixels[i * 2]).collect();
            image::GrayImage::from_raw(dw, dh, luma).map(image::DynamicImage::ImageLuma8)
        }
        jpeg_decoder::PixelFormat::RGB24 => {
            image::RgbImage::from_raw(dw, dh, pixels).map(image::DynamicImage::ImageRgb8)
        }
        // Four channel JPEGs are CMYK or YCCK, which the crate returns
        // unconverted; a draft of them is not worth a wrong colour.
        jpeg_decoder::PixelFormat::CMYK32 => return None,
    }?;
    // The draft is a fast downscale of an already scaled decode, not the
    // output file: Triangle keeps a slider drag inside its frame budget, and
    // the engine still writes the result with Lanczos3.
    let rgba = image
        .resize_exact(tw, th, image::imageops::FilterType::Triangle)
        .to_rgba8();
    Some(to_buffer(rgba))
}

/// An SVG rendered at the target size with resvg.
fn svg_draft(path: &Path, tw: u32, th: u32) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
    let tree = svg_tree(path)?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(tw, th)?;
    let sx = tw as f32 / tree.size().width();
    let sy = th as f32 / tree.size().height();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(sx, sy),
        &mut pixmap.as_mut(),
    );
    // tiny-skia keeps premultiplied alpha; Slint wants straight RGBA.
    let data = pixmap.data();
    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(tw, th);
    let pixels = buffer.make_mut_slice();
    for (i, dst) in pixels.iter_mut().enumerate() {
        let px = &data[i * 4..i * 4 + 4];
        let alpha = u16::from(px[3]);
        let straight = |c: u8| {
            (u16::from(c) * 255 + alpha / 2)
                .checked_div(alpha)
                .unwrap_or(0)
                .min(255) as u8
        };
        *dst = Rgba8Pixel::new(straight(px[0]), straight(px[1]), straight(px[2]), px[3]);
    }
    Some(buffer)
}

/// GIF, WebP, BMP, TIFF and anything else: the image crate decodes the first
/// frame, then the draft is scaled down.
fn other_draft(path: &Path, tw: u32, th: u32) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
    let image = image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()?;
    let rgba = image
        .resize_exact(tw, th, image::imageops::FilterType::Triangle)
        .to_rgba8();
    Some(to_buffer(rgba))
}

fn svg_tree(path: &Path) -> Option<resvg::usvg::Tree> {
    let data = std::fs::read(path).ok()?;
    resvg::usvg::Tree::from_data(&data, &resvg::usvg::Options::default()).ok()
}

fn to_buffer(image: image::RgbaImage) -> SharedPixelBuffer<Rgba8Pixel> {
    let (w, h) = image.dimensions();
    let raw = image.into_raw();
    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(w, h);
    let pixels = buffer.make_mut_slice();
    for (i, dst) in pixels.iter_mut().enumerate() {
        let px = &raw[i * 4..i * 4 + 4];
        *dst = Rgba8Pixel::new(px[0], px[1], px[2], px[3]);
    }
    buffer
}

/// Sets alpha to zero outside the pointy top hexagon of `circumradius`,
/// centred in the buffer, and antialiases the edge over one pixel. None for an
/// empty buffer or a radius that is zero or not finite.
pub fn mask_hexagon(buffer: &mut SharedPixelBuffer<Rgba8Pixel>, circumradius: f32) -> Option<()> {
    let (w, h) = (buffer.width(), buffer.height());
    if w == 0 || h == 0 || !circumradius.is_finite() || circumradius <= 0.0 {
        return None;
    }
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    let r = circumradius;
    let pixels = buffer.make_mut_slice();
    for y in 0..h {
        for x in 0..w {
            let ax = (x as f32 + 0.5 - cx).abs();
            let ay = (y as f32 + 0.5 - cy).abs();
            // Signed distance to the two half plane families of a pointy top
            // hexagon: the flat sides are at |x| = sqrt(3)/2 r and the slanted
            // sides at |x|/sqrt(3) + |y| = r.
            let flat = ax - (SQRT_3 / 2.0) * r;
            let slanted = (ax / SQRT_3 + ay - r) * (SQRT_3 / 2.0);
            let distance = flat.max(slanted);
            let coverage = (0.5 - distance).clamp(0.0, 1.0);
            let pixel = &mut pixels[(y * w + x) as usize];
            pixel.a = (f32::from(pixel.a) * coverage).round() as u8;
        }
    }
    Some(())
}

/// What the worker should do next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    /// Render this generation now.
    Render(u64),
    /// Nothing to render until the next submit.
    Idle,
}

/// The newest-wins state machine of one preview worker (S-118 AC5): one
/// render in flight, at most one generation waiting, and a finished render is
/// published only while it is still the newest.
#[derive(Clone, Debug, Default)]
pub struct Generation {
    next: u64,
    wanted: Option<u64>,
    running: Option<u64>,
    published: Option<u64>,
}

impl Generation {
    pub fn new() -> Self {
        Self::default()
    }

    /// The generation being rendered now, if any.
    pub fn running(&self) -> Option<u64> {
        self.running
    }

    /// The generation waiting for the render in flight, if any.
    pub fn wanted(&self) -> Option<u64> {
        self.wanted
    }

    /// The last published generation, if any.
    pub fn published(&self) -> Option<u64> {
        self.published
    }

    /// A value changed: it becomes the newest generation, and it starts only
    /// when nothing is in flight.
    pub fn submit(&mut self) -> Next {
        let id = self.next;
        self.next += 1;
        self.wanted = Some(id);
        self.start()
    }

    /// The render in flight finished. Returns the generation that may be
    /// published (only when it is still the newest) and what to render next.
    pub fn finish(&mut self) -> (Option<u64>, Next) {
        let Some(done) = self.running.take() else {
            return (None, self.start());
        };
        let published = self.wanted.is_none().then_some(done);
        if published.is_some() {
            self.published = Some(done);
        }
        (published, self.start())
    }

    fn start(&mut self) -> Next {
        if self.running.is_some() {
            return Next::Idle;
        }
        match self.wanted.take() {
            Some(id) => {
                self.running = Some(id);
                Next::Render(id)
            }
            None => Next::Idle,
        }
    }
}
