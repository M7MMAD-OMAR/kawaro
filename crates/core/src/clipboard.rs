//! The clipboard file list (S-135): one reader shared by the CLI and the
//! daemon, with the Wayland and X11 commands of S-110 and a pure Windows
//! CF_HDROP decoder that is tested on every platform.

use std::path::PathBuf;

/// The file paths of a `text/uri-list` clipboard text, comments and other
/// schemes skipped, percent-decoded.
pub fn files_from_uri_list(text: &str) -> Vec<PathBuf> {
    crate::uri_list::parse(text)
}

/// The command each session type uses to read copied files.
pub fn command(wayland: bool) -> (&'static str, Vec<&'static str>) {
    if wayland {
        ("wl-paste", vec!["--no-newline", "--type", "text/uri-list"])
    } else {
        (
            "xclip",
            vec!["-o", "-selection", "clipboard", "-t", "text/uri-list"],
        )
    }
}

/// `run` executes a command and returns (success, stdout).
pub type Runner<'a> = &'a dyn Fn(&str, &[&str]) -> std::io::Result<(bool, Vec<u8>)>;

/// Reads the file list with the session command through `run`; an empty or
/// unreadable clipboard is an error naming the reason.
pub fn read_files_with(wayland: bool, run: Runner<'_>) -> Result<Vec<PathBuf>, String> {
    let (cmd, args) = command(wayland);
    let borrowed: Vec<&str> = args.clone();
    let (ok, stdout) = run(cmd, &borrowed)
        .map_err(|e| format!("cannot read the clipboard ({e}); install wl-clipboard or xclip"))?;
    if !ok {
        return Err("the clipboard holds no files".into());
    }
    let files = files_from_uri_list(&String::from_utf8_lossy(&stdout));
    if files.is_empty() {
        Err("the clipboard holds no files".into())
    } else {
        Ok(files)
    }
}

/// Reads the file list from this session's clipboard.
pub fn read_files() -> Result<Vec<PathBuf>, String> {
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    read_files_with(wayland, &|cmd, args| {
        std::process::Command::new(cmd)
            .args(args)
            .output()
            .map(|o| (o.status.success(), o.stdout))
    })
}

/// The paths of a CF_HDROP `DROPFILES` buffer (Windows): a little endian
/// offset to a double NUL terminated UTF-16 list. Empty on any malformed
/// buffer, never a panic.
pub fn cf_hdrop_paths(buffer: &[u8]) -> Vec<PathBuf> {
    if buffer.len() < 20 {
        return Vec::new();
    }
    let offset = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]) as usize;
    if offset >= buffer.len() {
        return Vec::new();
    }
    let units: Vec<u16> = buffer[offset..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect();
    units
        .split(|unit| *unit == 0)
        .filter(|part| !part.is_empty())
        .map(|part| PathBuf::from(String::from_utf16_lossy(part)))
        .collect()
}
