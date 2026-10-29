//! The "Updates" section model (S-090, backlog/S-090-settings-updates-about.md).
//!
//! Pure state and strings, shared by the standalone settings window and the
//! settings panel inside the welcome window. The window shows what the daemon
//! last said; it never decides anything about updates itself.

use crate::Lang;
use kawaro_core::update::manifest::Notes;
use kawaro_core::update::whats_new;

/// The state of the update machine as the settings window shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateState {
    UpToDate,
    Checking,
    /// Percent downloaded, 0 to 100.
    Downloading(u8),
    /// A verified update waits for Update now.
    Ready,
    /// The install runs; the app restarts by itself (S-099 A3).
    Installing,
    /// The app restarted into this version (S-099 B5).
    Updated(String),
    /// A short reason.
    Failed(String),
    /// No curl, or offline.
    Unavailable,
}

/// The version string shared by the welcome footer and any About surface.
pub fn about_line(lang: Lang) -> String {
    let v = env!("CARGO_PKG_VERSION");
    match lang {
        Lang::Ar => format!("كوارو النسخة {v}"),
        Lang::En => format!("Kawaro version {v}"),
    }
}

/// What the section shows, ready to put on the window.
pub(crate) struct Rendered {
    pub(crate) version_text: String,
    /// `Available version: Y`, empty when the line is absent.
    pub(crate) available_text: String,
    pub(crate) state_line: String,
    /// The `Updated to X` line, empty outside the Updated state.
    pub(crate) banner: String,
    pub(crate) check_enabled: bool,
    pub(crate) apply_enabled: bool,
    pub(crate) cards: Vec<Card>,
    /// The available version's card is first (display order); the installed
    /// one is first in reading order for the tests.
    pub(crate) swapped: bool,
}

/// One "What is new" card, ready to draw.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Card {
    pub(crate) title: String,
    pub(crate) bullets: Vec<String>,
    pub(crate) label: String,
}

/// How often a window checks its update poll (the poll itself fires once a
/// second); the clock is monotonic milliseconds since the first call.
pub(crate) const POLL_TICK: std::time::Duration = std::time::Duration::from_millis(250);

pub(crate) fn clock_ms() -> u64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_millis() as u64
}

const MIB: f64 = 1024.0 * 1024.0;
const POLL_MS: u64 = 1000;
const NOTE_LINES: std::ops::RangeInclusive<usize> = 3..=5;
const NOTE_MAX_CHARS: usize = 80;

/// The lines of `notes` for `lang` when both languages obey the manifest
/// rule (3 to 5 lines, none empty, one line, at most 80 characters).
fn valid_lines(notes: &Notes, lang: Lang) -> Option<Vec<String>> {
    let ok = |lines: &[String]| {
        NOTE_LINES.contains(&lines.len())
            && lines.iter().all(|l| {
                !l.is_empty() && !l.contains(['\n', '\r']) && l.chars().count() <= NOTE_MAX_CHARS
            })
    };
    if !ok(&notes.en) || !ok(&notes.ar) {
        return None;
    }
    Some(match lang {
        Lang::En => notes.en.clone(),
        Lang::Ar => notes.ar.clone(),
    })
}

/// What the window knows about updates. Before the first answer there is no
/// state and the state line stays empty, rather than claiming "up to date".
pub(crate) struct UpdateModel {
    version: String,
    state: Option<UpdateState>,
    service: bool,
    /// Update now was pressed and no answer came back yet.
    applying: bool,
    /// The daemon answered that the install waits for running jobs.
    deferred: bool,
    /// The version, the download size and the notes of the known update.
    available: Option<String>,
    size: Option<u64>,
    notes: Notes,
    /// A check started while an update was known: its lines stay, so the
    /// rows below do not move.
    kept_available: bool,
    /// When the last poll fired, in the window clock (S-099 B3).
    poll_from: Option<u64>,
}

impl Default for UpdateModel {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            state: None,
            service: true,
            applying: false,
            deferred: false,
            available: None,
            size: None,
            notes: Notes::default(),
            kept_available: false,
            poll_from: None,
        }
    }
}

impl UpdateModel {
    pub(crate) fn set_view(&mut self, version: &str, state: UpdateState, service: bool) {
        self.version = version.to_string();
        self.state = Some(state);
        self.service = service;
        self.applying = false;
        self.deferred = false;
        self.kept_available = false;
    }

    /// The update the daemon knows about: its version, the download size in
    /// bytes and its notes. Independent of `set_view`.
    pub(crate) fn set_available(
        &mut self,
        version: Option<&str>,
        size: Option<u64>,
        notes: Option<&Notes>,
    ) {
        self.available = version.map(str::to_string);
        self.size = size;
        self.notes = notes.cloned().unwrap_or_default();
    }

    /// The window opened again: the banner and the poll clock start over.
    pub(crate) fn reset_view(&mut self) {
        if matches!(self.state, Some(UpdateState::Updated(_))) {
            self.state = None;
        }
        self.poll_from = None;
    }

    fn polling(&self) -> bool {
        self.service
            && matches!(
                self.state,
                Some(UpdateState::Downloading(_) | UpdateState::Installing)
            )
    }

    /// One tick of the window clock: true when the state asks the daemon
    /// again. The first tick after entering a polling state starts the clock.
    pub(crate) fn poll(&mut self, now_ms: u64) -> bool {
        if !self.polling() {
            self.poll_from = None;
            return false;
        }
        match self.poll_from {
            Some(from) if now_ms.saturating_sub(from) >= POLL_MS => {
                self.poll_from = Some(now_ms);
                true
            }
            Some(_) => false,
            None => {
                self.poll_from = Some(now_ms);
                false
            }
        }
    }

    /// Whether the available version, its line and its notes are shown.
    fn available_shown(&self) -> bool {
        self.available.is_some()
            && (matches!(
                self.state,
                Some(UpdateState::Downloading(_) | UpdateState::Ready | UpdateState::Installing)
            ) || (self.kept_available && self.state == Some(UpdateState::Checking)))
    }

    pub(crate) fn set_deferred(&mut self) {
        self.applying = false;
        self.deferred = true;
    }

    pub(crate) fn check_enabled(&self) -> bool {
        self.service
            && !matches!(
                self.state,
                Some(UpdateState::Checking | UpdateState::Downloading(_) | UpdateState::Installing)
            )
    }

    pub(crate) fn apply_enabled(&self) -> bool {
        self.service && self.state == Some(UpdateState::Ready) && !self.applying && !self.deferred
    }

    /// Update check pressed: the state line reads "checking" at once and the
    /// button stays disabled until the answer arrives.
    pub(crate) fn begin_check(&mut self) -> bool {
        if !self.check_enabled() {
            return false;
        }
        self.kept_available = self.available_shown();
        self.state = Some(UpdateState::Checking);
        self.deferred = false;
        true
    }

    pub(crate) fn begin_apply(&mut self) -> bool {
        if !self.apply_enabled() {
            return false;
        }
        self.applying = true;
        true
    }

    pub(crate) fn render(&self, lang: Lang) -> Rendered {
        let ar = matches!(lang, Lang::Ar);
        let version_text = if ar {
            format!("النسخة المثبّتة: {}", self.version)
        } else {
            format!("Installed version: {}", self.version)
        };
        let shown = self.available.as_deref().filter(|_| self.available_shown());
        let available_text = match (shown, ar) {
            (Some(v), true) => format!("النسخة المتاحة: {v}"),
            (Some(v), false) => format!("Available version: {v}"),
            (None, _) => String::new(),
        };
        let banner = match &self.state {
            Some(UpdateState::Updated(_)) => self.state_line(ar),
            _ => String::new(),
        };
        let mut cards = Vec::new();
        let installed = whats_new::installed(lang);
        if !installed.is_empty() {
            cards.push(card(lang, &self.version, installed));
        }
        // The notes of the version that matters come first: the available
        // version while it downloads, waits or installs.
        let mut swapped = false;
        if let (Some(v), Some(lines)) = (shown, valid_lines(&self.notes, lang)) {
            cards.push(card(lang, v, lines));
            if cards.len() == 2 {
                cards.swap(0, 1);
                swapped = true;
            }
        }
        Rendered {
            version_text,
            available_text,
            state_line: self.state_line(ar),
            banner,
            check_enabled: self.check_enabled(),
            apply_enabled: self.apply_enabled(),
            cards,
            swapped,
        }
    }

    /// `Downloading 0.4.3: 42% (21.0 of 50.0 MB)`; the parenthesis is left
    /// out when the size is unknown.
    fn downloading_line(&self, ar: bool, percent: u8) -> String {
        let Some(v) = &self.available else {
            return if ar {
                format!("جارٍ تنزيل التحديث، {percent}%")
            } else {
                format!("Downloading the update, {percent}%")
            };
        };
        let sizes = self.size.map(|total| {
            let done = u128::from(total) * u128::from(percent) / 100;
            (done as f64 / MIB, total as f64 / MIB)
        });
        match (ar, sizes) {
            (true, Some((d, t))) => {
                format!("جارٍ تنزيل {v}: {percent}% ({d:.1} من {t:.1} ميغابايت)")
            }
            (true, None) => format!("جارٍ تنزيل {v}: {percent}%"),
            (false, Some((d, t))) => format!("Downloading {v}: {percent}% ({d:.1} of {t:.1} MB)"),
            (false, None) => format!("Downloading {v}: {percent}%"),
        }
    }

    fn state_line(&self, ar: bool) -> String {
        let pick = |en: &str, ar_text: &str| (if ar { ar_text } else { en }).to_string();
        if !self.service {
            return pick(
                "The background service is off, so updates cannot be checked.",
                "الخدمة الخلفية متوقفة، لذا لا يمكن البحث عن تحديث.",
            );
        }
        if self.deferred {
            return pick(
                "The update will install when the running jobs finish.",
                "سيُثبَّت التحديث عند انتهاء المهام الجارية.",
            );
        }
        match &self.state {
            None => String::new(),
            Some(UpdateState::UpToDate) => pick("You are up to date.", "نسختك محدّثة."),
            Some(UpdateState::Checking) => pick("Checking for updates.", "جارٍ البحث عن تحديث."),
            Some(UpdateState::Downloading(p)) => self.downloading_line(ar, *p),
            Some(UpdateState::Ready) => match &self.available {
                Some(v) => {
                    if ar {
                        format!("اكتمل التنزيل. التحديث {v} جاهز للتثبيت.")
                    } else {
                        format!("Download finished. Ready to install {v}.")
                    }
                }
                None => pick("An update is ready to install.", "التحديث جاهز للتثبيت."),
            },
            Some(UpdateState::Installing) => match &self.available {
                Some(v) => {
                    if ar {
                        format!("جارٍ تثبيت {v}. سيعيد كوارو التشغيل تلقائياً.")
                    } else {
                        format!("Installing {v}. Kawaro will restart by itself.")
                    }
                }
                None => pick(
                    "Installing the update. Kawaro will restart by itself.",
                    "جارٍ تثبيت التحديث. سيعيد كوارو التشغيل تلقائياً.",
                ),
            },
            Some(UpdateState::Updated(v)) => {
                if ar {
                    format!("تم التحديث إلى {v}")
                } else {
                    format!("Updated to {v}")
                }
            }
            Some(UpdateState::Failed(reason)) if reason.is_empty() => pick(
                "The update could not be downloaded or installed. Check for updates to try again.",
                "تعذّر تنزيل التحديث أو تثبيته. ابحث عن تحديث للمحاولة مرة أخرى.",
            ),
            Some(UpdateState::Failed(reason)) => {
                if ar {
                    format!("تعذّر التحديث: {reason}")
                } else {
                    format!("The update did not finish: {reason}")
                }
            }
            Some(UpdateState::Unavailable) => pick(
                "Updates are unavailable now (offline, or curl is missing).",
                "التحديثات غير متاحة الآن (لا اتصال، أو curl غير مثبّت).",
            ),
        }
    }
}

/// One card: the title with the version and its bullets, labelled for a
/// screen reader with both.
fn card(lang: Lang, version: &str, bullets: Vec<String>) -> Card {
    let title = match lang {
        Lang::En => format!("What is new in {version}"),
        Lang::Ar => format!("الجديد في {version}"),
    };
    let label = format!("{title}: {}", bullets.join("; "));
    Card {
        title,
        bullets,
        label,
    }
}

/// Maps the daemon's `Response::Update` status word to the shown state. The
/// percent comes separately, so `downloading` starts at 0 here.
pub fn state_from_status(status: &str, latest: Option<&str>) -> UpdateState {
    match status {
        "up-to-date" => UpdateState::UpToDate,
        "downloading" => UpdateState::Downloading(0),
        "ready" => UpdateState::Ready,
        "installing" => UpdateState::Installing,
        // A plain message in the window's language; no raw status word.
        "failed" | "rejected" => UpdateState::Failed(String::new()),
        "offline" | "unavailable" => UpdateState::Unavailable,
        other => UpdateState::Failed(match latest {
            Some(v) => format!("{other} ({v})"),
            None => other.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The window clock counts real milliseconds, so the one second poll
    /// is a second and not a constant.
    #[test]
    fn the_window_clock_advances_with_real_time() {
        let a = clock_ms();
        std::thread::sleep(std::time::Duration::from_millis(30));
        let b = clock_ms();
        assert!(b >= a + 25, "{a} then {b}");
    }

    fn model_with_update() -> UpdateModel {
        let mut m = UpdateModel::default();
        m.set_available(Some("0.4.3"), Some(1024), None);
        m
    }

    /// A check pressed while an update is known keeps its lines, so the page
    /// does not jump; a check from any other state shows none.
    #[test]
    fn a_recheck_keeps_the_available_lines_only_when_they_were_shown() {
        let mut m = model_with_update();
        m.set_view("0.4.2", UpdateState::Ready, true);
        assert!(m.begin_check());
        assert_eq!(m.state, Some(UpdateState::Checking));
        assert!(m.available_shown(), "kept while re-checking");
        assert_eq!(
            m.render(Lang::En).available_text,
            "Available version: 0.4.3"
        );
        // Checking reached by set_view (not a press) shows nothing.
        m.set_view("0.4.2", UpdateState::Checking, true);
        assert!(!m.available_shown());
        // A check from a state that showed nothing keeps nothing.
        m.set_view("0.4.2", UpdateState::UpToDate, true);
        assert!(m.begin_check());
        assert!(!m.available_shown());
        // The kept lines end with the answer.
        let mut m = model_with_update();
        m.set_view("0.4.2", UpdateState::Ready, true);
        m.begin_check();
        m.set_view("0.4.2", UpdateState::UpToDate, true);
        assert!(!m.available_shown());
    }

    /// An empty failure reason is the plain sentence; a given reason is
    /// shown as it is.
    #[test]
    fn a_failure_reason_is_shown_and_an_empty_one_reads_plainly() {
        let mut m = UpdateModel::default();
        m.set_view("0.4.2", UpdateState::Failed("disk full".into()), true);
        let en = m.render(Lang::En).state_line;
        let ar = m.render(Lang::Ar).state_line;
        assert!(
            en.contains("disk full") && ar.contains("disk full"),
            "{en} {ar}"
        );
        m.set_view("0.4.2", UpdateState::Failed(String::new()), true);
        let plain = m.render(Lang::En).state_line;
        assert_ne!(plain, en);
        assert!(
            !plain.contains("disk full") && !plain.ends_with(": "),
            "{plain}"
        );
        assert_ne!(m.render(Lang::Ar).state_line, ar);
    }
}
