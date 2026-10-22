//! Socket client for the daemon protocol in `sbarhive_core::ipc`.

use crate::spawn;
use sbarhive_core::ipc::{Request, Response};
use sbarhive_core::transport::{self, Stream};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const START_TIMEOUT: Duration = Duration::from_secs(2);

fn connect() -> std::io::Result<Stream> {
    let s = Stream::connect(&transport::endpoint())?;
    s.set_read_timeout(Some(Duration::from_secs(10)))?;
    Ok(s)
}

fn send(stream: &mut Stream, req: &Request) -> Result<(), String> {
    // Connected, nothing written yet: the service may raise the hive only if
    // this client hands it the foreground right first (ADR 0005, section 5).
    if let Some(_pid) = spawn::foreground_grant(req, stream.server_pid()) {
        #[cfg(windows)]
        spawn::allow_foreground(_pid);
    }
    let line = serde_json::to_string(req).map_err(|e| e.to_string())?;
    writeln!(stream, "{line}").map_err(|e| format!("daemon: {e}"))
}

fn parse(line: &str) -> Result<Response, String> {
    serde_json::from_str(line).map_err(|e| format!("bad reply from daemon: {e}"))
}

/// Sends one request and returns the first reply. `Error` replies become `Err`.
pub fn request(req: &Request) -> Result<Response, String> {
    let mut s = connect().map_err(|e| format!("daemon not running ({e})"))?;
    send(&mut s, req)?;
    let mut line = String::new();
    BufReader::new(s)
        .read_line(&mut line)
        .map_err(|e| format!("daemon: {e}"))?;
    match parse(&line)? {
        Response::Error { message } => Err(message),
        r => Ok(r),
    }
}

/// Sends one request and feeds every reply line to `f` until it returns false
/// or the daemon closes the connection.
pub fn stream(req: &Request, mut f: impl FnMut(Response) -> bool) -> Result<(), String> {
    let mut s = connect().map_err(|e| format!("daemon not running ({e})"))?;
    s.set_read_timeout(None).ok();
    send(&mut s, req)?;
    for line in BufReader::new(s).lines() {
        let line = line.map_err(|e| format!("daemon: {e}"))?;
        match parse(&line)? {
            Response::Error { message } => return Err(message),
            r => {
                if !f(r) {
                    break;
                }
            }
        }
    }
    Ok(())
}

pub fn daemon_up() -> bool {
    matches!(request(&Request::Ping), Ok(Response::Ok))
}

/// The daemon binary: next to this executable when present, else from PATH.
fn daemon_binary() -> PathBuf {
    let dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_default();
    spawn::daemon_binary_in(&dir, std::env::consts::EXE_SUFFIX)
}

/// Starts the daemon detached when it does not answer, then waits for it.
pub fn ensure_daemon() -> Result<(), String> {
    if daemon_up() {
        return Ok(());
    }
    let mut cmd = Command::new(daemon_binary());
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        // The activation token belongs to this request, not to the daemon's lifetime.
        .env_remove("XDG_ACTIVATION_TOKEN")
        .env_remove("DESKTOP_STARTUP_ID");
    #[cfg(unix)]
    let started = {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0).spawn().map(drop)
    };
    #[cfg(windows)]
    let started = spawn::start_detached(&mut cmd);
    started.map_err(|e| format!("cannot start sbarhived: {e}"))?;
    let t0 = Instant::now();
    while t0.elapsed() < START_TIMEOUT {
        if daemon_up() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    Err("sbarhived did not start within 2 seconds".into())
}

#[cfg(unix)]
pub fn exec_daemon() -> Result<(), String> {
    use std::os::unix::process::CommandExt;
    let err = Command::new(daemon_binary()).exec();
    Err(format!("cannot run sbarhived: {err}"))
}

/// Windows has no `exec`: the service runs in the foreground and its exit
/// code becomes this process's.
#[cfg(windows)]
pub fn exec_daemon() -> Result<(), String> {
    let code = spawn::wait_exit(&mut Command::new(daemon_binary()))?;
    std::process::exit(code)
}
