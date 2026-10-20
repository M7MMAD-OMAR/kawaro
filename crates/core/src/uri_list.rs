//! `text/uri-list` (RFC 2483), as file managers put it on the clipboard and
//! in drag and drop: one URI per line, `#` lines are comments. Only local
//! `file:` URIs count.

use std::path::PathBuf;

/// The local files in `text`, in order. CRLF and LF both work; anything that
/// is not a well formed `file:` URI is skipped.
pub fn parse(text: &str) -> Vec<PathBuf> {
    text.lines()
        .map(str::trim)
        // Blank and `#` comment lines never start with `file://`, so this drops them too.
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
