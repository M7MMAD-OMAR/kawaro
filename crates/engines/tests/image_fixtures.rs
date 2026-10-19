//! Fixture files for the image engine tests (S-004), committed under
//! `tests/fixtures/images`. `regenerate` rewrites them; `fixtures_are_current`
//! checks that the committed files still decode to the documented pictures.

mod image_common;
use image_common::*;

#[test]
#[ignore = "writes into the repository; run by hand to refresh fixtures"]
fn regenerate() {
    let dir = fixture("");
    std::fs::create_dir_all(&dir).unwrap();
    quadrants_alpha().save(fixture("alpha.png")).unwrap();
    std::fs::write(
        fixture("orient6.jpg"),
        jpeg_bytes(&quadrants(64, 48), 90, Some(6)),
    )
    .unwrap();
    quadrants(32, 24).save(fixture("sample.gif")).unwrap();
    image::DynamicImage::ImageRgba8(quadrants(40, 30))
        .to_rgb8()
        .save(fixture("sample.bmp"))
        .unwrap();
}

#[test]
fn fixtures_are_current() {
    let png = image::open(fixture("alpha.png")).unwrap().to_rgba8();
    assert_eq!(png, quadrants_alpha());

    let jpg = std::fs::read(fixture("orient6.jpg")).unwrap();
    assert!(contains(&jpg, &exif_orientation_segment(6)));
    let stored = decode(&jpg);
    assert_eq!((stored.width(), stored.height()), (64, 48));

    let gif = image::open(fixture("sample.gif")).unwrap();
    assert_eq!((gif.width(), gif.height()), (32, 24));
    let bmp = image::open(fixture("sample.bmp")).unwrap();
    assert_eq!((bmp.width(), bmp.height()), (40, 30));
}
