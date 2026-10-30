//! S-074: the hive has no background plate. Renders the real hive window
//! with Slint's headless testing platform and reads the alpha of the
//! pixels. One `#[test]` per criterion; each runs on its own thread, which
//! is what the testing platform needs.

use super::*;
use slint::PlatformError;
use slint::platform::software_renderer::{
    MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType, TargetPixel,
};
use slint::platform::{Platform, WindowAdapter};
use std::path::PathBuf;

fn cells(n: usize) -> Vec<HiveCell> {
    (0..n)
        .map(|i| HiveCell {
            id: format!("c{i}"),
            code: format!("C{i}"),
            label: format!("label {i}"),
            icon: "image".into(),
            ring: if i < 6 { 1 } else { 2 },
            enabled: true,
            reason: String::new(),
        })
        .collect()
}

/// Presents `files` (none: the plain hive) with `n` cells, in `scheme`
/// and `lang`, and returns (width, height, RGBA bytes, cell centres,
/// hive centre) in physical pixels.
type Shot = (u32, u32, Vec<u8>, Vec<(f32, f32)>, (f32, f32), f32, bool);

thread_local! {
    static WINDOW: RefCell<Option<Rc<MinimalSoftwareWindow>>> = const { RefCell::new(None) };
}

/// A platform whose windows draw with the software renderer into a buffer
/// we own (the testing platform cannot take snapshots).
struct SoftPlatform;

impl Platform for SoftPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        let w = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
        WINDOW.with(|c| *c.borrow_mut() = Some(w.clone()));
        Ok(w)
    }
}

#[derive(Clone, Copy, Default)]
struct Px {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl TargetPixel for Px {
    fn blend(&mut self, c: PremultipliedRgbaColor) {
        let keep = u16::from(255 - c.alpha);
        let mix = |d: u8, s: u8| (u16::from(d) * keep / 255) as u8 + s;
        self.r = mix(self.r, c.red);
        self.g = mix(self.g, c.green);
        self.b = mix(self.b, c.blue);
        self.a = mix(self.a, c.alpha);
    }
    fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
    fn background() -> Self {
        Self::default()
    }
}

fn render(scheme: &str, lang: Lang, files: &[&str], n: usize) -> Shot {
    slint::platform::set_platform(Box::new(SoftPlatform)).ok();
    let hive = test_env::build(&[("KAWARO_COLOR_SCHEME", scheme)], lang);
    let paths: Vec<PathBuf> = files.iter().map(PathBuf::from).collect();
    hive.present_files("centre", &paths, cells(n), None)
        .unwrap();
    // S-017 (manager decision 3): the ring-by-ring open starts hidden, so
    // the S-074 probes read the settled hive. The assertions are unchanged.
    hive.inner.ui.borrow().set_opened(true);
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(500));
    let win = WINDOW
        .with(|c| c.borrow().clone())
        .expect("the hive made a window");
    let (lw, lh) = {
        let ui = hive.inner.ui.borrow();
        (ui.get_win_w(), ui.get_win_h())
    };
    let scale = 1.0_f32;
    win.set_size(slint::PhysicalSize::new(lw as u32, lh as u32));
    win.dispatch_event(slint::platform::WindowEvent::ScaleFactorChanged {
        scale_factor: scale,
    });
    let (w, h) = (lw as u32, lh as u32);
    let mut buf = vec![Px::default(); (w * h) as usize];
    win.request_redraw();
    let drew = win.draw_if_needed(|r| {
        r.render(&mut buf, w as usize);
    });
    assert!(drew, "the hive window drew nothing");
    let plan = hive.inner.plan.borrow();
    let plan = plan.as_ref().unwrap();
    let centres = plan
        .cells
        .iter()
        .map(|c| (c.x * scale, c.y * scale))
        .collect();
    let o = plan.layout.origin;
    let bytes = buf.iter().flat_map(|p| [p.r, p.g, p.b, p.a]).collect();
    (
        w,
        h,
        bytes,
        centres,
        (o.0 * scale, o.1 * scale),
        plan.layout.radius * scale,
        plan.layout.rtl,
    )
}

fn alpha(s: &Shot, x: f32, y: f32) -> u8 {
    let (w, h) = (s.0 as i64, s.1 as i64);
    let (x, y) = (
        (x.round() as i64).clamp(0, w - 1),
        (y.round() as i64).clamp(0, h - 1),
    );
    s.2[((y * w + x) * 4 + 3) as usize]
}

/// Every pixel of the 6x6 block in each window corner is transparent.
fn corners_clear(s: &Shot, what: &str) {
    let (w, h) = (s.0 as f32, s.1 as f32);
    for (cx, cy) in [
        (0.0, 0.0),
        (w - 6.0, 0.0),
        (0.0, h - 6.0),
        (w - 6.0, h - 6.0),
    ] {
        for dy in 0..6 {
            for dx in 0..6 {
                let a = alpha(s, cx + dx as f32, cy + dy as f32);
                assert_eq!(
                    a,
                    0,
                    "{what}: corner pixel ({}, {}) has alpha {a}",
                    cx + dx as f32,
                    cy + dy as f32
                );
            }
        }
    }
}

/// Cell edge ring (`reach` in hex-cell.slint) plus the 1 px of anti-aliasing
/// the spec allows beyond it. Manager decision D2: the ring and the stack
/// hexagons are part of the cell look; only what lies outside them is the
/// plate's territory.
const GROW: f32 = 3.0;
/// `stack-offset` in design/tokens.json.
const STACK_OFFSET: f32 = 4.0;
/// `badge-size` in design/tokens.json; the badge is grown generously.
const BADGE: f32 = 18.0;

/// True when (x, y) is inside the pointy top hexagon of circumradius `r`
/// at `c`, grown by `grow` on every side.
fn in_hex(c: (f32, f32), r: f32, grow: f32, x: f32, y: f32) -> bool {
    let (dx, dy) = ((x - c.0).abs(), (y - c.1).abs());
    let a = r * 3.0_f32.sqrt() / 2.0 + grow;
    // Grown flat sides: apothem a; the points reach a / cos 30 along y.
    dx <= a && dx * 0.5 + dy * 3.0_f32.sqrt() / 2.0 <= a
}

/// Sample points of the window area outside the union of every hexagon the
/// hive draws (cells, centre, stack hexagons) grown by `GROW`, and outside
/// the count badge. This is exactly where the old plate painted. All must
/// be alpha 0, and there must be at least 20 of them.
fn gaps_clear(s: &Shot, files: usize, what: &str) {
    let r = s.5;
    let side = if s.6 { -1.0 } else { 1.0 };
    let mut hexes: Vec<(f32, f32)> = s.3.clone();
    hexes.push(s.4);
    if files > 1 {
        hexes.push((s.4.0 + side * STACK_OFFSET, s.4.1 - STACK_OFFSET));
    }
    if files > 2 {
        hexes.push((
            s.4.0 + side * 2.0 * STACK_OFFSET,
            s.4.1 - 2.0 * STACK_OFFSET,
        ));
    }
    let badge = (
        s.4.0 + side * r * 1.2679492 / 2.0,
        s.4.1 - r * 1.2679492 / 2.0,
    );
    let mut checked = 0;
    for y in 0..s.1 {
        for x in 0..s.0 {
            let (fx, fy) = (x as f32, y as f32);
            if hexes.iter().any(|&c| in_hex(c, r, GROW, fx, fy)) {
                continue;
            }
            if files > 1 && (fx - badge.0).abs() <= BADGE && (fy - badge.1).abs() <= BADGE {
                continue;
            }
            checked += 1;
            let a = alpha(s, fx, fy);
            assert_eq!(
                a, 0,
                "{what}: pixel ({x}, {y}) outside every cell has alpha {a}"
            );
        }
    }
    assert!(
        checked >= 20,
        "{what}: only {checked} points outside every cell"
    );
}

fn centres_opaque(s: &Shot, what: &str) {
    let mut pts = s.3.clone();
    pts.push(s.4);
    for (x, y) in pts {
        let a = alpha(s, x, y);
        assert_eq!(a, 255, "{what}: cell centre ({x}, {y}) has alpha {a}");
    }
}

fn check(scheme: &str, lang: Lang, files: &[&str], n: usize, what: &str) {
    let s = render(scheme, lang, files, n);
    corners_clear(&s, what);
    gaps_clear(&s, files.len(), what);
    centres_opaque(&s, what);
}

#[test]
fn ac2_eight_cells_corners_and_gaps_clear_cell_centres_opaque() {
    check("dark", Lang::En, &[], 8, "8 cells, no file");
}

#[test]
fn ac3_one_file_light_ltr() {
    check(
        "light",
        Lang::En,
        &["/nonexistent/a.png"],
        9,
        "1 file light en",
    );
}

#[test]
fn ac3_one_file_dark_rtl() {
    check(
        "dark",
        Lang::Ar,
        &["/nonexistent/a.png"],
        9,
        "1 file dark ar",
    );
}

#[test]
fn ac3_several_files_light_rtl() {
    let f = [
        "/nonexistent/a.png",
        "/nonexistent/b.png",
        "/nonexistent/c.png",
    ];
    check("light", Lang::Ar, &f, 9, "3 files light ar");
}

#[test]
fn ac3_several_files_dark_ltr() {
    let f = [
        "/nonexistent/a.png",
        "/nonexistent/b.png",
        "/nonexistent/c.png",
    ];
    check("dark", Lang::En, &f, 9, "3 files dark en");
}

#[test]
fn ac1_hive_slint_has_no_plate_and_a_transparent_window() {
    let src = include_str!("../ui/hive.slint");
    assert!(
        !src.contains("Tokens.hive-plate"),
        "hive.slint still uses Tokens.hive-plate"
    );
    let window_bg: Vec<&str> = src
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("background:"))
        .collect();
    assert!(
        window_bg
            .first()
            .is_some_and(|l| *l == "background: transparent;"),
        "the first (root Window) background is not transparent: {window_bg:?}"
    );
}

#[test]
fn ac7_window_path_requests_an_alpha_surface() {
    // The window is built with winit's transparent attribute, or Slint
    // sets it for a transparent root background; either way the source
    // that builds the window must say so.
    let lib = include_str!("lib.rs");
    let plat = include_str!("platform.rs");
    let win = include_str!("win_window.rs");
    let all = format!("{lib}\n{plat}\n{win}");
    assert!(
        all.contains("with_transparent(true)") || all.contains("set_transparent(true)"),
        "no code asks winit for a transparent window (with_transparent(true))"
    );
}
