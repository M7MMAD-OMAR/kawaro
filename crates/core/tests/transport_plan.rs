//! S-040b AC8, AC9, AC11 and AC12 (backlog/S-040-windows-ipc.md): the
//! Windows pipe decisions as pure functions, tested and mutated on Linux.
//!
//! Contract (tester, from AC8 to AC12; do not edit). In
//! `sbarhive_core::transport`, `pub` on every platform:
//! - `pub fn dacl_sddl(sid: &str) -> String`: a protected DACL with one
//!   allow ACE, generic all, for `sid` only.
//! - `pub enum BindOutcome { Listen, AlreadyRunning, Failed(std::io::Error) }`
//!   and `pub fn bind_result(first_instance_error: Option<u32>) -> BindOutcome`:
//!   `None` is `Listen`, `Some(5)` (`ERROR_ACCESS_DENIED`) is
//!   `AlreadyRunning`, any other code is `Failed` with that code in
//!   `raw_os_error()`.
//! - `pub enum ConnectStep { Connected, Wait(Duration), Fail(std::io::Error) }`
//!   and `pub fn connect_plan(open_error: Option<u32>, elapsed: Duration)
//!   -> ConnectStep`, one step of the client's connect loop, given the
//!   result of the last `CreateFileW` and the time spent since the first try:
//!   - `None` is `Connected`.
//!   - `Some(2)` (`ERROR_FILE_NOT_FOUND`) is `Fail` with
//!     `ErrorKind::NotFound`, at any elapsed time, never a wait.
//!   - `Some(231)` (`ERROR_PIPE_BUSY`) with `elapsed < 2000 ms` is
//!     `Wait(2000 ms - elapsed)`, the timeout to give `WaitNamedPipeW`
//!     before the next try; with `elapsed >= 2000 ms` it is `Fail` with
//!     `ErrorKind::TimedOut`.
//!   - Any other code is `Fail` with that code in `raw_os_error()`.
//! - `pub fn verify_owner<S>(server_sid: &str, client_sid: &str, stream: S)
//!   -> std::io::Result<S>`: hands the stream back only when the pipe server
//!   runs as the client's own user; otherwise `ErrorKind::PermissionDenied`
//!   and the stream is dropped unwritten.
//! - `pub fn open_mode(first: bool) -> u32` and `pub fn pipe_mode() -> u32`,
//!   the `CreateNamedPipeW` flags.
//!
//! Added by S-028b AC4 and AC5 (backlog/S-028-review-fixes.md), `pub` on
//! every platform:
//! - `pub fn accept_backoff(consecutive_errors: u32) -> Duration`: 0 for 0,
//!   50 ms for every value from 1 to `u32::MAX`.
//! - `pub enum NextInstance { Fresh, Keep, Empty }`,
//!   `pub struct AcceptPlan { pub hand_over: bool, pub next: NextInstance }`
//!   and `pub fn accept_plan(connected: bool, connect_error: Option<u32>,
//!   fresh_ok: bool) -> AcceptPlan`: `hand_over` is
//!   `connected || connect_error == Some(535)`; `next` is `Fresh` when
//!   `fresh_ok`, `Empty` when the client is handed over and the fresh
//!   instance failed, `Keep` when there is no client and it failed.

use sbarhive_core::transport::{
    AcceptPlan, BindOutcome, ConnectStep, NextInstance, accept_backoff, accept_plan, bind_result,
    connect_plan, dacl_sddl, open_mode, pipe_mode, verify_owner,
};
use std::io::{ErrorKind, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

const SID: &str = "S-1-5-21-1-2-3-1001";
const LONGEST_SID: &str = "S-1-5-21-4294967295-4294967295-4294967295-4294967295";
const ERROR_FILE_NOT_FOUND: u32 = 2;
const ERROR_ACCESS_DENIED: u32 = 5;
const ERROR_PIPE_BUSY: u32 = 231;

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

// AC8

#[test]
fn ac8_dacl_sddl_is_the_exact_literal() {
    assert_eq!(dacl_sddl(SID), "D:P(A;;GA;;;S-1-5-21-1-2-3-1001)");
}

#[test]
fn ac8_dacl_sddl_grants_no_one_else() {
    for sid in [SID, LONGEST_SID] {
        let sddl = dacl_sddl(sid);
        for other in ["WD", "AN", "BU"] {
            assert!(!sddl.contains(other), "{sddl} must not name {other}");
        }
    }
}

#[test]
fn ac8_dacl_sddl_embeds_the_given_sid() {
    assert_eq!(
        dacl_sddl(LONGEST_SID),
        format!("D:P(A;;GA;;;{LONGEST_SID})")
    );
    assert_ne!(dacl_sddl(SID), dacl_sddl(LONGEST_SID));
}

// AC9

#[test]
fn ac9_bind_result_none_listens() {
    assert!(matches!(bind_result(None), BindOutcome::Listen));
}

#[test]
fn ac9_bind_result_access_denied_is_already_running() {
    assert!(matches!(
        bind_result(Some(ERROR_ACCESS_DENIED)),
        BindOutcome::AlreadyRunning
    ));
}

#[test]
fn ac9_bind_result_other_codes_fail_with_the_same_code() {
    for code in [ERROR_FILE_NOT_FOUND, ERROR_PIPE_BUSY, 1, 87, 0xFFFF_FFF0] {
        match bind_result(Some(code)) {
            BindOutcome::Failed(e) => assert_eq!(
                e.raw_os_error(),
                Some(code as i32),
                "code {code} must be carried as is"
            ),
            BindOutcome::Listen => panic!("code {code} gave Listen"),
            BindOutcome::AlreadyRunning => panic!("code {code} gave AlreadyRunning"),
        }
    }
}

// AC11

#[test]
fn ac11_connect_plan_no_error_is_connected() {
    assert!(matches!(connect_plan(None, ms(0)), ConnectStep::Connected));
    assert!(matches!(
        connect_plan(None, ms(1999)),
        ConnectStep::Connected
    ));
}

#[test]
fn ac11_connect_plan_file_not_found_is_not_found_at_once() {
    for elapsed in [0, 1, 1999, 2000, 5000] {
        match connect_plan(Some(ERROR_FILE_NOT_FOUND), ms(elapsed)) {
            ConnectStep::Fail(e) => assert_eq!(e.kind(), ErrorKind::NotFound, "at {elapsed} ms"),
            ConnectStep::Wait(d) => panic!("no service: must not wait, got Wait({d:?})"),
            ConnectStep::Connected => panic!("no service: must not connect"),
        }
    }
}

#[test]
fn ac11_connect_plan_pipe_busy_waits_for_the_rest_of_2000_ms() {
    for (elapsed, rest) in [(0, 2000), (1, 1999), (750, 1250), (1999, 1)] {
        match connect_plan(Some(ERROR_PIPE_BUSY), ms(elapsed)) {
            ConnectStep::Wait(d) => assert_eq!(d, ms(rest), "at {elapsed} ms"),
            ConnectStep::Fail(e) => panic!("busy at {elapsed} ms must wait, got {e:?}"),
            ConnectStep::Connected => panic!("busy at {elapsed} ms must wait"),
        }
    }
}

#[test]
fn ac11_connect_plan_pipe_busy_times_out_at_2000_ms() {
    for elapsed in [2000, 2001, 10_000] {
        match connect_plan(Some(ERROR_PIPE_BUSY), ms(elapsed)) {
            ConnectStep::Fail(e) => assert_eq!(e.kind(), ErrorKind::TimedOut, "at {elapsed} ms"),
            ConnectStep::Wait(d) => panic!("busy at {elapsed} ms must time out, got Wait({d:?})"),
            ConnectStep::Connected => panic!("busy at {elapsed} ms must time out"),
        }
    }
}

#[test]
fn ac11_connect_plan_other_codes_fail_with_the_same_code() {
    for code in [ERROR_ACCESS_DENIED, 1, 87] {
        match connect_plan(Some(code), ms(0)) {
            ConnectStep::Fail(e) => assert_eq!(e.raw_os_error(), Some(code as i32)),
            ConnectStep::Wait(d) => panic!("code {code} gave Wait({d:?})"),
            ConnectStep::Connected => panic!("code {code} gave Connected"),
        }
    }
}

/// A pipe stand-in that counts the bytes written to it and its drops.
struct CountingPipe {
    written: Arc<AtomicUsize>,
    dropped: Arc<AtomicUsize>,
}

impl Write for CountingPipe {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.written.fetch_add(buf.len(), Ordering::SeqCst);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Drop for CountingPipe {
    fn drop(&mut self) {
        self.dropped.fetch_add(1, Ordering::SeqCst);
    }
}

fn pipe() -> (CountingPipe, Arc<AtomicUsize>, Arc<AtomicUsize>) {
    let written = Arc::new(AtomicUsize::new(0));
    let dropped = Arc::new(AtomicUsize::new(0));
    let p = CountingPipe {
        written: written.clone(),
        dropped: dropped.clone(),
    };
    (p, written, dropped)
}

#[test]
fn ac11_another_users_server_is_permission_denied_and_gets_no_byte() {
    let (p, written, dropped) = pipe();
    let err: std::io::Error = match verify_owner(LONGEST_SID, SID, p) {
        Ok(_) => panic!("a server run by another user must be refused"),
        Err(e) => e,
    };
    assert_eq!(err.kind(), ErrorKind::PermissionDenied);
    assert_eq!(written.load(Ordering::SeqCst), 0, "no byte may be sent");
    assert_eq!(dropped.load(Ordering::SeqCst), 1, "the pipe must be closed");
}

#[test]
fn ac11_own_server_hands_the_stream_back_for_writing() {
    let (p, written, dropped) = pipe();
    let mut p = verify_owner(SID, SID, p).expect("own server must be accepted");
    assert_eq!(
        written.load(Ordering::SeqCst),
        0,
        "the check itself sends nothing"
    );
    assert_eq!(dropped.load(Ordering::SeqCst), 0);
    p.write_all(b"{\"cmd\":\"ping\"}\n").unwrap();
    assert_eq!(written.load(Ordering::SeqCst), 15);
}

// AC12

#[test]
fn ac12_open_mode_first_instance_is_duplex_and_first_pipe_instance() {
    assert_eq!(open_mode(true), 0x0008_0003);
}

#[test]
fn ac12_open_mode_later_instances_are_duplex_only() {
    assert_eq!(open_mode(false), 0x3);
}

#[test]
fn ac12_pipe_mode_is_byte_wait_and_rejects_remote_clients() {
    assert_eq!(pipe_mode(), 0x8);
}

// S-028b AC4

#[test]
fn s028_ac4_accept_backoff_is_zero_without_errors() {
    assert_eq!(accept_backoff(0), Duration::ZERO);
}

#[test]
fn s028_ac4_accept_backoff_is_50_ms_after_any_error() {
    for n in [1, 2, 1000, u32::MAX] {
        assert_eq!(accept_backoff(n), ms(50), "after {n} errors in a row");
    }
}

// S-028b AC5

const ERROR_NO_DATA: u32 = 232;
const ERROR_PIPE_CONNECTED: u32 = 535;

fn next_name(n: &NextInstance) -> &'static str {
    match n {
        NextInstance::Fresh => "Fresh",
        NextInstance::Keep => "Keep",
        NextInstance::Empty => "Empty",
    }
}

fn plan_is(p: AcceptPlan, hand_over: bool, next: &'static str, row: &str) {
    assert_eq!(p.hand_over, hand_over, "{row}: hand_over");
    assert_eq!(next_name(&p.next), next, "{row}: next");
}

#[test]
fn s028_ac5_connected_with_a_fresh_instance_hands_over_and_takes_it() {
    plan_is(
        accept_plan(true, None, true),
        true,
        "Fresh",
        "(true, None, true)",
    );
}

#[test]
fn s028_ac5_pipe_connected_without_a_fresh_instance_hands_over_and_empties() {
    plan_is(
        accept_plan(false, Some(ERROR_PIPE_CONNECTED), false),
        true,
        "Empty",
        "(false, Some(535), false)",
    );
}

#[test]
fn s028_ac5_connected_without_a_fresh_instance_empties() {
    plan_is(
        accept_plan(true, None, false),
        true,
        "Empty",
        "(true, None, false)",
    );
}

#[test]
fn s028_ac5_failed_connect_with_a_fresh_instance_takes_it() {
    plan_is(
        accept_plan(false, Some(ERROR_NO_DATA), true),
        false,
        "Fresh",
        "(false, Some(232), true)",
    );
}

#[test]
fn s028_ac5_failed_connect_without_a_fresh_instance_keeps_the_old_one() {
    plan_is(
        accept_plan(false, Some(ERROR_NO_DATA), false),
        false,
        "Keep",
        "(false, Some(232), false)",
    );
}
