//! Sample actions shared by the examples: a selection of three images.

#![allow(dead_code)]

use sbarhive_ui::{HiveCell, Lang};

pub fn lang_from_args(args: &[String]) -> Lang {
    args.windows(2)
        .find(|w| w[0] == "--lang")
        .map(|w| Lang::parse(&w[1]))
        .unwrap_or(Lang::En)
}

pub fn centre_text(lang: Lang) -> &'static str {
    match lang {
        Lang::Ar => "3 صور",
        Lang::En => "3 images",
    }
}

/// Three conversions in ring 1, five transforms in ring 2.
pub fn image_actions(lang: Lang) -> Vec<HiveCell> {
    let ar = lang == Lang::Ar;
    // (id, latin code, arabic code, english label, arabic label, ring)
    let rows = [
        (
            "image.to-webp",
            "WEBP",
            "WEBP",
            "Convert to WebP",
            "تحويل إلى WebP",
            1,
        ),
        (
            "image.to-png",
            "PNG",
            "PNG",
            "Convert to PNG",
            "تحويل إلى PNG",
            1,
        ),
        (
            "image.to-jpg",
            "JPG",
            "JPG",
            "Convert to JPEG",
            "تحويل إلى JPEG",
            1,
        ),
        (
            "image.resize",
            "RESIZE",
            "تحجيم",
            "Resize to half",
            "تصغير إلى النصف",
            2,
        ),
        (
            "image.rotate",
            "ROTATE",
            "تدوير",
            "Rotate 90 degrees",
            "تدوير 90 درجة",
            2,
        ),
        ("image.gray", "GRAY", "رمادي", "Grayscale", "تدرج رمادي", 2),
        (
            "image.to-pdf",
            "PDF",
            "PDF",
            "Combine into one PDF",
            "دمج في ملف PDF واحد",
            2,
        ),
        (
            "image.strip",
            "STRIP",
            "تنظيف",
            "Remove metadata",
            "حذف البيانات الوصفية",
            2,
        ),
    ];
    rows.iter()
        .map(|(id, latin, arabic, en, ar_label, ring)| HiveCell {
            id: (*id).into(),
            code: if ar { *arabic } else { *latin }.into(),
            label: if ar { *ar_label } else { *en }.into(),
            ring: *ring,
        })
        .collect()
}
