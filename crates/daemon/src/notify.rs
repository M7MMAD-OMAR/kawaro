//! Desktop notifications. They never take focus; the only one per job is
//! sent when it ends.

use crate::notify_logic::{log_line, open_folder_command};
use crate::state::{JobInfo, lock};
use crate::text;
#[cfg(windows)]
use kawaro_core::appid::APP_ID;
use kawaro_core::i18n::Lang;
use kawaro_core::paths::CURRENT;
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
        write_log(line);
    }
    fn show(&mut self, summary: &str, body: &str) {
        show(notification(summary).summary(summary).body(body));
    }
}

/// A notification with the app name and icon; on Windows it also carries
/// the app id, so the toast is labelled Kawaro (S-049).
fn notification(app_name: &str) -> Notification {
    let mut n = Notification::new();
    n.appname(app_name).icon("kawaro");
    #[cfg(windows)]
    n.app_id(APP_ID);
    n
}

/// Writes one log line to stderr (the log file on Windows, S-045); a test
/// build also keeps it for [`captured_log`].
fn write_log(line: &str) {
    eprintln!("{line}");
    #[cfg(test)]
    CAPTURED.with(|c| {
        let mut c = c.borrow_mut();
        c.push_str(line);
        c.push('\n');
    });
}

#[cfg(test)]
thread_local! {
    static CAPTURED: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

/// The notification lines this thread wrote so far, one per line.
#[cfg(test)]
pub fn captured_log() -> String {
    CAPTURED.with(|c| c.borrow().clone())
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
    write_log(&log_line(body));
    show(
        notification(text::app_name(lang))
            .summary(text::app_name(lang))
            .body(body),
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
    write_log(&log_line(&summary));
    let file = job.first_output.clone();
    let folder = file
        .as_deref()
        .and_then(Path::parent)
        .map(Path::to_path_buf);
    let mut n = notification(text::app_name(lang));
    n.summary(&summary).body(&body);
    if folder.is_some() {
        n.action("default", text::open_folder(lang));
    }
    let Some(handle) = show(&n) else { return };
    if let Some(folder) = folder {
        // Waiting blocks until the notification closes, so it gets its own thread.
        std::thread::spawn(move || {
            handle.wait_for_action(|action| {
                if action == "default" {
                    let (program, args) = open_folder_command(CURRENT, &folder, file.as_deref());
                    let _ = std::process::Command::new(program).args(args).spawn();
                }
            });
        });
    }
}
