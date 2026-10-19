//! Desktop notifications. They never take focus; the only one per job is
//! sent when it ends.

use crate::state::JobInfo;
use crate::text;
use notify_rust::Notification;
use sbarhive_core::i18n::Lang;
use std::path::Path;

pub fn info(lang: Lang, body: &str) {
    let _ = Notification::new()
        .appname(text::app_name(lang))
        .summary(text::app_name(lang))
        .body(body)
        .icon("sbarhive")
        .show();
}

pub fn finished(lang: Lang, job: &JobInfo, cancelled: bool) {
    let total = job.ok + job.failed;
    let (summary, body) = if cancelled {
        (text::cancelled(lang).to_string(), job.label.clone())
    } else if job.failed > 0 {
        (
            text::failed(lang, job.failed, total.max(job.tasks)),
            job.first_error.clone().unwrap_or_default(),
        )
    } else {
        let name = job
            .first_output
            .as_deref()
            .and_then(Path::file_name)
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        (text::done(lang, job.ok), name)
    };
    let folder = job
        .first_output
        .as_deref()
        .and_then(Path::parent)
        .map(Path::to_path_buf);
    let mut n = Notification::new();
    n.appname(text::app_name(lang))
        .summary(&summary)
        .body(&body)
        .icon("sbarhive");
    if folder.is_some() {
        n.action("default", text::open_folder(lang));
    }
    let Ok(handle) = n.show() else { return };
    if let Some(folder) = folder {
        // Waiting blocks until the notification closes, so it gets its own thread.
        std::thread::spawn(move || {
            handle.wait_for_action(|action| {
                if action == "default" {
                    let _ = std::process::Command::new("xdg-open").arg(&folder).spawn();
                }
            });
        });
    }
}
