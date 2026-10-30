//! S-096a (backlog/S-096-unsupported-file-feedback.md): the pure core of the
//! unsupported file feedback. AC1 classification, AC3 install hint table,
//! AC5 request URL, AC6 notification texts, AC7 suggestion line, AC9
//! classify budget, AC10 string tables.
//!
//! Hooks these tests pin (module `kawaro_core::unsupported`):
//! - `enum Outcome { Supported, Unsupported { ext: String },
//!   MissingEngine { tool: &'static str }, Mixed { supported: Vec<PathBuf>,
//!   unsupported: Vec<PathBuf> }, NoSelection }` (Debug, Clone, PartialEq, Eq)
//! - `classify(&[PathBuf], &Registry, &dyn Fn(&str) -> bool) -> Outcome`;
//!   the closure answers "is this engine id available". `Unsupported.ext`
//!   and `MissingEngine.tool` come from the first such file in selection
//!   order. A file whose extension is in an action's `skip_ext` is not
//!   accepted by that action. A file is runnable when some action accepts it
//!   and at least one such action has an available engine.
//! - `tool_name(engine_id) -> Option<&'static str>`
//! - `install_hints() -> &'static [InstallHint]` with `InstallHint { tool,
//!   url, package }` (all `&'static str`), and
//!   `missing_engine_message(tool, Lang) -> String`
//! - `request_url(ext: &str) -> String`
//! - `notification(&Outcome, Lang) -> Option<String>` (None for `Supported`
//!   and `Mixed`, which show a hive)
//! - `centre_text(&[PathBuf], Lang) -> String`, `unsupported_message(Lang)`,
//!   `request_label(Lang)`, `request_hover(Lang)`
//! - `suggestion(&[PathBuf], &Registry, &dyn Fn(&str) -> bool, Lang)
//!   -> Option<String>`

use kawaro_core::Registry;
use kawaro_core::i18n::Lang;
use kawaro_core::unsupported::{
    Outcome, centre_text, classify, install_hints, missing_engine_message, notification,
    request_hover, request_label, request_url, suggestion, tool_name, unsupported_message,
};
use std::path::PathBuf;

fn act(id: &str, engine: &str, accepts: &str, skip: &str, label: &str) -> String {
    format!(
        r#"
[[action]]
id = "{id}"
engine = "{engine}"
op = "convert"
ring = 1
accepts = ["{accepts}"]
skip_ext = [{skip}]
output = {{ mode = "per-file", ext = "out" }}
code = {{ ar = "ن", en = "N" }}
label = {{ ar = "{label}", en = "{label}" }}
"#
    )
}

fn registry() -> Registry {
    let mut reg = Registry::default();
    let text = [
        act("t.img", "image", "image", "\"heic\"", "Convert image"),
        act("t.vid", "ffmpeg", "video", "", "Convert video"),
        act("t.doc", "office", "document", "", "Convert document"),
        act("t.pdf", "ghostscript", "pdf", "", "Convert PDF"),
    ]
    .join("\n");
    reg.add_toml("s096.toml", &text).unwrap();
    reg
}

fn paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(PathBuf::from).collect()
}

fn all(_: &str) -> bool {
    true
}

fn without(off: &'static str) -> impl Fn(&str) -> bool {
    move |id| id != off
}

fn none(_: &str) -> bool {
    false
}

// ------------------------------------------------------------ AC1

#[test]
fn ac1_classify_table_of_twelve_rows() {
    let reg = registry();
    let mixed = Outcome::Mixed {
        supported: paths(&["a.png"]),
        unsupported: paths(&["b.xyz"]),
    };
    type Row = (
        &'static str,
        Vec<PathBuf>,
        Box<dyn Fn(&str) -> bool>,
        Outcome,
    );
    let rows: Vec<Row> = vec![
        ("empty", vec![], Box::new(all), Outcome::NoSelection),
        ("png", paths(&["a.png"]), Box::new(all), Outcome::Supported),
        (
            "xyz",
            paths(&["a.xyz"]),
            Box::new(all),
            Outcome::Unsupported { ext: "xyz".into() },
        ),
        (
            "no extension",
            paths(&["README"]),
            Box::new(all),
            Outcome::Unsupported { ext: "".into() },
        ),
        (
            "upper case",
            paths(&["A.XYZ"]),
            Box::new(all),
            Outcome::Unsupported { ext: "xyz".into() },
        ),
        (
            "mp4 ffmpeg off",
            paths(&["a.mp4"]),
            Box::new(without("ffmpeg")),
            Outcome::MissingEngine { tool: "FFmpeg" },
        ),
        (
            "docx office off",
            paths(&["a.docx"]),
            Box::new(without("office")),
            Outcome::MissingEngine {
                tool: "LibreOffice",
            },
        ),
        (
            "pdf ghostscript off",
            paths(&["a.pdf"]),
            Box::new(without("ghostscript")),
            Outcome::MissingEngine {
                tool: "Ghostscript",
            },
        ),
        (
            "two missing tools, first wins",
            paths(&["a.mp4", "a.docx"]),
            Box::new(|id| id == "image" || id == "ghostscript"),
            Outcome::MissingEngine { tool: "FFmpeg" },
        ),
        ("mixed", paths(&["a.png", "b.xyz"]), Box::new(all), mixed),
        (
            "two unsupported, first ext",
            paths(&["a.xyz", "b.abc"]),
            Box::new(all),
            Outcome::Unsupported { ext: "xyz".into() },
        ),
        (
            "mp4 ffmpeg on",
            paths(&["a.mp4"]),
            Box::new(all),
            Outcome::Supported,
        ),
    ];
    assert_eq!(rows.len(), 12);
    for (name, files, ok, want) in rows {
        assert_eq!(classify(&files, &reg, &*ok), want, "row {name}");
    }
}

#[test]
fn ac1_skip_ext_makes_a_file_unsupported_not_missing() {
    let reg = registry();
    assert_eq!(
        classify(&paths(&["a.heic"]), &reg, &none),
        Outcome::Unsupported { ext: "heic".into() }
    );
}

#[test]
fn ac1_tool_names_map_from_engine_ids() {
    assert_eq!(tool_name("ffmpeg"), Some("FFmpeg"));
    assert_eq!(tool_name("office"), Some("LibreOffice"));
    assert_eq!(tool_name("ghostscript"), Some("Ghostscript"));
    assert_eq!(tool_name("image"), None);
}

// ------------------------------------------------------------ AC3

#[test]
fn ac3_install_hint_table_names_tool_once_with_link_and_package() {
    let hints = install_hints();
    let tools: Vec<&str> = hints.iter().map(|h| h.tool).collect();
    assert_eq!(tools, ["FFmpeg", "LibreOffice", "Ghostscript"]);
    for h in hints {
        assert!(h.url.starts_with("https://"), "{}", h.url);
        assert!(!h.package.is_empty());
        for lang in [Lang::En, Lang::Ar] {
            let msg = missing_engine_message(h.tool, lang);
            assert_eq!(msg.matches(h.tool).count(), 1, "{lang:?}: {msg}");
            assert!(msg.contains(h.url), "{lang:?}: {msg}");
            assert!(msg.contains(h.package), "{lang:?}: {msg}");
        }
    }
}

// ------------------------------------------------------------ AC5

#[test]
fn ac5_request_url_table() {
    let prefix = "https://github.com/M7MMAD-OMAR/kawaro/issues/new?title=Format+request%3A+.";
    let rows = [
        ("xyz", "xyz"),
        ("ABC", "abc"),
        ("a b", "unknown"),
        ("abcdefghijklmnopq", "unknown"),
        ("", "unknown"),
        ("abcdefghijklmnop", "abcdefghijklmnop"),
    ];
    for (input, kept) in rows {
        let url = request_url(input);
        assert!(
            url.starts_with(&format!("{prefix}{kept}&body=")),
            "{input}: {url}"
        );
    }
}

#[test]
fn ac5_body_is_a_fixed_template_plus_the_extension_only() {
    let a = request_url("xyz");
    let b = request_url("abc");
    assert_eq!(a.replace("xyz", "abc"), b);
    let unknown = request_url("a b");
    assert_eq!(unknown, request_url("zzzzzzzzzzzzzzzzzzzz"));
    for url in [&a, &unknown] {
        for leak in ["/home", "$HOME", "a%20b", env!("CARGO_PKG_VERSION"), "0.4"] {
            assert!(!url.contains(leak), "{leak} in {url}");
        }
        for var in ["USER", "HOSTNAME", "LOGNAME"] {
            if let Ok(v) = std::env::var(var)
                && v.len() > 2
            {
                assert!(!url.contains(&v), "{var} value leaked");
            }
        }
    }
}

// ------------------------------------------------------------ AC6

#[test]
fn ac6_notification_texts_exact() {
    let rows = [
        (
            Outcome::NoSelection,
            "No file selected. Select a file, then press the shortcut again.",
        ),
        (
            Outcome::Unsupported { ext: "xyz".into() },
            ".xyz files are not supported yet",
        ),
        (
            Outcome::MissingEngine { tool: "FFmpeg" },
            "FFmpeg is not installed",
        ),
    ];
    for (outcome, en) in rows {
        assert_eq!(notification(&outcome, Lang::En).as_deref(), Some(en));
        let ar = notification(&outcome, Lang::Ar).expect("arabic text");
        assert!(!ar.is_empty() && ar != en);
    }
    let ar_unsupported = notification(&Outcome::Unsupported { ext: "xyz".into() }, Lang::Ar);
    assert!(ar_unsupported.unwrap().contains("xyz"));
    let ar_missing = notification(&Outcome::MissingEngine { tool: "FFmpeg" }, Lang::Ar);
    assert!(ar_missing.unwrap().contains("FFmpeg"));
    assert_eq!(notification(&Outcome::Supported, Lang::En), None);
}

// ------------------------------------------------------------ AC2 strings

#[test]
fn ac2_centre_text_and_message_exact() {
    assert_eq!(centre_text(&paths(&["a.xyz"]), Lang::En), "File .xyz");
    assert_eq!(
        centre_text(&paths(&["a.xyz"]), Lang::Ar),
        "ملف \u{200e}.xyz"
    );
    assert_eq!(
        centre_text(&paths(&["a.xyz", "b.xyz", "c.XYZ"]), Lang::En),
        "3 files .xyz"
    );
    assert_eq!(
        centre_text(&paths(&["a.a", "b.b", "c.c", "d.d"]), Lang::En),
        ".a .b .c +1"
    );
    assert_eq!(centre_text(&paths(&["a.a", "b.b"]), Lang::En), ".a .b");
    assert_eq!(
        unsupported_message(Lang::En),
        "This format is not supported yet"
    );
    assert_eq!(unsupported_message(Lang::Ar), "هذه الصيغة غير مدعومة بعد");
}

// ------------------------------------------------------------ AC5 strings

#[test]
fn ac5_request_cell_strings() {
    assert_eq!(request_label(Lang::En), "Request this format");
    assert_eq!(request_label(Lang::Ar), "اطلب هذه الصيغة");
    let en = request_hover(Lang::En).to_lowercase();
    assert!(en.contains("nothing is sent automatically"), "{en}");
    assert!(en.contains("no telemetry"), "{en}");
    let ar = request_hover(Lang::Ar);
    assert!(!ar.is_empty() && ar != request_hover(Lang::En));
}

// ------------------------------------------------------------ AC7

#[test]
fn ac7_suggestion_only_for_same_family_with_available_engine() {
    let reg = registry();
    // heic is in the image family, the image action skips it: suggest it.
    assert_eq!(
        suggestion(&paths(&["a.heic"]), &reg, &all, Lang::En).as_deref(),
        Some("Convert image")
    );
    // Other family: no line.
    assert_eq!(suggestion(&paths(&["a.xyz"]), &reg, &all, Lang::En), None);
    // Family present, engine off: no line.
    assert_eq!(suggestion(&paths(&["a.heic"]), &reg, &none, Lang::En), None);
    // Family with no action at all: no line.
    let mut audio_only = Registry::default();
    audio_only
        .add_toml(
            "a.toml",
            &act("t.vid", "ffmpeg", "video", "", "Convert video"),
        )
        .unwrap();
    assert_eq!(
        suggestion(&paths(&["a.heic"]), &audio_only, &all, Lang::En),
        None
    );
}

// ------------------------------------------------------------ AC9

#[test]
fn ac9_classify_one_thousand_paths_within_twenty_ms() {
    let reg = registry();
    let exts = ["png", "xyz", "mp4", "docx", "pdf", "", "HEIC", "abc"];
    let files: Vec<PathBuf> = (0..1000)
        .map(|i| {
            let e = exts[i % exts.len()];
            PathBuf::from(format!("/some/dir/file{i}.{e}"))
        })
        .collect();
    let ok = without("office");
    let t = std::time::Instant::now();
    let _ = classify(&files, &reg, &ok);
    let took = t.elapsed();
    assert!(took.as_millis() <= 20, "{took:?}");
}

// ------------------------------------------------------------ AC10

#[test]
fn ac10_strings_have_no_long_dashes_and_use_western_digits() {
    let mut all_strings = vec![
        unsupported_message(Lang::En).to_string(),
        unsupported_message(Lang::Ar).to_string(),
        request_label(Lang::En).to_string(),
        request_label(Lang::Ar).to_string(),
        request_hover(Lang::En).to_string(),
        request_hover(Lang::Ar).to_string(),
        request_url("xyz"),
        centre_text(&paths(&["a.xyz", "b.xyz"]), Lang::Ar),
    ];
    for h in install_hints() {
        for lang in [Lang::En, Lang::Ar] {
            all_strings.push(missing_engine_message(h.tool, lang));
        }
    }
    for lang in [Lang::En, Lang::Ar] {
        for o in [
            Outcome::NoSelection,
            Outcome::Unsupported { ext: "xyz".into() },
            Outcome::MissingEngine { tool: "FFmpeg" },
        ] {
            all_strings.push(notification(&o, lang).unwrap());
        }
    }
    for s in all_strings {
        assert!(!s.contains('\u{2013}') && !s.contains('\u{2014}'), "{s}");
        assert!(
            !s.chars().any(|c| ('\u{0660}'..='\u{0669}').contains(&c)),
            "{s}"
        );
    }
}
