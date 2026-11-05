//! S-139 AC3/AC6, ADR 0015. This pins a small proposed production seam, not a
//! fake translator. layer_keyboard::NativeKeyboard::handle(KeyboardEvent) returns
//! owned Slint WindowEvents. The actual wl_keyboard Dispatch MUST call this same
//! function and dispatch its output in order. Reviewer must check that callsite.
//! Keymap owns/closes its FD; format uses Wayland numeric format, size includes
//! NUL. Key codes are evdev. Enter records held keys without retyping them.
//! Leave also represents terminal teardown. Unknown/unmatched releases emit none.
//! No public/testing-hooks API or extra dependency is needed for these child tests.
use crate::layer_keyboard::{
    KeyboardEvent as E, MAX_KEYMAP_BYTES, NativeKeyboard, update_keyboard_capability,
};
use slint::platform::{Key, WindowEvent};
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::os::fd::{AsRawFd, OwnedFd};
use std::sync::atomic::{AtomicUsize, Ordering};

const US: &[u8] = include_bytes!("../../../tests/fixtures/keymaps/us.xkb");
const FR: &[u8] = include_bytes!("../../../tests/fixtures/keymaps/us-fr.xkb");

fn fd(bytes: &[u8]) -> OwnedFd {
    static ID: AtomicUsize = AtomicUsize::new(0);
    let path = std::path::PathBuf::from(std::env::var_os("HOME").unwrap()).join(format!(
        ".s139-keymap-{}-{}",
        std::process::id(),
        ID.fetch_add(1, Ordering::Relaxed)
    ));
    let mut f = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    std::fs::remove_file(path).unwrap();
    f.write_all(bytes).unwrap();
    f.seek(SeekFrom::Start(0)).unwrap();
    f.into()
}
fn map(k: &mut NativeKeyboard, bytes: &[u8]) {
    assert!(
        k.handle(E::Keymap {
            format: 1,
            fd: fd(bytes),
            size: bytes.len() as u32
        })
        .is_empty()
    );
}
fn ready(bytes: &[u8]) -> NativeKeyboard {
    let mut k = NativeKeyboard::default();
    map(&mut k, bytes);
    assert!(k.handle(E::Enter { held: vec![] }).is_empty());
    k
}
fn events(events: Vec<WindowEvent>) -> Vec<(bool, String)> {
    events
        .into_iter()
        .map(|e| match e {
            WindowEvent::KeyPressed { text } => (true, text.to_string()),
            WindowEvent::KeyReleased { text } => (false, text.to_string()),
            _ => panic!("keyboard dispatcher emitted a non-key event"),
        })
        .collect()
}
fn key(k: &mut NativeKeyboard, code: u32, pressed: bool) -> Vec<(bool, String)> {
    events(k.handle(E::Key { key: code, pressed }))
}
fn masks(
    k: &mut NativeKeyboard,
    depressed: u32,
    latched: u32,
    locked: u32,
    group: u32,
) -> Vec<(bool, String)> {
    events(k.handle(E::Modifiers {
        depressed,
        latched,
        locked,
        group,
    }))
}
fn tap(k: &mut NativeKeyboard, code: u32, text: &str) {
    assert_eq!(key(k, code, true), [(true, text.to_owned())]);
    assert_eq!(key(k, code, false), [(false, text.to_owned())]);
}
fn special(key: Key) -> String {
    let s: slint::SharedString = key.into();
    s.to_string()
}

#[test]
fn ac3_ac6_real_us_digits_letters_and_editing_keysyms() {
    let mut k = ready(US);
    assert!(key(&mut k, u32::MAX, true).is_empty());
    assert!(key(&mut k, u32::MAX, false).is_empty());
    for (code, text) in [(3, "2"), (6, "5"), (30, "a"), (47, "v")] {
        tap(&mut k, code, text);
    }
    for (code, symbol) in [
        (14, Key::Backspace),
        (111, Key::Delete),
        (15, Key::Tab),
        (28, Key::Return),
    ] {
        tap(&mut k, code, &special(symbol));
    }
    assert!(
        masks(&mut k, 1, 0, 0, 0)
            .iter()
            .any(|e| *e == (true, special(Key::Shift)))
    );
    tap(&mut k, 3, "@");
    tap(&mut k, 30, "A");
    masks(&mut k, 0, 0, 2, 0);
    tap(&mut k, 30, "A");
    masks(&mut k, 0, 1, 0, 0);
    tap(&mut k, 6, "%");
}

#[test]
fn ac3_ac6_real_french_group_shift_altgr_not_a_us_table() {
    let mut k = ready(FR);
    masks(&mut k, 0, 0, 0, 1);
    tap(&mut k, 3, "é");
    tap(&mut k, 6, "(");
    tap(&mut k, 16, "a");
    masks(&mut k, 1, 0, 0, 1);
    tap(&mut k, 3, "2");
    tap(&mut k, 6, "5");
    masks(&mut k, 128, 0, 0, 1);
    tap(&mut k, 18, "€");
    masks(&mut k, 0, 0, 0, 0);
    tap(&mut k, 16, "q");
}

#[test]
fn ac3_ac6_ctrl_shortcut_identity_and_modifier_order_without_control_text() {
    let mut k = ready(US);
    let modifiers = masks(&mut k, 5, 0, 0, 0);
    assert!(modifiers.contains(&(true, special(Key::Control))));
    assert!(modifiers.contains(&(true, special(Key::Shift))));
    for (code, text) in [(30, "A"), (47, "V")] {
        tap(&mut k, code, text);
    }
    masks(&mut k, 4, 0, 0, 0);
    for (code, text) in [(30, "a"), (47, "v"), (45, "x"), (46, "c")] {
        tap(&mut k, code, text);
    }
    let released = masks(&mut k, 0, 0, 0, 0);
    assert!(released.contains(&(false, special(Key::Control))));
    tap(&mut k, 3, "2");
}

#[test]
fn ac3_ac6_releases_use_press_text_across_masks_and_groups() {
    let mut k = ready(FR);
    masks(&mut k, 1, 0, 0, 0);
    assert_eq!(key(&mut k, 30, true), [(true, "A".into())]);
    masks(&mut k, 0, 0, 0, 1);
    assert_eq!(key(&mut k, 30, false), [(false, "A".into())]);
    assert_eq!(key(&mut k, 3, true), [(true, "é".into())]);
    masks(&mut k, 1, 0, 0, 0);
    assert_eq!(key(&mut k, 3, false), [(false, "é".into())]);
    assert!(key(&mut k, 3, false).is_empty());
}

#[test]
fn ac3_ac6_enter_held_leave_reenter_and_map_replace_clear_stale_state() {
    let mut k = ready(US);
    assert!(k.handle(E::Enter { held: vec![3, 30] }).is_empty());
    assert!(key(&mut k, 3, false).is_empty());
    tap(&mut k, 6, "5");
    for _ in 0..3 {
        masks(&mut k, 5, 0, 0, 0);
        key(&mut k, 30, true);
        k.handle(E::Leave);
        assert!(k.handle(E::Enter { held: vec![] }).is_empty());
        assert!(key(&mut k, 30, false).is_empty());
        tap(&mut k, 3, "2");
        masks(&mut k, 1, 0, 0, 0);
        key(&mut k, 30, true);
        map(&mut k, FR);
        assert!(key(&mut k, 30, false).is_empty());
        tap(&mut k, 3, "2");
    }
    k.handle(E::Leave);
    assert!(
        key(&mut k, 6, true).is_empty(),
        "unfocused input must not reach a new session"
    );
}

// AC3/AC6 lifecycle addendum: this MUST be the production wl_seat::Capabilities
// seam, not a test-only model. Its WlKeyboard release closure checks version >= 3.
// Signature: (bool, &mut Option<T>, &mut NativeKeyboard, FnOnce() -> T,
// FnOnce(T)) -> Vec<WindowEvent>. The real callback forwards these events in order.
#[test]
fn ac3_ac6_keyboard_capability_loss_releases_resets_and_reacquires() {
    use std::cell::{Cell, RefCell};

    let acquired = Cell::new(0_u32);
    let released = RefCell::new(Vec::new());
    let update = |present, current: &mut Option<u32>, keys: &mut NativeKeyboard| {
        update_keyboard_capability(
            present,
            current,
            keys,
            || {
                acquired.set(acquired.get() + 1);
                acquired.get()
            },
            |device| released.borrow_mut().push(device),
        )
    };
    let mut current = None;
    let mut k = NativeKeyboard::default();
    assert!(update(true, &mut current, &mut k).is_empty());
    assert_eq!(current, Some(1));
    assert!(update(true, &mut current, &mut k).is_empty());
    assert_eq!(acquired.get(), 1, "existing device must not be reacquired");
    map(&mut k, US);
    assert!(k.handle(E::Enter { held: vec![] }).is_empty());
    assert_eq!(masks(&mut k, 4, 0, 0, 0), [(true, special(Key::Control))]);
    assert_eq!(key(&mut k, 30, true), [(true, "a".into())]);

    let mut loss = update(false, &mut current, &mut k);
    assert_eq!(current, None);
    assert_eq!(*released.borrow(), [1]);
    assert!(matches!(
        loss.pop(),
        Some(WindowEvent::WindowActiveChanged(false))
    ));
    let mut resets = events(loss);
    resets.sort();
    let mut expected = vec![(false, special(Key::Control)), (false, "a".into())];
    expected.sort();
    assert_eq!(
        resets, expected,
        "release each held input once before focus loss"
    );
    assert!(key(&mut k, 30, false).is_empty(), "ignore old release");
    assert!(
        key(&mut k, 6, true).is_empty(),
        "disabled until a fresh Enter"
    );
    // A redundant inactive notification is harmless; duplicate input releases are not.
    for event in update(false, &mut current, &mut k) {
        assert!(matches!(event, WindowEvent::WindowActiveChanged(false)));
    }
    assert_eq!(*released.borrow(), [1], "never release an absent proxy");
    assert_eq!(acquired.get(), 1);

    assert!(update(true, &mut current, &mut k).is_empty());
    assert_eq!(current, Some(2));
    assert_eq!(acquired.get(), 2);
    assert!(update(true, &mut current, &mut k).is_empty());
    assert_eq!(acquired.get(), 2);
    map(&mut k, US);
    assert!(key(&mut k, 3, true).is_empty(), "acquisition is not focus");
    assert!(k.handle(E::Enter { held: vec![] }).is_empty());
    assert!(
        key(&mut k, 30, false).is_empty(),
        "no cached old press after regain"
    );
    tap(&mut k, 3, "2");
    assert_eq!(masks(&mut k, 4, 0, 0, 0), [(true, special(Key::Control))]);
    tap(&mut k, 47, "v");
    assert_eq!(masks(&mut k, 0, 0, 0, 0), [(false, special(Key::Control))]);

    // Even without a proxy, loss must clear real translator state, not early-return.
    let mut no_proxy = None;
    assert_eq!(masks(&mut k, 4, 0, 0, 0), [(true, special(Key::Control))]);
    assert_eq!(key(&mut k, 30, true), [(true, "a".into())]);
    let mut loss = update(false, &mut no_proxy, &mut k);
    assert!(matches!(
        loss.pop(),
        Some(WindowEvent::WindowActiveChanged(false))
    ));
    let mut resets = events(loss);
    resets.sort();
    assert_eq!(resets, expected);
    assert!(key(&mut k, 30, false).is_empty());
    assert!(key(&mut k, 3, true).is_empty());
    assert_eq!(*released.borrow(), [1]);
    assert_eq!(acquired.get(), 2);
    // No GUI, queue or job API is called: only owned dummy devices and NativeKeyboard.
}

#[test]
fn ac3_ac6_bad_keymap_fds_are_consumed_clear_state_and_never_guess_text() {
    assert!(MAX_KEYMAP_BYTES >= FR.len() && MAX_KEYMAP_BYTES <= 16 * 1024 * 1024);
    let missing_nul = &US[..US.len() - 1];
    let cases: Vec<(u32, Vec<u8>, u32)> = vec![
        (0, US.to_vec(), US.len() as u32),
        (99, US.to_vec(), US.len() as u32),
        (1, vec![], 0),
        (1, vec![], MAX_KEYMAP_BYTES as u32 + 1),
        (1, US[..80].to_vec(), US.len() as u32),
        (1, missing_nul.to_vec(), missing_nul.len() as u32),
        (
            1,
            b"not an xkb keymap\0".to_vec(),
            b"not an xkb keymap\0".len() as u32,
        ),
        (1, vec![255, 0], 2),
    ];
    for (format, bytes, size) in cases {
        let mut k = ready(US);
        key(&mut k, 3, true);
        let owned = fd(&bytes);
        let raw = owned.as_raw_fd();
        let observer = std::path::PathBuf::from(format!("/proc/self/fd/{raw}"));
        let original_target = std::fs::read_link(&observer).unwrap();
        assert!(
            k.handle(E::Keymap {
                format,
                fd: owned,
                size
            })
            .is_empty()
        );
        assert!(
            std::fs::read_link(&observer).ok().as_ref() != Some(&original_target),
            "keymap FD leaked, format={format}, size={size}"
        );
        assert!(key(&mut k, 3, false).is_empty());
        assert!(
            key(&mut k, 3, true).is_empty(),
            "bad map must not retain or guess printable text"
        );
        map(&mut k, US);
        tap(&mut k, 3, "2");
    }
    // An owned write-only descriptor is valid but cannot be read.
    let mut k = ready(US);
    let owned: OwnedFd = OpenOptions::new()
        .write(true)
        .open("/dev/null")
        .unwrap()
        .into();
    let raw = owned.as_raw_fd();
    k.handle(E::Keymap {
        format: 1,
        fd: owned,
        size: US.len() as u32,
    });
    assert!(!std::path::Path::new(&format!("/proc/self/fd/{raw}")).exists());
    assert!(key(&mut k, 3, true).is_empty());
    // A directory is also an owned FD whose read fails, without unsafe invalid ownership.
    let directory: OwnedFd = File::open("/").unwrap().into();
    k.handle(E::Keymap {
        format: 1,
        fd: directory,
        size: 20,
    });
    assert!(key(&mut k, 3, true).is_empty());
}

#[test]
fn ac3_ac6_legacy_nine_key_fallback_contract_is_unchanged() {
    for (code, symbol) in [
        (1, Key::Escape),
        (28, Key::Return),
        (96, Key::Return),
        (57, Key::Space),
        (15, Key::Tab),
        (103, Key::UpArrow),
        (108, Key::DownArrow),
        (105, Key::LeftArrow),
        (106, Key::RightArrow),
    ] {
        assert_eq!(crate::overlay::key_for_evdev(code), Some(symbol));
        let mut k = NativeKeyboard::default();
        k.handle(E::Enter { held: vec![] });
        tap(&mut k, code, &special(symbol));
    }
    for code in [0, 3, 6, 30, 47, u32::MAX] {
        assert_eq!(crate::overlay::key_for_evdev(code), None);
    }
}
