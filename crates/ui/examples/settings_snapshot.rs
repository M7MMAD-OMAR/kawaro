//! Renders the settings window once and writes a PNG, or compares it with a
//! reference.
//!
//!     settings_snapshot OUT.png [--lang ar|en] [--compare REF.png]
//!
//! The window shows a sample configuration with the subfolder option on, so
//! the folder field is enabled. With `--compare` the exit code is 1 when more
//! than 0.1 % of the pixels differ by more than 8 levels in any channel;
//! OUT.png is still written. Needs a Wayland compositor: run it through
//! crates/ui/tests/settings_snapshots.sh, which starts a private headless one.

use sbarhive_core::config::{Config, OutputPlace};
use sbarhive_ui::Lang;
use std::path::PathBuf;
use std::time::Duration;

const CHANNEL_TOLERANCE: u8 = 8;
const MAX_DIFF_RATIO: f64 = 0.001;

struct Opts {
    out: PathBuf,
    lang: Lang,
    compare: Option<PathBuf>,
}

fn parse(args: &[String]) -> Result<Opts, String> {
    let mut o = Opts {
        out: PathBuf::new(),
        lang: Lang::En,
        compare: None,
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--lang" => o.lang = it.next().map(|v| Lang::parse(v)).unwrap_or(Lang::En),
            "--compare" => o.compare = it.next().map(PathBuf::from),
            other => o.out = PathBuf::from(other),
        }
    }
    if o.out.as_os_str().is_empty() {
        return Err("usage: settings_snapshot OUT.png [--lang ar|en] [--compare REF.png]".into());
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
    sbarhive_ui::init()?;
    let win = std::rc::Rc::new(sbarhive_ui::SettingsWindow::new(o.lang)?);
    let config = Config {
        output: OutputPlace::Subfolder,
        ..Config::default()
    };
    win.open(&config)?;

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

fn snapshot(win: &sbarhive_ui::SettingsWindow, o: &Opts) -> Result<(), String> {
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
