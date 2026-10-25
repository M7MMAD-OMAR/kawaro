//! S-054c (backlog/S-054c-update-cli.md): the `kawaro update` command.
//!
//! The CLI is a binary crate, so this test target compiles the module file
//! itself: `crates/cli/src/update.rs` must only use `kawaro_core`, `clap` and
//! `std` (no `crate::` paths), so it builds both inside the binary and here.
//!
//! The contract pinned by these tests (tester decisions where the story is
//! silent are marked DECISION):
//!
//! - `update::run(args, &dyn Fetcher, &Facts, out, err) -> i32`, exactly the
//!   AC1 signature. `args` are the words after `update` (`&[String]`), `out`
//!   and `err` take `&mut Vec<u8>` (a `&mut dyn Write` or a generic `Write`).
//! - DECISION: `update::Facts<'a>` is the CLI's view of the machine, with
//!   public fields: `system` (the `apply::Facts` that pick the install plan),
//!   `installed` (the running version), `keys` (trusted minisign keys),
//!   `platform`, `config_dir` (holds `config.toml`, `update-state.json`,
//!   `update-result.json`), `cache_dir` (packages go to
//!   `<cache_dir>/updates/<version>/<name>`, S-053b AC7), `exe` (the path given
//!   to the privileged step) and `runner` (the `CommandRunner` that
//!   `apply_now` uses; the fake installer). The story's test table needs the
//!   keys and the installer injected, and the five argument signature has no
//!   other place for them.
//! - DECISION: when no manifest was verified (`offline`, `unavailable`,
//!   `rejected`) the second line is `latest unknown`.
//! - DECISION: the `--apply` failure line goes to stderr as one line.
//! - No network: every download goes through the fake `Fetcher` below.

#[allow(dead_code)]
#[path = "../src/update.rs"]
mod update;

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

use kawaro_core::update::UpdateError;
use kawaro_core::update::apply::{CommandRunner, Facts as SystemFacts, PkgKind, privileged_argv};
use kawaro_core::update::fetch::{FetchError, Fetcher};
use kawaro_core::update::policy::Platform;
use kawaro_core::update::result::{Failure, Outcome, UpdateResult, write_result};

/// The 17 bytes the S-054b manifest signs for every asset.
const GOOD: &[u8] = b"first-asset-bytes";
/// Same length, other bytes: passes the size check, fails the hash.
const TAMPERED: &[u8] = b"first-asset-bytez";
const SHA: &str = "bff5b3ca49e18ea61c9f21a39f95922bcf75f2e0f8ffedf46e07c7cfd70d5d64";
const RPM: &str = "kawaro-0.4.0-1.x86_64.rpm";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/update")
}

fn fx(name: &str) -> Vec<u8> {
    fs::read(fixtures().join(name)).unwrap()
}

/// The key that signed `s054b/manifest-stable-0.4.0.json`.
fn key_d() -> String {
    String::from_utf8(fx("s054b/key-d.pub")).unwrap()
}

/// A key that did not sign it: the manifest is rejected.
fn key_a() -> String {
    String::from_utf8(fx("key-a.pub")).unwrap()
}

#[derive(Clone, Copy)]
enum Feed {
    Good,
    Tampered,
    Offline,
    Unavailable,
}

/// The fake network: the S-054b signed manifest (stable, 0.4.0) for any
/// `.json` url, its signature for `.minisig`, the package bytes otherwise.
struct FakeFetcher {
    feed: Feed,
    urls: RefCell<Vec<String>>,
}

impl FakeFetcher {
    fn new(feed: Feed) -> FakeFetcher {
        FakeFetcher {
            feed,
            urls: RefCell::new(Vec::new()),
        }
    }

    fn calls(&self) -> usize {
        self.urls.borrow().len()
    }
}

impl Fetcher for FakeFetcher {
    fn get(&self, url: &str, dest: &Path, _max_bytes: u64) -> Result<(), FetchError> {
        self.urls.borrow_mut().push(url.to_string());
        let bytes = match self.feed {
            Feed::Offline => return Err(FetchError::Offline),
            Feed::Unavailable => return Err(FetchError::Unavailable),
            _ if url.ends_with(".minisig") => fx("s054b/manifest-stable-0.4.0.json.minisig"),
            _ if url.ends_with(".json") => fx("s054b/manifest-stable-0.4.0.json"),
            Feed::Good => GOOD.to_vec(),
            Feed::Tampered => TAMPERED.to_vec(),
        };
        fs::write(dest, bytes).unwrap();
        Ok(())
    }
}

/// The fake installer: records every command and answers `code`.
struct FakeRunner {
    code: i32,
    calls: RefCell<Vec<Vec<String>>>,
}

impl FakeRunner {
    fn new(code: i32) -> FakeRunner {
        FakeRunner {
            code,
            calls: RefCell::new(Vec::new()),
        }
    }

    fn count(&self) -> usize {
        self.calls.borrow().len()
    }
}

impl CommandRunner for FakeRunner {
    fn run(&self, argv: &[String]) -> Result<i32, UpdateError> {
        self.calls.borrow_mut().push(argv.to_vec());
        Ok(self.code)
    }
}

/// A Fedora like machine with pkexec and no repository: the plan is
/// `Local(Rpm)`, so `apply_now` builds the privileged command.
fn system() -> SystemFacts {
    SystemFacts {
        has_dnf: true,
        has_apt: false,
        has_pkexec: true,
        repo_configured: false,
        installed_kind: Some(PkgKind::Rpm),
    }
}

fn facts<'a>(
    root: &Path,
    installed: &str,
    key: String,
    runner: &'a FakeRunner,
) -> update::Facts<'a> {
    update::Facts {
        system: system(),
        installed: installed.to_string(),
        keys: vec![key],
        platform: Platform {
            os: "linux".into(),
            arch: "x86_64".into(),
            pkg: "rpm".into(),
        },
        config_dir: root.join("config"),
        cache_dir: root.join("cache"),
        exe: PathBuf::from("/usr/bin/kawaro"),
        runner,
    }
}

fn write_config(root: &Path, text: &str) {
    let dir = root.join("config");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("config.toml"), text).unwrap();
}

struct Run {
    code: i32,
    out: String,
    err: String,
}

fn run(args: &[&str], fetcher: &FakeFetcher, facts: &update::Facts) -> Run {
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = update::run(&args, fetcher, facts, &mut out, &mut err);
    Run {
        code,
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

/// Exactly one non empty line ending in a newline.
fn one_line(text: &str) -> bool {
    text.ends_with('\n') && text.matches('\n').count() == 1 && text.trim().len() > 1
}

fn package_path(root: &Path) -> PathBuf {
    root.join("cache").join("updates").join("0.4.0").join(RPM)
}

// ------------------------------------------------------------ AC1, AC2

#[test]
fn ac1_ac2_check_prints_three_lines_and_the_exit_code_for_each_of_the_five_outcomes() {
    // (feed, installed, key, expected stdout, expected code, name)
    let cases: [(Feed, &str, String, &str, i32, &str); 5] = [
        (
            Feed::Good,
            "0.4.0",
            key_d(),
            "current 0.4.0\nlatest 0.4.0\nstatus up-to-date\n",
            0,
            "up-to-date",
        ),
        (
            Feed::Good,
            "0.3.3",
            key_d(),
            "current 0.3.3\nlatest 0.4.0\nstatus update-available\n",
            10,
            "update-available",
        ),
        (
            Feed::Offline,
            "0.3.3",
            key_d(),
            "current 0.3.3\nlatest unknown\nstatus offline\n",
            3,
            "offline",
        ),
        (
            Feed::Unavailable,
            "0.3.3",
            key_d(),
            "current 0.3.3\nlatest unknown\nstatus unavailable\n",
            4,
            "unavailable",
        ),
        (
            Feed::Good,
            "0.3.3",
            key_a(),
            "current 0.3.3\nlatest unknown\nstatus rejected\n",
            5,
            "rejected",
        ),
    ];
    for (feed, installed, key, stdout, code, name) in cases {
        let tmp = tempfile::tempdir().unwrap();
        let runner = FakeRunner::new(0);
        let f = facts(tmp.path(), installed, key, &runner);
        let fetcher = FakeFetcher::new(feed);
        let r = run(&["--check"], &fetcher, &f);
        assert_eq!(r.out, stdout, "{name}: stdout byte for byte");
        assert_eq!(r.code, code, "{name}: exit code; stderr {:?}", r.err);
        assert!(fetcher.calls() >= 1, "{name}: --check fetched the manifest");
        assert_eq!(runner.count(), 0, "{name}: --check never installs");
        assert!(
            !package_path(tmp.path()).exists(),
            "{name}: --check downloads no package"
        );
        if name == "rejected" {
            assert!(
                one_line(&r.err),
                "rejected: one error line on stderr, got {:?}",
                r.err
            );
        } else {
            assert_eq!(r.err, "", "{name}: nothing on stderr");
        }
    }

    // A manual check runs even when `update_check = false`.
    let tmp = tempfile::tempdir().unwrap();
    write_config(tmp.path(), "update_check = false\n");
    let runner = FakeRunner::new(0);
    let f = facts(tmp.path(), "0.4.0", key_d(), &runner);
    let fetcher = FakeFetcher::new(Feed::Good);
    let r = run(&["--check"], &fetcher, &f);
    assert_eq!(r.out, "current 0.4.0\nlatest 0.4.0\nstatus up-to-date\n");
    assert_eq!(r.code, 0);
    assert!(fetcher.calls() >= 2, "manifest and signature were fetched");

    // A usage error exits 2.
    let tmp = tempfile::tempdir().unwrap();
    let runner = FakeRunner::new(0);
    let f = facts(tmp.path(), "0.4.0", key_d(), &runner);
    let fetcher = FakeFetcher::new(Feed::Good);
    let r = run(&["--no-such-flag"], &fetcher, &f);
    assert_eq!(r.code, 2, "usage error");
    assert_eq!(fetcher.calls(), 0, "a usage error fetches nothing");
}

// ------------------------------------------------------------------ AC3

#[test]
fn ac3_download_verify_and_apply_with_good_and_tampered_packages() {
    let three = "current 0.3.3\nlatest 0.4.0\nstatus update-available\n";
    let five = format!("{three}downloaded {RPM}\nverified sha256 {}\n", &SHA[..12]);

    // No flag, good package: check, download, verify, exit 10, no install.
    let tmp = tempfile::tempdir().unwrap();
    let runner = FakeRunner::new(0);
    let f = facts(tmp.path(), "0.3.3", key_d(), &runner);
    let fetcher = FakeFetcher::new(Feed::Good);
    let r = run(&[], &fetcher, &f);
    assert_eq!(r.out, five, "no flag: stdout byte for byte");
    assert_eq!(r.code, 10, "no flag never installs; stderr {:?}", r.err);
    assert_eq!(r.err, "");
    assert_eq!(
        runner.count(),
        0,
        "the installer has 0 calls without --apply"
    );
    assert_eq!(
        fs::read(package_path(tmp.path())).unwrap(),
        GOOD,
        "the verified package is kept"
    );

    // --apply, good package: installs once through apply_now, exit 0.
    let tmp = tempfile::tempdir().unwrap();
    let runner = FakeRunner::new(0);
    let f = facts(tmp.path(), "0.3.3", key_d(), &runner);
    let fetcher = FakeFetcher::new(Feed::Good);
    let r = run(&["--apply"], &fetcher, &f);
    assert_eq!(
        r.out,
        format!("{five}installed 0.4.0\n"),
        "--apply: stdout byte for byte"
    );
    assert_eq!(r.code, 0, "--apply finished; stderr {:?}", r.err);
    assert_eq!(r.err, "");
    assert_eq!(runner.count(), 1, "the installer has 1 call with --apply");
    let expected = privileged_argv(
        Path::new("/usr/bin/kawaro"),
        &package_path(tmp.path()),
        SHA,
        PkgKind::Rpm,
    );
    assert_eq!(
        runner.calls.borrow()[0],
        expected,
        "apply_now ran the Local(Rpm) plan"
    );

    // --apply when the installer fails: the failure line and exit 1.
    let tmp = tempfile::tempdir().unwrap();
    let runner = FakeRunner::new(1);
    let f = facts(tmp.path(), "0.3.3", key_d(), &runner);
    let fetcher = FakeFetcher::new(Feed::Good);
    let r = run(&["--apply"], &fetcher, &f);
    assert_eq!(r.code, 1, "a failed install exits 1");
    assert_eq!(runner.count(), 1);
    assert_eq!(r.out, five, "no installed line after a failure");
    assert!(
        one_line(&r.err),
        "one failure line on stderr, got {:?}",
        r.err
    );

    // Tampered package, with and without --apply: exit 5, 0 installs, no file.
    for args in [&[][..], &["--apply"][..]] {
        let tmp = tempfile::tempdir().unwrap();
        let runner = FakeRunner::new(0);
        let f = facts(tmp.path(), "0.3.3", key_d(), &runner);
        let fetcher = FakeFetcher::new(Feed::Tampered);
        let r = run(args, &fetcher, &f);
        assert_eq!(
            r.code, 5,
            "{args:?}: tampered is rejected; stdout {:?}",
            r.out
        );
        assert_eq!(runner.count(), 0, "{args:?}: the installer has 0 calls");
        assert!(
            r.out.starts_with(three),
            "{args:?}: the check lines come first"
        );
        assert!(
            !r.out.contains("verified"),
            "{args:?}: nothing is called verified"
        );
        assert!(
            !r.out.contains("installed"),
            "{args:?}: nothing is installed"
        );
        assert!(
            one_line(&r.err),
            "{args:?}: one error line, got {:?}",
            r.err
        );
        assert!(
            !package_path(tmp.path()).exists(),
            "{args:?}: no package file remains"
        );
    }
}

// ------------------------------------------------------------------ AC4

#[test]
fn ac4_status_reads_the_state_files_with_no_fetch() {
    // Empty config dir: `status none`.
    let tmp = tempfile::tempdir().unwrap();
    let runner = FakeRunner::new(0);
    let f = facts(tmp.path(), "0.3.3", key_d(), &runner);
    let fetcher = FakeFetcher::new(Feed::Good);
    let r = run(&["--status"], &fetcher, &f);
    assert_eq!(r.out, "status none\n");
    assert_eq!(r.code, 0);
    assert_eq!(fetcher.calls(), 0, "--status never fetches");

    // After `kawaro update` verified 0.4.0: `status ready 0.4.0`.
    let tmp = tempfile::tempdir().unwrap();
    let runner = FakeRunner::new(0);
    let f = facts(tmp.path(), "0.3.3", key_d(), &runner);
    let fetcher = FakeFetcher::new(Feed::Good);
    assert_eq!(run(&[], &fetcher, &f).code, 10, "the download step ran");
    let offline = FakeFetcher::new(Feed::Offline);
    let r = run(&["--status"], &offline, &f);
    assert_eq!(r.out, "status ready 0.4.0\n");
    assert_eq!(r.code, 0);
    assert_eq!(offline.calls(), 0, "--status never fetches");
    assert_eq!(runner.count(), 0);

    // A failed install recorded in update-result.json: `status failed 0.4.0`.
    let tmp = tempfile::tempdir().unwrap();
    let runner = FakeRunner::new(0);
    let f = facts(tmp.path(), "0.3.3", key_d(), &runner);
    write_result(
        &tmp.path().join("config"),
        &UpdateResult {
            version: "0.4.0".into(),
            outcome: Outcome::Failed(Failure::Text("dnf failed".into())),
            at: 1_761_400_000,
        },
    )
    .unwrap();
    let fetcher = FakeFetcher::new(Feed::Good);
    let r = run(&["--status"], &fetcher, &f);
    assert_eq!(r.out, "status failed 0.4.0\n");
    assert_eq!(r.code, 0);
    assert_eq!(fetcher.calls(), 0, "--status never fetches");
}

// ------------------------------------------------------------------ AC5

#[test]
fn ac5_help_channel_override_unknown_channel_and_arabic_words() {
    // --help lists the four options in English.
    let tmp = tempfile::tempdir().unwrap();
    let runner = FakeRunner::new(0);
    let f = facts(tmp.path(), "0.4.0", key_d(), &runner);
    let fetcher = FakeFetcher::new(Feed::Good);
    let r = run(&["--help"], &fetcher, &f);
    assert_eq!(r.code, 0, "--help exits 0");
    for word in [
        "--check",
        "--apply",
        "--status",
        "--channel",
        "stable",
        "beta",
    ] {
        assert!(r.out.contains(word), "--help lists {word}: {}", r.out);
    }
    assert_eq!(fetcher.calls(), 0, "--help fetches nothing");

    // An unknown channel is a usage error.
    let r = run(&["--check", "--channel", "nightly"], &fetcher, &f);
    assert_eq!(r.code, 2, "unknown channel exits 2");
    assert!(r.err.contains("unknown channel"), "stderr: {:?}", r.err);
    assert_eq!(fetcher.calls(), 0, "nothing is fetched for a bad channel");

    // --channel overrides the config for this run, both ways.
    let tmp = tempfile::tempdir().unwrap();
    write_config(tmp.path(), "update_channel = \"beta\"\n");
    let runner = FakeRunner::new(0);
    let f = facts(tmp.path(), "0.4.0", key_d(), &runner);
    let fetcher = FakeFetcher::new(Feed::Good);
    let r = run(&["--check", "--channel", "stable"], &fetcher, &f);
    assert!(
        fetcher.urls.borrow()[0].ends_with("/stable.json"),
        "{:?}",
        fetcher.urls.borrow()
    );
    assert_eq!(r.out, "current 0.4.0\nlatest 0.4.0\nstatus up-to-date\n");
    assert_eq!(r.code, 0);

    let tmp = tempfile::tempdir().unwrap();
    write_config(tmp.path(), "update_channel = \"stable\"\n");
    let f = facts(tmp.path(), "0.4.0", key_d(), &runner);
    let fetcher = FakeFetcher::new(Feed::Good);
    let r = run(&["--check", "--channel", "beta"], &fetcher, &f);
    assert!(
        fetcher.urls.borrow()[0].ends_with("/beta.json"),
        "{:?}",
        fetcher.urls.borrow()
    );
    // The fake serves the stable manifest, so the beta run rejects it.
    assert_eq!(
        r.code, 5,
        "stable manifest on the beta channel: {:?}",
        r.out
    );

    // With `language = "ar"` the status words are Arabic.
    let tmp = tempfile::tempdir().unwrap();
    write_config(tmp.path(), "language = \"ar\"\n");
    let f = facts(tmp.path(), "0.4.0", key_d(), &runner);
    let fetcher = FakeFetcher::new(Feed::Good);
    let r = run(&["--check"], &fetcher, &f);
    assert_eq!(r.code, 0, "exit codes do not depend on the language");
    assert_eq!(r.out.lines().count(), 3, "still three lines: {:?}", r.out);
    let status = r.out.lines().nth(2).unwrap();
    assert!(
        status
            .chars()
            .any(|c| ('\u{0600}'..='\u{06FF}').contains(&c)),
        "Arabic status line: {status:?}"
    );
    assert!(
        !r.out.contains("up-to-date"),
        "no English status word: {:?}",
        r.out
    );
    assert!(
        r.out.contains("0.4.0"),
        "versions stay as they are: {:?}",
        r.out
    );
}
