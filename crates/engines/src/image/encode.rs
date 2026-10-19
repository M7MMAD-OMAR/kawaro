//! Output formats: choosing one from the output extension and encoding.

use image::codecs::{
    avif::AvifEncoder,
    bmp::BmpEncoder,
    gif::GifEncoder,
    ico::{IcoEncoder, IcoFrame},
    jpeg::JpegEncoder,
    png::{CompressionType, FilterType as PngFilter, PngEncoder},
    tiff::TiffEncoder,
};
use image::{
    ColorType, DynamicImage, ExtendedColorType, GrayImage, ImageEncoder, RgbImage, RgbaImage,
    imageops::{self, FilterType},
};
use sbarhive_core::EngineError;
use sbarhive_core::media::extension;
use std::io::Cursor;
use std::path::Path;

/// rav1e speed, 1 (slowest) to 10. 7 keeps a 12 megapixel photo in seconds.
const AVIF_SPEED: u8 = 7;
/// NeuQuant sampling for GIF palettes, 1 (best) to 30 (fastest).
const GIF_SPEED: i32 = 10;
/// Frame sizes of a multi-size icon, kept when the source is at least as large.
const ICO_SIZES: [u32; 6] = [16, 32, 48, 64, 128, 256];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Target {
    Png,
    Jpeg,
    Webp,
    Avif,
    Gif,
    Bmp,
    Tiff,
    Ico,
}

impl Target {
    pub(super) fn from_path(path: &Path) -> Result<Self, EngineError> {
        Ok(match extension(path).as_str() {
            "png" => Target::Png,
            "jpg" | "jpeg" => Target::Jpeg,
            "webp" => Target::Webp,
            "avif" => Target::Avif,
            "gif" => Target::Gif,
            "bmp" => Target::Bmp,
            "tif" | "tiff" => Target::Tiff,
            "ico" => Target::Ico,
            other => {
                return Err(EngineError::BadParam {
                    name: "output".into(),
                    reason: format!(
                        "cannot write `.{other}` images; use png, jpg, webp, avif, gif, bmp, tiff or ico"
                    ),
                });
            }
        })
    }
}

fn failed(target: Target, e: impl std::fmt::Display) -> EngineError {
    EngineError::Failed(format!("cannot encode {target:?}: {e}"))
}

/// Encodes `img` as `target`. `quality` applies to JPEG, lossy WebP and
/// AVIF; WebP at quality 100 is lossless. `best` asks for the strongest
/// lossless compression (PNG).
pub(super) fn encode(
    img: &DynamicImage,
    target: Target,
    quality: u8,
    best: bool,
) -> Result<Vec<u8>, EngineError> {
    let mut out = Vec::new();
    let (w, h) = (img.width(), img.height());
    match target {
        Target::Png => {
            let (compression, filter) = if best {
                (CompressionType::Best, PngFilter::Adaptive)
            } else {
                (CompressionType::Default, PngFilter::Adaptive)
            };
            let img = png_safe(img);
            PngEncoder::new_with_quality(&mut out, compression, filter)
                .write_image(img.as_bytes(), w, h, img.color().into())
                .map_err(|e| failed(target, e))?;
        }
        Target::Jpeg => {
            let enc = JpegEncoder::new_with_quality(&mut out, quality);
            if is_gray(img.color()) {
                enc.write_image(gray_on_white(img).as_raw(), w, h, ExtendedColorType::L8)
            } else {
                enc.write_image(flatten(img).as_raw(), w, h, ExtendedColorType::Rgb8)
            }
            .map_err(|e| failed(target, e))?;
        }
        Target::Bmp => {
            BmpEncoder::new(&mut out)
                .write_image(flatten(img).as_raw(), w, h, ExtendedColorType::Rgb8)
                .map_err(|e| failed(target, e))?;
        }
        Target::Gif => {
            GifEncoder::new_with_speed(&mut out, GIF_SPEED)
                .encode(img.to_rgba8().as_raw(), w, h, ExtendedColorType::Rgba8)
                .map_err(|e| failed(target, e))?;
        }
        Target::Tiff => {
            let enc = TiffEncoder::new(Cursor::new(&mut out));
            if img.color().has_alpha() {
                enc.write_image(img.to_rgba8().as_raw(), w, h, ExtendedColorType::Rgba8)
            } else {
                enc.write_image(img.to_rgb8().as_raw(), w, h, ExtendedColorType::Rgb8)
            }
            .map_err(|e| failed(target, e))?;
        }
        Target::Avif => {
            let enc = AvifEncoder::new_with_speed_quality(&mut out, AVIF_SPEED, quality);
            if img.color().has_alpha() {
                enc.write_image(img.to_rgba8().as_raw(), w, h, ExtendedColorType::Rgba8)
            } else {
                enc.write_image(img.to_rgb8().as_raw(), w, h, ExtendedColorType::Rgb8)
            }
            .map_err(|e| failed(target, e))?;
        }
        Target::Webp => out = webp(img, quality)?,
        Target::Ico => out = ico(img)?,
    }
    Ok(out)
}

fn webp(img: &DynamicImage, quality: u8) -> Result<Vec<u8>, EngineError> {
    let rgba;
    let rgb;
    let encoder = if img.color().has_alpha() {
        rgba = img.to_rgba8();
        webp::Encoder::from_rgba(rgba.as_raw(), img.width(), img.height())
    } else {
        rgb = img.to_rgb8();
        webp::Encoder::from_rgb(rgb.as_raw(), img.width(), img.height())
    };
    let config = webp::WebPConfig::new().map_err(|()| failed(Target::Webp, "no libwebp config"))?;
    let mut config = config;
    config.lossless = i32::from(quality == 100);
    config.quality = f32::from(quality);
    let mem = encoder
        .encode_advanced(&config)
        .map_err(|e| failed(Target::Webp, format!("{e:?}")))?;
    Ok(mem.to_vec())
}

/// Multi-size icon: the picture centred on a transparent square, then one
/// PNG frame for every standard size up to the square side (16 at least).
fn ico(img: &DynamicImage) -> Result<Vec<u8>, EngineError> {
    let (w, h) = (img.width(), img.height());
    let side = w.max(h);
    let mut square = RgbaImage::new(side, side);
    imageops::overlay(
        &mut square,
        &img.to_rgba8(),
        i64::from((side - w) / 2),
        i64::from((side - h) / 2),
    );
    let mut sizes: Vec<u32> = ICO_SIZES.into_iter().filter(|&s| s <= side).collect();
    if sizes.is_empty() {
        sizes.push(ICO_SIZES[0]);
    }
    let frames = sizes
        .iter()
        .map(|&s| {
            let frame = if s == side {
                square.clone()
            } else {
                imageops::resize(&square, s, s, FilterType::Lanczos3)
            };
            IcoFrame::as_png(frame.as_raw(), s, s, ExtendedColorType::Rgba8)
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| failed(Target::Ico, e))?;
    let mut out = Vec::new();
    IcoEncoder::new(&mut out)
        .encode_images(&frames)
        .map_err(|e| failed(Target::Ico, e))?;
    Ok(out)
}

fn is_gray(c: ColorType) -> bool {
    matches!(
        c,
        ColorType::L8 | ColorType::La8 | ColorType::L16 | ColorType::La16
    )
}

/// PNG takes 8 and 16 bit gray and RGB with or without alpha; anything
/// else (float) becomes 16 bit RGBA.
fn png_safe(img: &DynamicImage) -> std::borrow::Cow<'_, DynamicImage> {
    match img.color() {
        ColorType::Rgb32F | ColorType::Rgba32F => {
            std::borrow::Cow::Owned(DynamicImage::ImageRgba16(img.to_rgba16()))
        }
        _ => std::borrow::Cow::Borrowed(img),
    }
}

/// `c` over white at opacity `a`, rounded.
pub(super) fn over_white(c: u8, a: u8) -> u8 {
    let (c, a) = (u32::from(c), u32::from(a));
    ((c * a + 255 * (255 - a) + 127) / 255) as u8
}

/// RGB with any alpha composited onto white.
pub(super) fn flatten(img: &DynamicImage) -> RgbImage {
    if !img.color().has_alpha() {
        return img.to_rgb8();
    }
    let rgba = img.to_rgba8();
    RgbImage::from_fn(img.width(), img.height(), |x, y| {
        let p = rgba.get_pixel(x, y).0;
        image::Rgb([
            over_white(p[0], p[3]),
            over_white(p[1], p[3]),
            over_white(p[2], p[3]),
        ])
    })
}

/// Gray with any alpha composited onto white.
pub(super) fn gray_on_white(img: &DynamicImage) -> GrayImage {
    let la = img.to_luma_alpha8();
    GrayImage::from_fn(img.width(), img.height(), |x, y| {
        let p = la.get_pixel(x, y).0;
        image::Luma([over_white(p[0], p[1])])
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn over_white_blends() {
        assert_eq!(over_white(0, 0), 255);
        assert_eq!(over_white(0, 255), 0);
        assert_eq!(over_white(200, 255), 200);
        assert_eq!(over_white(0, 128), 127);
        assert_eq!(over_white(100, 51), 224);
    }

    #[test]
    fn gray_alpha_flattens_to_gray() {
        let img = DynamicImage::ImageLumaA8(image::GrayAlphaImage::from_fn(2, 1, |x, _| {
            image::LumaA([10, if x == 0 { 0 } else { 255 }])
        }));
        let g = gray_on_white(&img);
        assert_eq!(g.as_raw(), &vec![255, 10]);
        let rgb = flatten(&img);
        assert_eq!(rgb.as_raw(), &vec![255, 255, 255, 10, 10, 10]);
    }

    #[test]
    fn float_images_become_16_bit_for_png() {
        let f = DynamicImage::ImageRgb32F(image::Rgb32FImage::new(2, 2));
        assert_eq!(png_safe(&f).color(), ColorType::Rgba16);
        let f = DynamicImage::ImageRgba32F(image::Rgba32FImage::new(2, 2));
        assert_eq!(png_safe(&f).color(), ColorType::Rgba16);
        let i = DynamicImage::ImageLuma16(image::ImageBuffer::new(2, 2));
        assert_eq!(png_safe(&i).color(), ColorType::L16);
        let bytes = encode(&f, Target::Png, 90, false).unwrap();
        assert_eq!(
            image::load_from_memory(&bytes).unwrap().color(),
            ColorType::Rgba16
        );
    }

    #[test]
    fn gray_detection() {
        for c in [
            ColorType::L8,
            ColorType::La8,
            ColorType::L16,
            ColorType::La16,
        ] {
            assert!(is_gray(c), "{c:?}");
        }
        for c in [
            ColorType::Rgb8,
            ColorType::Rgba8,
            ColorType::Rgb16,
            ColorType::Rgba32F,
        ] {
            assert!(!is_gray(c), "{c:?}");
        }
    }

    #[test]
    fn gray_jpeg_stays_single_channel() {
        let img = DynamicImage::ImageLuma8(GrayImage::from_pixel(8, 8, image::Luma([90])));
        let bytes = encode(&img, Target::Jpeg, 90, false).unwrap();
        assert_eq!(
            image::load_from_memory(&bytes).unwrap().color(),
            ColorType::L8
        );
        let rgb = DynamicImage::ImageRgb8(RgbImage::new(8, 8));
        let bytes = encode(&rgb, Target::Jpeg, 90, false).unwrap();
        assert_eq!(
            image::load_from_memory(&bytes).unwrap().color(),
            ColorType::Rgb8
        );
    }

    #[test]
    fn opaque_tiff_and_avif_have_no_alpha() {
        let rgb = DynamicImage::ImageRgb8(RgbImage::from_pixel(8, 8, image::Rgb([1, 2, 3])));
        let tiff = encode(&rgb, Target::Tiff, 90, false).unwrap();
        assert_eq!(
            image::load_from_memory(&tiff).unwrap().color(),
            ColorType::Rgb8
        );
        let rgba = DynamicImage::ImageRgba8(RgbaImage::new(8, 8));
        let tiff = encode(&rgba, Target::Tiff, 90, false).unwrap();
        assert_eq!(
            image::load_from_memory(&tiff).unwrap().color(),
            ColorType::Rgba8
        );
        // ravif writes an alpha auxiliary image only when there is alpha.
        let avif = encode(&rgb, Target::Avif, 50, false).unwrap();
        let avif_alpha = encode(&rgba, Target::Avif, 50, false).unwrap();
        assert!(!avif.windows(5).any(|w| w == b"alpha"));
        assert!(avif_alpha.windows(5).any(|w| w == b"alpha"));
    }

    #[test]
    fn opaque_webp_has_no_alpha_chunk() {
        let rgb = DynamicImage::ImageRgb8(RgbImage::from_pixel(8, 8, image::Rgb([1, 2, 3])));
        let bytes = encode(&rgb, Target::Webp, 80, false).unwrap();
        assert!(!bytes.windows(4).any(|w| w == b"ALPH"));
        let rgba = DynamicImage::ImageRgba8(RgbaImage::from_pixel(8, 8, image::Rgba([1, 2, 3, 7])));
        let bytes = encode(&rgba, Target::Webp, 80, false).unwrap();
        assert!(bytes.windows(4).any(|w| w == b"ALPH"));
    }
}
