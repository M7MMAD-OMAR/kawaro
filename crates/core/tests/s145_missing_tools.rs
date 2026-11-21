//! S145-A: a missing Poppler or qpdf tool gets a truthful reason.
//!
//! The unsupported-file classifier in `kawaro_core::unsupported` knows the
//! engine ids of the general engines only, so a selection whose only
//! accepting action needs Poppler or qpdf was reported as an unsupported
//! format instead of a missing tool. These cases pin the new mapping, the
//! curated help of the two tools and the missing against runnable
//! classification.
//!
//! The cases use a registry of their own with one action, so the built in
//! PDF actions, which run on the pure Rust `pdf` engine, cannot hide the
//! missing engine branch. Nothing is executed: the classifier, the hint
//! table and the message templates are pure.

use kawaro_core::Registry;
use kawaro_core::i18n::Lang;
use kawaro_core::unsupported::{
    CapabilityReport, Outcome, ProbeCache, ProbeOutcome, UnavailableReason, arabic_plural,
    capability_reports, classify, default_engine_reason, help_message, install_hint,
    install_hint_for_help, install_hints, missing_engine_message, notification, reason_message,
    tool_name, unavailable_actions_hint,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

/// One action, so the registry holds nothing else.
fn act(id: &str, engine: &str, accepts: &str, label: &str) -> String {
    format!(
        r#"
[[action]]
id = "{id}"
engine = "{engine}"
op = "convert"
ring = 1
accepts = ["{accepts}"]
output = {{ mode = "per-file", ext = "out" }}
code = {{ ar = "ن", en = "N" }}
label = {{ ar = "{label}", en = "{label}" }}
"#
    )
}

fn registry(actions: &[(&str, &str, &str)]) -> Registry {
    let mut reg = Registry::default();
    let text: String = actions
        .iter()
        .map(|(id, engine, accepts)| act(id, engine, accepts, "PDF action"))
        .collect::<Vec<_>>()
        .join("\n");
    reg.add_toml("s145.toml", &text).unwrap();
    reg
}

fn files(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(PathBuf::from).collect()
}

/// The engine ids that are available in a case.
fn available(ids: &'static [&'static str]) -> impl Fn(&str) -> bool {
    move |id: &str| ids.contains(&id)
}

/// No engine at all: every action is missing its tool.
const NONE: &[&str] = &[];

// ------------------------------------------------------------ mapping

#[test]
fn ac1_poppler_and_qpdf_map_to_their_tool_names() {
    assert_eq!(tool_name("poppler"), Some("Poppler"));
    assert_eq!(tool_name("qpdf"), Some("qpdf"));
    assert_eq!(tool_name("ocr"), Some("Tesseract"));
    // The other mappings keep their published names.
    assert_eq!(tool_name("ffmpeg"), Some("FFmpeg"));
    assert_eq!(tool_name("office"), Some("LibreOffice"));
    assert_eq!(tool_name("ghostscript"), Some("Ghostscript"));
    // An engine that needs no external tool has no tool name.
    assert_eq!(tool_name("pdf"), None);
    assert_eq!(tool_name("image"), None);
}

// ------------------------------------------------------------ hints

#[test]
fn ac2_the_pdf_tools_have_a_curated_page_and_a_package() {
    let hints = install_hints();
    let tools: Vec<&str> = hints.iter().map(|h| h.tool).collect();
    assert_eq!(
        tools,
        [
            "FFmpeg",
            "LibreOffice",
            "Ghostscript",
            "Poppler",
            "qpdf",
            "Tesseract",
            "OCRmyPDF"
        ]
    );
    let poppler = install_hint("Poppler").expect("the Poppler hint");
    let qpdf = install_hint("qpdf").expect("the qpdf hint");
    assert_eq!(poppler.url, "https://poppler.freedesktop.org/");
    assert_eq!(poppler.package, "poppler-utils");
    assert_eq!(qpdf.url, "https://qpdf.readthedocs.io/");
    assert_eq!(qpdf.package, "qpdf");
    for h in install_hints() {
        assert!(h.url.starts_with("https://"), "{}", h.url);
        assert!(!h.package.is_empty());
        assert!(
            h.package
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "{}",
            h.package
        );
    }
}

#[test]
fn ac3_one_complete_table_names_every_tool_once() {
    let tools: Vec<&str> = install_hints().iter().map(|h| h.tool).collect();
    assert_eq!(
        tools,
        [
            "FFmpeg",
            "LibreOffice",
            "Ghostscript",
            "Poppler",
            "qpdf",
            "Tesseract",
            "OCRmyPDF"
        ]
    );
    // Every tool is named once in the same public table used by messages.
    let mut unique = tools.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), tools.len());
}

#[test]
fn ac4_the_message_names_the_tool_the_package_and_the_page() {
    for (tool, package, url) in [
        (
            "Poppler",
            "poppler-utils",
            "https://poppler.freedesktop.org/",
        ),
        ("qpdf", "qpdf", "https://qpdf.readthedocs.io/"),
        (
            "Tesseract",
            "tesseract-ocr",
            "https://tesseract-ocr.github.io/tessdoc/",
        ),
    ] {
        let en = missing_engine_message(tool, Lang::En);
        assert!(en.contains(tool), "{en}");
        assert!(en.contains(package), "{en}");
        assert!(en.contains(url), "{en}");
        let ar = missing_engine_message(tool, Lang::Ar);
        assert!(ar.contains(tool), "{ar}");
        assert!(ar.contains(package), "{ar}");
        assert!(ar.contains(url), "{ar}");
        assert_ne!(ar, en);
        // No long dashes and no Arabic Indic digits in either language.
        for s in [&en, &ar] {
            assert!(!s.contains('\u{2013}') && !s.contains('\u{2014}'), "{s}");
            assert!(
                !s.chars().any(|c| ('\u{0660}'..='\u{0669}').contains(&c)),
                "{s}"
            );
        }
        // The notification of a missing tool keeps the same wording.
        let outcome = Outcome::MissingEngine { tool };
        let expected = format!("{tool} is not installed");
        assert_eq!(
            notification(&outcome, Lang::En).as_deref(),
            Some(expected.as_str())
        );
        assert!(notification(&outcome, Lang::Ar).is_some_and(|s| s.contains(tool)));
    }
}

// ------------------------------------------------------------ classification

#[test]
fn ac5_a_missing_poppler_is_a_missing_engine_not_an_unsupported_format() {
    let reg = registry(&[("t.pdf", "poppler", "pdf")]);
    let pdf = files(&["a.pdf"]);
    assert_eq!(
        classify(&pdf, &reg, &available(NONE)),
        Outcome::MissingEngine { tool: "Poppler" }
    );
    // The same selection with the tool present is runnable.
    assert_eq!(
        classify(&pdf, &reg, &available(&["poppler"])),
        Outcome::Supported
    );
}

#[test]
fn ac6_a_missing_qpdf_is_a_missing_engine_not_an_unsupported_format() {
    let reg = registry(&[("t.pdf", "qpdf", "pdf")]);
    let pdf = files(&["a.pdf"]);
    assert_eq!(
        classify(&pdf, &reg, &available(NONE)),
        Outcome::MissingEngine { tool: "qpdf" }
    );
    assert_eq!(
        classify(&pdf, &reg, &available(&["qpdf"])),
        Outcome::Supported
    );
}

#[test]
fn ac7_a_format_without_any_action_stays_unsupported() {
    let reg = registry(&[("t.pdf", "poppler", "pdf")]);
    assert_eq!(
        classify(&files(&["a.xyz"]), &reg, &available(NONE)),
        Outcome::Unsupported { ext: "xyz".into() }
    );
}

#[test]
fn ac8_a_mixed_selection_keeps_the_supported_and_the_unsupported_order() {
    let reg = registry(&[("t.pdf", "poppler", "pdf"), ("t.img", "image", "image")]);
    let selection = files(&["a.pdf", "b.png"]);
    assert_eq!(
        classify(&selection, &reg, &available(&["image"])),
        Outcome::Mixed {
            supported: files(&["b.png"]),
            unsupported: files(&["a.pdf"]),
        }
    );
}

#[test]
fn ac9_the_first_missing_tool_of_the_registry_wins() {
    let poppler_first = registry(&[("t.pdf", "poppler", "pdf"), ("t.pdf2", "qpdf", "pdf")]);
    assert_eq!(
        classify(&files(&["a.pdf"]), &poppler_first, &available(NONE)),
        Outcome::MissingEngine { tool: "Poppler" }
    );
    let qpdf_first = registry(&[("t.pdf", "qpdf", "pdf"), ("t.pdf2", "poppler", "pdf")]);
    assert_eq!(
        classify(&files(&["a.pdf"]), &qpdf_first, &available(NONE)),
        Outcome::MissingEngine { tool: "qpdf" }
    );
    // With one of the two present, the selection is runnable.
    assert_eq!(
        classify(&files(&["a.pdf"]), &poppler_first, &available(&["poppler"])),
        Outcome::Supported
    );
}

// ------------------------------------------------------------ S-145 capability reports

#[test]
fn s145_capability_reports_distinguish_every_unavailable_reason() {
    let reg = registry(&[
        ("t.qpdf", "qpdf", "pdf"),
        ("t.poppler", "poppler", "pdf"),
        ("t.ocr", "ocr", "image"),
        ("t.img", "image", "image"),
    ]);

    let reports = capability_reports(&reg, &[], &|action| match action.id.as_str() {
        "t.qpdf" => default_engine_reason(&action.engine, false),
        "t.poppler" => Some(UnavailableReason::DisabledByPolicy),
        "t.ocr" => Some(UnavailableReason::MissingLanguageData {
            language: "ara".to_string(),
        }),
        "t.img" => None,
        _ => None,
    });

    let find = |id: &str| -> &CapabilityReport {
        reports
            .iter()
            .find(|r| r.action_id == id)
            .unwrap_or_else(|| panic!("missing report for {id}"))
    };

    // Runnable action carries no reason and no help id.
    let img = find("t.img");
    assert!(img.runnable);
    assert_eq!(img.reason, None);
    assert_eq!(img.help_id, None);

    // Missing qpdf reports MissingExecutable with curated qpdf help.
    let qpdf = find("t.qpdf");
    assert!(!qpdf.runnable);
    assert_eq!(
        qpdf.reason,
        Some(UnavailableReason::MissingExecutable {
            program: "qpdf".to_string()
        })
    );
    assert_eq!(qpdf.help_id.as_deref(), Some("qpdf"));
    let qpdf_help = help_message(
        qpdf.help_id.as_deref().unwrap(),
        qpdf.reason.as_ref().unwrap(),
        Lang::En,
    )
    .expect("curated qpdf help");
    assert!(
        qpdf_help.contains("https://qpdf.readthedocs.io/"),
        "{qpdf_help}"
    );
    assert!(!qpdf_help.contains("sudo"), "{qpdf_help}");

    // Disabled policy is never reported as missing software and has no install help.
    let poppler = find("t.poppler");
    assert!(!poppler.runnable);
    assert_eq!(poppler.reason, Some(UnavailableReason::DisabledByPolicy));
    assert_eq!(poppler.help_id, None);

    // Installed Tesseract missing `ara` reports MissingLanguageData, not MissingExecutable.
    let ocr = find("t.ocr");
    assert!(!ocr.runnable);
    assert_eq!(
        ocr.reason,
        Some(UnavailableReason::MissingLanguageData {
            language: "ara".to_string()
        })
    );
    assert_eq!(ocr.help_id.as_deref(), Some("ocr-lang"));
    let ocr_help = help_message(
        ocr.help_id.as_deref().unwrap(),
        ocr.reason.as_ref().unwrap(),
        Lang::En,
    )
    .expect("curated language pack help");
    assert!(ocr_help.contains("ara"), "{ocr_help}");
    assert!(
        ocr_help.contains("https://tesseract-ocr.github.io/tessdoc/"),
        "{ocr_help}"
    );
    assert!(!ocr_help.contains("sudo"), "{ocr_help}");
}

#[test]
fn s145_incompatible_selection_is_unsupported_input_and_never_probes_engine() {
    let reg = registry(&[("t.qpdf", "qpdf", "pdf"), ("t.img", "image", "image")]);
    let probes = AtomicUsize::new(0);
    let png = Path::new("photo.png");
    let reports = capability_reports(&reg, &[png], &|action| {
        if action.engine == "qpdf" {
            probes.fetch_add(1, Ordering::SeqCst);
            return default_engine_reason(&action.engine, false);
        }
        None
    });

    assert_eq!(
        probes.load(Ordering::SeqCst),
        0,
        "incompatible PDF action must not probe qpdf for a PNG selection"
    );
    let qpdf = reports.iter().find(|r| r.action_id == "t.qpdf").unwrap();
    assert!(!qpdf.runnable);
    assert_eq!(qpdf.reason, Some(UnavailableReason::UnsupportedInput));
    assert_eq!(qpdf.help_id, None);
    let img = reports.iter().find(|r| r.action_id == "t.img").unwrap();
    assert!(img.runnable);
}

#[test]
fn s145_probe_cache_caches_by_tool_path_and_refreshes_explicitly() {
    let cache = ProbeCache::new();
    let path_a = Path::new("/usr/bin/tesseract");
    let path_b = Path::new("/usr/local/bin/tesseract");
    let calls = AtomicUsize::new(0);

    // First probe times out or fails -> ProbeUnknown, never ready.
    let first = cache.get_or_probe(path_a, |_| {
        calls.fetch_add(1, Ordering::SeqCst);
        ProbeOutcome::Unknown
    });
    assert_eq!(first, ProbeOutcome::Unknown);
    assert_eq!(
        first.check_language("tesseract", &["eng"]),
        Some(UnavailableReason::ProbeUnknown)
    );
    assert_eq!(
        first.into_reason("tesseract"),
        Some(UnavailableReason::ProbeUnknown)
    );

    // Second lookup for the same path hits the cache without re-probing.
    let second = cache.get_or_probe(path_a, |_| {
        calls.fetch_add(1, Ordering::SeqCst);
        ProbeOutcome::Ready {
            items: vec!["eng".to_string()],
        }
    });
    assert_eq!(second, ProbeOutcome::Unknown);
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    // A different tool path probes independently (PATH changed).
    let other = cache.get_or_probe(path_b, |_| {
        calls.fetch_add(1, Ordering::SeqCst);
        ProbeOutcome::Ready {
            items: vec!["eng".to_string()],
        }
    });
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(other.check_language("tesseract", &["eng"]), None);
    assert_eq!(
        other.check_language("tesseract", &["ara", "eng"]),
        Some(UnavailableReason::MissingLanguageData {
            language: "ara".to_string()
        })
    );

    // Explicit refresh invalidates path_a and observes the installed language.
    let refreshed = cache.refresh(path_a, |_| {
        calls.fetch_add(1, Ordering::SeqCst);
        ProbeOutcome::Ready {
            items: vec!["ara".to_string(), "eng".to_string()],
        }
    });
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert_eq!(refreshed.check_language("tesseract", &["ara", "eng"]), None);

    // Explicit invalidate_all clears every cached tool path.
    cache.invalidate_all();
    assert_eq!(cache.get(path_a), None);
    assert_eq!(cache.get(path_b), None);
}

#[test]
fn s145_reason_and_help_messages_respect_copy_rules() {
    let reasons = [
        UnavailableReason::MissingExecutable {
            program: "qpdf".to_string(),
        },
        UnavailableReason::MissingLanguageData {
            language: "ara".to_string(),
        },
        UnavailableReason::UnsupportedInput,
        UnavailableReason::InvalidOptions,
        UnavailableReason::DisabledByPolicy,
        UnavailableReason::ProbeUnknown,
    ];
    for reason in &reasons {
        let en = reason_message(reason, Lang::En);
        let ar = reason_message(reason, Lang::Ar);
        assert!(!en.is_empty());
        assert!(!ar.is_empty());
        assert_ne!(en, ar);
        for s in [&en, &ar] {
            assert!(!s.contains('\u{2013}') && !s.contains('\u{2014}'), "{s}");
            assert!(
                !s.chars().any(|c| ('\u{0660}'..='\u{0669}').contains(&c)),
                "{s}"
            );
            assert!(!s.contains("sudo"), "{s}");
        }
    }
    assert!(install_hint_for_help("ocr-lang").is_some());
    assert!(install_hint_for_help("qpdf").is_some());
    assert!(install_hint_for_help("poppler").is_some());
    for lang in [Lang::En, Lang::Ar] {
        let probe_help = help_message("ocr", &UnavailableReason::ProbeUnknown, lang)
            .expect("probe unknown help for known tool");
        assert!(!probe_help.contains('\u{2013}') && !probe_help.contains('\u{2014}'));
        assert!(!probe_help.contains("sudo"));
    }
}

#[test]
fn s145_unavailable_actions_hint_covers_english_and_arabic_agreement() {
    assert_eq!(unavailable_actions_hint(0, Lang::En), "");
    assert_eq!(unavailable_actions_hint(0, Lang::Ar), "");

    assert_eq!(
        unavailable_actions_hint(1, Lang::En),
        "1 more action needs setup, see kawaro doctor"
    );
    assert_eq!(
        unavailable_actions_hint(2, Lang::En),
        "2 more actions need setup, see kawaro doctor"
    );
    assert_eq!(
        unavailable_actions_hint(5, Lang::En),
        "5 more actions need setup, see kawaro doctor"
    );

    assert_eq!(
        unavailable_actions_hint(1, Lang::Ar),
        "إجراء آخر يحتاج إعداداً، راجع kawaro doctor"
    );
    assert_eq!(
        unavailable_actions_hint(2, Lang::Ar),
        "إجراءان آخران يحتاجان إعداداً، راجع kawaro doctor"
    );
    assert_eq!(
        unavailable_actions_hint(3, Lang::Ar),
        "3 إجراءات أخرى تحتاج إعداداً، راجع kawaro doctor"
    );
    assert_eq!(
        unavailable_actions_hint(10, Lang::Ar),
        "10 إجراءات أخرى تحتاج إعداداً، راجع kawaro doctor"
    );
    assert_eq!(
        unavailable_actions_hint(11, Lang::Ar),
        "11 إجراءً آخر يحتاج إعداداً، راجع kawaro doctor"
    );
    assert_eq!(
        unavailable_actions_hint(24, Lang::Ar),
        "24 إجراءً آخر يحتاج إعداداً، راجع kawaro doctor"
    );

    assert_eq!(arabic_plural(1, "ملف", "ملفان", "ملفات", "ملفاً"), "ملف");
    assert_eq!(arabic_plural(2, "ملف", "ملفان", "ملفات", "ملفاً"), "ملفان");
    assert_eq!(arabic_plural(7, "ملف", "ملفان", "ملفات", "ملفاً"), "7 ملفات");
    assert_eq!(
        arabic_plural(15, "ملف", "ملفان", "ملفات", "ملفاً"),
        "15 ملفاً"
    );

    for n in [1, 2, 3, 10, 11, 24] {
        for lang in [Lang::En, Lang::Ar] {
            let line = unavailable_actions_hint(n, lang);
            assert!(!line.contains('\n'), "{line}");
            assert!(
                !line.contains('\u{2013}') && !line.contains('\u{2014}'),
                "{line}"
            );
            assert!(
                !line.chars().any(|c| ('\u{0660}'..='\u{0669}').contains(&c)),
                "{line}"
            );
            assert!(!line.contains("sudo"), "{line}");
        }
    }
}
