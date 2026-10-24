//! S-051 AC5: the Registry table of the per-user MSI against the S-050
//! `install_commands` output, so the two sources cannot drift.
//!
//! Needs a built MSI, so it is ignored in a plain `cargo test` (CI has no
//! msitools). `tests/windows/test_s051.sh` builds the MSI with
//! `scripts/windows/make-msi.sh`, runs `msiinfo export <msi> Registry` into a
//! file and runs this test with `--ignored`, passing that file in
//! `S051_REGISTRY_EXPORT`. The test panics when the variable is unset.
//!
//! Test side mappings (not changes to the acceptance criteria):
//! - the `HKCU\` prefix of a `reg.exe` key becomes Root `1` and is cut off;
//! - `/ve` (the default value) becomes an empty Name column;
//! - `[INSTALLDIR]\` becomes `[INSTALLDIR]`, because the MSI property already
//!   ends with a backslash (the literal of AC5 is `[INSTALLDIR]sbarhived.exe`).
//!
//! The AUMID key is taken from `install_commands`, not written here, so the
//! test does not depend on `APP_ID` (S-049) and cannot drift from it.

use sbarhive_core::i18n::Lang;
use sbarhive_core::registry::install_commands;

type Row = Vec<String>;

fn registry_rows() -> Vec<Row> {
    let path = std::env::var("S051_REGISTRY_EXPORT")
        .expect("S051_REGISTRY_EXPORT must name a file made by `msiinfo export <msi> Registry`");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let rows: Vec<Row> = text
        .lines()
        .skip(3)
        .filter(|l| !l.is_empty())
        .map(|l| l.split('\t').map(str::to_string).collect())
        .collect();
    assert!(!rows.is_empty(), "no Registry rows in {path} (AC5)");
    rows
}

/// (key, name, value) of one `reg add` argument list, with the mappings above.
fn expected(args: &[String]) -> (String, String, String) {
    assert_eq!(args[0], "add");
    let key = args[1]
        .strip_prefix("HKCU\\")
        .expect("install_commands writes HKCU only")
        .to_string();
    let name = if args[2] == "/ve" {
        String::new()
    } else {
        assert_eq!(args[2], "/v");
        args[3].clone()
    };
    let at = args.iter().position(|a| a == "/d").expect("/d") + 1;
    (
        key,
        name,
        args[at].replace("[INSTALLDIR]\\", "[INSTALLDIR]"),
    )
}

fn find<'a>(rows: &'a [Row], key: &str, name: &str) -> Vec<&'a Row> {
    rows.iter()
        .filter(|r| r.len() >= 6 && r[2] == key && r[3] == name)
        .collect()
}

#[test]
#[ignore = "needs an MSI export, run by tests/windows/test_s051.sh"]
fn ac5_every_s050_row_is_in_the_msi_registry_table() {
    let rows = registry_rows();
    let commands = install_commands("[INSTALLDIR]", Lang::En).unwrap();
    assert_eq!(commands.len(), 6, "S-050 writes six values");
    for c in &commands {
        let (key, name, value) = expected(c);
        let hits = find(&rows, &key, &name);
        assert_eq!(hits.len(), 1, "exactly one row for {key} [{name}] (AC5)");
        assert_eq!(hits[0][1], "1", "Root of {key} [{name}] is HKCU (AC5)");
        assert_eq!(hits[0][4], value, "value of {key} [{name}] (AC5)");
    }
}

#[test]
#[ignore = "needs an MSI export, run by tests/windows/test_s051.sh"]
fn ac5_the_literal_strings_of_the_acceptance_criterion() {
    let rows = registry_rows();
    let shell = r"Software\Classes\*\shell\SbarHive";
    assert_eq!(find(&rows, shell, "MultiSelectModel")[0][4], "Player");
    assert_eq!(
        find(&rows, &format!(r"{shell}\command"), "")[0][4],
        r#""[INSTALLDIR]sbarhived.exe" open "%1""#
    );
    let aumid = install_commands("[INSTALLDIR]", Lang::En).unwrap()[4][1]
        .strip_prefix("HKCU\\")
        .unwrap()
        .to_string();
    assert!(aumid.starts_with(r"Software\Classes\AppUserModelId\"));
    assert_eq!(find(&rows, &aumid, "DisplayName")[0][4], "SbarHive");
}

#[test]
#[ignore = "needs an MSI export, run by tests/windows/test_s051.sh"]
fn ac5_run_value_starts_the_daemon_at_logon() {
    let rows = registry_rows();
    let hits = find(
        &rows,
        r"Software\Microsoft\Windows\CurrentVersion\Run",
        "SbarHive",
    );
    assert_eq!(hits.len(), 1, "one Run value named SbarHive (AC5)");
    assert_eq!(hits[0][1], "1", "Run value is under HKCU (AC5)");
    assert_eq!(hits[0][4], r#""[INSTALLDIR]sbarhived.exe""#);
}

#[test]
#[ignore = "needs an MSI export, run by tests/windows/test_s051.sh"]
fn ac5_no_row_outside_hkcu() {
    for r in registry_rows() {
        assert_eq!(r[1], "1", "row {r:?} is not under HKCU (per-user install)");
    }
}
