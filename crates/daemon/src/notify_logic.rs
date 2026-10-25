//! The pure side of the notifications (S-049): the command that opens the
//! output folder on each platform and the log line the tests count. Only
//! `std` and `kawaro_core`, so `tests/s049_notify_logic.rs` includes it.

pub use kawaro_core::paths::Os;
use std::path::Path;

/// The start of every notification log line.
const LOG_PREFIX: &str = "kawarod: notify: ";

/// The program and its arguments that show the result: Explorer selects
/// `file` (one `/select,` argument, spaces kept) or opens `folder`;
/// `xdg-open` opens `folder` on Linux.
pub fn open_folder_command(os: Os, folder: &Path, file: Option<&Path>) -> (String, Vec<String>) {
    match os {
        Os::Windows => {
            let arg = match file {
                Some(file) => format!("/select,{}", file.to_string_lossy()),
                None => folder.to_string_lossy().into_owned(),
            };
            ("explorer.exe".to_string(), vec![arg])
        }
        Os::Linux => (
            "xdg-open".to_string(),
            vec![folder.to_string_lossy().into_owned()],
        ),
    }
}

/// One notification log line: `kawarod: notify: <body>`.
pub fn log_line(body: &str) -> String {
    format!("{LOG_PREFIX}{body}")
}

/// The lines of `log` that start with the notification prefix. Only the
/// tests count them.
#[cfg_attr(not(test), allow(dead_code))]
pub fn count_notify_lines(log: &str) -> usize {
    log.lines().filter(|l| l.starts_with(LOG_PREFIX)).count()
}
