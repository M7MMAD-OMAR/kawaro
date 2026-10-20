//! Pointer and keyboard for a private KWin through org_kde_kwin_fake_input.
//! KWin only lets it through with KWIN_WAYLAND_NO_PERMISSION_CHECKS=1, which
//! the test sets for its own private compositor. Commands on stdin, one per line:
//!   move X Y | down | up | keydown CODE | keyup CODE | sleep MS
//! CODE is an evdev key code (125 Meta, 67 F9).
use std::io::BufRead;
use wayland_client::protocol::wl_registry::{self, WlRegistry};
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols_plasma::fake_input::client::org_kde_kwin_fake_input::OrgKdeKwinFakeInput as Fake;

struct S(Option<Fake>);
impl Dispatch<WlRegistry, ()> for S {
    fn event(s: &mut Self, r: &WlRegistry, e: wl_registry::Event, _: &(), _: &Connection, qh: &QueueHandle<Self>) {
        if let wl_registry::Event::Global { name, interface, version } = e {
            if interface == "org_kde_kwin_fake_input" {
                s.0 = Some(r.bind(name, version.min(5), qh, ()));
            }
        }
    }
}
impl Dispatch<Fake, ()> for S {
    fn event(_: &mut Self, _: &Fake, _: <Fake as wayland_client::Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {}
}

fn main() {
    let conn = Connection::connect_to_env().expect("wayland");
    let mut q = conn.new_event_queue();
    let qh = q.handle();
    conn.display().get_registry(&qh, ());
    let mut s = S(None);
    q.roundtrip(&mut s).unwrap();
    let f = s.0.clone().expect("no org_kde_kwin_fake_input");
    f.authenticate("drop-spike".into(), "test".into());
    q.roundtrip(&mut s).unwrap();
    eprintln!("kwin_input: ready");
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let p: Vec<&str> = line.split_whitespace().collect();
        match p.as_slice() {
            ["move", x, y] => f.pointer_motion_absolute(x.parse().unwrap(), y.parse().unwrap()),
            ["down"] => f.button(272, 1),
            ["up"] => f.button(272, 0),
            ["keydown", c] => f.keyboard_key(c.parse().unwrap(), 1),
            ["keyup", c] => f.keyboard_key(c.parse().unwrap(), 0),
            ["sleep", ms] => std::thread::sleep(std::time::Duration::from_millis(ms.parse().unwrap())),
            _ => {}
        }
        q.roundtrip(&mut s).unwrap();
    }
}
