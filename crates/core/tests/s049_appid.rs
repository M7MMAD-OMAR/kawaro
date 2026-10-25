//! S-049 AC1 (backlog/S-049-windows-notifications.md): one app id constant,
//! `kawaro_core::appid::APP_ID`, and the Windows pipe name uses it as its
//! prefix.
//!
//! Contract (tester, from the acceptance criteria; do not edit):
//! - `pub mod appid;` in `crates/core/src/lib.rs`, with
//!   `pub const APP_ID: &str = "io.github.m7mmad_omar.Kawaro";`.
//! - `transport::pipe_name(sid, session)` builds its name from `APP_ID`, so the
//!   literal is defined once: no code line of `crates/core/src/transport.rs`
//!   spells it out.
//! - On Windows the daemon builds the notification with `.app_id(APP_ID)`.

use kawaro_core::appid::APP_ID;
use kawaro_core::transport::pipe_name;
use std::path::PathBuf;

const LITERAL: &str = "io.github.m7mmad_omar.Kawaro";

fn repo_file(rel: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Code lines only: `//` comments (and `///`, `//!`) are dropped.
fn code_lines(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect()
}

#[test]
fn ac1_app_id_is_the_reverse_dns_id_and_prefixes_the_pipe_name() {
    assert_eq!(APP_ID, "io.github.m7mmad_omar.Kawaro");
    assert_eq!(
        pipe_name("S-1-5-21-1-2-3-1001", 2),
        format!(r"\\.\pipe\{APP_ID}.S-1-5-21-1-2-3-1001.2")
    );
    assert!(pipe_name("S-1", 0).starts_with(&format!(r"\\.\pipe\{APP_ID}.")));
}

#[test]
fn ac1_the_id_has_one_definition_and_the_windows_toast_uses_it() {
    let appid = repo_file("crates/core/src/appid.rs");
    assert!(
        appid.contains(&format!("pub const APP_ID: &str = \"{LITERAL}\";")),
        "appid.rs must define APP_ID:\n{appid}"
    );

    let transport = repo_file("crates/core/src/transport.rs");
    let spelled: Vec<&str> = code_lines(&transport)
        .into_iter()
        .filter(|l| l.contains(LITERAL))
        .collect();
    assert!(
        spelled.is_empty(),
        "transport.rs must take the prefix from APP_ID, found: {spelled:?}"
    );
    assert!(transport.contains("APP_ID"), "transport.rs must use APP_ID");

    let notify = repo_file("crates/daemon/src/notify.rs");
    let code = code_lines(&notify).join("\n");
    assert!(
        code.contains("app_id(APP_ID)") || code.contains("app_id(kawaro_core::appid::APP_ID)"),
        "notify.rs must build the Windows toast with .app_id(APP_ID)"
    );
}
