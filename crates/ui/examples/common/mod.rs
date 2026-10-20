//! Sample actions shared by the examples: a selection of three images.

#![allow(dead_code)]

use sbarhive_ui::{Hive, HiveCell, Lang, thumb};
use std::path::PathBuf;

pub fn lang_from_args(args: &[String]) -> Lang {
    args.windows(2)
        .find(|w| w[0] == "--lang")
        .map(|w| Lang::parse(&w[1]))
        .unwrap_or(Lang::En)
}

/// Every `--file PATH` in the arguments, in order.
pub fn files_from_args(args: &[String]) -> Vec<PathBuf> {
    args.windows(2)
        .filter(|w| w[0] == "--file")
        .map(|w| PathBuf::from(&w[1]))
        .collect()
}

/// Gives the hive its thumbnail cache and MIME globs from this process's
/// environment, as the daemon does (S-018h).
pub fn preview_source(hive: &Hive) {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let thumbnails = thumb::thumbnails_dir(std::env::var_os("XDG_CACHE_HOME").as_deref(), &home);
    let dirs = thumb::data_dirs(
        std::env::var_os("XDG_DATA_HOME").as_deref(),
        std::env::var_os("XDG_DATA_DIRS").as_deref(),
        &home,
    );
    hive.set_preview_source(thumbnails, thumb::Globs::load(&dirs));
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
            icon: icon_for(id).into(),
            id: (*id).into(),
            code: if ar { *arabic } else { *latin }.into(),
            label: if ar { *ar_label } else { *en }.into(),
            ring: *ring,
        })
        .collect()
}

/// The icon each sample action uses, as in actions/images.toml.
fn icon_for(id: &str) -> &'static str {
    match id {
        "image.to-webp" | "image.to-png" | "image.to-jpg" => "image",
        "image.resize" => "resize",
        "image.rotate" => "rotate",
        "image.gray" => "grayscale",
        "image.to-pdf" => "pdf",
        "image.strip" => "clean",
        _ => "",
    }
}
