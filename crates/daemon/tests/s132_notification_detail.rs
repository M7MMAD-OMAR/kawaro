//! S-132 AC3: the daemon end-of-job notification renders the shared detail
//! line in the window language, so its numbers and the Arabic phrase come
//! from kawaro_core::engine::detail_line.
//! Contract pinned here (tester): the notification path in crates/daemon/src
//! calls detail_line(detail, lang) with the window language (st.lang).

use kawaro_core::engine::{TaskDetail, detail_line};
use kawaro_core::i18n::Lang;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> Option<String> {
    std::fs::read_to_string(root().join(rel)).ok()
}

#[test]
fn ac3_the_daemon_notification_uses_the_shared_detail_line() {
    let mut daemon = String::new();
    for rel in [
        "crates/daemon/src/jobs.rs",
        "crates/daemon/src/notify.rs",
        "crates/daemon/src/text.rs",
    ] {
        if let Some(text) = read(rel) {
            daemon.push_str(&text);
            daemon.push('\n');
        }
    }
    assert!(
        daemon.contains("detail_line"),
        "the end-of-job notification must render kawaro_core::engine::detail_line"
    );
    assert!(
        daemon.contains("st.lang") || daemon.contains(".lang"),
        "the notification renders in the window language"
    );
}

#[test]
fn ac3_the_detail_line_is_localized_for_the_notification() {
    let detail = TaskDetail {
        width: 1536,
        height: 1024,
        source_width: 2400,
        source_height: 1600,
        bytes: 4242,
        downscaled: true,
    };
    let english = detail_line(&detail, Lang::En);
    let arabic = detail_line(&detail, Lang::Ar);
    assert!(english.contains("downscaled from"), "{english}");
    assert!(
        !arabic.contains("downscaled from"),
        "Arabic has its own phrase: {arabic}"
    );
    assert!(
        arabic
            .chars()
            .any(|c| ('\u{0600}'..='\u{06FF}').contains(&c)),
        "the notification body has Arabic text: {arabic}"
    );
    for line in [&english, &arabic] {
        assert!(
            !line.contains('\u{2013}') && !line.contains('\u{2014}'),
            "{line}"
        );
    }
}
