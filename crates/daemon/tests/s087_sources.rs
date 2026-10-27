//! S-087 AC1, AC4, AC5 and AC7 (backlog/S-087-first-run-welcome.md): source
//! checks for the parts no test can observe headless, like `s086_sources.rs`.
//! The marker timing, the shared request path of the try button, the tray
//! item ignoring the marker and the bilingual CHANGELOG line are checked on
//! the files themselves; the live first click stays owner-run.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    let path = root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} must exist: {e}", path.display()))
}

/// The body of `fn name` up to the next top level `fn` or `pub fn`.
fn fn_body<'a>(src: &'a str, name: &str) -> &'a str {
    let start = src
        .find(&format!("fn {name}"))
        .unwrap_or_else(|| panic!("{name} exists"));
    let end = src[start + 1..]
        .find("\nfn ")
        .map(|i| start + 1 + i)
        .unwrap_or(src.len());
    &src[start..end]
}

#[test]
fn ac1_the_marker_is_saved_only_after_language_choice() {
    let src = read("crates/daemon/src/ui.rs");
    let body = fn_body(&src, "welcome_window");
    assert!(body.contains("win.on_language("));
    assert!(body.contains("save_language_choice("));
    let save = fn_body(&src, "save_language_choice");
    assert!(save.find("config.save_to(path)?").unwrap() < save.find("st.config = config").unwrap());
}

#[test]
fn ac4_the_try_button_uses_the_same_request_path_as_the_explorer_menu() {
    let src = read("crates/daemon/src/ui.rs");
    let body = fn_body(&src, "try_sample");
    assert!(
        body.contains("server::open("),
        "the try button goes through server::open, the request path of Request::Open: {body}"
    );
    assert!(
        body.contains("install_sample"),
        "the try button installs the bundled sample first: {body}"
    );
    // The welcome wires that path to the button.
    assert!(
        src.contains("on_try("),
        "the welcome window's try callback is wired in ui.rs"
    );
}

#[test]
fn ac1_the_tray_get_started_item_opens_the_welcome_regardless_of_the_marker() {
    let linux = read("crates/daemon/src/tray/linux.rs");
    assert!(
        linux.contains("text::get_started"),
        "the Linux tray menu carries the bilingual item"
    );
    assert!(
        linux.contains("present_welcome"),
        "the Linux tray item opens the welcome window"
    );
    assert!(
        !linux.contains("welcome_seen"),
        "the tray item never looks at the marker"
    );
    let windows = read("crates/daemon/src/shell_logic.rs");
    assert!(
        windows.contains("Command::GetStarted"),
        "the Windows menu carries the item"
    );
    let dispatch = read("crates/daemon/src/shell.rs");
    let arm = dispatch
        .split("Command::GetStarted")
        .nth(1)
        .expect("the Windows dispatch handles GetStarted");
    assert!(
        arm.contains("present_welcome"),
        "the Windows item opens the welcome window: {arm}"
    );
    assert!(
        !dispatch.contains("welcome_seen"),
        "no marker check in the dispatch"
    );
}

#[test]
fn ac5_changelog_has_one_s087_line_in_english_and_arabic() {
    let s = read("CHANGELOG.md");
    let lines: Vec<&str> = s.lines().filter(|l| l.contains("[S-087]")).collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].contains('«') && lines[0].contains('»'),
        "the S-087 line carries its Arabic part in guillemets like the other entries: {}",
        lines[0]
    );
    assert!(
        !lines[0].contains('\u{2013}') && !lines[0].contains('\u{2014}'),
        "no long dash in the S-087 line: {}",
        lines[0]
    );
}

#[test]
fn ac1_the_settings_entries_all_go_through_the_one_route() {
    // `Request::Settings`, `kawarod settings` and `kawaro settings` all
    // reach `server::settings`, which is the only caller of `welcome::route`.
    let server = read("crates/daemon/src/server.rs");
    let body = fn_body(&server, "settings");
    assert!(
        body.contains("welcome::route") || body.contains("route(&config)"),
        "server::settings routes on the marker: {body}"
    );
    let main = read("crates/daemon/src/main.rs");
    assert!(
        main.contains("server::settings(&state)"),
        "the standalone `kawarod settings` path calls the same route"
    );
}
