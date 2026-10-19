//! S-002 AC1 to AC6: action registry and built-in image actions.

use sbarhive_core::action::{OutputMode, RegistryError};
use sbarhive_core::{MediaKind, Registry};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/core")
}

fn action(id: &str, extra: &str) -> String {
    format!(
        r#"
[[action]]
id = "{id}"
engine = "fake"
op = "copy"
ring = 1
accepts = ["image"]
output = {{ mode = "per-file", ext = "out" }}
code = {{ ar = "ن", en = "N" }}
label = {{ ar = "نسخ", en = "Copy" }}
{extra}
"#
    )
}

fn ids(reg: &Registry) -> Vec<&str> {
    reg.all().iter().map(|a| a.id.as_str()).collect()
}

fn parse_err_file(e: RegistryError) -> String {
    match e {
        RegistryError::Parse { file, .. } => file,
        other => panic!("expected Parse, got {other:?}"),
    }
}

#[test]
fn ac1_parses_every_field_and_defaults() {
    let text = r#"
[[action]]
id = "x.full"
engine = "image"
op = "convert"
ring = 2
order = 7
accepts = ["image", "pdf"]
skip_ext = ["webp"]
min_files = 2
max_files = 5
output = { mode = "combined", ext = "pdf", suffix = "-all" }
code = { ar = "ك", en = "K" }
label = { ar = "كل", en = "All" }
[action.params]
quality = 85
page = "fit"

[[action]]
id = "x.min"
engine = "image"
op = "grayscale"
ring = 1
accepts = ["image"]
output = { mode = "per-file" }
code = { ar = "ر", en = "G" }
label = { ar = "رمادي", en = "Gray" }
"#;
    let mut reg = Registry::default();
    reg.add_toml("full.toml", text).unwrap();
    assert_eq!(ids(&reg), ["x.full", "x.min"]);
    let a = reg.get("x.full").unwrap();
    assert_eq!(a.engine, "image");
    assert_eq!(a.op, "convert");
    assert_eq!(a.ring, 2);
    assert_eq!(a.order, 7);
    assert_eq!(a.accepts, [MediaKind::Image, MediaKind::Pdf]);
    assert_eq!(a.skip_ext, ["webp"]);
    assert_eq!((a.min_files, a.max_files), (2, 5));
    assert_eq!(a.output.mode, OutputMode::Combined);
    assert_eq!(a.output.ext, "pdf");
    assert_eq!(a.output.suffix, "-all");
    assert_eq!(a.code.en, "K");
    assert_eq!(a.label.ar, "كل");
    assert_eq!(a.params.get("quality").unwrap().as_integer(), Some(85));
    assert_eq!(a.params.get("page").unwrap().as_str(), Some("fit"));
    let m = reg.get("x.min").unwrap();
    assert_eq!((m.order, m.min_files, m.max_files), (0, 1, 0));
    assert!(m.skip_ext.is_empty() && m.params.is_empty());
    assert_eq!(m.output.ext, "");
    assert_eq!(m.output.suffix, "");
}

#[test]
fn ac1_empty_document_adds_nothing() {
    let mut reg = Registry::default();
    reg.add_toml("empty.toml", "").unwrap();
    assert!(reg.all().is_empty());
}

#[test]
fn ac1_broken_toml_is_parse_error_with_file_name() {
    let mut reg = Registry::default();
    let e = reg.add_toml("broken.toml", "[[action]\nid = ").unwrap_err();
    assert_eq!(parse_err_file(e), "broken.toml");
    let e = reg
        .add_toml("missing.toml", "[[action]]\nid = \"a\"\n")
        .unwrap_err();
    assert_eq!(parse_err_file(e), "missing.toml");
    assert!(reg.all().is_empty());
}

#[test]
fn ac1_rejects_invalid_ring_accepts_and_counts() {
    let bad = [
        action("r0", "").replace("ring = 1", "ring = 0"),
        action("r3", "").replace("ring = 1", "ring = 3"),
        action("acc", "").replace(r#"accepts = ["image"]"#, "accepts = []"),
        action("cnt", "min_files = 3\nmax_files = 2"),
    ];
    for (i, text) in bad.iter().enumerate() {
        let mut reg = Registry::default();
        let name = format!("bad{i}.toml");
        let e = reg.add_toml(&name, text).unwrap_err();
        assert_eq!(parse_err_file(e), name);
        assert!(reg.all().is_empty());
    }
    let ok = [
        action("r2", "").replace("ring = 1", "ring = 2"),
        action("eq", "min_files = 2\nmax_files = 2"),
        action("nolimit", "min_files = 3\nmax_files = 0"),
    ];
    for text in ok {
        let mut reg = Registry::default();
        reg.add_toml("ok.toml", &text).unwrap();
        assert_eq!(reg.all().len(), 1);
    }
}

#[test]
fn ac2_duplicate_in_one_document_is_rejected_and_registry_unchanged() {
    let mut reg = Registry::default();
    reg.add_toml("base.toml", &action("keep", "")).unwrap();
    let text = format!("{}{}{}", action("a", ""), action("b", ""), action("a", ""));
    match reg.add_toml("dup.toml", &text).unwrap_err() {
        RegistryError::Duplicate(id) => assert_eq!(id, "a"),
        other => panic!("expected Duplicate, got {other:?}"),
    }
    assert_eq!(ids(&reg), ["keep"]);
}

#[test]
fn ac2_later_load_overrides_in_place() {
    let mut reg = Registry::default();
    let first = format!("{}{}{}", action("a", ""), action("b", ""), action("c", ""));
    reg.add_toml("builtin.toml", &first).unwrap();
    let over = format!(
        "{}{}",
        action("b", "order = 99").replace("op = \"copy\"", "op = \"user\""),
        action("d", "")
    );
    reg.add_toml("user.toml", &over).unwrap();
    assert_eq!(ids(&reg), ["a", "b", "c", "d"]);
    assert_eq!(reg.get("b").unwrap().op, "user");
    assert_eq!(reg.get("b").unwrap().order, 99);
    assert_eq!(reg.get("a").unwrap().op, "copy");
}

#[test]
fn ac2_duplicate_across_files_of_one_dir_is_rejected() {
    let mut reg = Registry::default();
    reg.add_toml("base.toml", &action("keep", "")).unwrap();
    match reg.add_dir(&fixtures().join("dup-dir")).unwrap_err() {
        RegistryError::Duplicate(id) => assert_eq!(id, "fx.same"),
        other => panic!("expected Duplicate, got {other:?}"),
    }
    assert_eq!(ids(&reg), ["keep"]);
}

#[test]
fn ac2_parse_error_in_dir_leaves_registry_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.toml"), action("good", "")).unwrap();
    std::fs::write(dir.path().join("b.toml"), "not toml [[").unwrap();
    let mut reg = Registry::default();
    let e = reg.add_dir(dir.path()).unwrap_err();
    assert!(parse_err_file(e).ends_with("b.toml"));
    assert!(reg.all().is_empty());
}

#[test]
fn ac2_user_dir_overrides_builtin() {
    let mut reg = Registry::builtin();
    let before = reg.all().len();
    let pos = |r: &Registry| r.all().iter().position(|a| a.id == "image.to-webp");
    let webp_pos = pos(&reg);
    reg.add_dir(&fixtures().join("user-dir")).unwrap();
    assert_eq!(reg.all().len(), before + 1);
    let webp = reg.get("image.to-webp").unwrap();
    assert_eq!(webp.params.get("quality").unwrap().as_integer(), Some(50));
    // The override replaces the built-in in place.
    assert!(webp_pos.is_some());
    assert_eq!(pos(&reg), webp_pos);
    assert!(reg.get("user.extra").is_some());
}

#[test]
fn ac3_dir_loads_sorted_toml_files_only() {
    let mut reg = Registry::default();
    reg.add_dir(&fixtures().join("sorted-dir")).unwrap();
    assert_eq!(ids(&reg), ["fx.from-a", "fx.from-b"]);
}

#[test]
fn ac3_missing_dir_is_ok() {
    let mut reg = Registry::default();
    reg.add_dir(&fixtures().join("no-such-dir")).unwrap();
    assert!(reg.all().is_empty());
}

#[test]
fn ac3_unreadable_entry_is_io_error() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("trap.toml")).unwrap();
    let mut reg = Registry::default();
    assert!(matches!(
        reg.add_dir(dir.path()).unwrap_err(),
        RegistryError::Io(_)
    ));
}

#[test]
fn ac3_dir_path_that_is_a_file_is_io_error() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("plain");
    std::fs::write(&f, "x").unwrap();
    let mut reg = Registry::default();
    assert!(matches!(reg.add_dir(&f).unwrap_err(), RegistryError::Io(_)));
}

const IMAGES: &str = include_str!("../../../actions/images.toml");

#[test]
fn ac4_builtin_contains_every_repository_action_once() {
    let reg = Registry::builtin();
    let mut images = Registry::default();
    images.add_toml("images.toml", IMAGES).unwrap();
    for a in images.all() {
        assert_eq!(reg.get(&a.id), Some(a), "{} missing from builtin", a.id);
    }
    let unique: HashSet<_> = reg.all().iter().map(|a| &a.id).collect();
    assert_eq!(unique.len(), reg.all().len());
    // Every file in actions/ must be embedded: compare ids with a fresh parse of the folder.
    let mut from_dir = Registry::default();
    from_dir
        .add_dir(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../actions"))
        .unwrap();
    assert_eq!(from_dir.all(), reg.all());
}

fn matched(reg: &Registry, files: &[&str], ok: &dyn Fn(&str) -> bool) -> Vec<String> {
    let paths: Vec<PathBuf> = files.iter().map(PathBuf::from).collect();
    let refs: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
    reg.matching(&refs, ok)
        .iter()
        .map(|a| a.id.clone())
        .collect()
}

fn any(_: &str) -> bool {
    true
}

fn match_reg() -> Registry {
    let mut reg = Registry::default();
    let text = [
        action("img", "skip_ext = [\"webp\", \"Png\"]"),
        action("imgpdf", "").replace(r#"accepts = ["image"]"#, r#"accepts = ["image", "pdf"]"#),
        action("two-three", "min_files = 2\nmax_files = 3"),
        action("one-only", "max_files = 1"),
        action("other-engine", "").replace("engine = \"fake\"", "engine = \"gone\""),
        action("audio", "").replace(r#"accepts = ["image"]"#, r#"accepts = ["audio"]"#),
        action("zero-min", "min_files = 0"),
    ]
    .concat();
    reg.add_toml("m.toml", &text).unwrap();
    reg
}

#[test]
fn ac5_every_file_must_be_accepted() {
    let reg = match_reg();
    let one_img = matched(&reg, &["/a/x.jpg"], &any);
    assert!(one_img.contains(&"img".to_string()));
    assert!(!one_img.contains(&"audio".to_string()));
    let mixed = matched(&reg, &["/a/x.jpg", "/a/y.pdf"], &any);
    assert_eq!(mixed, ["imgpdf"]);
    assert_eq!(matched(&reg, &["/a/s.mp3"], &any), ["audio"]);
    assert!(matched(&reg, &["/a/x.jpg", "/a/s.mp3"], &any).is_empty());
}

#[test]
fn ac5_file_count_limits() {
    let reg = match_reg();
    let one = matched(&reg, &["a.jpg"], &any);
    assert!(one.contains(&"one-only".to_string()));
    assert!(!one.contains(&"two-three".to_string()));
    let two = matched(&reg, &["a.jpg", "b.jpg"], &any);
    assert!(two.contains(&"two-three".to_string()));
    assert!(!two.contains(&"one-only".to_string()));
    let three = matched(&reg, &["a.jpg", "b.jpg", "c.jpg"], &any);
    assert!(three.contains(&"two-three".to_string()));
    let four = matched(&reg, &["a.jpg", "b.jpg", "c.jpg", "d.jpg"], &any);
    assert!(!four.contains(&"two-three".to_string()));
    assert!(four.contains(&"img".to_string()));
}

#[test]
fn ac5_empty_selection_matches_nothing() {
    let reg = match_reg();
    assert!(matched(&reg, &[], &any).is_empty());
}

#[test]
fn ac5_skip_ext_only_when_every_file_has_it_case_insensitive() {
    let reg = match_reg();
    assert!(!matched(&reg, &["a.WEBP"], &any).contains(&"img".to_string()));
    assert!(!matched(&reg, &["a.webp", "b.png"], &any).contains(&"img".to_string()));
    assert!(matched(&reg, &["a.webp", "b.jpg"], &any).contains(&"img".to_string()));
    assert!(matched(&reg, &["a.jpg"], &any).contains(&"img".to_string()));
}

#[test]
fn ac5_engine_filter_receives_the_engine_id() {
    let reg = match_reg();
    let all = matched(&reg, &["a.jpg"], &any);
    assert!(all.contains(&"other-engine".to_string()));
    let no_gone = matched(&reg, &["a.jpg"], &|e| e != "gone");
    assert!(!no_gone.contains(&"other-engine".to_string()));
    assert!(no_gone.contains(&"img".to_string()));
    assert!(matched(&reg, &["a.jpg"], &|_| false).is_empty());
}

#[test]
fn ac5_sorted_by_ring_order_id() {
    let mut reg = Registry::default();
    let text = [
        action("c", "order = 1").replace("ring = 1", "ring = 2"),
        action("b", "order = 5"),
        action("a", "order = 5"),
        action("z", "order = -3"),
        action("y", "order = 0").replace("ring = 1", "ring = 2"),
    ]
    .concat();
    reg.add_toml("s.toml", &text).unwrap();
    assert_eq!(matched(&reg, &["p.png"], &any), ["z", "a", "b", "y", "c"]);
}

type Row = (
    &'static str,
    u8,
    &'static str,
    OutputMode,
    &'static str,
    &'static str,
    &'static [&'static str],
    &'static [(&'static str, &'static str)],
);

#[test]
fn ac6_builtin_image_actions_follow_the_engine_table() {
    use OutputMode::{Combined as C, PerFile as P};
    let reg = Registry::builtin();
    // (id, ring, op, mode, ext, suffix, skip_ext, params as TOML text)
    let rows: &[Row] = &[
        (
            "image.to-webp",
            1,
            "convert",
            P,
            "webp",
            "",
            &["webp"],
            &[("quality", "85")],
        ),
        ("image.to-png", 1, "convert", P, "png", "", &["png"], &[]),
        (
            "image.to-jpg",
            1,
            "convert",
            P,
            "jpg",
            "",
            &["jpg", "jpeg", "jpe"],
            &[("quality", "90")],
        ),
        (
            "image.to-avif",
            1,
            "convert",
            P,
            "avif",
            "",
            &["avif"],
            &[("quality", "70")],
        ),
        (
            "image.to-pdf",
            1,
            "to-pdf",
            C,
            "pdf",
            "",
            &[],
            &[("page", "\"fit\"")],
        ),
        ("image.to-ico", 1, "convert", P, "ico", "", &["ico"], &[]),
        (
            "image.resize-50",
            2,
            "resize",
            P,
            "",
            "-50",
            &[],
            &[("scale", "50")],
        ),
        (
            "image.fit-1920",
            2,
            "resize",
            P,
            "",
            "-1920",
            &[],
            &[("max_side", "1920")],
        ),
        (
            "image.rotate-90",
            2,
            "rotate",
            P,
            "",
            "-rotated",
            &[],
            &[("degrees", "90")],
        ),
        (
            "image.flip-h",
            2,
            "flip",
            P,
            "",
            "-flipped",
            &[],
            &[("axis", "\"horizontal\"")],
        ),
        ("image.grayscale", 2, "grayscale", P, "", "-gray", &[], &[]),
        (
            "image.compress",
            2,
            "compress",
            P,
            "",
            "-compressed",
            &[],
            &[("quality", "75")],
        ),
        ("image.strip", 2, "strip", P, "", "-clean", &[], &[]),
        ("image.to-gif", 2, "convert", P, "gif", "", &["gif"], &[]),
        (
            "image.to-tiff",
            2,
            "convert",
            P,
            "tiff",
            "",
            &["tif", "tiff"],
            &[],
        ),
        ("image.to-bmp", 2, "convert", P, "bmp", "", &["bmp"], &[]),
    ];
    let image_actions = reg.all().iter().filter(|a| a.engine == "image").count();
    assert_eq!(image_actions, rows.len());
    for (id, ring, op, mode, ext, suffix, skip, params) in rows {
        let a = reg.get(id).unwrap_or_else(|| panic!("missing {id}"));
        assert_eq!(a.engine, "image", "{id}");
        assert_eq!(a.ring, *ring, "{id}");
        assert_eq!(a.op, *op, "{id}");
        assert_eq!(a.output.mode, *mode, "{id}");
        assert_eq!(a.output.ext, *ext, "{id}");
        assert_eq!(a.output.suffix, *suffix, "{id}");
        assert_eq!(a.skip_ext, *skip, "{id}");
        assert_eq!(a.accepts, [MediaKind::Image], "{id}");
        assert_eq!((a.min_files, a.max_files), (1, 0), "{id}");
        let got: Vec<(String, String)> = a
            .params
            .iter()
            .map(|(k, v)| (k.clone(), v.to_string()))
            .collect();
        let want: Vec<(String, String)> = params
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        assert_eq!(got, want, "{id}");
        for code in [&a.code.ar, &a.code.en] {
            assert!(
                !code.is_empty() && code.chars().count() <= 5,
                "{id} code {code}"
            );
        }
        assert!(!a.label.ar.is_empty() && !a.label.en.is_empty(), "{id}");
    }
    let image = |ring| {
        reg.all()
            .iter()
            .filter(|a| a.engine == "image" && a.ring == ring)
            .count()
    };
    assert_eq!((image(1), image(2)), (6, 10));
}

#[test]
fn ac6_builtin_hive_for_one_photo() {
    let reg = Registry::builtin();
    let ids = matched(&reg, &["/home/u/photo.JPG"], &any);
    assert_eq!(ids.len(), 15);
    assert_eq!(ids[0], "image.to-webp");
    assert!(!ids.contains(&"image.to-jpg".to_string()));
    assert!(matched(&reg, &["/home/u/photo.jpg"], &|e| e != "image").is_empty());
}
