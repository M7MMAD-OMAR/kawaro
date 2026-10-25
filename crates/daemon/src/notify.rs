//! Desktop notifications. They never take focus; the only one per job is
//! sent when it ends.

use crate::state::{JobInfo, lock};
use crate::text;
use kawaro_core::i18n::Lang;
use kawaro_core::update::result::{Outcome, take_result};
use notify_rust::Notification;
use std::path::Path;
use std::sync::Mutex;

/// What an update notification is about (S-054b).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UpdateKind {
    Ready,
    Updated,
    Failed,
}

/// Where an update notification goes: a log line and a desktop popup.
pub trait UpdateSink {
    fn log(&mut self, line: &str);
    fn show(&mut self, summary: &str, body: &str);
}

/// The (kind, version) pairs already shown, so each is shown once.
#[derive(Default)]
pub struct Sent {
    pairs: Vec<(UpdateKind, String)>,
}

/// One log line and one notification for a kind and version, once.
pub fn notify_update_with(
    sink: &mut dyn UpdateSink,
    sent: &mut Sent,
    lang: Lang,
    kind: UpdateKind,
    version: &str,
) {
    if sent.pairs.iter().any(|(k, v)| *k == kind && v == version) {
        return;
    }
    sent.pairs.push((kind, version.to_string()));
    let message = text::update_message(lang, kind, version);
    sink.log(&format!("kawarod: notify: {message}"));
    sink.show(text::app_name(lang), &message);
}

/// Reports the result a helper process left (`update-result.json`), once.
pub fn report_update_result_with(
    sink: &mut dyn UpdateSink,
    sent: &mut Sent,
    lang: Lang,
    dir: &Path,
) {
    let Some(result) = take_result(dir) else {
        return;
    };
    let kind = match result.outcome {
        Outcome::Installed => UpdateKind::Updated,
        Outcome::Failed(_) => UpdateKind::Failed,
    };
    notify_update_with(sink, sent, lang, kind, &result.version);
}

/// The desktop: stderr for the log line, a notification that takes no focus.
struct Desktop;

impl UpdateSink for Desktop {
    fn log(&mut self, line: &str) {
        eprintln!("{line}");
    }
    fn show(&mut self, summary: &str, body: &str) {
        show(
            Notification::new()
                .appname(summary)
                .summary(summary)
                .body(body)
                .icon("kawaro"),
        );
    }
}

static SENT: Mutex<Sent> = Mutex::new(Sent { pairs: Vec::new() });

/// The production call: the real desktop, one notification per kind and
/// version for the life of the daemon.
pub fn notify_update(lang: Lang, kind: UpdateKind, version: &str) {
    notify_update_with(&mut Desktop, &mut lock(&SENT), lang, kind, version);
}

/// At start: the result of an install that ran in another process.
pub fn report_update_result(lang: Lang) {
    if let Some(dir) = kawaro_core::paths::config_dir() {
        report_update_result_with(&mut Desktop, &mut lock(&SENT), lang, &dir);
    }
}

/// The gate in front of every real desktop notification: true only outside a
/// test build and without `KAWARO_NO_NOTIFY` (any value), so no test run
/// can reach a live session's notification service. The `kawarod: notify:`
/// log line is written either way.
pub fn notifications_allowed(is_test: bool, no_notify_set: bool) -> bool {
    !is_test && !no_notify_set
}

/// The only call into the notification service. A test build has none, so
/// no test, and no mutant of the gate above, can reach a live desktop.
#[cfg(not(test))]
fn send(n: &Notification) -> Option<notify_rust::NotificationHandle> {
    n.show().ok()
}

#[cfg(test)]
fn send(_: &Notification) -> Option<notify_rust::NotificationHandle> {
    None
}

fn show(n: &Notification) -> Option<notify_rust::NotificationHandle> {
    if !notifications_allowed(cfg!(test), std::env::var_os("KAWARO_NO_NOTIFY").is_some()) {
        return None;
    }
    send(n)
}

pub fn info(lang: Lang, body: &str) {
    // One log line per notification, so tests can count them.
    eprintln!("kawarod: notify: {body}");
    show(
        Notification::new()
            .appname(text::app_name(lang))
            .summary(text::app_name(lang))
            .body(body)
            .icon("kawaro"),
    );
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
    eprintln!("kawarod: notify: {summary}");
    let folder = job
        .first_output
        .as_deref()
        .and_then(Path::parent)
        .map(Path::to_path_buf);
    let mut n = Notification::new();
    n.appname(text::app_name(lang))
        .summary(&summary)
        .body(&body)
        .icon("kawaro");
    if folder.is_some() {
        n.action("default", text::open_folder(lang));
    }
    let Some(handle) = show(&n) else { return };
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
