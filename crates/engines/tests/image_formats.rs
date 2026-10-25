//! S-004 image engine: format conversion, quality, flattening, ICO,
//! compress and strip.

mod image_common;
use image_common::*;

use image::GenericImageView;
use kawaro_core::EngineError;
use tempfile::tempdir;

fn magic_ok(ext: &str, b: &[u8]) -> bool {
    match ext {
        "png" => b.starts_with(b"\x89PNG\r\n\x1a\n"),
        "jpg" | "jpeg" => b.starts_with(&[0xFF, 0xD8, 0xFF]),
        "webp" => b.starts_with(b"RIFF") && &b[8..12] == b"WEBP",
        "avif" => &b[4..8] == b"ftyp" && contains(&b[8..32], b"avif"),
        "gif" => b.starts_with(b"GIF8"),
        "bmp" => b.starts_with(b"BM"),
        "tif" | "tiff" => b.starts_with(b"II*\0") || b.starts_with(b"MM\0*"),
        "ico" => b.starts_with(&[0, 0, 1, 0]),
        _ => false,
    }
}

// AC2
#[test]
fn convert_to_every_format() {
    let d = tempdir().unwrap();
    for ext in [
        "png", "jpg", "jpeg", "webp", "gif", "bmp", "tif", "tiff", "avif", "PNG",
    ] {
        let bytes = run_one("convert", &fixture("alpha.png"), d.path(), ext, "");
        let lower = ext.to_ascii_lowercase();
        assert!(
            magic_ok(&lower, &bytes),
            "{ext}: bad magic {:?}",
            &bytes[..12]
        );
        if lower != "avif" {
            let img = decode(&bytes);
            assert_eq!(img.dimensions(), (64, 48), "{ext}");
            let tol = if matches!(lower.as_str(), "png" | "bmp" | "tif" | "tiff" | "gif") {
                0
            } else {
                30
            };
            assert!(
                near(rgb_at(&img, 40, 6), GREEN, tol),
                "{ext} {:?}",
                rgb_at(&img, 40, 6)
            );
            assert!(
                near(rgb_at(&img, 24, 36), BLUE, tol),
                "{ext} {:?}",
                rgb_at(&img, 24, 36)
            );
        }
    }
}

#[test]
fn convert_to_png_uses_default_compression() {
    // Plain PNG output uses balanced compression, so only `compress` pays
    // for the slow best compression.
    let d = tempdir().unwrap();
    let src = d.path().join("src.png");
    std::fs::write(&src, png_bytes(&detailed())).unwrap();
    let out = run_one("convert", &src, d.path(), "png", "");
    assert_eq!(out, png_default(&detailed()));
}

#[test]
fn convert_from_gif_bmp_jpeg() {
    let d = tempdir().unwrap();
    let gif = decode(&run_one(
        "convert",
        &fixture("sample.gif"),
        d.path(),
        "png",
        "",
    ));
    assert_eq!(gif.dimensions(), (32, 24));
    let bmp = decode(&run_one(
        "convert",
        &fixture("sample.bmp"),
        d.path(),
        "webp",
        "",
    ));
    assert_eq!(bmp.dimensions(), (40, 30));
    let jpg = decode(&run_one(
        "convert",
        &fixture("orient6.jpg"),
        d.path(),
        "tiff",
        "",
    ));
    assert_eq!(jpg.dimensions(), (48, 64));
}

#[test]
fn convert_keeps_alpha_where_supported() {
    let d = tempdir().unwrap();
    for ext in ["png", "webp", "tiff", "gif"] {
        let img = decode(&run_one(
            "convert",
            &fixture("alpha.png"),
            d.path(),
            ext,
            "",
        ));
        let rgba = img.to_rgba8();
        assert_eq!(rgba.get_pixel(4, 4)[3], 0, "{ext} lost transparency");
        assert_eq!(rgba.get_pixel(40, 40)[3], 255, "{ext}");
    }
}

#[test]
fn convert_rejects_unknown_output_extension() {
    let d = tempdir().unwrap();
    for name in ["out.xyz", "out"] {
        let out = d.path().join(name);
        let t = task("convert", vec![fixture("alpha.png")], out.clone(), "");
        match run_task(&t).0 {
            Err(EngineError::BadParam { name, reason }) => {
                assert_eq!(name, "output");
                assert!(!reason.is_empty());
            }
            other => panic!("expected BadParam output, got {other:?}"),
        }
        assert!(!out.exists());
    }
}

// AC3
#[test]
fn quality_out_of_range_is_rejected() {
    let d = tempdir().unwrap();
    for (op, q) in [
        ("convert", "0"),
        ("convert", "101"),
        ("compress", "0"),
        ("compress", "-1"),
        ("compress", "200"),
    ] {
        let out = d.path().join("o.jpg");
        let t = task(
            op,
            vec![fixture("alpha.png")],
            out.clone(),
            &format!("quality = {q}"),
        );
        assert!(
            matches!(run_task(&t).0, Err(EngineError::BadParam { ref name, .. }) if name == "quality"),
            "{op} quality {q}"
        );
        assert!(!out.exists());
    }
}

/// A 128x96 picture with smooth gradients and fine detail, so lossy
/// quality changes are visible in the file size.
fn detailed() -> image::RgbaImage {
    image::RgbaImage::from_fn(128, 96, |x, y| {
        // Cheap integer hash: noise that neither PNG filters nor zlib can
        // fully remove, next to smooth red and green ramps.
        let mut n = x.wrapping_mul(0x9E37_79B9) ^ y.wrapping_mul(0x85EB_CA6B);
        n ^= n >> 15;
        n = n.wrapping_mul(0x2C1B_3C6D);
        n ^= n >> 12;
        image::Rgba([(x * 2) as u8, (y * 2) as u8, (n % 64) as u8, 255])
    })
}

/// PNG at `CompressionType::Default` (balanced) with adaptive filters.
fn png_default(img: &image::RgbaImage) -> Vec<u8> {
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new_with_quality(
        &mut out,
        image::codecs::png::CompressionType::Default,
        image::codecs::png::FilterType::Adaptive,
    )
    .write_image(
        img.as_raw(),
        img.width(),
        img.height(),
        image::ExtendedColorType::Rgba8,
    )
    .unwrap();
    out
}

#[test]
fn jpeg_quality_changes_size() {
    let d = tempdir().unwrap();
    let src = d.path().join("src.png");
    std::fs::write(&src, png_bytes(&detailed())).unwrap();
    let low = run_one("convert", &src, d.path(), "jpg", "quality = 30");
    let high = run_one("convert", &src, d.path(), "jpg", "quality = 95");
    let default = run_one("convert", &src, d.path(), "jpg", "");
    assert!(
        low.len() < default.len(),
        "{} vs {}",
        low.len(),
        default.len()
    );
    assert!(
        default.len() < high.len(),
        "{} vs {}",
        default.len(),
        high.len()
    );
    let one = run_one("convert", &src, d.path(), "jpg", "quality = 1");
    let hundred = run_one("convert", &src, d.path(), "jpg", "quality = 100");
    assert!(one.len() < hundred.len());
}

#[test]
fn webp_lossy_and_lossless() {
    let d = tempdir().unwrap();
    let src = d.path().join("src.png");
    std::fs::write(&src, png_bytes(&detailed())).unwrap();
    let lossy = run_one("convert", &src, d.path(), "webp", "quality = 50");
    assert!(contains(&lossy, b"VP8 "), "lossy WebP has no VP8 chunk");
    assert!(!contains(&lossy, b"VP8L"));
    let default = run_one("convert", &src, d.path(), "webp", "");
    assert!(contains(&default, b"VP8 "));
    let lossless = run_one("convert", &src, d.path(), "webp", "quality = 100");
    assert!(
        contains(&lossless, b"VP8L"),
        "quality 100 WebP is not lossless"
    );
    assert_eq!(decode(&lossless).to_rgba8(), detailed());
    let small = run_one("convert", &src, d.path(), "webp", "quality = 10");
    assert!(small.len() < lossy.len());
}

#[test]
fn avif_quality_changes_size() {
    let d = tempdir().unwrap();
    let src = d.path().join("src.png");
    std::fs::write(&src, png_bytes(&detailed())).unwrap();
    let low = run_one("convert", &src, d.path(), "avif", "quality = 20");
    let high = run_one("convert", &src, d.path(), "avif", "quality = 95");
    assert!(magic_ok("avif", &low));
    assert!(low.len() < high.len(), "{} vs {}", low.len(), high.len());
}

// AC4
#[test]
fn alpha_flattens_onto_white() {
    let d = tempdir().unwrap();
    let jpg = decode(&run_one(
        "convert",
        &fixture("alpha.png"),
        d.path(),
        "jpg",
        "",
    ));
    let p = rgb_at(&jpg, 6, 6);
    assert!(p.iter().all(|&c| c >= 240), "jpeg corner {p:?}");
    let bmp = decode(&run_one(
        "convert",
        &fixture("alpha.png"),
        d.path(),
        "bmp",
        "",
    ));
    assert_eq!(rgb_at(&bmp, 6, 6), [255, 255, 255]);
    assert!(!bmp.color().has_alpha(), "bmp kept alpha");
    assert_eq!(rgb_at(&bmp, 40, 6), GREEN);
}

#[test]
fn semi_transparent_pixels_blend_with_white() {
    let d = tempdir().unwrap();
    let src = d.path().join("half.png");
    let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([0, 0, 0, 128]));
    std::fs::write(&src, png_bytes(&img)).unwrap();
    let bmp = decode(&run_one("convert", &src, d.path(), "bmp", ""));
    let p = rgb_at(&bmp, 3, 3);
    assert!(p.iter().all(|&c| (125..=129).contains(&c)), "{p:?}");
}

fn ico_entries(b: &[u8]) -> Vec<(u32, u32)> {
    assert_eq!(&b[..4], &[0, 0, 1, 0]);
    let n = u16::from_le_bytes([b[4], b[5]]) as usize;
    (0..n)
        .map(|i| {
            let e = &b[6 + 16 * i..];
            let side = |v: u8| if v == 0 { 256 } else { v as u32 };
            (side(e[0]), side(e[1]))
        })
        .collect()
}

// AC5
#[test]
fn ico_has_every_size_up_to_the_source() {
    let d = tempdir().unwrap();
    let big = d.path().join("big.png");
    let mut src = quadrants(300, 200);
    src.put_pixel(0, 0, image::Rgba([1, 2, 3, 255]));
    std::fs::write(&big, png_bytes(&src)).unwrap();
    let bytes = run_one("convert", &big, d.path(), "ico", "");
    let sizes: Vec<u32> = ico_entries(&bytes)
        .iter()
        .map(|&(w, h)| {
            assert_eq!(w, h, "icon frames are square");
            w
        })
        .collect();
    assert_eq!(sizes, [16, 32, 48, 64, 128, 256]);

    let small = run_one("convert", &fixture("alpha.png"), d.path(), "ico", "");
    let sizes: Vec<u32> = ico_entries(&small).iter().map(|&(w, _)| w).collect();
    assert_eq!(sizes, [16, 32, 48, 64]);
    // The decoder picks the largest frame: 64x64 with the 64x48 picture
    // centred and padded with transparency, not stretched.
    let img = decode(&small).to_rgba8();
    assert_eq!(img.dimensions(), (64, 64));
    assert_eq!(img.get_pixel(40, 2)[3], 0, "top padding");
    assert_eq!(img.get_pixel(40, 61)[3], 0, "bottom padding");
    assert_eq!(
        img.get_pixel(40, 10).0,
        [0, 255, 0, 255],
        "picture starts at row 8"
    );
    assert_eq!(
        img.get_pixel(40, 54).0,
        [255, 255, 0, 255],
        "picture ends at row 55"
    );
}

#[test]
fn ico_from_tiny_source_has_one_frame() {
    let d = tempdir().unwrap();
    let tiny = d.path().join("tiny.png");
    std::fs::write(&tiny, png_bytes(&quadrants(10, 6))).unwrap();
    let bytes = run_one("convert", &tiny, d.path(), "ico", "");
    assert_eq!(ico_entries(&bytes), [(16, 16)]);
    let exact = d.path().join("exact.png");
    std::fs::write(&exact, png_bytes(&quadrants(32, 32))).unwrap();
    let bytes = run_one("convert", &exact, d.path(), "ico", "");
    assert_eq!(ico_entries(&bytes), [(16, 16), (32, 32)]);
}

// AC11
#[test]
fn compress_shrinks_a_quality_95_jpeg() {
    let d = tempdir().unwrap();
    let src = d.path().join("q95.jpg");
    let jpeg = jpeg_bytes(&detailed(), 95, None);
    std::fs::write(&src, &jpeg).unwrap();
    let out = run_one("compress", &src, d.path(), "jpg", "");
    assert!(magic_ok("jpg", &out));
    assert!(out.len() < jpeg.len(), "{} >= {}", out.len(), jpeg.len());
    assert_eq!(decode(&out).dimensions(), (128, 96));
    let harder = run_one("compress", &src, d.path(), "jpg", "quality = 40");
    assert!(harder.len() < out.len());
}

#[test]
fn compress_png_uses_best_compression() {
    let d = tempdir().unwrap();
    let src = d.path().join("src.png");
    let mut fast = Vec::new();
    image::codecs::png::PngEncoder::new_with_quality(
        &mut fast,
        image::codecs::png::CompressionType::Fast,
        image::codecs::png::FilterType::NoFilter,
    )
    .write_image(
        detailed().as_raw(),
        128,
        96,
        image::ExtendedColorType::Rgba8,
    )
    .unwrap();
    std::fs::write(&src, &fast).unwrap();
    let default = png_default(&detailed());
    let out = run_one("compress", &src, d.path(), "png", "");
    assert!(magic_ok("png", &out));
    assert!(
        out.len() < default.len(),
        "{} >= {}",
        out.len(),
        default.len()
    );
    assert_eq!(
        decode(&out).to_rgba8(),
        detailed(),
        "PNG compress is lossless"
    );
}

#[test]
fn compress_other_formats_stay_valid() {
    let d = tempdir().unwrap();
    let src = d.path().join("src.png");
    std::fs::write(&src, png_bytes(&detailed())).unwrap();
    for ext in ["webp", "avif", "gif", "bmp", "tiff"] {
        let input = d.path().join(format!("in.{ext}"));
        std::fs::write(&input, run_one("convert", &src, d.path(), ext, "")).unwrap();
        if ext == "avif" {
            continue;
        }
        let out = run_one("compress", &input, d.path(), ext, "");
        assert!(magic_ok(ext, &out), "{ext}");
        assert_eq!(decode(&out).dimensions(), (128, 96), "{ext}");
    }
}

// AC12
#[test]
fn strip_removes_exif() {
    let d = tempdir().unwrap();
    let src = std::fs::read(fixture("orient6.jpg")).unwrap();
    assert!(contains(&src, b"Exif"));
    let out = run_one("strip", &fixture("orient6.jpg"), d.path(), "jpg", "");
    assert!(magic_ok("jpg", &out));
    assert!(!contains(&out, b"Exif"), "Exif marker survived strip");
    assert!(
        !contains(&out, &[0xFF, 0xE1]),
        "APP1 segment survived strip"
    );
    let img = decode(&out);
    assert_eq!(img.dimensions(), (48, 64), "orientation baked in");
    assert!(near(rgb_at(&img, 40, 8), RED, 40));
}

#[test]
fn strip_png_keeps_pixels() {
    let d = tempdir().unwrap();
    let out = run_one("strip", &fixture("alpha.png"), d.path(), "png", "");
    assert_eq!(decode(&out).to_rgba8(), quadrants_alpha());
    assert!(!contains(&out, b"eXIf"));
}

#[test]
fn ico_pads_tall_sources_left_and_right() {
    let d = tempdir().unwrap();
    // orient6.jpg displays as 48x64, so padding is 8 columns on each side.
    let bytes = run_one("convert", &fixture("orient6.jpg"), d.path(), "ico", "");
    let img = decode(&bytes).to_rgba8();
    assert_eq!(img.dimensions(), (64, 64));
    assert_eq!(img.get_pixel(2, 40)[3], 0, "left padding");
    assert_eq!(img.get_pixel(61, 40)[3], 0, "right padding");
    assert_eq!(img.get_pixel(10, 40)[3], 255, "picture starts at column 8");
    assert_eq!(img.get_pixel(54, 40)[3], 255, "picture ends at column 55");
}

#[test]
fn default_quality_is_90_and_75_for_compress() {
    let d = tempdir().unwrap();
    let src = d.path().join("src.png");
    std::fs::write(&src, png_bytes(&detailed())).unwrap();
    let default = run_one("convert", &src, d.path(), "jpg", "");
    let ninety = run_one("convert", &src, d.path(), "jpg", "quality = 90");
    assert_eq!(default, ninety);
    let compressed = run_one("compress", &src, d.path(), "jpg", "");
    let seventy_five = run_one("compress", &src, d.path(), "jpg", "quality = 75");
    assert_eq!(compressed, seventy_five);
    assert_ne!(compressed, ninety);
}

use image::ImageEncoder;
