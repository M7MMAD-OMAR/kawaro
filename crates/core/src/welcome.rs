//! The first-run welcome (S-087, backlog/S-087-first-run-welcome.md): the
//! pure routing of the `settings` entry on the one-shot `welcome_seen`
//! marker, the bilingual window strings, and the demonstration captions
//! resolved from the registration result only. Pure, so the first-run rules
//! are tested on Linux; the daemon and the window consume exactly this
//! logic and never invent their own.

use crate::config::Config;
use crate::i18n::Lang;

/// What actually opens the hive while a document is dragged: the
/// registration result the caption reads (AC3), never the configured key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Registration {
    /// The S-084 manager decision D3, the Windows trigger: the right-button
    /// drag, which is a button and never a key.
    RightDrag,
    /// A measured native binding (S-075): `key` is held while dragging.
    Native(String),
    /// The named portal fallback of S-075 (for example `Super+Z`): `key` is
    /// held while dragging.
    Fallback(String),
    /// Nothing is registered or known: no key can be named.
    Unbound,
}

/// Where the `settings` entry goes (AC1, D5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    /// The first-run welcome window.
    Welcome,
    /// The settings window of S-076.
    Settings,
}

/// The routing is a pure function of the marker (D5).
pub fn route(config: &Config) -> Route {
    if config.welcome_seen {
        Route::Settings
    } else {
        Route::Welcome
    }
}

/// Every visible string of the window that is not a caption.
pub struct Strings {
    pub title: String,
    pub try_button: String,
    pub try_accessible: String,
    pub settings_button: String,
    pub settings_accessible: String,
}

/// The window strings (AC4 and AC5 pin the button wording).
pub fn strings(lang: Lang) -> Strings {
    match lang {
        Lang::En => Strings {
            title: "Welcome to Kawaro".to_string(),
            try_button: "Try on an image".to_string(),
            try_accessible: "Try Kawaro on a sample image, copied for you to a temporary folder"
                .to_string(),
            settings_button: "Open Settings".to_string(),
            settings_accessible: "Open the Kawaro settings window".to_string(),
        },
        Lang::Ar => Strings {
            title: "أهلاً بك في كوارو".to_string(),
            try_button: "جرّب على صورة".to_string(),
            try_accessible: "جرّب كوارو على صورة تجريبية تُنسخ لك إلى مجلد مؤقت".to_string(),
            settings_button: "افتح الإعدادات".to_string(),
            settings_accessible: "افتح نافذة إعدادات كوارو".to_string(),
        },
    }
}

/// The five caption steps under the demonstration. `steps[1]` is the
/// trigger step of AC3, resolved from [`Registration`].
#[derive(Clone, Debug)]
pub struct Captions {
    pub steps: [String; 5],
}

/// The demonstration captions (AC3): the trigger step names exactly the
/// registration result. `RightDrag` uses the wording "drag with the right
/// button", a named key is named, and no registration gives the key-free
/// wording (drag, drop on the cell).
pub fn captions(lang: Lang, registration: &Registration) -> Captions {
    let trigger = match (lang, registration) {
        (Lang::En, Registration::RightDrag) => "Drag with the right button".to_string(),
        (Lang::Ar, Registration::RightDrag) => "اسحب بالزر الأيمن".to_string(),
        (Lang::En, Registration::Native(key) | Registration::Fallback(key)) => {
            format!("Hold {key} while you drag")
        }
        (Lang::Ar, Registration::Native(key) | Registration::Fallback(key)) => {
            format!("استمر بالضغط على {key} أثناء السحب")
        }
        (Lang::En, Registration::Unbound) => "Drag the file and drop it on a cell".to_string(),
        (Lang::Ar, Registration::Unbound) => "اسحب الملف وأفلته على الخلية".to_string(),
    };
    match lang {
        Lang::En => Captions {
            steps: [
                "Drag a file out of your file manager".to_string(),
                trigger,
                "The hive shows what fits your file".to_string(),
                "Drop on a cell to choose what happens".to_string(),
                "The result appears beside the untouched original".to_string(),
            ],
        },
        Lang::Ar => Captions {
            steps: [
                "اسحب ملفاً من مدير الملفات".to_string(),
                trigger,
                "تظهر الخلية ما يناسب ملفك".to_string(),
                "أفلت على الخلية لتختار ما يحدث".to_string(),
                "يظهر الناتج بجانب الأصل دون تغيير".to_string(),
            ],
        },
    }
}
