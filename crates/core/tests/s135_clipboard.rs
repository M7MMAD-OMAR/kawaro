//! S-135 AC5: the shared clipboard file-list reader.
//! Contract pinned here (tester): kawaro_core::clipboard exposes
//! files_from_uri_list(text), read_files_with(wayland, run) with the same
//! Wayland and X11 commands as today, and a pure Windows decoder
//! cf_hdrop_paths(buffer) for a CF_HDROP DROPFILES buffer of UTF-16 paths.

use kawaro_core::clipboard;
use std::path::PathBuf;

#[test]
fn ac5_the_uri_list_reader_is_unchanged() {
    let text = "# copied\r\nfile:///home/u/My%20Photo.png\r\nfile://localhost/tmp/a.jpg\nhttps://x.org/y\nfile:///home/u/%D8%B5%D9%88%D8%B1%D8%A9.webp\n";
    assert_eq!(
        clipboard::files_from_uri_list(text),
        vec![
            PathBuf::from("/home/u/My Photo.png"),
            PathBuf::from("/tmp/a.jpg"),
            PathBuf::from("/home/u/صورة.webp"),
        ]
    );
}

#[test]
fn ac5_the_session_commands_are_the_same_as_today() {
    let (cmd, args) = clipboard::command(true);
    assert_eq!(cmd, "wl-paste");
    assert!(args.contains(&"text/uri-list"));
    let (cmd, args) = clipboard::command(false);
    assert_eq!(cmd, "xclip");
    assert!(args.contains(&"text/uri-list"));
}

#[test]
fn ac5_read_files_with_reports_files_and_errors() {
    let wayland = |_cmd: &str, _args: &[&str]| {
        Ok((
            true,
            b"file:///home/u/a.png\r\nfile:///tmp/b.jpg\r\n".to_vec(),
        ))
    };
    assert_eq!(
        clipboard::read_files_with(true, &wayland).unwrap(),
        vec![PathBuf::from("/home/u/a.png"), PathBuf::from("/tmp/b.jpg")]
    );
    let empty = |_cmd: &str, _args: &[&str]| Ok((true, b"# no files\r\n".to_vec()));
    assert!(clipboard::read_files_with(true, &empty).is_err());
    let failure = |_cmd: &str, _args: &[&str]| Ok((false, Vec::new()));
    assert!(clipboard::read_files_with(false, &failure).is_err());
    let missing = |_cmd: &str, _args: &[&str]| {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no wl-paste",
        ))
    };
    assert!(clipboard::read_files_with(true, &missing).is_err());
}

fn hdrop(paths: &[&str]) -> Vec<u8> {
    let mut buffer = Vec::new();
    buffer.extend_from_slice(&20u32.to_le_bytes());
    buffer.extend_from_slice(&0i32.to_le_bytes());
    buffer.extend_from_slice(&0i32.to_le_bytes());
    buffer.extend_from_slice(&0i32.to_le_bytes());
    buffer.extend_from_slice(&1i32.to_le_bytes());
    for path in paths {
        for unit in path.encode_utf16() {
            buffer.extend_from_slice(&unit.to_le_bytes());
        }
        buffer.extend_from_slice(&0u16.to_le_bytes());
    }
    buffer.extend_from_slice(&0u16.to_le_bytes());
    buffer
}

#[test]
fn ac5_the_windows_decoder_reads_one_and_several_files() {
    assert_eq!(
        clipboard::cf_hdrop_paths(&hdrop(&["C:\\Users\\u\\a.png"])),
        vec![PathBuf::from("C:\\Users\\u\\a.png")]
    );
    assert_eq!(
        clipboard::cf_hdrop_paths(&hdrop(&["C:\\a b.jpg", "C:\\c.webp"])),
        vec![PathBuf::from("C:\\a b.jpg"), PathBuf::from("C:\\c.webp")]
    );
}

#[test]
fn ac5_the_windows_decoder_reads_arabic_names() {
    let arabic = "C:\\مجلد\\صورة.png";
    assert_eq!(
        clipboard::cf_hdrop_paths(&hdrop(&[arabic])),
        vec![PathBuf::from(arabic)]
    );
}

#[test]
fn ac5_empty_and_malformed_windows_buffers_decode_to_nothing() {
    assert_eq!(
        clipboard::cf_hdrop_paths(&hdrop(&[])),
        Vec::<PathBuf>::new()
    );
    assert_eq!(clipboard::cf_hdrop_paths(&[]), Vec::<PathBuf>::new());
    let mut short = vec![0u8; 3];
    short.extend_from_slice(&[0, 0]);
    assert_eq!(clipboard::cf_hdrop_paths(&short), Vec::<PathBuf>::new());
    let mut bad_offset = hdrop(&["C:\\a.png"]);
    bad_offset[..4].copy_from_slice(&9999u32.to_le_bytes());
    assert_eq!(
        clipboard::cf_hdrop_paths(&bad_offset),
        Vec::<PathBuf>::new(),
        "an offset past the buffer decodes to an empty list"
    );
}
