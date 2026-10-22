//! S-022a AC5 (backlog/S-022-windows-ipc.md): the client and the service
//! reach the socket only through `sbarhive_core::transport`. The same check
//! as `grep -rn 'std::os::unix::net' crates/cli/src crates/daemon/src`.

use std::path::{Path, PathBuf};

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let p = entry.unwrap().path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn ac5_no_std_os_unix_net_in_cli_or_daemon_sources() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    rust_files(&root.join("crates/cli/src"), &mut files);
    rust_files(&root.join("crates/daemon/src"), &mut files);
    assert!(files.len() >= 4, "found only {files:?}");
    let mut hits = Vec::new();
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap();
        for (n, line) in text.lines().enumerate() {
            if line.contains("std::os::unix::net") {
                hits.push(format!("{}:{}: {}", f.display(), n + 1, line.trim()));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "{} lines:\n{}",
        hits.len(),
        hits.join("\n")
    );
}
