//! S-099a A6: the helper recovery check (S-097 regression), with a fake
//! `HelperOps` that counts every call.
//!
//! Test hooks (pinned in backlog/S-099-update-experience.md):
//! - `HelperOps::daemon_present(&self) -> bool` (new required method).
//! - `kawaro_core::update::apply::msiexec_repair_args(msi: &Path, log: &Path)
//!   -> Vec<String>` = `/fa <msi> /qn /norestart /l*v <log>`.
//! - `run_helper` keeps its signature; after `Installed` (exit 0 or 3010) it
//!   calls `daemon_present` once; when false it runs the repair once and
//!   checks again; still false: `Failed(Text("kawarod.exe is missing after the
//!   update. Run <msi path> by hand."))`, the result is written and no
//!   daemon is started.

use std::cell::{Cell, RefCell};
use std::path::Path;
use std::time::Duration;

use kawaro_core::update::UpdateError;
use kawaro_core::update::apply::{
    Daemon, HelperOps, MsiResult, msiexec_args, msiexec_repair_args, run_helper,
};
use kawaro_core::update::result::{Failure, Outcome, UpdateResult};

const MSI: &str = "C:/Cache/kawaro.msi";
const LOG: &str = "C:/Temp/kawaro-update.log";

struct Fake {
    events: RefCell<Vec<String>>,
    /// Exit codes handed out by `run_msiexec`, in order.
    codes: RefCell<Vec<Result<i32, UpdateError>>>,
    /// Answers handed out by `daemon_present`, in order; the last repeats.
    presence: Vec<bool>,
    presence_calls: Cell<usize>,
    /// `events` length at each `daemon_present` call.
    presence_after: RefCell<Vec<Vec<String>>>,
    msiexec_args: RefCell<Vec<Vec<String>>>,
    written: RefCell<Option<UpdateResult>>,
}

impl Fake {
    fn new(codes: Vec<Result<i32, UpdateError>>, presence: Vec<bool>) -> Fake {
        Fake {
            events: RefCell::new(Vec::new()),
            codes: RefCell::new(codes),
            presence,
            presence_calls: Cell::new(0),
            presence_after: RefCell::new(Vec::new()),
            msiexec_args: RefCell::new(Vec::new()),
            written: RefCell::new(None),
        }
    }
    fn events(&self) -> Vec<String> {
        self.events.borrow().clone()
    }
}

impl HelperOps for Fake {
    fn wait_for_daemon_exit(&self, _timeout: Duration) -> bool {
        self.events.borrow_mut().push("wait".into());
        true
    }
    fn run_msiexec(&self, args: &[String]) -> Result<i32, UpdateError> {
        let name = if args.first().map(String::as_str) == Some("/fa") {
            "repair"
        } else {
            "install"
        };
        self.events.borrow_mut().push(name.into());
        self.msiexec_args.borrow_mut().push(args.to_vec());
        let mut codes = self.codes.borrow_mut();
        if codes.is_empty() {
            Ok(0)
        } else {
            codes.remove(0)
        }
    }
    fn daemon_present(&self) -> bool {
        let n = self.presence_calls.get();
        self.presence_calls.set(n + 1);
        self.presence_after.borrow_mut().push(self.events());
        *self
            .presence
            .get(n)
            .or(self.presence.last())
            .expect("a presence script")
    }
    fn write_result(&self, r: &UpdateResult) -> Result<(), UpdateError> {
        self.events.borrow_mut().push("write".into());
        *self.written.borrow_mut() = Some(r.clone());
        Ok(())
    }
    fn start_daemon(&self, which: Daemon) {
        let name = match which {
            Daemon::New => "start-new",
            Daemon::Old => "start-old",
        };
        self.events.borrow_mut().push(name.into());
    }
}

fn run(fake: &Fake) -> UpdateResult {
    run_helper(fake, Path::new(MSI), Path::new(LOG), "0.4.3", 77)
}

fn installed() -> UpdateResult {
    UpdateResult {
        version: "0.4.3".into(),
        outcome: Outcome::Installed,
        at: 77,
    }
}

#[test]
fn ac_a6_repair_args_are_exactly_the_story_line() {
    let args = msiexec_repair_args(Path::new("C:/a b/k.msi"), Path::new("C:/Temp/u.log"));
    let want: Vec<String> = [
        "/fa",
        "C:/a b/k.msi",
        "/qn",
        "/norestart",
        "/l*v",
        "C:/Temp/u.log",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    assert_eq!(args, want);
}

#[test]
fn ac_a6_exit_0_with_the_daemon_present_checks_once_and_repairs_nothing() {
    let fake = Fake::new(vec![Ok(0)], vec![true]);
    assert_eq!(run(&fake), installed());
    assert_eq!(fake.events(), ["wait", "install", "write", "start-new"]);
    assert_eq!(fake.presence_calls.get(), 1);
    assert_eq!(fake.msiexec_args.borrow().len(), 1, "exactly 1 run_msiexec");
    assert_eq!(fake.presence_after.borrow()[0], ["wait", "install"]);
}

#[test]
fn ac_a6_exit_0_daemon_missing_then_present_repairs_once_and_starts_the_new_daemon() {
    let fake = Fake::new(vec![Ok(0), Ok(0)], vec![false, true]);
    let got = run(&fake);
    assert_eq!(got, installed());
    assert_eq!(
        fake.events(),
        ["wait", "install", "repair", "write", "start-new"]
    );
    assert_eq!(fake.presence_calls.get(), 2);
    assert_eq!(
        fake.presence_after.borrow()[1],
        ["wait", "install", "repair"],
        "the second check comes after the repair"
    );
    let calls = fake.msiexec_args.borrow();
    assert_eq!(calls.len(), 2, "exactly 2 run_msiexec calls");
    assert_eq!(calls[0], msiexec_args(Path::new(MSI), Path::new(LOG)));
    assert_eq!(
        calls[1],
        msiexec_repair_args(Path::new(MSI), Path::new(LOG))
    );
    assert_eq!(fake.written.borrow().clone(), Some(got));
}

#[test]
fn ac_a6_daemon_missing_twice_is_an_actionable_failure_and_starts_no_daemon() {
    let fake = Fake::new(vec![Ok(0), Ok(0)], vec![false, false]);
    let got = run(&fake);
    let text = "kawarod.exe is missing after the update. Run C:/Cache/kawaro.msi by hand.";
    assert_eq!(
        got,
        UpdateResult {
            version: "0.4.3".into(),
            outcome: Outcome::Failed(Failure::Text(text.into())),
            at: 77,
        }
    );
    assert_eq!(fake.events(), ["wait", "install", "repair", "write"]);
    assert!(
        !fake.events().iter().any(|e| e.starts_with("start")),
        "no start_daemon call at all"
    );
    assert_eq!(fake.presence_calls.get(), 2, "no third check");
    assert_eq!(fake.msiexec_args.borrow().len(), 2, "at most one repair");
    assert_eq!(fake.written.borrow().clone(), Some(got));
}

#[test]
fn ac_a6_exit_3010_behaves_like_exit_0() {
    let fake = Fake::new(vec![Ok(3010), Ok(0)], vec![false, true]);
    assert_eq!(run(&fake), installed());
    assert_eq!(
        fake.events(),
        ["wait", "install", "repair", "write", "start-new"]
    );
    let fake = Fake::new(vec![Ok(3010), Ok(0)], vec![false, false]);
    let got = run(&fake);
    assert!(matches!(got.outcome, Outcome::Failed(Failure::Text(ref t))
        if t.contains("kawarod.exe") && t.contains(MSI)));
    assert_eq!(fake.events(), ["wait", "install", "repair", "write"]);
    let fake = Fake::new(vec![Ok(3010)], vec![true]);
    assert_eq!(run(&fake), installed());
    assert_eq!(fake.events(), ["wait", "install", "write", "start-new"]);
    assert_eq!(fake.presence_calls.get(), 1);
}

#[test]
fn ac_a6_failed_cancelled_and_busy_codes_skip_the_presence_check() {
    for (code, class) in [
        (1603, MsiResult::Failed),
        (1602, MsiResult::Cancelled),
        (1618, MsiResult::Busy),
    ] {
        // The daemon is "missing" in the script: it must never be asked.
        let fake = Fake::new(vec![Ok(code)], vec![false]);
        let got = run(&fake);
        assert_eq!(
            got.outcome,
            Outcome::Failed(Failure::Msi(class)),
            "code {code}"
        );
        assert_eq!(fake.events(), ["wait", "install", "write", "start-old"]);
        assert_eq!(fake.presence_calls.get(), 0, "code {code}");
        assert_eq!(fake.msiexec_args.borrow().len(), 1, "code {code}");
    }
}

#[test]
fn ac_a6_a_msiexec_that_cannot_start_skips_the_presence_check() {
    let fake = Fake::new(vec![Err(UpdateError::Unavailable)], vec![false]);
    let got = run(&fake);
    assert!(matches!(got.outcome, Outcome::Failed(Failure::Text(_))));
    assert_eq!(fake.events(), ["wait", "install", "write", "start-old"]);
    assert_eq!(fake.presence_calls.get(), 0);
}
