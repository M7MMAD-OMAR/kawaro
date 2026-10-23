//! S-028b AC6 and AC7 (backlog/S-028-review-fixes.md): source checks.
//! - AC6: `server.rs` has one line with `try_clone` (the `wait` subscriber).
//! - AC7: `main.rs` names no `socket_path`; the transport removes its own
//!   socket through `Listener::cleanup`.

fn lines_with(file: &str, word: &str) -> usize {
    let path = format!("{}/src/{file}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    text.lines().filter(|l| l.contains(word)).count()
}

#[test]
fn s028_ac6_server_rs_has_one_try_clone() {
    assert_eq!(lines_with("server.rs", "try_clone"), 1);
}

#[test]
fn s028_ac7_main_rs_names_no_socket_path() {
    assert_eq!(lines_with("main.rs", "socket_path"), 0);
}
