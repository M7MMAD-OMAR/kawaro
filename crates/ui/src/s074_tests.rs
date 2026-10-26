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
        })
        .collect()
}

/// Presents `files` (none: the plain hive) with `n` cells, in `scheme`
/// and `lang`, and returns (width, height, RGBA bytes, cell centres,
/// hive centre) in physical pixels.
type Shot = (u32, u32, Vec<u8>, Vec<(f32, f32)>, (f32, f32));

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
    // SAFETY: each test owns its thread and sets this before building.
    unsafe { std::env::set_var("KAWARO_COLOR_SCHEME", scheme) };
    let hive = Hive::new(lang).unwrap();
    let paths: Vec<PathBuf> = files.iter().map(PathBuf::from).collect();
    hive.present_files("centre", &paths, cells(n), None)
        .unwrap();
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
    (w, h, bytes, centres, (o.0 * scale, o.1 * scale))
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

/// The midpoint (and its 4 neighbours) between two neighbouring cell
/// centres is the gap between them: alpha 0.
fn gaps_clear(s: &Shot, what: &str) {
    let mut pts: Vec<(f32, f32)> = s.3.clone();
    pts.push(s.4);
    let d = |a: (f32, f32), b: (f32, f32)| ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
    let mut min = f32::MAX;
    for i in 0..pts.len() {
        for j in i + 1..pts.len() {
            min = min.min(d(pts[i], pts[j]));
        }
    }
    let mut gaps = 0;
    for i in 0..pts.len() {
        for j in i + 1..pts.len() {
            if d(pts[i], pts[j]) < min * 1.1 {
                gaps += 1;
                let (mx, my) = ((pts[i].0 + pts[j].0) / 2.0, (pts[i].1 + pts[j].1) / 2.0);
                for (ox, oy) in [(0.0, 0.0), (1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                    let a = alpha(s, mx + ox, my + oy);
                    assert_eq!(a, 0, "{what}: gap pixel near ({mx}, {my}) has alpha {a}");
                }
            }
        }
    }
    assert!(gaps >= 6, "{what}: found only {gaps} neighbouring pairs");
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
    gaps_clear(&s, what);
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
