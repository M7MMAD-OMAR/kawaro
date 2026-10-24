//! S-045 AC7 (backlog/S-045-windows-cfg-gates.md): `logfile::open_append`
//! creates missing parent directories and appends. The daemon is a binary
//! crate, so the module file is included by path.

#[allow(dead_code)]
#[path = "../src/logfile.rs"]
mod logfile;

use std::io::Write;

#[test]
fn ac7_two_appends_to_a_nested_missing_path_leave_two_lines() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp
        .path()
        .join("a")
        .join("b")
        .join("c")
        .join("sbarhived.log");
    assert!(!path.parent().unwrap().exists());

    let mut first = logfile::open_append(&path).unwrap();
    writeln!(first, "first line").unwrap();
    drop(first);
    let mut second = logfile::open_append(&path).unwrap();
    writeln!(second, "second line").unwrap();
    drop(second);

    let text = std::fs::read_to_string(&path).unwrap();
    assert_eq!(text.lines().count(), 2, "got: {text:?}");
    assert_eq!(text, "first line\nsecond line\n");
}

#[test]
fn ac7_open_append_keeps_existing_content() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("x.log");
    std::fs::write(&path, "old\n").unwrap();
    let mut f = logfile::open_append(&path).unwrap();
    writeln!(f, "new").unwrap();
    drop(f);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "old\nnew\n");
}
