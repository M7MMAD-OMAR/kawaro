//! Files copied in a file manager arrive as `text/uri-list`.

use std::path::PathBuf;
use std::process::Command;

pub fn read_files() -> Result<Vec<PathBuf>, String> {
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    let out = if wayland {
        Command::new("wl-paste")
            .args(["--no-newline", "--type", "text/uri-list"])
            .output()
    } else {
        Command::new("xclip")
            .args(["-o", "-selection", "clipboard", "-t", "text/uri-list"])
            .output()
    }
    .map_err(|e| format!("cannot read the clipboard ({e}); install wl-clipboard or xclip"))?;
    if !out.status.success() {
        return Err("the clipboard holds no files".into());
    }
    let files = parse_uri_list(&String::from_utf8_lossy(&out.stdout));
    if files.is_empty() {
        Err("the clipboard holds no files".into())
    } else {
        Ok(files)
    }
}

/// RFC 2483: one URI per line, `#` lines are comments. Only local `file:` URIs count.
pub fn parse_uri_list(text: &str) -> Vec<PathBuf> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.strip_prefix("file://"))
        // Drop an optional host part: file://localhost/x or file:///x.
        .filter_map(|rest| rest.find('/').map(|i| &rest[i..]))
        .filter_map(percent_decode)
        .map(PathBuf::from)
        .collect()
}

fn percent_decode(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = std::str::from_utf8(b.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_file_uris_with_spaces_arabic_and_comments() {
        let text = "# copied\r\nfile:///home/u/My%20Photo.png\r\nfile://localhost/tmp/a.jpg\nhttps://x.org/y\nfile:///home/u/%D8%B5%D9%88%D8%B1%D8%A9.webp\n";
        let got = parse_uri_list(text);
        assert_eq!(
            got,
            vec![
                PathBuf::from("/home/u/My Photo.png"),
                PathBuf::from("/tmp/a.jpg"),
                PathBuf::from("/home/u/صورة.webp"),
            ]
        );
    }

    #[test]
    fn rejects_broken_escapes() {
        assert!(parse_uri_list("file:///a%2").is_empty());
        assert!(parse_uri_list("file:///a%zz").is_empty());
    }
}
