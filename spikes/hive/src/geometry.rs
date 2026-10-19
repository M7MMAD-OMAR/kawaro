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

const DIRECTIONS: [Axial; 6] = [
    Axial { q: 1, r: 0 },
    Axial { q: 1, r: -1 },
    Axial { q: 0, r: -1 },
    Axial { q: -1, r: 0 },
    Axial { q: -1, r: 1 },
    Axial { q: 0, r: 1 },
];

/// Centre first, then each ring in order, so index 0 is always the middle cell.
pub fn spiral(rings: i32) -> Vec<Axial> {
    let mut out = vec![Axial { q: 0, r: 0 }];
    for k in 1..=rings {
        // Start at the cell `k` steps in direction 4, then walk the six sides.
        let mut cur = Axial {
            q: DIRECTIONS[4].q * k,
            r: DIRECTIONS[4].r * k,
        };
        for dir in DIRECTIONS {
            for _ in 0..k {
                out.push(cur);
                cur = Axial {
                    q: cur.q + dir.q,
                    r: cur.r + dir.r,
                };
            }
        }
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
}
