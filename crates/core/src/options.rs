//! Typed option descriptors for actions (S-117).
//!
//! An action may describe the options its engine takes, so the MCP server
//! (S-121) can build a JSON Schema from the same data and the hive can render
//! a form later. The block is data, like the rest of the action:
//!
//! `toml
//! [action.params]
//! quality = 85
//! [action.options]
//! kind = "form"
//! [[action.options.field]]
//! id = "quality"
//! type = "int"
//! label = { ar = "الجودة", en = "Quality" }
//! default = 85
//! min = 1
//! max = 100
//! unit = "%"
//! maps_to = "quality"
//! `
//!
//! [OptionsSpec::resolve] and [OptionsSpec::resolve_params] turn the values
//! a caller gives into the params table the queue runs with. No default is
//! applied there: the action's `[action.params]` table already carries them, so
//! the schema and a future UI read the defaults from the field.

use crate::action::Localized;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// The value pattern of the `px-pair` kind, as a JSON Schema `pattern`.
pub const PX_PAIR_PATTERN: &str = "^[1-9][0-9]*x[1-9][0-9]*$";

/// The shape of an options block.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OptionsKind {
    /// The action takes no options and its block holds no field.
    #[default]
    None,
    /// The options render as a form.
    Form,
}

/// The value type of one option field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// A whole percent, for example a scale of 50.
    Percent,
    /// Whole pixels, for example a maximum side of 1920.
    Px,
    /// Two whole pixel counts written `WIDTHxHEIGHT`, for example `1920x1080`.
    PxPair,
    /// One of the field's `values`.
    Enum,
    /// `true` or `false`.
    Bool,
    /// A whole number.
    Int,
    /// A number, whole or fractional.
    Float,
    /// Free text.
    String,
}

/// One accepted value of an `enum` field.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Choice {
    /// The value the params table receives.
    pub id: String,
    pub label: Localized,
}

/// A named set of field values a caller may apply in one go.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preset {
    pub id: String,
    /// Field id to value.
    #[serde(default)]
    pub values: toml::Table,
}

/// One option field of an action.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    /// The option name shown to the user; the first lookup key of
    /// [OptionsSpec::resolve].
    pub id: String,
    /// The value type.
    #[serde(rename = "type")]
    pub kind: Kind,
    pub label: Localized,
    /// The value the form starts with. The engine receives it through the
    /// action's `[action.params]` table, not through resolution.
    #[serde(default)]
    pub default: Option<toml::Value>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub step: Option<f64>,
    /// Unit shown beside the value, for example `%`.
    #[serde(default)]
    pub unit: Option<String>,
    /// The accepted values of an `enum` field.
    #[serde(default)]
    pub values: Vec<Choice>,
    /// Engine param this field sets. Defaults to the field id.
    #[serde(default)]
    pub maps_to: Option<String>,
    /// Params removed from the base table when this field is applied, for
    /// options that exclude each other.
    #[serde(default)]
    pub clears: Vec<String>,
    /// UI hint: do not offer to grow a value above the current one.
    #[serde(default)]
    pub no_enlarge: bool,
}

/// The `[action.options]` block of one action.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptionsSpec {
    #[serde(default)]
    pub kind: OptionsKind,
    /// Readouts the cell shows, for example "dimensions" or "percent".
    #[serde(default)]
    pub readout: Vec<String>,
    /// Keep the last used values between runs.
    #[serde(default)]
    pub remember: bool,
    /// Template for the output name suffix, for example "-{percent}".
    #[serde(default)]
    pub suffix_template: String,
    #[serde(default, rename = "field")]
    pub fields: Vec<Field>,
    #[serde(default, rename = "preset")]
    pub presets: Vec<Preset>,
}

/// An explicit option value that cannot be used.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum OptionsError {
    #[error("unknown option `{0}`")]
    Unknown(String),
    #[error("option `{name}`: expected {expected}")]
    Type { name: String, expected: String },
    #[error("option `{name}`: {reason}")]
    Value { name: String, reason: String },
}

/// A failed value check before it is attached to a name.
enum Bad {
    /// The TOML type does not match the field kind.
    Type(&'static str),
    /// The type matches but the value does not.
    Value(String),
}

impl Field {
    /// The engine param this field sets.
    pub fn param_key(&self) -> &str {
        self.maps_to.as_deref().unwrap_or(&self.id)
    }
}

fn number(v: &toml::Value) -> Option<f64> {
    match v {
        toml::Value::Integer(n) => Some(*n as f64),
        toml::Value::Float(n) => Some(*n),
        _ => None,
    }
}

/// True for `WIDTHxHEIGHT`: two positive whole counts with no leading zero.
fn is_px_pair(s: &str) -> bool {
    let Some((w, h)) = s.split_once('x') else {
        return false;
    };
    let dim = |t: &str| {
        let mut bytes = t.bytes();
        matches!(bytes.next(), Some(b'1'..=b'9')) && bytes.all(|b| b.is_ascii_digit())
    };
    dim(w) && dim(h)
}

/// Checks one value against the field's kind, enum values and range.
fn check_value(f: &Field, v: &toml::Value) -> Result<(), Bad> {
    match f.kind {
        Kind::Int | Kind::Percent | Kind::Px => {
            if !matches!(v, toml::Value::Integer(_)) {
                return Err(Bad::Type("integer"));
            }
        }
        Kind::Float => {
            if number(v).is_none() {
                return Err(Bad::Type("number"));
            }
        }
        Kind::Bool => {
            if !matches!(v, toml::Value::Boolean(_)) {
                return Err(Bad::Type("boolean"));
            }
        }
        Kind::String => {
            if !matches!(v, toml::Value::String(_)) {
                return Err(Bad::Type("string"));
            }
        }
        Kind::PxPair => {
            let toml::Value::String(s) = v else {
                return Err(Bad::Type("string"));
            };
            if !is_px_pair(s) {
                return Err(Bad::Value(format!(
                    "expected WIDTHxHEIGHT, like 1920x1080, not `{s}`"
                )));
            }
        }
        Kind::Enum => {
            let toml::Value::String(s) = v else {
                return Err(Bad::Type("string"));
            };
            if !f.values.iter().any(|c| &c.id == s) {
                let ids: Vec<&str> = f.values.iter().map(|c| c.id.as_str()).collect();
                return Err(Bad::Value(format!("expected one of {}", ids.join(", "))));
            }
        }
    }
    if let Some(n) = number(v) {
        if let Some(min) = f.min
            && n < min
        {
            return Err(Bad::Value(format!("{n} is below the minimum {min}")));
        }
        if let Some(max) = f.max
            && n > max
        {
            return Err(Bad::Value(format!("{n} is above the maximum {max}")));
        }
    }
    Ok(())
}

fn bad_message(f: &Field, bad: Bad) -> String {
    match bad {
        Bad::Type(expected) => format!("option `{}`: expected {expected}", f.id),
        Bad::Value(reason) => format!("option `{}`: {reason}", f.id),
    }
}

/// Validates `v` as the value of `f` and writes it under the field's param
/// key, after removing the params the field clears.
fn apply(f: &Field, v: &toml::Value, out: &mut toml::Table) -> Result<(), OptionsError> {
    check_value(f, v).map_err(|bad| match bad {
        Bad::Type(expected) => OptionsError::Type {
            name: f.id.clone(),
            expected: expected.to_string(),
        },
        Bad::Value(reason) => OptionsError::Value {
            name: f.id.clone(),
            reason,
        },
    })?;
    for key in &f.clears {
        out.remove(key);
    }
    out.insert(f.param_key().to_string(), v.clone());
    Ok(())
}

/// The JSON Schema property type of a kind.
fn schema_type(kind: Kind) -> &'static str {
    match kind {
        Kind::Int | Kind::Percent | Kind::Px => "integer",
        Kind::Float => "number",
        Kind::Bool => "boolean",
        Kind::Enum | Kind::String | Kind::PxPair => "string",
    }
}

/// An integral f64 as a JSON integer, so a schema bound compares like the TOML
/// number it came from.
fn json_number(n: f64) -> serde_json::Value {
    if n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
        serde_json::Value::from(n as i64)
    } else {
        serde_json::Value::from(n)
    }
}

impl OptionsSpec {
    /// Checks the block. The error names the field or preset at fault.
    pub fn validate(&self) -> Result<(), String> {
        if self.kind == OptionsKind::None {
            if !self.fields.is_empty() {
                return Err(String::from("kind `none` takes no field"));
            }
            if !self.presets.is_empty() {
                return Err(String::from("kind `none` takes no preset"));
            }
        }
        let mut ids: HashSet<&str> = HashSet::new();
        let mut keys: HashSet<&str> = HashSet::new();
        for f in &self.fields {
            if f.id.is_empty() {
                return Err(String::from("a field id is empty"));
            }
            if !ids.insert(f.id.as_str()) {
                return Err(format!("duplicate field id `{}`", f.id));
            }
            let key = f.param_key();
            if !keys.insert(key) {
                return Err(format!(
                    "field `{}` and another field both set the param `{key}`",
                    f.id
                ));
            }
            if f.kind == Kind::Enum && f.values.is_empty() {
                return Err(format!("option `{}`: an enum needs values", f.id));
            }
            if let Some(v) = &f.default {
                check_value(f, v).map_err(|bad| bad_message(f, bad))?;
            }
            if let (Some(min), Some(max)) = (f.min, f.max)
                && min > max
            {
                return Err(format!("option `{}`: min is above max", f.id));
            }
            if f.clears.iter().any(|c| c == key) {
                return Err(format!("option `{}`: clears its own param `{key}`", f.id));
            }
        }
        for p in &self.presets {
            for (k, v) in &p.values {
                let Some(f) = self.fields.iter().find(|f| &f.id == k) else {
                    return Err(format!("preset `{}`: `{k}` is not a field id", p.id));
                };
                check_value(f, v).map_err(|bad| bad_message(f, bad))?;
            }
        }
        Ok(())
    }

    /// The field an explicit option key names: the field id wins over a param
    /// key, so a key that is one field's id and another field's param key
    /// belongs to the field whose id it is.
    fn field_for_option(&self, key: &str) -> Option<usize> {
        self.fields
            .iter()
            .position(|f| f.id == key)
            .or_else(|| self.fields.iter().position(|f| f.param_key() == key))
    }

    /// The field an explicit engine param names: the param key wins over a
    /// field id.
    fn field_for_param(&self, key: &str) -> Option<usize> {
        self.fields
            .iter()
            .position(|f| f.param_key() == key)
            .or_else(|| self.fields.iter().position(|f| f.id == key))
    }

    /// Every explicit key resolves to one field, and every field is applied at
    /// most once, so two keys never write the same field twice.
    fn picks(&self, keys: &[&String], by_param: bool) -> Result<Vec<usize>, OptionsError> {
        let mut picks = Vec::with_capacity(keys.len());
        for key in keys {
            let found = if by_param {
                self.field_for_param(key)
            } else {
                self.field_for_option(key)
            };
            match found {
                Some(i) => picks.push(i),
                None => return Err(OptionsError::Unknown((*key).clone())),
            }
        }
        Ok(picks)
    }

    /// Applies explicit option values to a copy of `base`. A key is looked up
    /// by field id first, then by param key, so a caller may speak either.
    pub fn resolve(
        &self,
        base: &toml::Table,
        explicit_fields: &toml::Table,
    ) -> Result<toml::Table, OptionsError> {
        let keys: Vec<&String> = explicit_fields.keys().collect();
        let picks = self.picks(&keys, false)?;
        let mut out = base.clone();
        let mut applied = vec![false; self.fields.len()];
        for ((_, v), i) in explicit_fields.iter().zip(picks) {
            if applied[i] {
                continue;
            }
            applied[i] = true;
            apply(&self.fields[i], v, &mut out)?;
        }
        Ok(out)
    }

    /// Applies explicit engine params to a copy of `base`. A key is looked up
    /// by param key first, then by field id.
    pub fn resolve_params(
        &self,
        base: &toml::Table,
        explicit_params: &toml::Table,
    ) -> Result<toml::Table, OptionsError> {
        let keys: Vec<&String> = explicit_params.keys().collect();
        let picks = self.picks(&keys, true)?;
        let mut out = base.clone();
        let mut applied = vec![false; self.fields.len()];
        for ((_, v), i) in explicit_params.iter().zip(picks) {
            if applied[i] {
                continue;
            }
            applied[i] = true;
            apply(&self.fields[i], v, &mut out)?;
        }
        Ok(out)
    }

    /// A JSON Schema for the resolved params table, one property per field
    /// under its param key.
    pub fn params_schema(&self) -> serde_json::Value {
        let mut properties = serde_json::Map::new();
        for f in &self.fields {
            let mut schema = serde_json::Map::new();
            schema.insert(
                String::from("type"),
                serde_json::Value::from(schema_type(f.kind)),
            );
            if let Some(min) = f.min {
                schema.insert(String::from("minimum"), json_number(min));
            }
            if let Some(max) = f.max {
                schema.insert(String::from("maximum"), json_number(max));
            }
            if f.kind == Kind::Enum {
                let ids: Vec<serde_json::Value> = f
                    .values
                    .iter()
                    .map(|c| serde_json::Value::from(c.id.as_str()))
                    .collect();
                schema.insert(String::from("enum"), serde_json::Value::Array(ids));
            }
            if f.kind == Kind::PxPair {
                schema.insert(
                    String::from("pattern"),
                    serde_json::Value::from(PX_PAIR_PATTERN),
                );
            }
            if let Some(default) = &f.default {
                let value = serde_json::to_value(default).unwrap_or(serde_json::Value::Null);
                schema.insert(String::from("default"), value);
            }
            let description = match &f.unit {
                Some(unit) => format!("{} ({unit})", f.label.en),
                None => f.label.en.clone(),
            };
            schema.insert(
                String::from("description"),
                serde_json::Value::from(description),
            );
            properties.insert(f.param_key().to_string(), serde_json::Value::Object(schema));
        }
        serde_json::json!({
            "type": "object",
            "properties": properties,
            "additionalProperties": false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Registry;

    fn label(text: &str) -> Localized {
        Localized {
            ar: String::from("ع"),
            en: text.to_string(),
        }
    }

    fn field(id: &str, kind: Kind) -> Field {
        Field {
            id: id.to_string(),
            kind,
            label: label(id),
            default: None,
            min: None,
            max: None,
            step: None,
            unit: None,
            values: vec![],
            maps_to: None,
            clears: vec![],
            no_enlarge: false,
        }
    }

    fn spec(fields: Vec<Field>) -> OptionsSpec {
        OptionsSpec {
            kind: OptionsKind::Form,
            readout: vec![],
            remember: false,
            suffix_template: String::new(),
            fields,
            presets: vec![],
        }
    }

    fn int(n: i64) -> toml::Value {
        toml::Value::Integer(n)
    }

    fn text(t: &str) -> toml::Value {
        toml::Value::String(t.to_string())
    }

    fn table(pairs: &[(&str, toml::Value)]) -> toml::Table {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.clone()))
            .collect()
    }

    /// A valid action document with the options block placed after
    /// [action.params].
    fn doc(options: &str) -> String {
        format!(
            r#"
[[action]]
id = "x.test"
engine = "image"
op = "convert"
ring = 1
accepts = ["image"]
output = {{ mode = "per-file", ext = "webp" }}
code = {{ ar = "و", en = "W" }}
label = {{ ar = "ويب", en = "WebP" }}
[action.params]
quality = 85
{options}
"#
        )
    }

    #[test]
    fn parses_a_full_options_block() {
        let options = r#"[action.options]
kind = "form"
readout = ["percent"]
remember = true
suffix_template = "-{percent}"
[[action.options.field]]
id = "quality"
type = "int"
label = { ar = "الجودة", en = "Quality" }
default = 85
min = 1
max = 100
unit = "%"
maps_to = "quality"
[[action.options.field]]
id = "preset"
type = "enum"
label = { ar = "إعداد", en = "Preset" }
default = "ebook"
[[action.options.field.values]]
id = "ebook"
label = { ar = "كتاب", en = "Ebook" }
[[action.options.field.values]]
id = "screen"
label = { ar = "شاشة", en = "Screen" }
[[action.options.preset]]
id = "small"
[action.options.preset.values]
quality = 60
"#;
        let mut reg = Registry::default();
        reg.add_toml("x.toml", &doc(options)).unwrap();
        let block = reg.get("x.test").unwrap().options.as_ref().unwrap();
        assert_eq!(block.kind, OptionsKind::Form);
        assert_eq!(block.readout, ["percent"]);
        assert!(block.remember);
        assert_eq!(block.suffix_template, "-{percent}");
        assert_eq!(block.fields.len(), 2);
        let quality = &block.fields[0];
        assert_eq!(quality.kind, Kind::Int);
        assert_eq!(quality.default, Some(int(85)));
        assert_eq!((quality.min, quality.max), (Some(1.0), Some(100.0)));
        assert_eq!(quality.unit.as_deref(), Some("%"));
        assert_eq!(quality.param_key(), "quality");
        assert_eq!(quality.label.en, "Quality");
        let preset = &block.fields[1];
        assert_eq!(preset.kind, Kind::Enum);
        assert_eq!(preset.values.len(), 2);
        assert_eq!(preset.values[0].id, "ebook");
        assert_eq!(block.presets[0].id, "small");
        assert_eq!(block.presets[0].values.get("quality"), Some(&int(60)));
        block.validate().unwrap();
    }

    #[test]
    fn options_are_absent_by_default() {
        let mut reg = Registry::default();
        reg.add_toml("x.toml", &doc("")).unwrap();
        let action = reg.get("x.test").unwrap();
        assert!(action.options.is_none());
        let json = serde_json::to_value(action).unwrap();
        assert!(json.get("options").is_none());
    }

    #[test]
    fn an_unknown_options_key_is_a_parse_error() {
        let options = r#"[action.options]
kind = "form"
nope = 1
"#;
        let mut reg = Registry::default();
        let err = reg.add_toml("x.toml", &doc(options)).unwrap_err();
        assert!(matches!(err, crate::action::RegistryError::Parse { .. }));
    }

    #[test]
    fn action_validation_wraps_a_bad_options_block() {
        let options = r#"[action.options]
kind = "form"
[[action.options.field]]
id = "quality"
type = "int"
label = { ar = "الجودة", en = "Quality" }
default = "high"
"#;
        let mut reg = Registry::default();
        match reg.add_toml("x.toml", &doc(options)).unwrap_err() {
            crate::action::RegistryError::Parse { file, reason } => {
                assert_eq!(file, "x.toml");
                assert!(reason.contains("x.test"), "{reason}");
                assert!(reason.contains("quality"), "{reason}");
            }
            other => panic!("expected Parse, got {other:?}"),
        }
    }

    #[test]
    fn validate_none_kind_takes_no_field_or_preset() {
        let mut block = spec(vec![field("quality", Kind::Int)]);
        block.kind = OptionsKind::None;
        assert!(block.validate().is_err());
        let mut block = spec(vec![]);
        block.kind = OptionsKind::None;
        block.presets.push(Preset {
            id: String::from("small"),
            values: toml::Table::new(),
        });
        assert!(block.validate().is_err());
        assert!(spec(vec![]).validate().is_ok());
    }

    #[test]
    fn validate_rejects_duplicate_and_empty_ids() {
        assert!(spec(vec![field("", Kind::Int)]).validate().is_err());
        let block = spec(vec![field("a", Kind::Int), field("a", Kind::String)]);
        assert!(block.validate().is_err());
    }

    #[test]
    fn validate_rejects_two_fields_on_one_param() {
        let mut a = field("a", Kind::Int);
        a.maps_to = Some(String::from("same"));
        let mut b = field("b", Kind::Int);
        b.maps_to = Some(String::from("same"));
        assert!(spec(vec![a, b]).validate().is_err());
    }

    #[test]
    fn validate_checks_enum_values() {
        let mut e = field("preset", Kind::Enum);
        assert!(spec(vec![e.clone()]).validate().is_err());
        e.values.push(Choice {
            id: String::from("ebook"),
            label: label("Ebook"),
        });
        e.default = Some(text("screen"));
        assert!(spec(vec![e.clone()]).validate().is_err());
        e.default = Some(text("ebook"));
        assert!(spec(vec![e]).validate().is_ok());
    }

    #[test]
    fn validate_checks_default_type_and_range() {
        let mut f = field("quality", Kind::Int);
        f.default = Some(text("high"));
        assert!(spec(vec![f.clone()]).validate().is_err());
        f.default = Some(int(101));
        f.min = Some(1.0);
        f.max = Some(100.0);
        assert!(spec(vec![f.clone()]).validate().is_err());
        f.default = Some(int(85));
        assert!(spec(vec![f]).validate().is_ok());
        let mut whole_float = field("ratio", Kind::Float);
        whole_float.default = Some(int(2));
        assert!(spec(vec![whole_float]).validate().is_ok());
        let mut inverted = field("ratio", Kind::Float);
        inverted.min = Some(5.0);
        inverted.max = Some(1.0);
        assert!(spec(vec![inverted]).validate().is_err());
    }

    #[test]
    fn validate_rejects_clearing_its_own_param() {
        let mut f = field("scale", Kind::Percent);
        f.clears = vec![String::from("scale")];
        assert!(spec(vec![f]).validate().is_err());
    }

    #[test]
    fn validate_checks_preset_values() {
        let mut unknown = spec(vec![field("quality", Kind::Int)]);
        unknown.presets.push(Preset {
            id: String::from("small"),
            values: table(&[("nope", int(1))]),
        });
        assert!(unknown.validate().is_err());
        let mut wrong = spec(vec![field("quality", Kind::Int)]);
        wrong.presets.push(Preset {
            id: String::from("small"),
            values: table(&[("quality", text("x"))]),
        });
        assert!(wrong.validate().is_err());
        let mut ok = spec(vec![field("quality", Kind::Int)]);
        ok.presets.push(Preset {
            id: String::from("small"),
            values: table(&[("quality", int(60))]),
        });
        assert!(ok.validate().is_ok());
    }

    #[test]
    fn px_pair_needs_two_positive_counts() {
        let mut f = field("size", Kind::PxPair);
        for good in ["1x1", "1920x1080", "16x9000"] {
            f.default = Some(text(good));
            assert!(spec(vec![f.clone()]).validate().is_ok(), "{good}");
        }
        for bad in [
            "0x100", "100x0", "100", "x100", "100x", "01x10", "10x01", "-1x2", "1X2", "1 2", "",
        ] {
            f.default = Some(text(bad));
            assert!(spec(vec![f.clone()]).validate().is_err(), "{bad}");
        }
        f.default = Some(int(5));
        assert!(spec(vec![f]).validate().is_err());
    }

    fn quality() -> Field {
        let mut f = field("quality", Kind::Int);
        f.min = Some(1.0);
        f.max = Some(100.0);
        f
    }

    #[test]
    fn resolve_applies_explicit_values_and_clears() {
        let mut scale = field("scale", Kind::Percent);
        scale.clears = vec![String::from("max_side")];
        let mut max_side = field("max_side", Kind::Px);
        max_side.clears = vec![String::from("scale")];
        let block = spec(vec![scale, max_side]);
        let base = table(&[("scale", int(50)), ("max_side", int(1920))]);
        let out = block.resolve(&base, &table(&[("scale", int(25))])).unwrap();
        assert_eq!(out.get("scale"), Some(&int(25)));
        assert_eq!(out.get("max_side"), None);
        assert_eq!(base.get("max_side"), Some(&int(1920)));
        let out = block
            .resolve(&base, &table(&[("max_side", int(800))]))
            .unwrap();
        assert_eq!(out.get("max_side"), Some(&int(800)));
        assert_eq!(out.get("scale"), None);
        let out = block.resolve(&base, &toml::Table::new()).unwrap();
        assert_eq!(out, base);
    }

    #[test]
    fn resolve_rejects_unknown_keys() {
        let block = spec(vec![quality()]);
        let base = table(&[("quality", int(85))]);
        assert_eq!(
            block
                .resolve(&base, &table(&[("nope", int(1))]))
                .unwrap_err(),
            OptionsError::Unknown(String::from("nope"))
        );
        assert_eq!(
            spec(vec![])
                .resolve(&toml::Table::new(), &table(&[("quality", int(1))]))
                .unwrap_err(),
            OptionsError::Unknown(String::from("quality"))
        );
    }

    #[test]
    fn resolve_rejects_a_wrong_type_and_a_range() {
        let block = spec(vec![quality()]);
        let base = table(&[("quality", int(85))]);
        match block
            .resolve(&base, &table(&[("quality", text("high"))]))
            .unwrap_err()
        {
            OptionsError::Type { name, expected } => {
                assert_eq!(name, "quality");
                assert_eq!(expected, "integer");
            }
            other => panic!("expected Type, got {other:?}"),
        }
        match block
            .resolve(&base, &table(&[("quality", int(0))]))
            .unwrap_err()
        {
            OptionsError::Value { name, reason } => {
                assert_eq!(name, "quality");
                assert!(reason.contains("below"), "{reason}");
            }
            other => panic!("expected Value, got {other:?}"),
        }
        match block
            .resolve(&base, &table(&[("quality", int(101))]))
            .unwrap_err()
        {
            OptionsError::Value { reason, .. } => assert!(reason.contains("above"), "{reason}"),
            other => panic!("expected Value, got {other:?}"),
        }
        assert_eq!(base.get("quality"), Some(&int(85)));
    }

    #[test]
    fn resolve_does_not_apply_field_defaults() {
        let mut f = field("quality", Kind::Int);
        f.default = Some(int(85));
        let base = table(&[("page", text("fit"))]);
        let out = spec(vec![f]).resolve(&base, &toml::Table::new()).unwrap();
        assert_eq!(out.get("quality"), None);
    }

    #[test]
    fn resolve_params_maps_param_keys_and_field_ids() {
        let mut f = field("quality_option", Kind::Int);
        f.maps_to = Some(String::from("quality"));
        let block = spec(vec![f]);
        let base = table(&[("quality", int(85)), ("page", text("fit"))]);
        let out = block
            .resolve_params(&base, &table(&[("quality", int(60))]))
            .unwrap();
        assert_eq!(out.get("quality"), Some(&int(60)));
        assert_eq!(out.get("page"), Some(&text("fit")));
        assert_eq!(out.get("quality_option"), None);
        let out = block
            .resolve_params(&base, &table(&[("quality_option", int(70))]))
            .unwrap();
        assert_eq!(out.get("quality"), Some(&int(70)));
        assert_eq!(
            block
                .resolve_params(&base, &table(&[("nope", int(1))]))
                .unwrap_err(),
            OptionsError::Unknown(String::from("nope"))
        );
    }

    #[test]
    fn params_schema_describes_every_kind() {
        let mut quality = field("quality", Kind::Int);
        quality.label = label("Quality");
        quality.min = Some(1.0);
        quality.max = Some(100.0);
        quality.default = Some(int(85));
        quality.unit = Some(String::from("%"));

        let mut preset = field("preset", Kind::Enum);
        preset.default = Some(text("ebook"));
        preset.values.push(Choice {
            id: String::from("ebook"),
            label: label("Ebook"),
        });
        preset.values.push(Choice {
            id: String::from("screen"),
            label: label("Screen"),
        });

        let mut size = field("size", Kind::PxPair);
        size.default = Some(text("1920x1080"));

        let mut ratio = field("ratio", Kind::Float);
        ratio.min = Some(0.5);
        ratio.step = Some(0.1);

        let mut flag = field("flag", Kind::Bool);
        flag.default = Some(toml::Value::Boolean(true));

        let block = spec(vec![
            quality,
            preset,
            size,
            ratio,
            flag,
            field("name", Kind::String),
        ]);
        let schema = block.params_schema();
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["additionalProperties"], serde_json::json!(false));
        assert_eq!(schema["properties"].as_object().unwrap().len(), 6);

        let q = &schema["properties"]["quality"];
        assert_eq!(q["type"], "integer");
        assert_eq!(q["minimum"], 1);
        assert_eq!(q["maximum"], 100);
        assert_eq!(q["default"], 85);
        assert_eq!(q["description"], "Quality (%)");

        let p = &schema["properties"]["preset"];
        assert_eq!(p["type"], "string");
        assert_eq!(p["enum"], serde_json::json!(["ebook", "screen"]));

        let z = &schema["properties"]["size"];
        assert_eq!(z["type"], "string");
        assert_eq!(z["pattern"], PX_PAIR_PATTERN);
        assert_eq!(z["default"], "1920x1080");

        let r = &schema["properties"]["ratio"];
        assert_eq!(r["type"], "number");
        assert_eq!(r["minimum"], 0.5);

        let f = &schema["properties"]["flag"];
        assert_eq!(f["type"], "boolean");
        assert_eq!(f["default"], serde_json::json!(true));

        let n = &schema["properties"]["name"];
        assert_eq!(n["type"], "string");
        assert_eq!(n["description"], "name");
    }

    #[test]
    fn schema_keys_follow_maps_to() {
        let mut f = field("quality_option", Kind::Int);
        f.maps_to = Some(String::from("quality"));
        let schema = spec(vec![f]).params_schema();
        let props = schema["properties"].as_object().unwrap();
        assert!(props.contains_key("quality"));
        assert!(!props.contains_key("quality_option"));
    }

    #[test]
    fn param_key_defaults_to_the_field_id() {
        assert_eq!(field("scale", Kind::Percent).param_key(), "scale");
    }

    /// Two fields where one field id is another field's param key:
    /// a maps to b, b maps to c.
    fn chained() -> OptionsSpec {
        let mut a = field("a", Kind::Int);
        a.maps_to = Some(String::from("b"));
        let mut b = field("b", Kind::Int);
        b.maps_to = Some(String::from("c"));
        spec(vec![a, b])
    }

    #[test]
    fn resolve_gives_a_key_to_one_field_with_the_id_first() {
        let block = chained();
        // "b" is the id of field b and the param key of field a. The id wins,
        // so only field b is applied.
        let out = block
            .resolve(&toml::Table::new(), &table(&[("b", int(7))]))
            .unwrap();
        assert_eq!(out.get("c"), Some(&int(7)));
        assert_eq!(out.get("b"), None);
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn resolve_accepts_a_param_key_that_is_no_field_id() {
        let block = chained();
        // "c" matches no field id, so it finds field b by its param key.
        let out = block
            .resolve(&toml::Table::new(), &table(&[("c", int(5))]))
            .unwrap();
        assert_eq!(out.get("c"), Some(&int(5)));
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn resolve_params_prefers_a_param_key_over_a_field_id() {
        let block = chained();
        // "b" is field b's id and field a's param key. Params give the param
        // key precedence, so field a is the one applied.
        let out = block
            .resolve_params(&toml::Table::new(), &table(&[("b", int(5))]))
            .unwrap();
        assert_eq!(out.get("b"), Some(&int(5)));
        assert_eq!(out.get("c"), None);
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn one_field_is_applied_once() {
        let mut a = field("a", Kind::Int);
        a.maps_to = Some(String::from("p"));
        let block = spec(vec![a]);
        // The field id and the param key name the same field. The first key in
        // the table wins and the field is written once.
        let explicit = table(&[("a", int(3)), ("p", int(9))]);
        let out = block.resolve(&toml::Table::new(), &explicit).unwrap();
        assert_eq!(out.get("p"), Some(&int(3)));
        assert_eq!(out.len(), 1);
        let out = block
            .resolve_params(&toml::Table::new(), &explicit)
            .unwrap();
        assert_eq!(out.get("p"), Some(&int(3)));
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn an_unknown_key_inside_a_field_choice_or_preset_is_a_parse_error() {
        let field_typo = r#"
kind = "form"
[[field]]
id = "quality"
type = "int"
label = { ar = "ج", en = "Q" }
defualt = 85
"#;
        assert!(toml::from_str::<OptionsSpec>(field_typo).is_err());

        let choice_typo = r#"
kind = "form"
[[field]]
id = "mode"
type = "enum"
label = { ar = "م", en = "M" }
values = [{ id = "a", label = { ar = "أ", en = "A" }, nope = 1 }]
"#;
        assert!(toml::from_str::<OptionsSpec>(choice_typo).is_err());

        let preset_typo = r#"
kind = "form"
[[field]]
id = "mode"
type = "int"
label = { ar = "م", en = "M" }
[[preset]]
id = "p"
values = { mode = 1 }
nope = 2
"#;
        assert!(toml::from_str::<OptionsSpec>(preset_typo).is_err());
    }

    #[test]
    fn error_messages_read_as_documented() {
        assert_eq!(
            OptionsError::Unknown(String::from("nope")).to_string(),
            "unknown option `nope`"
        );
        assert_eq!(
            OptionsError::Type {
                name: String::from("quality"),
                expected: String::from("integer"),
            }
            .to_string(),
            "option `quality`: expected integer"
        );
        assert_eq!(
            OptionsError::Value {
                name: String::from("quality"),
                reason: String::from("below the minimum 1"),
            }
            .to_string(),
            "option `quality`: below the minimum 1"
        );
    }
}
