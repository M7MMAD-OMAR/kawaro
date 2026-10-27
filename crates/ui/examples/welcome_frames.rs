//! Renders the welcome demonstration frames (S-087) from the scene in
//! `crates/ui/ui/welcome-frames.slint`, which draws the hive with the same
//! hex cell component the real hive uses. One family per run:
//!
//!     welcome_frames --family lp|lc|dp|dc --out DIR
//!
//! `lp` and `dp` show the hive at the pointer (the layer, X11 and Windows
//! paths), `lc` and `dc` centred (the Wayland window path); `l` and `d` are
//! the light and dark token schemes. Sixteen PNGs are written per run as
//! `DIR/frame-FAMILY-NN.png`. Needs a Wayland compositor: run it through
//! scripts/gen-welcome-frames.sh, which starts a private headless one.

mod frames_ui {
    include!(concat!(env!("OUT_DIR"), "/welcome_frames_ui.rs"));
}

use frames_ui::{FramesWindow, Palette};
use slint::ComponentHandle;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

/// The frames of one demonstration sequence: enough to tell the flow
/// (drag, trigger, hive, drop, result) at a readable pace.
const FRAMES: i32 = 16;

struct Opts {
    family: String,
    out: PathBuf,
}

fn parse(args: &[String]) -> Result<Opts, String> {
    let mut o = Opts {
        family: String::new(),
        out: PathBuf::new(),
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--family" => o.family = it.next().unwrap_or(&String::new()).clone(),
            "--out" => o.out = it.next().map(PathBuf::from).unwrap_or_default(),
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if o.family.is_empty() || o.out.as_os_str().is_empty() {
        return Err("usage: welcome_frames --family lp|lc|dp|dc --out DIR".into());
    }
    Ok(o)
}

fn write_png(path: &PathBuf, w: u32, h: u32, rgba: &[u8]) -> std::io::Result<()> {
    let file = std::fs::File::create(path)?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()?.write_image_data(rgba)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let o = parse(&args)?;
    let (centred, dark) = match o.family.as_str() {
        "lp" => (false, false),
        "lc" => (true, false),
        "dp" => (false, true),
        "dc" => (true, true),
        other => return Err(format!("unknown family {other}").into()),
    };
    std::fs::create_dir_all(&o.out)?;

    kawaro_ui::init()?;
    let ui = Rc::new(FramesWindow::new()?);
    ui.global::<Palette>().set_color_scheme(if dark {
        slint::language::ColorScheme::Dark
    } else {
        slint::language::ColorScheme::Light
    });
    ui.set_centred(centred);
    ui.set_frame(0);
    ui.show()?;

    // One tick per frame: the previous set has been drawn during the tick,
    // so every snapshot catches exactly one frame.
    let idx = Rc::new(std::cell::Cell::new(0i32));
    let written = Rc::new(std::cell::Cell::new(0i32));
    let timer = slint::Timer::default();
    {
        let (ui, idx, written, out, family) = (
            ui.clone(),
            idx.clone(),
            written.clone(),
            o.out.clone(),
            o.family.clone(),
        );
        timer.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(350),
            move || {
                let i = idx.get();
                if i > 0 {
                    let prev = i - 1;
                    let buf = ui.window().take_snapshot().expect("frame renders");
                    let name = format!("frame-{family}-{prev:02}.png");
                    write_png(&out.join(name), buf.width(), buf.height(), buf.as_bytes())
                        .expect("frame writes");
                    written.set(prev);
                }
                if i >= FRAMES {
                    slint::quit_event_loop().ok();
                    return;
                }
                ui.set_frame(i);
                idx.set(i + 1);
            },
        );
    }
    slint::run_event_loop_until_quit()?;
    drop(timer);
    ui.hide().ok();
    if written.get() != FRAMES - 1 {
        return Err(format!("wrote {} of {} frames", written.get() + 1, FRAMES).into());
    }
    println!("{}: {FRAMES} frames in {}", o.family, o.out.display());
    Ok(())
}
