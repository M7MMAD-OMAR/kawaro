//! Files copied in a file manager arrive as `text/uri-list`.

use std::path::PathBuf;
use std::process::Command;

pub fn read_files() -> Result<Vec<PathBuf>, String> {
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    read_files_with(wayland, &|cmd, args| {
        Command::new(cmd)
            .args(args)
            .output()
            .map(|o| (o.status.success(), o.stdout))
    })
}

/// The command each session type uses to read copied files.
pub fn command(wayland: bool) -> (&'static str, &'static [&'static str]) {
    if wayland {
        ("wl-paste", &["--no-newline", "--type", "text/uri-list"])
    } else {
        (
            "xclip",
            &["-o", "-selection", "clipboard", "-t", "text/uri-list"],
        )
    }
}

type Runner<'a> = &'a dyn Fn(&str, &[&str]) -> std::io::Result<(bool, Vec<u8>)>;

/// `run` executes a command and returns (success, stdout).
pub fn read_files_with(wayland: bool, run: Runner) -> Result<Vec<PathBuf>, String> {
    let (cmd, args) = command(wayland);
    let (ok, stdout) = run(cmd, args)
        .map_err(|e| format!("cannot read the clipboard ({e}); install wl-clipboard or xclip"))?;
    if !ok {
        return Err("the clipboard holds no files".into());
    }
    let files = parse_uri_list(&String::from_utf8_lossy(&stdout));
    if files.is_empty() {
        Err("the clipboard holds no files".into())
    } else {
        Ok(files)
    }
}

/// Shared with drag and drop, see `sbarhive_core::uri_list`.
pub use sbarhive_core::uri_list::parse as parse_uri_list;

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
    fn picks_the_tool_for_the_session() {
        assert_eq!(command(true).0, "wl-paste");
        assert!(command(true).1.contains(&"text/uri-list"));
        assert_eq!(command(false).0, "xclip");
        assert!(command(false).1.contains(&"clipboard"));
    }

    #[test]
    fn reads_through_the_runner_and_reports_failures() {
        let seen = std::cell::RefCell::new(String::new());
        let ok = |cmd: &str, _: &[&str]| {
            *seen.borrow_mut() = cmd.to_string();
            Ok((true, b"file:///a/b.png\n".to_vec()))
        };
        assert_eq!(
            read_files_with(true, &ok).unwrap(),
            vec![PathBuf::from("/a/b.png")]
        );
        assert_eq!(*seen.borrow(), "wl-paste");
        assert_eq!(read_files_with(false, &ok).unwrap().len(), 1);
        assert_eq!(*seen.borrow(), "xclip");

        let failed = |_: &str, _: &[&str]| Ok((false, b"file:///a/b.png".to_vec()));
        assert!(
            read_files_with(true, &failed)
                .unwrap_err()
                .contains("no files")
        );
        let empty = |_: &str, _: &[&str]| Ok((true, b"https://x.org/a".to_vec()));
        assert!(
            read_files_with(true, &empty)
                .unwrap_err()
                .contains("no files")
        );
        let missing = |_: &str, _: &[&str]| Err(std::io::Error::other("not found"));
        assert!(
            read_files_with(true, &missing)
                .unwrap_err()
                .contains("wl-clipboard")
        );
    }

    #[test]
    fn rejects_broken_escapes() {
        assert!(parse_uri_list("file:///a%2").is_empty());
        assert!(parse_uri_list("file:///a%zz").is_empty());
    }
}
