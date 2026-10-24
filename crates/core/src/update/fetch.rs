//! Fetching a file over https with the system `curl` (ADR 0006 section 4).
//!
//! No HTTP or TLS crate: the program is found by absolute path only (never
//! `PATH`), speaks https only, sends no cookie, header or referer, and a fixed
//! user agent with no version or platform in it.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::paths::{CURRENT, EnvFn, Os};

/// The most a manifest may weigh.
pub const MANIFEST_MAX_BYTES: u64 = 262_144;
/// The most a signature file may weigh.
pub const SIGNATURE_MAX_BYTES: u64 = 4_096;

/// Seconds allowed for a manifest or a signature, and for a package.
const SMALL_TIMEOUT: &str = "30";
const PACKAGE_TIMEOUT: &str = "1800";

/// Why a download did not happen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FetchError {
    /// The network cannot be reached (no DNS, refused, timed out).
    Offline,
    /// The server answered with this HTTP status.
    Http(u16),
    /// The file is larger than the cap.
    TooLarge,
    /// No curl program could be run.
    Unavailable,
    /// The url holds `..`, `%2e`, a backslash, whitespace or a control character.
    BadUrl,
}

/// Something that can download a url to a file.
pub trait Fetcher {
    fn get(&self, url: &str, dest: &Path, max_bytes: u64) -> Result<(), FetchError>;
}

/// False for a url with `..`, a percent encoded dot, a backslash, whitespace
/// or a control character (AC9).
pub fn url_is_clean(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    !(lower.contains("..")
        || lower.contains("%2e")
        || url
            .chars()
            .any(|c| c == '\\' || c.is_whitespace() || c.is_control()))
}

/// The exact arguments for curl. A manifest or a signature (`.json`,
/// `.minisig`) gets 30 seconds, a package 1800.
pub fn curl_args(url: &str, dest: &Path, max_bytes: u64) -> Vec<String> {
    let small = url.ends_with(".json") || url.ends_with(".minisig");
    let timeout = if small {
        SMALL_TIMEOUT
    } else {
        PACKAGE_TIMEOUT
    };
    let mut args: Vec<String> = [
        "--silent",
        "--show-error",
        "--fail",
        "--location",
        "--proto",
        "=https",
        "--proto-redir",
        "=https",
        "--max-redirs",
        "5",
        "--max-time",
        timeout,
        "--max-filesize",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    args.push(max_bytes.to_string());
    args.extend(["--user-agent", "sbarhive-update", "--output"].map(String::from));
    args.push(dest.to_string_lossy().into_owned());
    args.push(url.to_string());
    args
}

/// Where curl may live: two absolute paths on Linux, `System32` on Windows.
/// `PATH` is never read.
pub fn curl_candidates(os: Os, env: EnvFn) -> Vec<PathBuf> {
    match os {
        Os::Linux => vec![PathBuf::from("/usr/bin/curl"), PathBuf::from("/bin/curl")],
        Os::Windows => env("SystemRoot")
            .filter(|v| !v.is_empty())
            .map(|root| PathBuf::from(root).join("System32").join("curl.exe"))
            .into_iter()
            .collect(),
    }
}

/// Runs a curl program. An empty program path means none was found.
#[derive(Clone, Debug)]
pub struct CurlFetcher {
    program: PathBuf,
}

impl CurlFetcher {
    pub fn with_program(program: impl Into<PathBuf>) -> CurlFetcher {
        CurlFetcher {
            program: program.into(),
        }
    }

    /// The first system curl that exists, or a fetcher that is `Unavailable`.
    pub fn system() -> CurlFetcher {
        let found = curl_candidates(CURRENT, &|k| std::env::var_os(k))
            .into_iter()
            .find(|p| p.is_file())
            .unwrap_or_default();
        CurlFetcher::with_program(found)
    }
}

/// The `404` of curl's "The requested URL returned error: 404"; 0 when absent.
fn http_status(stderr: &str) -> u16 {
    stderr
        .split("returned error: ")
        .nth(1)
        .map(|rest| {
            rest.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
        })
        .and_then(|digits| digits.parse().ok())
        .unwrap_or(0)
}

impl Fetcher for CurlFetcher {
    fn get(&self, url: &str, dest: &Path, max_bytes: u64) -> Result<(), FetchError> {
        if !url_is_clean(url) {
            return Err(FetchError::BadUrl);
        }
        let mut command = Command::new(&self.program);
        command
            .args(curl_args(url, dest, max_bytes))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // CREATE_NO_WINDOW: no console flashes up.
            command.creation_flags(0x0800_0000);
        }
        let output = command.output().map_err(|_| FetchError::Unavailable)?;
        match output.status.code() {
            Some(0) => Ok(()),
            Some(6 | 7 | 28) => Err(FetchError::Offline),
            Some(22) => Err(FetchError::Http(http_status(&String::from_utf8_lossy(
                &output.stderr,
            )))),
            Some(63) => Err(FetchError::TooLarge),
            _ => Err(FetchError::Unavailable),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_is_read_from_the_curl_message() {
        assert_eq!(
            http_status("curl: (22) The requested URL returned error: 404\n"),
            404
        );
        assert_eq!(http_status("returned error: 503"), 503);
        assert_eq!(http_status("curl: (22) nothing"), 0);
        assert_eq!(http_status("returned error: "), 0);
    }

    #[test]
    fn clean_urls() {
        assert!(url_is_clean("https://h/a.b/c%20d%2f"));
        assert!(!url_is_clean("https://h/a/../b"));
        assert!(!url_is_clean("https://h/%2Ex"));
        assert!(!url_is_clean("https://h/a\\b"));
        assert!(!url_is_clean("https://h/a b"));
        assert!(!url_is_clean("https://h/a\u{7f}b"));
    }
}
