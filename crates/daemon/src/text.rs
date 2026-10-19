//! Daemon strings in Arabic and English.

use sbarhive_core::MediaKind;
use sbarhive_core::i18n::Lang;

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

pub fn quit(lang: Lang) -> &'static str {
    match lang {
        Lang::Ar => "إنهاء صبار هايف",
        Lang::En => "Quit SbarHive",
    }
}

pub fn idle(lang: Lang) -> &'static str {
    match lang {
        Lang::Ar => "جاهز. حدّد ملفات في مدير الملفات وافتح صبار هايف من قائمة النقر اليمين.",
        Lang::En => {
            "Ready. Select files in your file manager and open SbarHive from the right-click menu."
        }
    }
}

pub fn app_name(lang: Lang) -> &'static str {
    match lang {
        Lang::Ar => "صبار هايف",
        Lang::En => "SbarHive",
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
    fn english_counts_pluralise() {
        assert_eq!(selection(Lang::En, 1, Some(MediaKind::Image)), "1 image");
        assert_eq!(selection(Lang::En, 4, Some(MediaKind::Video)), "4 videos");
        assert_eq!(selection(Lang::En, 2, None), "2 files");
    }
}
