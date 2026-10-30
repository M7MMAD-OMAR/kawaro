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
            enabled: true,
            reason: String::new(),
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

/// S-017c (manager decision 3): a user driven close keeps the surface for
/// the 90 ms of `Tokens.hive-close` and is gone after it. The callbacks run
/// in the same turn as before, so only the hide is delayed.
fn after_close(hive: &Hive) {
    assert!(
        hive.inner.visible(),
        "the close animation keeps the surface for 90 ms"
    );
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(120));
    assert!(!hive.inner.visible(), "hidden after the 90 ms close");
}

#[test]
fn drop_hive_behaviour() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = test_env::build(&[], Lang::En);
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
    after_close(&hive);
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
    after_close(&hive);

    // Leave after enter closes; leave before enter does not.
    hive.present_drop("", None).unwrap();
    on_note(Note::Leave);
    assert!(hive.inner.visible());
    on_note(Note::Enter(Vec::new()));
    assert_eq!(hive.inner.ui.borrow().get_cells().row_count(), 0);
    on_note(Note::Leave);
    assert_eq!(log.dismissed.get(), 2);
    after_close(&hive);

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
    after_close(&hive);

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
    let hive = test_env::build(&[], Lang::En);
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
    after_close(&hive); // waiting hold show closed by the release
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
    after_close(&hive);
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
    after_close(&hive);
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

    // A drop show without hold (`kawaro drop`) ignores it.
    hive.present_drop("", None).unwrap();
    hive.key_released();
    assert!(hive.inner.visible());
    assert_eq!(state(&hive), dnd::State::Waiting);
    assert_eq!(log.dismissed.get(), dismissed);
    on_note(Note::Enter(vec![png.clone()]));
    on_note(Note::Leave);
    after_close(&hive); // S-014: leave after enter closes it

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
        let dir = std::env::temp_dir().join(format!("kawaro-s019f-{test}-{}", std::process::id()));
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

    let hive = test_env::build(&[], Lang::En);
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

// ------------------------------------------------------------------------
// S-021a (backlog/S-021-drop-only-on-drag.md, AC1, AC2 and AC9): on the layer
// path a drop show probes for a real file drag before it is revealed.
//
// Contract (written by the tester from the acceptance criteria; do not edit):
// - `dnd::Event::Show { drop, hold, layer }`, `dnd::State::Probing`,
//   `dnd::Event::PointerEnter` and `dnd::Event::Reject` (see drop_tests.rs).
// - `receiver::Note::PointerEnter`: the pointer entered the hidden catcher
//   with no drag (`wl_pointer.enter`); `on_note` passes it to the machine as
//   `Event::PointerEnter`. layer.rs only forwards it.
// - While the machine is `Probing`, `Note::Enter(files)` calls
//   `on_drop_files` once. One cell or more: the cells and the centre text
//   are applied as today and the machine gets `Event::Enter` (`Waiting`).
//   No cell: the machine gets `Event::Reject`, the centre text the callback
//   returned is not applied, and the hive hides.
// - Every `Probing` to `Hidden` transition fires `on_dismiss` exactly once.
// - The headless platform is the window path, so the tests put the machine
//   in `Probing` themselves after `present_drop*`, as the timeout check above
//   drives the machine directly.

/// The text the daemon puts in the centre for files without an action.
const NO_ACTIONS: &str = "No actions";

/// A hive whose drop callback gives cells to `.png` files only.
fn probing_hive() -> (Hive, Rc<Log>) {
    i_slint_backend_testing::init_no_event_loop();
    let hive = test_env::build(&[], Lang::En);
    let log = Rc::new(Log::default());
    let l = log.clone();
    hive.on_choose(move |id| l.chosen.borrow_mut().push(id));
    let l = log.clone();
    hive.on_dismiss(move || l.dismissed.set(l.dismissed.get() + 1));
    let l = log.clone();
    hive.on_drop_files(move |files| {
        l.entered.borrow_mut().push(files.clone());
        if !files.is_empty()
            && files
                .iter()
                .all(|f| f.extension().is_some_and(|e| e == "png"))
        {
            ("1 image".into(), cells())
        } else {
            (NO_ACTIONS.into(), Vec::new())
        }
    });
    (hive, log)
}

/// Puts the drop show just presented on the layer path, shown at `since`.
fn probe_since(hive: &Hive, hold: bool, since: Instant) {
    let s = hive.inner.machine.borrow_mut().handle(
        dnd::Event::Show {
            drop: true,
            hold,
            layer: true,
        },
        since,
    );
    assert_eq!(s, dnd::State::Probing);
}

fn probe(hive: &Hive, hold: bool) {
    probe_since(hive, hold, Instant::now());
}

fn centre(hive: &Hive) -> String {
    hive.inner.ui.borrow().get_centre_text().to_string()
}

#[test]
fn ac1_ac2_s021a_enter_with_cells_reveals_a_probing_show() {
    let (hive, log) = probing_hive();
    let png = PathBuf::from("/tmp/a.png");
    hive.present_drop_hold("", None, None).unwrap();
    probe(&hive, true);
    on_note(Note::Enter(vec![png.clone()]));
    assert_eq!(log.entered.borrow().as_slice(), [vec![png.clone()]]);
    assert_eq!(state(&hive), dnd::State::Waiting, "revealed for the drag");
    assert!(hive.inner.visible());
    assert_eq!(hive.inner.ui.borrow().get_cells().row_count(), 3);
    assert_eq!(centre(&hive), "1 image");
    assert_eq!(log.dismissed.get(), 0);
    // Then the drop runs the cell as today.
    on_note(Note::Motion(Some(0)));
    on_note(Note::Drop);
    assert_eq!(log.chosen.borrow().as_slice(), ["webp"]);
    assert_eq!(log.dismissed.get(), 0);
}

#[test]
fn ac2_s021a_enter_without_cells_hides_once_without_the_no_actions_text() {
    let (hive, log) = probing_hive();
    for files in [vec![PathBuf::from("/tmp/notes.unknownext")], Vec::new()] {
        let before = log.dismissed.get();
        let calls = log.entered.borrow().len();
        hive.present_drop("", None).unwrap();
        probe(&hive, false);
        on_note(Note::Enter(files.clone()));
        assert_eq!(
            log.entered.borrow().len(),
            calls + 1,
            "{files:?}: asked once"
        );
        assert_eq!(state(&hive), dnd::State::Hidden, "{files:?}: rejected");
        assert!(!hive.inner.visible(), "{files:?}: hidden");
        assert_ne!(centre(&hive), NO_ACTIONS, "{files:?}: no `no_actions` text");
        assert_eq!(log.dismissed.get(), before + 1, "{files:?}: one dismiss");
        // Anything after the hide adds no dismiss.
        on_note(Note::Leave);
        on_note(Note::PointerEnter);
        hive.inner.drag(dnd::Event::Tick);
        assert_eq!(log.dismissed.get(), before + 1, "{files:?}: still one");
    }
    assert!(log.chosen.borrow().is_empty());
}

#[test]
fn ac2_s021a_every_hide_of_a_probing_show_dismisses_exactly_once() {
    let (hive, log) = probing_hive();
    let mut expected = 0;
    let mut check = |hive: &Hive, what: &str| {
        expected += 1;
        assert_eq!(state(hive), dnd::State::Hidden, "{what}");
        assert!(!hive.inner.visible(), "{what}: hidden");
        assert_eq!(log.dismissed.get(), expected, "{what}: one dismiss");
        on_note(Note::PointerEnter);
        hive.key_released();
        hive.inner.drag(dnd::Event::Tick);
        assert_eq!(log.dismissed.get(), expected, "{what}: no second dismiss");
    };

    hive.present_drop_at("", None, Some((400.0, 500.0)))
        .unwrap();
    probe(&hive, false);
    on_note(Note::PointerEnter);
    check(&hive, "pointer");

    hive.present_drop_hold("", None, None).unwrap();
    probe(&hive, true);
    hive.key_released();
    check(&hive, "release");

    hive.present_drop("", None).unwrap();
    probe(&hive, false);
    hive.inner.drag(dnd::Event::Reject);
    check(&hive, "reject");

    hive.present_drop("", None).unwrap();
    probe(&hive, false);
    hive.inner.drag(dnd::Event::Escape);
    check(&hive, "escape");

    hive.present_drop("", None).unwrap();
    probe(&hive, false);
    hive.inner.drag(dnd::Event::Cancel);
    check(&hive, "cancel");

    // The cap: shown 10.1 s ago with no event, the next tick hides it.
    hive.present_drop("", None).unwrap();
    let since = Instant::now()
        .checked_sub(Duration::from_secs_f64(10.1))
        .unwrap();
    probe_since(&hive, false, since);
    hive.inner.drag(dnd::Event::Tick);
    check(&hive, "cap");

    assert!(log.chosen.borrow().is_empty());
}

#[test]
fn ac1_s021a_focus_loss_and_click_outside_keep_a_probing_show() {
    let (hive, log) = probing_hive();
    hive.present_drop("", None).unwrap();
    probe(&hive, false);
    focus_changed(true);
    focus_changed(false);
    hive.inner.drag(dnd::Event::ClickOutside);
    assert_eq!(state(&hive), dnd::State::Probing);
    assert!(hive.inner.visible(), "the catcher stays bound");
    assert_eq!(log.dismissed.get(), 0);
    // A short tick changes nothing either.
    hive.inner.drag(dnd::Event::Tick);
    assert_eq!(state(&hive), dnd::State::Probing);
}

#[test]
fn ac1_s021a_the_window_path_is_unchanged() {
    let (hive, log) = probing_hive();
    // No probe: the window path waits, and a pointer enter does nothing.
    hive.present_drop("", None).unwrap();
    assert_eq!(state(&hive), dnd::State::Waiting);
    on_note(Note::PointerEnter);
    assert_eq!(state(&hive), dnd::State::Waiting);
    assert!(hive.inner.visible());
    // A file without an action keeps the hive with the no_actions text (Q5).
    on_note(Note::Enter(vec![PathBuf::from("/tmp/notes.unknownext")]));
    assert_eq!(state(&hive), dnd::State::Waiting);
    assert!(hive.inner.visible());
    assert_eq!(centre(&hive), NO_ACTIONS);
    assert_eq!(log.dismissed.get(), 0);
    // A normal show ignores it too.
    hive.present("3 images", cells(), None).unwrap();
    on_note(Note::PointerEnter);
    assert_eq!(state(&hive), dnd::State::Normal);
    assert!(hive.inner.visible());
    assert_eq!(log.dismissed.get(), 0);
}

#[test]
fn ac9_s021a_two_hides_without_a_drag_then_a_held_png_drag_reveals() {
    let (hive, log) = probing_hive();
    let png = PathBuf::from("/tmp/a.png");

    // 1: `drop --at` with no drag, the pointer moves into the catcher.
    hive.present_drop_at("", None, Some((400.0, 500.0)))
        .unwrap();
    probe(&hive, false);
    on_note(Note::PointerEnter);
    assert_eq!(state(&hive), dnd::State::Hidden);
    assert_eq!(log.dismissed.get(), 1);

    // 2: `drop --hold`, then `drop --release` with the pointer still.
    hive.present_drop_hold("", None, None).unwrap();
    probe(&hive, true);
    hive.key_released();
    assert_eq!(state(&hive), dnd::State::Hidden);
    assert_eq!(log.dismissed.get(), 2);
    // A release after the hide adds nothing and shows nothing.
    hive.key_released();
    assert_eq!(log.dismissed.get(), 2);
    assert!(!hive.inner.visible());

    // 3: `drop --hold` with a PNG drag: revealed, and the drop runs.
    hive.present_drop_hold("", None, None).unwrap();
    probe(&hive, true);
    on_note(Note::Enter(vec![png.clone()]));
    assert_eq!(state(&hive), dnd::State::Waiting, "the third show reveals");
    assert!(hive.inner.visible());
    assert_eq!(hive.inner.ui.borrow().get_cells().row_count(), 3);
    on_note(Note::Motion(Some(0)));
    on_note(Note::Drop);
    assert_eq!(log.chosen.borrow().as_slice(), ["webp"]);
    assert_eq!(log.dismissed.get(), 2);
    hive.key_released();
    assert_eq!(log.dismissed.get(), 2, "no dismiss after the drop");
}

// ------------------------------------------------------------------------
// S-022 (backlog/S-022-drop-followups.md, AC3 and AC8): the opening
// animation of a probed drop show starts at its reveal, and a new show over
// a probing one ends it with the reason `replaced` and no dismissal.
//
// Contract (written by the tester from the acceptance criteria; do not edit):
// - `opened` (`HiveWindow::get_opened` and `set_opened`) is the property the
//   opening animation runs to, and `Inner::opener` the timer that sets it
//   after the bind, as today. The tests stop `opener` and set `opened` to
//   false right after putting the show in `Probing`, which is what `show()`
//   does on the layer path (no `opener` while `Probing`, AC4 checks that
//   half on a real compositor). From there only the reveal (`Probing` to
//   `Waiting` in `Inner::drag`) sets it: with motion within one turn of the
//   event loop (`mock_elapsed_time`), with reduced motion at once. A later
//   `Waiting` (the drag leaving a cell) does not open it again.
// - Normal shows and window path drop shows: `opened` true once the bind's
//   `opener` has run, as today.
// - `Inner::replace(&self) -> Option<&'static str>`: ends the shown hive
//   for a new show and gives the reason the replaced show prints. It is
//   `Some("replaced")` when that show was `Probing` and `None` when it was
//   `Waiting`, `Over`, `Normal` or already hidden. It never calls
//   `on_dismiss` (decision 8). `show()` replaces a visible hive through it,
//   and layer.rs prints `hive probe: hidden (replaced)` for `Some`.
// - Reduced motion is read once, by `Hive::new`, from
//   `KAWARO_REDUCED_MOTION`, so one test builds both hives with the
//   variable set, then removes it (the environment is shared by threads).

fn opened(hive: &Hive) -> bool {
    hive.inner.ui.borrow().get_opened()
}

/// The layer path's start of a probed show, on the headless platform.
fn probe_closed(hive: &Hive, hold: bool, since: Instant) {
    probe_since(hive, hold, since);
    hive.inner.opener.stop();
    hive.inner.ui.borrow().set_opened(false);
}

/// One turn of the event loop: the timers due now run.
fn turn() {
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(1));
}

/// A hive built with `KAWARO_REDUCED_MOTION` set to `value`, wired as
/// `probing_hive` is (the platform is installed by the caller, once).
fn hive_with_motion(value: &str) -> (Hive, Rc<Log>) {
    // The build lock inside `test_env::build` keeps the variable private to
    // this build: the environment is shared by every test thread.
    let hive = test_env::build(&[("KAWARO_REDUCED_MOTION", value)], Lang::En);
    let log = Rc::new(Log::default());
    let l = log.clone();
    hive.on_choose(move |id| l.chosen.borrow_mut().push(id));
    let l = log.clone();
    hive.on_dismiss(move || l.dismissed.set(l.dismissed.get() + 1));
    hive.on_drop_files(|files| {
        if !files.is_empty()
            && files
                .iter()
                .all(|f| f.extension().is_some_and(|e| e == "png"))
        {
            ("1 image".into(), cells())
        } else {
            (NO_ACTIONS.into(), Vec::new())
        }
    });
    (hive, log)
}

#[test]
fn ac3_s022_the_opening_animation_starts_at_the_reveal() {
    i_slint_backend_testing::init_no_event_loop();
    let (hive, log) = hive_with_motion("0");
    let (calm, _) = hive_with_motion("1");
    let png = PathBuf::from("/tmp/a.png");
    let txt = PathBuf::from("/tmp/notes.unknownext");

    // With motion. The cap's last moment: still probing, still closed.
    hive.present_drop("", None).unwrap();
    let since = Instant::now()
        .checked_sub(Duration::from_secs_f64(9.9))
        .unwrap();
    probe_closed(&hive, false, since);
    hive.inner.drag(dnd::Event::Tick);
    turn();
    assert_eq!(state(&hive), dnd::State::Probing, "9.9 s: still probing");
    assert!(!opened(&hive), "9.9 s: not opened before the reveal");
    // The reveal opens it within one turn of the loop.
    on_note(Note::Enter(vec![png.clone()]));
    assert_eq!(state(&hive), dnd::State::Waiting);
    turn();
    assert!(opened(&hive), "the reveal starts the opening animation");
    // Only the reveal: a later Waiting (over a cell, then off it) does not
    // open it again.
    hive.inner.ui.borrow().set_opened(false);
    on_note(Note::Motion(Some(1)));
    on_note(Note::Motion(None));
    assert_eq!(state(&hive), dnd::State::Waiting);
    turn();
    assert!(!opened(&hive), "no second opening after the reveal");
    hive.close();

    // Hidden before a reveal: never opened.
    hive.present_drop("", None).unwrap();
    probe_closed(&hive, false, Instant::now());
    on_note(Note::Enter(vec![txt.clone()]));
    assert_eq!(state(&hive), dnd::State::Hidden, "rejected");
    turn();
    assert!(!opened(&hive), "a rejected show is never opened");
    hive.present_drop_at("", None, Some((400.0, 500.0)))
        .unwrap();
    probe_closed(&hive, false, Instant::now());
    on_note(Note::PointerEnter);
    assert_eq!(state(&hive), dnd::State::Hidden, "pointer");
    turn();
    assert!(
        !opened(&hive),
        "a show hidden by the pointer is never opened"
    );

    // As today: a normal show and a window path drop show open once the
    // bind's timer has run.
    hive.present("3 images", cells(), None).unwrap();
    turn();
    assert!(opened(&hive), "a normal show opens after the bind");
    hive.close();
    hive.present_drop("", None).unwrap();
    assert_eq!(state(&hive), dnd::State::Waiting, "the window path waits");
    turn();
    assert!(
        opened(&hive),
        "a window path drop show opens after the bind"
    );
    hive.close();

    // Reduced motion: opened at the reveal itself, no loop turn needed.
    calm.present_drop_hold("", None, None).unwrap();
    probe_closed(&calm, true, Instant::now());
    on_note(Note::Enter(vec![png.clone()]));
    assert_eq!(state(&calm), dnd::State::Waiting);
    assert!(opened(&calm), "reduced motion: opened at the reveal");
    calm.close();
    assert_eq!(log.dismissed.get(), 2, "the reject and the pointer hide");
}

#[test]
fn ac8_s022_a_new_show_over_a_probing_one_is_replaced_without_dismissal() {
    let (hive, log) = probing_hive();
    let png = PathBuf::from("/tmp/a.png");

    // Probing: the reason `replaced`, once, and no on_dismiss.
    hive.present_drop_at("", None, Some((500.0, 300.0)))
        .unwrap();
    probe(&hive, false);
    assert_eq!(hive.inner.replace(), Some("replaced"));
    assert!(!hive.inner.visible(), "the replaced show is gone");
    assert_eq!(state(&hive), dnd::State::Hidden);
    assert_eq!(log.dismissed.get(), 0, "replacing does not dismiss");
    assert_eq!(hive.inner.replace(), None, "one reason per show");
    assert_eq!(log.dismissed.get(), 0);

    // The same through a new show: the first is replaced, the second shown.
    hive.present_drop_hold("", None, None).unwrap();
    probe(&hive, true);
    hive.present_drop_hold("", None, None).unwrap();
    assert!(hive.inner.visible(), "the new show is up");
    assert_eq!(
        state(&hive),
        dnd::State::Waiting,
        "the new show (window path)"
    );
    assert_eq!(log.dismissed.get(), 0, "no dismissal for the replaced show");

    // Waiting, Over and Normal: no reason, and no dismissal either.
    assert_eq!(hive.inner.replace(), None, "Waiting");
    hive.present_drop("", None).unwrap();
    on_note(Note::Enter(vec![png.clone()]));
    on_note(Note::Motion(Some(0)));
    assert_eq!(state(&hive), dnd::State::Over(0));
    assert_eq!(hive.inner.replace(), None, "Over");
    hive.present_drop("", None).unwrap();
    probe(&hive, false);
    on_note(Note::Enter(vec![png.clone()]));
    assert_eq!(state(&hive), dnd::State::Waiting, "revealed");
    assert_eq!(hive.inner.replace(), None, "a revealed show");
    hive.present("3 images", cells(), None).unwrap();
    assert_eq!(state(&hive), dnd::State::Normal);
    assert_eq!(hive.inner.replace(), None, "Normal");
    assert!(!hive.inner.visible());
    assert_eq!(log.dismissed.get(), 0);
    assert!(log.chosen.borrow().is_empty());
}
