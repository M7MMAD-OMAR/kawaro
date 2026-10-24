//! S-050 AC4 and AC5: `sbarhive integration install|uninstall` run as the
//! real binary. Stdout lines and exit codes are compared exactly.

use std::path::Path;
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_sbarhive");
const SHELL: &str = r"HKCU\Software\Classes\*\shell\SbarHive";
const AUMID: &str = r"HKCU\Software\Classes\AppUserModelId\io.github.m7mmad_omar.SbarHive";

fn run(lang: &str, args: &[&str]) -> Output {
    Command::new(BIN)
        .env("SBARHIVE_LANG", lang)
        .args(args)
        .output()
        .unwrap()
}

fn lines(o: &Output) -> Vec<String> {
    String::from_utf8(o.stdout.clone())
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
fn ac4_dry_run_with_dir_prints_six_reg_lines() {
    let o = run(
        "en",
        &["integration", "install", "--dry-run", "--dir", "/x"],
    );
    assert_eq!(o.status.code(), Some(0));
    assert!(
        o.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let l = lines(&o);
    assert_eq!(l.len(), 6);
    assert_eq!(
        l[0],
        format!("reg.exe add {SHELL} /v MUIVerb /t REG_SZ /d SbarHive /f")
    );
    assert_eq!(
        l[1],
        format!(r"reg.exe add {SHELL} /v Icon /t REG_SZ /d /x\sbarhive.ico /f")
    );
    assert_eq!(
        l[2],
        format!("reg.exe add {SHELL} /v MultiSelectModel /t REG_SZ /d Player /f")
    );
    assert_eq!(
        l[4],
        format!("reg.exe add {AUMID} /v DisplayName /t REG_SZ /d SbarHive /f")
    );
    assert_eq!(
        l[5],
        format!(r"reg.exe add {AUMID} /v IconUri /t REG_SZ /d /x\sbarhive.ico /f")
    );
}

#[test]
fn ac4_fourth_line_is_the_command_value() {
    let o = run(
        "en",
        &["integration", "install", "--dry-run", "--dir", "/x"],
    );
    let l = lines(&o);
    assert_eq!(l.len(), 6);
    assert!(l[3].starts_with(&format!(r"reg.exe add {SHELL}\command /ve /t REG_SZ /d ")));
    assert!(l[3].contains(r#"open "%1""#), "{}", l[3]);
    assert!(l[3].contains(r"/x\sbarhived.exe"), "{}", l[3]);
    assert!(l[3].ends_with(" /f"));
}

#[test]
fn ac4_every_line_is_a_reg_exe_add() {
    let o = run(
        "ar",
        &["integration", "install", "--dry-run", "--dir", "/x"],
    );
    assert_eq!(o.status.code(), Some(0));
    let l = lines(&o);
    assert_eq!(l.len(), 6);
    for line in &l {
        assert!(line.starts_with("reg.exe add HKCU"), "{line}");
        assert!(line.ends_with(" /f"), "{line}");
    }
    assert!(l[0].contains("صبار هايف"), "{}", l[0]);
    assert!(!l[0].contains("SbarHive /f"), "{}", l[0]);
}

#[test]
fn ac4_default_dir_is_the_executable_folder() {
    let o = run("en", &["integration", "install", "--dry-run"]);
    assert_eq!(o.status.code(), Some(0));
    let folder = Path::new(BIN).parent().unwrap().display().to_string();
    let l = lines(&o);
    assert_eq!(l.len(), 6);
    assert!(
        l[1].contains(&format!(r"{folder}\sbarhive.ico")),
        "{}",
        l[1]
    );
    assert!(
        l[3].contains(&format!(r"{folder}\sbarhived.exe")),
        "{}",
        l[3]
    );
}

#[cfg(not(windows))]
#[test]
fn ac5_install_without_dry_run_is_windows_only() {
    let o = run("en", &["integration", "install", "--dir", "/x"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(o.stdout.is_empty());
    let err = String::from_utf8(o.stderr).unwrap();
    assert!(
        err.contains("integration install: available on Windows only"),
        "{err}"
    );
}

#[cfg(not(windows))]
#[test]
fn ac5_install_without_dry_run_or_dir_is_windows_only() {
    let o = run("en", &["integration", "install"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(o.stdout.is_empty());
    let err = String::from_utf8(o.stderr).unwrap();
    assert!(
        err.contains("integration install: available on Windows only"),
        "{err}"
    );
}

#[test]
fn ac5_uninstall_dry_run_prints_two_lines() {
    let o = run("en", &["integration", "uninstall", "--dry-run"]);
    assert_eq!(o.status.code(), Some(0));
    assert!(
        o.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    assert_eq!(
        lines(&o),
        vec![
            format!("reg.exe delete {SHELL} /f"),
            format!("reg.exe delete {AUMID} /f"),
        ]
    );
}
