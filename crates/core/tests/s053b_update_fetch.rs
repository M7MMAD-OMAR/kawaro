//! S-053b (backlog/S-053b-update-fetch-verify.md, docs/adr/0006-auto-update.md):
//! the fetcher, the check flow, the verified download and the cache folder.
//! Written by the tester from the acceptance criteria. No test touches the
//! network: a fake `Fetcher` serves bytes from memory and a fake `curl` is a
//! shell script in a temp dir.
//!
//! Contract (the implementer follows it; the tests are not edited). Where the
//! story text leaves a gap, the choice is marked DECISION and is reported to
//! the manager.
//!
//! `update::fetch` (`crates/core/src/update/fetch.rs`), all `pub`:
//! - `trait Fetcher { fn get(&self, url: &str, dest: &Path, max_bytes: u64) -> Result<(), FetchError>; }`
//!   object safe, no `Sync` bound.
//! - `FetchError` (`Debug + Clone + PartialEq + Eq`): `Offline`, `Http(u16)`,
//!   `TooLarge`, `Unavailable`, and DECISION `BadUrl` (AC9 says `BadUrl` but
//!   `get` returns `FetchError`; `check` and `download_verified` report the
//!   same refusal as `UpdateError::BadUrl`).
//! - `curl_args(url: &str, dest: &Path, max_bytes: u64) -> Vec<String>`: exactly
//!   `--silent --show-error --fail --location --proto =https --proto-redir =https
//!   --max-redirs 5 --max-time <T> --max-filesize <max_bytes> --user-agent
//!   sbarhive-update --output <dest> <url>`, each flag and value its own element.
//!   DECISION: `<T>` is 30 when the url path ends in `.json` or `.minisig` (the
//!   manifest and its signature), otherwise 1800 (a package); the signature has
//!   no room for a flag.
//! - `CurlFetcher` with `CurlFetcher::with_program(impl Into<PathBuf>)` and
//!   `CurlFetcher::system()`, `impl Fetcher`. An empty or missing program is
//!   `Unavailable`. Exit 6, 7 and 28 are `Offline` (DECISION for 7 and 28, the
//!   story names only 6), 63 is `TooLarge`, 22 is `Http(n)` where `n` is the
//!   number in curl's stderr text `The requested URL returned error: <n>`
//!   (DECISION; `Http(_)` when there is none). Exit 0 is `Ok`.
//! - `curl_candidates(os: paths::Os, env: EnvFn) -> Vec<PathBuf>`: Linux
//!   `/usr/bin/curl` then `/bin/curl`; Windows `<SystemRoot>/System32/curl.exe`
//!   (empty when `SystemRoot` is unset). It never reads `PATH`. `system()` uses it.
//! - `get` refuses a url with `..`, `%2e`, `%2E`, `\`, whitespace or a control
//!   character with `FetchError::BadUrl` before running anything.
//! - `pub const MANIFEST_MAX_BYTES: u64` and `SIGNATURE_MAX_BYTES: u64` (each
//!   between 1 KiB and 1 MiB) are the caps `check` passes for the two fetches.
//!
//! `update::flow` (`crates/core/src/update/flow.rs`):
//! - `UpdateError` (in `update/mod.rs`) gains `Offline`, `Unavailable`,
//!   `Http(u16)`, `HashMismatch`, `SizeMismatch`, `NoAsset`, `Io` (DECISION: the
//!   story names `Offline`, `HashMismatch`, `SizeMismatch`; the rest are needed
//!   to report fetch and file problems). A fetcher `TooLarge` during a package
//!   download is `SizeMismatch`.
//! - `Update { version: String, notes_url: String, asset: manifest::Asset }`
//!   (`Debug + Clone + PartialEq`), a new type in `flow`; `policy::Update` stays
//!   as it is (DECISION: `download_verified` needs name, url, size, sha256).
//! - `CheckState { schedule: schedule::State, last_seen: Option<String> }`
//!   (`Debug + Clone + PartialEq + Default`). DECISION: `last_seen` cannot be a
//!   field of `schedule::State`, because `s053a_update.rs` builds that struct
//!   with literals; AC3 says `&mut State`, the flow takes `&mut CheckState`.
//! - `CheckOutcome` (`Debug + PartialEq`): `Disabled`, `NotDue`, `UpToDate`,
//!   `Available(Update)`, `Offline`, `Unavailable`, `Rejected(UpdateError)`.
//! - `CheckEnv<'a> { installed: &'a str, keys: &'a [&'a str], platform:
//!   policy::Platform, interval_secs: u64, scratch: &'a Path }`, plain pub fields.
//! - `check_with(&CheckEnv, &Config, &mut CheckState, &dyn Fetcher, now: u64,
//!   manual: bool) -> CheckOutcome` is the flow. `check(&Config, &mut CheckState,
//!   &dyn Fetcher, now, manual)` calls it with the running version, `UPDATE_KEYS`,
//!   the running platform, 24 h and a scratch folder of its own (DECISION: the
//!   story's `check` has no way to inject keys or the installed version).
//!   `scratch` is where the two fetches land; it is empty again afterwards.
//! - Order: disabled (automatic only), not due (automatic only), fetch
//!   `<base>/<channel>.json`, fetch `<base>/<channel>.json.minisig`, verify the
//!   signature, parse, `policy::evaluate`, every asset url checked for the AC9
//!   characters (`BadUrl`), `select_asset` (none: `Rejected(NoAsset)`).
//!   `<base>` is `https://github.com/M7MMAD-OMAR/sbarhive/releases/download/updates`.
//! - State (DECISION where the story is silent, marked *): a verified manifest
//!   (Available, UpToDate, downgrade): `last_check = now`, `failures = 0`,
//!   `last_seen = Some(manifest version)`. `Offline` and `Unavailable`
//!   (ADR 9: no curl counts as a failure): `last_check` and `last_seen`
//!   unchanged, `failures + 1`, `last_failure = Some(now)`. `Rejected`:
//!   `failures + 1`, `last_failure = Some(now)`* (the backoff of `due` needs it),
//!   `last_check` and `last_seen` unchanged*. `Disabled` and `NotDue`: untouched.
//! - `download_verified(&Update, dir: &Path, &dyn Fetcher) -> Result<VerifiedPackage, UpdateError>`
//!   creates `dir` when missing, fetches to `<dir>/<asset.name>.part` with
//!   `max_bytes = asset.size`, checks the length (`SizeMismatch`) then the SHA-256
//!   (`HashMismatch`), renames to `<dir>/<asset.name>`. Any failure removes the
//!   `.part`. A url with the AC9 characters is `BadUrl` before any fetch.
//!   `Offline`, `Unavailable`, `Http(n)` map to the same `UpdateError`. After
//!   success every other directory in `dir.parent()` is deleted; after a failure
//!   nothing is deleted. `VerifiedPackage::path(&self) -> &Path` (`Debug`).
//!
//! `paths` (`crates/core/src/paths.rs`), pure like `config_dir_with`:
//! - `cache_dir_with(os, env, known) -> Option<PathBuf>`: Linux
//!   `XDG_CACHE_HOME/sbarhive`, else `HOME/.cache/sbarhive`; Windows
//!   `%LOCALAPPDATA%/SbarHive` (known folder when unset or empty).
//! - `update_dir_with(os, env, known, version: &str) -> Option<PathBuf>`:
//!   `<cache>/updates/<version>` (DECISION: the name).
//! - `update_state_file_with(os, env, known) -> Option<PathBuf>`:
//!   `<config_dir>/update-state.json` (DECISION: the name).
//!
//! macOS has no `Os` variant yet, so it is not covered.
//!
//! `VerifiedPackage` having no public constructor cannot be seen at run time;
//! `ac5_verified_package_has_no_public_constructor` reads the source.
#![cfg(unix)]

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use sbarhive_core::config::Config;
use sbarhive_core::paths::{
    Os, cache_dir_with, config_dir_with, update_dir_with, update_state_file_with,
};
use sbarhive_core::update::fetch::{
    CurlFetcher, FetchError, Fetcher, MANIFEST_MAX_BYTES, SIGNATURE_MAX_BYTES, curl_args,
    curl_candidates,
};
use sbarhive_core::update::flow::{
    CheckEnv, CheckOutcome, CheckState, Update, check, check_with, download_verified,
};
use sbarhive_core::update::manifest::{Asset, Manifest};
use sbarhive_core::update::policy::{Platform, select_asset};
use sbarhive_core::update::schedule::State;
use sbarhive_core::update::{Channel, UpdateError};

const DAY: u64 = 86_400;
const BASE: &str = "https://github.com/M7MMAD-OMAR/sbarhive/releases/download/updates";
const REL: &str = "https://github.com/M7MMAD-OMAR/sbarhive/releases/download/v0.4.0";
const FIRST: &[u8] = b"first-asset-bytes";
const SECOND: &[u8] = b"second-asset-bytes";
const SHA_FIRST: &str = "bff5b3ca49e18ea61c9f21a39f95922bcf75f2e0f8ffedf46e07c7cfd70d5d64";
const SHA_SECOND: &str = "7c629aec8d8e564b91e9853e0bf446bcbc83fb9182e6046edac086df63fec646";
/// SHA-256 of `big()`: 200000 bytes, byte i is `i % 251`.
const SHA_BIG: &str = "e24bc62381f1224fbbb74688663f8f9743b9680b193edd666835e97b06e730eb";

fn big() -> Vec<u8> {
    (0..200_000usize).map(|i| (i % 251) as u8).collect()
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/update")
}

fn fx(rel: &str) -> Vec<u8> {
    let p = fixtures().join(rel);
    fs::read(&p).unwrap_or_else(|e| panic!("fixture {} must exist: {e}", p.display()))
}

fn fx_text(rel: &str) -> String {
    String::from_utf8(fx(rel)).unwrap()
}

// ------------------------------------------------------------ fake fetcher

#[derive(Clone, Debug, PartialEq)]
struct Call {
    url: String,
    dest: PathBuf,
    max_bytes: u64,
}

/// What the fake does for one url: bytes written to `dest` first (may be
/// empty), then the result.
type Served = (Vec<u8>, Result<(), FetchError>);

struct Fake {
    calls: RefCell<Vec<Call>>,
    serve: Box<dyn Fn(&str) -> Served>,
}

impl Fake {
    fn new(serve: impl Fn(&str) -> Served + 'static) -> Fake {
        Fake {
            calls: RefCell::new(Vec::new()),
            serve: Box::new(serve),
        }
    }
    /// Serves a manifest and its signature for any channel.
    fn feed(manifest: Vec<u8>, minisig: Vec<u8>) -> Fake {
        Fake::new(move |url| {
            if url.ends_with(".minisig") {
                (minisig.clone(), Ok(()))
            } else {
                (manifest.clone(), Ok(()))
            }
        })
    }
    fn failing(e: FetchError) -> Fake {
        Fake::new(move |_| (Vec::new(), Err(e.clone())))
    }
    fn bytes(b: Vec<u8>) -> Fake {
        Fake::new(move |_| (b.clone(), Ok(())))
    }
    fn calls(&self) -> Vec<Call> {
        self.calls.borrow().clone()
    }
    fn urls(&self) -> Vec<String> {
        self.calls().into_iter().map(|c| c.url).collect()
    }
}

impl Fetcher for Fake {
    fn get(&self, url: &str, dest: &Path, max_bytes: u64) -> Result<(), FetchError> {
        self.calls.borrow_mut().push(Call {
            url: url.to_string(),
            dest: dest.to_path_buf(),
            max_bytes,
        });
        let (bytes, result) = (self.serve)(url);
        if !bytes.is_empty() {
            fs::write(dest, bytes).unwrap();
        }
        result
    }
}

// ------------------------------------------------------------ helpers

fn linux_rpm() -> Platform {
    Platform {
        os: "linux".into(),
        arch: "x86_64".into(),
        pkg: "rpm".into(),
    }
}

fn cfg() -> Config {
    Config::default()
}

fn listing(dir: &Path) -> BTreeSet<String> {
    fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect()
}

fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|s| s.to_string()).collect()
}

fn state(
    last_check: Option<u64>,
    last_failure: Option<u64>,
    failures: u32,
    last_seen: Option<&str>,
) -> CheckState {
    CheckState {
        schedule: State {
            last_check,
            last_failure,
            failures,
        },
        last_seen: last_seen.map(String::from),
    }
}

/// Runs `check_with` for the Linux rpm platform and a one second sooner clock
/// than `DAY`; returns the outcome, the state and the calls.
struct Run {
    outcome: CheckOutcome,
    state: CheckState,
    calls: Vec<Call>,
    scratch_left: BTreeSet<String>,
}

fn run(
    fake: &Fake,
    key_file: &str,
    installed: &str,
    mut st: CheckState,
    config: &Config,
    now: u64,
    manual: bool,
) -> Run {
    let scratch = tempfile::tempdir().unwrap();
    let key = fx_text(key_file);
    let keys = [key.as_str()];
    let env = CheckEnv {
        installed,
        keys: &keys,
        platform: linux_rpm(),
        interval_secs: DAY,
        scratch: scratch.path(),
    };
    let outcome = check_with(&env, config, &mut st, fake, now, manual);
    Run {
        outcome,
        state: st,
        calls: fake.calls(),
        scratch_left: listing(scratch.path()),
    }
}

fn good_feed() -> Fake {
    Fake::feed(
        fx("manifest-stable-0.4.0.json"),
        fx("manifest-stable-0.4.0.json.minisig"),
    )
}

fn feed_c(name: &str) -> Fake {
    Fake::feed(
        fx(&format!("s053b/{name}.json")),
        fx(&format!("s053b/{name}.json.minisig")),
    )
}

fn run_c(name: &str) -> Run {
    run(
        &feed_c(name),
        "s053b/key-c.pub",
        "0.3.3",
        CheckState::default(),
        &cfg(),
        10_000,
        true,
    )
}

// ------------------------------------------------------------ fake curl

static LOCK: Mutex<()> = Mutex::new(());

/// Serialises writing a script and running it, so no forked child holds a
/// write handle of a script that another thread is about to execute.
fn lock() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// A fake curl that records its argv (one per line) in `argv` and then runs
/// `body` (`$out` is the value after `--output`).
fn script(dir: &Path, argv: &Path, body: &str) -> PathBuf {
    let path = dir.join("curl");
    let text = format!(
        "#!/bin/sh\n[ -n \"$FAKE_EXEC_PROBE\" ] && exit 0\nprintf '%s\\n' \"$@\" > '{}'\nout=\"\"; prev=\"\"\n\
         for a in \"$@\"; do if [ \"$prev\" = \"--output\" ]; then out=\"$a\"; fi; prev=\"$a\"; done\n{body}\n",
        argv.display()
    );
    fs::write(&path, text).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    wait_until_executable(&path);
    path
}

/// Waits until a freshly written fake program can be executed. A fork in
/// another test thread can briefly inherit the write descriptor, and exec then
/// fails with ETXTBSY ("Text file busy"), which the code under test reports as
/// a missing program. The probe run exits at once (`FAKE_EXEC_PROBE`).
fn wait_until_executable(path: &Path) {
    const ETXTBSY: i32 = 26;
    for _ in 0..400 {
        match std::process::Command::new(path)
            .env("FAKE_EXEC_PROBE", "1")
            .status()
        {
            Err(e) if e.raw_os_error() == Some(ETXTBSY) => {
                std::thread::sleep(std::time::Duration::from_millis(5))
            }
            _ => return,
        }
    }
    panic!("{} stayed busy", path.display());
}

const PKG_URL: &str =
    "https://github.com/M7MMAD-OMAR/sbarhive/releases/download/v0.4.0/sbarhive-0.4.0-1.x86_64.rpm";

// ------------------------------------------------------------ AC1

fn expected(url: &str, dest: &str, max: &str, time: &str) -> Vec<String> {
    [
        "--disable",
        "--silent",
        "--show-error",
        "--fail",
        "--location",
        "--proto",
        "=https",
        "--proto-redir",
        "=https",
        "--max-redirs",
        "5",
        "--max-time",
        time,
        "--max-filesize",
        max,
        "--user-agent",
        "sbarhive-update",
        "--output",
        dest,
        url,
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

#[test]
fn ac1_curl_args_for_a_manifest_and_its_signature() {
    let url = format!("{BASE}/stable.json");
    assert_eq!(
        curl_args(&url, Path::new("/tmp/x/out.part"), 262_144),
        expected(&url, "/tmp/x/out.part", "262144", "30")
    );
    let sig = format!("{BASE}/beta.json.minisig");
    assert_eq!(
        curl_args(&sig, Path::new("/tmp/x/sig"), 4096),
        expected(&sig, "/tmp/x/sig", "4096", "30")
    );
}

#[test]
fn ac1_curl_args_for_a_package_allow_1800_seconds() {
    assert_eq!(
        curl_args(PKG_URL, Path::new("/tmp/x/p.rpm.part"), 3_145_728),
        expected(PKG_URL, "/tmp/x/p.rpm.part", "3145728", "1800")
    );
    let deb = format!("{REL}/sbarhive_0.4.0_amd64.deb");
    assert_eq!(
        curl_args(&deb, Path::new("/tmp/x/d"), 1)[12],
        "1800",
        "the max-time value"
    );
}

#[test]
fn ac1_no_cookie_header_referer_or_version_in_the_arguments() {
    let dest = Path::new("/tmp/x/out.part");
    for url in [format!("{BASE}/stable.json"), PKG_URL.to_string()] {
        let args = curl_args(&url, dest, 1000);
        for a in &args {
            for banned in [
                "--cookie",
                "--cookie-jar",
                "--header",
                "--referer",
                "--user",
                "-b",
                "-c",
                "-H",
                "-e",
                "-A",
            ] {
                assert_ne!(a, banned, "{url}");
            }
        }
        // A package url names its own version; every other argument must not.
        let flags = &args[..args.len() - 1];
        for a in flags.iter().filter(|a| a.as_str() != "/tmp/x/out.part") {
            let digits: Vec<&str> = a.split('.').collect();
            let looks_versioned = digits.len() >= 3
                && digits
                    .iter()
                    .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
            assert!(!looks_versioned, "argument {a:?} looks like a version");
        }
    }
    let manifest_args = curl_args(&format!("{BASE}/stable.json"), dest, 1000);
    // The last argument is the url, which names the repository (M7MMAD).
    for a in &manifest_args[..manifest_args.len() - 1] {
        assert!(
            !a.chars().any(|c| c.is_ascii_digit()) || a.parse::<u64>().is_ok(),
            "{a:?}"
        );
    }
}

#[test]
fn ac1_fetch_error_is_comparable_and_clonable() {
    let all = [
        FetchError::Offline,
        FetchError::Http(404),
        FetchError::TooLarge,
        FetchError::Unavailable,
        FetchError::BadUrl,
    ];
    for (i, a) in all.iter().enumerate() {
        assert_eq!(a, &a.clone());
        for (j, b) in all.iter().enumerate() {
            assert_eq!(i == j, a == b);
        }
    }
    assert_ne!(FetchError::Http(404), FetchError::Http(500));
}

// ------------------------------------------------------------ AC2

#[test]
fn ac2_exit_codes_map_to_fetch_errors() {
    let _g = lock();
    let t = tempfile::tempdir().unwrap();
    let argv = t.path().join("argv");
    let dest = t.path().join("out.part");
    let url = format!("{BASE}/stable.json");
    let cases: [(&str, Result<(), FetchError>); 7] = [
        ("exit 6", Err(FetchError::Offline)),
        ("exit 7", Err(FetchError::Offline)),
        ("exit 28", Err(FetchError::Offline)),
        (
            "echo 'curl: (22) The requested URL returned error: 404' >&2; exit 22",
            Err(FetchError::Http(404)),
        ),
        (
            "echo 'curl: (22) The requested URL returned error: 503' >&2; exit 22",
            Err(FetchError::Http(503)),
        ),
        ("exit 63", Err(FetchError::TooLarge)),
        ("printf payload > \"$out\"; exit 0", Ok(())),
    ];
    for (body, want) in cases {
        let program = script(t.path(), &argv, body);
        let got = CurlFetcher::with_program(&program).get(&url, &dest, 1000);
        assert_eq!(got, want, "{body}");
    }
    assert_eq!(fs::read(&dest).unwrap(), b"payload");
    // Exit 22 without a status text is still an Http error.
    let program = script(t.path(), &argv, "exit 22");
    assert!(matches!(
        CurlFetcher::with_program(&program).get(&url, &dest, 1000),
        Err(FetchError::Http(_))
    ));
}

#[test]
fn ac2_the_program_gets_exactly_curl_args() {
    let _g = lock();
    let t = tempfile::tempdir().unwrap();
    let argv = t.path().join("argv");
    let dest = t.path().join("out.part");
    let program = script(t.path(), &argv, "exit 0");
    let fetcher = CurlFetcher::with_program(&program);
    for (url, max) in [
        (format!("{BASE}/stable.json"), 777u64),
        (PKG_URL.into(), 4242),
    ] {
        fs::remove_file(&argv).ok();
        fetcher.get(&url, &dest, max).unwrap();
        let recorded: Vec<String> = fs::read_to_string(&argv)
            .unwrap()
            .lines()
            .map(String::from)
            .collect();
        assert_eq!(recorded, curl_args(&url, &dest, max), "{url}");
    }
}

#[test]
fn ac2_an_empty_or_missing_program_is_unavailable() {
    let t = tempfile::tempdir().unwrap();
    let dest = t.path().join("out.part");
    let url = format!("{BASE}/stable.json");
    assert_eq!(
        CurlFetcher::with_program("").get(&url, &dest, 10),
        Err(FetchError::Unavailable)
    );
    assert_eq!(
        CurlFetcher::with_program(t.path().join("no-such-curl")).get(&url, &dest, 10),
        Err(FetchError::Unavailable)
    );
    assert!(!dest.exists());
}

fn env_of(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<OsString> {
    move |k| {
        pairs
            .iter()
            .find(|(n, _)| *n == k)
            .map(|(_, v)| OsString::from(*v))
    }
}

#[test]
fn ac2_linux_candidates_are_the_two_absolute_paths_and_never_path() {
    let asked = RefCell::new(Vec::<String>::new());
    let env = |k: &str| -> Option<OsString> {
        asked.borrow_mut().push(k.to_string());
        Some(OsString::from("/tmp/evil"))
    };
    assert_eq!(
        curl_candidates(Os::Linux, &env),
        vec![PathBuf::from("/usr/bin/curl"), PathBuf::from("/bin/curl")]
    );
    assert!(
        !asked
            .borrow()
            .iter()
            .any(|k| k.eq_ignore_ascii_case("PATH"))
    );
}

#[test]
fn ac2_windows_candidate_is_system32_under_system_root() {
    let env = env_of(&[("SystemRoot", "C:\\Windows"), ("PATH", "C:\\evil")]);
    assert_eq!(
        curl_candidates(Os::Windows, &env),
        vec![
            PathBuf::from("C:\\Windows")
                .join("System32")
                .join("curl.exe")
        ]
    );
    let none = env_of(&[("PATH", "C:\\evil")]);
    assert_eq!(curl_candidates(Os::Windows, &none), Vec::<PathBuf>::new());
}

// ------------------------------------------------------------ AC3

fn manual_off() -> Config {
    Config {
        update_check: false,
        ..Config::default()
    }
}

#[test]
fn ac3_automatic_check_with_update_check_off_is_disabled_and_silent() {
    let fake = good_feed();
    let before = state(Some(1), Some(2), 3, Some("0.9.9"));
    let r = run(
        &fake,
        "key-a.pub",
        "0.3.3",
        before.clone(),
        &manual_off(),
        10 * DAY,
        false,
    );
    assert_eq!(r.outcome, CheckOutcome::Disabled);
    assert_eq!(r.calls.len(), 0);
    assert_eq!(r.state, before);
}

#[test]
fn ac3_automatic_check_that_is_not_due_is_not_due_and_silent() {
    let fake = good_feed();
    let before = state(Some(1000), None, 0, Some("0.3.3"));
    let r = run(
        &fake,
        "key-a.pub",
        "0.3.3",
        before.clone(),
        &cfg(),
        1000 + DAY - 1,
        false,
    );
    assert_eq!(r.outcome, CheckOutcome::NotDue);
    assert_eq!(r.calls.len(), 0);
    assert_eq!(r.state, before);
    // Exactly one interval later it is due.
    let r = run(
        &fake,
        "key-a.pub",
        "0.3.3",
        before,
        &cfg(),
        1000 + DAY,
        false,
    );
    assert_ne!(r.outcome, CheckOutcome::NotDue);
    assert_eq!(r.calls.len(), 2);
}

#[test]
fn ac3_backoff_after_failures_also_means_not_due() {
    let fake = good_feed();
    let before = state(Some(0), Some(5000), 1, None);
    let r = run(
        &fake,
        "key-a.pub",
        "0.3.3",
        before.clone(),
        &cfg(),
        5000 + 3599,
        false,
    );
    assert_eq!(r.outcome, CheckOutcome::NotDue);
    assert_eq!(r.calls.len(), 0);
    assert_eq!(r.state, before);
    let r = run(
        &fake,
        "key-a.pub",
        "0.3.3",
        before,
        &cfg(),
        5000 + 3600,
        false,
    );
    assert_eq!(r.calls.len(), 2);
}

#[test]
fn ac3_manual_check_ignores_update_check_off() {
    let fake = good_feed();
    let r = run(
        &fake,
        "key-a.pub",
        "0.3.3",
        CheckState::default(),
        &manual_off(),
        50,
        true,
    );
    assert!(
        matches!(r.outcome, CheckOutcome::Available(_)),
        "{:?}",
        r.outcome
    );
    assert_eq!(
        r.calls.iter().map(|c| c.url.clone()).collect::<Vec<_>>(),
        vec![
            format!("{BASE}/stable.json"),
            format!("{BASE}/stable.json.minisig")
        ]
    );
}

#[test]
fn ac3_manual_check_ignores_not_due() {
    let fake = good_feed();
    let r = run(
        &fake,
        "key-a.pub",
        "0.3.3",
        state(Some(40), None, 0, None),
        &cfg(),
        50,
        true,
    );
    assert!(matches!(r.outcome, CheckOutcome::Available(_)));
    assert_eq!(r.calls.len(), 2);
}

#[test]
fn ac3_manual_check_ignores_both_at_once() {
    let fake = good_feed();
    let r = run(
        &fake,
        "key-a.pub",
        "0.3.3",
        state(Some(40), Some(45), 4, None),
        &manual_off(),
        50,
        true,
    );
    assert!(matches!(r.outcome, CheckOutcome::Available(_)));
    assert_eq!(r.calls.len(), 2);
}

#[test]
fn ac3_the_beta_channel_uses_the_beta_urls() {
    let fake = feed_c("manifest-beta-channel");
    let config = Config {
        update_channel: Channel::Beta,
        ..Config::default()
    };
    let r = run(
        &fake,
        "s053b/key-c.pub",
        "0.3.3",
        CheckState::default(),
        &config,
        5,
        true,
    );
    assert!(
        matches!(r.outcome, CheckOutcome::Available(_)),
        "{:?}",
        r.outcome
    );
    assert_eq!(
        fake.urls(),
        vec![
            format!("{BASE}/beta.json"),
            format!("{BASE}/beta.json.minisig")
        ]
    );
}

#[test]
fn ac3_the_fetches_land_in_scratch_with_the_caps_and_leave_it_empty() {
    assert!((1024..=1 << 20).contains(&MANIFEST_MAX_BYTES));
    assert!((1024..=1 << 20).contains(&SIGNATURE_MAX_BYTES));
    let fake = good_feed();
    let scratch = tempfile::tempdir().unwrap();
    let key = fx_text("key-a.pub");
    let keys = [key.as_str()];
    let env = CheckEnv {
        installed: "0.3.3",
        keys: &keys,
        platform: linux_rpm(),
        interval_secs: DAY,
        scratch: scratch.path(),
    };
    let mut st = CheckState::default();
    check_with(&env, &cfg(), &mut st, &fake, 1, true);
    let calls = fake.calls();
    assert_eq!(calls[0].max_bytes, MANIFEST_MAX_BYTES);
    assert_eq!(calls[1].max_bytes, SIGNATURE_MAX_BYTES);
    for c in &calls {
        assert!(c.dest.starts_with(scratch.path()), "{:?}", c.dest);
    }
    assert_ne!(calls[0].dest, calls[1].dest);
    assert_eq!(listing(scratch.path()), set(&[]));
}

#[test]
fn ac3_check_with_the_real_keys_refuses_a_fixture_signed_by_a_throwaway_key() {
    // `check` itself: the running version, UPDATE_KEYS, the running platform.
    let fake = good_feed();
    let mut st = CheckState::default();
    let out = check(&cfg(), &mut st, &fake, 77, true);
    assert_eq!(out, CheckOutcome::Rejected(UpdateError::BadSignature));
    assert_eq!(fake.calls().len(), 2);
    assert_eq!(st.schedule.failures, 1);
}

#[test]
fn ac3_check_disabled_and_not_due_make_no_call() {
    let fake = good_feed();
    let mut st = CheckState::default();
    assert_eq!(
        check(&manual_off(), &mut st, &fake, 77, false),
        CheckOutcome::Disabled
    );
    assert_eq!(fake.calls().len(), 0);
    let mut st = state(Some(70), None, 0, None);
    assert_eq!(
        check(&cfg(), &mut st, &fake, 77, false),
        CheckOutcome::NotDue
    );
    assert_eq!(fake.calls().len(), 0);
}

// ------------------------------------------------------------ AC4

fn rpm_update() -> Update {
    Update {
        version: "0.4.0".into(),
        notes_url: "https://github.com/M7MMAD-OMAR/sbarhive/releases/tag/v0.4.0".into(),
        asset: Asset {
            kind: "rpm".into(),
            os: "linux".into(),
            arch: "x86_64".into(),
            name: "sbarhive-0.4.0-1.x86_64.rpm".into(),
            url: PKG_URL.into(),
            size: 3_145_728,
            sha256: "a3f1c2d4e5b60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90".into(),
        },
    }
}

#[test]
fn ac4_a_good_run_is_available_with_the_selected_asset_and_resets_the_state() {
    let r = run(
        &good_feed(),
        "key-a.pub",
        "0.3.3",
        state(Some(100), Some(50), 3, None),
        &cfg(),
        9000,
        true,
    );
    assert_eq!(r.outcome, CheckOutcome::Available(rpm_update()));
    assert_eq!(r.state, state(Some(9000), Some(50), 0, Some("0.4.0")));
    assert_eq!(r.scratch_left, set(&[]));
}

#[test]
fn ac4_offline_on_the_manifest_counts_a_failure_and_moves_nothing_else() {
    let fake = Fake::failing(FetchError::Offline);
    let r = run(
        &fake,
        "key-a.pub",
        "0.3.3",
        state(Some(100), None, 1, Some("0.3.9")),
        &cfg(),
        9000,
        true,
    );
    assert_eq!(r.outcome, CheckOutcome::Offline);
    assert_eq!(
        r.calls.len(),
        1,
        "no signature fetch after a failed manifest"
    );
    assert_eq!(r.state, state(Some(100), Some(9000), 2, Some("0.3.9")));
    assert_eq!(r.scratch_left, set(&[]));
}

#[test]
fn ac4_offline_on_the_signature_is_the_same_outcome() {
    let manifest = fx("manifest-stable-0.4.0.json");
    let fake = Fake::new(move |url| {
        if url.ends_with(".minisig") {
            (Vec::new(), Err(FetchError::Offline))
        } else {
            (manifest.clone(), Ok(()))
        }
    });
    let r = run(
        &fake,
        "key-a.pub",
        "0.3.3",
        state(Some(100), None, 0, None),
        &cfg(),
        9000,
        true,
    );
    assert_eq!(r.outcome, CheckOutcome::Offline);
    assert_eq!(r.calls.len(), 2);
    assert_eq!(r.state, state(Some(100), Some(9000), 1, None));
    assert_eq!(r.scratch_left, set(&[]));
}

#[test]
fn ac4_a_missing_curl_is_unavailable_and_counts_as_a_failure() {
    let fake = Fake::failing(FetchError::Unavailable);
    let r = run(
        &fake,
        "key-a.pub",
        "0.3.3",
        state(Some(100), None, 0, None),
        &cfg(),
        9000,
        true,
    );
    assert_eq!(r.outcome, CheckOutcome::Unavailable);
    assert_eq!(r.state, state(Some(100), Some(9000), 1, None));
}

#[test]
fn ac4_an_http_error_is_rejected_with_the_status() {
    let fake = Fake::failing(FetchError::Http(404));
    let r = run(
        &fake,
        "key-a.pub",
        "0.3.3",
        state(Some(100), None, 0, None),
        &cfg(),
        9000,
        true,
    );
    assert_eq!(r.outcome, CheckOutcome::Rejected(UpdateError::Http(404)));
    assert_eq!(r.state, state(Some(100), Some(9000), 1, None));
}

#[test]
fn ac4_a_bad_signature_is_rejected_and_counts_a_failure() {
    // The manifest is fine but the signature is by key B and only key A is trusted.
    let fake = Fake::feed(
        fx("manifest-stable-0.4.0.json"),
        fx("manifest-stable-0.4.0.json.b.minisig"),
    );
    let r = run(
        &fake,
        "key-a.pub",
        "0.3.3",
        state(Some(100), None, 1, Some("0.3.9")),
        &cfg(),
        9000,
        true,
    );
    assert_eq!(r.outcome, CheckOutcome::Rejected(UpdateError::BadSignature));
    assert_eq!(r.calls.len(), 2);
    assert_eq!(r.state, state(Some(100), Some(9000), 2, Some("0.3.9")));
    assert_eq!(r.scratch_left, set(&[]));
}

#[test]
fn ac4_a_signature_text_that_is_not_a_signature_is_a_bad_signature() {
    let fake = Fake::feed(
        fx("manifest-stable-0.4.0.json"),
        b"not a signature".to_vec(),
    );
    let r = run(
        &fake,
        "key-a.pub",
        "0.3.3",
        CheckState::default(),
        &cfg(),
        5,
        true,
    );
    assert_eq!(r.outcome, CheckOutcome::Rejected(UpdateError::BadSignature));
}

#[test]
fn ac4_a_signed_future_schema_is_rejected() {
    let r = run_c("manifest-schema-2");
    assert_eq!(
        r.outcome,
        CheckOutcome::Rejected(UpdateError::UnsupportedSchema)
    );
    assert_eq!(r.state, state(None, Some(10_000), 1, None));
}

#[test]
fn ac4_a_signed_manifest_of_another_channel_is_rejected() {
    // The stable config fetched stable.json, which holds a beta manifest.
    let r = run_c("manifest-beta-channel");
    assert_eq!(
        r.outcome,
        CheckOutcome::Rejected(UpdateError::ChannelMismatch)
    );
    assert_eq!(r.state, state(None, Some(10_000), 1, None));
}

#[test]
fn ac4_a_manifest_without_an_asset_for_this_platform_is_rejected() {
    let r = run_c("manifest-no-linux-asset");
    assert_eq!(r.outcome, CheckOutcome::Rejected(UpdateError::NoAsset));
    assert_eq!(r.state, state(None, Some(10_000), 1, None));
}

#[test]
fn ac4_the_same_version_is_up_to_date_and_still_a_success() {
    let r = run(
        &good_feed(),
        "key-a.pub",
        "0.4.0",
        state(Some(100), None, 2, None),
        &cfg(),
        9000,
        true,
    );
    assert_eq!(r.outcome, CheckOutcome::UpToDate);
    assert_eq!(r.state, state(Some(9000), None, 0, Some("0.4.0")));
}

#[test]
fn ac4_a_lower_manifest_version_is_up_to_date_not_an_error() {
    let r = run(
        &good_feed(),
        "key-a.pub",
        "0.5.0",
        state(Some(100), None, 2, None),
        &cfg(),
        9000,
        true,
    );
    assert_eq!(r.outcome, CheckOutcome::UpToDate);
    assert_eq!(r.state, state(Some(9000), None, 0, Some("0.4.0")));
}

#[test]
fn ac4_a_prerelease_installed_build_is_below_the_final_release() {
    let r = run(
        &good_feed(),
        "key-a.pub",
        "0.4.0-beta.1",
        CheckState::default(),
        &cfg(),
        9000,
        true,
    );
    assert!(matches!(r.outcome, CheckOutcome::Available(_)));
}

#[test]
fn ac4_a_successful_check_after_failures_restores_the_normal_schedule() {
    // 3 failures, then a good run: the next automatic check waits a full day.
    let r = run(
        &good_feed(),
        "key-a.pub",
        "0.3.3",
        state(Some(1), Some(8000), 3, None),
        &cfg(),
        9000,
        true,
    );
    assert_eq!(r.state.schedule.failures, 0);
    let again = run(
        &good_feed(),
        "key-a.pub",
        "0.3.3",
        r.state.clone(),
        &cfg(),
        9000 + DAY - 1,
        false,
    );
    assert_eq!(again.outcome, CheckOutcome::NotDue);
}

// ------------------------------------------------------------ AC5

fn update_for(name: &str, payload_len: u64, sha: &str) -> Update {
    let mut u = rpm_update();
    u.asset.name = name.into();
    u.asset.url = format!("{REL}/{name}");
    u.asset.size = payload_len;
    u.asset.sha256 = sha.into();
    u
}

fn first_update() -> Update {
    update_for("sbarhive-0.4.0-1.x86_64.rpm", FIRST.len() as u64, SHA_FIRST)
}

#[test]
fn ac5_matching_bytes_give_a_verified_package_and_no_part() {
    let t = tempfile::tempdir().unwrap();
    let u = first_update();
    let fake = Fake::bytes(FIRST.to_vec());
    let pkg = download_verified(&u, t.path(), &fake).unwrap();
    assert_eq!(pkg.path(), t.path().join("sbarhive-0.4.0-1.x86_64.rpm"));
    assert_eq!(listing(t.path()), set(&["sbarhive-0.4.0-1.x86_64.rpm"]));
    assert_eq!(fs::read(pkg.path()).unwrap(), FIRST);
    let calls = fake.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].url, u.asset.url);
    assert_eq!(
        calls[0].dest,
        t.path().join("sbarhive-0.4.0-1.x86_64.rpm.part")
    );
    assert_eq!(calls[0].max_bytes, 17);
}

#[test]
fn ac5_a_payload_of_several_read_chunks_is_streamed_and_verified() {
    let t = tempfile::tempdir().unwrap();
    let u = update_for("big.rpm", 200_000, SHA_BIG);
    let pkg = download_verified(&u, t.path(), &Fake::bytes(big())).unwrap();
    assert_eq!(fs::read(pkg.path()).unwrap(), big());
    assert_eq!(listing(t.path()), set(&["big.rpm"]));
}

#[test]
fn ac5_one_flipped_byte_is_a_hash_mismatch_and_leaves_nothing() {
    for flip in [0usize, 5, 16] {
        let t = tempfile::tempdir().unwrap();
        let mut bytes = FIRST.to_vec();
        bytes[flip] ^= 1;
        let got = download_verified(&first_update(), t.path(), &Fake::bytes(bytes)).unwrap_err();
        assert_eq!(got, UpdateError::HashMismatch, "flip at {flip}");
        assert_eq!(listing(t.path()), set(&[]), "flip at {flip}");
    }
}

#[test]
fn ac5_a_flip_after_the_first_chunks_is_still_caught() {
    // 65535, 65536, 65537 straddle the 64 KiB read; 131072 the second; the last byte.
    for flip in [65_535usize, 65_536, 65_537, 131_072, 70_000, 199_999] {
        let t = tempfile::tempdir().unwrap();
        let mut bytes = big();
        bytes[flip] ^= 0x80;
        let u = update_for("big.rpm", 200_000, SHA_BIG);
        let got = download_verified(&u, t.path(), &Fake::bytes(bytes)).unwrap_err();
        assert_eq!(got, UpdateError::HashMismatch, "flip at {flip}");
        assert_eq!(listing(t.path()), set(&[]), "flip at {flip}");
    }
}

#[test]
fn ac5_more_bytes_than_size_is_a_size_mismatch_and_leaves_nothing() {
    let t = tempfile::tempdir().unwrap();
    let mut bytes = FIRST.to_vec();
    bytes.extend_from_slice(b"xx");
    let got = download_verified(&first_update(), t.path(), &Fake::bytes(bytes)).unwrap_err();
    assert_eq!(got, UpdateError::SizeMismatch);
    assert_eq!(listing(t.path()), set(&[]));
}

#[test]
fn ac5_fewer_bytes_than_size_is_a_size_mismatch_and_leaves_nothing() {
    let t = tempfile::tempdir().unwrap();
    let got = download_verified(
        &first_update(),
        t.path(),
        &Fake::bytes(FIRST[..16].to_vec()),
    )
    .unwrap_err();
    assert_eq!(got, UpdateError::SizeMismatch);
    assert_eq!(listing(t.path()), set(&[]));
}

#[test]
fn ac5_a_size_that_matches_a_hash_that_does_not_is_a_hash_mismatch_not_a_size_one() {
    let t = tempfile::tempdir().unwrap();
    let got = download_verified(
        &first_update(),
        t.path(),
        &Fake::bytes(b"first-asset-byteZ".to_vec()),
    )
    .unwrap_err();
    assert_eq!(got, UpdateError::HashMismatch);
}

#[test]
fn ac5_no_file_is_created_when_the_fetcher_writes_nothing_and_succeeds() {
    let t = tempfile::tempdir().unwrap();
    let fake = Fake::new(|_| (Vec::new(), Ok(())));
    let got = download_verified(&first_update(), t.path(), &fake).unwrap_err();
    assert!(
        matches!(got, UpdateError::SizeMismatch | UpdateError::Io),
        "{got:?}"
    );
    assert_eq!(listing(t.path()), set(&[]));
}

#[test]
fn ac5_fetcher_failures_map_and_remove_a_partial_file() {
    let cases = [
        (FetchError::Offline, UpdateError::Offline),
        (FetchError::Unavailable, UpdateError::Unavailable),
        (FetchError::Http(404), UpdateError::Http(404)),
        (FetchError::TooLarge, UpdateError::SizeMismatch),
    ];
    for (fetch_err, want) in cases {
        let t = tempfile::tempdir().unwrap();
        let e = fetch_err.clone();
        let fake = Fake::new(move |_| (b"par".to_vec(), Err(e.clone())));
        let got = download_verified(&first_update(), t.path(), &fake).unwrap_err();
        assert_eq!(got, want, "{fetch_err:?}");
        assert_eq!(listing(t.path()), set(&[]), "{fetch_err:?}");
    }
}

#[test]
fn ac5_the_directory_is_created_when_missing() {
    let t = tempfile::tempdir().unwrap();
    let dir = t.path().join("updates").join("0.4.0");
    let pkg = download_verified(&first_update(), &dir, &Fake::bytes(FIRST.to_vec())).unwrap();
    assert_eq!(pkg.path(), dir.join("sbarhive-0.4.0-1.x86_64.rpm"));
    assert!(pkg.path().is_file());
}

#[test]
fn ac5_verified_package_has_no_public_constructor() {
    let src = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/update/flow.rs"))
        .expect("flow.rs must exist");
    let at = src
        .find("struct VerifiedPackage")
        .expect("VerifiedPackage must be defined");
    let body_start = src[at..].find(['{', '(', ';']).map(|i| at + i).unwrap();
    let body_end = src[body_start..]
        .find('}')
        .map(|i| body_start + i)
        .unwrap_or(src.len());
    let fields = &src[body_start..body_end.min(body_start + 400)];
    assert!(
        !fields.contains("pub "),
        "VerifiedPackage must have private fields: {fields}"
    );
    assert!(
        !src.contains("impl From<") || !src.contains("for VerifiedPackage"),
        "no From impl may build a VerifiedPackage"
    );
    // Only functions that return it from this module: no `pub fn new`.
    assert!(!src.contains("pub fn new("), "no public constructor");
}

// ------------------------------------------------------------ AC6

fn updates_tree() -> (tempfile::TempDir, PathBuf) {
    let t = tempfile::tempdir().unwrap();
    let updates = t.path().join("updates");
    for stale in ["0.3.8", "0.3.9"] {
        fs::create_dir_all(updates.join(stale)).unwrap();
        fs::write(updates.join(stale).join("old.rpm"), b"old").unwrap();
    }
    let dir = updates.join("0.4.0");
    (t, dir)
}

#[test]
fn ac6_after_a_good_download_only_the_new_version_folder_remains() {
    let (t, dir) = updates_tree();
    download_verified(&first_update(), &dir, &Fake::bytes(FIRST.to_vec())).unwrap();
    assert_eq!(listing(&t.path().join("updates")), set(&["0.4.0"]));
    assert_eq!(listing(&dir), set(&["sbarhive-0.4.0-1.x86_64.rpm"]));
}

#[test]
fn ac6_a_folder_that_already_exists_with_the_new_version_is_kept() {
    let (t, dir) = updates_tree();
    fs::create_dir_all(&dir).unwrap();
    download_verified(&first_update(), &dir, &Fake::bytes(FIRST.to_vec())).unwrap();
    assert_eq!(listing(&t.path().join("updates")), set(&["0.4.0"]));
}

#[test]
fn ac6_offline_leaves_an_empty_folder_and_keeps_the_stale_ones() {
    let (t, dir) = updates_tree();
    fs::create_dir_all(&dir).unwrap();
    let got =
        download_verified(&first_update(), &dir, &Fake::failing(FetchError::Offline)).unwrap_err();
    assert_eq!(got, UpdateError::Offline);
    assert_eq!(listing(&dir), set(&[]));
    assert_eq!(
        listing(&t.path().join("updates")),
        set(&["0.3.8", "0.3.9", "0.4.0"])
    );
}

#[test]
fn ac6_a_failed_verification_does_not_delete_the_stale_folders() {
    let (t, dir) = updates_tree();
    let mut bad = FIRST.to_vec();
    bad[3] ^= 1;
    let got = download_verified(&first_update(), &dir, &Fake::bytes(bad)).unwrap_err();
    assert_eq!(got, UpdateError::HashMismatch);
    assert_eq!(
        listing(&t.path().join("updates")),
        set(&["0.3.8", "0.3.9", "0.4.0"])
    );
    assert_eq!(listing(&dir), set(&[]));
}

// ------------------------------------------------------------ AC7

fn no_known() -> Option<PathBuf> {
    None
}

#[test]
fn ac7_linux_cache_dir_prefers_xdg_then_home() {
    let both = env_of(&[("XDG_CACHE_HOME", "/x/cache"), ("HOME", "/home/u")]);
    assert_eq!(
        cache_dir_with(Os::Linux, &both, &no_known),
        Some(PathBuf::from("/x/cache").join("sbarhive"))
    );
    let home = env_of(&[("HOME", "/home/u")]);
    assert_eq!(
        cache_dir_with(Os::Linux, &home, &no_known),
        Some(PathBuf::from("/home/u").join(".cache").join("sbarhive"))
    );
    let xdg = env_of(&[("XDG_CACHE_HOME", "/x/cache")]);
    assert_eq!(
        cache_dir_with(Os::Linux, &xdg, &no_known),
        Some(PathBuf::from("/x/cache").join("sbarhive"))
    );
    let none = env_of(&[]);
    assert_eq!(cache_dir_with(Os::Linux, &none, &no_known), None);
}

#[test]
fn ac7_linux_ignores_the_windows_variable_and_the_known_folder() {
    let env = env_of(&[("LOCALAPPDATA", "C:\\L"), ("HOME", "/home/u")]);
    let known = || Some(PathBuf::from("/known"));
    assert_eq!(
        cache_dir_with(Os::Linux, &env, &known),
        Some(PathBuf::from("/home/u").join(".cache").join("sbarhive"))
    );
}

#[test]
fn ac7_windows_cache_dir_is_local_app_data_sbarhive() {
    let env = env_of(&[
        ("LOCALAPPDATA", "C:\\Users\\u\\AppData\\Local"),
        ("APPDATA", "C:\\Users\\u\\AppData\\Roaming"),
        ("XDG_CACHE_HOME", "/nope"),
        ("HOME", "/nope"),
    ]);
    let known = || Some(PathBuf::from("C:\\known"));
    assert_eq!(
        cache_dir_with(Os::Windows, &env, &known),
        Some(PathBuf::from("C:\\Users\\u\\AppData\\Local").join("SbarHive"))
    );
}

#[test]
fn ac7_windows_falls_back_to_the_known_folder_when_unset_or_empty() {
    let known = || Some(PathBuf::from("C:\\known"));
    let unset = env_of(&[]);
    assert_eq!(
        cache_dir_with(Os::Windows, &unset, &known),
        Some(PathBuf::from("C:\\known").join("SbarHive"))
    );
    let empty = env_of(&[("LOCALAPPDATA", "")]);
    assert_eq!(
        cache_dir_with(Os::Windows, &empty, &known),
        Some(PathBuf::from("C:\\known").join("SbarHive"))
    );
    assert_eq!(cache_dir_with(Os::Windows, &unset, &no_known), None);
}

#[test]
fn ac7_the_update_folder_is_cache_updates_version() {
    let env = env_of(&[("XDG_CACHE_HOME", "/x/cache")]);
    assert_eq!(
        update_dir_with(Os::Linux, &env, &no_known, "0.4.0"),
        Some(
            PathBuf::from("/x/cache")
                .join("sbarhive")
                .join("updates")
                .join("0.4.0")
        )
    );
    let win = env_of(&[("LOCALAPPDATA", "C:\\L")]);
    assert_eq!(
        update_dir_with(Os::Windows, &win, &no_known, "0.4.0-beta.2"),
        Some(
            PathBuf::from("C:\\L")
                .join("SbarHive")
                .join("updates")
                .join("0.4.0-beta.2")
        )
    );
    assert_eq!(
        update_dir_with(Os::Linux, &env_of(&[]), &no_known, "0.4.0"),
        None
    );
}

#[test]
fn ac7_update_state_sits_next_to_config_toml() {
    let lin = env_of(&[("XDG_CONFIG_HOME", "/x/conf"), ("HOME", "/home/u")]);
    let cfg_dir = config_dir_with(Os::Linux, &lin, &no_known).unwrap();
    assert_eq!(
        update_state_file_with(Os::Linux, &lin, &no_known),
        Some(cfg_dir.join("update-state.json"))
    );
    assert_eq!(
        update_state_file_with(Os::Linux, &lin, &no_known),
        Some(
            PathBuf::from("/x/conf")
                .join("sbarhive")
                .join("update-state.json")
        )
    );
    let win = env_of(&[("APPDATA", "C:\\R")]);
    assert_eq!(
        update_state_file_with(Os::Windows, &win, &no_known),
        Some(
            PathBuf::from("C:\\R")
                .join("SbarHive")
                .join("update-state.json")
        )
    );
    assert_eq!(
        update_state_file_with(Os::Linux, &env_of(&[]), &no_known),
        None
    );
}

// ------------------------------------------------------------ AC9

/// One url per class the story names, plus the neighbours that must pass.
fn bad_urls() -> Vec<(&'static str, String)> {
    let p = format!("{REL}/");
    vec![
        ("dotdot", format!("{p}../../evil/x.rpm")),
        ("dotdot at end", format!("{p}x..")),
        ("pct2e", format!("{p}%2e%2e/x.rpm")),
        ("pct2E", format!("{p}%2E%2E/x.rpm")),
        ("pct2e one dot", format!("{p}a%2eb.rpm")),
        ("backslash", format!("{p}a\\b.rpm")),
        ("space", format!("{p}a b.rpm")),
        ("tab", format!("{p}a\tb.rpm")),
        ("newline", format!("{p}a\nb.rpm")),
        ("cr", format!("{p}a\rb.rpm")),
        ("nbsp", format!("{p}a\u{a0}b.rpm")),
        ("ctrl 0x01", format!("{p}a\u{1}b.rpm")),
        ("del 0x7f", format!("{p}a\u{7f}b.rpm")),
        ("trailing space", format!("{p}x.rpm ")),
        ("leading space", format!(" {p}x.rpm")),
    ]
}

#[test]
fn ac9_curl_get_refuses_bad_urls_before_running_curl() {
    let _g = lock();
    let t = tempfile::tempdir().unwrap();
    let argv = t.path().join("argv");
    let dest = t.path().join("out.part");
    let program = script(t.path(), &argv, "printf payload > \"$out\"; exit 0");
    let fetcher = CurlFetcher::with_program(&program);
    for (what, url) in bad_urls() {
        assert_eq!(
            fetcher.get(&url, &dest, 100),
            Err(FetchError::BadUrl),
            "{what}"
        );
        assert!(!argv.exists(), "curl ran for {what}");
        assert!(!dest.exists(), "{what}");
    }
}

#[test]
fn ac9_good_urls_with_single_dots_and_other_percent_codes_still_run() {
    let _g = lock();
    let t = tempfile::tempdir().unwrap();
    let argv = t.path().join("argv");
    let dest = t.path().join("out.part");
    let program = script(t.path(), &argv, "printf payload > \"$out\"; exit 0");
    let fetcher = CurlFetcher::with_program(&program);
    for url in [
        PKG_URL.to_string(),
        format!("{BASE}/stable.json.minisig"),
        format!("{REL}/sbarhive_0.4.0~beta.1_amd64.deb"),
        format!("{REL}/a%2fb.rpm"),
        format!("{REL}/a%20b.rpm"),
    ] {
        fs::remove_file(&argv).ok();
        assert_eq!(fetcher.get(&url, &dest, 100), Ok(()), "{url}");
        assert!(argv.exists(), "{url}");
    }
}

#[test]
fn ac9_the_system_fetcher_refuses_too() {
    for (what, url) in bad_urls() {
        let dest = Path::new("/nonexistent-dir/out.part");
        assert_eq!(
            CurlFetcher::system().get(&url, dest, 100),
            Err(FetchError::BadUrl),
            "{what}"
        );
    }
}

#[test]
fn ac9_check_rejects_a_signed_manifest_with_a_bad_asset_url() {
    for tag in [
        "dotdot",
        "pctlower",
        "pctupper",
        "backslash",
        "space",
        "ctrl",
        "tab",
    ] {
        let r = run_c(&format!("manifest-url-{tag}"));
        assert_eq!(
            r.outcome,
            CheckOutcome::Rejected(UpdateError::BadUrl),
            "{tag}"
        );
        assert_eq!(r.calls.len(), 2, "{tag}");
        assert_eq!(r.state, state(None, Some(10_000), 1, None), "{tag}");
    }
}

#[test]
fn ac9_download_verified_refuses_a_bad_url_before_any_fetch() {
    for (what, url) in bad_urls() {
        let t = tempfile::tempdir().unwrap();
        let mut u = first_update();
        u.asset.url = url;
        let fake = Fake::bytes(FIRST.to_vec());
        assert_eq!(
            download_verified(&u, t.path(), &fake).unwrap_err(),
            UpdateError::BadUrl,
            "{what}"
        );
        assert_eq!(fake.calls().len(), 0, "{what}");
        assert_eq!(listing(t.path()), set(&[]), "{what}");
    }
}

#[test]
fn ac9_select_asset_returns_the_first_duplicate() {
    let m = Manifest::parse(&fx("s053b/manifest-dup-assets.json")).unwrap();
    let a = select_asset(&m, linux_rpm()).unwrap();
    assert_eq!(a.name, "sbarhive-0.4.0-1.x86_64.rpm");
    assert_eq!(a.size, 17);
    assert_eq!(a.sha256, SHA_FIRST);
    assert_eq!(m.assets.len(), 3);
    assert_eq!(m.assets[1].sha256, SHA_SECOND);
}

#[test]
fn ac9_check_selects_the_first_duplicate_and_download_verifies_that_one() {
    let r = run_c("manifest-dup-assets");
    let CheckOutcome::Available(update) = r.outcome else {
        panic!("expected Available, got {:?}", r.outcome);
    };
    assert_eq!(update.asset.name, "sbarhive-0.4.0-1.x86_64.rpm");
    assert_eq!(update.asset.size, 17);
    assert_eq!(update.asset.sha256, SHA_FIRST);
    assert_eq!(
        update.asset.url,
        format!("{REL}/sbarhive-0.4.0-1.x86_64.rpm")
    );

    // The first asset's bytes pass.
    let t = tempfile::tempdir().unwrap();
    let pkg = download_verified(&update, t.path(), &Fake::bytes(FIRST.to_vec())).unwrap();
    assert_eq!(fs::read(pkg.path()).unwrap(), FIRST);

    // The second asset's bytes (another size) do not.
    let t = tempfile::tempdir().unwrap();
    assert_eq!(
        download_verified(&update, t.path(), &Fake::bytes(SECOND.to_vec())).unwrap_err(),
        UpdateError::SizeMismatch
    );
    assert_eq!(listing(t.path()), set(&[]));

    // Seventeen bytes of something else do not either.
    let t = tempfile::tempdir().unwrap();
    assert_eq!(
        download_verified(
            &update,
            t.path(),
            &Fake::bytes(b"second-asset-byt".to_vec())
        )
        .unwrap_err(),
        UpdateError::SizeMismatch
    );
    let t = tempfile::tempdir().unwrap();
    assert_eq!(
        download_verified(
            &update,
            t.path(),
            &Fake::bytes(b"second-asset-bytx".to_vec())
        )
        .unwrap_err(),
        UpdateError::HashMismatch
    );
}
