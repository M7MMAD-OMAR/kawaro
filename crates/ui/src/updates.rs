//! The "Updates" section model (S-090, backlog/S-090-settings-updates-about.md).
//!
//! Pure state and strings, shared by the standalone settings window and the
//! settings panel inside the welcome window. The window shows what the daemon
//! last said; it never decides anything about updates itself.

use crate::Lang;

/// The state of the update machine as the settings window shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateState {
    UpToDate,
    Checking,
    /// Percent downloaded, 0 to 100.
    Downloading(u8),
    /// A verified update waits for Update now.
    Ready,
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
    pub(crate) state_line: String,
    pub(crate) check_enabled: bool,
    pub(crate) apply_enabled: bool,
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
}

impl Default for UpdateModel {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            state: None,
            service: true,
            applying: false,
            deferred: false,
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
    }

    pub(crate) fn set_deferred(&mut self) {
        self.applying = false;
        self.deferred = true;
    }

    pub(crate) fn check_enabled(&self) -> bool {
        self.service && !matches!(self.state, Some(UpdateState::Checking))
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
        Rendered {
            version_text,
            state_line: self.state_line(ar),
            check_enabled: self.check_enabled(),
            apply_enabled: self.apply_enabled(),
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
            Some(UpdateState::Downloading(p)) => {
                if ar {
                    format!("جارٍ تنزيل التحديث، {p}%")
                } else {
                    format!("Downloading the update, {p}%")
                }
            }
            Some(UpdateState::Ready) => {
                pick("An update is ready to install.", "التحديث جاهز للتثبيت.")
            }
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

/// Maps the daemon's `Response::Update` status word to the shown state. The
/// daemon reports no percent, so `downloading` starts at 0.
pub fn state_from_status(status: &str, latest: Option<&str>) -> UpdateState {
    match status {
        "up-to-date" => UpdateState::UpToDate,
        "downloading" => UpdateState::Downloading(0),
        "ready" => UpdateState::Ready,
        "offline" | "unavailable" => UpdateState::Unavailable,
        other => UpdateState::Failed(match latest {
            Some(v) => format!("{other} ({v})"),
            None => other.to_string(),
        }),
    }
}
