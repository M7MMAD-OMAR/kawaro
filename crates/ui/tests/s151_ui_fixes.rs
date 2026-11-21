//! S-151 UI review fixes:
//! - Drop on More transitions out of `dnd::State::Over` into click-to-choose,
//!   keeps the dropped files, advances the page, accepts clicks/Escape/focus
//!   loss/click outside, and arms the 10 s idle timeout.
//! - Paging with `outer_slots <= 1` and exact-capacity single-page layouts
//!   keeps every action without dropping or reserving ring-1 slots early.
//! - `PresetReply::Applied` clears the pending label with a bilingual success
//!   message.
//! - `image.to-pdf` and `OutputMode::Combined` report no preview and schedule
//!   no background preview work.
//! - Header dimension probing works on every platform (including Windows).

use kawaro_core::CancelToken;
use kawaro_core::action::{ActionDef, OutputMode, Registry};
use kawaro_core::config::DropMode;
use kawaro_core::i18n::Lang;
use kawaro_core::image_preview::{
    ImagePreviewArtifact, ImagePreviewError, ImagePreviewProvider, ImagePreviewRequest,
};
use kawaro_core::presets::{PresetStatus, StoreRevision, UserPreset, fingerprint};
use kawaro_ui::dnd::{ENTER_TIMEOUT, Event, Machine, State};
use kawaro_ui::layout::{
    MORE_ID, fits_single_page, page_for_slots, pages_for_slots, split_for_placement,
};
use kawaro_ui::preview_schedule::Scheduler;
use kawaro_ui::saved_presets::{
    PresetApplied, PresetFailure, PresetItem, PresetReply, PresetRequest, PresetService,
    PresetSnapshot,
};
use kawaro_ui::stage_two::{StageTwoState, applied_text};
use kawaro_ui::{Hive, HiveCell};
use slint::platform::{Key, WindowEvent};
use slint::{ComponentHandle, Model, SharedString};
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

fn cell(id: &str, ring: u8) -> HiveCell {
    HiveCell {
        id: id.into(),
        code: id.to_uppercase(),
        label: format!("label {id}"),
        icon: String::new(),
        ring,
        enabled: true,
        reason: String::new(),
        customize: false,
    }
}

fn many(prefix: &str, n: usize, ring: u8) -> Vec<HiveCell> {
    (0..n)
        .map(|i| cell(&format!("{prefix}{i}"), ring))
        .collect()
}

fn actions(total: usize, common: usize) -> Vec<HiveCell> {
    (0..total)
        .map(|i| {
            let ring = if i < common { 1 } else { 2 };
            cell(&format!("image.act-{i}"), ring)
        })
        .collect()
}

fn tiny_png(path: &Path) {
    let file = std::fs::File::create(path).expect("create png");
    let writer = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, 8, 8);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut png_writer = encoder.write_header().expect("write header");
    png_writer
        .write_image_data(&vec![128u8; 8 * 8 * 4])
        .expect("write pixels");
}

// ------------------------------------------------------------------ Item 2: paging with outer_slots <= 1

#[test]
fn pages_for_slots_handles_one_and_zero_outer_slots_and_exact_capacity() {
    // outer_slots = 1: 0 or 1 action fits on one page; 2+ actions must page.
    assert_eq!(pages_for_slots(&many("a", 0, 2), 1), 1);
    assert_eq!(pages_for_slots(&many("a", 1, 2), 1), 1);
    assert_eq!(pages_for_slots(&many("a", 2, 2), 1), 2);
    assert_eq!(pages_for_slots(&many("a", 3, 2), 1), 3);

    // outer_slots = 0: only an empty queue fits; 2+ actions must page.
    assert_eq!(pages_for_slots(&many("a", 0, 2), 0), 1);
    assert_eq!(pages_for_slots(&many("a", 2, 2), 0), 2);
    assert_eq!(pages_for_slots(&many("a", 3, 2), 0), 3);

    // Exact capacity on wider outer rings: one page, no More cell.
    for slots in [2, 5, 8, 12] {
        let exact = many("a", slots, 2);
        assert_eq!(pages_for_slots(&exact, slots), 1, "exact {slots}");
        let page = page_for_slots(&exact, 0, slots, Lang::En);
        assert_eq!(page.len(), slots);
        assert!(page.iter().all(|c| c.id != MORE_ID));

        let over = many("a", slots + 1, 2);
        assert!(pages_for_slots(&over, slots) >= 2, "over {slots}");
        let first = page_for_slots(&over, 0, slots, Lang::En);
        assert_eq!(first.last().map(|c| c.id.as_str()), Some(MORE_ID));
    }
}

#[test]
fn page_cells_for_seven_actions_with_ring1_six_and_outer_one_keep_all_seven() {
    for common in [6, 7] {
        let all = actions(7, common);
        assert!(
            fits_single_page(&all, 6, 1),
            "{common} ring-1 actions fit in 6 + 1"
        );
        let (ring1, outer) = split_for_placement(&all, 6, 1);
        assert_eq!(ring1.len(), 6, "no ring-1 slot reserved when all 7 fit");
        assert_eq!(outer.len(), 1);
        assert_eq!(pages_for_slots(&outer, 1), 1);
        let mut shown = ring1;
        shown.extend(page_for_slots(&outer, 0, 1, Lang::En));
        assert_eq!(shown.len(), 7, "all 7 actions kept");
        assert!(shown.iter().all(|c| c.id != MORE_ID), "no More cell");
        let got: Vec<&str> = shown.iter().map(|c| c.id.as_str()).collect();
        let want: Vec<&str> = all.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(got, want);
    }

    // 8 actions (6 ring-1 + 2 ring-2) exceed 6 + 1: page with 1 reserved ring-1 slot.
    let eight = actions(8, 6);
    assert!(!fits_single_page(&eight, 6, 1));
    let (ring1, outer) = split_for_placement(&eight, 6, 1);
    assert_eq!(ring1.len(), 5);
    assert_eq!(outer.len(), 3);
    assert_eq!(pages_for_slots(&outer, 1), 3);
    let mut seen: Vec<String> = ring1.iter().map(|c| c.id.clone()).collect();
    for p in 0..pages_for_slots(&outer, 1) {
        let pg = page_for_slots(&outer, p, 1, Lang::En);
        assert_eq!(pg.len(), 2, "one action and More on page {p}");
        assert_eq!(pg[1].id, MORE_ID);
        seen.push(pg[0].id.clone());
    }
    let want: Vec<String> = eight.iter().map(|c| c.id.clone()).collect();
    assert_eq!(seen, want);
}

#[test]
fn page_cells_with_ring1_six_and_outer_zero_fit_six_and_page_seven() {
    let six = actions(6, 6);
    assert!(fits_single_page(&six, 6, 0));
    let (ring1, outer) = split_for_placement(&six, 6, 0);
    assert_eq!(ring1.len(), 6);
    assert!(outer.is_empty());

    let seven = actions(7, 6);
    assert!(!fits_single_page(&seven, 6, 0));
    let (ring1, outer) = split_for_placement(&seven, 6, 0);
    assert_eq!(ring1.len(), 4);
    assert_eq!(outer.len(), 3);
    assert_eq!(pages_for_slots(&outer, 0), 3);
}

// ------------------------------------------------------------------ Item 1: drop on More leaves State::Over

#[test]
fn dnd_machine_enter_choose_leaves_over_and_arms_close_paths() {
    let t0 = Instant::now();
    let mut m = Machine::new();
    assert_eq!(
        m.handle(
            Event::Show {
                drop: true,
                hold: true,
                layer: true,
            },
            t0,
        ),
        State::Probing
    );
    assert_eq!(
        m.handle(Event::Enter, t0 + Duration::from_secs(1)),
        State::Waiting
    );
    assert_eq!(
        m.handle(Event::Motion(Some(11)), t0 + Duration::from_secs(2)),
        State::Over(11)
    );

    let t1 = t0 + Duration::from_secs(3);
    assert_eq!(m.enter_choose(t1), State::Normal);
    assert!(m.has_idle_timeout());

    // Late drag and hold events are ignored in click-to-choose mode.
    for ev in [
        Event::Leave,
        Event::Release,
        Event::Cancel,
        Event::Drop,
        Event::Motion(Some(0)),
    ] {
        assert_eq!(m.handle(ev, t1 + Duration::from_millis(50)), State::Normal);
    }

    // Idle timeout is armed for ENTER_TIMEOUT from t1.
    assert_eq!(
        m.handle(Event::Tick, t1 + ENTER_TIMEOUT - Duration::from_millis(100)),
        State::Normal
    );
    m.reset_idle(t1 + Duration::from_secs(5));
    assert_eq!(
        m.handle(Event::Tick, t1 + ENTER_TIMEOUT),
        State::Normal,
        "paging resets the idle clock"
    );
    assert_eq!(
        m.handle(Event::Tick, t1 + Duration::from_secs(5) + ENTER_TIMEOUT),
        State::Hidden
    );

    // Escape, FocusLost and ClickOutside each close click-to-choose mode.
    for close_event in [Event::Escape, Event::FocusLost, Event::ClickOutside] {
        let mut m = Machine::new();
        m.enter_choose(t1);
        assert_eq!(m.handle(close_event, t1), State::Hidden, "{close_event:?}");
    }
}

#[test]
fn drop_on_more_advances_page_and_lets_click_choose_or_escape_close() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = Hive::new(Lang::En).expect("a hive window");
    hive.set_drop_mode(DropMode::ChooseDuringDrag);

    let chosen = Rc::new(RefCell::new(Vec::<String>::new()));
    let c = chosen.clone();
    hive.on_choose(move |id| c.borrow_mut().push(id));

    let dismissed = Rc::new(Cell::new(0u32));
    let d = dismissed.clone();
    hive.on_dismiss(move || d.set(d.get() + 1));

    hive.on_drop_files(|_| (String::from("1 image"), actions(20, 6)));

    // Hold drop show -> drop on More -> key_released must not dismiss -> click page-2 action chooses it.
    hive.present_drop_hold("", None, None).expect("drop hold");
    hive.feed_enter(vec![PathBuf::from("/tmp/s151-a.png")]);

    let cells_before = hive.testing_window().get_cells();
    let more = (0..cells_before.row_count())
        .find(|&i| {
            cells_before
                .row_data(i)
                .is_some_and(|c| c.label.starts_with("More actions"))
        })
        .expect("More cell on page 1");
    hive.feed_drop(Some(more));

    assert!(hive.testing_window().window().is_visible());
    assert_eq!(dismissed.get(), 0);
    // Releasing the held drop key after dropping on More does not dismiss the hive.
    hive.key_released();
    assert!(hive.testing_window().window().is_visible());
    assert_eq!(dismissed.get(), 0);

    // Clicking an action on page 2 runs that action.
    assert!(
        hive.click_cell("image.act-17"),
        "page-2 action is clickable"
    );
    assert_eq!(chosen.borrow().as_slice(), ["image.act-17"]);
    assert_eq!(dismissed.get(), 0);

    // Second drop on More -> Escape closes and dismisses.
    hive.present_drop("", None).expect("second drop");
    hive.feed_enter(vec![PathBuf::from("/tmp/s151-b.png")]);
    hive.feed_drop(Some(more));
    assert!(hive.testing_window().window().is_visible());
    // The drag session is over: a late leave of the drag changes nothing.
    hive.feed_leave();
    assert!(hive.testing_window().window().is_visible());
    assert_eq!(dismissed.get(), 0, "a late leave does not dismiss");
    let esc: SharedString = Key::Escape.into();
    let win = hive.testing_window();
    win.window()
        .dispatch_event(WindowEvent::KeyPressed { text: esc.clone() });
    win.window()
        .dispatch_event(WindowEvent::KeyReleased { text: esc });
    assert_eq!(dismissed.get(), 1, "Escape dismisses after drop on More");
}

// ------------------------------------------------------------------ Item 6: Applied clears pending label

struct FakePresetService {
    action: ActionDef,
    preset: UserPreset,
}

impl PresetService for FakePresetService {
    fn submit(&self, request: PresetRequest) -> Result<mpsc::Receiver<PresetReply>, PresetFailure> {
        let (tx, rx) = mpsc::channel();
        let reply = match request {
            PresetRequest::List { .. } => PresetReply::List(PresetSnapshot {
                revision: StoreRevision(String::from("rev-1")),
                items: vec![PresetItem::new(self.preset.clone(), PresetStatus::Ready)],
                read_only: None,
            }),
            PresetRequest::Apply { .. } => {
                let mut applied_action = self.action.clone();
                applied_action
                    .params
                    .insert(String::from("quality"), toml::Value::Integer(70));
                PresetReply::Applied(PresetApplied {
                    action: applied_action,
                })
            }
            _ => PresetReply::Failed(PresetFailure::Unsupported(String::from("not needed"))),
        };
        let _ = tx.send(reply);
        Ok(rx)
    }
}

#[test]
fn preset_apply_clears_pending_label_in_english_and_arabic() {
    let action = Registry::builtin()
        .get("image.to-webp")
        .expect("image.to-webp")
        .clone();
    let mut values = toml::Table::new();
    values.insert(String::from("quality"), toml::Value::Integer(70));
    let preset = UserPreset {
        id: String::from("preset-1"),
        name: String::from("Web"),
        action_id: action.id.clone(),
        schema_fingerprint: fingerprint(&action),
        values,
    };

    for (lang, pending_expected) in [
        (Lang::En, "Applying preset"),
        (Lang::Ar, "جار تطبيق الإعداد"),
    ] {
        let state = StageTwoState::build(&action, &[], lang, lang.rtl()).expect("panel");
        let service = Arc::new(FakePresetService {
            action: action.clone(),
            preset: preset.clone(),
        });
        state.attach_preset_service(service, 1, 1);
        let list_update = state.poll_preset(1, 1).expect("list reply");
        assert!(!list_update.pending);

        state.apply_named_preset(1, 1, "preset-1");
        assert_eq!(state.preset_status(), pending_expected);

        let applied_update = state.poll_preset(1, 1).expect("apply reply");
        assert!(!applied_update.pending);
        assert!(applied_update.rebuild);
        assert_ne!(applied_update.status, pending_expected);
        assert_eq!(applied_update.status, applied_text(lang));
        assert_eq!(state.preset_status(), applied_text(lang));
        assert_eq!(
            state
                .resolved()
                .expect("resolved")
                .get("quality")
                .and_then(toml::Value::as_integer),
            Some(70)
        );
    }
}

// ------------------------------------------------------------------ Items 4 & 5: to-pdf / Combined report no preview

struct CountingProvider {
    calls: AtomicUsize,
}

impl ImagePreviewProvider for CountingProvider {
    fn preview(
        &self,
        _request: &ImagePreviewRequest,
        _cancel: &CancelToken,
    ) -> Result<ImagePreviewArtifact, ImagePreviewError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(ImagePreviewError::Unsupported(String::from(
            "should not be called",
        )))
    }
}

#[test]
fn to_pdf_and_combined_image_actions_report_no_preview_and_schedule_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let png_path = dir.path().join("sample.png");
    tiny_png(&png_path);

    // Sanity check: a normal single-image action probes dimensions and offers a preview
    // on every platform (including Windows, Item 4).
    let webp = Registry::builtin()
        .get("image.to-webp")
        .expect("image.to-webp")
        .clone();
    let webp_state = StageTwoState::build(&webp, std::slice::from_ref(&png_path), Lang::En, false)
        .expect("image.to-webp panel");
    assert!(webp_state.preview_available());

    // image.to-pdf: op == "to-pdf" and OutputMode::Combined -> no preview.
    let to_pdf = Registry::builtin()
        .get("image.to-pdf")
        .expect("image.to-pdf")
        .clone();
    let pdf_state = StageTwoState::build(&to_pdf, std::slice::from_ref(&png_path), Lang::En, false)
        .expect("image.to-pdf panel");
    assert!(!pdf_state.preview_available());
    assert_eq!(pdf_state.preview_note(), Some("Preview not available"));

    let provider = Arc::new(CountingProvider {
        calls: AtomicUsize::new(0),
    });
    let scheduler = Scheduler::new();
    scheduler.set_provider(provider.clone());
    pdf_state.attach_preview(scheduler.clone(), 1, 1);
    pdf_state.request_preview();
    pdf_state.request_encoded();
    scheduler.tick(Instant::now() + Duration::from_secs(1));
    std::thread::sleep(Duration::from_millis(20));
    assert!(pdf_state.poll_preview().is_none());
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);

    // Combined output with a non-to-pdf op must also report no preview.
    let mut combined = webp;
    combined.output.mode = OutputMode::Combined;
    let combined_state =
        StageTwoState::build(&combined, std::slice::from_ref(&png_path), Lang::En, false)
            .expect("combined panel");
    assert!(!combined_state.preview_available());

    // In the real Hive window, both stage-preview and stage-preview-encode stay false.
    i_slint_backend_testing::init_no_event_loop();
    let hive = Hive::new(Lang::En).expect("a hive window");
    hive.set_preview_provider(provider.clone());
    let state = Rc::new(
        StageTwoState::build(&to_pdf, std::slice::from_ref(&png_path), Lang::En, false)
            .expect("image.to-pdf panel"),
    );
    hive.present_options(vec![cell("image.to-pdf", 1)], state)
        .expect("present options");
    let win = hive.testing_window();
    assert!(!win.get_stage_preview());
    assert!(!win.get_stage_preview_encode());
}
