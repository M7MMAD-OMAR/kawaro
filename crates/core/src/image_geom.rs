//! Shared resize geometry (S-118).
//!
//! The stage 2 preview and the image engine both call `target`, so a preview
//! is never a pixel off the file the engine writes. The arithmetic is
//! integer: a product with the ratio, rounded half up, and never below one
//! pixel. A zero source side or a zero box gives None.
//!
//! The modes:
//!
//! - Percent: scale to a whole percent of the source.
//! - LongestSide: fit the longest side, never enlarging.
//! - Fit: contain in the box, never enlarging.
//! - Fill: cover the box, with a centred crop in source pixels.
//! - Exact: exactly the box, enlarging when needed.

/// How a target size relates to the source or to a box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Scale to a whole percent of the source, at least 1 px.
    Percent(u32),
    /// Fit the longest side to this many pixels, never enlarging.
    LongestSide(u32),
    /// Contain in this box, keeping the aspect ratio and never enlarging.
    Fit { width: u32, height: u32 },
    /// Cover this box; the result is the box and the crop says which part of
    /// the source to use.
    Fill { width: u32, height: u32 },
    /// Exactly this size, enlarging when needed.
    Exact { width: u32, height: u32 },
}

/// The part of the source a Mode::Fill target uses, in source pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Crop {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// The output size, plus the source crop when the mode crops.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target {
    pub width: u32,
    pub height: u32,
    pub crop: Option<Crop>,
}

/// The target size of `mode` for a `source` of `(width, height)` pixels.
pub fn target(source: (u32, u32), mode: Mode) -> Option<Target> {
    let (w, h) = source;
    if w == 0 || h == 0 {
        return None;
    }
    match mode {
        Mode::Percent(percent) => {
            if percent == 0 {
                return None;
            }
            Some(Target {
                width: round_ratio(w, percent, 100),
                height: round_ratio(h, percent, 100),
                crop: None,
            })
        }
        Mode::LongestSide(max) => {
            if max == 0 {
                return None;
            }
            if w.max(h) <= max {
                return Some(Target {
                    width: w,
                    height: h,
                    crop: None,
                });
            }
            if w >= h {
                Some(Target {
                    width: max,
                    height: round_ratio(h, max, w),
                    crop: None,
                })
            } else {
                Some(Target {
                    width: round_ratio(w, max, h),
                    height: max,
                    crop: None,
                })
            }
        }
        Mode::Fit { width, height } => {
            if width == 0 || height == 0 {
                return None;
            }
            if width >= w && height >= h {
                return Some(Target {
                    width: w,
                    height: h,
                    crop: None,
                });
            }
            // The box side that binds decides the ratio: compare the two
            // candidates by cross multiplying.
            if u64::from(width) * u64::from(h) <= u64::from(height) * u64::from(w) {
                Some(Target {
                    width,
                    height: round_ratio(h, width, w),
                    crop: None,
                })
            } else {
                Some(Target {
                    width: round_ratio(w, height, h),
                    height,
                    crop: None,
                })
            }
        }
        Mode::Fill { width, height } => {
            if width == 0 || height == 0 {
                return None;
            }
            // Cover: the box side with the larger ratio binds the scale.
            let (cw, ch) = if u64::from(width) * u64::from(h) >= u64::from(height) * u64::from(w) {
                (w, round_ratio(height, w, width).min(h))
            } else {
                (round_ratio(width, h, height).min(w), h)
            };
            Some(Target {
                width,
                height,
                crop: Some(Crop {
                    x: (w - cw) / 2,
                    y: (h - ch) / 2,
                    width: cw,
                    height: ch,
                }),
            })
        }
        Mode::Exact { width, height } => {
            if width == 0 || height == 0 {
                return None;
            }
            Some(Target {
                width,
                height,
                crop: None,
            })
        }
    }
}

/// `target` for a fractional percent, for a hand written action file whose
/// `scale` is not a whole number (the option schema's percent field is whole).
/// Same rounding and 1 px floor.
pub fn target_percent(source: (u32, u32), percent: f64) -> Option<Target> {
    let (w, h) = source;
    if w == 0 || h == 0 || !percent.is_finite() || percent <= 0.0 {
        return None;
    }
    let side = |v: u32| ((f64::from(v) * percent / 100.0).round() as u32).max(1);
    Some(Target {
        width: side(w),
        height: side(h),
        crop: None,
    })
}

/// The edge of the source a [`window`] is anchored to (S-127); the other
/// axis is centred.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    Center,
    Top,
    Bottom,
    Left,
    Right,
}

/// The largest window of `aspect` (`width:height`) inside `source`, placed
/// at `anchor`; `None` when a side or the aspect is zero.
pub fn window(source: (u32, u32), aspect: (u32, u32), anchor: Anchor) -> Option<Crop> {
    let (sw, sh) = source;
    let (num, den) = aspect;
    if sw == 0 || sh == 0 || num == 0 || den == 0 {
        return None;
    }
    // The source side that binds decides the window: compare the two
    // candidates by cross multiplying.
    let (width, height) = if u64::from(sw) * u64::from(den) >= u64::from(sh) * u64::from(num) {
        (round_ratio(sh, num, den).min(sw), sh)
    } else {
        (sw, round_ratio(sw, den, num).min(sh))
    };
    let (x, y) = match anchor {
        Anchor::Center => ((sw - width) / 2, (sh - height) / 2),
        Anchor::Top => ((sw - width) / 2, 0),
        Anchor::Bottom => ((sw - width) / 2, sh - height),
        Anchor::Left => (0, (sh - height) / 2),
        Anchor::Right => (sw - width, (sh - height) / 2),
    };
    Some(Crop {
        x,
        y,
        width,
        height,
    })
}

/// `round(value * numerator / denominator)` half up, at least one pixel.
fn round_ratio(value: u32, numerator: u32, denominator: u32) -> u32 {
    let n = u64::from(value) * u64::from(numerator);
    let d = u64::from(denominator);
    let rounded = (n + d / 2) / d;
    u32::try_from(rounded).unwrap_or(u32::MAX).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(source: (u32, u32), mode: Mode) -> (u32, u32) {
        let t = target(source, mode).expect("a target");
        (t.width, t.height)
    }

    #[test]
    fn percent_rounds_half_up_and_floors_at_one_pixel() {
        assert_eq!(size((64, 48), Mode::Percent(50)), (32, 24));
        assert_eq!(size((3, 3), Mode::Percent(150)), (5, 5));
        assert_eq!(size((10, 6), Mode::Percent(25)), (3, 2));
        assert_eq!(size((10, 10), Mode::Percent(1)), (1, 1));
        // 4.5 rounds up, 2.5 rounds up, 1.5 rounds up.
        assert_eq!(size((9, 5), Mode::Percent(50)), (5, 3));
        // 1000 percent is ten times the source.
        assert_eq!(size((1000, 500), Mode::Percent(1000)), (10000, 5000));
        // A 100 MP source stays a whole number of pixels.
        assert_eq!(size((10000, 10000), Mode::Percent(100)), (10000, 10000));
        assert_eq!(size((10000, 10000), Mode::Percent(50)), (5000, 5000));
    }

    #[test]
    fn odd_sizes_keep_the_aspect_with_one_pixel_edges() {
        // 1001 x 999 at 50 percent: 500.5 and 499.5 both round up.
        assert_eq!(size((1001, 999), Mode::Percent(50)), (501, 500));
        // A one pixel source stays one pixel.
        assert_eq!(size((1, 1), Mode::Percent(1)), (1, 1));
        assert_eq!(size((1, 900), Mode::Percent(10)), (1, 90));
        // A thin side never disappears.
        assert_eq!(size((997, 3), Mode::Percent(50)), (499, 2));
        // 3 * 50 / 100 = 1.5 rounds up, 1 * 50 = 0.5 rounds up to 1.
        assert_eq!(size((3, 1), Mode::Percent(50)), (2, 1));
    }

    #[test]
    fn longest_side_never_enlarges() {
        assert_eq!(size((64, 48), Mode::LongestSide(32)), (32, 24));
        assert_eq!(size((48, 64), Mode::LongestSide(32)), (24, 32));
        assert_eq!(size((64, 48), Mode::LongestSide(64)), (64, 48));
        assert_eq!(size((64, 48), Mode::LongestSide(63)), (63, 47));
        assert_eq!(size((100, 1), Mode::LongestSide(10)), (10, 1));
        // A small source is returned as it is.
        assert_eq!(size((20, 10), Mode::LongestSide(1000)), (20, 10));
        // A 100 MP source needed for a draft.
        assert_eq!(size((12000, 9000), Mode::LongestSide(1000)), (1000, 750));
    }

    #[test]
    fn fit_contains_in_the_box_without_enlarging() {
        assert_eq!(
            size(
                (100, 59),
                Mode::Fit {
                    width: 52,
                    height: 52
                }
            ),
            (52, 31)
        );
        assert_eq!(
            size(
                (59, 100),
                Mode::Fit {
                    width: 52,
                    height: 52
                }
            ),
            (31, 52)
        );
        // The wider box does not enlarge the source.
        assert_eq!(
            size(
                (40, 20),
                Mode::Fit {
                    width: 52,
                    height: 52
                }
            ),
            (40, 20)
        );
        // Width binds, then height binds.
        assert_eq!(
            size(
                (100, 100),
                Mode::Fit {
                    width: 50,
                    height: 200
                }
            ),
            (50, 50)
        );
        assert_eq!(
            size(
                (100, 100),
                Mode::Fit {
                    width: 200,
                    height: 50
                }
            ),
            (50, 50)
        );
        // A box larger on one side and smaller on the other still shrinks.
        assert_eq!(
            size(
                (100, 100),
                Mode::Fit {
                    width: 200,
                    height: 40
                }
            ),
            (40, 40)
        );
        // An exact aspect match keeps both sides.
        assert_eq!(
            size(
                (400, 300),
                Mode::Fit {
                    width: 200,
                    height: 150
                }
            ),
            (200, 150)
        );
    }

    #[test]
    fn fill_covers_the_box_and_crops_the_centre() {
        // A 4:3 source in a square box: keep the full height, crop the width.
        let t = target(
            (400, 300),
            Mode::Fill {
                width: 200,
                height: 200,
            },
        )
        .unwrap();
        assert_eq!((t.width, t.height), (200, 200));
        let crop = t.crop.unwrap();
        assert_eq!((crop.width, crop.height), (300, 300));
        assert_eq!((crop.x, crop.y), (50, 0));
        assert_eq!((crop.x + crop.width, crop.y + crop.height), (350, 300));

        // A 9:16 source in a square box: keep the full width, crop the height.
        let t = target(
            (900, 1600),
            Mode::Fill {
                width: 100,
                height: 100,
            },
        )
        .unwrap();
        assert_eq!((t.width, t.height), (100, 100));
        let crop = t.crop.unwrap();
        assert_eq!((crop.width, crop.height), (900, 900));
        assert_eq!((crop.x, crop.y), (0, 350));

        // A small source is enlarged to cover.
        let t = target(
            (100, 100),
            Mode::Fill {
                width: 200,
                height: 200,
            },
        )
        .unwrap();
        assert_eq!((t.width, t.height), (200, 200));
        let crop = t.crop.unwrap();
        assert_eq!(
            crop,
            Crop {
                x: 0,
                y: 0,
                width: 100,
                height: 100
            }
        );

        // An exact aspect match needs no crop at all.
        let t = target(
            (400, 300),
            Mode::Fill {
                width: 800,
                height: 600,
            },
        )
        .unwrap();
        assert_eq!((t.width, t.height), (800, 600));
        assert_eq!(
            t.crop,
            Some(Crop {
                x: 0,
                y: 0,
                width: 400,
                height: 300
            })
        );

        // A square crop of a square source is the whole source.
        let t = target(
            (101, 101),
            Mode::Fill {
                width: 50,
                height: 50,
            },
        )
        .unwrap();
        let crop = t.crop.unwrap();
        assert_eq!((crop.width, crop.height), (101, 101));
        assert_eq!((crop.x, crop.y), (0, 0));
    }

    #[test]
    fn exact_uses_the_box_and_enlarges() {
        assert_eq!(
            size(
                (10, 20),
                Mode::Exact {
                    width: 100,
                    height: 100
                }
            ),
            (100, 100)
        );
        assert_eq!(
            size(
                (10, 20),
                Mode::Exact {
                    width: 10,
                    height: 20
                }
            ),
            (10, 20)
        );
    }

    #[test]
    fn zero_source_or_zero_box_gives_none() {
        for mode in [
            Mode::Percent(50),
            Mode::LongestSide(100),
            Mode::Fit {
                width: 10,
                height: 10,
            },
            Mode::Fill {
                width: 10,
                height: 10,
            },
            Mode::Exact {
                width: 10,
                height: 10,
            },
        ] {
            assert_eq!(target((0, 10), mode), None, "{mode:?}");
            assert_eq!(target((10, 0), mode), None, "{mode:?}");
        }
        assert_eq!(target((10, 10), Mode::Percent(0)), None);
        assert_eq!(target((10, 10), Mode::LongestSide(0)), None);
        assert_eq!(
            target(
                (10, 10),
                Mode::Fit {
                    width: 0,
                    height: 10
                }
            ),
            None
        );
        assert_eq!(
            target(
                (10, 10),
                Mode::Fill {
                    width: 10,
                    height: 0
                }
            ),
            None
        );
        assert_eq!(
            target(
                (10, 10),
                Mode::Exact {
                    width: 0,
                    height: 10
                }
            ),
            None
        );
    }

    #[test]
    fn window_takes_the_largest_box_at_the_anchor() {
        assert_eq!(
            window((64, 48), (1, 1), Anchor::Center),
            Some(Crop {
                x: 8,
                y: 0,
                width: 48,
                height: 48
            })
        );
        assert_eq!(
            window((64, 48), (4, 3), Anchor::Center),
            Some(Crop {
                x: 0,
                y: 0,
                width: 64,
                height: 48
            })
        );
        // A 16:9 window of a 64 x 48 source is 64 x 36, centred at row 6.
        assert_eq!(
            window((64, 48), (16, 9), Anchor::Center),
            Some(Crop {
                x: 0,
                y: 6,
                width: 64,
                height: 36
            })
        );
        assert_eq!(
            window((48, 64), (1, 1), Anchor::Top),
            Some(Crop {
                x: 0,
                y: 0,
                width: 48,
                height: 48
            })
        );
        assert_eq!(
            window((48, 64), (1, 1), Anchor::Bottom),
            Some(Crop {
                x: 0,
                y: 16,
                width: 48,
                height: 48
            })
        );
        assert_eq!(
            window((64, 48), (1, 1), Anchor::Left),
            Some(Crop {
                x: 0,
                y: 0,
                width: 48,
                height: 48
            })
        );
        assert_eq!(
            window((64, 48), (1, 1), Anchor::Right),
            Some(Crop {
                x: 16,
                y: 0,
                width: 48,
                height: 48
            })
        );
        assert_eq!(window((0, 48), (1, 1), Anchor::Center), None);
        assert_eq!(window((48, 0), (1, 1), Anchor::Center), None);
        assert_eq!(window((48, 48), (0, 1), Anchor::Center), None);
        assert_eq!(window((48, 48), (1, 0), Anchor::Center), None);
    }

    #[test]
    fn target_percent_keeps_a_fractional_scale_exact() {
        let t = target_percent((64, 48), 50.0).unwrap();
        assert_eq!((t.width, t.height), (32, 24));
        let t = target_percent((100, 100), 12.5).unwrap();
        assert_eq!((t.width, t.height), (13, 13));
        let t = target_percent((10, 10), 12.0).unwrap();
        assert_eq!((t.width, t.height), (1, 1));
        assert_eq!(target_percent((0, 10), 50.0), None);
        assert_eq!(target_percent((10, 10), 0.0), None);
    }

    #[test]
    fn a_100_mp_source_has_a_cheap_target() {
        // 100 MP is 10000 x 10000: no overflow and no zero.
        let t = target((10000, 10000), Mode::LongestSide(1000)).unwrap();
        assert_eq!((t.width, t.height), (1000, 1000));
        let t = target(
            (12000, 9000),
            Mode::Fit {
                width: 1000,
                height: 1000,
            },
        )
        .unwrap();
        assert_eq!((t.width, t.height), (1000, 750));
    }
}
