//! Behaviour of the real hive window on Slint's headless testing platform:
//! no compositor, no display. One `#[test]` because the platform is
//! installed once per thread and the focus state is thread local.

use super::*;
use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{LogicalPosition, SharedString};

#[derive(Default)]
struct Log {
    chosen: RefCell<Vec<String>>,
    dismissed: Cell<u32>,
}

fn cells() -> Vec<HiveCell> {
    let c = |id: &str, icon: &str, ring| HiveCell {
        id: id.into(),
        code: id.to_uppercase(),
        label: format!("label {id}"),
        icon: icon.into(),
        ring,
    };
    // An icon, another icon, none, and a name outside the set.
    vec![
        c("webp", "image", 1),
        c("png", "convert", 1),
        c("jpg", "", 1),
        c("gray", "no-such-icon", 2),
    ]
}

fn key(hive: &Hive, k: Key) {
    let text: SharedString = k.into();
    let ui = hive.inner.ui.borrow();
    let w = ui.window();
    w.dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    w.dispatch_event(WindowEvent::KeyReleased { text });
}

fn click(hive: &Hive, x: f32, y: f32) {
    let ui = hive.inner.ui.borrow();
    let w = ui.window();
    let position = LogicalPosition::new(x, y);
    w.dispatch_event(WindowEvent::PointerMoved { position });
    w.dispatch_event(WindowEvent::PointerPressed {
        position,
        button: PointerEventButton::Left,
    });
    w.dispatch_event(WindowEvent::PointerReleased {
        position,
        button: PointerEventButton::Left,
    });
}

fn centre_of(hive: &Hive, index: usize) -> (f32, f32) {
    let plan = hive.inner.plan.borrow();
    let p = &plan.as_ref().unwrap().cells[index];
    (p.x, p.y)
}

fn origin(hive: &Hive) -> (f32, f32) {
    hive.inner.plan.borrow().as_ref().unwrap().layout.origin
}

fn wired(lang: Lang) -> (Hive, Rc<Log>) {
    let hive = Hive::new(lang).unwrap();
    let log = Rc::new(Log::default());
    let l = log.clone();
    hive.on_choose(move |id| l.chosen.borrow_mut().push(id));
    let l = log.clone();
    hive.on_dismiss(move || l.dismissed.set(l.dismissed.get() + 1));
    (hive, log)
}

#[test]
fn hive_behaviour() {
    i_slint_backend_testing::init_no_event_loop();
    let (hive, log) = wired(Lang::En);
    let ui = hive.inner.ui.borrow();
    // Before any present there is no plan, so nothing can be picked.
    assert_eq!(ui.invoke_pick(10.0, 10.0), -1);

    // present maps cells to the model and shows the window.
    hive.present("3 images", cells(), None).unwrap();
    assert!(hive.inner.visible());
    assert_eq!(ui.get_cells().row_count(), 4);
    assert_eq!(ui.get_centre_text(), "3 images");
    let first = ui.get_cells().row_data(0).unwrap();
    assert_eq!(
        (first.code.as_str(), first.label.as_str()),
        ("WEBP", "label webp")
    );
    // Known icons reach the cell as a loaded image; empty and unknown names
    // leave the code alone.
    let icons: Vec<(bool, u32)> = (0..4)
        .map(|i| {
            let c = ui.get_cells().row_data(i).unwrap();
            (c.has_icon, c.icon.size().width)
        })
        .collect();
    assert_eq!(icons, [(true, 24), (true, 24), (false, 0), (false, 0)]);
    assert!(
        first.x > ui.get_centre_x(),
        "LTR ring 1 starts right of the centre"
    );
    assert!(
        first.y < ui.get_centre_y(),
        "ring 1 starts above the centre"
    );

    // Esc closes and dismisses.
    key(&hive, Key::Escape);
    assert!(!hive.inner.visible());
    assert_eq!(log.dismissed.get(), 1);

    // Keyboard: Tab twice, then Enter chooses the second ring-1 cell.
    hive.present("3 images", cells(), None).unwrap();
    key(&hive, Key::Tab);
    assert_eq!(ui.get_highlight(), 0);
    key(&hive, Key::Tab);
    assert_eq!(ui.get_highlight(), 1);
    key(&hive, Key::Return);
    assert!(!hive.inner.visible());
    assert_eq!(*log.chosen.borrow(), ["png"]);

    // Arrows wrap; a fresh present clears the highlight.
    hive.present("3 images", cells(), None).unwrap();
    assert_eq!(ui.get_highlight(), -1);
    key(&hive, Key::LeftArrow);
    assert_eq!(ui.get_highlight(), 3);
    key(&hive, Key::Escape);
    assert_eq!(log.dismissed.get(), 2);

    // Mouse: hover highlights, click on a cell chooses it.
    hive.present("3 images", cells(), None).unwrap();
    let (x, y) = centre_of(&hive, 2);
    hive.inner
        .ui
        .borrow()
        .window()
        .dispatch_event(WindowEvent::PointerMoved {
            position: LogicalPosition::new(x, y),
        });
    assert_eq!(ui.get_highlight(), 2);
    click(&hive, x, y);
    assert_eq!(*log.chosen.borrow(), ["png", "jpg"]);
    assert!(!hive.inner.visible());

    // Click on the centre dismisses.
    hive.present("3 images", cells(), None).unwrap();
    let (ox, oy) = origin(&hive);
    click(&hive, ox, oy);
    assert_eq!(log.dismissed.get(), 3);
    assert!(!hive.inner.visible());

    // Click in the gap between the centre and the first cell dismisses.
    hive.present("3 images", cells(), None).unwrap();
    let (x, y) = centre_of(&hive, 0);
    click(&hive, (ox + x) / 2.0, (oy + y) / 2.0);
    assert_eq!(log.dismissed.get(), 4);

    // Focus loss dismisses only after the window had focus.
    hive.present("3 images", cells(), None).unwrap();
    focus_changed(false);
    assert!(
        hive.inner.visible(),
        "focus loss without prior focus keeps the hive"
    );
    focus_changed(true);
    focus_changed(false);
    assert!(!hive.inner.visible());
    assert_eq!(log.dismissed.get(), 5);

    // close() hides without callbacks, and a later focus loss is ignored.
    hive.present("3 images", cells(), None).unwrap();
    focus_changed(true);
    hive.close();
    focus_changed(false);
    assert!(!hive.inner.visible());
    assert_eq!(log.dismissed.get(), 5);
    assert_eq!(log.chosen.borrow().len(), 2);

    // Accessibility: each cell is a button labelled with its full name, and
    // its default action chooses it.
    hive.present("3 images", cells(), None).unwrap();
    let found: Vec<ElementHandle> =
        ElementHandle::find_by_accessible_label(&*ui, "label gray").collect();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].accessible_role(), Some(AccessibleRole::Button));
    found[0].invoke_accessible_default_action();
    assert_eq!(log.chosen.borrow().last().unwrap(), "gray");
    assert!(!hive.inner.visible());

    // More than 18 cells: 18 shown.
    let many: Vec<HiveCell> = (0..25)
        .map(|i| HiveCell {
            id: format!("a{i}"),
            code: format!("A{i}"),
            label: format!("action {i}"),
            icon: "image".into(),
            ring: if i < 10 { 1 } else { 2 },
        })
        .collect();
    hive.present("25", many, None).unwrap();
    assert_eq!(ui.get_cells().row_count(), 18);
    // The snapshot hook highlights; closing clears the highlight.
    hive.set_highlight(2);
    assert_eq!(ui.get_highlight(), 2);
    hive.close();
    assert_eq!(ui.get_highlight(), -1);
    drop(ui);
    drop(hive);

    // Arabic mirrors: ring 1 starts left of the centre.
    let (ar, ar_log) = wired(Lang::Ar);
    ar.present("3 صور", cells(), None).unwrap();
    let ui = ar.inner.ui.borrow();
    let first = ui.get_cells().row_data(0).unwrap();
    assert!(
        first.x < ui.get_centre_x(),
        "Arabic ring 1 starts left of the centre"
    );
    // Right arrow goes back in Arabic.
    key(&ar, Key::Tab);
    key(&ar, Key::RightArrow);
    assert_eq!(ui.get_highlight(), 3);
    ar.close();

    // Focus events of another window (the settings window in the daemon)
    // never count as the hive's: only the latest hive window does.
    let other = HiveWindow::new().unwrap();
    assert!(is_hive_window(Some(ar.inner.ui.borrow().window())));
    assert!(!is_hive_window(Some(other.window())));
    assert!(is_hive_window(None));
    drop(ui);

    // S-013: Slint applies the activation token only when a component creates
    // its window adapter, so a token builds a fresh, fully wired component;
    // without a token the window is reused.
    let builds = ar.inner.builds.get();
    ar.present("3 صور", cells(), None).unwrap();
    assert_eq!(ar.inner.builds.get(), builds, "no token, same component");
    let old = window_key(ar.inner.ui.borrow().window());
    ar.present("3 صور", cells(), Some("token-1".into()))
        .unwrap();
    assert_eq!(
        ar.inner.builds.get(),
        builds + 1,
        "a token builds a new one"
    );
    let fresh = window_key(ar.inner.ui.borrow().window());
    assert_ne!(fresh, old);
    assert!(ar.inner.visible());
    assert!(is_hive_window(Some(ar.inner.ui.borrow().window())));
    assert!(
        PENDING_TOKEN.lock().unwrap().is_none(),
        "the token never waits for a later window"
    );
    assert_eq!(ar.inner.ui.borrow().get_cells().row_count(), 4);
    let dismissed = ar_log.dismissed.get();
    key(&ar, Key::Escape);
    assert_eq!(
        ar_log.dismissed.get(),
        dismissed + 1,
        "the new window is wired"
    );
    assert!(!ar.inner.visible());

    // A window that outlives its hive answers navigation with -1.
    let orphan = ar.inner.ui.borrow().clone_strong();
    drop(ar);
    assert_eq!(orphan.invoke_navigate(0, 0), -1);
}

#[test]
fn colour_scheme_override() {
    assert_eq!(forced_dark(Some("light")), Some(false));
    assert_eq!(forced_dark(Some("dark")), Some(true));
    assert_eq!(forced_dark(Some("auto")), None);
    assert_eq!(forced_dark(None), None);
}

/// S-018c reference values for tests/overlay/kwin.sh input, which can only see
/// the daemon from outside. Own thread, so its own testing platform.
/// AC9: Right arrow then Enter on a fresh English hive chooses the first cell
/// (index 0), so for a PNG the script expects the first ring-1 action (WEBP).
/// AC8: a close request (what a click on the layer catcher sends) dismisses
/// exactly once, and a second one while hidden adds nothing; the script checks
/// the closed surface, no output and no click below, not the callback count.
#[test]
fn ac8_ac9_layer_input_reference() {
    i_slint_backend_testing::init_no_event_loop();
    let (hive, log) = wired(Lang::En);
    hive.present("photo.png", cells(), None).unwrap();
    key(&hive, Key::RightArrow);
    assert_eq!(hive.inner.ui.borrow().get_highlight(), 0);
    key(&hive, Key::Return);
    assert_eq!(*log.chosen.borrow(), ["webp"]);
    assert!(!hive.inner.visible());

    hive.present("photo.png", cells(), None).unwrap();
    let close = |h: &Hive| {
        h.inner
            .ui
            .borrow()
            .window()
            .dispatch_event(WindowEvent::CloseRequested)
    };
    close(&hive);
    assert!(!hive.inner.visible());
    assert_eq!(log.dismissed.get(), 1);
    close(&hive);
    assert_eq!(
        log.dismissed.get(),
        1,
        "a hidden hive is not dismissed again"
    );
    assert_eq!(log.chosen.borrow().len(), 1);
}

// ------------------------------------------------------------------------
// S-018h, AC24 and AC25: the selected files in the centre cell.
//
// Contract (written by the tester from the acceptance criteria; do not edit):
// - `Hive::set_preview_source(&self, thumbnails: PathBuf, globs: thumb::Globs)`:
//   where the hive looks for thumbnails (`thumb::find_thumbnail`) and file
//   kinds. The daemon and the examples pass `thumb::thumbnails_dir` and
//   `thumb::Globs::load(&thumb::data_dirs(..))` from their environment; these
//   tests pass a private cache, never the owner's.
// - `Hive::present_files(&self, centre: &str, files: &[PathBuf],
//   cells: Vec<HiveCell>, token: Option<String>) -> Result<(), PlatformError>`:
//   `present` with the selected files. `centre` is the daemon's
//   `text::selection` text. `present` keeps its signature and shows no preview.
// - `HiveWindow` properties: `preview-image` (the thumbnail, decoded and
//   downscaled in Rust; empty when there is none), `preview-icon` (the kind
//   icon when there is no thumbnail; empty otherwise), `preview-code` (the
//   format code drawn with the icon, `thumb::format_code`), `preview-count`
//   (number of files; 0 for `present`). Both images are emptied on every hide.
// - Elements in hive.slint, found by an id ending in `::preview` (the Image in
//   the square inscribed in the centre hexagon, `image-fit: contain`),
//   `::stack-1` and `::stack-2` (the two hexagons behind the centre) and
//   `::badge` (the count). "Not shown" means the element is absent (`if`),
//   has opacity 0, or has a zero size. `::centre` stays the centre HexCell.
// - The centre's accessible label: the file name for one file, `centre` for
//   several.

use crate::thumb::{Globs, Size, file_uri, find_thumbnail, thumbnail_path};
use i_slint_backend_testing::ElementRoot;
use std::path::Path;

/// A private directory under the system temp dir, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(test: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("sbarhive-s018h-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("files")).unwrap();
        Self(dir)
    }

    /// The thumbnails folder of the private cache (what `find_thumbnail` takes).
    fn thumbnails(&self) -> PathBuf {
        self.0.join("cache").join("thumbnails")
    }

    /// A selected file with some bytes in it.
    fn file(&self, name: &str) -> PathBuf {
        let p = self.0.join("files").join(name);
        std::fs::write(&p, b"the selected file").unwrap();
        p
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn mtime_secs(p: &Path) -> u64 {
    std::fs::metadata(p)
        .unwrap()
        .modified()
        .unwrap()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

const RED: [u8; 4] = [230, 20, 20, 255];
const BLUE: [u8; 4] = [20, 20, 230, 255];

/// A w x h RGBA thumbnail, left half red and right half blue, with the tEXt
/// chunks a thumbnailer writes. `mtime` is what goes into Thumb::MTime.
fn write_thumb(path: &Path, w: u32, h: u32, uri: &str, mtime: u64) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let f = std::fs::File::create(path).unwrap();
    let mut e = png::Encoder::new(std::io::BufWriter::new(f), w, h);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    e.add_text_chunk("Thumb::URI".into(), uri.into()).unwrap();
    e.add_text_chunk("Thumb::MTime".into(), mtime.to_string())
        .unwrap();
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for _ in 0..h {
        for x in 0..w {
            data.extend_from_slice(if x < w / 2 { &RED } else { &BLUE });
        }
    }
    e.write_header().unwrap().write_image_data(&data).unwrap();
}

/// A valid thumbnail of `file` in the `size` folder of the private cache.
fn thumb_of(s: &Scratch, file: &Path, size: Size, w: u32, h: u32) {
    let uri = file_uri(file);
    let path = thumbnail_path(&s.thumbnails(), size, &uri);
    write_thumb(&path, w, h, &uri, mtime_secs(file));
    // The fixture is one the S-018g lookup accepts.
    assert_eq!(find_thumbnail(&s.thumbnails(), file), Some(path));
}

fn fixture_globs() -> Globs {
    let mime = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/mime");
    Globs::load(&[mime.join("home"), mime.join("system")])
}

/// A wired hive reading the private cache, at `scale` physical pixels per
/// logical pixel.
fn preview_hive(lang: Lang, s: &Scratch, scale: f32) -> (Hive, Rc<Log>) {
    let (hive, log) = wired(lang);
    hive.set_preview_source(s.thumbnails(), fixture_globs());
    if scale != 1.0 {
        hive.inner
            .ui
            .borrow()
            .window()
            .dispatch_event(WindowEvent::ScaleFactorChanged {
                scale_factor: scale,
            });
    }
    (hive, log)
}

/// Ends the opening scale, so element boxes are where they stay.
fn settle(hive: &Hive) {
    hive.inner.ui.borrow().set_opened(true);
    i_slint_backend_testing::mock_elapsed_time(Duration::from_secs(1));
}

type Rect = (f32, f32, f32, f32);

/// Boxes (x, y, w, h, window coordinates) of the shown elements whose id ends
/// in `::name`.
fn shown(hive: &Hive, name: &str) -> Vec<Rect> {
    let suffix = format!("::{name}");
    let ui = hive.inner.ui.borrow();
    ui.root_element()
        .query_descendants()
        .match_predicate(move |e| e.id().is_some_and(|i| i.ends_with(&suffix)))
        .find_all()
        .into_iter()
        .filter(|e| {
            let s = e.size();
            e.computed_opacity() > 0.0 && s.width > 0.0 && s.height > 0.0
        })
        .map(|e| {
            let (p, s) = (e.absolute_position(), e.size());
            (p.x, p.y, s.width, s.height)
        })
        .collect()
}

fn one(hive: &Hive, name: &str) -> Rect {
    let found = shown(hive, name);
    assert_eq!(found.len(), 1, "exactly one shown `{name}`: {found:?}");
    found[0]
}

/// The square inscribed in the centre hexagon (pointy top, circumradius r):
/// side (3 - sqrt 3) r, centred on the hexagon.
fn inscribed_square(hive: &Hive) -> Rect {
    let (x, y, w, h) = one(hive, "centre");
    let r = h / 2.0;
    let side = (3.0 - 3f32.sqrt()) * r;
    (
        x + w / 2.0 - side / 2.0,
        y + h / 2.0 - side / 2.0,
        side,
        side,
    )
}

fn inside(inner: Rect, outer: Rect, tol: f32) -> bool {
    inner.0 >= outer.0 - tol
        && inner.1 >= outer.1 - tol
        && inner.0 + inner.2 <= outer.0 + outer.2 + tol
        && inner.1 + inner.3 <= outer.1 + outer.3 + tol
}

/// Physical side of the inscribed square at the window's scale factor.
fn physical_side(hive: &Hive) -> f32 {
    let ui = hive.inner.ui.borrow();
    (3.0 - 3f32.sqrt()) * ui.get_hex_r() * ui.window().scale_factor()
}

fn image_size(img: &slint::Image) -> (u32, u32) {
    let s = img.size();
    (s.width, s.height)
}

fn preview_sizes(hive: &Hive) -> ((u32, u32), (u32, u32)) {
    let ui = hive.inner.ui.borrow();
    (
        image_size(&ui.get_preview_image()),
        image_size(&ui.get_preview_icon()),
    )
}

/// The thumbnail reached Slint downscaled to the physical side of the
/// inscribed square or less (and not to a token size), with its 2:1 aspect.
fn assert_downscaled(hive: &Hive, what: &str) -> (u32, u32) {
    let ((w, h), icon) = preview_sizes(hive);
    let side = physical_side(hive);
    let long = w.max(h) as f32;
    assert!(
        long <= side.ceil() && long >= 0.75 * side,
        "{what}: thumbnail {w}x{h} should fit the physical square side {side:.1}"
    );
    assert!(
        w.abs_diff(2 * h) <= 2,
        "{what}: aspect 2:1 kept, got {w}x{h}"
    );
    assert_eq!(icon, (0, 0), "{what}: no kind icon over a thumbnail");
    (w, h)
}

#[test]
fn ac24_thumbnail_fills_the_inscribed_square_downscaled_to_its_physical_side() {
    i_slint_backend_testing::init_no_event_loop();
    let s = Scratch::new("thumb");
    let photo = s.file("photo.png");
    thumb_of(&s, &photo, Size::Large, 256, 128);
    let (hive, _log) = preview_hive(Lang::En, &s, 1.0);
    hive.present_files("1 image", std::slice::from_ref(&photo), cells(), None)
        .unwrap();
    settle(&hive);

    let (w, h) = assert_downscaled(&hive, "large 256x128");
    // The pixels are the thumbnail's: red on the left, blue on the right.
    let buf = hive
        .inner
        .ui
        .borrow()
        .get_preview_image()
        .to_rgba8()
        .expect("the preview is a decoded pixel image");
    let px = |x: u32, y: u32| {
        let p = buf.as_slice()[(y * w + x) as usize];
        [p.r, p.g, p.b]
    };
    let (l, r) = (px(1, h / 2), px(w - 2, h / 2));
    assert!(l[0] > 150 && l[2] < 100, "left is the red half: {l:?}");
    assert!(r[2] > 150 && r[0] < 100, "right is the blue half: {r:?}");

    // Shown in the square inscribed in the centre hexagon, filling it.
    let ui = hive.inner.ui.borrow();
    assert_eq!(ui.get_preview_count(), 1);
    drop(ui);
    let square = inscribed_square(&hive);
    let preview = one(&hive, "preview");
    assert!(
        inside(preview, square, 1.0),
        "preview {preview:?} inside the inscribed square {square:?}"
    );
    assert!(
        preview.2 >= square.2 - 2.0 && preview.3 >= square.3 - 2.0,
        "the preview fills the inscribed square: {preview:?} in {square:?}"
    );
    // One file: no stack and no count.
    for name in ["stack-1", "stack-2", "badge"] {
        assert!(shown(&hive, name).is_empty(), "one file shows no {name}");
    }
}

#[test]
fn ac24_downscale_uses_the_physical_side_at_scale_2() {
    i_slint_backend_testing::init_no_event_loop();
    let s = Scratch::new("scale2");
    let photo = s.file("photo.png");
    thumb_of(&s, &photo, Size::Large, 256, 128);
    let (hive, _log) = preview_hive(Lang::En, &s, 2.0);
    assert_eq!(hive.inner.ui.borrow().window().scale_factor(), 2.0);
    hive.present_files("1 image", std::slice::from_ref(&photo), cells(), None)
        .unwrap();
    let (w, _) = assert_downscaled(&hive, "scale 2");
    let logical = (3.0 - 3f32.sqrt()) * hive.inner.ui.borrow().get_hex_r();
    assert!(
        w as f32 > logical.ceil(),
        "at scale 2 the thumbnail is wider ({w}) than the logical side {logical:.1}"
    );
}

#[test]
fn ac24_xx_large_thumbnail_alone_is_found_and_downscaled() {
    i_slint_backend_testing::init_no_event_loop();
    let s = Scratch::new("xxlarge");
    let photo = s.file("photo.png");
    thumb_of(&s, &photo, Size::XxLarge, 1024, 512);
    let (hive, _log) = preview_hive(Lang::En, &s, 1.0);
    hive.present_files("1 image", std::slice::from_ref(&photo), cells(), None)
        .unwrap();
    assert_downscaled(&hive, "xx-large 1024x512");
}

#[test]
fn ac24_layout_is_unchanged_by_the_preview() {
    i_slint_backend_testing::init_no_event_loop();
    let s = Scratch::new("layout");
    let photo = s.file("photo.png");
    thumb_of(&s, &photo, Size::Large, 256, 128);
    let three = [s.file("a.png"), s.file("b.png"), s.file("c.png")];
    for lang in [Lang::En, Lang::Ar] {
        let (hive, _log) = preview_hive(lang, &s, 1.0);
        let geometry = |h: &Hive| {
            let ui = h.inner.ui.borrow();
            let cells: Vec<(f32, f32)> = (0..ui.get_cells().row_count())
                .map(|i| {
                    let c = ui.get_cells().row_data(i).unwrap();
                    (c.x, c.y)
                })
                .collect();
            (
                ui.get_win_w(),
                ui.get_win_h(),
                ui.get_hex_r(),
                ui.get_centre_x(),
                ui.get_centre_y(),
                cells,
            )
        };
        hive.present("1 image", cells(), None).unwrap();
        let plain = geometry(&hive);
        assert_eq!(hive.inner.ui.borrow().get_preview_count(), 0);
        hive.present_files("1 image", std::slice::from_ref(&photo), cells(), None)
            .unwrap();
        assert_eq!(geometry(&hive), plain, "{lang:?}: one file, same layout");
        assert_eq!(hive.inner.ui.borrow().get_preview_count(), 1);
        assert_ne!(
            preview_sizes(&hive).0,
            (0, 0),
            "{lang:?}: the thumbnail is there"
        );
        hive.present_files("3 images", &three, cells(), None)
            .unwrap();
        assert_eq!(geometry(&hive), plain, "{lang:?}: three files, same layout");
        assert_eq!(hive.inner.ui.borrow().get_preview_count(), 3);
    }
}

#[test]
fn ac24_hover_shows_the_name_instead_of_the_preview() {
    i_slint_backend_testing::init_no_event_loop();
    let s = Scratch::new("hover");
    let photo = s.file("photo.png");
    thumb_of(&s, &photo, Size::Large, 256, 128);
    let (hive, _log) = preview_hive(Lang::En, &s, 1.0);
    hive.present_files("1 image", std::slice::from_ref(&photo), cells(), None)
        .unwrap();
    settle(&hive);
    assert_eq!(
        shown(&hive, "preview").len(),
        1,
        "no cell under the pointer"
    );

    let (x, y) = centre_of(&hive, 0);
    let ui = hive.inner.ui.borrow();
    ui.window().dispatch_event(WindowEvent::PointerMoved {
        position: LogicalPosition::new(x, y),
    });
    assert_eq!(ui.get_highlight(), 0);
    drop(ui);
    assert!(
        shown(&hive, "preview").is_empty(),
        "a cell under the pointer: the centre shows its name, not the preview"
    );

    // Back on the centre (no cell): the preview again.
    let (ox, oy) = origin(&hive);
    let ui = hive.inner.ui.borrow();
    ui.window().dispatch_event(WindowEvent::PointerMoved {
        position: LogicalPosition::new(ox, oy),
    });
    assert_eq!(ui.get_highlight(), -1);
    drop(ui);
    assert_eq!(shown(&hive, "preview").len(), 1);
}

#[test]
fn ac24_preview_is_freed_on_every_hide() {
    i_slint_backend_testing::init_no_event_loop();
    let s = Scratch::new("freed");
    let photo = s.file("photo.png");
    thumb_of(&s, &photo, Size::Large, 256, 128);
    let doc = s.file("report.pdf");
    let (hive, log) = preview_hive(Lang::En, &s, 1.0);
    let empty = ((0, 0), (0, 0));

    hive.present_files("1 image", std::slice::from_ref(&photo), cells(), None)
        .unwrap();
    assert_ne!(preview_sizes(&hive).0, (0, 0));
    hive.close();
    assert_eq!(preview_sizes(&hive), empty, "close() frees the thumbnail");

    hive.present_files("1 image", std::slice::from_ref(&photo), cells(), None)
        .unwrap();
    key(&hive, Key::Escape);
    assert_eq!(log.dismissed.get(), 1);
    assert_eq!(preview_sizes(&hive), empty, "Esc frees the thumbnail");

    hive.present_files("1 image", std::slice::from_ref(&photo), cells(), None)
        .unwrap();
    let (x, y) = centre_of(&hive, 0);
    click(&hive, x, y);
    assert_eq!(log.chosen.borrow().len(), 1);
    assert_eq!(preview_sizes(&hive), empty, "a choice frees the thumbnail");

    hive.present_files("1 PDF", std::slice::from_ref(&doc), cells(), None)
        .unwrap();
    assert_ne!(preview_sizes(&hive).1, (0, 0));
    hive.close();
    assert_eq!(preview_sizes(&hive), empty, "close() frees the icon");

    // A plain present after a preview shows none.
    hive.present_files("1 image", std::slice::from_ref(&photo), cells(), None)
        .unwrap();
    hive.close();
    hive.present("3 images", cells(), None).unwrap();
    assert_eq!(preview_sizes(&hive), empty);
    assert_eq!(hive.inner.ui.borrow().get_preview_count(), 0);
}

#[test]
fn ac24_without_a_valid_thumbnail_the_kind_icon_and_format_code() {
    i_slint_backend_testing::init_no_event_loop();
    let s = Scratch::new("icon");
    let doc = s.file("report.pdf");
    let notes = s.file("notes.xyz");
    // A thumbnail written for an older version of the file.
    let stale = s.file("stale.png");
    let uri = file_uri(&stale);
    write_thumb(
        &thumbnail_path(&s.thumbnails(), Size::Large, &uri),
        256,
        128,
        &uri,
        mtime_secs(&stale) + 7,
    );
    // A thumbnailer failure record and no thumbnail.
    let failed = s.file("failed.png");
    let uri = file_uri(&failed);
    let name = thumbnail_path(&s.thumbnails(), Size::Normal, &uri);
    write_thumb(
        &s.thumbnails()
            .join("fail")
            .join("gnome-thumbnail-factory")
            .join(name.file_name().unwrap()),
        1,
        1,
        &uri,
        mtime_secs(&failed),
    );
    for f in [&doc, &notes, &stale, &failed] {
        assert_eq!(find_thumbnail(&s.thumbnails(), f), None, "{f:?}");
    }
    let (hive, _log) = preview_hive(Lang::En, &s, 1.0);
    for (file, code) in [
        (&doc, "PDF"),
        (&notes, "XYZ"),
        (&stale, "PNG"),
        (&failed, "PNG"),
    ] {
        hive.present_files("1 file", std::slice::from_ref(file), cells(), None)
            .unwrap();
        settle(&hive);
        let ((iw, ih), icon) = preview_sizes(&hive);
        assert_eq!((iw, ih), (0, 0), "{file:?}: no thumbnail");
        assert!(icon.0 > 0 && icon.1 > 0, "{file:?}: the kind icon");
        assert_eq!(hive.inner.ui.borrow().get_preview_code(), code);
        let square = inscribed_square(&hive);
        let preview = one(&hive, "preview");
        assert!(
            inside(preview, square, 1.0),
            "{file:?}: the icon {preview:?} inside the inscribed square {square:?}"
        );
        hive.close();
    }
}

/// Horizontal and vertical offset of an element's centre from the centre
/// hexagon's centre.
fn offset(hive: &Hive, name: &str) -> (f32, f32) {
    let c = one(hive, "centre");
    let e = one(hive, name);
    (
        e.0 + e.2 / 2.0 - (c.0 + c.2 / 2.0),
        e.1 + e.3 / 2.0 - (c.1 + c.3 / 2.0),
    )
}

#[test]
fn ac25_several_files_stack_two_hexagons_and_a_count_mirrored_in_arabic() {
    i_slint_backend_testing::init_no_event_loop();
    let s = Scratch::new("stack");
    let three = [s.file("a.png"), s.file("b.png"), s.file("c.png")];
    let mut offsets = Vec::new();
    for (lang, centre) in [(Lang::En, "3 images"), (Lang::Ar, "3 صور")] {
        let (hive, _log) = preview_hive(lang, &s, 1.0);
        hive.present_files(centre, &three, cells(), None).unwrap();
        settle(&hive);
        let ui = hive.inner.ui.borrow();
        assert_eq!(ui.get_preview_count(), 3, "{lang:?}");
        let window = (0.0, 0.0, ui.get_win_w(), ui.get_win_h());
        drop(ui);
        for name in ["stack-1", "stack-2", "badge"] {
            let r = one(&hive, name);
            assert!(
                inside(r, window, 0.5),
                "{lang:?}: {name} {r:?} has no pixel outside the hive {window:?}"
            );
        }
        let (d1, d2, badge) = (
            offset(&hive, "stack-1"),
            offset(&hive, "stack-2"),
            offset(&hive, "badge"),
        );
        assert!(
            d1.0.abs() >= 1.0,
            "{lang:?}: stack-1 is offset sideways {d1:?}"
        );
        assert!(
            d2.0.signum() == d1.0.signum() && d2.0.abs() > d1.0.abs(),
            "{lang:?}: stack-2 {d2:?} further than stack-1 {d1:?}, same side"
        );
        offsets.push((d1, d2, badge));
        hive.close();
    }
    let ((e1, e2, eb), (a1, a2, ab)) = (offsets[0], offsets[1]);
    for (what, en, ar) in [("stack-1", e1, a1), ("stack-2", e2, a2), ("badge", eb, ab)] {
        assert!(
            (en.0 + ar.0).abs() <= 1.0 && (en.1 - ar.1).abs() <= 1.0,
            "{what}: Arabic {ar:?} mirrors English {en:?}"
        );
    }
}

#[test]
fn ac25_centre_accessible_name_is_the_file_name_or_the_selection_text() {
    i_slint_backend_testing::init_no_event_loop();
    let s = Scratch::new("label");
    let photo = s.file("صورة 1.png");
    let (hive, _log) = preview_hive(Lang::Ar, &s, 1.0);
    let labelled = |h: &Hive, label: &str| -> Vec<Option<AccessibleRole>> {
        let ui = h.inner.ui.borrow();
        ElementHandle::find_by_accessible_label(&*ui, label)
            .map(|e| e.accessible_role())
            .collect()
    };

    hive.present_files("صورة واحدة", std::slice::from_ref(&photo), cells(), None)
        .unwrap();
    assert_eq!(
        labelled(&hive, "صورة 1.png"),
        [Some(AccessibleRole::Text)],
        "one file: the centre is named by the file name"
    );
    assert!(labelled(&hive, "صورة واحدة").is_empty());

    // Several files: the daemon's text::selection text, as given.
    for (n, text) in [(2, "ملفان"), (3, "3 ملفات"), (12, "12 ملفاً")] {
        let files: Vec<PathBuf> = (0..n).map(|i| s.file(&format!("f{i}.png"))).collect();
        hive.present_files(text, &files, cells(), None).unwrap();
        assert_eq!(
            labelled(&hive, text),
            [Some(AccessibleRole::Text)],
            "{n} files: the centre is named {text}"
        );
        assert_eq!(hive.inner.ui.borrow().get_preview_count(), n);
    }
}

// ------------------------------------------------------------------------
// S-018 review: tests that pin what the earlier ones left loose.

#[test]
fn ac24_several_files_share_a_format_code_only_when_all_agree() {
    i_slint_backend_testing::init_no_event_loop();
    let s = Scratch::new("shared-code");
    let (hive, _log) = preview_hive(Lang::En, &s, 1.0);
    let same = [s.file("a.png"), s.file("b.png"), s.file("c.png")];
    hive.present_files("3 images", &same, cells(), None)
        .unwrap();
    assert_eq!(
        hive.inner.ui.borrow().get_preview_code(),
        "PNG",
        "three PNG files show their shared code"
    );
    hive.close();
    let mixed = [s.file("d.png"), s.file("e.pdf")];
    hive.present_files("2 files", &mixed, cells(), None)
        .unwrap();
    assert_eq!(
        hive.inner.ui.borrow().get_preview_code(),
        "",
        "a PNG and a PDF share no code"
    );
    hive.close();
    let three = [s.file("f.png"), s.file("g.png"), s.file("h.pdf")];
    hive.present_files("3 files", &three, cells(), None)
        .unwrap();
    assert_eq!(hive.inner.ui.borrow().get_preview_code(), "");
}

/// The handed thumbnail's longer side is exactly the floor of the physical
/// inscribed square side (the source is larger, so `fit` reaches it).
fn assert_exact_side(hive: &Hive, what: &str) -> u32 {
    let ((w, h), _) = preview_sizes(hive);
    let side = physical_side(hive);
    assert_eq!(
        w.max(h),
        side.floor() as u32,
        "{what}: long side of {w}x{h} against physical side {side:.2} (hex-r {})",
        hive.inner.ui.borrow().get_hex_r()
    );
    w.max(h)
}

#[test]
fn ac24_thumbnail_long_side_is_the_floor_of_the_physical_side_at_scales_1_and_2() {
    i_slint_backend_testing::init_no_event_loop();
    let s = Scratch::new("exact-side");
    let photo = s.file("photo.png");
    thumb_of(&s, &photo, Size::Large, 256, 128);

    let (hive, _log) = preview_hive(Lang::En, &s, 1.0);
    hive.present_files("1 image", std::slice::from_ref(&photo), cells(), None)
        .unwrap();
    assert_eq!(assert_exact_side(&hive, "scale 1"), 52);
    drop(hive);

    let (hive, _log) = preview_hive(Lang::En, &s, 2.0);
    assert_eq!(hive.inner.ui.borrow().window().scale_factor(), 2.0);
    hive.present_files("1 image", std::slice::from_ref(&photo), cells(), None)
        .unwrap();
    let long = assert_exact_side(&hive, "scale 2");
    assert!(long >= 104, "scale 2 doubles the side: {long}");
}

#[test]
fn ac11_window_path_records_its_report_size() {
    i_slint_backend_testing::init_no_event_loop();
    let (hive, _log) = wired(Lang::En);
    WINDOW_REPORT.with(|c| c.set(None));
    hive.present("3 images", cells(), None).unwrap();
    assert!(
        !hive.inner.on_layer.get(),
        "the headless hive is on the window path"
    );
    let ui = hive.inner.ui.borrow();
    assert_eq!(
        WINDOW_REPORT.with(Cell::get),
        Some((ui.get_win_w(), ui.get_win_h())),
        "the window path owes a surface line for the hive's size"
    );
}
