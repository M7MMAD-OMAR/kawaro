//! The work area under the pointer (S-046 AC1). Pure: the Windows window
//! lists the monitors and the cursor, this picks one.

/// A rectangle in virtual screen pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// The work area when no monitor is listed.
const DEFAULT: Rect = Rect {
    x: 0,
    y: 0,
    w: 1920,
    h: 1080,
};

/// Squared distance from `point` to the pixels of `r`, 0 inside it.
fn distance2(point: (i32, i32), r: &Rect) -> i64 {
    let gap = |p: i32, start: i32, len: i32| -> i64 {
        i64::from(start - p)
            .max(i64::from(p) - (i64::from(start) + i64::from(len) - 1))
            .max(0)
    };
    let (dx, dy) = (gap(point.0, r.x, r.w), gap(point.1, r.y, r.h));
    dx * dx + dy * dy
}

/// The monitor holding `point` (right and bottom edges exclusive), else the
/// nearest one, else a 1080p default for an empty list.
pub fn work_area_for(point: (i32, i32), monitors: &[Rect]) -> Rect {
    monitors
        .iter()
        .min_by_key(|r| distance2(point, r))
        .copied()
        .unwrap_or(DEFAULT)
}
