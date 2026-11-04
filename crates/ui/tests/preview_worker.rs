//! S-118: the preview worker helpers.
//!
//! The fixtures are generated here so the repository carries no 24 MP photo:
//! a 24 MP baseline JPEG, an 8 MP PNG, one small file of each other supported
//! format, an SVG and a patched JPEG header that claims 100 MP. Every helper
//! must answer with an Option and never panic, whatever the file holds.
#![cfg(unix)]

use kawaro_ui::preview_worker::{
    Generation, MAX_DRAFT_SIDE, Next, decode_draft, mask_hexagon, probe_dimensions,
};
use slint::{Rgba8Pixel, SharedPixelBuffer};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// A private directory under the build folder, so parallel tests and the bench
/// never share a file and nothing lands in the /tmp tmpfs.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("s118-preview-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn jpeg(path: &Path, width: u32, height: u32) {
    let image = image::RgbImage::from_fn(width, height, |x, y| {
        image::Rgb([
            (x % 256) as u8,
            (y % 256) as u8,
            ((x / 3 + y / 2) % 256) as u8,
        ])
    });
    let file = std::fs::File::create(path).unwrap();
    let mut encoder =
        image::codecs::jpeg::JpegEncoder::new_with_quality(std::io::BufWriter::new(file), 80);
    encoder
        .encode_image(&image::DynamicImage::ImageRgb8(image))
        .unwrap();
}

fn png(path: &Path, width: u32, height: u32) {
    let image = image::RgbImage::from_fn(width, height, |x, y| {
        image::Rgb([(x % 256) as u8, 200, (y % 256) as u8])
    });
    let file = std::fs::File::create(path).unwrap();
    let encoder = image::codecs::png::PngEncoder::new_with_quality(
        std::io::BufWriter::new(file),
        image::codecs::png::CompressionType::Fast,
        image::codecs::png::FilterType::NoFilter,
    );
    image::ImageEncoder::write_image(
        encoder,
        image.as_raw(),
        width,
        height,
        image::ExtendedColorType::Rgb8,
    )
    .unwrap();
}

fn rgba_file(path: &Path, format: image::ImageFormat) {
    let image = image::RgbaImage::from_fn(8, 6, |x, y| {
        image::Rgba([(x * 30) as u8, (y * 40) as u8, 128, 255])
    });
    let file = std::fs::File::create(path).unwrap();
    let mut writer = std::io::BufWriter::new(file);
    match format {
        image::ImageFormat::Gif => {
            image::codecs::gif::GifEncoder::new(&mut writer)
                .encode(&image, 8, 6, image::ExtendedColorType::Rgba8)
                .unwrap();
        }
        image::ImageFormat::WebP => {
            image::codecs::webp::WebPEncoder::new_lossless(&mut writer)
                .encode(image.as_raw(), 8, 6, image::ExtendedColorType::Rgba8)
                .unwrap();
        }
        image::ImageFormat::Bmp => {
            image::codecs::bmp::BmpEncoder::new(&mut writer)
                .encode(&image, 8, 6, image::ExtendedColorType::Rgba8)
                .unwrap();
        }
        other => panic!("unsupported fixture format {other:?}"),
    }
}

fn svg(path: &Path) {
    std::fs::write(
        path,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300">
             <rect width="400" height="300" fill="#3366cc"/>
             <circle cx="200" cy="150" r="100" fill="#ffcc00"/>
           </svg>"##,
    )
    .unwrap();
}

/// A real tiny JPEG with its SOF dimensions patched to 10000 x 10000, so the
/// header probe sees 100 MP without the test encoding one.
fn huge_jpeg_header(path: &Path) {
    let mut bytes = Vec::new();
    {
        let image = image::RgbImage::from_fn(32, 32, |x, y| image::Rgb([x as u8, y as u8, 7]));
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 50);
        encoder
            .encode_image(&image::DynamicImage::ImageRgb8(image))
            .unwrap();
    }
    let sof = bytes
        .windows(2)
        .position(|w| (w[0] == 0xFF && w[1] == 0xC0) || (w[0] == 0xFF && w[1] == 0xC2))
        .expect("a JPEG has a start of frame marker");
    let height = 10000u16.to_be_bytes();
    let width = 10000u16.to_be_bytes();
    bytes[sof + 5..sof + 7].copy_from_slice(&height);
    bytes[sof + 7..sof + 9].copy_from_slice(&width);
    std::fs::write(path, bytes).unwrap();
}

/// The 24 MP photo, built once per test binary: encoding it costs seconds.
fn photo() -> &'static Path {
    static PHOTO: OnceLock<PathBuf> = OnceLock::new();
    PHOTO.get_or_init(|| {
        let dir = scratch("photo");
        let path = dir.join("photo.jpg");
        jpeg(&path, 6000, 4000);
        path
    })
}

#[test]
fn the_dimensions_come_from_the_header_before_any_pixels() {
    assert_eq!(probe_dimensions(photo()), Some((6000, 4000)));

    let dir = scratch("headers");
    let map = dir.join("map.png");
    png(&map, 3264, 2448);
    assert_eq!(probe_dimensions(&map), Some((3264, 2448)));

    let icon = dir.join("icon.svg");
    svg(&icon);
    assert_eq!(probe_dimensions(&icon), Some((400, 300)));

    let gif = dir.join("small.gif");
    rgba_file(&gif, image::ImageFormat::Gif);
    assert_eq!(probe_dimensions(&gif), Some((8, 6)));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_draft_is_capped_scaled_and_never_full_size() {
    let buffer = decode_draft(photo(), 1000).expect("a draft of the 24 MP photo");
    let longest = buffer.width().max(buffer.height());
    assert_eq!(longest, 1000, "the draft uses the whole cap");
    assert_eq!((buffer.width(), buffer.height()), (1000, 667));
    assert_eq!(buffer.as_slice().len(), 1000 * 667);

    // A smaller request is respected, and the cap is not exceeded even when
    // the caller asks for more.
    let small = decode_draft(photo(), 200).expect("a small draft");
    assert_eq!(small.width().max(small.height()), 200);
    let capped = decode_draft(photo(), 5000).expect("a capped draft");
    assert_eq!(capped.width().max(capped.height()), MAX_DRAFT_SIDE);

    let dir = scratch("drafts");
    let map = dir.join("map.png");
    png(&map, 3264, 2448);
    let buffer = decode_draft(&map, 1000).expect("a PNG draft");
    assert_eq!((buffer.width(), buffer.height()), (1000, 750));

    let icon = dir.join("icon.svg");
    svg(&icon);
    let buffer = decode_draft(&icon, 1000).expect("an SVG draft");
    assert_eq!((buffer.width(), buffer.height()), (400, 300));

    for (name, format) in [
        ("small.gif", image::ImageFormat::Gif),
        ("small.webp", image::ImageFormat::WebP),
        ("small.bmp", image::ImageFormat::Bmp),
    ] {
        let path = dir.join(name);
        rgba_file(&path, format);
        let buffer = decode_draft(&path, 1000).expect(name);
        assert_eq!((buffer.width(), buffer.height()), (8, 6), "{name}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_100_mp_source_shows_dimensions_but_no_draft() {
    let dir = scratch("huge");
    let path = dir.join("huge.jpg");
    huge_jpeg_header(&path);
    assert_eq!(probe_dimensions(&path), Some((10000, 10000)));
    assert!(
        decode_draft(&path, 1000).is_none(),
        "over the 40 MP cap there is no draft"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_broken_or_missing_file_is_a_soft_failure() {
    let dir = scratch("broken");
    let junk = dir.join("not-an-image.png");
    std::fs::write(&junk, b"this is not a picture").unwrap();
    assert_eq!(probe_dimensions(&junk), None);
    assert!(decode_draft(&junk, 1000).is_none());
    assert_eq!(probe_dimensions(&dir.join("missing.jpg")), None);
    assert!(decode_draft(&dir.join("missing.jpg"), 1000).is_none());
    // A zero size asks for nothing.
    assert!(decode_draft(photo(), 0).is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_mask_clears_the_corners_and_softens_the_edge() {
    let (w, h, r) = (128u32, 128u32, 64.0f32);
    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(w, h);
    for pixel in buffer.make_mut_slice() {
        *pixel = Rgba8Pixel::new(10, 20, 30, 255);
    }
    mask_hexagon(&mut buffer, r).expect("a square buffer with a positive radius");
    let pixel = |x: u32, y: u32| buffer.as_slice()[(y * w + x) as usize];

    for (x, y) in [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)] {
        assert_eq!(pixel(x, y).a, 0, "corner {x},{y} is outside the hexagon");
    }
    assert_eq!(pixel(w / 2, h / 2).a, 255, "the centre stays opaque");
    // The colour channels are untouched.
    let inside = pixel(w / 2, h / 2);
    assert_eq!((inside.r, inside.g, inside.b), (10, 20, 30));

    let partial = buffer
        .as_slice()
        .iter()
        .filter(|p| p.a > 0 && p.a < 255)
        .count();
    assert!(partial > 0, "the edge is antialiased, not a hard cut");
}

#[test]
fn a_degenerate_mask_answers_none() {
    let mut empty = SharedPixelBuffer::<Rgba8Pixel>::new(0, 0);
    assert!(mask_hexagon(&mut empty, 10.0).is_none());
    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(8, 8);
    assert!(mask_hexagon(&mut buffer, 0.0).is_none());
    assert!(mask_hexagon(&mut buffer, f32::NAN).is_none());
}

#[test]
fn twenty_rapid_changes_publish_only_the_newest_generation() {
    let mut generation = Generation::new();
    assert_eq!(generation.submit(), Next::Render(0));
    for _ in 0..19 {
        assert_eq!(generation.submit(), Next::Idle, "one render at a time");
    }
    assert_eq!(generation.running(), Some(0));
    assert_eq!(generation.wanted(), Some(19));

    let (published, next) = generation.finish();
    assert_eq!(published, None, "the stale render is dropped");
    assert_eq!(next, Next::Render(19));

    let (published, next) = generation.finish();
    assert_eq!(published, Some(19));
    assert_eq!(next, Next::Idle);
    assert_eq!(generation.published(), Some(19));
}

#[test]
fn a_fake_renderer_keeps_one_render_in_flight_and_publishes_in_order() {
    // The fake renderer finishes immediately unless the next change lands
    // while it runs: every third change waits for the render before it.
    let mut generation = Generation::new();
    let mut published = Vec::new();
    let mut renders = 0;
    for _ in 0..20 {
        match generation.submit() {
            Next::Render(_) => renders += 1,
            Next::Idle => {}
        }
        if renders > 0 && renders % 3 == 0 {
            let (done, next) = generation.finish();
            if let Some(id) = done {
                published.push(id);
            }
            if let Next::Render(_) = next {
                renders += 1;
            }
        }
    }
    // Drain what is left, twice at most: one render plus one waiting.
    for _ in 0..2 {
        let (done, next) = generation.finish();
        if let Some(id) = done {
            published.push(id);
        }
        if !matches!(next, Next::Render(_)) {
            break;
        }
    }
    assert!(!published.is_empty());
    assert!(
        published.windows(2).all(|pair| pair[0] < pair[1]),
        "published in order: {published:?}"
    );
    assert_eq!(
        published.last().copied(),
        Some(19),
        "the newest generation is the last published"
    );
    assert_eq!(generation.running(), None);
}
