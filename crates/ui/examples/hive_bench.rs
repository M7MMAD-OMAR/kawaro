//! Daemon-like timing harness for S-001 AC1 and AC2, built on the public
//! hive API. Run it only inside crates/ui/tests/measure.sh (private headless
//! compositor), never on a live desktop.
//!
//!     hive_bench daemon [--lang ar|en]   waits on $XDG_RUNTIME_DIR/sbarhive-ui-bench.sock
//!     hive_bench show [--token T]        asks for the hive, prints first_frame_ms
//!     hive_bench ctl hide|quit
//!     hive_bench run [--lang ar|en]      cold start, prints first_frame_epoch_ms
//!
//! `first_frame_ms` is measured inside the daemon from the moment the request
//! line arrives to the first frame presented after `present`, with the full
//! 18-cell hive (1 + 6 + 12 = 19 hexagons, as AC1 asks).

use sbarhive_ui::{Hive, HiveCell, Lang};
use std::cell::RefCell;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

fn socket_path() -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR").unwrap_or_else(|| "/tmp".into());
    PathBuf::from(dir).join("sbarhive-ui-bench.sock")
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

fn daemon(lang: Lang) -> Result<(), Box<dyn std::error::Error>> {
    sbarhive_ui::init()?;
    let hive = Rc::new(Hive::new(lang)?);
    hive.on_choose(|id| eprintln!("chosen {id}"));
    hive.on_dismiss(|| eprintln!("dismissed"));
    let path = socket_path();
    std::fs::remove_file(&path).ok();
    let listener = UnixListener::bind(&path)?;
    eprintln!("listening on {}", path.display());

    thread_local! {
        static HIVE: RefCell<Option<Rc<Hive>>> = const { RefCell::new(None) };
    }
    HIVE.with(|h| *h.borrow_mut() = Some(hive));

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
                    "hide" => hive.close(),
                    _ => {
                        let token = line
                            .strip_prefix("show ")
                            .filter(|t| *t != "-")
                            .map(str::to_owned);
                        let mut stream = stream;
                        sbarhive_ui::after_next_frame(move || {
                            let ms = received.elapsed().as_secs_f64() * 1000.0;
                            writeln!(stream, "first_frame_ms={ms:.1}").ok();
                        });
                        if let Err(e) = hive.present("18", full_hive(), token) {
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

fn show(token: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let t0 = Instant::now();
    let mut s = UnixStream::connect(socket_path())?;
    writeln!(s, "show {}", token.as_deref().unwrap_or("-"))?;
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

fn cold(lang: Lang) -> Result<(), Box<dyn std::error::Error>> {
    let t0 = Instant::now();
    sbarhive_ui::init()?;
    let hive = Hive::new(lang)?;
    sbarhive_ui::after_next_frame(move || {
        println!(
            "first_frame_ms={:.1} first_frame_epoch_ms={}",
            t0.elapsed().as_secs_f64() * 1000.0,
            epoch_ms()
        );
        slint::quit_event_loop().ok();
    });
    hive.present("18", full_hive(), None)?;
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
        Some("daemon") => daemon(lang),
        Some("show") => show(arg_after(&args, "--token")),
        Some("ctl") => ctl(args.get(1).map(String::as_str).unwrap_or("hide")),
        Some("run") => cold(lang),
        _ => Err("usage: hive_bench daemon|show|ctl|run".into()),
    }
}
