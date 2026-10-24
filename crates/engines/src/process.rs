//! S-030: one killable process tree per engine child.
//!
//! `ProcessTree` owns a spawned child and every process it starts. On Unix
//! the child leads its own process group and `kill_all` sends SIGKILL to the
//! group; on Windows the child runs in a kill-on-close job object. Dropping a
//! tree kills it and reaps the child.

#[cfg(test)]
mod tests {
    // Every test here is Unix only: `scripts/windows/check.sh` builds the
    // test targets for Windows, which has no `sh`.
    #[cfg(unix)]
    use super::*;
    #[cfg(unix)]
    use std::io::{BufRead, BufReader};
    #[cfg(unix)]
    use std::process::{Command, Stdio};
    #[cfg(unix)]
    use std::sync::mpsc;
    #[cfg(unix)]
    use std::time::{Duration, Instant};

    // S-030 (backlog/S-030-process-tree.md): the tree is
    // `sh -c 'sleep 30 & echo $!; wait'`; the grandchild pid comes from stdout.

    #[cfg(unix)]
    fn spawn_tree() -> (ProcessTree, i32, i32) {
        let mut cmd = Command::new("/bin/sh");
        cmd.args(["-c", "sleep 30 & echo $!; wait"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut tree = ProcessTree::spawn(&mut cmd).expect("spawn the tree");
        let sh = tree.child_mut().id() as i32;
        let stdout = tree.child_mut().stdout.take().expect("stdout is piped");
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut line = String::new();
            let _ = BufReader::new(stdout).read_line(&mut line);
            let _ = tx.send(line);
        });
        let line = rx
            .recv_timeout(Duration::from_secs(3))
            .expect("the tree did not print the grandchild pid within 3 s");
        let grandchild = line.trim().parse().expect("grandchild pid");
        (tree, sh, grandchild)
    }

    /// True when `/proc/<pid>` is absent or the process is a zombie.
    #[cfg(unix)]
    fn is_gone(pid: i32) -> bool {
        match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
            Err(_) => true,
            Ok(stat) => stat
                .rsplit_once(')')
                .and_then(|(_, rest)| rest.split_whitespace().next())
                .is_some_and(|state| state == "Z"),
        }
    }

    /// Polls every 20 ms for up to 2 s.
    #[cfg(unix)]
    fn gone_soon(pid: i32) -> bool {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if is_gone(pid) {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[cfg(unix)]
    #[test]
    fn s030_ac1_ac2_kill_all_kills_the_whole_tree_and_is_idempotent() {
        let (mut tree, sh, grandchild) = spawn_tree();
        assert!(!is_gone(grandchild), "grandchild {grandchild} not running");
        tree.kill_all();
        assert!(
            gone_soon(grandchild),
            "grandchild {grandchild} survived kill_all"
        );
        assert!(gone_soon(sh), "sh {sh} survived kill_all");
        tree.kill_all();
    }

    #[cfg(unix)]
    #[test]
    fn s030_ac1_drop_kills_the_tree_and_reaps_the_child() {
        let (tree, sh, grandchild) = spawn_tree();
        assert!(!is_gone(grandchild), "grandchild {grandchild} not running");
        drop(tree);
        assert!(
            !std::path::Path::new(&format!("/proc/{sh}")).exists(),
            "sh {sh} was not reaped by the drop"
        );
        assert!(
            gone_soon(grandchild),
            "grandchild {grandchild} survived the drop"
        );
    }
}
