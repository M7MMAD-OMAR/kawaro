// S-136 daemon held-mode test.
// Contract pinned here (tester): Hive::drop_mode() -> DropMode and gate_tick
// applies the configured mode to the held show.

use super::*;
use kawaro_core::config::DropMode;
use kawaro_core::hold::Signal;
use kawaro_ui::Hive;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

struct EnvGuard {
    home: Option<std::ffi::OsString>,
    xdg: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn set(dir: &std::path::Path) -> Self {
        let guard = EnvGuard {
            home: std::env::var_os("HOME"),
            xdg: std::env::var_os("XDG_CONFIG_HOME"),
        };
        unsafe {
            std::env::set_var("HOME", dir);
            std::env::set_var("XDG_CONFIG_HOME", dir);
        }
        guard
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            match self.home.take() {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
            match self.xdg.take() {
                Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
        }
    }
}

static ENV: Mutex<()> = Mutex::new(());

fn write_mode(dir: &std::path::Path, mode: &str) {
    let path = dir.join("kawaro").join("config.toml");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let text = format!("drop_mode = \"{mode}\"\n");
    std::fs::write(&path, text).unwrap();
}

/// The production shape: an empty state, then the 2 s poll's reload after
/// the config file changed, before the next show.
fn state() -> Shared {
    let mut st = crate::state::State::new(Config::default(), &[]);
    st.available = ["image".to_string()].into();
    Arc::new(Mutex::new(st))
}

fn held_show_with_mode(mode: &str) -> DropMode {
    let dir = tempfile::tempdir().unwrap();
    write_mode(dir.path(), mode);
    let _env = EnvGuard::set(dir.path());
    let shared = state();
    lock(&shared).reload();
    assert_eq!(
        lock(&shared).config.drop_mode,
        if mode == "drop-first" {
            DropMode::DropFirst
        } else {
            DropMode::ChooseDuringDrag
        },
        "the reload picked up {mode} before the show"
    );
    HIVE.with(|h| *h.borrow_mut() = Some(Hive::new(Lang::En).unwrap()));
    GATE.with(|g| *g.borrow_mut() = Some(Gate::with_max(|| 1, || 10_000)));
    let past = Instant::now() - Duration::from_millis(50);
    GATE.with(|g| {
        g.borrow_mut()
            .as_mut()
            .unwrap()
            .signal(Signal::Hold { at: None }, past);
    });
    gate_tick(&shared);
    HIVE.with(|h| h.borrow().as_ref().unwrap().drop_mode())
}

#[test]
fn ac3_ac4_a_held_show_captures_the_configured_mode() {
    i_slint_backend_testing::init_no_event_loop();
    let _lock = ENV.lock().unwrap();
    assert_eq!(
        held_show_with_mode("drop-first"),
        DropMode::DropFirst,
        "the held show applies the configured drop-first mode"
    );
    assert_eq!(
        held_show_with_mode("choose-during-drag"),
        DropMode::ChooseDuringDrag,
        "a reload applies at the next held show without a restart"
    );
}

#[test]
fn ac5_the_daemon_handoff_opens_once_and_keeps_pending_until_the_choice() {
    i_slint_backend_testing::init_no_event_loop();
    let hive = Hive::new(Lang::En).unwrap();
    HIVE.with(|h| *h.borrow_mut() = Some(hive));
    let shared = state();
    let (tx, _rx) = std::sync::mpsc::channel();
    let queue = Arc::new(Queue::new(kawaro_engines::all(), 1, tx));
    let presented = Rc::new(std::cell::Cell::new(0u32));
    let dismissed = Rc::new(std::cell::Cell::new(0u32));
    let files = vec![std::path::PathBuf::from("/tmp/s136-a.png")];

    let p = presented.clone();
    let s = shared.clone();
    let presenter = Rc::new(
        move |files: &[std::path::PathBuf], _at: Option<(f64, f64)>| {
            p.set(p.get() + 1);
            let files = files.to_vec();
            HIVE.with(|slot| {
                let slot = slot.borrow();
                let Some(hive) = slot.as_ref() else { return };
                lock(&s).pending = Some(files.clone());
                hive.present_files("1 image", &files, target_cells(), None)
                    .unwrap();
            });
        },
    );

    HIVE.with(|slot| {
        let slot = slot.borrow();
        let hive = slot.as_ref().unwrap();
        wire_choose(hive, &shared, &queue, Rc::new(|_url: &str| {}));
        wire_drop_first(hive, &shared, presenter.clone());
        let d = dismissed.clone();
        hive.on_dismiss(move || d.set(d.get() + 1));
        hive.on_drop_files(|_files| ("1 image".to_string(), target_cells()));
        hive.set_drop_mode(DropMode::DropFirst);
        hive.present_drop("", None).unwrap();
        hive.feed_enter(files.clone());
        hive.feed_drop(None);
    });

    assert_eq!(presented.get(), 1, "exactly one handoff presentation");
    assert_eq!(lock(&shared).jobs.len(), 0, "no job before the choice");
    assert_eq!(
        lock(&shared).pending.as_ref(),
        Some(&files),
        "pending waits"
    );
    let after_handoff = dismissed.get();

    // The old hold release cannot retire the handoff or clear pending.
    HIVE.with(|h| h.borrow().as_ref().unwrap().key_released());
    assert_eq!(
        dismissed.get(),
        after_handoff,
        "the old release does not dismiss the handed-off hive"
    );
    assert_eq!(lock(&shared).pending.as_ref(), Some(&files));

    // The normal choice submits exactly once.
    let clicked = HIVE.with(|h| h.borrow().as_ref().unwrap().click_cell("image.to-webp"));
    assert!(clicked, "the normal hive has the action cell");
    assert_eq!(lock(&shared).jobs.len(), 1, "exactly one submission");
    assert!(
        lock(&shared).pending.is_none(),
        "the choice consumes pending"
    );
    HIVE.with(|h| h.borrow().as_ref().unwrap().click_cell("image.to-webp"));
    assert_eq!(lock(&shared).jobs.len(), 1, "a second click adds nothing");
    assert_eq!(presented.get(), 1, "a late handoff never runs");
}

fn target_cells() -> Vec<kawaro_ui::HiveCell> {
    vec![kawaro_ui::HiveCell {
        id: "image.to-webp".to_string(),
        code: "WEBP".to_string(),
        label: "WEBP".to_string(),
        icon: String::new(),
        ring: 1,
        enabled: true,
        reason: String::new(),
        customize: false,
    }]
}
