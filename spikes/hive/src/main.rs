//! Kawaro hive spike. Answers three questions before any real code is built:
//! how fast the hive appears on Wayland, whether it gets keyboard focus, and
//! whether hex picking, Esc, focus loss and Arabic layout behave.
//!
//! Modes:
//!   run       [--lang ar|en] [--light] [--exit-after MS]   cold start, one window
//!   daemon    [--lang ar|en] [--light]                     waits on a Unix socket
//!   show      [--token T]                                  asks the daemon to open the hive
//!   ctl       hide|quit                                    hides the hive or stops the daemon
//!   snapshot  OUT.png [--lang ar|en] [--light]             renders once and saves a PNG

mod geometry;

use geometry::{Axial, Layout};
use slint::winit_030::{CustomApplicationHandler, EventResult, winit};
use std::cell::{Cell as StdCell, RefCell};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

slint::include_modules!();

const RADIUS: f32 = 41.6; // 72 px flat-to-flat, tokens hex.size.md
const GAP: f32 = 6.0;
const RINGS: i32 = 2;
const PAD: f32 = 4.0;
const FOCUS_PROBE: Duration = Duration::from_millis(400);

/// Activation token for the next window creation, consumed by the attributes hook.
static PENDING_TOKEN: Mutex<Option<String>> = Mutex::new(None);

thread_local! {
    static FOCUSED: StdCell<bool> = const { StdCell::new(false) };
    static WAS_FOCUSED: StdCell<bool> = const { StdCell::new(false) };
    static ON_FOCUS_LOST: RefCell<Option<Box<dyn Fn()>>> = const { RefCell::new(None) };
    /// Runs once right after the next frame is drawn.
    static AFTER_FRAME: RefCell<Option<Box<dyn FnOnce()>>> = const { RefCell::new(None) };
}

struct FocusWatcher;

impl CustomApplicationHandler for FocusWatcher {
    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _id: winit::window::WindowId,
        _winit_window: Option<&winit::window::Window>,
        _slint_window: Option<&slint::Window>,
        event: &winit::event::WindowEvent,
    ) -> EventResult {
        if let winit::event::WindowEvent::RedrawRequested = event {
            // Slint draws right after this hook returns; a zero timer fires on
            // the next loop turn, once the frame is submitted. Works with every renderer.
            if let Some(f) = AFTER_FRAME.with(|h| h.borrow_mut().take()) {
                slint::Timer::single_shot(Duration::ZERO, f);
            }
        }
        if let winit::event::WindowEvent::Focused(f) = event {
            FOCUSED.with(|c| c.set(*f));
            if *f {
                WAS_FOCUSED.with(|c| c.set(true));
            } else if WAS_FOCUSED.with(|c| c.replace(false)) {
                ON_FOCUS_LOST.with(|h| {
                    if let Some(h) = h.borrow().as_ref() {
                        h()
                    }
                });
            }
        }
        EventResult::Propagate
    }
}

struct Opts {
    arabic: bool,
    light: bool,
    exit_after: Option<u64>,
    token: Option<String>,
    out: Option<PathBuf>,
}

fn parse(args: &[String]) -> Opts {
    let mut o = Opts {
        arabic: false,
        light: false,
        exit_after: None,
        token: std::env::var("XDG_ACTIVATION_TOKEN").ok(),
        out: None,
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--lang" => o.arabic = it.next().map(|v| v == "ar").unwrap_or(false),
            "--light" => o.light = true,
            "--exit-after" => o.exit_after = it.next().and_then(|v| v.parse().ok()),
            "--token" => o.token = it.next().cloned(),
            other => o.out = Some(PathBuf::from(other)),
        }
    }
    o
}

fn socket_path() -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR").unwrap_or_else(|| "/tmp".into());
    PathBuf::from(dir).join("kawaro-spike.sock")
}

/// Dummy actions for a selection of three images. Ring 1 holds the common
/// conversions, ring 2 the transforms.
fn actions(arabic: bool) -> Vec<(&'static str, &'static str)> {
    let ring1 = [
        ("WEBP", "WebP", "WebP"),
        ("PNG", "PNG", "PNG"),
        ("JPG", "JPEG", "JPEG"),
        ("AVIF", "AVIF", "AVIF"),
        ("PDF", "ملف PDF واحد", "Single PDF"),
        ("ICO", "أيقونة", "Icon"),
    ];
    let ring2 = [
        ("تحجيم", "RESIZE", "تغيير الحجم", "Resize"),
        ("تدوير", "ROTATE", "تدوير 90 درجة", "Rotate 90"),
        ("ضغط", "SHRINK", "تقليل الحجم", "Compress"),
        ("تنظيف", "STRIP", "حذف البيانات الوصفية", "Strip metadata"),
        ("رمادي", "GRAY", "تدرج رمادي", "Grayscale"),
        ("قص", "CROP", "قص الحواف", "Trim edges"),
        ("قلب", "FLIP", "قلب أفقي", "Flip"),
        ("GIF", "GIF", "صورة متحركة", "Animated GIF"),
        ("TIFF", "TIFF", "TIFF", "TIFF"),
        ("JXL", "JXL", "JPEG XL", "JPEG XL"),
        ("BMP", "BMP", "BMP", "BMP"),
        ("نص", "OCR", "استخراج النص", "Extract text"),
    ];
    let mut v = vec![("", "")];
    for (code, ar, en) in ring1 {
        v.push((code, if arabic { ar } else { en }));
    }
    for (ar_code, en_code, ar, en) in ring2 {
        v.push(if arabic { (ar_code, ar) } else { (en_code, en) });
    }
    v
}

fn build_hive(o: &Opts) -> Result<(Hive, Rc<(Layout, Vec<Axial>)>), slint::PlatformError> {
    let (w, h) = Layout::extent(RADIUS, GAP, RINGS, PAD);
    let layout = Layout {
        radius: RADIUS,
        gap: GAP,
        rtl: o.arabic,
        origin: (w / 2.0, h / 2.0),
    };
    let axial = geometry::spiral(RINGS);
    let cells: Vec<Cell> = axial
        .iter()
        .zip(actions(o.arabic))
        .map(|(a, (code, name))| {
            let (x, y) = layout.center(*a);
            Cell {
                x,
                y,
                code: code.into(),
                name: name.into(),
            }
        })
        .collect();

    let hive = Hive::new()?;
    hive.global::<Tokens>().set_dark(!o.light);
    hive.set_cells(Rc::new(slint::VecModel::from(cells)).into());
    hive.set_hex_r(RADIUS);
    hive.set_win_w(w);
    hive.set_win_h(h);
    hive.set_center_text(if o.arabic { "3 صور" } else { "3 images" }.into());

    let geo = Rc::new((layout, axial));
    let g = geo.clone();
    hive.on_pick(move |x, y| g.0.pick(&g.1, x, y).map_or(-1, |i| i as i32));
    Ok((hive, geo))
}

fn select_backend(token: Option<String>) -> Result<(), slint::PlatformError> {
    *PENDING_TOKEN.lock().unwrap() = token;
    let mut selector = slint::BackendSelector::new().backend_name("winit".into());
    if let Ok(r) = std::env::var("SPIKE_RENDERER") {
        selector = selector.renderer_name(r);
    }
    selector
        .with_winit_window_attributes_hook(|attrs| {
            use winit::platform::startup_notify::WindowAttributesExtStartupNotify;
            use winit::window::ActivationToken;
            match PENDING_TOKEN.lock().unwrap().take() {
                Some(t) => attrs.with_activation_token(ActivationToken::from_raw(t)),
                None => attrs,
            }
        })
        .with_winit_custom_application_handler(FocusWatcher)
        .select()
}

/// Calls `on_frame` with the elapsed time after the next drawn frame, every
/// time `start` holds an instant.
fn arm_first_frame(_hive: &Hive, start: Rc<StdCell<Option<Instant>>>, on_frame: Rc<dyn Fn(f64)>) {
    fn arm(start: Rc<StdCell<Option<Instant>>>, on_frame: Rc<dyn Fn(f64)>) {
        let (s2, f2) = (start.clone(), on_frame.clone());
        AFTER_FRAME.with(|h| {
            *h.borrow_mut() = Some(Box::new(move || {
                if let Some(t0) = s2.take() {
                    f2(t0.elapsed().as_secs_f64() * 1000.0);
                }
                arm(s2, f2);
            }))
        });
    }
    arm(start, on_frame);
}

fn epoch_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

fn run(o: Opts) -> Result<(), Box<dyn std::error::Error>> {
    let t0 = Instant::now();
    select_backend(o.token.clone())?;
    let (hive, _geo) = build_hive(&o)?;

    let start = Rc::new(StdCell::new(Some(t0)));
    arm_first_frame(
        &hive,
        start,
        Rc::new(|ms| {
            println!("first_frame_ms={ms:.1} first_frame_epoch_ms={}", epoch_ms());
            slint::Timer::single_shot(FOCUS_PROBE, || {
                println!("focused={}", FOCUSED.with(|c| c.get()));
            });
        }),
    );

    hive.on_activated(|i| println!("activated={i}"));
    let quit = || {
        println!("dismissed");
        slint::quit_event_loop().ok();
    };
    hive.on_dismiss(quit);
    ON_FOCUS_LOST.with(|h| {
        *h.borrow_mut() = Some(Box::new(|| {
            println!("focus_lost");
            slint::quit_event_loop().ok();
        }))
    });
    if let Some(ms) = o.exit_after {
        slint::Timer::single_shot(Duration::from_millis(ms), || {
            slint::quit_event_loop().ok();
        });
    }
    hive.show()?;
    slint::run_event_loop()?;
    Ok(())
}

fn daemon(o: Opts) -> Result<(), Box<dyn std::error::Error>> {
    select_backend(None)?;
    let (hive, _geo) = build_hive(&o)?;
    let client: Rc<RefCell<Option<UnixStream>>> = Rc::new(RefCell::new(None));
    let start: Rc<StdCell<Option<Instant>>> = Rc::new(StdCell::new(None));

    let c = client.clone();
    arm_first_frame(
        &hive,
        start.clone(),
        Rc::new(move |ms| {
            if let Some(s) = c.borrow_mut().as_mut() {
                writeln!(s, "first_frame_ms={ms:.1}").ok();
            }
            let c = c.clone();
            slint::Timer::single_shot(FOCUS_PROBE, move || {
                if let Some(mut s) = c.borrow_mut().take() {
                    writeln!(s, "focused={}", FOCUSED.with(|f| f.get())).ok();
                }
            });
        }),
    );

    let weak = hive.as_weak();
    let hide = move |why: &str| {
        eprintln!("hide: {why}");
        if let Some(h) = weak.upgrade() {
            h.hide().ok();
        }
    };
    let h1 = hide.clone();
    hive.on_dismiss(move || h1("dismiss"));
    let h2 = hide.clone();
    hive.on_activated(move |i| h2(&format!("activated {i}")));
    ON_FOCUS_LOST.with(|f| *f.borrow_mut() = Some(Box::new(move || hide("focus lost"))));

    DAEMON_STATE.with(|st| *st.borrow_mut() = (client, start));

    let path = socket_path();
    std::fs::remove_file(&path).ok();
    let listener = UnixListener::bind(&path)?;
    eprintln!("listening on {}", path.display());

    let weak = hive.as_weak();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut line = String::new();
            if BufReader::new(&stream).read_line(&mut line).is_err() {
                continue;
            }
            let received = Instant::now();
            let token = line.trim().strip_prefix("show ").map(str::to_owned);
            let weak = weak.clone();
            slint::invoke_from_event_loop(move || {
                let Some(hive) = weak.upgrade() else { return };
                match line.trim() {
                    "quit" => {
                        slint::quit_event_loop().ok();
                        return;
                    }
                    "hide" => {
                        hive.hide().ok();
                        return;
                    }
                    _ => {}
                }
                DAEMON_STATE.with(|st| {
                    let (client, start) = &*st.borrow();
                    *client.borrow_mut() = Some(stream);
                    start.set(Some(received));
                });
                *PENDING_TOKEN.lock().unwrap() = token.filter(|t| t != "-");
                hive.invoke_reset();
                hive.show().ok();
            })
            .ok();
        }
    });

    slint::run_event_loop_until_quit()?;
    std::fs::remove_file(socket_path()).ok();
    Ok(())
}

type DaemonState = (Rc<RefCell<Option<UnixStream>>>, Rc<StdCell<Option<Instant>>>);

thread_local! {
    static DAEMON_STATE: RefCell<DaemonState> =
        RefCell::new((Rc::new(RefCell::new(None)), Rc::new(StdCell::new(None))));
}

fn show(o: Opts) -> Result<(), Box<dyn std::error::Error>> {
    let t0 = Instant::now();
    let mut s = UnixStream::connect(socket_path())?;
    writeln!(s, "show {}", o.token.as_deref().unwrap_or("-"))?;
    s.set_read_timeout(Some(Duration::from_secs(3)))?;
    for line in BufReader::new(s).lines().map_while(Result::ok) {
        println!("{line}");
    }
    println!("client_roundtrip_ms={:.1}", t0.elapsed().as_secs_f64() * 1000.0);
    Ok(())
}

fn ctl(cmd: &str) -> Result<(), Box<dyn std::error::Error>> {
    writeln!(UnixStream::connect(socket_path())?, "{cmd}")?;
    Ok(())
}

fn snapshot(o: Opts) -> Result<(), Box<dyn std::error::Error>> {
    select_backend(None)?;
    let out = o.out.clone().ok_or("snapshot needs an output path")?;
    let (hive, _geo) = build_hive(&o)?;
    hive.set_animate(false);
    // Hover a ring-1 cell so the snapshot shows the accent and the centre label.
    hive.set_hovered(5);
    let weak = hive.as_weak();
    slint::Timer::single_shot(Duration::from_millis(500), move || {
        let hive = weak.upgrade().unwrap();
        match hive.window().take_snapshot() {
            Ok(buf) => {
                let file = std::fs::File::create(&out).unwrap();
                let mut enc = png::Encoder::new(file, buf.width(), buf.height());
                enc.set_color(png::ColorType::Rgba);
                enc.set_depth(png::BitDepth::Eight);
                enc.write_header()
                    .unwrap()
                    .write_image_data(buf.as_bytes())
                    .unwrap();
                println!("wrote {} ({}x{})", out.display(), buf.width(), buf.height());
            }
            Err(e) => eprintln!("snapshot failed: {e}"),
        }
        slint::quit_event_loop().ok();
    });
    hive.show()?;
    slint::run_event_loop()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mode, rest) = args.split_first().ok_or("usage: hive-spike run|daemon|show|snapshot")?;
    let o = parse(rest);
    match mode.as_str() {
        "run" => run(o),
        "daemon" => daemon(o),
        "show" => show(o),
        "ctl" => ctl(rest.first().map(String::as_str).unwrap_or("hide")),
        "snapshot" => snapshot(o),
        m => Err(format!("unknown mode {m}").into()),
    }
}
