//! S-058 (backlog/S-058-ci-green.md): CI green again. Source and behaviour
//! checks for `ci.yml`, `scripts/windows/make-ico.sh`, `scripts/ci-status.sh`
//! and the `CLAUDE.md` rule. They live in the daemon crate so that the
//! `windows-native` job (core, engines, cli) does not run them.
//!
//! Contract of `scripts/ci-status.sh [<sha>] [--wait]` that these tests fix:
//! - it calls only `gh run list` (JSON fields `databaseId`, `status`,
//!   `conclusion`) and `gh run view <id>` (JSON fields `status`,
//!   `conclusion`, `jobs[].name|status|conclusion`); `--json` and `--jq` may
//!   be used, the fake `gh` applies `--jq` with the system `jq`;
//! - stdout is exactly `run <id> <conclusion|status>` then one line
//!   `<job name> <conclusion|status>` per job; a conclusion that is empty or
//!   null falls back to the status; progress and errors go to stderr;
//! - exit 0 when the run and all jobs are `success`, 1 when a job failed or
//!   the run is still going (also after the `--wait` limit), 2 with a message
//!   on stderr when `gh` is missing or no run exists for the commit;
//! - `--wait` polls every `CI_STATUS_POLL_SECONDS` (default 20) for at most
//!   `CI_STATUS_MAX_WAIT_SECONDS` (default 1200) and prints only the final
//!   state.
//! - `scripts/windows/make-ico.sh` uses `magick`, else `convert`, and exits 2
//!   with `missing tool: <name>` on stderr when inkscape or ImageMagick is
//!   missing.

use std::collections::BTreeSet;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn read(rel: &str) -> String {
    let path = root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} must exist: {e}", path.display()))
}

fn out_text(o: &Output) -> (String, String) {
    (
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

// ---------------------------------------------------------------- ci.yml

/// The lines of one job of ci.yml (from `  name:` to the next job).
fn job_lines(name: &str) -> Vec<String> {
    let text = read(".github/workflows/ci.yml");
    let head = format!("  {name}:");
    let mut inside = false;
    let mut out = Vec::new();
    for l in text.lines() {
        if l == head {
            inside = true;
            continue;
        }
        if inside {
            let top = l.starts_with("  ") && !l.starts_with("   ") && l.trim_end().ends_with(':');
            if top {
                break;
            }
            out.push(l.to_string());
        }
    }
    assert!(!out.is_empty(), "job {name} not found in ci.yml");
    out
}

/// Package names on the `apt-get install` command of the `check` job.
fn check_apt_packages() -> BTreeSet<String> {
    let lines = job_lines("check");
    let mut joined = String::new();
    let mut on = false;
    for l in &lines {
        if l.contains("apt-get install") {
            on = true;
        }
        if on {
            joined.push_str(l.trim_end_matches('\\').trim());
            joined.push(' ');
            if !l.trim_end().ends_with('\\') {
                break;
            }
        }
    }
    assert!(on, "no apt-get install in the check job");
    joined
        .split_whitespace()
        .skip_while(|w| *w != "install")
        .skip(1)
        .filter(|w| !w.starts_with('-'))
        .map(String::from)
        .collect()
}

#[test]
fn ac2_windows_native_runs_every_test_binary_with_no_fail_fast() {
    let lines = job_lines("windows-native");
    let want = "cargo test --no-fail-fast -p sbarhive-core -p sbarhive-engines -p sbarhive-cli";
    assert!(
        lines.iter().any(|l| l.contains(want)),
        "windows-native must run `{want}`, cargo otherwise stops at the first failing binary"
    );
}

#[test]
fn ac3_check_job_installs_inkscape_and_imagemagick() {
    let pkgs = check_apt_packages();
    for p in ["inkscape", "imagemagick"] {
        assert!(pkgs.contains(p), "{p} missing from the apt line: {pkgs:?}");
    }
}

// ---------------------------------------------------------------- make-ico

/// A directory used as the whole PATH: symlinks to the named programs only.
fn path_with(tools: &[&str]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for t in tools {
        let found = ["/usr/bin", "/bin", "/usr/local/bin"]
            .iter()
            .map(|d| Path::new(d).join(t))
            .find(|p| p.exists())
            .unwrap_or_else(|| panic!("{t} must exist on this machine for the test"));
        symlink(found, dir.path().join(t)).unwrap();
    }
    dir
}

/// A sandbox copy of the repo parts make-ico.sh needs, so the committed
/// design/brand/sbarhive.ico is never touched by these tests.
fn ico_sandbox() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("scripts/windows")).unwrap();
    std::fs::create_dir_all(d.path().join("design/brand")).unwrap();
    std::fs::copy(
        root().join("scripts/windows/make-ico.sh"),
        d.path().join("scripts/windows/make-ico.sh"),
    )
    .unwrap();
    std::fs::copy(
        root().join("design/brand/sbarhive.svg"),
        d.path().join("design/brand/sbarhive.svg"),
    )
    .unwrap();
    d
}

fn run_ico(sandbox: &Path, path_dir: &Path) -> Output {
    Command::new("/usr/bin/bash")
        .arg(sandbox.join("scripts/windows/make-ico.sh"))
        .env_clear()
        .env("PATH", path_dir)
        .env("HOME", sandbox)
        .output()
        .unwrap()
}

fn ico_sizes(bytes: &[u8]) -> Vec<u32> {
    assert_eq!(&bytes[0..4], &[0, 0, 1, 0], "not an ICO");
    let n = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    (0..n)
        .map(|i| match bytes[6 + 16 * i] {
            0 => 256,
            b => u32::from(b),
        })
        .collect()
}

const BASE_TOOLS: [&str; 6] = ["bash", "dirname", "mktemp", "rm", "mv", "env"];

#[test]
fn ac3_make_ico_falls_back_to_convert_and_is_deterministic() {
    // `magick` is hidden: only `convert` is on PATH.
    let mut tools = BASE_TOOLS.to_vec();
    tools.extend(["inkscape", "convert"]);
    let path = path_with(&tools);
    assert!(!path.path().join("magick").exists());
    let sb = ico_sandbox();
    let ico = sb.path().join("design/brand/sbarhive.ico");
    let o1 = run_ico(sb.path(), path.path());
    assert_eq!(o1.status.code(), Some(0), "run 1: {:?}", out_text(&o1));
    let first = std::fs::read(&ico).unwrap();
    let o2 = run_ico(sb.path(), path.path());
    assert_eq!(o2.status.code(), Some(0), "run 2: {:?}", out_text(&o2));
    let second = std::fs::read(&ico).unwrap();
    assert!(first == second, "two runs must give the same bytes");
    let sizes = ico_sizes(&first);
    for want in [16, 32, 48] {
        assert!(sizes.contains(&want), "{want} missing in {sizes:?}");
    }
}

#[test]
fn ac3_make_ico_still_prefers_magick_when_present() {
    let mut tools = BASE_TOOLS.to_vec();
    tools.extend(["inkscape", "magick"]);
    let path = path_with(&tools);
    let sb = ico_sandbox();
    let o = run_ico(sb.path(), path.path());
    assert_eq!(o.status.code(), Some(0), "{:?}", out_text(&o));
    assert!(sb.path().join("design/brand/sbarhive.ico").exists());
}

#[test]
fn ac3_make_ico_exits_2_naming_the_missing_tool_when_both_are_hidden() {
    let path = path_with(&BASE_TOOLS);
    let sb = ico_sandbox();
    let o = run_ico(sb.path(), path.path());
    let (_, err) = out_text(&o);
    assert_eq!(o.status.code(), Some(2), "{err}");
    assert!(err.contains("missing tool: "), "{err}");
    assert!(
        !sb.path().join("design/brand/sbarhive.ico").exists(),
        "nothing may be written when a tool is missing"
    );
}

#[test]
fn ac3_make_ico_names_inkscape_when_only_inkscape_is_missing() {
    let mut tools = BASE_TOOLS.to_vec();
    tools.push("convert");
    let path = path_with(&tools);
    let sb = ico_sandbox();
    let o = run_ico(sb.path(), path.path());
    let (_, err) = out_text(&o);
    assert_eq!(o.status.code(), Some(2), "{err}");
    assert!(err.contains("missing tool: inkscape"), "{err}");
}

#[test]
fn ac3_make_ico_names_imagemagick_when_only_imagemagick_is_missing() {
    let mut tools = BASE_TOOLS.to_vec();
    tools.push("inkscape");
    let path = path_with(&tools);
    let sb = ico_sandbox();
    let o = run_ico(sb.path(), path.path());
    let (_, err) = out_text(&o);
    assert_eq!(o.status.code(), Some(2), "{err}");
    let named = err
        .lines()
        .find_map(|l| l.strip_prefix("missing tool: "))
        .unwrap_or_else(|| panic!("no `missing tool: <name>` line in {err}"));
    assert!(
        named.contains("magick") || named.contains("convert") || named.contains("ImageMagick"),
        "{named}"
    );
}

// ---------------------------------------------------------------- AC1

/// The body of the unit test in config.rs, from its `fn` line to the next
/// `#[test]`.
fn config_test_body() -> (String, String) {
    let src = read("crates/core/src/config.rs");
    let at = src
        .find("fn path_and_load_follow_xdg_then_home")
        .expect("the test must stay in config.rs");
    let attr_line = src[..at].trim_end().lines().last().unwrap().to_string();
    let rest = &src[at..];
    let end = rest.find("#[test]").unwrap_or(rest.len());
    (attr_line, rest[..end].to_string())
}

#[test]
fn ac1_config_path_test_is_platform_aware_and_not_removed_by_cfg() {
    let (attr, body) = config_test_body();
    assert_eq!(attr.trim(), "#[test]", "no cfg may sit on the test itself");
    // Unix keeps today's assertions.
    assert!(body.contains("XDG_CONFIG_HOME"));
    assert!(body.contains("\"HOME\""));
    assert!(body.contains(".config"));
    assert!(body.contains("#[cfg(unix)]") || body.contains("cfg!(unix)"));
}

#[test]
fn ac1_config_path_test_windows_branch_checks_appdata_with_three_assertions() {
    let (_, body) = config_test_body();
    let at = body
        .find("#[cfg(windows)]")
        .or_else(|| body.find("cfg!(windows)"))
        .expect("the test needs a cfg(windows) branch");
    let win = &body[at..];
    assert!(
        win.contains("APPDATA"),
        "the Windows branch must use APPDATA"
    );
    assert!(win.contains("SbarHive"), "expected folder SbarHive");
    assert!(win.contains("config.toml"));
    assert!(
        win.contains("Quality::Light"),
        "round trip of quality = Light"
    );
    assert!(
        win.contains("remove_var(\"APPDATA\")"),
        "unset APPDATA case"
    );
    let asserts = win.matches("assert").count();
    assert!(
        asserts >= 3,
        "only {asserts} assert calls in the Windows branch"
    );
}

// ---------------------------------------------------------------- AC4 audit

/// (program, apt package that ships it). Packages must be on the check apt
/// line; the runner image preinstalls some of them but the audit does not
/// rely on that.
const TOOLS: [(&str, &str); 17] = [
    ("ffmpeg", "ffmpeg"),
    ("ffprobe", "ffmpeg"),
    ("gs", "ghostscript"),
    ("minisign", "minisign"),
    ("jq", "jq"),
    ("rpmbuild", "rpm"),
    ("shellcheck", "shellcheck"),
    ("wixl", "msitools"),
    ("msiinfo", "msitools"),
    ("zip", "zip"),
    ("unzip", "unzip"),
    ("file", "file"),
    ("python3", "python3-yaml"),
    ("dpkg-deb", "dpkg"),
    ("objdump", "binutils"),
    ("inkscape", "inkscape"),
    ("convert", "imagemagick"),
];

/// Tools that are always there on ubuntu-24.04 (coreutils and friends).
const BASE: [&str; 14] = [
    "bash",
    "sh",
    "id",
    "git",
    "grep",
    "sed",
    "awk",
    "curl",
    "tar",
    "sha256sum",
    "timeout",
    "mktemp",
    "diff",
    "cmp",
];

/// Tools a test skips itself for, by design, with a printed SKIP line, when
/// they are absent (the office engine tests, 7z in the thumbnail test).
const SKIP_BY_DESIGN: [&str; 4] = ["soffice", "libreoffice", "7z", "reg.exe"];

/// Plausible external programs, none of them an English word, searched for
/// in the release scripts so a new one cannot slip in without a row above.
const CANDIDATES: [&str; 22] = [
    "minisign",
    "jq",
    "rpmbuild",
    "shellcheck",
    "wixl",
    "msiinfo",
    "unzip",
    "dpkg-deb",
    "objdump",
    "inkscape",
    "xmllint",
    "openssl",
    "gpg",
    "7z",
    "cpio",
    "rpm2cpio",
    "bsdtar",
    "xxd",
    "qpdf",
    "yq",
    "msiextract",
    "gcab",
];

fn tokens(line: &str) -> Vec<String> {
    line.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.'))
        .filter(|t| !t.is_empty())
        .map(String::from)
        .collect()
}

#[test]
fn ac4_every_tool_the_tests_need_is_installed_by_ci_or_skips_by_design() {
    let pkgs = check_apt_packages();
    for (prog, pkg) in TOOLS {
        assert!(
            pkgs.contains(pkg),
            "ci.yml apt line lacks `{pkg}` (needed for `{prog}`): {pkgs:?}"
        );
    }
    let known = |t: &str| {
        TOOLS.iter().any(|(p, _)| *p == t) || BASE.contains(&t) || SKIP_BY_DESIGN.contains(&t)
    };
    // Release scripts.
    let dir = root().join("tests/release");
    let mut seen = Vec::new();
    for e in std::fs::read_dir(&dir).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_none_or(|x| x != "sh") {
            continue;
        }
        let text = std::fs::read_to_string(&p).unwrap();
        for l in text.lines().filter(|l| !l.trim_start().starts_with('#')) {
            for t in tokens(l) {
                if CANDIDATES.contains(&t.as_str()) && !known(&t) {
                    seen.push(format!("{}: {t}", p.display()));
                }
            }
        }
    }
    assert!(
        seen.is_empty(),
        "release tests use programs with no audit row: {seen:?}"
    );
    // Rust integration tests: every literal Command::new("x").
    let mut unknown = Vec::new();
    for krate in std::fs::read_dir(root().join("crates")).unwrap() {
        let tests = krate.unwrap().path().join("tests");
        let Ok(rd) = std::fs::read_dir(&tests) else {
            continue;
        };
        for e in rd {
            let p = e.unwrap().path();
            if p.extension().is_none_or(|x| x != "rs") {
                continue;
            }
            let text = std::fs::read_to_string(&p).unwrap();
            for part in text.split("Command::new(\"").skip(1) {
                let name = part.split('"').next().unwrap();
                // Absolute paths and source fragments inside string literals
                // are not program names; this file quotes the pattern itself.
                let plain = !name.is_empty()
                    && name
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c));
                if p.file_name().is_some_and(|f| f == "s058_ci.rs") || !plain {
                    continue;
                }
                if !known(name) {
                    unknown.push(format!("{}: {name}", p.display()));
                }
            }
        }
    }
    assert!(
        unknown.is_empty(),
        "Rust tests run programs with no audit row: {unknown:?}"
    );
}

#[test]
fn ac4_the_by_design_skip_of_the_office_tests_is_printed() {
    // The office engine tests are the model: absent tool, printed skipping line.
    let src = read("crates/engines/tests/office_engine.rs");
    assert!(
        src.contains("not installed, skipping"),
        "the office tests must print a skipping line when LibreOffice is absent"
    );
}

// ---------------------------------------------------------------- AC7

struct Fake {
    dir: tempfile::TempDir,
    bin: tempfile::TempDir,
}

const FAKE_GH: &str = r#"#!/usr/bin/env bash
dir="${FAKE_GH_DIR:?}"
echo "$*" >> "$dir/calls.log"
[ -f "$dir/start" ] || date +%s > "$dir/start"
phase=""
if [ -f "$dir/flip_after" ]; then
  now=$(date +%s); s=$(cat "$dir/start")
  if [ $((now - s)) -ge "$(cat "$dir/flip_after")" ]; then phase=".done"; fi
fi
jqexpr=""
args=("$@")
for ((i = 0; i < ${#args[@]}; i++)); do
  case "${args[i]}" in --jq | -q) jqexpr="${args[i + 1]}" ;; esac
done
case "$1 $2" in
  "run list") base="list" ;;
  "run view") base="view" ;;
  "auth status") exit 0 ;;
  *) echo "fake gh: unsupported call: $*" >&2; exit 64 ;;
esac
f="$dir/$base$phase.json"
[ -f "$f" ] || f="$dir/$base.json"
if [ -n "$jqexpr" ]; then jq -r "$jqexpr" "$f"; else cat "$f"; fi
"#;

impl Fake {
    fn new() -> Self {
        let f = Fake {
            dir: tempfile::tempdir().unwrap(),
            bin: tempfile::tempdir().unwrap(),
        };
        let gh = f.bin.path().join("gh");
        std::fs::write(&gh, FAKE_GH).unwrap();
        std::fs::set_permissions(&gh, std::fs::Permissions::from_mode(0o755)).unwrap();
        f
    }

    fn set(&self, name: &str, json: &str) {
        std::fs::write(self.dir.path().join(name), json).unwrap();
    }

    fn state(&self, suffix: &str, status: &str, conclusion: &str, jobs: &[(&str, &str, &str)]) {
        let run = |c: &str| {
            if c == "null" {
                "null".to_string()
            } else {
                format!("\"{c}\"")
            }
        };
        self.set(
            &format!("list{suffix}.json"),
            &format!(
                r#"[{{"databaseId":4242,"status":"{status}","conclusion":{},"workflowName":"CI","name":"CI"}}]"#,
                run(conclusion)
            ),
        );
        let js: Vec<String> = jobs
            .iter()
            .map(|(n, s, c)| format!(r#"{{"name":"{n}","status":"{s}","conclusion":{}}}"#, run(c)))
            .collect();
        self.set(
            &format!("view{suffix}.json"),
            &format!(
                r#"{{"databaseId":4242,"status":"{status}","conclusion":{},"jobs":[{}]}}"#,
                run(conclusion),
                js.join(",")
            ),
        );
    }

    fn run(&self, args: &[&str], envs: &[(&str, &str)]) -> Output {
        let orig = std::env::var("PATH").unwrap();
        let path = format!("{}:{orig}", self.bin.path().display());
        let mut c = Command::new("/usr/bin/bash");
        c.arg(root().join("scripts/ci-status.sh"))
            .args(args)
            .current_dir(root())
            .env("PATH", path)
            .env("FAKE_GH_DIR", self.dir.path());
        for (k, v) in envs {
            c.env(k, v);
        }
        c.output().unwrap()
    }

    fn calls(&self) -> String {
        std::fs::read_to_string(self.dir.path().join("calls.log")).unwrap_or_default()
    }
}

const ALL_JOBS_OK: [(&str, &str, &str); 3] = [
    ("check", "completed", "success"),
    ("windows-cross", "completed", "success"),
    ("windows-native", "completed", "success"),
];

#[test]
fn ac7_all_success_prints_the_exact_four_lines_and_exits_0() {
    let f = Fake::new();
    f.state("", "completed", "success", &ALL_JOBS_OK);
    let o = f.run(&[], &[]);
    let (out, err) = out_text(&o);
    assert_eq!(
        out, "run 4242 success\ncheck success\nwindows-cross success\nwindows-native success\n",
        "stderr: {err}"
    );
    assert_eq!(o.status.code(), Some(0), "{err}");
}

#[test]
fn ac7_it_asks_for_the_ci_workflow_run_of_head() {
    let f = Fake::new();
    f.state("", "completed", "success", &ALL_JOBS_OK);
    f.run(&[], &[]);
    let head = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root())
        .output()
        .unwrap();
    let head = String::from_utf8(head.stdout).unwrap().trim().to_string();
    let calls = f.calls();
    assert!(
        calls.contains(&head),
        "HEAD {head} not in gh calls:\n{calls}"
    );
    assert!(
        calls.contains("CI"),
        "workflow CI not in gh calls:\n{calls}"
    );
    assert!(
        calls.contains("4242"),
        "run id not used for the jobs:\n{calls}"
    );
    assert!(
        calls
            .lines()
            .all(|l| l.starts_with("run ") || l.starts_with("auth ")),
        "only `gh run ...` (and auth status) may be used:\n{calls}"
    );
}

#[test]
fn ac7_an_explicit_sha_replaces_head() {
    let f = Fake::new();
    f.state("", "completed", "success", &ALL_JOBS_OK);
    let sha = "0123456789abcdef0123456789abcdef01234567";
    let o = f.run(&[sha], &[]);
    assert_eq!(o.status.code(), Some(0), "{:?}", out_text(&o));
    assert!(f.calls().contains(sha));
}

#[test]
fn ac7_a_failed_job_exits_1_and_its_line_says_failure() {
    let f = Fake::new();
    f.state(
        "",
        "completed",
        "failure",
        &[
            ("check", "completed", "success"),
            ("windows-cross", "completed", "success"),
            ("windows-native", "completed", "failure"),
        ],
    );
    let o = f.run(&[], &[]);
    let (out, err) = out_text(&o);
    assert_eq!(
        out, "run 4242 failure\ncheck success\nwindows-cross success\nwindows-native failure\n",
        "{err}"
    );
    assert_eq!(o.status.code(), Some(1));
}

#[test]
fn ac7_a_failed_job_in_a_run_without_conclusion_still_exits_1() {
    let f = Fake::new();
    f.state(
        "",
        "in_progress",
        "null",
        &[
            ("check", "completed", "failure"),
            ("windows-native", "in_progress", "null"),
        ],
    );
    let o = f.run(&[], &[]);
    let (out, _) = out_text(&o);
    assert_eq!(
        out,
        "run 4242 in_progress\ncheck failure\nwindows-native in_progress\n"
    );
    assert_eq!(o.status.code(), Some(1));
}

#[test]
fn ac7_an_in_progress_run_exits_1_with_status_for_empty_or_null_conclusions() {
    for empty in ["", "null"] {
        let f = Fake::new();
        f.state(
            "",
            "in_progress",
            empty,
            &[
                ("check", "completed", "success"),
                ("windows-cross", "in_progress", empty),
                ("windows-native", "queued", empty),
            ],
        );
        let o = f.run(&[], &[]);
        let (out, err) = out_text(&o);
        assert_eq!(
            out,
            "run 4242 in_progress\ncheck success\nwindows-cross in_progress\nwindows-native queued\n",
            "conclusion {empty:?}: {err}"
        );
        assert_eq!(o.status.code(), Some(1), "conclusion {empty:?}");
    }
}

#[test]
fn ac7_a_cancelled_or_skipped_job_is_not_success() {
    let f = Fake::new();
    f.state(
        "",
        "completed",
        "cancelled",
        &[
            ("check", "completed", "success"),
            ("windows-native", "completed", "cancelled"),
        ],
    );
    let o = f.run(&[], &[]);
    assert_eq!(o.status.code(), Some(1), "{:?}", out_text(&o));
}

#[test]
fn ac7_no_run_for_the_commit_exits_2_with_a_message() {
    let f = Fake::new();
    f.set("list.json", "[]");
    let o = f.run(&[], &[]);
    let (out, err) = out_text(&o);
    assert_eq!(o.status.code(), Some(2), "out: {out} err: {err}");
    assert!(!err.trim().is_empty(), "a message on stderr is required");
    assert!(!out.starts_with("run "), "no run line without a run: {out}");
}

#[test]
fn ac7_missing_gh_exits_2_with_a_message() {
    // PATH holds every program of /usr/bin except gh.
    let dir = tempfile::tempdir().unwrap();
    for e in std::fs::read_dir("/usr/bin").unwrap() {
        let e = e.unwrap();
        if e.file_name() != "gh" {
            let _ = symlink(e.path(), dir.path().join(e.file_name()));
        }
    }
    let o = Command::new("/usr/bin/bash")
        .arg(root().join("scripts/ci-status.sh"))
        .current_dir(root())
        .env("PATH", dir.path())
        .output()
        .unwrap();
    let (_, err) = out_text(&o);
    assert_eq!(o.status.code(), Some(2), "{err}");
    assert!(err.contains("gh"), "the message must name gh: {err}");
}

#[test]
fn ac7_wait_polls_until_the_run_finishes_then_prints_only_the_final_state() {
    let f = Fake::new();
    f.state(
        "",
        "in_progress",
        "null",
        &[
            ("check", "completed", "success"),
            ("windows-cross", "in_progress", "null"),
            ("windows-native", "queued", "null"),
        ],
    );
    f.state(".done", "completed", "success", &ALL_JOBS_OK);
    f.set("flip_after", "2");
    let t = Instant::now();
    let o = f.run(
        &["--wait"],
        &[
            ("CI_STATUS_POLL_SECONDS", "1"),
            ("CI_STATUS_MAX_WAIT_SECONDS", "30"),
        ],
    );
    let took = t.elapsed();
    let (out, err) = out_text(&o);
    assert_eq!(
        out, "run 4242 success\ncheck success\nwindows-cross success\nwindows-native success\n",
        "{err}"
    );
    assert_eq!(o.status.code(), Some(0), "{err}");
    assert!(
        took >= Duration::from_secs(1),
        "it must have waited: {took:?}"
    );
    assert!(took < Duration::from_secs(20), "polling too slow: {took:?}");
    let polls = f
        .calls()
        .lines()
        .filter(|l| l.starts_with("run view"))
        .count();
    assert!(polls >= 2, "expected several polls, got {polls}");
}

#[test]
fn ac7_wait_returns_at_once_for_a_finished_failed_run_with_exit_1() {
    let f = Fake::new();
    f.state(
        "",
        "completed",
        "failure",
        &[("check", "completed", "failure")],
    );
    let t = Instant::now();
    let o = f.run(
        &["--wait"],
        &[
            ("CI_STATUS_POLL_SECONDS", "5"),
            ("CI_STATUS_MAX_WAIT_SECONDS", "30"),
        ],
    );
    assert!(
        t.elapsed() < Duration::from_secs(4),
        "a finished run needs no polling"
    );
    let (out, _) = out_text(&o);
    assert_eq!(out, "run 4242 failure\ncheck failure\n");
    assert_eq!(o.status.code(), Some(1));
}

#[test]
fn ac7_wait_gives_up_after_the_limit_with_exit_1() {
    let f = Fake::new();
    f.state(
        "",
        "in_progress",
        "null",
        &[("check", "in_progress", "null")],
    );
    let t = Instant::now();
    let o = f.run(
        &["--wait"],
        &[
            ("CI_STATUS_POLL_SECONDS", "1"),
            ("CI_STATUS_MAX_WAIT_SECONDS", "3"),
        ],
    );
    let took = t.elapsed();
    let (out, _) = out_text(&o);
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(out, "run 4242 in_progress\ncheck in_progress\n");
    assert!(took >= Duration::from_secs(2), "{took:?}");
    assert!(took < Duration::from_secs(15), "{took:?}");
}

#[test]
fn ac7_script_is_gh_only_executable_shellcheck_clean_and_documents_its_defaults() {
    let path = root().join("scripts/ci-status.sh");
    let src = read("scripts/ci-status.sh");
    assert!(src.starts_with("#!/usr/bin/env bash"));
    let mode = std::fs::metadata(&path).unwrap().permissions().mode();
    assert!(mode & 0o111 != 0, "scripts/ci-status.sh must be executable");
    let lower = src.to_lowercase();
    for bad in ["token", "secret", "gh_token"] {
        assert_eq!(lower.matches(bad).count(), 0, "`{bad}` must not appear");
    }
    assert!(src.contains("CI_STATUS_POLL_SECONDS"));
    assert!(src.contains("CI_STATUS_MAX_WAIT_SECONDS"));
    assert!(src.contains("20"), "default poll interval 20 s");
    assert!(src.contains("1200"), "default limit 20 min");
    assert!(!src.contains('\u{2014}') && !src.contains('\u{2013}'));
    let o = Command::new("shellcheck").arg(&path).output().unwrap();
    assert_eq!(o.status.code(), Some(0), "{:?}", out_text(&o));
}

// ---------------------------------------------------------------- AC8

/// The commit this story started from; CLAUDE.md is compared against it.
const BASE_COMMIT: &str = "82a8ffb";

fn working_rules(text: &str) -> Vec<String> {
    let mut on = false;
    let mut out = Vec::new();
    for l in text.lines() {
        if l.starts_with("## ") {
            on = l == "## Working rules";
            continue;
        }
        if on {
            out.push(l.to_string());
        }
    }
    out
}

#[test]
fn ac8_claude_md_gains_one_or_two_lines_in_working_rules_and_loses_none() {
    let now = read("CLAUDE.md");
    let base = Command::new("git")
        .args(["show", &format!("{BASE_COMMIT}:CLAUDE.md")])
        .current_dir(root())
        .output()
        .unwrap();
    assert!(
        base.status.success(),
        "base commit {BASE_COMMIT} must exist"
    );
    let base = String::from_utf8(base.stdout).unwrap();
    let (old_sec, new_sec) = (working_rules(&base), working_rules(&now));
    assert!(!old_sec.is_empty(), "base has a Working rules section");
    // No removed line: every base line is still there, in order.
    let mut it = new_sec.iter();
    for l in &old_sec {
        assert!(it.any(|n| n == l), "base line removed or moved: {l}");
    }
    let added: Vec<&String> = new_sec.iter().filter(|l| !old_sec.contains(l)).collect();
    assert!(
        (1..=2).contains(&added.len()),
        "expected 1 or 2 added lines in Working rules, got {added:?}"
    );
    let joined = added
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(joined.contains("scripts/ci-status.sh"), "{joined}");
    assert!(joined.contains("blocks the next story"), "{joined}");
    assert!(!joined.contains('\u{2014}') && !joined.contains('\u{2013}'));
}
