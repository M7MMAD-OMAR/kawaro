//! S-133 AC3: the daemon notification reports an already optimized PDF and
//! opens no folder; the skip is counted, not failed.
//! Contract pinned here (tester): jobs.rs handles JobEvent::TaskSkipped with
//! SkipReason and the localized "already optimized" text exists in the
//! daemon strings.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> Option<String> {
    std::fs::read_to_string(root().join(rel)).ok()
}

#[test]
fn ac3_the_daemon_handles_a_skipped_task_with_localized_text() {
    let jobs = read("crates/daemon/src/jobs.rs").unwrap_or_default();
    let text = read("crates/daemon/src/text.rs").unwrap_or_default();
    let notify = read("crates/daemon/src/notify.rs").unwrap_or_default();
    let all = format!("{jobs}\n{text}\n{notify}");
    assert!(
        all.contains("TaskSkipped"),
        "the daemon matches the queue skip event"
    );
    assert!(
        all.contains("AlreadyOptimized") || all.contains("already-optimized"),
        "the daemon reads the closed skip reason"
    );
    assert!(
        all.to_lowercase().contains("already optimized"),
        "the English notification says already optimized"
    );
    assert!(
        all.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c)),
        "the Arabic wording exists for the localized notification"
    );
    assert!(
        !all.contains('\u{2013}') && !all.contains('\u{2014}'),
        "no long dash in the notification strings"
    );
}
