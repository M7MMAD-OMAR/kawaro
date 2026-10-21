//! S-014b: the drop hive on the main thread, on Slint's headless testing
//! platform. `on_note` stands in for the Wayland receiver thread. One
//! `#[test]` because the platform is installed once per thread.

use super::*;
use receiver::Note;

#[derive(Default)]
struct Log {
    chosen: RefCell<Vec<String>>,
    dismissed: Cell<u32>,
    entered: RefCell<Vec<Vec<PathBuf>>>,
}

fn cells() -> Vec<HiveCell> {
    ["webp", "jpg", "gray"]
        .iter()
        .map(|id| HiveCell {
            id: (*id).into(),
            code: id.to_uppercase(),
            label: format!("label {id}"),
            icon: String::new(),
            ring: 1,
        })
        .collect()
}

fn size(hive: &Hive) -> (f32, f32) {
    let ui = hive.inner.ui.borrow();
    (ui.get_win_w(), ui.get_win_h())
}

fn state(hive: &Hive) -> dnd::State {
    hive.inner.machine.borrow().state()
}

#[test]
fn drop_hive_behaviour() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = Hive::new(Lang::En).unwrap();
    let log = Rc::new(Log::default());
    let l = log.clone();
    hive.on_choose(move |id| l.chosen.borrow_mut().push(id));
    let l = log.clone();
    hive.on_dismiss(move || l.dismissed.set(l.dismissed.get() + 1));
    let l = log.clone();
    hive.on_drop_files(move |files| {
        l.entered.borrow_mut().push(files.clone());
        if files.is_empty() {
            (String::new(), Vec::new())
        } else {
            ("1 image".into(), cells())
        }
    });
    let png = PathBuf::from("/tmp/a.png");

    // Empty two-ring hive; the cells arrive on enter without a resize.
    hive.present_drop("", None).unwrap();
    assert!(hive.inner.visible());
    assert_eq!(state(&hive), dnd::State::Waiting);
    assert_eq!(hive.inner.ui.borrow().get_cells().row_count(), 0);
    let before = size(&hive);
    on_note(Note::Enter(vec![png.clone()]));
    assert_eq!(log.entered.borrow().as_slice(), [vec![png.clone()]]);
    assert_eq!(size(&hive), before);
    assert_eq!(hive.inner.ui.borrow().get_cells().row_count(), 3);
    assert_eq!(hive.inner.ui.borrow().get_centre_text().as_str(), "1 image");

    // Focus loss does not close a waiting hive.
    focus_changed(true);
    focus_changed(false);
    assert!(hive.inner.visible());
    assert_eq!(log.dismissed.get(), 0);

    // Hover highlights, a drop on a cell runs its action only.
    on_note(Note::Motion(Some(1)));
    assert_eq!(hive.inner.ui.borrow().get_highlight(), 1);
    assert_eq!(state(&hive), dnd::State::Over(1));
    on_note(Note::Motion(None));
    assert_eq!(hive.inner.ui.borrow().get_highlight(), -1);
    on_note(Note::Motion(Some(0)));
    on_note(Note::Drop);
    assert_eq!(log.chosen.borrow().as_slice(), ["webp"]);
    assert_eq!(log.dismissed.get(), 0);
    assert!(!hive.inner.visible());
    // A leave after the drop changes nothing.
    on_note(Note::Leave);
    assert_eq!(log.dismissed.get(), 0);

    // A refused drop (off the cells or on the centre) closes and runs nothing.
    hive.present_drop("", None).unwrap();
    on_note(Note::Enter(vec![png.clone()]));
    on_note(Note::Motion(Some(2)));
    on_note(Note::Motion(None));
    on_note(Note::Drop);
    assert_eq!(log.chosen.borrow().len(), 1);
    assert_eq!(log.dismissed.get(), 1);
    assert!(!hive.inner.visible());

    // Leave after enter closes; leave before enter does not.
    hive.present_drop("", None).unwrap();
    on_note(Note::Leave);
    assert!(hive.inner.visible());
    on_note(Note::Enter(Vec::new()));
    assert_eq!(hive.inner.ui.borrow().get_cells().row_count(), 0);
    on_note(Note::Leave);
    assert_eq!(log.dismissed.get(), 2);
    assert!(!hive.inner.visible());

    // A normal hive ignores the receiver and still closes on focus loss.
    let entered = log.entered.borrow().len();
    hive.present("3 images", cells(), None).unwrap();
    on_note(Note::Enter(vec![png.clone()]));
    on_note(Note::Motion(Some(0)));
    on_note(Note::Drop);
    assert_eq!(log.entered.borrow().len(), entered);
    assert_eq!(log.chosen.borrow().len(), 1);
    assert!(hive.inner.visible());
    focus_changed(true);
    focus_changed(false);
    assert_eq!(log.dismissed.get(), 3);
    assert!(!hive.inner.visible());

    // The enter timeout closes a hive the drag never reached.
    hive.present_drop("", None).unwrap();
    hive.inner.drag(dnd::Event::Tick);
    assert!(hive.inner.visible());
    let late = Instant::now() + dnd::ENTER_TIMEOUT;
    let s = hive
        .inner
        .machine
        .borrow_mut()
        .handle(dnd::Event::Tick, late);
    assert_eq!(s, dnd::State::Hidden);
}

// ------------------------------------------------------------------------
// S-019e (backlog/S-019-hive-at-pointer.md, AC12 and the manager's decision
// 3): a drop show made by a held key closes when the key is released.
//
// Contract (written by the tester from the acceptance criteria; do not edit):
// - `Hive::present_drop_hold(&self, centre: &str, token: Option<String>,
//   at: Option<(f64, f64)>) -> Result<(), slint::PlatformError>`: the drop
//   show the daemon makes when the long press gate's threshold has passed
//   (AC17), `present_drop_at` in hold mode.
// - `Hive::key_released(&self)`: the gate passed the key release on. In a
//   hold show that is waiting or over a cell it closes the hive with one
//   `on_dismiss` and runs nothing; after a drop on a cell, in a show without
//   hold, or in a normal show it does nothing.
// - Decision 3: in a hold show the drag leaving the hive after entering it
//   does not close it.

#[test]
fn ac12_s019e_key_release_closes_a_hold_show_once() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = Hive::new(Lang::En).unwrap();
    let log = Rc::new(Log::default());
    let l = log.clone();
    hive.on_choose(move |id| l.chosen.borrow_mut().push(id));
    let l = log.clone();
    hive.on_dismiss(move || l.dismissed.set(l.dismissed.get() + 1));
    let l = log.clone();
    hive.on_drop_files(move |files| {
        l.entered.borrow_mut().push(files.clone());
        ("1 image".into(), cells())
    });
    let png = PathBuf::from("/tmp/a.png");

    // Waiting for the drag: the release closes it, once.
    hive.present_drop_hold("", None, None).unwrap();
    assert!(hive.inner.visible());
    assert_eq!(state(&hive), dnd::State::Waiting);
    hive.key_released();
    assert!(
        !hive.inner.visible(),
        "waiting hold show closed by the release"
    );
    assert_eq!(state(&hive), dnd::State::Hidden);
    assert_eq!(log.dismissed.get(), 1);
    hive.key_released();
    assert_eq!(log.dismissed.get(), 1, "a second release adds nothing");

    // Over a cell: closed, nothing runs, and a drop after it runs nothing.
    hive.present_drop_hold("", None, Some((400.0, 500.0)))
        .unwrap();
    on_note(Note::Enter(vec![png.clone()]));
    on_note(Note::Motion(Some(0)));
    assert_eq!(state(&hive), dnd::State::Over(0));
    hive.key_released();
    assert!(!hive.inner.visible());
    assert_eq!(log.dismissed.get(), 2);
    on_note(Note::Drop);
    assert!(
        log.chosen.borrow().is_empty(),
        "no action after the release"
    );

    // Decision 3: leaving keeps it while the key is held; the release closes.
    hive.present_drop_hold("", None, None).unwrap();
    on_note(Note::Enter(vec![png.clone()]));
    on_note(Note::Motion(Some(1)));
    on_note(Note::Leave);
    assert!(hive.inner.visible(), "the drag left, the key is still held");
    assert_eq!(log.dismissed.get(), 2);
    hive.key_released();
    assert!(!hive.inner.visible());
    assert_eq!(log.dismissed.get(), 3);

    // After a drop on a cell the release changes nothing.
    hive.present_drop_hold("", None, None).unwrap();
    on_note(Note::Enter(vec![png.clone()]));
    on_note(Note::Motion(Some(0)));
    on_note(Note::Drop);
    assert_eq!(log.chosen.borrow().as_slice(), ["webp"]);
    let dismissed = log.dismissed.get();
    hive.key_released();
    assert_eq!(log.dismissed.get(), dismissed, "no dismiss after the drop");
    assert_eq!(log.chosen.borrow().len(), 1);

    // A drop show without hold (`sbarhive drop`) ignores it.
    hive.present_drop("", None).unwrap();
    hive.key_released();
    assert!(hive.inner.visible());
    assert_eq!(state(&hive), dnd::State::Waiting);
    assert_eq!(log.dismissed.get(), dismissed);
    on_note(Note::Enter(vec![png.clone()]));
    on_note(Note::Leave);
    assert!(!hive.inner.visible(), "S-014: leave after enter closes it");

    // A normal show ignores it too.
    hive.present("3 images", cells(), None).unwrap();
    let dismissed = log.dismissed.get();
    hive.key_released();
    assert!(hive.inner.visible());
    assert_eq!(state(&hive), dnd::State::Normal);
    assert_eq!(log.dismissed.get(), dismissed);
}

// ------------------------------------------------------------------------
// S-019f (backlog/S-019-hive-at-pointer.md, AC14): after enter, the drop hive
// shows the dragged files in its centre by the S-018 AC24 and AC25 rules
// (the preview properties and the centre's accessible label fixed in
// ui_tests.rs, S-018h); before enter, the waiting text and no preview.
// The thumbnails come from the source set with `Hive::set_preview_source`
// (a private cache here).

/// A private directory under the system temp dir, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(test: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("sbarhive-s019f-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("files")).unwrap();
        Self(dir)
    }

    fn thumbnails(&self) -> PathBuf {
        self.0.join("cache").join("thumbnails")
    }

    fn file(&self, name: &str) -> PathBuf {
        let p = self.0.join("files").join(name);
        std::fs::write(&p, b"the dragged file").unwrap();
        p
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A valid 128 x 128 thumbnail of `file` in the private cache, one colour.
fn seed_thumbnail(s: &Scratch, file: &std::path::Path) {
    use crate::thumb::{Size, file_uri, find_thumbnail, thumbnail_path};
    let uri = file_uri(file);
    let path = thumbnail_path(&s.thumbnails(), Size::Large, &uri);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mtime = std::fs::metadata(file)
        .unwrap()
        .modified()
        .unwrap()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let f = std::fs::File::create(&path).unwrap();
    let mut e = png::Encoder::new(std::io::BufWriter::new(f), 128, 128);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    e.add_text_chunk("Thumb::URI".into(), uri).unwrap();
    e.add_text_chunk("Thumb::MTime".into(), mtime.to_string())
        .unwrap();
    let data: Vec<u8> = [70u8, 130, 200, 255].repeat(128 * 128);
    e.write_header().unwrap().write_image_data(&data).unwrap();
    assert_eq!(find_thumbnail(&s.thumbnails(), file), Some(path));
}

fn image_size(img: &slint::Image) -> (u32, u32) {
    let s = img.size();
    (s.width, s.height)
}

fn centre_labelled(hive: &Hive, label: &str) -> usize {
    use i_slint_backend_testing::ElementHandle;
    let ui = hive.inner.ui.borrow();
    ElementHandle::find_by_accessible_label(&*ui, label).count()
}

#[test]
fn ac14_s019f_the_drop_hive_previews_the_dragged_files_after_enter() {
    i_slint_backend_testing::init_no_event_loop();
    let s = Scratch::new("drop");
    let photo = s.file("holiday.png");
    seed_thumbnail(&s, &photo);
    let doc = s.file("report.pdf");
    let three = vec![s.file("a.png"), s.file("b.png"), s.file("c.png")];

    let hive = Hive::new(Lang::En).unwrap();
    let mime = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/mime");
    hive.set_preview_source(
        s.thumbnails(),
        thumb::Globs::load(&[mime.join("home"), mime.join("system")]),
    );
    hive.on_drop_files(move |files| match files.len() {
        0 => (String::new(), Vec::new()),
        1 => ("1 file".into(), cells()),
        n => (format!("{n} files"), cells()),
    });
    let waiting = "Drop files here";

    // Before enter: the waiting text, no preview (S-014c AC10 as it is).
    hive.present_drop(waiting, None).unwrap();
    {
        let ui = hive.inner.ui.borrow();
        assert_eq!(ui.get_centre_text().as_str(), waiting);
        assert_eq!(ui.get_preview_count(), 0, "no preview before enter");
        assert_eq!(image_size(&ui.get_preview_image()), (0, 0));
        assert_eq!(image_size(&ui.get_preview_icon()), (0, 0));
    }

    // A PNG with a thumbnail: one file, the picture, named by its file name.
    on_note(Note::Enter(vec![photo.clone()]));
    {
        let ui = hive.inner.ui.borrow();
        assert_eq!(ui.get_preview_count(), 1);
        let (w, h) = image_size(&ui.get_preview_image());
        assert!(w > 0 && h > 0, "the thumbnail after enter: {w}x{h}");
        assert_eq!(image_size(&ui.get_preview_icon()), (0, 0), "no kind icon");
    }
    assert_eq!(
        centre_labelled(&hive, "holiday.png"),
        1,
        "named by the file"
    );
    hive.close();

    // A PDF without a thumbnail: the kind icon and the format code.
    hive.present_drop(waiting, None).unwrap();
    assert_eq!(hive.inner.ui.borrow().get_preview_count(), 0);
    on_note(Note::Enter(vec![doc.clone()]));
    {
        let ui = hive.inner.ui.borrow();
        assert_eq!(ui.get_preview_count(), 1);
        assert_eq!(image_size(&ui.get_preview_image()), (0, 0), "no thumbnail");
        let (w, h) = image_size(&ui.get_preview_icon());
        assert!(w > 0 && h > 0, "the pdf kind icon: {w}x{h}");
        assert_eq!(ui.get_preview_code().as_str(), "PDF");
    }
    assert_eq!(centre_labelled(&hive, "report.pdf"), 1);
    hive.close();

    // Three files: the count, and the centre named by the centre text.
    hive.present_drop(waiting, None).unwrap();
    on_note(Note::Enter(three));
    assert_eq!(hive.inner.ui.borrow().get_preview_count(), 3);
    assert_eq!(
        centre_labelled(&hive, "3 files"),
        1,
        "named by the centre text"
    );
    assert_eq!(centre_labelled(&hive, "a.png"), 0);
    hive.close();

    // Hidden again: the pictures are freed (S-018 AC24).
    let ui = hive.inner.ui.borrow();
    assert_eq!(image_size(&ui.get_preview_image()), (0, 0));
    assert_eq!(image_size(&ui.get_preview_icon()), (0, 0));
}
