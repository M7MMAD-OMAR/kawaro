//! S-117: action options (`crates/core/src/options.rs`).
//!
//! The options layer sits between the action TOML and the UI and MCP
//! surfaces: it parses `[action.options]`, validates values, resolves the
//! explicit ones onto the base `[action.params]` table and renders the JSON
//! Schema that clients read. The built-in actions carry the first real specs.

use kawaro_core::action::{Registry, RegistryError};
use kawaro_core::options::{Field, Kind, OptionsError, OptionsKind, OptionsSpec};

const LABEL: &str = r#"{ ar = "وسم", en = "Label" }"#;

/// One `[[field]]` table at the document root, for the validate tests.
fn field(id: &str, kind: &str, extra: &str) -> String {
    format!("\n[[field]]\nid = \"{id}\"\ntype = \"{kind}\"\nlabel = {LABEL}\n{extra}\n")
}

fn spec(text: &str) -> OptionsSpec {
    toml::from_str(text).unwrap_or_else(|e| panic!("options did not parse: {e}\n{text}"))
}

fn field_of<'a>(o: &'a OptionsSpec, id: &str) -> &'a Field {
    o.fields
        .iter()
        .find(|f| f.id == id)
        .unwrap_or_else(|| panic!("no field {id}"))
}

fn resolve_one(
    o: &OptionsSpec,
    key: &str,
    value: toml::Value,
) -> Result<toml::Table, OptionsError> {
    let mut explicit = toml::Table::new();
    explicit.insert(key.into(), value);
    o.resolve(&toml::Table::new(), &explicit)
}

fn invalid(text: &str) {
    let o = spec(text);
    assert!(o.validate().is_err(), "accepted invalid options:\n{text}");
}

/// A full action with every options table: fields and presets.
const CUSTOM_ACTION: &str = r#"
[[action]]
id = "test.options"
engine = "image"
op = "resize"
ring = 2
accepts = ["image"]
output = { mode = "per-file", suffix = "-opt" }
code = { ar = "خ", en = "OPT" }
label = { ar = "خيارات", en = "Options" }

[action.params]
scale = 50
max_side = 1920

[action.options]
kind = "form"
readout = ["dimensions", "percent"]
remember = true
suffix_template = "-{scale}"

[[action.options.field]]
id = "scale"
type = "percent"
label = { ar = "المقياس", en = "Scale" }
default = 50
min = 1
max = 1000
step = 5
unit = "%"
maps_to = "scale"
clears = ["max_side"]

[[action.options.field]]
id = "max_side"
type = "px"
label = { ar = "الجانب الأقصى", en = "Max side" }
default = 1920
min = 16
max = 20000
maps_to = "max_side"
clears = ["scale"]

[[action.options.field]]
id = "fit"
type = "enum"
label = { ar = "الملاءمة", en = "Fit" }
default = "cover"
values = [
  { id = "cover", label = { ar = "تغطية", en = "Cover" } },
  { id = "contain", label = { ar = "احتواء", en = "Contain" } },
]
maps_to = "fit"
no_enlarge = true

[[action.options.preset]]
id = "small"
values = { scale = 50, fit = "contain" }

[[action.options.preset]]
id = "large"
values = { max_side = 4000 }
"#;

/// A field id that differs from the param key it writes, plus a px pair.
const MAPPED: &str = r#"
kind = "form"

[[field]]
id = "level"
type = "int"
label = { ar = "المستوى", en = "Level" }
default = 50
min = 1.0
max = 100.0
maps_to = "quality"
clears = ["strip"]

[[field]]
id = "size"
type = "px-pair"
label = { ar = "الحجم", en = "Size" }
default = "1920x1080"
maps_to = "size"
"#;

/// One field of every kind, for the value and schema tests.
const KINDS: &str = r#"
kind = "form"

[[field]]
id = "quality"
type = "int"
label = { ar = "الجودة", en = "Quality" }
default = 70
min = 1.0
max = 100.0
unit = "%"
maps_to = "quality"

[[field]]
id = "ratio"
type = "float"
label = { ar = "النسبة", en = "Ratio" }
default = 1.0
min = 0.0
max = 2.0
maps_to = "ratio"

[[field]]
id = "text"
type = "string"
label = { ar = "النص", en = "Text" }
maps_to = "text"

[[field]]
id = "flag"
type = "bool"
label = { ar = "العلم", en = "Flag" }
default = false
maps_to = "flag"

[[field]]
id = "size"
type = "px-pair"
label = { ar = "الحجم", en = "Size" }
default = "1920x1080"
maps_to = "size"

[[field]]
id = "preset"
type = "enum"
label = { ar = "القالب", en = "Preset" }
default = "a"
values = [
  { id = "a", label = { ar = "أ", en = "A" } },
  { id = "b", label = { ar = "ب", en = "B" } },
]
maps_to = "preset"
"#;

#[test]
fn parses_action_options_tables_and_presets() {
    let mut reg = Registry::default();
    reg.add_toml("custom.toml", CUSTOM_ACTION).unwrap();
    let o = reg
        .get("test.options")
        .unwrap()
        .options
        .as_ref()
        .expect("the action carries options");
    assert_eq!(o.kind, OptionsKind::Form);
    assert_eq!(o.readout, ["dimensions", "percent"]);
    assert!(o.remember);
    assert_eq!(o.suffix_template, "-{scale}");
    assert_eq!(o.fields.len(), 3);

    let scale = field_of(o, "scale");
    assert_eq!(scale.kind, Kind::Percent);
    assert_eq!(scale.label.en, "Scale");
    assert_eq!(scale.label.ar, "المقياس");
    assert_eq!(
        scale.default.as_ref().and_then(toml::Value::as_integer),
        Some(50)
    );
    assert_eq!(scale.min, Some(1.0));
    assert_eq!(scale.max, Some(1000.0));
    assert_eq!(scale.step, Some(5.0));
    assert_eq!(scale.unit.as_deref(), Some("%"));
    assert_eq!(scale.maps_to.as_deref(), Some("scale"));
    assert_eq!(scale.clears, ["max_side"]);
    assert!(!scale.no_enlarge);
    assert_eq!(scale.param_key(), "scale");

    let max_side = field_of(o, "max_side");
    assert_eq!(max_side.kind, Kind::Px);
    assert_eq!(
        max_side.default.as_ref().and_then(toml::Value::as_integer),
        Some(1920)
    );
    assert_eq!(max_side.clears, ["scale"]);

    let fit = field_of(o, "fit");
    assert_eq!(fit.kind, Kind::Enum);
    assert_eq!(
        fit.default.as_ref().and_then(toml::Value::as_str),
        Some("cover")
    );
    assert!(fit.no_enlarge);
    assert_eq!(fit.param_key(), "fit");
    let ids: Vec<&str> = fit.values.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, ["cover", "contain"]);
    assert_eq!(fit.values[0].label.en, "Cover");
    assert_eq!(fit.values[1].label.ar, "احتواء");

    assert_eq!(o.presets.len(), 2);
    assert_eq!(o.presets[0].id, "small");
    assert_eq!(
        o.presets[0]
            .values
            .get("scale")
            .and_then(toml::Value::as_integer),
        Some(50)
    );
    assert_eq!(
        o.presets[0].values.get("fit").and_then(toml::Value::as_str),
        Some("contain")
    );
    assert_eq!(o.presets[1].id, "large");
    assert_eq!(
        o.presets[1]
            .values
            .get("max_side")
            .and_then(toml::Value::as_integer),
        Some(4000)
    );
    assert!(o.validate().is_ok());
}

#[test]
fn resolve_with_no_explicit_keeps_base_params_and_no_default_is_applied() {
    let reg = Registry::builtin();
    let a = reg.get("image.to-webp").unwrap();
    let o = a.options.as_ref().expect("image.to-webp has options");
    let resolved = o.resolve(&a.params, &toml::Table::new()).unwrap();
    assert_eq!(resolved, a.params);

    let resize = reg.get("image.resize-50").unwrap();
    let o = resize
        .options
        .as_ref()
        .expect("image.resize-50 has options");
    let resolved = o
        .resolve_params(&toml::Table::new(), &toml::Table::new())
        .unwrap();
    assert!(resolved.is_empty(), "no default may be applied by resolve");
}

#[test]
fn resolve_inserts_under_the_param_key_for_field_id_and_param_key() {
    let o = spec(MAPPED);
    let base: toml::Table = toml::from_str("quality = 85\nstrip = true\nkeep = \"yes\"").unwrap();

    let mut by_id = toml::Table::new();
    by_id.insert("level".into(), toml::Value::Integer(60));
    let resolved = o.resolve(&base, &by_id).unwrap();
    assert_eq!(
        resolved.get("quality").and_then(toml::Value::as_integer),
        Some(60)
    );
    assert!(
        !resolved.contains_key("strip"),
        "clears removes the sibling key"
    );
    assert!(
        !resolved.contains_key("level"),
        "the field id is not a param"
    );
    assert_eq!(
        resolved.get("keep").and_then(toml::Value::as_str),
        Some("yes")
    );

    // The param key is looked up when the field id does not match.
    let mut by_param = toml::Table::new();
    by_param.insert("quality".into(), toml::Value::Integer(40));
    assert_eq!(
        o.resolve(&base, &by_param)
            .unwrap()
            .get("quality")
            .and_then(toml::Value::as_integer),
        Some(40)
    );

    // resolve_params reads param keys, and still accepts the field id.
    assert_eq!(
        o.resolve_params(&base, &by_param)
            .unwrap()
            .get("quality")
            .and_then(toml::Value::as_integer),
        Some(40)
    );
    assert_eq!(
        o.resolve_params(&base, &by_id)
            .unwrap()
            .get("quality")
            .and_then(toml::Value::as_integer),
        Some(60)
    );
}

#[test]
fn resolve_params_clears_the_other_resize_parameter() {
    let reg = Registry::builtin();
    let a = reg.get("image.resize-50").unwrap();
    let o = a.options.as_ref().unwrap();
    assert_eq!(
        a.params.get("scale").and_then(toml::Value::as_integer),
        Some(50)
    );

    let mut explicit = toml::Table::new();
    explicit.insert("max_side".into(), toml::Value::Integer(1920));
    let resolved = o.resolve_params(&a.params, &explicit).unwrap();
    assert_eq!(
        resolved.get("max_side").and_then(toml::Value::as_integer),
        Some(1920)
    );
    assert!(!resolved.contains_key("scale"), "scale must be cleared");

    let mut explicit = toml::Table::new();
    explicit.insert("scale".into(), toml::Value::Integer(25));
    let resolved = o.resolve_params(&a.params, &explicit).unwrap();
    assert_eq!(
        resolved.get("scale").and_then(toml::Value::as_integer),
        Some(25)
    );
    assert!(
        !resolved.contains_key("max_side"),
        "max_side must be cleared"
    );
}

#[test]
fn errors_name_the_option_and_the_reason() {
    let reg = Registry::builtin();
    let a = reg.get("image.to-webp").unwrap();
    let o = a.options.as_ref().unwrap();

    // Out of range: the field id and the side are in the message.
    for (value, side) in [(0, "below"), (101, "above")] {
        let err = resolve_one(o, "quality", toml::Value::Integer(value)).unwrap_err();
        assert!(matches!(err, OptionsError::Value { .. }), "{err:?}");
        let text = err.to_string();
        assert!(text.contains("quality"), "{text}");
        assert!(text.contains(side), "{text}");
    }

    // Wrong type.
    for bad in [toml::Value::String("high".into()), toml::Value::Float(60.5)] {
        let err = resolve_one(o, "quality", bad).unwrap_err();
        assert!(matches!(err, OptionsError::Type { .. }), "{err:?}");
        assert!(err.to_string().contains("expected"), "{err}");
    }

    // Unknown key.
    let o = spec(MAPPED);
    let err = resolve_one(&o, "nope", toml::Value::Integer(1)).unwrap_err();
    match &err {
        OptionsError::Unknown(name) => assert_eq!(name, "nope"),
        other => panic!("expected Unknown, got {other:?}"),
    }
    let text = err.to_string();
    assert!(text.contains("unknown option"), "{text}");
    assert!(text.contains("nope"), "{text}");
}

#[test]
fn values_are_checked_against_their_kind() {
    let o = spec(KINDS);
    let int_ok = [50, 1, 100];
    for value in int_ok {
        assert!(resolve_one(&o, "quality", toml::Value::Integer(value)).is_ok());
    }
    // Float accepts an integer or a float, but no string or bool.
    assert!(resolve_one(&o, "ratio", toml::Value::Integer(1)).is_ok());
    assert!(resolve_one(&o, "ratio", toml::Value::Float(1.5)).is_ok());
    assert!(matches!(
        resolve_one(&o, "ratio", toml::Value::String("x".into())),
        Err(OptionsError::Type { .. })
    ));
    assert!(matches!(
        resolve_one(&o, "ratio", toml::Value::Float(2.5)),
        Err(OptionsError::Value { .. })
    ));
    // Bool and string.
    assert!(resolve_one(&o, "flag", toml::Value::Boolean(true)).is_ok());
    assert!(matches!(
        resolve_one(&o, "flag", toml::Value::Integer(1)),
        Err(OptionsError::Type { .. })
    ));
    assert!(resolve_one(&o, "text", toml::Value::String("x".into())).is_ok());
    assert!(matches!(
        resolve_one(&o, "text", toml::Value::Integer(1)),
        Err(OptionsError::Type { .. })
    ));
    // px pair: two positive whole numbers joined by a lower case x.
    assert!(resolve_one(&o, "size", toml::Value::String("1920x1080".into())).is_ok());
    for bad in ["0x10", "1920X1080", "1920x", "x1080", "1920x0", "a"] {
        let err = resolve_one(&o, "size", toml::Value::String(bad.into())).unwrap_err();
        assert!(err.to_string().contains("size"), "{bad}: {err}");
    }
    // Enum membership.
    assert!(resolve_one(&o, "preset", toml::Value::String("a".into())).is_ok());
    let err = resolve_one(&o, "preset", toml::Value::String("c".into())).unwrap_err();
    assert!(err.to_string().contains("preset"), "{err}");
    assert!(matches!(
        resolve_one(&o, "preset", toml::Value::Integer(1)),
        Err(OptionsError::Type { .. })
    ));
}

#[test]
fn schema_describes_types_ranges_defaults_and_enums() {
    let o = spec(KINDS);
    let schema = o.params_schema();
    assert_eq!(schema["type"].as_str(), Some("object"));
    assert_eq!(schema["additionalProperties"].as_bool(), Some(false));
    let props = &schema["properties"];

    assert_eq!(props["quality"]["type"].as_str(), Some("integer"));
    assert_eq!(props["quality"]["minimum"].as_f64(), Some(1.0));
    assert_eq!(props["quality"]["maximum"].as_f64(), Some(100.0));
    assert_eq!(props["quality"]["default"].as_i64(), Some(70));
    assert_eq!(
        props["quality"]["description"].as_str(),
        Some("Quality (%)"),
        "the description is the English label plus the unit"
    );
    assert_eq!(props["ratio"]["type"].as_str(), Some("number"));
    assert_eq!(props["ratio"]["default"].as_f64(), Some(1.0));
    assert_eq!(props["flag"]["type"].as_str(), Some("boolean"));
    assert_eq!(props["flag"]["default"].as_bool(), Some(false));
    assert_eq!(props["text"]["type"].as_str(), Some("string"));
    assert_eq!(props["size"]["type"].as_str(), Some("string"));
    assert_eq!(
        props["size"]["pattern"].as_str(),
        Some("^[1-9][0-9]*x[1-9][0-9]*$")
    );
    assert_eq!(props["preset"]["type"].as_str(), Some("string"));
    assert_eq!(props["preset"]["default"].as_str(), Some("a"));
    let ids: Vec<&str> = props["preset"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(ids, ["a", "b"]);

    // The property key is the param key, not the field id.
    let mapped = spec(MAPPED).params_schema();
    let props = &mapped["properties"];
    assert!(props.get("level").is_none());
    assert_eq!(props["quality"]["type"].as_str(), Some("integer"));
    assert!(props.get("size").is_some());
}

#[test]
fn validate_rejects_invalid_specs() {
    assert!(spec("").validate().is_ok(), "an empty spec is valid");
    assert!(spec("kind = \"none\"").validate().is_ok());

    // kind none forbids fields and presets.
    invalid(&field("a", "int", ""));
    invalid(&format!(
        "kind = \"none\"\n{}\n[[preset]]\nid = \"p\"\nvalues = {{}}\n",
        field("a", "int", "")
    ));

    // Field ids are unique and not empty.
    invalid(&format!(
        "kind = \"form\"\n{}{}",
        field("dup", "int", ""),
        field("dup", "int", "")
    ));
    invalid(&format!("kind = \"form\"\n{}", field("", "int", "")));

    // Enums need values, and a default that is one of them.
    invalid(&format!(
        "kind = \"form\"\n{}",
        field("mode", "enum", "default = \"a\"")
    ));
    invalid(&format!(
        "kind = \"form\"\n{}",
        field(
            "mode",
            "enum",
            "default = \"b\"\nvalues = [{ id = \"a\", label = { ar = \"أ\", en = \"A\" } }]"
        )
    ));

    // The default follows the kind and the range.
    invalid(&format!(
        "kind = \"form\"\n{}",
        field("q", "int", "default = \"high\"")
    ));
    invalid(&format!(
        "kind = \"form\"\n{}",
        field("q", "int", "default = 500\nmin = 1.0\nmax = 100.0")
    ));

    // min above max, shared param keys, and a clear of the field's own key.
    invalid(&format!(
        "kind = \"form\"\n{}",
        field("q", "int", "min = 10.0\nmax = 5.0")
    ));
    invalid(&format!(
        "kind = \"form\"\n{}{}",
        field("a", "int", "maps_to = \"same\""),
        field("b", "int", "maps_to = \"same\"")
    ));
    invalid(&format!(
        "kind = \"form\"\n{}",
        field("x", "int", "maps_to = \"q\"\nclears = [\"q\"]")
    ));

    // Preset values name fields and pass the same value rules.
    invalid(&format!(
        "kind = \"form\"\n{}\n[[preset]]\nid = \"p\"\nvalues = {{ nope = 1 }}\n",
        field("mode", "int", "")
    ));
    invalid(&format!(
        "kind = \"form\"\n{}\n[[preset]]\nid = \"p\"\nvalues = {{ mode = \"x\" }}\n",
        field("mode", "int", "")
    ));

    // A complete spec with one preset is valid.
    let good = format!(
        "kind = \"form\"\n{}\n[[preset]]\nid = \"p\"\nvalues = {{ mode = \"a\" }}\n",
        field(
            "mode",
            "enum",
            "default = \"a\"\nvalues = [{ id = \"a\", label = { ar = \"أ\", en = \"A\" } }]"
        )
    );
    assert!(spec(&good).validate().is_ok(), "valid spec rejected");
}

#[test]
fn invalid_options_fail_the_action_load_with_the_action_id() {
    let doc = r#"
[[action]]
id = "bad.options"
engine = "image"
op = "resize"
ring = 1
accepts = ["image"]
output = { mode = "per-file" }
code = { ar = "ب", en = "BAD" }
label = { ar = "سيئ", en = "Bad" }

[action.options]
kind = "form"

[[action.options.field]]
id = "mode"
type = "enum"
label = { ar = "نمط", en = "Mode" }
default = "a"
"#;
    let mut reg = Registry::default();
    let err = reg.add_toml("bad.toml", doc).unwrap_err();
    assert!(matches!(err, RegistryError::Parse { .. }), "{err:?}");
    assert!(err.to_string().contains("bad.options"), "{err}");
    assert!(reg.all().is_empty());
}

#[test]
fn builtin_actions_carry_options_and_all_56_parse() {
    let reg = Registry::builtin();
    // 56 since S-123 added pdf.extract-pages, pdf.delete-pages and
    // pdf.reorder-pages, S-125 pdf.metadata, S-126 pdf.to-text, S-127
    // image.resize-to, image.crop and image.compress-to, and S-128
    // pdf.linearize, pdf.cleanup and pdf.check (none of the qpdf actions
    // carries options, so the options list below is unchanged).
    assert_eq!(reg.all().len(), 56, "every built-in action still parses");
    let with_options: Vec<&str> = reg
        .all()
        .iter()
        .filter(|a| a.options.is_some())
        .map(|a| a.id.as_str())
        .collect();
    assert_eq!(
        with_options,
        [
            "image.to-webp",
            "image.to-jpg",
            "image.to-avif",
            // S-123 added page size, orientation, margin and dpi here too.
            "image.to-pdf",
            "image.resize-50",
            "image.compress",
            // S-127 appended the three image mode actions at the end of
            // actions/images.toml, so they follow the image group here.
            "image.resize-to",
            "image.crop",
            "image.compress-to",
            "pdf.compress",
            "pdf.to-images",
            "pdf.split",
            "pdf.extract-pages",
            "pdf.delete-pages",
            "pdf.reorder-pages",
            "pdf.metadata",
            "pdf.to-text",
        ]
    );
    assert!(reg.get("image.to-png").unwrap().options.is_none());

    // The quality spec of every lossy image action takes the base params default.
    for (id, quality) in [
        ("image.to-webp", 85),
        ("image.to-jpg", 90),
        ("image.to-avif", 70),
        ("image.compress", 75),
    ] {
        let a = reg.get(id).unwrap();
        let o = a.options.as_ref().unwrap();
        let f = field_of(o, "quality");
        assert_eq!(f.kind, Kind::Int, "{id}");
        assert_eq!(f.param_key(), "quality", "{id}");
        assert_eq!(f.min, Some(1.0), "{id}");
        assert_eq!(f.max, Some(100.0), "{id}");
        assert_eq!(f.unit.as_deref(), Some("%"), "{id}");
        assert_eq!(
            f.default.as_ref().and_then(toml::Value::as_integer),
            Some(a.params["quality"].as_integer().unwrap()),
            "{id} default follows [action.params]"
        );
        assert_eq!(
            f.default.as_ref().and_then(toml::Value::as_integer),
            Some(quality),
            "{id}"
        );
    }

    // image.resize-50 is the form spec with two mutually clearing fields,
    // plus the S-129 keep_metadata flag.
    let resize = reg.get("image.resize-50").unwrap();
    let o = resize.options.as_ref().unwrap();
    assert_eq!(o.kind, OptionsKind::Form);
    assert_eq!(o.readout, ["dimensions", "ratio", "percent", "output_name"]);
    assert_eq!(o.fields.len(), 3);
    let scale = field_of(o, "scale");
    assert_eq!(scale.kind, Kind::Percent);
    assert_eq!(scale.min, Some(1.0));
    assert_eq!(scale.max, Some(1000.0));
    assert_eq!(
        scale.default.as_ref().and_then(toml::Value::as_integer),
        Some(50)
    );
    assert_eq!(scale.maps_to.as_deref(), Some("scale"));
    assert_eq!(scale.clears, ["max_side"]);
    let max_side = field_of(o, "max_side");
    assert_eq!(max_side.kind, Kind::Px);
    assert_eq!(max_side.min, Some(16.0));
    assert_eq!(max_side.max, Some(20000.0));
    assert_eq!(
        max_side.default.as_ref().and_then(toml::Value::as_integer),
        Some(1920)
    );
    assert_eq!(max_side.maps_to.as_deref(), Some("max_side"));
    assert_eq!(max_side.clears, ["scale"]);
    let keep = field_of(o, "keep_metadata");
    assert_eq!(keep.kind, Kind::Bool);
    assert_eq!(
        keep.default.as_ref().and_then(toml::Value::as_bool),
        Some(false)
    );
    assert_eq!(keep.maps_to.as_deref(), Some("keep_metadata"));

    // pdf.compress picks a preset, pdf.to-images a dpi.
    let compress = reg.get("pdf.compress").unwrap();
    let f = field_of(compress.options.as_ref().unwrap(), "preset");
    assert_eq!(f.kind, Kind::Enum);
    assert_eq!(f.param_key(), "preset");
    let ids: Vec<&str> = f.values.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, ["screen", "ebook", "printer", "prepress"]);
    assert_eq!(
        f.default.as_ref().and_then(toml::Value::as_str),
        Some("ebook")
    );
    let to_images = reg.get("pdf.to-images").unwrap();
    let f = field_of(to_images.options.as_ref().unwrap(), "dpi");
    assert_eq!(f.kind, Kind::Int);
    assert_eq!(f.min, Some(72.0));
    assert_eq!(f.max, Some(600.0));
    assert_eq!(
        f.default.as_ref().and_then(toml::Value::as_integer),
        Some(150)
    );

    // Every built-in spec is valid and every label has Arabic and English.
    for a in reg.all() {
        let Some(o) = &a.options else { continue };
        o.validate().unwrap_or_else(|e| panic!("{}: {e}", a.id));
        for f in &o.fields {
            assert!(
                !f.label.ar.is_empty() && !f.label.en.is_empty(),
                "{} field {} label",
                a.id,
                f.id
            );
            for c in &f.values {
                assert!(
                    !c.label.ar.is_empty() && !c.label.en.is_empty(),
                    "{} choice {}",
                    a.id,
                    c.id
                );
            }
        }
    }
}

/// A field id that collides with another field's param key.
const COLLIDING: &str = r#"
kind = "form"

[[field]]
id = "quality"
type = "int"
label = { ar = "الجودة", en = "Quality" }
default = 70
min = 1.0
max = 100.0
maps_to = "q"

[[field]]
id = "level"
type = "int"
label = { ar = "المستوى", en = "Level" }
default = 5
min = 1.0
max = 10.0
maps_to = "quality"
"#;

#[test]
fn a_colliding_param_key_does_not_apply_a_second_field() {
    let o = spec(COLLIDING);
    assert!(o.validate().is_ok(), "the colliding spec is valid");
    let mut explicit = toml::Table::new();
    explicit.insert("quality".into(), toml::Value::Integer(50));
    let resolved = o.resolve(&toml::Table::new(), &explicit).unwrap();
    assert_eq!(
        resolved.get("q").and_then(toml::Value::as_integer),
        Some(50),
        "the field id match wins"
    );
    assert!(!resolved.contains_key("quality"), "{resolved:?}");
    assert_eq!(resolved.len(), 1, "only one field is applied");
}

#[test]
fn a_typo_inside_an_options_field_fails_to_parse() {
    let doc = r#"
[[action]]
id = "typo.options"
engine = "image"
op = "resize"
ring = 1
accepts = ["image"]
output = { mode = "per-file" }
code = { ar = "خ", en = "TYPO" }
label = { ar = "خطأ", en = "Typo" }

[action.options]
kind = "form"

[[action.options.field]]
id = "quality"
type = "int"
label = { ar = "الجودة", en = "Quality" }
deafult = 85
"#;
    let mut reg = Registry::default();
    let err = reg.add_toml("typo.toml", doc).unwrap_err();
    assert!(matches!(err, RegistryError::Parse { .. }), "{err:?}");
    assert!(reg.all().is_empty(), "a failed load changes nothing");
}
