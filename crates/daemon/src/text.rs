//! Daemon strings in Arabic and English.

use crate::notify::UpdateKind;
use crate::progress::percent;
use crate::update::{Phase, UpdateState};
use kawaro_core::MediaKind;
use kawaro_core::i18n::Lang;

/// Arabic counted noun: 1 and 2 have their own forms, 3..=10 take the plural,
/// 11 and above take the singular accusative-like form used in everyday counts.
fn ar_count(n: usize, one: &str, two: &str, plural: &str, many: &str) -> String {
    match n {
        1 => format!("{one} واحد{}", if one.ends_with('ة') { "ة" } else { "" }),
        2 => two.to_string(),
        3..=10 => format!("{n} {plural}"),
        _ => format!("{n} {many}"),
    }
}

/// Centre text of the hive: how many files of which kind are selected.
pub fn selection(lang: Lang, n: usize, kind: Option<MediaKind>) -> String {
    match lang {
        Lang::Ar => match kind {
            Some(MediaKind::Image) => ar_count(n, "صورة", "صورتان", "صور", "صورة"),
            Some(MediaKind::Video) => {
                ar_count(n, "فيديو", "مقطعا فيديو", "مقاطع فيديو", "مقطع فيديو")
            }
            Some(MediaKind::Audio) => {
                ar_count(n, "ملف صوتي", "ملفان صوتيان", "ملفات صوتية", "ملفاً صوتياً")
            }
            Some(MediaKind::Document) => ar_count(n, "مستند", "مستندان", "مستندات", "مستنداً"),
            Some(MediaKind::Pdf) => ar_count(n, "ملف PDF", "ملفا PDF", "ملفات PDF", "ملف PDF"),
            _ => ar_count(n, "ملف", "ملفان", "ملفات", "ملفاً"),
        },
        Lang::En => {
            let (one, many) = match kind {
                Some(MediaKind::Image) => ("image", "images"),
                Some(MediaKind::Video) => ("video", "videos"),
                Some(MediaKind::Audio) => ("audio file", "audio files"),
                Some(MediaKind::Document) => ("document", "documents"),
                Some(MediaKind::Pdf) => ("PDF", "PDFs"),
                _ => ("file", "files"),
            };
            format!("{n} {}", if n == 1 { one } else { many })
        }
    }
}

pub fn no_actions(lang: Lang) -> &'static str {
    match lang {
        Lang::Ar => "لا توجد إجراءات لهذه الملفات",
        Lang::En => "No actions for these files",
    }
}

/// Centre text of a drop-mode hive while it waits for the drag (S-014c AC10).
pub fn drop_waiting(lang: Lang) -> &'static str {
    match lang {
        Lang::Ar => "أفلت الملف هنا",
        Lang::En => "Drop files here",
    }
}

pub fn done(lang: Lang, ok: usize) -> String {
    match lang {
        Lang::Ar => format!("اكتمل: {}", ar_count(ok, "ملف", "ملفان", "ملفات", "ملفاً")),
        Lang::En => format!("Done: {ok} {}", if ok == 1 { "file" } else { "files" }),
    }
}

pub fn failed(lang: Lang, failed: usize, total: usize) -> String {
    match lang {
        Lang::Ar => format!("تعذّر إنجاز {failed} من {total}"),
        Lang::En => format!("{failed} of {total} failed"),
    }
}

pub fn cancelled(lang: Lang) -> &'static str {
    match lang {
        Lang::Ar => "أُلغيت المهمة",
        Lang::En => "Cancelled",
    }
}

pub fn open_folder(lang: Lang) -> &'static str {
    match lang {
        Lang::Ar => "فتح المجلد",
        Lang::En => "Open folder",
    }
}

pub fn cancel_all(lang: Lang) -> &'static str {
    match lang {
        Lang::Ar => "إلغاء كل المهام",
        Lang::En => "Cancel all jobs",
    }
}

pub fn settings(lang: Lang) -> &'static str {
    match lang {
        Lang::Ar => "الإعدادات",
        Lang::En => "Settings",
    }
}

/// The tray item that opens the first-run welcome window again (S-087 AC1),
/// at any time and regardless of the marker.
pub fn get_started(lang: Lang) -> &'static str {
    match lang {
        Lang::Ar => "ابدأ هنا",
        Lang::En => "Get started",
    }
}

pub fn quit(lang: Lang) -> &'static str {
    match lang {
        Lang::Ar => "إنهاء كوارو",
        Lang::En => "Quit Kawaro",
    }
}

pub fn idle(lang: Lang) -> &'static str {
    match lang {
        Lang::Ar => "جاهز. حدّد ملفات في مدير الملفات وافتح كوارو من قائمة النقر اليمين.",
        Lang::En => {
            "Ready. Select files in your file manager and open Kawaro from the right-click menu."
        }
    }
}

pub fn app_name(lang: Lang) -> &'static str {
    match lang {
        Lang::Ar => "كوارو",
        Lang::En => "Kawaro",
    }
}

/// The notification text of an update (S-054b).
pub fn update_message(lang: Lang, kind: UpdateKind, version: &str) -> String {
    match (lang, kind) {
        (Lang::Ar, UpdateKind::Ready) => {
            format!("التحديث {version} جاهز. اختر «حدّث الآن» من قائمة الدرج.")
        }
        (Lang::Ar, UpdateKind::Updated) => format!("تم تحديث كوارو إلى {version}."),
        (Lang::Ar, UpdateKind::Failed) => {
            format!("تعذّر التحديث {version}. النسخة الحالية لم تتغير.")
        }
        (Lang::En, UpdateKind::Ready) => {
            format!("Update {version} is ready. Choose Update now in the tray menu.")
        }
        (Lang::En, UpdateKind::Updated) => format!("Kawaro was updated to {version}."),
        (Lang::En, UpdateKind::Failed) => {
            format!("Update {version} failed. The current version is unchanged.")
        }
    }
}

/// The tray entry that installs a ready update.
pub fn update_now(lang: Lang) -> &'static str {
    match lang {
        Lang::Ar => "حدّث الآن",
        Lang::En => "Update now",
    }
}

/// What the tray entry does, for the tooltip next to it.
pub fn update_now_hint(lang: Lang) -> &'static str {
    match lang {
        Lang::Ar => "ثبّت التحديث الذي نُزّل ثم أعد تشغيل كوارو",
        Lang::En => "Install the downloaded update and restart Kawaro",
    }
}

/// What to tell the user when the update cannot be installed from here:
/// `how` is the command to run or the page to open.
pub fn update_manual(lang: Lang, how: &str) -> String {
    match lang {
        Lang::Ar => format!("لتثبيت التحديث نفّذ: {how}"),
        Lang::En => format!("To install the update, run: {how}"),
    }
}

/// The tray menu labels in order. The first is always the update entry;
/// `update_item_enabled` says whether it can be chosen. S-087 D7: the
/// second is the Get started item.
pub fn menu_labels(lang: Lang, _update: &UpdateState) -> Vec<String> {
    [update_now, get_started, cancel_all, settings, quit]
        .iter()
        .map(|label| label(lang).to_string())
        .collect()
}

/// The update entry is enabled only when a verified package is ready.
pub fn update_item_enabled(update: &UpdateState) -> bool {
    matches!(update.phase, Phase::Ready(_))
}

/// The tooltip lines: the download progress of an update (only a
/// `Downloading` phase counts), then one line per running job; the idle text
/// when there is nothing to show.
fn tooltip_lines(lang: Lang, phase: Option<&Phase>, active: &[(String, f32)]) -> String {
    let mut lines = Vec::new();
    if let Some(Phase::Downloading {
        downloaded, size, ..
    }) = phase
    {
        let pct = percent(*downloaded, *size);
        lines.push(match lang {
            Lang::Ar => format!("تنزيل التحديث {pct}٪"),
            Lang::En => format!("Downloading update {pct}%"),
        });
    }
    lines.extend(
        active
            .iter()
            .map(|(label, f)| format!("{label}: {}%", (f * 100.0).round() as i32)),
    );
    if lines.is_empty() {
        idle(lang).to_string()
    } else {
        lines.join("\n")
    }
}

/// The tray tooltip from the update state.
pub fn tooltip_text(lang: Lang, update: &UpdateState, active: &[(String, f32)]) -> String {
    tooltip_lines(lang, Some(&update.phase), active)
}

/// The tray tooltip when the update lock may be busy: a running download
/// (`progress`) wins; otherwise the state when the lock was got; otherwise
/// only the job lines or the idle text.
pub fn tooltip_from(
    lang: Lang,
    update: Option<&UpdateState>,
    progress: Option<&Phase>,
    active: &[(String, f32)],
) -> String {
    match progress {
        Some(phase) => tooltip_lines(lang, Some(phase), active),
        None => update.map_or_else(
            || tooltip_lines(lang, None, active),
            |update| tooltip_text(lang, update, active),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arabic_counts_follow_number_agreement() {
        let img = Some(MediaKind::Image);
        assert_eq!(selection(Lang::Ar, 1, img), "صورة واحدة");
        assert_eq!(selection(Lang::Ar, 2, img), "صورتان");
        assert_eq!(selection(Lang::Ar, 3, img), "3 صور");
        assert_eq!(selection(Lang::Ar, 10, img), "10 صور");
        assert_eq!(selection(Lang::Ar, 11, img), "11 صورة");
        assert_eq!(selection(Lang::Ar, 1, None), "ملف واحد");
        assert_eq!(selection(Lang::Ar, 12, None), "12 ملفاً");
    }

    #[test]
    fn every_kind_has_its_own_noun_in_both_languages() {
        use MediaKind::*;
        let ar = |k| selection(Lang::Ar, 3, Some(k));
        assert_eq!(ar(Video), "3 مقاطع فيديو");
        assert_eq!(ar(Audio), "3 ملفات صوتية");
        assert_eq!(ar(Document), "3 مستندات");
        assert_eq!(ar(Pdf), "3 ملفات PDF");
        assert_eq!(ar(Other), "3 ملفات");
        assert_eq!(selection(Lang::Ar, 1, Some(Video)), "فيديو واحد");
        assert_eq!(selection(Lang::Ar, 2, Some(Audio)), "ملفان صوتيان");
        assert_eq!(selection(Lang::Ar, 12, Some(Document)), "12 مستنداً");
        let en = |k| selection(Lang::En, 1, Some(k));
        assert_eq!(en(Audio), "1 audio file");
        assert_eq!(en(Document), "1 document");
        assert_eq!(en(Pdf), "1 PDF");
        assert_eq!(en(Other), "1 file");
        assert_eq!(selection(Lang::En, 2, Some(Audio)), "2 audio files");
        assert_eq!(selection(Lang::En, 2, Some(Document)), "2 documents");
        assert_eq!(selection(Lang::En, 2, Some(Pdf)), "2 PDFs");
        assert_eq!(selection(Lang::En, 2, Some(Image)), "2 images");
    }

    #[test]
    fn fixed_strings_in_both_languages() {
        type Text = fn(Lang) -> &'static str;
        let cases: [(Text, &str, &str); 8] = [
            (
                no_actions,
                "لا توجد إجراءات لهذه الملفات",
                "No actions for these files",
            ),
            (cancelled, "أُلغيت المهمة", "Cancelled"),
            (open_folder, "فتح المجلد", "Open folder"),
            (cancel_all, "إلغاء كل المهام", "Cancel all jobs"),
            (settings, "الإعدادات", "Settings"),
            (quit, "إنهاء كوارو", "Quit Kawaro"),
            (app_name, "كوارو", "Kawaro"),
            (
                idle,
                "جاهز. حدّد ملفات في مدير الملفات وافتح كوارو من قائمة النقر اليمين.",
                "Ready. Select files in your file manager and open Kawaro from the right-click menu.",
            ),
        ];
        for (f, ar, en) in cases {
            assert_eq!(f(Lang::Ar), ar);
            assert_eq!(f(Lang::En), en);
        }
    }

    #[test]
    fn job_summaries_in_both_languages() {
        assert_eq!(done(Lang::Ar, 1), "اكتمل: ملف واحد");
        assert_eq!(done(Lang::Ar, 3), "اكتمل: 3 ملفات");
        assert_eq!(done(Lang::En, 1), "Done: 1 file");
        assert_eq!(done(Lang::En, 3), "Done: 3 files");
        assert_eq!(failed(Lang::Ar, 1, 3), "تعذّر إنجاز 1 من 3");
        assert_eq!(failed(Lang::En, 1, 3), "1 of 3 failed");
    }

    /// S-018 AC25: with several files the centre's accessible name is this
    /// text, so the Arabic number forms must hold for a mixed selection (no
    /// common kind) and for `Other`: 2 "ملفان", 3 "3 ملفات", 12 "12 ملفاً".
    #[test]
    fn ac25_several_files_name_the_centre_with_the_arabic_number_forms() {
        for kind in [None, Some(MediaKind::Other)] {
            assert_eq!(selection(Lang::Ar, 2, kind), "ملفان");
            assert_eq!(selection(Lang::Ar, 3, kind), "3 ملفات");
            assert_eq!(selection(Lang::Ar, 12, kind), "12 ملفاً");
            assert_eq!(selection(Lang::En, 2, kind), "2 files");
            assert_eq!(selection(Lang::En, 12, kind), "12 files");
        }
    }

    #[test]
    fn english_counts_pluralise() {
        assert_eq!(selection(Lang::En, 1, Some(MediaKind::Image)), "1 image");
        assert_eq!(selection(Lang::En, 4, Some(MediaKind::Video)), "4 videos");
        assert_eq!(selection(Lang::En, 2, None), "2 files");
    }

    // S-014c AC10: the centre text of a drop-mode hive while it waits for the
    // drag (`kawaro drop`, before enter). The wording is the implementer's;
    // the test pins that both languages exist and are distinct.
    #[test]
    fn ac10_drop_waiting_text_in_both_languages() {
        let (ar, en): (&'static str, &'static str) =
            (drop_waiting(Lang::Ar), drop_waiting(Lang::En));
        for t in [ar, en] {
            assert!(!t.trim().is_empty(), "empty waiting text");
            assert_eq!(t, t.trim(), "no padding: {t:?}");
            assert!(
                !t.contains('\u{2014}') && !t.contains('\u{2013}'),
                "no long dash: {t:?}"
            );
        }
        assert!(
            ar.chars().any(|c| ('\u{0621}'..='\u{064A}').contains(&c)),
            "Arabic letters in {ar:?}"
        );
        assert!(
            en.chars().any(|c| c.is_ascii_alphabetic()),
            "Latin letters in {en:?}"
        );
        assert!(
            !en.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c)),
            "no Arabic letters in the English text {en:?}"
        );
        assert_ne!(ar, en);
        // Not a string the hive already shows for something else.
        for other in [
            no_actions(Lang::Ar),
            no_actions(Lang::En),
            idle(Lang::Ar),
            idle(Lang::En),
        ] {
            assert_ne!(ar, other);
            assert_ne!(en, other);
        }
    }
}
