//! Renders the welcome window once and writes a PNG, or compares it with a
//! reference.
//!
//!     welcome_snapshot OUT.png [--lang ar|en] [--compare REF.png]
//!
//! The film is frozen at its poster, or at the frame given by `--frame N`,
//! so the render is deterministic. Needs a Wayland compositor: run it through
//! crates/ui/tests/welcome_snapshots.sh, which starts a private headless
//! one.

use kawaro_core::welcome::{Captions, Registration};
use kawaro_ui::Lang;
use kawaro_ui::welcome::WelcomeWindow;
use std::path::PathBuf;
use std::time::Duration;

const CHANNEL_TOLERANCE: u8 = 8;
const MAX_DIFF_RATIO: f64 = 0.001;

struct Opts {
    out: PathBuf,
    lang: Lang,
    compare: Option<PathBuf>,
    /// S-094: the rail item to show (0 is the first item of the window).
    page: Option<usize>,
    /// S-094: the window size to render, `WIDTHxHEIGHT` in logical px.
    size: Option<(f32, f32)>,
    /// S-094c: the film frame to show; the poster when absent.
    frame: Option<usize>,
    /// S-094c: show the hive at the pointer instead of centred.
    at_pointer: bool,
}

fn parse(args: &[String]) -> Result<Opts, String> {
    let mut o = Opts {
        out: PathBuf::new(),
        lang: Lang::En,
        compare: None,
        page: None,
        size: None,
        frame: None,
        at_pointer: false,
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--lang" => o.lang = it.next().map(|v| Lang::parse(v)).unwrap_or(Lang::En),
            "--compare" => o.compare = it.next().map(PathBuf::from),
            "--frame" => o.frame = it.next().and_then(|v| v.parse().ok()),
            "--at-pointer" => o.at_pointer = true,
            "--page" => o.page = it.next().and_then(|v| v.parse().ok()),
            "--size" => {
                o.size = it.next().and_then(|v| {
                    let (w, h) = v.split_once('x')?;
                    Some((w.parse().ok()?, h.parse().ok()?))
                })
            }
            other => o.out = PathBuf::from(other),
        }
    }
    if o.out.as_os_str().is_empty() {
        return Err("usage: welcome_snapshot OUT.png [--lang ar|en] [--compare REF.png]".into());
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

fn read_png(path: &PathBuf) -> Result<(u32, u32, Vec<u8>), Box<dyn std::error::Error>> {
    let decoder = png::Decoder::new(std::fs::File::open(path)?);
    let mut reader = decoder.read_info()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf)?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return Err(format!("{}: expected 8-bit RGBA", path.display()).into());
    }
    buf.truncate(info.buffer_size());
    Ok((info.width, info.height, buf))
}

fn diff_pixels(a: &[u8], b: &[u8]) -> usize {
    let (a, _) = a.as_chunks::<4>();
    let (b, _) = b.as_chunks::<4>();
    a.iter()
        .zip(b)
        .filter(|(p, q)| {
            p.iter()
                .zip(q.iter())
                .any(|(x, y)| x.abs_diff(*y) > CHANNEL_TOLERANCE)
        })
        .count()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let o = parse(&args)?;
    kawaro_ui::init()?;
    let win = std::rc::Rc::new(WelcomeWindow::new(o.lang)?);
    // The key-free caption step keeps the reference platform neutral (AC3).
    let captions: Captions = kawaro_core::welcome::captions(o.lang, &Registration::Unbound);
    win.open(&captions, !o.at_pointer)?;
    if let Some(page) = o.page {
        win.show_page(page);
    }
    if let Some((w, h)) = o.size {
        win.set_window_size(w, h);
    }
    win.freeze(o.frame.unwrap_or_else(kawaro_ui::film::poster_index));

    let result = std::rc::Rc::new(std::cell::Cell::new(Ok::<(), String>(())));
    let (w, r) = (win.clone(), result.clone());
    // Give the compositor time to configure the surface.
    slint::Timer::single_shot(Duration::from_millis(800), move || {
        r.set(snapshot(&w, &o));
        slint::quit_event_loop().ok();
    });
    slint::run_event_loop_until_quit()?;
    win.close();
    result.replace(Ok(())).map_err(Into::into)
}

fn snapshot(win: &WelcomeWindow, o: &Opts) -> Result<(), String> {
    let buf = win.take_snapshot().map_err(|e| e.to_string())?;
    let (w, h) = (buf.width(), buf.height());
    write_png(&o.out, w, h, buf.as_bytes()).map_err(|e| e.to_string())?;
    println!("wrote {} ({w}x{h})", o.out.display());
    let Some(reference) = &o.compare else {
        return Ok(());
    };
    let (rw, rh, rgba) = read_png(reference).map_err(|e| e.to_string())?;
    if (rw, rh) != (w, h) {
        return Err(format!(
            "size {w}x{h} differs from reference {rw}x{rh} ({})",
            reference.display()
        ));
    }
    let bad = diff_pixels(buf.as_bytes(), &rgba);
    let ratio = bad as f64 / (w as f64 * h as f64);
    println!(
        "diff {bad} pixels ({:.3} %) against {}",
        ratio * 100.0,
        reference.display()
    );
    if ratio > MAX_DIFF_RATIO {
        return Err(format!("snapshot differs from {}", reference.display()));
    }
    Ok(())
}
