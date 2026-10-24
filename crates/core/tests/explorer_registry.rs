//! S-050 AC1, AC2, AC3, AC6: the Explorer menu registry commands, built as
//! pure `reg.exe` argument lists. `APP_ID` (S-049) and the icon (S-048) are
//! not available yet, so the AppUserModelId is the literal string of the
//! acceptance criteria and the icon is only a file name inside `dir`.

use sbarhive_core::i18n::Lang;
use sbarhive_core::registry::{install_commands, missing_files, uninstall_commands};
use std::cell::RefCell;

const DIR: &str = r"C:\Users\u\AppData\Local\Programs\SbarHive";
const SHELL: &str = r"HKCU\Software\Classes\*\shell\SbarHive";
const CMD_KEY: &str = r"HKCU\Software\Classes\*\shell\SbarHive\command";
const AUMID: &str = r"HKCU\Software\Classes\AppUserModelId\io.github.m7mmad_omar.SbarHive";

fn v(args: &[&str]) -> Vec<String> {
    args.iter().map(|s| s.to_string()).collect()
}

fn add_named(key: &str, name: &str, data: &str) -> Vec<String> {
    v(&["add", key, "/v", name, "/t", "REG_SZ", "/d", data, "/f"])
}

fn expected(verb: &str, dir: &str) -> Vec<Vec<String>> {
    let ico = format!(r"{dir}\sbarhive.ico");
    vec![
        add_named(SHELL, "MUIVerb", verb),
        add_named(SHELL, "Icon", &ico),
        add_named(SHELL, "MultiSelectModel", "Player"),
        v(&[
            "add",
            CMD_KEY,
            "/ve",
            "/t",
            "REG_SZ",
            "/d",
            &format!(r#""{dir}\sbarhived.exe" open "%1""#),
            "/f",
        ]),
        add_named(AUMID, "DisplayName", "SbarHive"),
        add_named(AUMID, "IconUri", &ico),
    ]
}

#[test]
fn ac1_install_commands_english_exact() {
    let got = install_commands(DIR, Lang::En).unwrap();
    assert_eq!(got.len(), 6);
    assert_eq!(got, expected("SbarHive", DIR));
}

#[test]
fn ac1_install_commands_arabic_exact() {
    let got = install_commands(DIR, Lang::Ar).unwrap();
    assert_eq!(got.len(), 6);
    assert_eq!(got, expected("صبار هايف", DIR));
}

#[test]
fn ac1_only_the_verb_name_differs_between_languages() {
    let en = install_commands(DIR, Lang::En).unwrap();
    let ar = install_commands(DIR, Lang::Ar).unwrap();
    let differing: Vec<usize> = (0..6).filter(|&i| en[i] != ar[i]).collect();
    assert_eq!(differing, vec![0]);
}

#[test]
fn ac1_default_value_command_keeps_its_quotes() {
    let got = install_commands(r"C:\Program Files\SbarHive", Lang::En).unwrap();
    assert_eq!(
        got[3][7],
        r#""C:\Program Files\SbarHive\sbarhived.exe" open "%1""#
    );
    assert_eq!(got[3][2], "/ve");
    assert_eq!(got[3].len(), 9);
}

#[test]
fn ac2_uninstall_commands_exact() {
    assert_eq!(
        uninstall_commands(),
        vec![
            v(&["delete", r"HKCU\Software\Classes\*\shell\SbarHive", "/f"]),
            v(&[
                "delete",
                r"HKCU\Software\Classes\AppUserModelId\io.github.m7mmad_omar.SbarHive",
                "/f"
            ]),
        ]
    );
}

#[test]
fn ac2_every_uninstalled_key_is_a_key_that_install_writes_under() {
    let keys: Vec<String> = install_commands(DIR, Lang::En)
        .unwrap()
        .iter()
        .map(|c| c[1].clone())
        .collect();
    for d in uninstall_commands() {
        assert_eq!(d.len(), 3);
        assert!(
            keys.iter()
                .any(|k| k == &d[1] || k.starts_with(&format!(r"{}\", d[1])))
        );
    }
    for k in &keys {
        assert!(
            uninstall_commands()
                .iter()
                .any(|d| k == &d[1] || k.starts_with(&format!(r"{}\", d[1])))
        );
    }
}

#[test]
fn ac3_quote_in_dir_is_an_error_in_both_languages() {
    for lang in [Lang::En, Lang::Ar] {
        for dir in [r#"C:\a"b"#, "\""] {
            let err = install_commands(dir, lang).unwrap_err();
            assert_eq!(err.to_string(), "path contains a quote");
        }
    }
}

#[test]
fn ac3_quote_error_wins_over_trailing_backslash() {
    let err = install_commands("C:\\a\"b\\", Lang::En).unwrap_err();
    assert_eq!(err.to_string(), "path contains a quote");
}

#[test]
fn ac3_trailing_backslash_is_normalised() {
    let with = install_commands(&format!(r"{DIR}\"), Lang::En).unwrap();
    assert_eq!(with, install_commands(DIR, Lang::En).unwrap());
    assert_eq!(with[3][7], format!(r#""{DIR}\sbarhived.exe" open "%1""#));
    assert!(!with[3][7].contains(r"\\"));
    assert_eq!(with[1][7], format!(r"{DIR}\sbarhive.ico"));
    assert_eq!(with[5][7], format!(r"{DIR}\sbarhive.ico"));
}

fn probe(present: &'static [&'static str]) -> (Vec<String>, Vec<String>) {
    let calls = RefCell::new(Vec::new());
    let missing = missing_files(DIR, |p: &str| {
        calls.borrow_mut().push(p.to_string());
        present.iter().any(|f| p == format!(r"{DIR}\{f}"))
    });
    (missing, calls.into_inner())
}

#[test]
fn ac6_nothing_missing_when_both_exist() {
    let (missing, calls) = probe(&["sbarhived.exe", "sbarhive.ico"]);
    assert_eq!(missing, Vec::<String>::new());
    assert_eq!(
        calls,
        vec![
            format!(r"{DIR}\sbarhived.exe"),
            format!(r"{DIR}\sbarhive.ico")
        ]
    );
}

#[test]
fn ac6_missing_service_exe_is_named() {
    let (missing, _) = probe(&["sbarhive.ico"]);
    assert_eq!(missing, vec!["sbarhived.exe".to_string()]);
}

#[test]
fn ac6_missing_icon_is_named() {
    let (missing, _) = probe(&["sbarhived.exe"]);
    assert_eq!(missing, vec!["sbarhive.ico".to_string()]);
}

#[test]
fn ac6_both_missing_are_named_in_order() {
    let (missing, _) = probe(&[]);
    assert_eq!(
        missing,
        vec!["sbarhived.exe".to_string(), "sbarhive.ico".to_string()]
    );
}

#[test]
fn ac6_trailing_backslash_does_not_double_the_separator() {
    let seen = RefCell::new(Vec::new());
    let _ = missing_files(&format!(r"{DIR}\"), |p: &str| {
        seen.borrow_mut().push(p.to_string());
        true
    });
    assert_eq!(
        seen.into_inner(),
        vec![
            format!(r"{DIR}\sbarhived.exe"),
            format!(r"{DIR}\sbarhive.ico")
        ]
    );
}
