//! S-085 (backlog/S-085-windows-app-identity.md): the Windows app identity,
//! tests from the acceptance criteria and manager decisions D1 to D9 only.
//!
//! The pure API pinned here (D3), all in `kawaro_core::identity`:
//! - `DISPLAY_NAME: &str`, the fixed brand string "Kawaro | كوارو" (D8);
//! - `MSI_SHORTCUT_FILE: &str`, the `.lnk` the MSI installs, "Kawaro.lnk";
//! - `SHORTCUT_FILE: &str`, the bilingual `.lnk` name, "Kawaro ｜ كوارو.lnk"
//!   with U+FF5C FULLWIDTH VERTICAL LINE, since `|` is reserved in Win32
//!   file names (D10);
//! - `AUMID_KEY: &str`, the HKCU subkey of the AUMID, without the `HKCU\`
//!   prefix, derived from `appid::APP_ID`;
//! - `shortcut_rename(programs: &Path, exists: impl Fn(&Path) -> bool)
//!   -> Option<(PathBuf, PathBuf)>`, the idempotent rename plan (from, to);
//! - `reg_sz(value: &str) -> Vec<u8>`, the REG_SZ data bytes: UTF-16LE with
//!   one trailing NUL code unit;
//! - `sync()`, the entry point on every platform, a no-op off Windows.
//!
//! The docs, CHANGELOG and source checks below read repository files in the
//! s048_sources.rs style (comment lines filtered for source checks).

use kawaro_core::appid::APP_ID;
use kawaro_core::i18n::Lang;
use kawaro_core::identity::{
    self, AUMID_KEY, DISPLAY_NAME, MSI_SHORTCUT_FILE, SHORTCUT_FILE, shortcut_rename,
};
use kawaro_core::registry::install_commands;
use std::cell::RefCell;
use std::path::{Path, PathBuf};

const BRAND: &str = "Kawaro | كوارو";
const UPGRADE_CODE: &str = "6f2d5c1e-8a47-4b39-9d0e-5b1c7a3e2f84";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    let path = root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} must exist: {e}", path.display()))
}

fn code_lines(s: &str) -> String {
    s.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn has_arabic(s: &str) -> bool {
    s.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c))
}

// ---- D8: the one fixed brand string ----

#[test]
fn d8_display_name_is_the_bilingual_brand_latin_first() {
    assert_eq!(DISPLAY_NAME, BRAND);
    assert!(DISPLAY_NAME.starts_with("Kawaro"), "Latin part first (AC4)");
    assert!(DISPLAY_NAME.ends_with("كوارو"));
}

#[test]
fn d1_shortcut_file_names() {
    assert_eq!(MSI_SHORTCUT_FILE, "Kawaro.lnk");
    // D10: the file name uses U+FF5C FULLWIDTH VERTICAL LINE, not the ASCII
    // bar of DISPLAY_NAME, which Windows rejects in a file name.
    assert_eq!(SHORTCUT_FILE, "Kawaro \u{FF5C} كوارو.lnk");
    assert_eq!(SHORTCUT_FILE, "Kawaro ｜ كوارو.lnk");
}

/// D10: the old D1 pin `Kawaro | كوارو.lnk` could never exist on Windows,
/// because `|` is a reserved Win32 file name character. This is why D10
/// replaced the ASCII bar with U+FF5C in the file name only; this guard keeps
/// every pinned shortcut file name a valid Win32 name.
#[test]
fn d10_shortcut_file_names_have_no_reserved_win32_characters() {
    const RESERVED: [char; 9] = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    for name in [MSI_SHORTCUT_FILE, SHORTCUT_FILE] {
        for c in name.chars() {
            assert!(
                !RESERVED.contains(&c),
                "{name:?} contains the reserved Win32 character {c:?}"
            );
            assert!(
                !c.is_control(),
                "{name:?} contains the control character {c:?}"
            );
        }
    }
}

// ---- AC2, D1, D3: the idempotent Start shortcut rename plan ----

const PROGRAMS: &str = r"C:\Users\u\AppData\Roaming\Microsoft\Windows\Start Menu\Programs";

/// Runs the plan with the files in `present` existing; returns the plan and
/// every path the probe was asked about.
fn plan(present: &[PathBuf]) -> (Option<(PathBuf, PathBuf)>, Vec<PathBuf>) {
    let asked = RefCell::new(Vec::new());
    let got = shortcut_rename(Path::new(PROGRAMS), |p: &Path| {
        asked.borrow_mut().push(p.to_path_buf());
        present.iter().any(|f| f == p)
    });
    (got, asked.into_inner())
}

fn ascii_lnk() -> PathBuf {
    Path::new(PROGRAMS).join(MSI_SHORTCUT_FILE)
}

fn bilingual_lnk() -> PathBuf {
    Path::new(PROGRAMS).join(SHORTCUT_FILE)
}

#[test]
fn ac2_renames_the_msi_shortcut_to_the_bilingual_name() {
    let (got, _) = plan(&[ascii_lnk()]);
    assert_eq!(got, Some((ascii_lnk(), bilingual_lnk())));
    let (from, to) = got.unwrap();
    assert_eq!(from.file_name().unwrap(), "Kawaro.lnk");
    assert_eq!(to.file_name().unwrap(), "Kawaro ｜ كوارو.lnk");
    assert_eq!(to.parent().unwrap(), Path::new(PROGRAMS));
}

#[test]
fn ac2_no_rename_when_already_renamed() {
    let (got, asked) = plan(&[bilingual_lnk()]);
    assert_eq!(
        got, None,
        "idempotent: the source is gone after the first run"
    );
    assert!(
        asked.contains(&ascii_lnk()),
        "the plan probes the source {asked:?}"
    );
}

#[test]
fn ac2_no_rename_when_the_target_already_exists() {
    let (got, asked) = plan(&[ascii_lnk(), bilingual_lnk()]);
    assert_eq!(got, None, "never overwrite an existing bilingual shortcut");
    assert!(
        asked.contains(&bilingual_lnk()),
        "the plan probes the target {asked:?}"
    );
}

#[test]
fn ac2_no_rename_without_any_shortcut() {
    let (got, _) = plan(&[]);
    assert_eq!(got, None, "portable installs have no Start shortcut");
}

#[test]
fn ac2_the_plan_only_probes_inside_the_programs_folder() {
    let (_, asked) = plan(&[ascii_lnk()]);
    assert!(!asked.is_empty());
    for p in &asked {
        assert_eq!(p.parent().unwrap(), Path::new(PROGRAMS), "{p:?}");
    }
}

// ---- AC2: the AUMID DisplayName registry bytes ----

#[test]
fn ac2_aumid_key_is_derived_from_the_app_id() {
    assert_eq!(
        AUMID_KEY,
        format!(r"Software\Classes\AppUserModelId\{APP_ID}")
    );
    assert_eq!(
        AUMID_KEY,
        r"Software\Classes\AppUserModelId\io.github.m7mmad_omar.Kawaro"
    );
}

#[test]
fn ac2_display_name_reg_sz_bytes_are_utf16le_with_one_nul() {
    let bytes = identity::reg_sz(DISPLAY_NAME);
    assert_eq!(bytes.len() % 2, 0);
    let units: Vec<u16> = (0..bytes.len() / 2)
        .map(|i| u16::from_le_bytes([bytes[2 * i], bytes[2 * i + 1]]))
        .collect();
    // K a w a r o, space, |, space, ك و ا ر و, NUL.
    let want: Vec<u16> = vec![
        0x004B, 0x0061, 0x0077, 0x0061, 0x0072, 0x006F, 0x0020, 0x007C, 0x0020, 0x0643, 0x0648,
        0x0627, 0x0631, 0x0648, 0x0000,
    ];
    assert_eq!(units, want);
    assert_eq!(&bytes[..4], &[0x4B, 0x00, 0x61, 0x00], "little endian");
    assert_eq!(&bytes[bytes.len() - 2..], &[0x00, 0x00], "one trailing NUL");
}

#[test]
fn ac2_reg_sz_of_an_empty_string_is_one_nul() {
    assert_eq!(identity::reg_sz(""), vec![0x00, 0x00]);
}

#[cfg(not(windows))]
#[test]
fn d3_sync_is_a_callable_noop_off_windows() {
    identity::sync();
    identity::sync();
}

// ---- AC3, AC4: install_commands ----

fn display_name(lang: Lang) -> String {
    let commands = install_commands(r"C:\k", lang).unwrap();
    let hits: Vec<&Vec<String>> = commands
        .iter()
        .filter(|c| c[1] == format!(r"HKCU\{AUMID_KEY}") && c[3] == "DisplayName")
        .collect();
    assert_eq!(hits.len(), 1, "one DisplayName value");
    let at = hits[0].iter().position(|a| a == "/d").unwrap() + 1;
    hits[0][at].clone()
}

fn menu_verb(lang: Lang) -> String {
    install_commands(r"C:\k", lang).unwrap()[0][7].clone()
}

#[test]
fn ac3_install_commands_writes_the_bilingual_display_name_in_both_languages() {
    assert_eq!(display_name(Lang::En), BRAND);
    assert_eq!(display_name(Lang::Ar), BRAND);
}

#[test]
fn ac4_menu_verb_stays_localized() {
    assert_eq!(menu_verb(Lang::En), "Kawaro");
    assert_eq!(menu_verb(Lang::Ar), "كوارو");
}

// ---- AC3: the UpgradeCode pin ----

#[test]
fn ac3_upgrade_code_is_unchanged_byte_for_byte() {
    let wxs = read("installer/windows/kawaro.wxs");
    let codes: Vec<&str> = wxs
        .match_indices("UpgradeCode=\"")
        .map(|(i, m)| {
            let start = i + m.len();
            let end = start + wxs[start..].find('"').unwrap();
            &wxs[start..end]
        })
        .collect();
    assert_eq!(codes, vec![UPGRADE_CODE]);
}

// ---- AC4, D8: the policy line in the user docs ----
//
// Expected wording (any phrasing that keeps these words): the English line
// names "Kawaro | كوارو" and says the Latin part comes first, for example
// "The app name is the fixed brand string Kawaro | كوارو: the Latin part
// comes first in every language and it is never reordered." The Arabic line
// names "Kawaro | كوارو" and the Latin part ("اللاتيني" or "اللاتينية"),
// for example "اسم التطبيق ثابت: Kawaro | كوارو، الجزء اللاتيني أولا في كل
// اللغات ولا يُعاد ترتيبه."

fn policy_lines<'a>(text: &'a str, words: &[&str]) -> Vec<&'a str> {
    text.lines()
        .filter(|l| l.contains(BRAND) && words.iter().all(|w| l.to_lowercase().contains(w)))
        .collect()
}

#[test]
fn ac4_english_install_guide_has_one_policy_line() {
    let doc = read("docs/guide/install.md");
    let lines = policy_lines(&doc, &["latin", "first"]);
    assert_eq!(
        lines.len(),
        1,
        "one policy line in docs/guide/install.md: {lines:?}"
    );
}

#[test]
fn ac4_arabic_install_guide_has_one_policy_line_in_arabic() {
    let doc = read("docs/guide/install.ar.md");
    let lines = policy_lines(&doc, &["لاتيني"]);
    assert_eq!(
        lines.len(),
        1,
        "one policy line in docs/guide/install.ar.md: {lines:?}"
    );
    let rest = lines[0].replace(BRAND, "");
    assert!(
        has_arabic(&rest),
        "the Arabic line is written in Arabic: {}",
        lines[0]
    );
}

// ---- AC2, AC3, D8: the owner checklist ----

/// The text of the step `W<id>` up to the next step heading.
fn step(text: &str, id: &str) -> String {
    let mut out = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        let is_step = line.starts_with('W')
            && line
                .get(1..3)
                .is_some_and(|d| d.chars().all(|c| c.is_ascii_digit()))
            && line.get(3..).is_some_and(|r| r.starts_with(' '));
        if is_step {
            inside = line.starts_with(&format!("{id} "));
        }
        if inside {
            out.push(line);
        }
    }
    out.join("\n")
}

#[test]
fn ac2_checklist_w01_owns_the_start_icon_and_the_bilingual_name() {
    let w01 = step(&read("docs/windows-beta-checklist.md"), "W01");
    assert!(!w01.is_empty(), "W01 exists");
    let expect = w01
        .lines()
        .find(|l| l.starts_with("Expect:"))
        .expect("W01 has an Expect line");
    assert!(expect.contains(BRAND), "W01 Expect names {BRAND}: {expect}");
    assert!(
        expect.to_lowercase().contains("icon"),
        "W01 Expect names the icon: {expect}"
    );
    assert!(
        expect.contains("Start"),
        "W01 Expect names the Start menu: {expect}"
    );
}

#[test]
fn ac3_checklist_keeps_its_msi_upgrade_step() {
    let cl = read("docs/windows-beta-checklist.md");
    let hits: Vec<&str> = cl
        .lines()
        .filter(|l| {
            l.starts_with("Do:") && l.contains("newer MSI") && l.to_lowercase().contains("upgrade")
        })
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "one step installs the newer MSI as an upgrade: {hits:?}"
    );
}

// ---- AC5, D9: licence note and CHANGELOG ----

#[test]
fn ac5_engines_doc_records_the_resource_build_dependency_licence() {
    let doc = read("docs/engines.md");
    let lines: Vec<&str> = doc
        .lines()
        .filter(|l| l.contains("winresource") && l.contains("MIT"))
        .collect();
    assert!(
        !lines.is_empty(),
        "docs/engines.md names winresource and MIT"
    );
}

#[test]
fn ac5_changelog_has_one_bilingual_s085_line() {
    let log = read("CHANGELOG.md");
    let lines: Vec<&str> = log.lines().filter(|l| l.contains("S-085")).collect();
    assert_eq!(
        lines.len(),
        1,
        "exactly one CHANGELOG line for S-085: {lines:?}"
    );
    assert!(has_arabic(lines[0]), "the S-085 line has Arabic text");
    assert!(
        lines[0].contains("[S-085](https://github.com/M7MMAD-OMAR/kawaro)"),
        "D9 markdown link form"
    );
}

// ---- AC2, D3: call sites of the identity sync ----

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let p = entry.path();
        if p.is_dir() {
            rs_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn d3_daemon_startup_calls_the_identity_sync() {
    let mut files = Vec::new();
    rs_files(&root().join("crates/daemon/src"), &mut files);
    let callers: Vec<&PathBuf> = files
        .iter()
        .filter(|p| code_lines(&std::fs::read_to_string(p).unwrap()).contains("identity::sync("))
        .collect();
    assert!(
        !callers.is_empty(),
        "crates/daemon/src calls identity::sync()"
    );
}

#[test]
fn d3_cli_windows_install_calls_the_identity_sync() {
    let s = code_lines(&read("crates/cli/src/integration/windows.rs"));
    assert!(
        s.contains("identity::sync("),
        "kawaro integration install runs the sync"
    );
}

// ---- AC1, D4: the windows-cross CI job runs the resource check ----

/// The lines of one top level job of `.github/workflows/ci.yml`.
fn ci_job(name: &str) -> Vec<String> {
    let ci = read(".github/workflows/ci.yml");
    let mut out = Vec::new();
    let mut inside = false;
    for line in ci.lines() {
        let is_job = line.starts_with("  ") && !line.starts_with("   ") && line.ends_with(':');
        if is_job {
            inside = line.trim() == format!("{name}:");
        }
        if inside {
            out.push(line.to_string());
        }
    }
    out
}

#[test]
fn d4_windows_cross_runs_test_s085_after_the_build() {
    let job = ci_job("windows-cross");
    assert!(!job.is_empty(), "ci.yml has a windows-cross job");
    let build = job
        .iter()
        .position(|l| l.contains("scripts/windows/build.sh"))
        .expect("windows-cross runs scripts/windows/build.sh");
    let check = job
        .iter()
        .position(|l| l.contains("tests/windows/test_s085.sh"))
        .expect("windows-cross runs tests/windows/test_s085.sh (D4)");
    assert!(check > build, "test_s085.sh runs after the build (D4)");
}
