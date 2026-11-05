//! The compositor mask is authoritative, never update it from physical keys.
use slint::platform::{Key, WindowEvent};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Read;
use std::os::fd::OwnedFd;
use xkbcommon::xkb;

/// Finite allocation/read bound including the wire NUL.
pub(crate) const MAX_KEYMAP_BYTES: usize = 16 * 1024 * 1024;
pub(crate) enum KeyboardEvent {
    Keymap {
        format: u32,
        fd: OwnedFd,
        size: u32,
    },
    Modifiers {
        depressed: u32,
        latched: u32,
        locked: u32,
        group: u32,
    },
    Enter {
        held: Vec<u32>,
    },
    Leave,
    Key {
        key: u32,
        pressed: bool,
    },
}
#[derive(Default)]
pub(crate) struct NativeKeyboard {
    state: Option<xkb::State>,
    received_map: bool,
    focused: bool,
    pressed: BTreeMap<u32, slint::SharedString>,
    held: BTreeSet<u32>,
    modifiers: [bool; 4],
}
const MODIFIERS: [(&str, Key); 4] = [
    (xkb::MOD_NAME_SHIFT, Key::Shift),
    (xkb::MOD_NAME_CTRL, Key::Control),
    (xkb::MOD_NAME_ALT, Key::Alt),
    (xkb::MOD_NAME_LOGO, Key::Meta),
];
fn event(text: slint::SharedString, pressed: bool) -> WindowEvent {
    if pressed {
        WindowEvent::KeyPressed { text }
    } else {
        WindowEvent::KeyReleased { text }
    }
}
impl NativeKeyboard {
    pub(crate) fn reset(&mut self) -> Vec<WindowEvent> {
        let mut out: Vec<_> = std::mem::take(&mut self.pressed)
            .into_values()
            .map(|text| event(text, false))
            .collect();
        self.held.clear();
        for (i, (_, key)) in MODIFIERS.iter().enumerate() {
            if self.modifiers[i] {
                out.push(event((*key).into(), false));
            }
        }
        self.modifiers = [false; 4];
        if let Some(state) = &mut self.state {
            state.update_mask(0, 0, 0, 0, 0, 0);
        }
        out
    }
    pub(crate) fn handle(&mut self, input: KeyboardEvent) -> Vec<WindowEvent> {
        match input {
            KeyboardEvent::Keymap { format, fd, size } => {
                self.reset();
                self.received_map = true;
                self.state = None;
                // Safe ownership conversion closes the FD on every exit. No mmap.
                let mut file = File::from(fd);
                let size = size as usize;
                if format != 1 || size == 0 || size > MAX_KEYMAP_BYTES {
                    return vec![];
                }
                let mut bytes = vec![0; size];
                if file.read_exact(&mut bytes).is_err()
                    || bytes.pop() != Some(0)
                    || bytes.contains(&0)
                {
                    return vec![];
                }
                let Ok(text) = String::from_utf8(bytes) else {
                    return vec![];
                };
                let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
                if let Some(map) = xkb::Keymap::new_from_string(
                    &context,
                    text,
                    xkb::KEYMAP_FORMAT_TEXT_V1,
                    xkb::KEYMAP_COMPILE_NO_FLAGS,
                ) {
                    self.state = Some(xkb::State::new(&map));
                }
                vec![]
            }
            KeyboardEvent::Enter { held } => {
                let out = self.reset();
                self.focused = true;
                self.held.extend(held);
                out
            }
            KeyboardEvent::Leave => {
                self.focused = false;
                self.reset()
            }
            KeyboardEvent::Modifiers {
                depressed,
                latched,
                locked,
                group,
            } => {
                let Some(state) = &mut self.state else {
                    return vec![];
                };
                state.update_mask(depressed, latched, locked, 0, 0, group);
                let mut out = vec![];
                if self.focused {
                    for (i, (name, key)) in MODIFIERS.iter().enumerate() {
                        let active = state.mod_name_is_active(name, xkb::STATE_MODS_EFFECTIVE);
                        if active != self.modifiers[i] {
                            out.push(event((*key).into(), active));
                            self.modifiers[i] = active;
                        }
                    }
                }
                out
            }
            KeyboardEvent::Key { key, pressed } => {
                if !self.focused {
                    return vec![];
                }
                if self.held.contains(&key) {
                    if !pressed {
                        self.held.remove(&key);
                    }
                    return vec![];
                }
                if !pressed {
                    return self
                        .pressed
                        .remove(&key)
                        .map(|text| vec![event(text, false)])
                        .unwrap_or_default();
                }
                if let Some(text) = self.pressed.get(&key) {
                    return vec![event(text.clone(), true)];
                }
                let text: slint::SharedString = if let Some(state) = &self.state {
                    let Some(code) = key.checked_add(8) else {
                        return vec![];
                    };
                    let sym = state.key_get_one_sym(code.into());
                    let named = match sym.raw() {
                        xkb::keysyms::KEY_Escape => Some(Key::Escape),
                        xkb::keysyms::KEY_Return | xkb::keysyms::KEY_KP_Enter => Some(Key::Return),
                        xkb::keysyms::KEY_Tab | xkb::keysyms::KEY_ISO_Left_Tab => Some(Key::Tab),
                        xkb::keysyms::KEY_BackSpace => Some(Key::Backspace),
                        xkb::keysyms::KEY_Delete | xkb::keysyms::KEY_KP_Delete => Some(Key::Delete),
                        xkb::keysyms::KEY_Left => Some(Key::LeftArrow),
                        xkb::keysyms::KEY_Right => Some(Key::RightArrow),
                        xkb::keysyms::KEY_Up => Some(Key::UpArrow),
                        xkb::keysyms::KEY_Down => Some(Key::DownArrow),
                        xkb::keysyms::KEY_Home => Some(Key::Home),
                        xkb::keysyms::KEY_End => Some(Key::End),
                        xkb::keysyms::KEY_Page_Up => Some(Key::PageUp),
                        xkb::keysyms::KEY_Page_Down => Some(Key::PageDown),
                        xkb::keysyms::KEY_Insert => Some(Key::Insert),
                        _ => None,
                    };
                    if let Some(named) = named {
                        named.into()
                    } else {
                        // keysym UTF-8 retains Ctrl shortcut identity, unlike state UTF-8.
                        let text = xkb::keysym_to_utf8(sym);
                        if text.is_empty() || text.chars().any(char::is_control) {
                            return vec![];
                        }
                        text.into()
                    }
                } else if !self.received_map {
                    let Some(named) = crate::overlay::key_for_evdev(key) else {
                        return vec![];
                    };
                    named.into()
                } else {
                    return vec![];
                };
                self.pressed.insert(key, text.clone());
                vec![event(text, true)]
            }
        }
    }
}

/// The actual seat capability transition, shared with the lifecycle regression.
/// Acquisition does not imply keyboard focus; disappearance releases all Slint
/// input state before making the old keyboard proxy unavailable.
pub(crate) fn update_keyboard_capability<T>(
    present: bool,
    current: &mut Option<T>,
    keys: &mut NativeKeyboard,
    acquire: impl FnOnce() -> T,
    release: impl FnOnce(T),
) -> Vec<WindowEvent> {
    if present {
        if current.is_none() {
            *current = Some(acquire());
        }
        return Vec::new();
    }
    if let Some(device) = current.take() {
        release(device);
    }
    let mut events = keys.handle(KeyboardEvent::Leave);
    events.push(WindowEvent::WindowActiveChanged(false));
    events
}
