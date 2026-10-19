//! Pointy-top hex layout in axial coordinates, and pixel picking.
//!
//! `radius` is the circumradius of a drawn cell. Cells are laid out on a grid
//! whose spacing leaves exactly `gap` pixels between neighbouring flat edges.
//! Picking returns a cell only when the point falls inside the drawn hexagon,
//! so the gaps and the transparent corners of the window select nothing.

const SQRT3: f32 = 1.732_050_8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Axial {
    pub q: i32,
    pub r: i32,
}

/// Top-right neighbour on screen (y grows downwards). Each ring starts at
/// `TOP_RIGHT * k`.
const TOP_RIGHT: Axial = Axial { q: 1, r: -1 };

/// The six sides of a ring, walked clockwise on screen from the top-right
/// corner: down to the right corner, then bottom right, bottom left, left,
/// top left and back to the top right.
const CLOCKWISE: [Axial; 6] = [
    Axial { q: 0, r: 1 },
    Axial { q: -1, r: 1 },
    Axial { q: -1, r: 0 },
    Axial { q: 0, r: -1 },
    Axial { q: 1, r: -1 },
    Axial { q: 1, r: 0 },
];

/// The `k`-th ring (6k cells), clockwise on screen from its top-right corner.
/// With `Layout::rtl` the same order is drawn mirrored: from the top-left
/// corner, counterclockwise.
pub fn ring(k: i32) -> Vec<Axial> {
    let mut out = Vec::with_capacity(6 * k.max(0) as usize);
    let mut cur = Axial {
        q: TOP_RIGHT.q * k,
        r: TOP_RIGHT.r * k,
    };
    for dir in CLOCKWISE {
        for _ in 0..k {
            out.push(cur);
            cur = Axial {
                q: cur.q + dir.q,
                r: cur.r + dir.r,
            };
        }
    }
    out
}

/// Centre first, then each ring in order, so index 0 is always the middle cell.
#[cfg(test)]
pub fn spiral(rings: i32) -> Vec<Axial> {
    let mut out = vec![Axial { q: 0, r: 0 }];
    for k in 1..=rings {
        out.extend(ring(k));
    }
    out
}

#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub radius: f32,
    pub gap: f32,
    /// Mirror horizontally for right-to-left interfaces.
    pub rtl: bool,
    /// Pixel position of the centre cell inside the window.
    pub origin: (f32, f32),
}

impl Layout {
    fn spacing(&self) -> f32 {
        self.radius + self.gap / SQRT3
    }

    pub fn center(&self, a: Axial) -> (f32, f32) {
        let s = self.spacing();
        let mut x = s * (SQRT3 * a.q as f32 + SQRT3 / 2.0 * a.r as f32);
        let y = s * 1.5 * a.r as f32;
        if self.rtl {
            x = -x;
        }
        (self.origin.0 + x, self.origin.1 + y)
    }

    /// Window size needed to hold `rings` rings around the centre, plus `pad`.
    pub fn extent(radius: f32, gap: f32, rings: i32, pad: f32) -> (f32, f32) {
        let s = radius + gap / SQRT3;
        let half_w = s * SQRT3 * rings as f32 + radius * SQRT3 / 2.0;
        let half_h = s * 1.5 * rings as f32 + radius;
        (2.0 * (half_w + pad), 2.0 * (half_h + pad))
    }

    /// Index into `cells` of the hexagon under the point, if any.
    pub fn pick(&self, cells: &[Axial], px: f32, py: f32) -> Option<usize> {
        let s = self.spacing();
        let mut x = px - self.origin.0;
        let y = py - self.origin.1;
        if self.rtl {
            x = -x;
        }
        let fq = (SQRT3 / 3.0 * x - y / 3.0) / s;
        let fr = (2.0 / 3.0 * y) / s;
        let hit = cube_round(fq, fr);
        let idx = cells.iter().position(|c| *c == hit)?;
        let (cx, cy) = self.center(hit);
        inside_hex(px - cx, py - cy, self.radius).then_some(idx)
    }
}

fn cube_round(fq: f32, fr: f32) -> Axial {
    let fs = -fq - fr;
    let (mut q, mut r, s) = (fq.round(), fr.round(), fs.round());
    let (dq, dr, ds) = ((q - fq).abs(), (r - fr).abs(), (s - fs).abs());
    if dq > dr && dq > ds {
        q = -r - s;
    } else if dr > ds {
        r = -q - s;
    }
    Axial {
        q: q as i32,
        r: r as i32,
    }
}

/// Point relative to the centre of a pointy-top hexagon with circumradius `r`.
fn inside_hex(dx: f32, dy: f32, r: f32) -> bool {
    let (dx, dy) = (dx.abs(), dy.abs());
    dx <= SQRT3 / 2.0 * r && dy <= r - dx / SQRT3
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(rtl: bool) -> Layout {
        Layout {
            radius: 40.0,
            gap: 6.0,
            rtl,
            origin: (200.0, 200.0),
        }
    }

    #[test]
    fn spiral_has_19_unique_cells_for_two_rings() {
        let cells = spiral(2);
        assert_eq!(cells.len(), 19);
        for (i, a) in cells.iter().enumerate() {
            assert!(!cells[i + 1..].contains(a), "duplicate {a:?}");
        }
        assert_eq!(cells[0], Axial { q: 0, r: 0 });
    }

    #[test]
    fn every_centre_picks_its_own_cell() {
        for rtl in [false, true] {
            let l = layout(rtl);
            let cells = spiral(2);
            for (i, c) in cells.iter().enumerate() {
                let (x, y) = l.center(*c);
                assert_eq!(l.pick(&cells, x, y), Some(i), "rtl={rtl} cell {i}");
            }
        }
    }

    #[test]
    fn points_just_inside_each_vertex_pick_the_cell() {
        let l = layout(false);
        let cells = spiral(2);
        for (i, c) in cells.iter().enumerate() {
            let (cx, cy) = l.center(*c);
            for k in 0..6 {
                let angle = (60.0 * k as f32 - 30.0).to_radians();
                let rr = l.radius - 1.0;
                let (x, y) = (cx + rr * angle.cos(), cy + rr * angle.sin());
                assert_eq!(l.pick(&cells, x, y), Some(i), "cell {i} vertex {k}");
            }
        }
    }

    #[test]
    fn gaps_and_window_corners_pick_nothing() {
        let l = layout(false);
        let cells = spiral(2);
        // Midpoint between the centre and its right neighbour sits in the gap.
        let (ax, ay) = l.center(cells[0]);
        let (bx, by) = l.center(Axial { q: 1, r: 0 });
        assert_eq!(l.pick(&cells, (ax + bx) / 2.0, (ay + by) / 2.0), None);
        // Just outside the top vertex of the centre cell.
        assert_eq!(l.pick(&cells, ax, ay - l.radius - 0.5), None);
        // Window corner.
        assert_eq!(l.pick(&cells, 0.0, 0.0), None);
    }

    #[test]
    fn rtl_mirrors_horizontally() {
        let a = Axial { q: 1, r: 0 };
        let (lx, ly) = layout(false).center(a);
        let (rx, ry) = layout(true).center(a);
        assert!((lx - 200.0 + (rx - 200.0)).abs() < 1e-3);
        assert_eq!(ly, ry);
    }

    /// Screen angle of a cell centre around the origin, measured clockwise
    /// from 12 o'clock (y grows downwards on screen), in degrees 0..360.
    fn clock_angle(l: &Layout, a: Axial) -> f32 {
        let (x, y) = l.center(a);
        let (dx, dy) = (x - l.origin.0, y - l.origin.1);
        dx.atan2(-dy).to_degrees().rem_euclid(360.0)
    }

    #[test]
    fn ring_one_starts_top_right_and_runs_clockwise() {
        let cells = spiral(1);
        let want = [(1, -1), (1, 0), (0, 1), (-1, 1), (-1, 0), (0, -1)];
        let got: Vec<(i32, i32)> = cells[1..].iter().map(|a| (a.q, a.r)).collect();
        assert_eq!(got, want);
        // Top right on screen: right of and above the centre.
        let l = layout(false);
        let (x, y) = l.center(cells[1]);
        assert!(x > l.origin.0 && y < l.origin.1);
    }

    #[test]
    fn every_ring_runs_clockwise_from_its_top_right_corner() {
        let l = layout(false);
        for k in 1..=3 {
            let cells = ring(k);
            assert_eq!(cells.len(), 6 * k as usize, "ring {k} size");
            assert_eq!(cells[0], Axial { q: k, r: -k }, "ring {k} start");
            let angles: Vec<f32> = cells.iter().map(|a| clock_angle(&l, *a)).collect();
            assert!(
                (angles[0] - 30.0).abs() < 1e-2,
                "ring {k} start not top right"
            );
            // Clockwise from the start: the angle swept from the first cell
            // grows along the ring and stays below a full turn.
            let swept: Vec<f32> = angles
                .iter()
                .map(|a| (a - angles[0]).rem_euclid(360.0))
                .collect();
            for w in swept.windows(2) {
                assert!(w[1] > w[0], "ring {k} not clockwise: {angles:?}");
            }
        }
    }

    #[test]
    fn spiral_is_centre_then_rings_in_order() {
        let s = spiral(2);
        assert_eq!(s[0], Axial { q: 0, r: 0 });
        assert_eq!(&s[1..7], ring(1).as_slice());
        assert_eq!(&s[7..19], ring(2).as_slice());
        assert_eq!(spiral(0), vec![Axial { q: 0, r: 0 }]);
    }

    #[test]
    fn arabic_mirrors_the_order_to_start_top_left_counterclockwise() {
        let l = layout(true);
        let cells = ring(1);
        let (x, y) = l.center(cells[0]);
        assert!(
            x < l.origin.0 && y < l.origin.1,
            "ring 1 must start top left in RTL"
        );
        let angles: Vec<f32> = cells.iter().map(|a| clock_angle(&l, *a)).collect();
        for w in angles.windows(2) {
            assert!(w[1] < w[0], "RTL ring not counterclockwise: {angles:?}");
        }
    }

    #[test]
    fn pick_ignores_cells_that_are_not_placed() {
        let l = layout(false);
        let placed = vec![Axial { q: 0, r: 0 }, Axial { q: 1, r: -1 }];
        let (x, y) = l.center(Axial { q: 1, r: 0 });
        assert_eq!(l.pick(&placed, x, y), None);
        let (x, y) = l.center(Axial { q: 1, r: -1 });
        assert_eq!(l.pick(&placed, x, y), Some(1));
    }

    #[test]
    fn extent_grows_with_rings_and_padding() {
        let (w1, h1) = Layout::extent(40.0, 6.0, 1, 4.0);
        let (w2, h2) = Layout::extent(40.0, 6.0, 2, 4.0);
        assert!(w2 > w1 && h2 > h1);
        let s = 40.0 + 6.0 / SQRT3;
        assert!((w1 - 2.0 * (s * SQRT3 + 40.0 * SQRT3 / 2.0 + 4.0)).abs() < 1e-3);
        assert!((h1 - 2.0 * (s * 1.5 + 40.0 + 4.0)).abs() < 1e-3);
        // Every cell of every ring fits inside the window.
        let l = Layout {
            radius: 40.0,
            gap: 6.0,
            rtl: false,
            origin: (w2 / 2.0, h2 / 2.0),
        };
        for a in spiral(2) {
            let (x, y) = l.center(a);
            assert!(x - 40.0 * SQRT3 / 2.0 >= 0.0 && x + 40.0 * SQRT3 / 2.0 <= w2);
            assert!(y - 40.0 >= 0.0 && y + 40.0 <= h2);
        }
    }
}
