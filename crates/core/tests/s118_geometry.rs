//! S-118: image geometry, kawaro_core::image_geom.
//!
//! target() maps a resize mode to the output size and, for Fill, the crop in
//! source pixels. Contract (tester): integer rounding half up, a 1 px floor,
//! None for a zero source or a zero box, no enlargement for LongestSide and
//! Fit, Exact takes the given size, and Fit reports the fitted image size
//! rather than the box.

use kawaro_core::image_geom::{Mode, target};

fn check(
    source: (u32, u32),
    mode: Mode,
    width: u32,
    height: u32,
    crop: Option<(u32, u32, u32, u32)>,
) {
    let target = target(source, mode).unwrap_or_else(|| panic!("no target for {source:?}"));
    assert_eq!(
        (target.width, target.height),
        (width, height),
        "size for {source:?}"
    );
    match crop {
        None => assert!(target.crop.is_none(), "unexpected crop for {source:?}"),
        Some(expected) => {
            let crop = target.crop.as_ref().expect("a fill target carries a crop");
            assert_eq!(
                (crop.x, crop.y, crop.width, crop.height),
                expected,
                "crop for {source:?}"
            );
        }
    }
}

#[test]
fn target_sizes_and_crops_follow_the_contract() {
    // Percent: round each side half up, 1 px floor.
    check((1000, 600), Mode::Percent(50), 500, 300, None);
    check((3, 3), Mode::Percent(50), 2, 2, None);
    check((1234, 567), Mode::Percent(50), 617, 284, None);
    check((640, 480), Mode::Percent(37), 237, 178, None);
    check((1920, 1080), Mode::Percent(100), 1920, 1080, None);
    check((1, 1000), Mode::Percent(1), 1, 10, None);
    check((1000, 1), Mode::Percent(1000), 10000, 10, None);
    check((1, 1), Mode::Percent(1000), 10, 10, None);
    check((6000, 4000), Mode::Percent(50), 3000, 2000, None);

    // LongestSide: the long edge lands on the value, the short edge is
    // rounded, and a value at or above the source keeps the source.
    check((1000, 1), Mode::LongestSide(1), 1, 1, None);
    check((6000, 4000), Mode::LongestSide(1920), 1920, 1280, None);
    check((333, 1000), Mode::LongestSide(100), 33, 100, None);
    check((1000, 500), Mode::LongestSide(3000), 1000, 500, None);
    check((10000, 10000), Mode::LongestSide(1920), 1920, 1920, None);

    // Fit: the fitted image size, never the box, and no enlargement.
    check(
        (1000, 500),
        Mode::Fit {
            width: 800,
            height: 800,
        },
        800,
        400,
        None,
    );
    check(
        (1000, 500),
        Mode::Fit {
            width: 200,
            height: 200,
        },
        200,
        100,
        None,
    );
    check(
        (4000, 3000),
        Mode::Fit {
            width: 1920,
            height: 1080,
        },
        1440,
        1080,
        None,
    );
    check(
        (1920, 1080),
        Mode::Fit {
            width: 1000,
            height: 1000,
        },
        1000,
        563,
        None,
    );
    check(
        (640, 480),
        Mode::Fit {
            width: 4000,
            height: 4000,
        },
        640,
        480,
        None,
    );
    check(
        (1234, 567),
        Mode::Fit {
            width: 1234,
            height: 567,
        },
        1234,
        567,
        None,
    );
    check(
        (1234, 567),
        Mode::Fit {
            width: 1233,
            height: 567,
        },
        1233,
        567,
        None,
    );
    check(
        (1, 1000),
        Mode::Fit {
            width: 100,
            height: 100,
        },
        1,
        100,
        None,
    );
    check(
        (999, 1),
        Mode::Fit {
            width: 100,
            height: 100,
        },
        100,
        1,
        None,
    );

    // Fill: the box, with the centred source crop that covers it.
    check(
        (1000, 1000),
        Mode::Fill {
            width: 800,
            height: 600,
        },
        800,
        600,
        Some((0, 125, 1000, 750)),
    );
    check(
        (1000, 1000),
        Mode::Fill {
            width: 600,
            height: 800,
        },
        600,
        800,
        Some((125, 0, 750, 1000)),
    );
    check(
        (1200, 800),
        Mode::Fill {
            width: 800,
            height: 800,
        },
        800,
        800,
        Some((200, 0, 800, 800)),
    );
    check(
        (640, 480),
        Mode::Fill {
            width: 100,
            height: 100,
        },
        100,
        100,
        Some((80, 0, 480, 480)),
    );
    check(
        (1000, 600),
        Mode::Fill {
            width: 300,
            height: 300,
        },
        300,
        300,
        Some((200, 0, 600, 600)),
    );
    check(
        (1000, 600),
        Mode::Fill {
            width: 600,
            height: 300,
        },
        600,
        300,
        Some((0, 50, 1000, 500)),
    );
    check(
        (1600, 800),
        Mode::Fill {
            width: 800,
            height: 400,
        },
        800,
        400,
        Some((0, 0, 1600, 800)),
    );
    check(
        (100, 100),
        Mode::Fill {
            width: 200,
            height: 200,
        },
        200,
        200,
        Some((0, 0, 100, 100)),
    );

    // Exact: the given size, up or down, with no crop.
    check(
        (1000, 600),
        Mode::Exact {
            width: 123,
            height: 456,
        },
        123,
        456,
        None,
    );
    check(
        (1, 1),
        Mode::Exact {
            width: 1920,
            height: 1080,
        },
        1920,
        1080,
        None,
    );
    check(
        (6000, 4000),
        Mode::Exact {
            width: 6000,
            height: 4000,
        },
        6000,
        4000,
        None,
    );
}

#[test]
fn scaled_modes_keep_the_aspect_ratio_within_one_pixel() {
    let sources = [
        (1u32, 1u32),
        (4, 3),
        (100, 100),
        (640, 480),
        (1000, 333),
        (1000, 1000),
        (1920, 1080),
        (6000, 4000),
    ];
    for (source_width, source_height) in sources {
        let modes = [
            Mode::Percent(1),
            Mode::Percent(37),
            Mode::Percent(50),
            Mode::Percent(100),
            Mode::LongestSide(1),
            Mode::LongestSide(100),
            Mode::LongestSide(1920),
            Mode::LongestSide(3000),
            Mode::Fit {
                width: 200,
                height: 200,
            },
            Mode::Fit {
                width: 1920,
                height: 1080,
            },
            Mode::Fit {
                width: 4000,
                height: 4000,
            },
        ];
        for mode in modes {
            let target = target((source_width, source_height), mode)
                .unwrap_or_else(|| panic!("no target for {source_width}x{source_height}"));
            assert!(
                target.width >= 1 && target.height >= 1,
                "1 px floor for {source_width}x{source_height}"
            );
            assert!(
                target.crop.is_none(),
                "only Fill crops, got a crop for {source_width}x{source_height}"
            );
            let ideal_height = target.width as f64 * source_height as f64 / source_width as f64;
            let ideal_width = target.height as f64 * source_width as f64 / source_height as f64;
            let height_error = (target.height as f64 - ideal_height).abs();
            let width_error = (target.width as f64 - ideal_width).abs();
            assert!(
                height_error <= 1.0 || width_error <= 1.0,
                "aspect {}x{} for {}x{}",
                target.width,
                target.height,
                source_width,
                source_height
            );
        }
    }
}

#[test]
fn fill_crops_the_source_centred_and_keeps_the_box_aspect() {
    let sources = [
        (1u32, 1u32),
        (4, 3),
        (1, 1000),
        (1000, 1),
        (640, 480),
        (1000, 333),
        (1000, 1000),
        (1200, 800),
        (1920, 1080),
    ];
    let boxes = [
        (1u32, 1u32),
        (100, 100),
        (50, 200),
        (800, 600),
        (600, 800),
        (1920, 1080),
    ];
    for (source_width, source_height) in sources {
        for (box_width, box_height) in boxes {
            let target = target(
                (source_width, source_height),
                Mode::Fill {
                    width: box_width,
                    height: box_height,
                },
            )
            .unwrap_or_else(|| panic!("no fill target for {source_width}x{source_height}"));
            assert_eq!(
                (target.width, target.height),
                (box_width, box_height),
                "Fill reports the box for {source_width}x{source_height}"
            );
            let crop = target.crop.as_ref().expect("Fill carries the source crop");
            assert!(
                crop.width >= 1 && crop.height >= 1,
                "a 1 px floor on the crop"
            );
            assert!(
                crop.x + crop.width <= source_width && crop.y + crop.height <= source_height,
                "crop inside the source for {source_width}x{source_height}"
            );
            assert!(
                ((2 * crop.x + crop.width) as i64 - source_width as i64).abs() <= 1,
                "crop centred on x for {source_width}x{source_height}"
            );
            assert!(
                ((2 * crop.y + crop.height) as i64 - source_height as i64).abs() <= 1,
                "crop centred on y for {source_width}x{source_height}"
            );
            let ideal_width = crop.height as f64 * box_width as f64 / box_height as f64;
            let ideal_height = crop.width as f64 * box_height as f64 / box_width as f64;
            let width_error = (crop.width as f64 - ideal_width).abs();
            let height_error = (crop.height as f64 - ideal_height).abs();
            assert!(
                width_error <= 1.0 || height_error <= 1.0,
                "crop aspect {}x{} for box {}x{}",
                crop.width,
                crop.height,
                box_width,
                box_height
            );
        }
    }
}

#[test]
fn zero_sources_and_zero_boxes_give_none() {
    assert!(target((0, 0), Mode::Percent(50)).is_none());
    assert!(target((0, 100), Mode::Percent(50)).is_none());
    assert!(target((100, 0), Mode::LongestSide(100)).is_none());
    assert!(
        target(
            (0, 100),
            Mode::Fit {
                width: 100,
                height: 100
            }
        )
        .is_none()
    );
    assert!(
        target(
            (100, 0),
            Mode::Fill {
                width: 10,
                height: 10
            }
        )
        .is_none()
    );
    assert!(
        target(
            (0, 0),
            Mode::Exact {
                width: 10,
                height: 10
            }
        )
        .is_none()
    );

    assert!(
        target(
            (100, 100),
            Mode::Fit {
                width: 0,
                height: 100
            }
        )
        .is_none()
    );
    assert!(
        target(
            (100, 100),
            Mode::Fit {
                width: 100,
                height: 0
            }
        )
        .is_none()
    );
    assert!(
        target(
            (100, 100),
            Mode::Fill {
                width: 0,
                height: 10
            }
        )
        .is_none()
    );
    assert!(
        target(
            (100, 100),
            Mode::Fill {
                width: 10,
                height: 0
            }
        )
        .is_none()
    );
    assert!(
        target(
            (100, 100),
            Mode::Exact {
                width: 0,
                height: 0
            }
        )
        .is_none()
    );
    assert!(
        target(
            (100, 100),
            Mode::Exact {
                width: 10,
                height: 0
            }
        )
        .is_none()
    );
}
