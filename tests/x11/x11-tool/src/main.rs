// SPDX-License-Identifier: GPL-3.0-or-later
//! Test helper for `tests/x11/run.sh` (S-114). It talks to the X server named
//! by `DISPLAY` and offers five subcommands:
//!
//! - `move X Y`: moves the pointer through XTEST, then reads it back with
//!   `QueryPointer` and prints `X Y`; exits 1 when the read back differs.
//! - `key NAME`: presses and releases a key through XTEST; `button1` is a
//!   click of the left pointer button at the current pointer position.
//! - `geometry WINDOW_NAME`: prints `X Y W H` of the viewable window with that
//!   name, in root coordinates (`TranslateCoordinates`).
//! - `pixel X Y`: prints `RRGGBB` of the root screen pixel.
//! - `stacking`: prints top level window names, top to bottom, one per line.
//! - `down NAME` and `up NAME`: only the press or only the release of a key or
//!   of `button1`, to hold it while the pointer moves (S-115 drags).

use std::process::ExitCode;

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{Atom, AtomEnum, ConnectionExt, ImageFormat, MapState, Window};
use x11rb::protocol::xtest::ConnectionExt as _;
use x11rb::rust_connection::RustConnection;

type Res<T> = Result<T, Box<dyn std::error::Error>>;

const KEY_PRESS: u8 = 2;
const KEY_RELEASE: u8 = 3;
const BUTTON_PRESS: u8 = 4;
const BUTTON_RELEASE: u8 = 5;
const MOTION_NOTIFY: u8 = 6;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("x11-tool: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Res<()> {
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    let (conn, screen_num) = x11rb::connect(None)?;
    let root = conn.setup().roots[screen_num].root;
    match words.as_slice() {
        ["move", x, y] => move_pointer(&conn, root, x.parse()?, y.parse()?),
        ["key", name] => key(&conn, root, name),
        ["geometry", name] => geometry(&conn, root, name),
        ["pixel", x, y] => pixel(&conn, screen_num, root, x.parse()?, y.parse()?),
        ["stacking"] => stacking(&conn, root),
        ["down", name] => press(&conn, root, name, true),
        ["up", name] => press(&conn, root, name, false),
        _ => Err(
            "usage: x11-tool move X Y | key NAME | down NAME | up NAME | geometry WINDOW_NAME | pixel X Y | stacking"
                .into(),
        ),
    }
}

fn move_pointer(conn: &RustConnection, root: Window, x: i16, y: i16) -> Res<()> {
    conn.xtest_fake_input(MOTION_NOTIFY, 0, 0, root, x, y, 0)?;
    conn.get_input_focus()?.reply()?;
    let p = conn.query_pointer(root)?.reply()?;
    println!("{} {}", p.root_x, p.root_y);
    if (p.root_x, p.root_y) == (x, y) {
        Ok(())
    } else {
        Err(format!("pointer is at {} {}, wanted {x} {y}", p.root_x, p.root_y).into())
    }
}

fn keysym(name: &str) -> Option<u32> {
    let lower = name.to_ascii_lowercase();
    let named = match lower.as_str() {
        "escape" | "esc" => 0xff1b,
        "return" | "enter" => 0xff0d,
        "tab" => 0xff09,
        "space" => 0x20,
        "backspace" => 0xff08,
        "alt" | "alt_l" => 0xffe9,
        "shift" | "shift_l" => 0xffe1,
        _ => 0,
    };
    if named != 0 {
        return Some(named);
    }
    let mut chars = name.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_graphic() => Some(c as u32),
        _ => None,
    }
}

fn key(conn: &RustConnection, root: Window, name: &str) -> Res<()> {
    press(conn, root, name, true)?;
    press(conn, root, name, false)
}

/// The press (`down`) or the release of a key, or of the left button.
fn press(conn: &RustConnection, root: Window, name: &str, down: bool) -> Res<()> {
    if name == "button1" {
        let kind = if down { BUTTON_PRESS } else { BUTTON_RELEASE };
        conn.xtest_fake_input(kind, 1, 0, root, 0, 0, 0)?;
        conn.get_input_focus()?.reply()?;
        return Ok(());
    }
    let sym = keysym(name).ok_or_else(|| format!("unknown key {name}"))?;
    let setup = conn.setup();
    let (min, max) = (setup.min_keycode, setup.max_keycode);
    let map = conn.get_keyboard_mapping(min, max - min + 1)?.reply()?;
    let per = usize::from(map.keysyms_per_keycode);
    let code = map
        .keysyms
        .chunks(per)
        .position(|syms| syms.contains(&sym))
        .map(|i| min + i as u8)
        .ok_or_else(|| format!("no keycode for {name}"))?;
    let kind = if down { KEY_PRESS } else { KEY_RELEASE };
    conn.xtest_fake_input(kind, code, 0, root, 0, 0, 0)?;
    conn.get_input_focus()?.reply()?;
    Ok(())
}

struct Atoms {
    net_wm_name: Atom,
    utf8: Atom,
}

fn atoms(conn: &RustConnection) -> Res<Atoms> {
    Ok(Atoms {
        net_wm_name: conn.intern_atom(false, b"_NET_WM_NAME")?.reply()?.atom,
        utf8: conn.intern_atom(false, b"UTF8_STRING")?.reply()?.atom,
    })
}

fn name_of(conn: &RustConnection, a: &Atoms, win: Window) -> Option<String> {
    for (prop, ty) in [
        (a.net_wm_name, a.utf8),
        (AtomEnum::WM_NAME.into(), AtomEnum::STRING.into()),
    ] {
        let reply = conn
            .get_property(false, win, prop, ty, 0, 256)
            .ok()?
            .reply()
            .ok()?;
        if !reply.value.is_empty() {
            return Some(String::from_utf8_lossy(&reply.value).into_owned());
        }
    }
    None
}

/// The window itself or the first descendant with a name (a window manager
/// frame holds the client as a child).
fn find_named(
    conn: &RustConnection,
    a: &Atoms,
    win: Window,
    depth: u32,
) -> Option<(Window, String)> {
    if let Some(n) = name_of(conn, a, win) {
        return Some((win, n));
    }
    if depth == 0 {
        return None;
    }
    let tree = conn.query_tree(win).ok()?.reply().ok()?;
    tree.children
        .into_iter()
        .find_map(|c| find_named(conn, a, c, depth - 1))
}

fn viewable(conn: &RustConnection, win: Window) -> bool {
    conn.get_window_attributes(win)
        .ok()
        .and_then(|c| c.reply().ok())
        .is_some_and(|r| r.map_state == MapState::VIEWABLE)
}

fn geometry(conn: &RustConnection, root: Window, name: &str) -> Res<()> {
    let a = atoms(conn)?;
    let tops = conn.query_tree(root)?.reply()?.children;
    for top in tops.into_iter().rev() {
        if !viewable(conn, top) {
            continue;
        }
        let Some((win, found)) = find_named(conn, &a, top, 3) else {
            continue;
        };
        if found != name {
            continue;
        }
        let g = conn.get_geometry(win)?.reply()?;
        let t = conn.translate_coordinates(win, root, 0, 0)?.reply()?;
        println!("{} {} {} {}", t.dst_x, t.dst_y, g.width, g.height);
        return Ok(());
    }
    Err(format!("no viewable window named {name}").into())
}

fn pixel(conn: &RustConnection, screen_num: usize, root: Window, x: i16, y: i16) -> Res<()> {
    let screen = &conn.setup().roots[screen_num];
    let visual = screen
        .allowed_depths
        .iter()
        .flat_map(|d| d.visuals.iter())
        .find(|v| v.visual_id == screen.root_visual)
        .ok_or("root visual not found")?;
    let img = conn
        .get_image(ImageFormat::Z_PIXMAP, root, x, y, 1, 1, !0)?
        .reply()?;
    let mut raw = 0u32;
    for (i, b) in img.data.iter().take(4).enumerate() {
        raw |= u32::from(*b) << (8 * i);
    }
    let channel = |mask: u32| {
        if mask == 0 {
            return 0;
        }
        let shift = mask.trailing_zeros();
        let bits = (mask >> shift).count_ones();
        let v = (raw & mask) >> shift;
        (v * 255 / ((1 << bits) - 1)) as u8
    };
    println!(
        "{:02X}{:02X}{:02X}",
        channel(visual.red_mask),
        channel(visual.green_mask),
        channel(visual.blue_mask)
    );
    Ok(())
}

fn stacking(conn: &RustConnection, root: Window) -> Res<()> {
    let a = atoms(conn)?;
    let tops = conn.query_tree(root)?.reply()?.children;
    for top in tops.into_iter().rev() {
        if !viewable(conn, top) {
            continue;
        }
        if let Some((_, n)) = find_named(conn, &a, top, 3) {
            println!("{n}");
        }
    }
    Ok(())
}
