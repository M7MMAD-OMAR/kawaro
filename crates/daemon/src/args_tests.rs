//! S-046 AC5 (backlog/S-046-windows-hive-window.md): the command line.
//!
//! Contract (tester, from AC5; do not edit). In `crate::args`:
//! - `#[derive(Debug, PartialEq)] pub enum Mode { Daemon, Open(Vec<String>) }`.
//! - `pub fn parse(args: &[String]) -> Result<Mode, String>`; `args` are the
//!   arguments after the program name.

use crate::args::{Mode, parse};

fn v(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

#[test]
fn ac5_no_arguments_is_the_daemon() {
    assert_eq!(parse(&[]), Ok(Mode::Daemon));
}

#[test]
fn ac5_open_one_file() {
    assert_eq!(parse(&v(&["open", "a.png"])), Ok(Mode::Open(v(&["a.png"]))));
}

#[test]
fn ac5_open_keeps_the_order_of_many_files() {
    assert_eq!(
        parse(&v(&["open", "a", "b"])),
        Ok(Mode::Open(v(&["a", "b"])))
    );
}

#[test]
fn ac5_open_without_a_file_is_an_error_naming_the_problem() {
    let e = parse(&v(&["open"])).unwrap_err();
    assert!(e.contains("open needs at least one file"), "{e}");
}

#[test]
fn ac5_an_unknown_word_is_an_error() {
    assert!(parse(&v(&["bogus"])).is_err());
}

#[test]
fn ac5_a_file_named_open_after_the_command_is_a_path() {
    assert_eq!(parse(&v(&["open", "open"])), Ok(Mode::Open(v(&["open"]))));
}

#[test]
fn ac5_an_unknown_word_alone_is_an_unknown_argument() {
    assert_eq!(
        parse(&v(&["bogus"])),
        Err("kawarod: unknown argument `bogus`".to_string())
    );
}

#[test]
fn ac5_an_unknown_word_with_files_is_not_open() {
    assert_eq!(
        parse(&v(&["bogus", "x"])),
        Err("kawarod: unknown argument `bogus`".to_string())
    );
}
