//! S-054a (backlog/S-054a-update-apply.md, docs/adr/0006-auto-update.md):
//! the pure parts of applying a verified update. Written by the tester from
//! the acceptance criteria. Portable: nothing here starts a real program, so
//! it also builds for Windows (`scripts/windows/check.sh`). The tests that run
//! fake package managers are in `s054a_apply_privileged.rs`.
//!
//! Contract (the implementer follows it; the tests are not edited). Where the
//! story leaves a gap the choice is marked DECISION and reported.
//!
//! `update::apply` (`crates/core/src/update/apply.rs`), all `pub`:
//! - `msiexec_args(msi: &Path, log: &Path) -> Vec<String>`.
//! - `enum MsiResult { Installed, InstalledRebootPending, Cancelled, Busy,
//!   Failed }` (`Debug + Clone + Copy + PartialEq + Eq + Serialize +
//!   Deserialize`, serde `rename_all = "snake_case"`); `classify_msi(i32)`.
//! - `enum PkgKind { Rpm, Deb }` (`Debug + Clone + Copy + PartialEq + Eq`) with
//!   `as_str()` giving `rpm` or `deb` (DECISION: the `--kind` text).
//! - `struct Facts { has_dnf, has_apt, has_pkexec, repo_configured: bool,
//!   installed_kind: Option<PkgKind> }` (`Debug + Clone + Copy + PartialEq +
//!   Eq + Default`); `enum Plan { RepoUpgrade(String), Local(PkgKind),
//!   ShowCommand(String), ShowRelease(String) }` (`Debug + Clone + PartialEq
//!   + Eq`); `linux_plan(&Facts) -> Plan`; `const RELEASE_PAGE_URL: &str`.
//!   DECISION (installed_kind is in the story but not in its table): with both
//!   dnf and apt present, `installed_kind` picks the manager (`Rpm` dnf,
//!   `Deb` apt); `None` prefers dnf. With one manager only, it is ignored.
//!
//!   DECISION (`ShowCommand` text): the repo command with `sudo` in place of
//!   `pkexec`; for a local package, `sudo dnf install -y {file}` or
//!   `sudo apt-get install -y {file}`, where the literal `{file}` is replaced
//!   by the daemon. `ShowRelease` carries `RELEASE_PAGE_URL`, which is
//!   `https://github.com/M7MMAD-OMAR/sbarhive/releases/latest`.
//! - `struct Programs { dnf: PathBuf, apt_get: PathBuf }` (`Debug + Clone`);
//!   `install_privileged(file: &Path, sha256: &str, kind: PkgKind, root_dir:
//!   &Path, programs: &Programs) -> Result<(), UpdateError>` (see the
//!   privileged test file for the details). `fn root_gate(euid: u32) ->
//!   Result<(), i32>` is `Ok` for 0 and `Err(EXIT_NOT_ROOT)` otherwise;
//!   `const EXIT_NOT_ROOT: i32 = 77`.
//! - `enum AutoAction { ApplyWhenIdle, NotifyReady }` (`Debug + Clone + Copy +
//!   PartialEq + Eq`); `auto_action(UpdateMode, paths::Os) -> AutoAction`.
//! - `can_apply(running_jobs: usize, hive_open: bool) -> bool`;
//!   `const RETRY_INTERVAL: Duration` (30 s).
//! - `trait CommandRunner { fn run(&self, argv: &[String]) -> Result<i32,
//!   UpdateError>; }` (argv[0] is the program, the result its exit code);
//!   `privileged_argv(exe: &Path, file: &Path, sha256: &str, kind: PkgKind) ->
//!   Vec<String>`; `apply_now(plan: &Plan, exe: &Path, file: &Path, sha256:
//!   &str, runner: &dyn CommandRunner) -> Result<(), UpdateError>`: the only
//!   place that builds a `pkexec` command line. `RepoUpgrade(cmd)` runs
//!   `cmd` split on spaces; `Local(kind)` runs `privileged_argv`; `ShowCommand`
//!   and `ShowRelease` run nothing and give `Err(UpdateError::Unavailable)`
//!   (DECISION); a non zero exit code or a runner error is `Err(UpdateError::Io)`
//!   for the code (DECISION) and the runner's own error otherwise.
//! - Windows helper, pure: `helper_args(msi: &Path) -> Vec<String>` (`update
//!   --apply-msi <msi>`); `msiexec_path(env: EnvFn) -> Option<PathBuf>`
//!   (`<SystemRoot>/System32/msiexec.exe`, `None` when `SystemRoot` is unset
//!   or empty, never `PATH`); `enum Daemon { New, Old }`; `trait HelperOps {
//!   fn wait_for_daemon_exit(&self, timeout: Duration) -> bool; fn
//!   run_msiexec(&self, args: &[String]) -> Result<i32, UpdateError>; fn
//!   write_result(&self, r: &UpdateResult) -> Result<(), UpdateError>; fn
//!   start_daemon(&self, which: Daemon); }`; `const DAEMON_EXIT_TIMEOUT:
//!   Duration` (10 s to 5 min); `run_helper(ops: &dyn HelperOps, msi: &Path,
//!   log: &Path, version: &str, now: u64) -> UpdateResult`. Order: wait,
//!   msiexec, write the result, start the daemon (`New` for `Installed` and
//!   `InstalledRebootPending`, `Old` otherwise). DECISION: when the daemon does
//!   not exit in time, msiexec never runs, the result is `Failed(Text(..))`
//!   and no daemon is started (the old one is still running). DECISION: a
//!   result that cannot be written still starts the daemon.
//! - `apply/windows.rs`: `apply_msi(msi: &Path) -> Result<(), UpdateError>`
//!   under `#[cfg(windows)]`; read as source.
//!
//! `update::result` (`crates/core/src/update/result.rs`):
//! - `UpdateResult { version: String, outcome: Outcome, at: u64 }` (`Debug +
//!   Clone + PartialEq + Serialize + Deserialize`), `Outcome { Installed,
//!   Failed(Failure) }`, `Failure { Msi(MsiResult), Text(String) }`, serde
//!   `rename_all = "snake_case"`: the file holds `{"version":"0.4.0",
//!   "outcome":"installed","at":5}` or `"outcome":{"failed":{"msi":"busy"}}`
//!   or `{"failed":{"text":"boom"}}` (DECISION: the JSON shape).
//! - `Outcome::from_msi(MsiResult) -> Outcome` (DECISION: `Installed` and
//!   `InstalledRebootPending` are `Installed`, the rest `Failed(Msi(r))`).
//! - `const RESULT_FILE: &str = "update-result.json"`;
//!   `write_result(dir: &Path, &UpdateResult) -> Result<(), UpdateError>`
//!   (creates `dir`, writes `<RESULT_FILE>.tmp`-style temp then renames, leaves
//!   no other file; `Err(Io)` when it cannot); `read_result(dir) ->
//!   Option<UpdateResult>` (no deletion); `take_result(dir) ->
//!   Option<UpdateResult>` (reads, then deletes the file).

use std::cell::RefCell;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use sbarhive_core::config::UpdateMode;
use sbarhive_core::paths::Os;
use sbarhive_core::update::UpdateError;
use sbarhive_core::update::apply::{
    AutoAction, CommandRunner, DAEMON_EXIT_TIMEOUT, Daemon, EXIT_NOT_ROOT, Facts, HelperOps,
    MsiResult, PkgKind, Plan, RELEASE_PAGE_URL, RETRY_INTERVAL, apply_now, auto_action, can_apply,
    classify_msi, helper_args, linux_plan, msiexec_args, msiexec_path, privileged_argv, root_gate,
    run_helper,
};
use sbarhive_core::update::result::{
    Failure, Outcome, RESULT_FILE, UpdateResult, read_result, take_result, write_result,
};

fn s(items: &[&str]) -> Vec<String> {
    items.iter().map(|x| x.to_string()).collect()
}

fn root(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(name)
}

// ------------------------------------------------------------ AC1

#[test]
fn ac1_msiexec_args_are_exactly_the_quiet_install() {
    let args = msiexec_args(
        Path::new("C:/Users/a b/sbarhive.msi"),
        Path::new("C:/Temp/up date.log"),
    );
    assert_eq!(
        args,
        s(&[
            "/i",
            "C:/Users/a b/sbarhive.msi",
            "/qn",
            "/norestart",
            "/l*v",
            "C:/Temp/up date.log"
        ])
    );
}

#[test]
fn ac1_the_six_exit_codes() {
    assert_eq!(classify_msi(0), MsiResult::Installed);
    assert_eq!(classify_msi(3010), MsiResult::InstalledRebootPending);
    assert_eq!(classify_msi(1602), MsiResult::Cancelled);
    assert_eq!(classify_msi(1618), MsiResult::Busy);
    assert_eq!(classify_msi(1603), MsiResult::Failed);
    assert_eq!(classify_msi(1), MsiResult::Failed);
}

#[test]
fn ac1_neighbours_of_the_known_codes_and_negatives_are_failed() {
    for code in [
        -1,
        2,
        1601,
        1604,
        1617,
        1619,
        3009,
        3011,
        i32::MIN,
        i32::MAX,
    ] {
        assert_eq!(classify_msi(code), MsiResult::Failed, "code {code}");
    }
}

// ------------------------------------------------------------ AC2

const DNF_REPO: &str = "pkexec dnf upgrade -y sbarhive";
const APT_REPO: &str = "pkexec apt-get install -y --only-upgrade sbarhive";

fn facts(dnf: bool, apt: bool, pkexec: bool, repo: bool, kind: Option<PkgKind>) -> Facts {
    Facts {
        has_dnf: dnf,
        has_apt: apt,
        has_pkexec: pkexec,
        repo_configured: repo,
        installed_kind: kind,
    }
}

#[test]
fn ac2_repo_with_dnf_and_pkexec_upgrades_through_the_repo() {
    let f = facts(true, false, true, true, Some(PkgKind::Rpm));
    assert_eq!(linux_plan(&f), Plan::RepoUpgrade(DNF_REPO.to_string()));
}

#[test]
fn ac2_repo_with_apt_and_pkexec_upgrades_through_the_repo() {
    let f = facts(false, true, true, true, Some(PkgKind::Deb));
    assert_eq!(linux_plan(&f), Plan::RepoUpgrade(APT_REPO.to_string()));
}

#[test]
fn ac2_dnf_only_installs_the_local_rpm() {
    let f = facts(true, false, true, false, Some(PkgKind::Rpm));
    assert_eq!(linux_plan(&f), Plan::Local(PkgKind::Rpm));
}

#[test]
fn ac2_apt_only_installs_the_local_deb() {
    let f = facts(false, true, true, false, Some(PkgKind::Deb));
    assert_eq!(linux_plan(&f), Plan::Local(PkgKind::Deb));
}

#[test]
fn ac2_installed_kind_is_ignored_when_only_one_manager_exists() {
    assert_eq!(
        linux_plan(&facts(true, false, true, false, None)),
        Plan::Local(PkgKind::Rpm)
    );
    assert_eq!(
        linux_plan(&facts(false, true, true, false, None)),
        Plan::Local(PkgKind::Deb)
    );
    assert_eq!(
        linux_plan(&facts(false, true, true, true, Some(PkgKind::Rpm))),
        Plan::RepoUpgrade(APT_REPO.to_string())
    );
}

#[test]
fn ac2_decision_both_managers_follow_the_installed_kind_dnf_first_when_unknown() {
    assert_eq!(
        linux_plan(&facts(true, true, true, false, Some(PkgKind::Deb))),
        Plan::Local(PkgKind::Deb)
    );
    assert_eq!(
        linux_plan(&facts(true, true, true, false, Some(PkgKind::Rpm))),
        Plan::Local(PkgKind::Rpm)
    );
    assert_eq!(
        linux_plan(&facts(true, true, true, false, None)),
        Plan::Local(PkgKind::Rpm)
    );
    assert_eq!(
        linux_plan(&facts(true, true, true, true, Some(PkgKind::Deb))),
        Plan::RepoUpgrade(APT_REPO.to_string())
    );
    assert_eq!(
        linux_plan(&facts(true, true, true, true, None)),
        Plan::RepoUpgrade(DNF_REPO.to_string())
    );
}

#[test]
fn ac2_no_pkexec_shows_the_exact_command_for_the_manager_found() {
    assert_eq!(
        linux_plan(&facts(true, false, false, true, Some(PkgKind::Rpm))),
        Plan::ShowCommand("sudo dnf upgrade -y sbarhive".to_string())
    );
    assert_eq!(
        linux_plan(&facts(false, true, false, true, Some(PkgKind::Deb))),
        Plan::ShowCommand("sudo apt-get install -y --only-upgrade sbarhive".to_string())
    );
    assert_eq!(
        linux_plan(&facts(true, false, false, false, Some(PkgKind::Rpm))),
        Plan::ShowCommand("sudo dnf install -y {file}".to_string())
    );
    assert_eq!(
        linux_plan(&facts(false, true, false, false, Some(PkgKind::Deb))),
        Plan::ShowCommand("sudo apt-get install -y {file}".to_string())
    );
}

#[test]
fn ac2_neither_manager_shows_the_release_page() {
    let url = "https://github.com/M7MMAD-OMAR/sbarhive/releases/latest";
    assert_eq!(RELEASE_PAGE_URL, url);
    for pkexec in [true, false] {
        for repo in [true, false] {
            assert_eq!(
                linux_plan(&facts(false, false, pkexec, repo, None)),
                Plan::ShowRelease(url.to_string()),
                "pkexec {pkexec} repo {repo}"
            );
        }
    }
}

// ------------------------------------------------------------ AC4, AC5

#[test]
fn ac4_auto_action_table() {
    assert_eq!(
        auto_action(UpdateMode::Auto, Os::Windows),
        AutoAction::ApplyWhenIdle
    );
    assert_eq!(
        auto_action(UpdateMode::Auto, Os::Linux),
        AutoAction::NotifyReady
    );
    assert_eq!(
        auto_action(UpdateMode::Ask, Os::Linux),
        AutoAction::NotifyReady
    );
    assert_eq!(
        auto_action(UpdateMode::Ask, Os::Windows),
        AutoAction::NotifyReady
    );
}

#[test]
fn ac5_can_apply_only_when_idle_and_no_hive() {
    assert!(can_apply(0, false));
    assert!(!can_apply(0, true));
    assert!(!can_apply(1, false));
    assert!(!can_apply(1, true));
    assert!(!can_apply(usize::MAX, false));
}

#[test]
fn ac5_retry_interval_is_thirty_seconds() {
    assert_eq!(RETRY_INTERVAL, Duration::from_secs(30));
}

// ---------------------------------------------- AC4: pkexec only in apply_now

struct Recorder {
    calls: RefCell<Vec<Vec<String>>>,
    code: i32,
}

impl Recorder {
    fn new(code: i32) -> Recorder {
        Recorder {
            calls: RefCell::new(Vec::new()),
            code,
        }
    }
}

impl CommandRunner for Recorder {
    fn run(&self, argv: &[String]) -> Result<i32, UpdateError> {
        self.calls.borrow_mut().push(argv.to_vec());
        Ok(self.code)
    }
}

const SHA: &str = "bff5b3ca49e18ea61c9f21a39f95922bcf75f2e0f8ffedf46e07c7cfd70d5d64";

#[test]
fn ac4_privileged_argv_is_pkexec_then_the_sbarhive_update_call() {
    let argv = privileged_argv(
        Path::new("/usr/bin/sbarhive"),
        Path::new("/home/u/.cache/sbarhive/updates/0.4.0/sbarhive-0.4.0-1.x86_64.rpm"),
        SHA,
        PkgKind::Rpm,
    );
    assert_eq!(
        argv,
        s(&[
            "pkexec",
            "/usr/bin/sbarhive",
            "update",
            "--install-privileged",
            "/home/u/.cache/sbarhive/updates/0.4.0/sbarhive-0.4.0-1.x86_64.rpm",
            "--sha256",
            SHA,
            "--kind",
            "rpm"
        ])
    );
    let deb = privileged_argv(
        Path::new("/x/sbarhive"),
        Path::new("/p.deb"),
        SHA,
        PkgKind::Deb,
    );
    assert_eq!(deb.last().map(String::as_str), Some("deb"));
}

#[test]
fn ac4_apply_now_repo_upgrade_runs_the_plan_command_once() {
    let r = Recorder::new(0);
    let plan = Plan::RepoUpgrade(DNF_REPO.to_string());
    apply_now(
        &plan,
        Path::new("/usr/bin/sbarhive"),
        Path::new("/p.rpm"),
        SHA,
        &r,
    )
    .unwrap();
    assert_eq!(
        *r.calls.borrow(),
        vec![s(&["pkexec", "dnf", "upgrade", "-y", "sbarhive"])]
    );

    let r = Recorder::new(0);
    let plan = Plan::RepoUpgrade(APT_REPO.to_string());
    apply_now(
        &plan,
        Path::new("/usr/bin/sbarhive"),
        Path::new("/p.deb"),
        SHA,
        &r,
    )
    .unwrap();
    assert_eq!(
        *r.calls.borrow(),
        vec![s(&[
            "pkexec",
            "apt-get",
            "install",
            "-y",
            "--only-upgrade",
            "sbarhive"
        ])]
    );
}

#[test]
fn ac4_apply_now_local_runs_the_privileged_call_once() {
    let r = Recorder::new(0);
    let exe = Path::new("/usr/bin/sbarhive");
    let file = Path::new("/c/sbarhive.deb");
    apply_now(&Plan::Local(PkgKind::Deb), exe, file, SHA, &r).unwrap();
    assert_eq!(
        *r.calls.borrow(),
        vec![privileged_argv(exe, file, SHA, PkgKind::Deb)]
    );
}

#[test]
fn ac4_apply_now_runs_nothing_for_show_plans() {
    for plan in [
        Plan::ShowCommand("sudo dnf install -y {file}".to_string()),
        Plan::ShowRelease(RELEASE_PAGE_URL.to_string()),
    ] {
        let r = Recorder::new(0);
        let out = apply_now(&plan, Path::new("/e"), Path::new("/f.rpm"), SHA, &r);
        assert_eq!(out, Err(UpdateError::Unavailable), "{plan:?}");
        assert!(r.calls.borrow().is_empty(), "{plan:?}");
    }
}

#[test]
fn ac4_apply_now_reports_a_failing_exit_code_as_io() {
    for code in [1, 126, 127] {
        let r = Recorder::new(code);
        let out = apply_now(
            &Plan::Local(PkgKind::Rpm),
            Path::new("/e"),
            Path::new("/f.rpm"),
            SHA,
            &r,
        );
        assert_eq!(out, Err(UpdateError::Io), "code {code}");
    }
}

#[test]
fn ac4_apply_now_passes_a_runner_error_through() {
    struct Broken;
    impl CommandRunner for Broken {
        fn run(&self, _: &[String]) -> Result<i32, UpdateError> {
            Err(UpdateError::Unavailable)
        }
    }
    let out = apply_now(
        &Plan::RepoUpgrade(DNF_REPO.to_string()),
        Path::new("/e"),
        Path::new("/f.rpm"),
        SHA,
        &Broken,
    );
    assert_eq!(out, Err(UpdateError::Unavailable));
}

/// The text of `apply.rs` without comment lines.
fn apply_code() -> String {
    let text = fs::read_to_string(root("src/update/apply.rs"))
        .expect("crates/core/src/update/apply.rs exists");
    text.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn ac4_pkexec_appears_only_in_the_plan_text_and_the_privileged_argv() {
    let code = apply_code();
    let lines: Vec<&str> = code.lines().filter(|l| l.contains("pkexec")).collect();
    assert!(!lines.is_empty(), "the plan text names pkexec");
    // No call to a process API on a line that names pkexec, and no
    // `Command` anywhere that is not behind `install_privileged`'s managers.
    for l in &lines {
        assert!(
            !l.contains("Command::new") && !l.contains(".spawn(") && !l.contains(".status("),
            "{l}"
        );
    }
}

#[test]
fn ac3_no_nogpgcheck_anywhere_in_apply() {
    let code = apply_code();
    assert!(!code.contains("nogpgcheck"));
    assert!(!code.contains("--allow-unauthenticated"));
    assert!(!code.contains("--force-yes"));
}

// ------------------------------------------------------------ AC8 pure part

#[test]
fn ac8_root_gate_lets_only_root_through() {
    assert_eq!(EXIT_NOT_ROOT, 77);
    assert_eq!(root_gate(0), Ok(()));
    assert_eq!(root_gate(1), Err(77));
    assert_eq!(root_gate(1000), Err(77));
    assert_eq!(root_gate(u32::MAX), Err(77));
}

// ------------------------------------------------------------ AC6

fn sample(outcome: Outcome) -> UpdateResult {
    UpdateResult {
        version: "0.4.0".to_string(),
        outcome,
        at: 1_761_300_000,
    }
}

#[test]
fn ac6_write_then_read_gives_the_same_result() {
    let dir = tempfile::tempdir().unwrap();
    for outcome in [
        Outcome::Installed,
        Outcome::Failed(Failure::Msi(MsiResult::Busy)),
        Outcome::Failed(Failure::Msi(MsiResult::InstalledRebootPending)),
        Outcome::Failed(Failure::Text("the daemon did not exit".to_string())),
    ] {
        let r = sample(outcome);
        write_result(dir.path(), &r).unwrap();
        assert_eq!(read_result(dir.path()), Some(r.clone()));
        // Reading again does not consume it.
        assert_eq!(read_result(dir.path()), Some(r));
    }
}

#[test]
fn ac6_the_file_name_and_json_shape() {
    assert_eq!(RESULT_FILE, "update-result.json");
    let dir = tempfile::tempdir().unwrap();
    write_result(dir.path(), &sample(Outcome::Installed)).unwrap();
    let text = fs::read_to_string(dir.path().join("update-result.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        v,
        serde_json::json!({"version": "0.4.0", "outcome": "installed", "at": 1_761_300_000u64})
    );
    write_result(
        dir.path(),
        &sample(Outcome::Failed(Failure::Msi(MsiResult::Busy))),
    )
    .unwrap();
    let v: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.path().join(RESULT_FILE)).unwrap()).unwrap();
    assert_eq!(v["outcome"], serde_json::json!({"failed": {"msi": "busy"}}));
    write_result(
        dir.path(),
        &sample(Outcome::Failed(Failure::Text("boom".to_string()))),
    )
    .unwrap();
    let v: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.path().join(RESULT_FILE)).unwrap()).unwrap();
    assert_eq!(
        v["outcome"],
        serde_json::json!({"failed": {"text": "boom"}})
    );
}

#[test]
fn ac6_a_hand_written_file_reads_back() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(RESULT_FILE),
        r#"{"version":"1.2.3","outcome":{"failed":{"msi":"installed_reboot_pending"}},"at":5}"#,
    )
    .unwrap();
    assert_eq!(
        read_result(dir.path()),
        Some(UpdateResult {
            version: "1.2.3".to_string(),
            outcome: Outcome::Failed(Failure::Msi(MsiResult::InstalledRebootPending)),
            at: 5,
        })
    );
}

#[test]
fn ac6_write_is_atomic_leaves_one_file_and_replaces_the_old_one() {
    let dir = tempfile::tempdir().unwrap();
    write_result(dir.path(), &sample(Outcome::Installed)).unwrap();
    let newer = UpdateResult {
        version: "0.5.0".to_string(),
        outcome: Outcome::Failed(Failure::Msi(MsiResult::Cancelled)),
        at: 9,
    };
    write_result(dir.path(), &newer).unwrap();
    let names: Vec<OsString> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names, vec![OsString::from(RESULT_FILE)]);
    assert_eq!(read_result(dir.path()), Some(newer));
}

#[test]
fn ac6_write_creates_a_missing_folder_and_reports_io_when_it_cannot() {
    let dir = tempfile::tempdir().unwrap();
    let nested = dir.path().join("a").join("b");
    write_result(&nested, &sample(Outcome::Installed)).unwrap();
    assert!(nested.join(RESULT_FILE).is_file());

    let blocker = dir.path().join("plain-file");
    fs::write(&blocker, b"x").unwrap();
    assert_eq!(
        write_result(&blocker, &sample(Outcome::Installed)),
        Err(UpdateError::Io)
    );
}

#[test]
fn ac6_take_result_reports_once() {
    let dir = tempfile::tempdir().unwrap();
    let r = sample(Outcome::Failed(Failure::Msi(MsiResult::Failed)));
    write_result(dir.path(), &r).unwrap();
    assert_eq!(take_result(dir.path()), Some(r));
    assert!(!dir.path().join(RESULT_FILE).exists());
    assert_eq!(take_result(dir.path()), None);
    assert_eq!(read_result(dir.path()), None);
}

#[test]
fn ac6_a_missing_empty_or_truncated_file_reads_as_none() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(read_result(dir.path()), None);
    assert_eq!(take_result(dir.path()), None);
    assert_eq!(read_result(&dir.path().join("no-such-folder")), None);

    let path = dir.path().join(RESULT_FILE);
    fs::write(&path, b"").unwrap();
    assert_eq!(read_result(dir.path()), None);

    write_result(dir.path(), &sample(Outcome::Installed)).unwrap();
    let full = fs::read(&path).unwrap();
    fs::write(&path, &full[..full.len() / 2]).unwrap();
    assert_eq!(read_result(dir.path()), None);
    assert_eq!(take_result(dir.path()), None);

    fs::write(&path, br#"{"version":"0.4.0"}"#).unwrap();
    assert_eq!(read_result(dir.path()), None);
}

#[test]
fn ac6_outcome_from_msi_maps_the_classes() {
    assert_eq!(Outcome::from_msi(MsiResult::Installed), Outcome::Installed);
    assert_eq!(
        Outcome::from_msi(MsiResult::InstalledRebootPending),
        Outcome::Installed
    );
    for r in [MsiResult::Cancelled, MsiResult::Busy, MsiResult::Failed] {
        assert_eq!(Outcome::from_msi(r), Outcome::Failed(Failure::Msi(r)));
    }
}

// ------------------------------------------------------------ AC7 pure parts

#[test]
fn ac7_helper_args_start_the_copy_with_apply_msi() {
    assert_eq!(
        helper_args(Path::new("C:/Cache/sbarhive 0.4.0.msi")),
        s(&["update", "--apply-msi", "C:/Cache/sbarhive 0.4.0.msi"])
    );
}

#[test]
fn ac7_msiexec_is_found_by_absolute_path_never_through_path() {
    let env = |k: &str| -> Option<OsString> {
        match k {
            "SystemRoot" => Some(OsString::from("C:/Windows")),
            "PATH" => Some(OsString::from("/evil")),
            _ => None,
        }
    };
    assert_eq!(
        msiexec_path(&env),
        Some(Path::new("C:/Windows").join("System32").join("msiexec.exe"))
    );
    assert_eq!(msiexec_path(&|_: &str| None), None);
    assert_eq!(
        msiexec_path(&|k: &str| (k == "SystemRoot").then(|| OsString::from(""))),
        None
    );
    assert_eq!(
        msiexec_path(&|k: &str| (k == "PATH").then(|| OsString::from("/evil"))),
        None
    );
}

struct Fake {
    events: RefCell<Vec<String>>,
    exits: bool,
    code: Result<i32, UpdateError>,
    write_ok: bool,
    written: RefCell<Option<UpdateResult>>,
    timeout: RefCell<Option<Duration>>,
    args: RefCell<Vec<String>>,
}

impl Fake {
    fn new(exits: bool, code: Result<i32, UpdateError>) -> Fake {
        Fake {
            events: RefCell::new(Vec::new()),
            exits,
            code,
            write_ok: true,
            written: RefCell::new(None),
            timeout: RefCell::new(None),
            args: RefCell::new(Vec::new()),
        }
    }
    fn events(&self) -> Vec<String> {
        self.events.borrow().clone()
    }
}

impl HelperOps for Fake {
    fn wait_for_daemon_exit(&self, timeout: Duration) -> bool {
        self.events.borrow_mut().push("wait".into());
        *self.timeout.borrow_mut() = Some(timeout);
        self.exits
    }
    fn run_msiexec(&self, args: &[String]) -> Result<i32, UpdateError> {
        self.events.borrow_mut().push("msiexec".into());
        *self.args.borrow_mut() = args.to_vec();
        self.code.clone()
    }
    fn write_result(&self, r: &UpdateResult) -> Result<(), UpdateError> {
        self.events.borrow_mut().push("write".into());
        *self.written.borrow_mut() = Some(r.clone());
        if self.write_ok {
            Ok(())
        } else {
            Err(UpdateError::Io)
        }
    }
    fn start_daemon(&self, which: Daemon) {
        let name = match which {
            Daemon::New => "start-new",
            Daemon::Old => "start-old",
        };
        self.events.borrow_mut().push(name.into());
    }
}

const MSI: &str = "C:/Cache/sbarhive.msi";
const LOG: &str = "C:/Temp/sbarhive-update.log";

fn run(fake: &Fake) -> UpdateResult {
    run_helper(fake, Path::new(MSI), Path::new(LOG), "0.4.0", 77)
}

#[test]
fn ac7_helper_success_waits_installs_writes_and_starts_the_new_daemon() {
    let fake = Fake::new(true, Ok(0));
    let got = run(&fake);
    assert_eq!(fake.events(), ["wait", "msiexec", "write", "start-new"]);
    assert_eq!(
        *fake.args.borrow(),
        msiexec_args(Path::new(MSI), Path::new(LOG))
    );
    assert_eq!(*fake.timeout.borrow(), Some(DAEMON_EXIT_TIMEOUT));
    assert!(DAEMON_EXIT_TIMEOUT >= Duration::from_secs(10));
    assert!(DAEMON_EXIT_TIMEOUT <= Duration::from_secs(300));
    let want = UpdateResult {
        version: "0.4.0".to_string(),
        outcome: Outcome::Installed,
        at: 77,
    };
    assert_eq!(got, want);
    assert_eq!(fake.written.borrow().clone(), Some(want));
}

#[test]
fn ac7_helper_reboot_pending_counts_as_installed_and_starts_the_new_daemon() {
    let fake = Fake::new(true, Ok(3010));
    let got = run(&fake);
    assert_eq!(got.outcome, Outcome::Installed);
    assert_eq!(fake.events(), ["wait", "msiexec", "write", "start-new"]);
}

#[test]
fn ac7_helper_any_other_code_starts_the_old_daemon() {
    for (code, class) in [
        (1602, MsiResult::Cancelled),
        (1618, MsiResult::Busy),
        (1603, MsiResult::Failed),
        (1, MsiResult::Failed),
    ] {
        let fake = Fake::new(true, Ok(code));
        let got = run(&fake);
        assert_eq!(fake.events(), ["wait", "msiexec", "write", "start-old"]);
        assert_eq!(
            got,
            UpdateResult {
                version: "0.4.0".to_string(),
                outcome: Outcome::Failed(Failure::Msi(class)),
                at: 77,
            },
            "code {code}"
        );
        assert_eq!(fake.written.borrow().clone(), Some(got));
    }
}

#[test]
fn ac7_helper_a_daemon_that_never_exits_blocks_msiexec_and_starts_nothing() {
    let fake = Fake::new(false, Ok(0));
    let got = run(&fake);
    assert_eq!(fake.events(), ["wait", "write"]);
    assert_eq!(got.version, "0.4.0");
    assert_eq!(got.at, 77);
    assert!(matches!(got.outcome, Outcome::Failed(Failure::Text(ref t)) if !t.is_empty()));
    assert_eq!(fake.written.borrow().clone(), Some(got));
}

#[test]
fn ac7_helper_a_msiexec_that_cannot_start_is_a_text_failure_and_the_old_daemon_returns() {
    let fake = Fake::new(true, Err(UpdateError::Unavailable));
    let got = run(&fake);
    assert_eq!(fake.events(), ["wait", "msiexec", "write", "start-old"]);
    assert!(matches!(got.outcome, Outcome::Failed(Failure::Text(ref t)) if !t.is_empty()));
}

#[test]
fn ac7_helper_still_starts_the_daemon_when_the_result_cannot_be_written() {
    let mut fake = Fake::new(true, Ok(0));
    fake.write_ok = false;
    run(&fake);
    assert_eq!(fake.events(), ["wait", "msiexec", "write", "start-new"]);
    let mut fake = Fake::new(true, Ok(1603));
    fake.write_ok = false;
    run(&fake);
    assert_eq!(fake.events(), ["wait", "msiexec", "write", "start-old"]);
}

#[test]
fn ac7_windows_only_file_is_wired_and_excluded_from_mutants() {
    let apply = fs::read_to_string(root("src/update/apply.rs")).unwrap();
    assert!(
        apply.contains("#[cfg(windows)]") && apply.contains("mod windows;"),
        "apply.rs declares `#[cfg(windows)] mod windows;`"
    );
    let win = fs::read_to_string(root("src/update/apply/windows.rs"))
        .expect("crates/core/src/update/apply/windows.rs exists");
    assert!(win.contains("pub fn apply_msi"));
    assert!(win.contains("sbarhive.exe"));
    assert!(win.contains("helper_args") || win.contains("--apply-msi"));
    assert!(win.contains("msiexec_path") || win.contains("msiexec.exe"));
    assert!(win.contains("run_helper"));
    assert!(
        win.contains("DETACHED_PROCESS") || win.contains("CREATE_NEW_PROCESS_GROUP"),
        "the helper copy starts detached"
    );
    assert!(
        !win.contains("\"msiexec\""),
        "msiexec only by absolute path"
    );
    let cfg = fs::read_to_string(root("../../.cargo/mutants.toml")).unwrap();
    assert!(cfg.contains("\"crates/core/src/update/apply/windows.rs\""));
}
