//! `to-pdf`: one page per input, each page exactly the size of its image at
//! one point per pixel. Upright 8 bit gray or RGB JPEGs are embedded as they
//! are (DCTDecode); everything else is stored losslessly with Flate, alpha
//! composited onto white.

use super::{Loaded, bad, encode::flatten, load};
use image::{ColorType, ImageFormat};
use miniz_oxide::deflate::compress_to_vec_zlib;
use pdf_writer::{Content, Filter, Finish, Name, Pdf, Rect, Ref};
use sbarhive_core::{CancelToken, EngineError, Progress, Task};

/// zlib level for Flate image streams.
const FLATE_LEVEL: u8 = 6;
const IMAGE_NAME: Name<'static> = Name(b"Im1");

pub(super) fn run(
    task: &Task,
    progress: Progress,
    cancel: &CancelToken,
) -> Result<(), EngineError> {
    if task.inputs.is_empty() {
        return Err(bad("inputs", "`to-pdf` needs at least one image"));
    }
    if task.params.contains_key("page") && task.param_str("page") != Some("fit") {
        return Err(bad("page", "only \"fit\" is supported"));
    }

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
        let (w, h) = (loaded.image.width() as f32, loaded.image.height() as f32);

        let mut page = pdf.page(page_id);
        page.media_box(Rect::new(0.0, 0.0, w, h));
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
        content.transform([w, 0.0, 0.0, h, 0.0, 0.0]);
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
