//! S-046 AC5 (backlog/S-046-windows-hive-window.md): the command line.
//!
//! Contract (tester, from AC5; do not edit). In `crate::args`:
//! - `#[derive(Debug, PartialEq)] pub enum Mode { Daemon, Open(Vec<String>) }`.
//! - `pub fn parse(args: &[String]) -> Result<Mode, String>`; `args` are the
//!   arguments after the program name.
//!
//! S-086 AC1 (backlog/S-086-icon-opens-window.md), amended by the tester:
//! `Mode` gains `Settings` for the single word `settings`. With any extra
//! word `settings` is an error `kawarod: settings takes no arguments`
//! (contract the AC leaves open, fixed here), and `open`, the daemon default
//! and the unknown argument message of today are unchanged.

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

// S-086 AC1 (backlog/S-086-icon-opens-window.md): the `settings` mode beside
// the modes of today.

#[test]
fn s086_ac1_settings_is_its_own_mode() {
    assert_eq!(parse(&v(&["settings"])), Ok(Mode::Settings));
}

#[test]
fn s086_ac1_settings_with_an_extra_word_is_an_error_naming_the_problem() {
    let e = parse(&v(&["settings", "x"])).unwrap_err();
    assert!(e.contains("settings takes no arguments"), "{e}");
}

#[test]
fn s086_ac1_settings_after_the_open_command_is_a_path() {
    assert_eq!(
        parse(&v(&["open", "settings"])),
        Ok(Mode::Open(v(&["settings"])))
    );
}

#[test]
fn s086_ac1_the_other_modes_and_the_unknown_message_are_unchanged() {
    assert_eq!(parse(&[]), Ok(Mode::Daemon));
    assert_eq!(
        parse(&v(&["open", "a", "b"])),
        Ok(Mode::Open(v(&["a", "b"])))
    );
    assert_eq!(
        parse(&v(&["bogus"])),
        Err("kawarod: unknown argument `bogus`".to_string())
    );
}
