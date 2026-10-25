//! S-014a AC1: the `text/uri-list` parser shared by drops and the clipboard.
//! It must behave like `crates/cli/src/clipboard.rs`: CRLF and LF, `#` and
//! blank lines skipped, only `file:` URIs kept, percent decoding, order kept.

use kawaro_core::uri_list::parse;
use std::path::PathBuf;

fn paths(list: &[&str]) -> Vec<PathBuf> {
    list.iter().map(PathBuf::from).collect()
}

#[test]
fn ac1_crlf_lines_give_every_file_in_order() {
    let text = "file:///home/u/a.png\r\nfile:///home/u/b.pdf\r\n";
    assert_eq!(parse(text), paths(&["/home/u/a.png", "/home/u/b.pdf"]));
}

#[test]
fn ac1_lf_lines_and_a_missing_final_newline_are_accepted() {
    let text = "file:///home/u/a.png\nfile:///home/u/b.pdf";
    assert_eq!(parse(text), paths(&["/home/u/a.png", "/home/u/b.pdf"]));
}

#[test]
fn ac1_comment_and_blank_lines_are_ignored() {
    let text = "# dragged from Files\r\n\r\nfile:///tmp/one.png\r\n# another\r\n\r\n";
    assert_eq!(parse(text), paths(&["/tmp/one.png"]));
}

#[test]
fn ac1_percent_encoding_is_decoded_for_spaces_and_arabic() {
    let text = "file:///home/u/a%20b.png\r\nfile:///home/u/%D8%B5%D9%88%D8%B1%D8%A9.png\r\n";
    assert_eq!(
        parse(text),
        paths(&[
            "/home/u/a b.png",
            "/home/u/\u{635}\u{648}\u{631}\u{629}.png"
        ])
    );
}

#[test]
fn ac1_non_file_uris_are_ignored() {
    let text = "https://example.com/a.png\r\nfile:///tmp/a.png\r\nsmb://host/share/b.png\r\ntrash:///c.png\r\n";
    assert_eq!(parse(text), paths(&["/tmp/a.png"]));
}

#[test]
fn ac1_a_localhost_host_part_is_dropped_like_the_clipboard() {
    let text = "file://localhost/tmp/x.png\r\n";
    assert_eq!(parse(text), paths(&["/tmp/x.png"]));
}

#[test]
fn ac1_two_files_mixed_with_everything_else_keep_their_order() {
    let text = "# c\r\nfile:///w/z%20last.png\r\nhttps://e.org/\r\n\r\nfile:///w/%D8%B5.pdf\r\n";
    assert_eq!(parse(text), paths(&["/w/z last.png", "/w/\u{635}.pdf"]));
}

#[test]
fn ac1_nothing_usable_gives_an_empty_list() {
    assert!(parse("").is_empty());
    assert!(parse("# only a comment\r\nhttps://e.org/a.png\r\n").is_empty());
}
