//! S-067 (backlog/S-067-review-fixes-update-drop-notify.md): `kawaro update`
//! through the CLI module, with a fake network and a fake installer, as in
//! `s054c_update.rs`. No network, no pkexec.
//!
//! Contract (tester): `update::run` and `update::Facts` keep their S-054c
//! shape. With a `cache_dir` that cannot be created, a check (`--check`)
//! writes exactly one stderr line that names the cache folder (its
//! `Path::display()` text) and exits 5 (AC7).
//!
//! Regression guard the story asks to keep (passes before and after the
//! fix): `ac1_apply_with_a_repository_runs_pkexec_dnf_and_exits_0`. The bug
//! was in the real runner, which this fake replaces; the real runner's half
//! is pinned in `crates/core/tests/s067_update_core.rs` (pkexec_target,
//! SystemRunner) and `s067_sources.rs` (one SystemRunner).

#[allow(dead_code)]
#[path = "../src/update.rs"]
mod update;

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

use kawaro_core::update::UpdateError;
use kawaro_core::update::apply::{CommandRunner, Facts as SystemFacts, PKEXEC, PkgKind};
use kawaro_core::update::fetch::{FetchError, Fetcher};
use kawaro_core::update::policy::Platform;

/// The 17 bytes the S-054b manifest signs for every asset.
const GOOD: &[u8] = b"first-asset-bytes";

fn fx(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/update")
            .join(name),
    )
    .unwrap()
}

/// The good feed: the signed S-054b manifest (stable, 0.4.0) and its package.
struct FakeFetcher {
    calls: RefCell<usize>,
}

impl Fetcher for FakeFetcher {
    fn get(&self, url: &str, dest: &Path, _max_bytes: u64) -> Result<(), FetchError> {
        *self.calls.borrow_mut() += 1;
        let bytes = if url.ends_with(".minisig") {
            fx("s054b/manifest-stable-0.4.0.json.minisig")
        } else if url.ends_with(".json") {
            fx("s054b/manifest-stable-0.4.0.json")
        } else {
            GOOD.to_vec()
        };
        fs::write(dest, bytes).map_err(|_| FetchError::Offline)
    }
}

fn fetcher() -> FakeFetcher {
    FakeFetcher {
        calls: RefCell::new(0),
    }
}

/// Records every command and answers 0.
#[derive(Default)]
struct FakeRunner {
    calls: RefCell<Vec<Vec<String>>>,
}

impl CommandRunner for FakeRunner {
    fn run(&self, argv: &[String]) -> Result<i32, UpdateError> {
        self.calls.borrow_mut().push(argv.to_vec());
        Ok(0)
    }
}

fn facts<'a>(
    root: &Path,
    installed: &str,
    system: SystemFacts,
    cache_dir: PathBuf,
    runner: &'a FakeRunner,
) -> update::Facts<'a> {
    update::Facts {
        system,
        installed: installed.to_string(),
        keys: vec![String::from_utf8(fx("s054b/key-d.pub")).unwrap()],
        platform: Platform {
            os: "linux".into(),
            arch: "x86_64".into(),
            pkg: "rpm".into(),
        },
        config_dir: root.join("config"),
        cache_dir,
        exe: PathBuf::from("/usr/bin/kawaro"),
        runner,
    }
}

fn fedora(repo_configured: bool) -> SystemFacts {
    SystemFacts {
        has_dnf: true,
        has_apt: false,
        has_pkexec: true,
        repo_configured,
        installed_kind: Some(PkgKind::Rpm),
    }
}

struct Run {
    code: i32,
    out: String,
    err: String,
}

fn run(args: &[&str], fetcher: &FakeFetcher, facts: &update::Facts) -> Run {
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = update::run(&args, fetcher, facts, &mut out, &mut err);
    Run {
        code,
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

fn one_line(text: &str) -> bool {
    text.ends_with('\n') && text.matches('\n').count() == 1 && text.trim().len() > 1
}

// ---------------------------------------------------------------- AC1

#[test]
fn ac1_apply_with_a_repository_runs_pkexec_dnf_and_exits_0() {
    let tmp = tempfile::tempdir().unwrap();
    let runner = FakeRunner::default();
    let f = facts(
        tmp.path(),
        "0.3.3",
        fedora(true),
        tmp.path().join("cache"),
        &runner,
    );
    let r = run(&["--apply"], &fetcher(), &f);
    assert_eq!(r.code, 0, "stdout {:?} stderr {:?}", r.out, r.err);
    let calls = runner.calls.borrow();
    assert_eq!(calls.len(), 1, "one install command");
    assert_eq!(
        calls[0],
        vec![PKEXEC, "dnf", "upgrade", "-y", "kawaro"],
        "the repository upgrade goes through pkexec dnf"
    );
    assert!(r.out.ends_with("installed 0.4.0\n"), "{:?}", r.out);
}

// ---------------------------------------------------------------- AC3

fn finished(cache: &Path, version: &str) {
    let dir = cache.join("updates").join(version);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(format!("kawaro-{version}-1.x86_64.rpm")), b"x").unwrap();
}

#[test]
fn ac3_status_prints_the_highest_ready_version_in_either_order() {
    for order in [["0.5.0", "0.6.0"], ["0.6.0", "0.5.0"]] {
        let tmp = tempfile::tempdir().unwrap();
        let cache = tmp.path().join("cache");
        for version in order {
            finished(&cache, version);
        }
        let runner = FakeRunner::default();
        let f = facts(tmp.path(), "0.3.3", fedora(false), cache, &runner);
        let fx = fetcher();
        let r = run(&["--status"], &fx, &f);
        assert_eq!(r.out, "status ready 0.6.0\n", "order {order:?}");
        assert_eq!(r.code, 0);
        assert_eq!(*fx.calls.borrow(), 0, "--status never fetches");
    }
    // More folders, so a first-found answer cannot pass by luck.
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().join("cache");
    for v in [
        "0.4.1", "0.6.0", "0.5.0", "0.5.3", "0.4.9", "0.5.9", "0.5.1",
    ] {
        finished(&cache, v);
    }
    let runner = FakeRunner::default();
    let f = facts(tmp.path(), "0.3.3", fedora(false), cache, &runner);
    assert_eq!(
        run(&["--status"], &fetcher(), &f).out,
        "status ready 0.6.0\n"
    );
}

// ---------------------------------------------------------------- AC7

#[test]
fn ac7_a_cache_folder_that_cannot_be_created_gives_one_line_and_exit_5() {
    let tmp = tempfile::tempdir().unwrap();
    // A regular file where a folder must go: create_dir_all fails.
    let blocker = tmp.path().join("blocker");
    fs::write(&blocker, b"not a folder").unwrap();
    let cache = blocker.join("cache");
    let runner = FakeRunner::default();
    let f = facts(tmp.path(), "0.3.3", fedora(false), cache.clone(), &runner);
    let r = run(&["--check"], &fetcher(), &f);
    assert_eq!(r.code, 5, "stdout {:?} stderr {:?}", r.out, r.err);
    assert!(one_line(&r.err), "one stderr line, got {:?}", r.err);
    assert!(
        r.err.contains(&cache.display().to_string()),
        "the line names the cache folder {}: {:?}",
        cache.display(),
        r.err
    );
    assert!(runner.calls.borrow().is_empty(), "nothing is installed");
}
