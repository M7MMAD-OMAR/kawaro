//! S-049 AC3 (backlog/S-049-windows-notifications.md): every "open the
//! folder" spawn goes through `notify_logic::open_folder_command`, so no
//! source file in `crates/daemon/src` outside `notify_logic.rs` spells
//! `xdg-open`. The files are read at test time.

use std::path::{Path, PathBuf};

fn src_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn ac3_no_xdg_open_in_daemon_sources_outside_notify_logic_and_both_call_sites_use_the_helper() {
    let src = src_dir();
    assert!(
        src.join("notify_logic.rs").is_file(),
        "crates/daemon/src/notify_logic.rs must exist"
    );
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    let offenders: Vec<String> = files
        .iter()
        .filter(|p| p.file_name().is_some_and(|n| n != "notify_logic.rs"))
        .filter(|p| std::fs::read_to_string(p).unwrap().contains("xdg-open"))
        .map(|p| p.strip_prefix(&src).unwrap().display().to_string())
        .collect();
    assert!(
        offenders.is_empty(),
        "xdg-open is spelled outside notify_logic.rs in: {offenders:?}"
    );

    for name in ["notify.rs", "jobs.rs"] {
        let text = std::fs::read_to_string(src.join(name)).unwrap();
        assert!(
            text.contains("open_folder_command("),
            "{name} must spawn through notify_logic::open_folder_command"
        );
    }
}
