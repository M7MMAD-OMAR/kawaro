// S-139 AC1/AC3/AC4/AC6, production callback-to-Queue integration.
// Minimal missing contracts pinned by the tester:
// ui::wire_options(&Hive, &Shared, &Arc<Queue>) extracts the EXISTING startup
// on_customize/on_options_run/on_options_cancel/on_dismiss wiring unchanged
// except for S139 behavior. Production startup MUST call this same function.
// Hive::testing_window() -> HiveWindow, doc-hidden, testing-hooks only, returns
// the actual generated component handle, not a copied state/model. This only
// exposes the existing window for native pointer/key/control observations.
use super::*;
use i_slint_backend_testing::{ElementHandle, ElementRoot};
use kawaro_core::engine::{CancelToken, Engine, EngineError, Progress, Task, TaskOutcome};
use kawaro_ui::Hive;
use slint::ComponentHandle;
use slint::platform::{Key, PointerEventButton, WindowEvent};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};

type Recorded = (String, Vec<PathBuf>, Option<i64>, Option<i64>);
struct ObservedEngine {
    real: Arc<dyn Engine>,
    tx: mpsc::Sender<Recorded>,
}
impl Engine for ObservedEngine {
    fn id(&self) -> &'static str {
        self.real.id()
    }
    fn available(&self) -> bool {
        self.real.available()
    }
    fn capabilities(&self) -> Vec<(&'static str, Vec<kawaro_core::MediaKind>)> {
        self.real.capabilities()
    }
    fn run(&self, t: &Task, p: Progress, c: &CancelToken) -> Result<TaskOutcome, EngineError> {
        self.tx
            .send((
                t.action_id.clone(),
                t.inputs.clone(),
                t.param_i64("scale"),
                t.param_i64("max_side"),
            ))
            .unwrap();
        self.real.run(t, p, c)
    }
}
fn window() -> kawaro_ui::HiveWindow {
    HIVE.with(|h| h.borrow().as_ref().unwrap().testing_window())
}
fn key(k: Key) {
    let w = window();
    let text: slint::SharedString = k.into();
    w.window()
        .dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    w.window().dispatch_event(WindowEvent::KeyReleased { text });
}
fn text(t: &str) {
    let w = window();
    w.window()
        .dispatch_event(WindowEvent::KeyPressed { text: t.into() });
    w.window()
        .dispatch_event(WindowEvent::KeyReleased { text: t.into() });
}
fn click(e: &ElementHandle) {
    let p = e.absolute_position();
    let s = e.size();
    let position = slint::LogicalPosition::new(p.x + s.width / 2., p.y + s.height / 2.);
    let w = window();
    for e in [
        WindowEvent::PointerMoved { position },
        WindowEvent::PointerPressed {
            position,
            button: PointerEventButton::Left,
        },
        WindowEvent::PointerReleased {
            position,
            button: PointerEventButton::Left,
        },
    ] {
        w.window().dispatch_event(e);
    }
}
fn button(label: &str) {
    let w = window();
    let e = ElementHandle::find_by_accessible_label(&w, label)
        .find(|e| e.size().width > 0.)
        .unwrap_or_else(|| panic!("missing actual {label} button"));
    click(&e);
}
fn edit(index: usize, draft: &str) {
    let w = window();
    let fs = w
        .root_element()
        .query_descendants()
        .match_inherits("TextField")
        .find_all();
    assert!(fs.len() > index, "actual Material field {index} missing");
    click(&fs[index]);
    w.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Control.into(),
    });
    text("a");
    w.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Control.into(),
    });
    key(Key::Backspace);
    if !draft.is_empty() {
        text(draft);
    }
}
fn turn() {
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(250));
}
fn cells() -> Vec<kawaro_ui::HiveCell> {
    vec![kawaro_ui::HiveCell {
        id: "image.resize-50".into(),
        code: "50%".into(),
        label: "Resize".into(),
        icon: "resize".into(),
        ring: 2,
        enabled: true,
        reason: String::new(),
        customize: true,
    }]
}
fn customize() {
    let w = window();
    let e = ElementHandle::find_by_accessible_label(&w, "Resize")
        .next()
        .expect("real resize cell");
    let p = e.absolute_position();
    let s = e.size();
    w.window().dispatch_event(WindowEvent::PointerMoved {
        position: slint::LogicalPosition::new(p.x + s.width / 2., p.y + s.height / 2.),
    });
    w.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Shift.into(),
    });
    key(Key::Return);
    w.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Shift.into(),
    });
    turn();
    assert!(
        w.get_stage_two(),
        "production Customize callback opens panel"
    );
}

#[test]
fn ac1_ac3_ac4_ac6_back_invalid_then_exact_run_uses_retained_files_once() {
    i_slint_backend_testing::init_no_event_loop();
    let dir = tempfile::tempdir().unwrap();
    let photo = dir.path().join("original.png");
    std::fs::copy(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/fixtures/images/kawaro-sample.png"
        ),
        &photo,
    )
    .unwrap();
    let original = std::fs::read(&photo).unwrap();
    let files = vec![photo];
    let mut st = crate::state::State::new(Config::default(), &[]);
    st.available = ["image".into()].into();
    st.pending = Some(files.clone());
    let shared: Shared = Arc::new(Mutex::new(st));
    let (tx, rx) = mpsc::channel();
    let (etx, erx) = mpsc::channel();
    let real = kawaro_engines::all()
        .into_iter()
        .find(|e| e.id() == "image")
        .unwrap();
    let queue = Arc::new(Queue::new(
        vec![Arc::new(ObservedEngine { real, tx })],
        1,
        etx,
    ));
    HIVE.with(|slot| *slot.borrow_mut() = Some(Hive::new(Lang::En).unwrap()));
    HIVE.with(|slot| {
        let slot = slot.borrow();
        let h = slot.as_ref().unwrap();
        wire_choose(h, &shared, &queue, std::rc::Rc::new(|_| {}));
        wire_options(h, &shared, &queue);
        h.present_files("original.png", &files, cells(), None)
            .unwrap();
    });
    turn();
    customize();
    edit(0, "25");
    button("Back");
    turn();
    assert!(
        window().window().is_visible(),
        "Back restores visible actions"
    );
    assert_eq!(lock(&shared).pending.as_ref(), Some(&files));
    assert!(lock(&shared).jobs.is_empty());
    assert!(rx.try_recv().is_err());
    customize();
    edit(0, "abc");
    button("Run");
    assert!(lock(&shared).jobs.is_empty());
    assert_eq!(lock(&shared).pending.as_ref(), Some(&files));
    assert!(rx.try_recv().is_err());
    edit(0, "37");
    key(Key::Return);
    assert!(lock(&shared).jobs.is_empty(), "field Return is not Run");
    button("Run");
    let actual = rx
        .recv_timeout(Duration::from_secs(15))
        .expect("one real engine task");
    assert_eq!(
        actual,
        ("image.resize-50".into(), files.clone(), Some(37), None)
    );
    assert_eq!(lock(&shared).jobs.len(), 1);
    assert!(lock(&shared).pending.is_none());
    window().invoke_stage_run();
    key(Key::Return);
    assert_eq!(lock(&shared).jobs.len(), 1);
    loop {
        if matches!(
            erx.recv_timeout(Duration::from_secs(15)).unwrap(),
            kawaro_core::queue::JobEvent::Finished { .. }
        ) {
            break;
        }
    }
    assert!(
        rx.try_recv().is_err(),
        "duplicates produce zero extra engine tasks"
    );
    assert_eq!(
        std::fs::read(&files[0]).unwrap(),
        original,
        "source unchanged"
    );
}

#[test]
fn ac1_ac4_escape_returns_once_then_terminal_escape_clears_daemon_selection() {
    i_slint_backend_testing::init_no_event_loop();
    let mut st = crate::state::State::new(Config::default(), &[]);
    st.available = ["image".into()].into();
    let files = vec![PathBuf::from("/selection/one.png")];
    st.pending = Some(files.clone());
    let shared: Shared = Arc::new(Mutex::new(st));
    let (tx, _rx) = mpsc::channel();
    let queue = Arc::new(Queue::new(kawaro_engines::all(), 1, tx));
    HIVE.with(|slot| *slot.borrow_mut() = Some(Hive::new(Lang::En).unwrap()));
    HIVE.with(|slot| {
        let slot = slot.borrow();
        let h = slot.as_ref().unwrap();
        wire_choose(h, &shared, &queue, std::rc::Rc::new(|_| {}));
        wire_options(h, &shared, &queue);
        h.present_files("selected", &files, cells(), None).unwrap();
    });
    turn();
    customize();
    key(Key::Escape);
    turn();
    assert!(window().window().is_visible());
    assert_eq!(lock(&shared).pending.as_ref(), Some(&files));
    assert!(lock(&shared).jobs.is_empty());
    key(Key::Escape);
    turn();
    assert!(!window().window().is_visible());
    assert!(lock(&shared).pending.is_none());
    assert!(lock(&shared).jobs.is_empty());
}
