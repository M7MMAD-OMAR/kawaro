//! Daemon-like timing harness for S-001 AC1 and AC2, built on the public
//! hive API. Run it only inside crates/ui/tests/measure.sh (private headless
//! compositor), never on a live desktop.
//!
//!     hive_bench daemon [--lang ar|en] [--file PATH]   waits on $XDG_RUNTIME_DIR/kawaro-ui-bench.sock
//!     hive_bench show [--token T] [--drop [--hold]]   asks for the hive, prints first_frame_ms (catcher_ms in drop mode)
//!     hive_bench ctl hide|quit
//!     hive_bench run [--lang ar|en] [--file PATH]      cold start, prints first_frame_epoch_ms
//!
//! With `--file` every show presents that selected file in the centre
//! (S-018h), with thumbnails and MIME globs from the environment.
//!
//! `first_frame_ms` is measured inside the daemon from the moment the request
//! line arrives to the first frame presented after `present`, with the full
//! 18-cell hive (1 + 6 + 12 = 19 hexagons, as AC1 asks).
//!
//! `--drop` shows the empty drop hive (`Hive::present_drop`, S-014 AC12) and
//! prints `mode=drop`. `--drop --hold` passes the request through the long
//! press gate (`kawaro_core::hold`, 400 ms) as the daemon does, shows the
//! hive with `Hive::present_drop_hold` when the gate's timer gives the show,
//! and prints `mode=hold hold_ms=400` (S-019e AC20).
//!
//! A drop show on the layer path binds a hidden catcher and draws the hive
//! only once a drag of files with an action enters it (S-021), so a bench
//! show, with no drag, has no first frame. Both drop modes are timed to the
//! catcher's ready commit instead (S-021 AC8): its first commit after the
//! first configure of its layer surface. `--drop` prints `catcher_ms` from
//! the request line; `--drop --hold` prints `catcher_ms` from the
//! threshold's end and `press_to_catcher_ms` from the request.

mod common;

use kawaro_core::hold::{Effect, Gate, Signal};
use kawaro_ui::{Hive, HiveCell, Lang};
use std::cell::RefCell;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

fn socket_path() -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR").unwrap_or_else(|| "/tmp".into());
    PathBuf::from(dir).join("kawaro-ui-bench.sock")
}

fn arg_after(args: &[String], flag: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == flag).map(|w| w[1].clone())
}

fn full_hive() -> Vec<HiveCell> {
    (0..18)
        .map(|i| HiveCell {
            id: format!("action.{i}"),
            code: format!("A{i}"),
            label: format!("Action number {i}"),
            icon: kawaro_ui::icon_names()
                .nth(i % 21)
                .unwrap_or_default()
                .into(),
            ring: if i < 6 { 1 } else { 2 },
        })
        .collect()
}

fn epoch_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// `present`, or `present_files` when the bench has selected files.
fn present(
    hive: &Hive,
    files: &[PathBuf],
    token: Option<String>,
) -> Result<(), slint::PlatformError> {
    if files.is_empty() {
        hive.present("18", full_hive(), token)
    } else {
        hive.present_files("18", files, full_hive(), token)
    }
}

/// Runs `f` once the next drop show's catcher is ready (S-021 AC8). Only
/// the layer path has a catcher; elsewhere the first frame stands in.
#[cfg(target_os = "linux")]
fn after_catcher_ready(f: impl FnOnce() + 'static) {
    kawaro_ui::after_catcher_ready(f);
}

#[cfg(not(target_os = "linux"))]
fn after_catcher_ready(f: impl FnOnce() + 'static) {
    kawaro_ui::after_next_frame(f);
}

/// How long a drop show's reply waits after the catcher is ready, so the
/// show is placed (its `hive surface` line comes after the 50 ms anchor
/// wait) before measure.sh hides it. The time it reports is taken at the
/// ready commit, not after this wait.
const SETTLE: Duration = Duration::from_millis(250);

/// Writes `line` to the client after [`SETTLE`].
fn reply_later(mut stream: UnixStream, line: String) {
    slint::Timer::single_shot(SETTLE, move || {
        writeln!(stream, "{line}").ok();
    });
}

/// The long press threshold of the hold shows (the default `hold_ms`).
const HOLD_MS: u64 = 400;

thread_local! {
    static HIVE: RefCell<Option<Rc<Hive>>> = const { RefCell::new(None) };
    static FILES: RefCell<Vec<PathBuf>> = const { RefCell::new(Vec::new()) };
    static GATE: RefCell<Gate> = RefCell::new(Gate::new(|| HOLD_MS));
    static GATE_TIMER: slint::Timer = slint::Timer::default();
}

/// A hold show: the press goes through the gate at `received`; its timer
/// shows the hive in hold mode and reports both times once the catcher is
/// ready.
fn hold_show(hive: Rc<Hive>, token: Option<String>, received: Instant, stream: UnixStream) {
    GATE.with(|g| g.borrow_mut().signal(Signal::Hold { at: None }, received));
    let pending = RefCell::new(Some((hive, token, stream)));
    GATE_TIMER.with(|t| {
        t.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(1),
            move || {
                let Some(Effect::Show { at }) = GATE.with(|g| g.borrow_mut().tick(Instant::now()))
                else {
                    return;
                };
                GATE_TIMER.with(|t| t.stop());
                let Some((hive, token, stream)) = pending.borrow_mut().take() else {
                    return;
                };
                let threshold = Instant::now();
                after_catcher_ready(move || {
                    let ms = threshold.elapsed().as_secs_f64() * 1000.0;
                    let total = received.elapsed().as_secs_f64() * 1000.0;
                    reply_later(
                        stream,
                        format!(
                            "mode=hold hold_ms={HOLD_MS} catcher_ms={ms:.1} press_to_catcher_ms={total:.1}"
                        ),
                    );
                });
                if let Err(e) = hive.present_drop_hold("", token, at) {
                    eprintln!("present failed: {e}");
                }
            },
        )
    });
}

fn daemon(lang: Lang, files: Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    kawaro_ui::init()?;
    let hive = Rc::new(Hive::new(lang)?);
    common::preview_source(&hive);
    hive.on_choose(|id| eprintln!("chosen {id}"));
    hive.on_dismiss(|| eprintln!("dismissed"));
    let path = socket_path();
    std::fs::remove_file(&path).ok();
    let listener = UnixListener::bind(&path)?;
    eprintln!("listening on {}", path.display());

    HIVE.with(|h| *h.borrow_mut() = Some(hive));
    FILES.with(|f| *f.borrow_mut() = files);

    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut line = String::new();
            if BufReader::new(&stream).read_line(&mut line).is_err() {
                continue;
            }
            let received = Instant::now();
            slint::invoke_from_event_loop(move || {
                let Some(hive) = HIVE.with(|h| h.borrow().clone()) else {
                    return;
                };
                let line = line.trim();
                match line {
                    "quit" => {
                        hive.close();
                        slint::quit_event_loop().ok();
                    }
                    "hide" => {
                        hive.close();
                        // The hive closed: the gate forgets it (decision 14).
                        GATE.with(|g| g.borrow_mut().hive_closed());
                    }
                    _ => {
                        let mut words = line.strip_prefix("show ").unwrap_or("-").split(' ');
                        let token = words.next().filter(|t| *t != "-").map(str::to_owned);
                        let mode = words.next().unwrap_or("normal");
                        if mode == "hold" {
                            return hold_show(hive, token, received, stream);
                        }
                        let mut stream = stream;
                        if mode == "drop" {
                            after_catcher_ready(move || {
                                let ms = received.elapsed().as_secs_f64() * 1000.0;
                                reply_later(stream, format!("mode=drop catcher_ms={ms:.1}"));
                            });
                        } else {
                            kawaro_ui::after_next_frame(move || {
                                let ms = received.elapsed().as_secs_f64() * 1000.0;
                                writeln!(stream, "first_frame_ms={ms:.1}").ok();
                            });
                        }
                        let shown = if mode == "drop" {
                            hive.present_drop("", token)
                        } else {
                            FILES.with(|f| present(&hive, &f.borrow(), token))
                        };
                        if let Err(e) = shown {
                            eprintln!("present failed: {e}");
                        }
                    }
                }
            })
            .ok();
        }
    });
    slint::run_event_loop_until_quit()?;
    std::fs::remove_file(socket_path()).ok();
    Ok(())
}

fn show(token: Option<String>, mode: &str) -> Result<(), Box<dyn std::error::Error>> {
    let t0 = Instant::now();
    let mut s = UnixStream::connect(socket_path())?;
    writeln!(s, "show {} {mode}", token.as_deref().unwrap_or("-"))?;
    s.set_read_timeout(Some(Duration::from_secs(10)))?;
    let mut line = String::new();
    BufReader::new(s).read_line(&mut line)?;
    println!(
        "{} client_roundtrip_ms={:.1}",
        line.trim(),
        t0.elapsed().as_secs_f64() * 1000.0
    );
    Ok(())
}

fn ctl(cmd: &str) -> Result<(), Box<dyn std::error::Error>> {
    writeln!(UnixStream::connect(socket_path())?, "{cmd}")?;
    Ok(())
}

fn cold(lang: Lang, files: Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    let t0 = Instant::now();
    kawaro_ui::init()?;
    let hive = Hive::new(lang)?;
    common::preview_source(&hive);
    kawaro_ui::after_next_frame(move || {
        println!(
            "first_frame_ms={:.1} first_frame_epoch_ms={}",
            t0.elapsed().as_secs_f64() * 1000.0,
            epoch_ms()
        );
        slint::quit_event_loop().ok();
    });
    present(&hive, &files, None)?;
    slint::run_event_loop_until_quit()?;
    hive.close();
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let lang = arg_after(&args, "--lang")
        .map(|l| Lang::parse(&l))
        .unwrap_or(Lang::En);
    match args.first().map(String::as_str) {
        Some("daemon") => daemon(lang, common::files_from_args(&args)),
        Some("show") => {
            let has = |f: &str| args.iter().any(|a| a == f);
            let mode = match (has("--drop"), has("--hold")) {
                (true, true) => "hold",
                (true, false) => "drop",
                _ => "normal",
            };
            show(arg_after(&args, "--token"), mode)
        }
        Some("ctl") => ctl(args.get(1).map(String::as_str).unwrap_or("hide")),
        Some("run") => cold(lang, common::files_from_args(&args)),
        _ => Err("usage: hive_bench daemon|show|ctl|run".into()),
    }
}
