//! `to-pdf`: one page per input. `page_size`, `orientation`, `margin_mm`
//! and `dpi` set the page box and where the image sits on it: `fit` (the
//! default) is the image at its dpi size plus the margin on every side, and
//! A4 or Letter scale the image into the margin inset box. Upright 8 bit gray
//! or RGB JPEGs are embedded as they are (DCTDecode); everything else is
//! stored losslessly with Flate, alpha composited onto white.

use super::{Loaded, bad, encode::flatten, load};
use image::{ColorType, ImageFormat};
use kawaro_core::{CancelToken, EngineError, Progress, Task};
use miniz_oxide::deflate::compress_to_vec_zlib;
use pdf_writer::{Content, Filter, Finish, Name, Pdf, Rect, Ref};

/// zlib level for Flate image streams.
const FLATE_LEVEL: u8 = 6;
const IMAGE_NAME: Name<'static> = Name(b"Im1");

/// Fixed page sizes in points, portrait.
const A4: (f64, f64) = (595.276, 841.89);
const LETTER: (f64, f64) = (612.0, 792.0);
const POINTS_PER_INCH: f64 = 72.0;
const MM_PER_INCH: f64 = 25.4;
/// One point per pixel when the action does not set a dpi.
const DEFAULT_DPI: u32 = 72;

#[derive(Clone, Copy, Debug, PartialEq)]
enum PageSize {
    Fit,
    A4,
    Letter,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Orientation {
    Auto,
    Portrait,
    Landscape,
}

/// Validated `to-pdf` options.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Options {
    page_size: PageSize,
    orientation: Orientation,
    margin_mm: f64,
    dpi: u32,
}

/// Where one page and its image go, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Placement {
    page_w: f32,
    page_h: f32,
    draw_w: f32,
    draw_h: f32,
    x: f32,
    y: f32,
}

impl Options {
    fn parse(task: &Task) -> Result<Self, EngineError> {
        // Older actions carry `page = "fit"`; `page_size` replaces it.
        if task.params.contains_key("page") && task.param_str("page") != Some("fit") {
            return Err(bad("page", "only \"fit\" is supported"));
        }
        let page_size = match task.params.get("page_size") {
            None => PageSize::Fit,
            Some(v) => match v.as_str() {
                Some("fit") => PageSize::Fit,
                Some("a4") => PageSize::A4,
                Some("letter") => PageSize::Letter,
                _ => return Err(bad("page_size", "must be \"fit\", \"a4\" or \"letter\"")),
            },
        };
        let orientation = match task.params.get("orientation") {
            None => Orientation::Auto,
            Some(v) => match v.as_str() {
                Some("auto") => Orientation::Auto,
                Some("portrait") => Orientation::Portrait,
                Some("landscape") => Orientation::Landscape,
                _ => {
                    return Err(bad(
                        "orientation",
                        "must be \"auto\", \"portrait\" or \"landscape\"",
                    ));
                }
            },
        };
        let margin_mm = match task.params.get("margin_mm") {
            None => 0.0,
            Some(_) => match task.param_f64("margin_mm") {
                Some(m) if (0.0..=50.0).contains(&m) => m,
                _ => {
                    return Err(bad(
                        "margin_mm",
                        "must be a number from 0 to 50 millimetres",
                    ));
                }
            },
        };
        let dpi = match task.params.get("dpi") {
            None => DEFAULT_DPI,
            Some(_) => match task.param_i64("dpi") {
                Some(d @ 72..=600) => d as u32,
                _ => return Err(bad("dpi", "must be a whole number from 72 to 600")),
            },
        };
        Ok(Self {
            page_size,
            orientation,
            margin_mm,
            dpi,
        })
    }

    /// Page size and placement for an image of `w` by `h` pixels.
    fn placement(self, w: u32, h: u32) -> Placement {
        let natural_w = f64::from(w) * POINTS_PER_INCH / f64::from(self.dpi);
        let natural_h = f64::from(h) * POINTS_PER_INCH / f64::from(self.dpi);
        let margin = self.margin_mm * POINTS_PER_INCH / MM_PER_INCH;
        let (page_w, page_h) = match self.page_size {
            PageSize::Fit => (natural_w + 2.0 * margin, natural_h + 2.0 * margin),
            PageSize::A4 => self.fixed_page(A4, w, h),
            PageSize::Letter => self.fixed_page(LETTER, w, h),
        };
        let content_w = page_w - 2.0 * margin;
        let content_h = page_h - 2.0 * margin;
        // Fit inside the content box, keeping the aspect ratio, and never
        // enlarge past the image's own dpi size.
        let scale = (content_w / natural_w).min(content_h / natural_h).min(1.0);
        let draw_w = natural_w * scale;
        let draw_h = natural_h * scale;
        // The margins are symmetric, so the page centre is the content centre.
        let x = (page_w - draw_w) / 2.0;
        let y = (page_h - draw_h) / 2.0;
        Placement {
            page_w: page_w as f32,
            page_h: page_h as f32,
            draw_w: draw_w as f32,
            draw_h: draw_h as f32,
            x: x as f32,
            y: y as f32,
        }
    }

    /// A fixed page turned to match the orientation. `auto` follows the
    /// image: wider than tall is landscape, otherwise portrait.
    fn fixed_page(self, portrait: (f64, f64), w: u32, h: u32) -> (f64, f64) {
        let landscape = match self.orientation {
            Orientation::Auto => w > h,
            Orientation::Portrait => false,
            Orientation::Landscape => true,
        };
        if landscape {
            (portrait.1, portrait.0)
        } else {
            portrait
        }
    }
}

pub(super) fn run(
    task: &Task,
    progress: Progress,
    cancel: &CancelToken,
) -> Result<(), EngineError> {
    if task.inputs.is_empty() {
        return Err(bad("inputs", "`to-pdf` needs at least one image"));
    }
    let options = Options::parse(task)?;

    let n = task.inputs.len();
    let catalog_id = Ref::new(1);
    let tree_id = Ref::new(2);
    let ids = |i: usize| {
        let base = 3 + 3 * i as i32;
        (Ref::new(base), Ref::new(base + 1), Ref::new(base + 2))
    };

    let mut pdf = Pdf::new();
    pdf.catalog(catalog_id).pages(tree_id);
    pdf.pages(tree_id)
        .kids((0..n).map(|i| ids(i).0))
        .count(n as i32);

    for (i, path) in task.inputs.iter().enumerate() {
        cancel.check()?;
        let (page_id, image_id, content_id) = ids(i);
        let loaded = load(path)?;
        let (w, h) = (loaded.image.width(), loaded.image.height());
        let placement = options.placement(w, h);

        let mut page = pdf.page(page_id);
        page.media_box(Rect::new(0.0, 0.0, placement.page_w, placement.page_h));
        page.parent(tree_id);
        page.contents(content_id);
        page.resources().x_objects().pair(IMAGE_NAME, image_id);
        page.finish();

        let stream = ImageStream::from(loaded);
        let mut xobj = pdf.image_xobject(image_id, &stream.data);
        xobj.filter(stream.filter);
        xobj.width(w as i32);
        xobj.height(h as i32);
        if stream.gray {
            xobj.color_space().device_gray();
        } else {
            xobj.color_space().device_rgb();
        }
        xobj.bits_per_component(8);
        xobj.finish();

        let mut content = Content::new();
        content.save_state();
        content.transform([
            placement.draw_w,
            0.0,
            0.0,
            placement.draw_h,
            placement.x,
            placement.y,
        ]);
        content.x_object(IMAGE_NAME);
        content.restore_state();
        pdf.stream(content_id, &content.finish());

        progress((i + 1) as f32 / (n + 1) as f32);
    }

    cancel.check()?;
    std::fs::write(&task.output, pdf.finish())?;
    progress(1.0);
    Ok(())
}

struct ImageStream {
    data: Vec<u8>,
    filter: Filter,
    gray: bool,
}

impl From<Loaded> for ImageStream {
    fn from(l: Loaded) -> Self {
        let passthrough = l.format == ImageFormat::Jpeg && !l.reoriented;
        match (passthrough, jpeg_components(&l.bytes), l.image.color()) {
            (true, Some(1), ColorType::L8) | (true, Some(3), ColorType::Rgb8) => ImageStream {
                gray: l.image.color() == ColorType::L8,
                data: l.bytes,
                filter: Filter::DctDecode,
            },
            (_, _, ColorType::L8 | ColorType::L16) => ImageStream {
                data: compress_to_vec_zlib(l.image.to_luma8().as_raw(), FLATE_LEVEL),
                filter: Filter::FlateDecode,
                gray: true,
            },
            _ => ImageStream {
                data: compress_to_vec_zlib(flatten(&l.image).as_raw(), FLATE_LEVEL),
                filter: Filter::FlateDecode,
                gray: false,
            },
        }
    }
}

/// Component count from the first JPEG start-of-frame segment.
fn jpeg_components(b: &[u8]) -> Option<u8> {
    if !b.starts_with(&[0xFF, 0xD8]) {
        return None;
    }
    let mut i = 2;
    while i + 4 <= b.len() {
        if b[i] != 0xFF {
            return None;
        }
        let marker = b[i + 1];
        let len = usize::from(u16::from_be_bytes([b[i + 2], b[i + 3]]));
        // SOF0 to SOF15, minus DHT (C4), JPG (C8) and DAC (CC).
        if (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            return b.get(i + 9).copied();
        }
        i += 2 + len;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sof(marker: u8, comps: u8) -> Vec<u8> {
        let mut v = vec![0xFF, 0xD8];
        // An APP0 segment to skip first.
        v.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x04, 0xAA, 0xBB]);
        // A DHT segment, which shares the C0..CF range but is not a frame.
        v.extend_from_slice(&[0xFF, 0xC4, 0x00, 0x03, 0x01]);
        v.extend_from_slice(&[0xFF, marker, 0x00, 0x08, 8, 0, 16, 0, 16, comps]);
        v
    }

    #[test]
    fn finds_component_count() {
        assert_eq!(jpeg_components(&sof(0xC0, 3)), Some(3));
        assert_eq!(jpeg_components(&sof(0xC2, 1)), Some(1));
        assert_eq!(jpeg_components(&sof(0xCF, 4)), Some(4));
        assert_eq!(jpeg_components(&sof(0xC8, 3)), None);
        assert_eq!(jpeg_components(&sof(0xCC, 3)), None);
        assert_eq!(jpeg_components(&sof(0xBF, 3)), None);
        assert_eq!(jpeg_components(&sof(0xD0, 3)), None);
        assert_eq!(jpeg_components(b"\x89PNG"), None);
        assert_eq!(jpeg_components(&[0xFF, 0xD8, 0x00, 0xC0, 0, 8]), None);
        // Truncated right after the frame header.
        let mut t = sof(0xC0, 3);
        t.pop();
        assert_eq!(jpeg_components(&t), None);
        // Exactly four bytes of a segment header left is still read.
        assert_eq!(jpeg_components(&[0xFF, 0xD8, 0xFF, 0xC0, 0, 8]), None);
    }

    #[test]
    fn cmyk_jpeg_is_not_passed_through() {
        let l = Loaded {
            image: image::DynamicImage::ImageRgb8(image::RgbImage::new(16, 16)),
            bytes: sof(0xC0, 4),
            format: ImageFormat::Jpeg,
            reoriented: false,
        };
        let s = ImageStream::from(l);
        assert!(matches!(s.filter, Filter::FlateDecode));
        assert!(!s.gray);
    }

    #[test]
    fn sixteen_bit_gray_uses_flate_gray() {
        let l = Loaded {
            image: image::DynamicImage::ImageLuma16(image::ImageBuffer::new(4, 4)),
            bytes: vec![],
            format: ImageFormat::Png,
            reoriented: false,
        };
        let s = ImageStream::from(l);
        assert!(s.gray);
        assert!(matches!(s.filter, Filter::FlateDecode));
        assert_eq!(
            miniz_oxide::inflate::decompress_to_vec_zlib(&s.data)
                .unwrap()
                .len(),
            16
        );
    }
}
