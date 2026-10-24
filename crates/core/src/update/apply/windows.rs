//! The Windows side of applying an update (S-054a AC7, ADR 0006 section 6).
//! Builds for Windows only; the command line, the exit codes and the order of
//! the steps are decided in `apply.rs`, and the live check is S-052 once the
//! per-user MSI exists. Left out of the mutants run (`.cargo/mutants.toml`).

use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::{Daemon, HelperOps, UpdateError, UpdateResult, helper_args, msiexec_path, run_helper};
use crate::ipc::Request;
use crate::paths;
use crate::transport::{Stream, endpoint};
use crate::update::result::write_result;

const DETACHED_PROCESS: u32 = 0x0000_0008;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

/// The variable that tells the helper copy where the program folder is.
const PROGRAM_DIR_VAR: &str = "SBARHIVE_PROGRAM_DIR";

/// Copies `sbarhive.exe` to a temporary folder and starts the copy detached
/// with `update --apply-msi <msi>`. The caller asks the daemon to quit next;
/// the copy waits for it, so `msiexec` never replaces files of a live daemon.
pub fn apply_msi(msi: &Path) -> Result<(), UpdateError> {
    let exe = std::env::current_exe().map_err(|_| UpdateError::Io)?;
    let program_dir = exe.parent().ok_or(UpdateError::Io)?;
    let folder =
        std::env::temp_dir().join(format!("sbarhive-update-helper-{}", std::process::id()));
    std::fs::create_dir_all(&folder).map_err(|_| UpdateError::Io)?;
    let copy = folder.join("sbarhive.exe");
    std::fs::copy(&exe, &copy).map_err(|_| UpdateError::Io)?;
    Command::new(&copy)
        .args(helper_args(msi))
        .env(PROGRAM_DIR_VAR, program_dir)
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
        .spawn()
        .map_err(|_| UpdateError::Io)?;
    ask_daemon_to_quit();
    Ok(())
}

/// Asks the running daemon to quit over IPC; the helper copy waits for it.
/// A daemon that cannot be reached is already gone, so errors are ignored.
fn ask_daemon_to_quit() {
    let Ok(mut stream) = Stream::connect(&endpoint()) else {
        return;
    };
    if let Ok(line) = serde_json::to_string(&Request::Quit) {
        let _ = writeln!(stream, "{line}");
    }
}

/// What the helper copy runs for `update --apply-msi <msi>`. The version is
/// the name of the folder that holds the package (`<cache>/updates/<version>`).
pub fn helper_main(msi: &Path) -> UpdateResult {
    let version = msi
        .parent()
        .and_then(Path::file_name)
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let ops = Ops {
        program_dir: std::env::var_os(PROGRAM_DIR_VAR).map(PathBuf::from),
    };
    run_helper(&ops, msi, &msi.with_extension("log"), &version, now)
}

struct Ops {
    program_dir: Option<PathBuf>,
}

impl HelperOps for Ops {
    fn wait_for_daemon_exit(&self, timeout: Duration) -> bool {
        let start = Instant::now();
        while Stream::connect(&endpoint()).is_ok() {
            if start.elapsed() >= timeout {
                return false;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        true
    }

    fn run_msiexec(&self, args: &[String]) -> Result<i32, UpdateError> {
        let exe = msiexec_path(&|k| std::env::var_os(k)).ok_or(UpdateError::Unavailable)?;
        let status = Command::new(exe)
            .args(args)
            .status()
            .map_err(|_| UpdateError::Unavailable)?;
        status.code().ok_or(UpdateError::Io)
    }

    fn write_result(&self, r: &UpdateResult) -> Result<(), UpdateError> {
        let dir = paths::config_dir().ok_or(UpdateError::Io)?;
        write_result(&dir, r)
    }

    fn start_daemon(&self, _which: Daemon) {
        // The MSI upgrades in place, so the new and the old daemon are the
        // same file in the same folder.
        let Some(dir) = &self.program_dir else {
            return;
        };
        let _ = Command::new(dir.join("sbarhived.exe"))
            .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
            .spawn();
    }
}
