// S-140 AC1/AC2/AC3: real Gate, Hive terminal callbacks and Queue.
// No new cancel type: generated Escape may send OtherKey followed by Release.
// The signal delivery below mirrors hold_signal after its event-loop hop; it
// retires Gate ownership before key_released can call wire_options/gate_rearm.
// Stamped receiver notes are private to kawaro-ui, covered by its retained S136
// tests. Unstamped feed_drop is intentionally NOT presented as a stale-A test.
use super::*;
use kawaro_core::config::DropMode;
use kawaro_core::engine::{CancelToken, Engine, EngineError, Progress, Task, TaskOutcome};
use kawaro_ui::Hive;
use slint::ComponentHandle;
use std::sync::mpsc;

struct RecordingEngine {
    real: Arc<dyn Engine>,
    tx: mpsc::Sender<(String, Vec<PathBuf>, Option<i64>)>,
}
impl Engine for RecordingEngine {
    fn id(&self) -> &'static str {
        self.real.id()
    }
    fn available(&self) -> bool {
        self.real.available()
    }
    fn capabilities(&self) -> Vec<(&'static str, Vec<kawaro_core::MediaKind>)> {
        self.real.capabilities()
    }
    fn run(
        &self,
        task: &Task,
        progress: Progress,
        cancel: &CancelToken,
    ) -> Result<TaskOutcome, EngineError> {
        self.tx
            .send((
                task.action_id.clone(),
                task.inputs.clone(),
                task.param_i64("scale"),
            ))
            .unwrap();
        self.real.run(task, progress, cancel)
    }
}
fn with_hive<T>(f: impl FnOnce(&Hive) -> T) -> T {
    HIVE.with(|slot| f(slot.borrow().as_ref().unwrap()))
}
fn signal(signal: Signal) {
    let effect = GATE.with(|g| {
        g.borrow_mut()
            .as_mut()
            .unwrap()
            .signal(signal, Instant::now())
    });
    if effect == Some(Effect::Release) {
        with_hive(Hive::key_released);
    }
}
fn show(state: &Shared) {
    GATE_TIMER.with(|t| t.stop());
    rearmed_clear();
    GATE.with(|g| *g.borrow_mut() = Some(Gate::with_max(|| 1, || 10_000)));
    GATE.with(|g| {
        g.borrow_mut().as_mut().unwrap().signal(
            Signal::Hold { at: None },
            Instant::now() - Duration::from_millis(20),
        );
    });
    gate_tick(state);
}
fn finish() {
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(250));
}
fn cells() -> Vec<HiveCell> {
    let registry = kawaro_core::action::Registry::builtin();
    vec![cell(registry.get("image.resize-50").unwrap(), Lang::En)]
}

fn exercise(escape: bool) {
    i_slint_backend_testing::init_no_event_loop();
    for mode in [DropMode::ChooseDuringDrag, DropMode::DropFirst] {
        for entered in [false, true] {
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
            let files = vec![photo.clone()];
            let mut st = crate::state::State::new(Config::default(), &[]);
            st.config.drop_mode = mode;
            st.available = ["image".into()].into();
            let shared: Shared = Arc::new(std::sync::Mutex::new(st));
            let (tx, rx) = mpsc::channel();
            let (events, event_rx) = mpsc::channel();
            let real = kawaro_engines::all()
                .into_iter()
                .find(|e| e.id() == "image")
                .unwrap();
            let queue = Arc::new(Queue::new(
                vec![Arc::new(RecordingEngine { real, tx })],
                1,
                events,
            ));
            HIVE.with(|slot| *slot.borrow_mut() = Some(Hive::new(Lang::En).unwrap()));
            let handoffs = Rc::new(std::cell::Cell::new(0));
            with_hive(|h| {
                wire_choose(
                    h,
                    &shared,
                    &queue,
                    Rc::new(|_| panic!("unexpected external open")),
                );
                wire_options(h, &shared, &queue);
                let state = shared.clone();
                h.on_drop_files(move |files| {
                    DRAG_SEEN.with(|d| d.set(true));
                    lock(&state).pending = Some(files);
                    ("1 image".into(), cells())
                });
                let count = handoffs.clone();
                let state = shared.clone();
                wire_drop_first(
                    h,
                    &shared,
                    Rc::new(move |files, _| {
                        count.set(count.get() + 1);
                        lock(&state).pending = Some(files.to_vec());
                        with_hive(|h| h.present_files("1 image", files, cells(), None).unwrap());
                    }),
                );
            });
            show(&shared);
            if entered {
                with_hive(|h| h.feed_enter(files.clone()));
            }
            // Ordinary keys after Shown must not cancel, including the first
            // command of the explicit Escape route. Timing still cancels below.
            signal(Signal::OtherKey);
            assert!(!GATE.with(|g| g.borrow().as_ref().unwrap().is_idle()));
            if !escape {
                assert_eq!(lock(&shared).pending.is_some(), entered);
                assert!(lock(&shared).jobs.is_empty());
            }
            signal(Signal::Release);
            // Drop arriving after terminal release cannot commit or hand off.
            with_hive(|h| h.feed_drop(Some(0)));
            finish();
            assert!(GATE.with(|g| g.borrow().as_ref().unwrap().is_idle()));
            assert!(
                !REARMED.with(|r| r.get()),
                "explicit termination is not pointer-hide rearm"
            );
            assert!(lock(&shared).pending.is_none());
            assert!(lock(&shared).jobs.is_empty());
            assert!(rx.try_recv().is_err());
            assert_eq!(handoffs.get(), 0);
            assert!(!with_hive(|h| h.testing_window().window().is_visible()));
            gate_tick(&shared);
            assert!(!with_hive(|h| h.testing_window().window().is_visible()));
            assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);

            // Fresh B recovers once with the unchanged default action payload.
            show(&shared);
            with_hive(|h| {
                h.feed_enter(files.clone());
                h.feed_drop(Some(0));
            });
            if mode == DropMode::DropFirst {
                assert_eq!(handoffs.get(), 1);
                assert!(lock(&shared).jobs.is_empty());
                signal(Signal::Release); // old hold cannot dismiss normal hive
                with_hive(Hive::key_released);
                assert_eq!(lock(&shared).pending.as_ref(), Some(&files));
                assert!(with_hive(|h| h.testing_window().window().is_visible()));
                assert!(with_hive(|h| h.click_cell("image.resize-50")));
            }
            let recorded = rx
                .recv_timeout(Duration::from_secs(5))
                .expect("fresh drag reaches the actual engine");
            assert_eq!(
                recorded,
                ("image.resize-50".into(), files.clone(), Some(50))
            );
            assert_eq!(lock(&shared).jobs.len(), 1);
            loop {
                if matches!(
                    event_rx.recv_timeout(Duration::from_secs(5)).unwrap(),
                    kawaro_core::queue::JobEvent::Finished { .. }
                ) {
                    break;
                }
            }
            assert_eq!(
                std::fs::read_dir(dir.path()).unwrap().count(),
                2,
                "one recovered output"
            );
            with_hive(|h| h.feed_drop(Some(0)));
            assert!(rx.try_recv().is_err());
            assert_eq!(handoffs.get(), u32::from(mode == DropMode::DropFirst));
            assert_eq!(std::fs::read(&photo).unwrap(), original);
            with_hive(Hive::close);
            GATE_TIMER.with(|t| t.stop());
        }
    }
    // The existing pre-threshold chord rule must survive any Escape fix.
    GATE.with(|g| *g.borrow_mut() = Some(Gate::with_max(|| 400, || 10_000)));
    signal(Signal::Hold { at: None });
    signal(Signal::OtherKey);
    signal(Signal::Release);
    assert!(GATE.with(|g| g.borrow().as_ref().unwrap().is_idle()));
}
#[test]
fn ac1_ac3_explicit_escape_command_pair_cancels_probe_and_shown_then_recovers_once() {
    exercise(true);
}
#[test]
fn ac2_ac3_ordinary_key_preserves_shown_until_release_then_recovers_once() {
    exercise(false);
}
