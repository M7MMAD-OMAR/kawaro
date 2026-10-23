// S-042 (backlog/S-042-windows-daemon-spawn.md): starting the service and
// handing it the foreground right. The test module below is the tester's
// contract and must not change; the implementation goes above it.
//
// Contract pinned by the tests (all items visible to `super::*`):
// - `DETACHED_PROCESS`, `CREATE_NEW_PROCESS_GROUP` and
//   `CREATE_BREAKAWAY_FROM_JOB`: `u32` constants, 0x8, 0x200, 0x01000000.
// - `start_with(attempt: impl FnMut(u32) -> io::Result<()>) -> io::Result<()>`;
//   the tests pass `&mut` closures, so `&mut dyn FnMut(u32) -> io::Result<()>`
//   also fits.
// - `wait_exit(cmd: &mut std::process::Command) -> Result<i32, String>`.
// - `daemon_binary_in(dir: &Path, suffix: &str) -> PathBuf`.
// - `foreground_grant(req: &Request, server_pid: Option<u32>) -> Option<u32>`,
//   `Request` being `sbarhive_core::ipc::Request`.

use sbarhive_core::ipc::Request;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{allow_foreground, start_detached};

/// Win32 process creation flags, checked against `windows_sys` at compile
/// time in `spawn/windows.rs`.
#[cfg_attr(not(windows), allow(dead_code))]
pub const DETACHED_PROCESS: u32 = 0x8;
#[cfg_attr(not(windows), allow(dead_code))]
pub const CREATE_NEW_PROCESS_GROUP: u32 = 0x200;
#[cfg_attr(not(windows), allow(dead_code))]
pub const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;

/// The first attempt leaves the terminal's job; the retry stays in it.
/// Literal values, so no operator can be mutated into an equivalent one.
const FIRST_FLAGS: u32 = 0x0100_0208;
const RETRY_FLAGS: u32 = 0x208;
const _: () =
    assert!(FIRST_FLAGS == DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_BREAKAWAY_FROM_JOB);
const _: () = assert!(RETRY_FLAGS == DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);

/// ERROR_ACCESS_DENIED: the job the terminal runs in forbids breaking away.
const ERROR_ACCESS_DENIED: i32 = 5;

/// Starts the service detached: once leaving the terminal's job, and once
/// more inside it when that is refused (ADR 0005, section 3).
#[cfg_attr(not(windows), allow(dead_code))]
pub fn start_with(mut attempt: impl FnMut(u32) -> io::Result<()>) -> io::Result<()> {
    match attempt(FIRST_FLAGS) {
        Err(e) if e.raw_os_error() == Some(ERROR_ACCESS_DENIED) => attempt(RETRY_FLAGS),
        r => r,
    }
}

/// Runs the service in the foreground and returns its exit code; this is
/// `sbarhive daemon` on Windows, which has no `exec`.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn wait_exit(cmd: &mut Command) -> Result<i32, String> {
    let status = cmd
        .status()
        .map_err(|e| format!("cannot run sbarhived: {e}"))?;
    Ok(status.code().unwrap_or(1))
}

/// The service binary in `dir` when present, else its bare name for PATH.
pub fn daemon_binary_in(dir: &Path, suffix: &str) -> PathBuf {
    let name = format!("sbarhived{suffix}");
    let path = dir.join(&name);
    if path.is_file() {
        path
    } else {
        PathBuf::from(name)
    }
}

/// The process the client lets take the foreground before sending `req`:
/// the service, for the requests that show the hive (ADR 0005, section 5).
pub fn foreground_grant(req: &Request, server_pid: Option<u32>) -> Option<u32> {
    match req {
        Request::Open { .. } | Request::Drop { .. } => server_pid,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbarhive_core::ipc::Request;
    use std::io;
    use std::path::{Path, PathBuf};

    const FIRST: u32 = 0x0100_0208;
    const RETRY: u32 = 0x208;

    fn refused() -> io::Error {
        // ERROR_ACCESS_DENIED on Windows; EIO on Linux, so only the raw code
        // can select the retry, not the error kind.
        io::Error::from_raw_os_error(5)
    }

    #[test]
    fn ac1_ac2_flag_constants_have_the_win32_values() {
        assert_eq!(DETACHED_PROCESS, 0x8_u32);
        assert_eq!(CREATE_NEW_PROCESS_GROUP, 0x200_u32);
        assert_eq!(CREATE_BREAKAWAY_FROM_JOB, 0x0100_0000_u32);
    }

    #[test]
    fn ac1_first_attempt_succeeds_with_breakaway() {
        let mut seen = Vec::new();
        let r = start_with(&mut |f: u32| {
            seen.push(f);
            Ok(())
        });
        assert!(r.is_ok(), "{r:?}");
        assert_eq!(seen, vec![FIRST]);
    }

    #[test]
    fn ac1_access_denied_then_success_retries_once_without_breakaway() {
        let mut seen = Vec::new();
        let r = start_with(&mut |f: u32| {
            seen.push(f);
            if seen.len() == 1 {
                Err(refused())
            } else {
                Ok(())
            }
        });
        assert!(r.is_ok(), "{r:?}");
        assert_eq!(seen, vec![FIRST, RETRY]);
    }

    #[test]
    fn ac1_access_denied_twice_returns_the_second_error() {
        let mut seen = Vec::new();
        let r = start_with(&mut |f: u32| {
            seen.push(f);
            Err(refused())
        });
        assert_eq!(seen, vec![FIRST, RETRY]);
        assert_eq!(r.expect_err("both refused").raw_os_error(), Some(5));
    }

    #[test]
    fn ac1_access_denied_then_other_error_returns_the_second_error() {
        let mut seen = Vec::new();
        let r = start_with(&mut |f: u32| {
            seen.push(f);
            if seen.len() == 1 {
                Err(refused())
            } else {
                Err(io::Error::other("second"))
            }
        });
        assert_eq!(seen, vec![FIRST, RETRY]);
        let e = r.expect_err("second attempt failed");
        assert_eq!(e.to_string(), "second");
        assert_eq!(e.raw_os_error(), None);
    }

    #[test]
    fn ac1_not_found_returns_without_retry() {
        let mut seen = Vec::new();
        let r = start_with(&mut |f: u32| {
            seen.push(f);
            Err(io::Error::from(io::ErrorKind::NotFound))
        });
        assert_eq!(seen, vec![FIRST]);
        assert_eq!(r.expect_err("not found").kind(), io::ErrorKind::NotFound);
    }

    #[cfg(unix)]
    #[test]
    fn ac3_wait_exit_returns_the_exit_code() {
        let mut cmd = std::process::Command::new("sh");
        cmd.args(["-c", "exit 3"]);
        assert_eq!(wait_exit(&mut cmd), Ok(3));
    }

    #[cfg(unix)]
    #[test]
    fn ac3_wait_exit_zero_is_ok_zero() {
        let mut cmd = std::process::Command::new("sh");
        cmd.args(["-c", "exit 0"]);
        assert_eq!(wait_exit(&mut cmd), Ok(0));
    }

    #[test]
    fn ac3_wait_exit_missing_program_is_cannot_run() {
        let dir = tempfile::tempdir().unwrap();
        let mut cmd = std::process::Command::new(dir.path().join("no-such-sbarhived"));
        let e = wait_exit(&mut cmd).expect_err("missing program");
        assert!(e.starts_with("cannot run sbarhived:"), "got {e:?}");
    }

    fn touch(dir: &Path, name: &str) -> PathBuf {
        let p = dir.join(name);
        std::fs::write(&p, b"").unwrap();
        p
    }

    #[test]
    fn ac4_exe_suffix_finds_sbarhived_exe_in_the_dir() {
        let dir = tempfile::tempdir().unwrap();
        let p = touch(dir.path(), "sbarhived.exe");
        assert_eq!(daemon_binary_in(dir.path(), ".exe"), p);
    }

    #[test]
    fn ac4_exe_suffix_without_the_file_falls_back_to_the_bare_name() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            daemon_binary_in(dir.path(), ".exe"),
            PathBuf::from("sbarhived.exe")
        );
    }

    #[test]
    fn ac4_exe_suffix_ignores_a_file_without_the_suffix() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), "sbarhived");
        assert_eq!(
            daemon_binary_in(dir.path(), ".exe"),
            PathBuf::from("sbarhived.exe")
        );
    }

    #[test]
    fn ac4_empty_suffix_finds_sbarhived_in_the_dir() {
        let dir = tempfile::tempdir().unwrap();
        let p = touch(dir.path(), "sbarhived");
        assert_eq!(daemon_binary_in(dir.path(), ""), p);
    }

    #[test]
    fn ac4_empty_suffix_without_the_file_falls_back_to_sbarhived() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(daemon_binary_in(dir.path(), ""), PathBuf::from("sbarhived"));
    }

    #[test]
    fn ac4_a_directory_named_sbarhived_is_not_the_binary() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sbarhived")).unwrap();
        assert_eq!(daemon_binary_in(dir.path(), ""), PathBuf::from("sbarhived"));
    }

    fn every_request() -> Vec<(Request, bool)> {
        let all = vec![
            (Request::Ping, false),
            (
                Request::Open {
                    files: vec![PathBuf::from("/tmp/a.png")],
                    token: None,
                },
                true,
            ),
            (
                Request::Drop {
                    token: None,
                    at: None,
                    hold: false,
                },
                true,
            ),
            (
                Request::Drop {
                    token: Some("t".into()),
                    at: Some((1.0, 2.0)),
                    hold: true,
                },
                true,
            ),
            (Request::DropRelease, false),
            (
                Request::Run {
                    action: "to-webp".into(),
                    files: vec![PathBuf::from("/tmp/a.png")],
                    wait: false,
                },
                false,
            ),
            (Request::Actions { files: vec![] }, false),
            (Request::CancelAll, false),
            (Request::Settings, false),
            (Request::Reload, false),
            (Request::Quit, false),
        ];
        // No wildcard: a new Request variant must be added to this list.
        for (req, _) in &all {
            match req {
                Request::Ping
                | Request::Open { .. }
                | Request::Drop { .. }
                | Request::DropRelease
                | Request::Run { .. }
                | Request::Actions { .. }
                | Request::CancelAll
                | Request::Settings
                | Request::Reload
                | Request::Quit => {}
            }
        }
        all
    }

    #[test]
    fn ac5_open_and_drop_grant_the_server_pid_and_nothing_else_does() {
        let reqs = every_request();
        assert_eq!(reqs.len(), 11);
        for (req, grants) in reqs {
            let want = if grants { Some(42) } else { None };
            assert_eq!(foreground_grant(&req, Some(42)), want, "{req:?}");
        }
    }

    #[test]
    fn ac5_no_server_pid_grants_nothing() {
        for (req, _) in every_request() {
            assert_eq!(foreground_grant(&req, None), None, "{req:?}");
        }
    }

    #[test]
    fn ac5_grant_passes_the_pid_through() {
        let open = Request::Open {
            files: vec![],
            token: None,
        };
        assert_eq!(foreground_grant(&open, Some(7)), Some(7));
        assert_eq!(foreground_grant(&open, Some(u32::MAX)), Some(u32::MAX));
    }
}
