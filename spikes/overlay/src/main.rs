//! Overlay spike for ADR 0004. Subcommands:
//!   globals              list the compositor's globals (layer-shell or not)
//!   uri PATH...          file URIs as the thumbnail cache names them
//!   thumb PATH...        what the centre cell would show for each file
//!   hive                 the real crates/ui hive on a layer-shell overlay
//!   preview PATH...      centre cell preview of the files, on the overlay
//! Environment: SPIKE_SETTINGS=1 also opens a normal (winit) window in the
//! same process; SPIKE_DROP=1 shows the catcher without input (drop mode);
//! SPIKE_EXIT_MS ends the run; SPIKE_LANG=ar mirrors the hive.

mod overlay;
mod platform;
mod thumb;

use overlay::log;
use kawaro_ui::{Hive, HiveCell, Lang};
use slint::ComponentHandle;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

slint::include_modules!();

static START: OnceLock<Instant> = OnceLock::new();

pub fn since_start() -> f64 {
    START.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
}

fn main() {
    since_start();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let files: Vec<PathBuf> = args.iter().skip(1).map(PathBuf::from).collect();
    match args.first().map(String::as_str) {
        Some("globals") => {
            if let Err(e) = overlay::print_globals() {
                eprintln!("globals: {e}");
                std::process::exit(1);
            }
        }
        Some("uri") => {
            for f in files {
                println!("{}", thumb::file_uri(&f));
            }
        }
        Some("thumb") => {
            for f in files {
                let (_, notes) = thumb::check(&f);
                println!("{}", f.display());
                for n in notes {
                    println!("  {n}");
                }
                println!("  mime {}", thumb::mime_of(&f));
                println!("  => {:?}", thumb::resolve(&f, &theme()));
            }
        }
        Some("hive") => run(|| hive()),
        Some("preview") => run(move || preview(files)),
        _ => {
            eprintln!("usage: overlay-spike globals|uri|thumb|hive|preview [PATH...]");
            std::process::exit(2);
        }
    }
}

fn theme() -> String {
    std::env::var("SPIKE_ICON_THEME").unwrap_or_else(|_| "Adwaita".into())
}

/// Installs the wrapping platform, runs `setup`, then the event loop.
fn run<T: 'static>(setup: impl FnOnce() -> T) {
    let winit = i_slint_backend_winit::Backend::builder()
        .with_renderer_name("software")
        .build()
        .expect("winit backend");
    slint::platform::set_platform(Box::new(platform::Wrap { winit })).expect("platform");
    overlay::set_drop_mode(std::env::var_os("SPIKE_DROP").is_some());
    let keep = setup();
    let mut settings = None;
    if std::env::var_os("SPIKE_SETTINGS").is_some() {
        let w = SettingsStandIn::new().expect("settings window");
        w.show().expect("show settings");
        log("settings stand-in: shown as a normal winit window");
        settings = Some(w);
    }
    if let Ok(ms) = std::env::var("SPIKE_EXIT_MS").map(|v| v.parse::<u64>().unwrap_or(5000)) {
        slint::Timer::single_shot(Duration::from_millis(ms), || {
            log("exit timer");
            let _ = slint::quit_event_loop();
        });
    }
    slint::run_event_loop_until_quit().expect("event loop");
    drop((keep, settings));
}

fn cell(id: &str, code: &str, label: &str, icon: &str, ring: u8) -> HiveCell {
    HiveCell { id: id.into(), code: code.into(), label: label.into(), icon: icon.into(), ring }
}

fn hive() -> Rc<Hive> {
    let lang = if std::env::var("SPIKE_LANG").as_deref() == Ok("ar") { Lang::Ar } else { Lang::En };
    platform::next_is_overlay();
    let hive = Rc::new(Hive::new(lang).expect("hive"));
    hive.on_choose(|id| log(format!("hive: chosen {id}")));
    hive.on_dismiss(|| log("hive: dismissed (Esc or gap)"));
    let h = Rc::downgrade(&hive);
    overlay::on_outside(move || {
        if let Some(h) = h.upgrade() {
            h.close();
            log("hive: dismissed (click outside)");
        }
    });
    let cells = vec![
        cell("image.to-webp", "WEBP", "Convert to WebP", "convert", 1),
        cell("image.to-png", "PNG", "Convert to PNG", "convert", 1),
        cell("image.to-jpg", "JPG", "Convert to JPEG", "convert", 1),
        cell("image.resize", "SIZE", "Resize", "resize", 1),
        cell("image.to-pdf", "PDF", "Make a PDF", "pdf", 1),
        cell("image.compress", "MIN", "Compress", "compress", 1),
        cell("image.rotate", "ROT", "Rotate", "rotate", 2),
        cell("image.grayscale", "GRAY", "Grayscale", "grayscale", 2),
    ];
    let t = Instant::now();
    hive.present("1 image", cells, None).expect("present");
    log(format!("hive: present returned after {:.1} ms", t.elapsed().as_secs_f64() * 1000.0));
    // A second present, as the daemon does for the next request.
    if let Ok(ms) = std::env::var("SPIKE_REPRESENT_MS").map(|v| v.parse::<u64>().unwrap_or(1500)) {
        let h = Rc::downgrade(&hive);
        slint::Timer::single_shot(Duration::from_millis(ms), move || {
            if let Some(h) = h.upgrade() {
                log("hive: present again");
                h.present("1 image", vec![cell("image.to-webp", "WEBP", "Convert to WebP", "convert", 1)], None)
                    .ok();
            }
        });
    }
    hive
}

fn preview(files: Vec<PathBuf>) -> PreviewWindow {
    platform::next_is_overlay();
    let w = PreviewWindow::new().expect("preview window");
    let first = files.first().cloned().unwrap_or_default();
    let t = Instant::now();
    let source = thumb::resolve(&first, &theme());
    let image = thumb::load(&source);
    log(format!(
        "preview: {:?} resolved and decoded in {:.1} ms",
        source,
        t.elapsed().as_secs_f64() * 1000.0
    ));
    let is_thumb = matches!(source, thumb::Source::Thumbnail(_));
    w.set_is_thumbnail(is_thumb);
    if let Some(i) = image {
        w.set_picture(i);
    }
    w.set_count(files.len() as i32);
    let caption = if is_thumb {
        String::new()
    } else {
        first.extension().map(|e| e.to_string_lossy().to_uppercase()).unwrap_or_default()
    };
    w.set_caption(caption.into());
    let weak = w.as_weak();
    w.on_dismiss(move || {
        if let Some(w) = weak.upgrade() {
            w.hide().ok();
            log("preview: dismissed (Esc)");
        }
    });
    overlay::on_outside(|| log("preview: dismissed (click outside)"));
    w.show().expect("show preview");
    w
}

slint::slint! {
    export component SettingsStandIn inherits Window {
        title: "Kawaro settings (stand-in)";
        width: 360px;
        height: 200px;
        Text { text: "Normal xdg-toplevel window from winit"; }
    }
}
