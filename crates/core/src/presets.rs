//! Named user presets (S-143), the foundation of the feature.
//!
//! A preset holds the option values of one action under a name the user
//! gives. The presets live in their own file, `presets.toml` next to
//! `config.toml`, with their own version and their own revision. Nothing
//! in this module changes [`crate::config::Config`] and nothing here turns
//! the remember-last-values option on.
//!
//! The store keeps what the user saved, not what the action happens to run
//! with:
//!
//! - The values are keyed by the field ids the action declares, not by the
//!   engine params. The two are mapped through the existing
//!   [`crate::options::OptionsSpec`], and the same resolver checks the
//!   types, the ranges and the enum values.
//! - A field whose param is not in the resolved table was cleared or was
//!   never set, so it is left out. The saved table holds only the values
//!   that are really in effect.
//! - The caller passes the effective form values after the configured base
//!   settings were applied. Save stores those displayed values as they are,
//!   while a named preset remains outside `Config` and never changes
//!   remember-last behavior.
//! - The fingerprint of the action travels with the preset. When the action
//!   changes, the preset asks for a review instead of being applied on
//!   trust.
//!
//! The store is written by one writer, the daemon. A save carries the
//! revision the writer read, so a file that changed in the meantime is not
//! overwritten. The store never grows a path, a command or a secret: only
//! numbers, booleans, enum ids and pixel pairs are saved.

use crate::action::{ActionDef, OutputMode, Registry};
use crate::options::{Field, Kind, OptionsSpec};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The store schema this build reads and writes.
pub const STORE_VERSION: u32 = 1;

/// The largest store file this build reads or writes, in bytes.
///
/// This is a proposed defensive policy, not a measurement: no user data
/// was measured. A bigger file is refused, never truncated.
pub const MAX_STORE_BYTES: usize = 1024 * 1024;

/// The largest number of presets one store holds.
///
/// A proposed defensive policy, not a measurement.
pub const MAX_PRESETS: usize = 256;

/// The largest number of saved fields one preset holds.
///
/// A proposed defensive policy, not a measurement.
pub const MAX_SAVED_FIELDS: usize = 64;

/// The longest preset name, in Unicode scalar values.
///
/// A proposed defensive policy, not a measurement.
pub const MAX_NAME_SCALARS: usize = 64;

/// The prefix of every preset id.
const ID_PREFIX: &str = "preset-";

/// One saved preset.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserPreset {
    /// The stable id of the preset, `preset-{counter}`. It never changes
    /// and it is never given out twice, not even after a delete.
    pub id: String,
    /// The name the user gave, as display text. It is never a path part.
    pub name: String,
    /// The action the preset belongs to.
    pub action_id: String,
    /// The fingerprint of the action when the preset was saved.
    pub schema_fingerprint: String,
    /// The saved values, keyed by field id.
    pub values: toml::Table,
}

/// The whole store.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresetDocument {
    /// The store schema version.
    pub version: u32,
    /// The counter of the next preset id. It only goes up, so a deleted id
    /// is never given out again.
    pub next_id: u64,
    /// The presets, in the order they were saved.
    pub presets: Vec<UserPreset>,
}

/// The content of the store file, as a digest.
///
/// A writer reads the file, keeps the revision and gives it back when it
/// saves. When the file on disk is not that revision, the save is refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreRevision(pub String);

/// What a preset can do with the action as it is now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PresetStatus {
    /// The preset fits the action and can be applied.
    Ready,
    /// The action changed since the preset was saved. The user has to
    /// look at it and save it again. The reason says what changed.
    NeedsReview(String),
    /// The preset cannot be used: the action is gone, it is not the
    /// built-in definition any more, or a saved value does not fit the
    /// current schema. The reason says why.
    Unavailable(String),
}

impl PresetStatus {
    /// True when the preset can be applied as it is.
    pub fn is_ready(&self) -> bool {
        matches!(self, PresetStatus::Ready)
    }
}

/// Why a preset operation did not work.
#[derive(Debug, thiserror::Error)]
pub enum PresetError {
    /// The name is empty, too long or holds a control character.
    #[error("invalid preset name: {reason}")]
    InvalidName { reason: String },
    /// A value, or the shape of the document, does not fit the action.
    #[error("invalid preset values: {reason}")]
    InvalidValues { reason: String },
    /// The action the preset belongs to is not in the built-in actions.
    #[error("the action `{action_id}` is not available")]
    MissingAction { action_id: String },
    /// The action is not one a preset may be saved for in this version.
    #[error("not supported for presets: {reason}")]
    UnsupportedScope { reason: String },
    /// The id counter of the store cannot give out another id.
    #[error("the preset id counter is exhausted: {reason}")]
    Exhausted { reason: String },
    /// The store file has a version this build does not know.
    #[error("the preset store has version {version}, this build reads an older one")]
    UnsupportedVersion { version: u32 },
    /// The store on disk is not the revision the writer read.
    #[error("the preset store changed: {reason}")]
    Conflict { reason: String },
    /// The store file cannot be read as a store of this version.
    #[error("the preset store is not readable: {reason}")]
    Corrupt { reason: String },
    /// The file could not be read or written.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// The store file, next to the settings file.
pub fn path() -> Option<PathBuf> {
    crate::paths::config_dir().map(|dir| dir.join("presets.toml"))
}

/// The revision of a store that is not on disk.
///
/// It is not the digest of an empty file, so a missing store and an empty
/// file never share a revision.
pub fn missing_revision() -> StoreRevision {
    StoreRevision(sha256_hex(b"kawaro-presets-missing"))
}

/// The revision of the store file as it is on disk now.
///
/// A missing file gives [`missing_revision`]. The digest covers the bytes
/// of the file, so any change, including a change that only touches the
/// formatting, gives another revision.
pub fn revision_of(path: &Path) -> Result<StoreRevision, PresetError> {
    Ok(match read_store_bytes(path)? {
        Some(bytes) => StoreRevision(sha256_hex(&bytes)),
        None => missing_revision(),
    })
}

/// The built-in actions, parsed once.
fn builtin_registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(Registry::builtin)
}

/// The built-in definition of an action id.
///
/// The built-in actions are the trusted source. A user action that takes
/// the id of a built-in action does not replace it here, so a preset can
/// never be bound to an arbitrary definition.
pub fn builtin_action(id: &str) -> Option<&'static ActionDef> {
    builtin_registry().get(id)
}

/// The id of the preset with the counter `n`.
fn preset_id(n: u64) -> String {
    format!("{ID_PREFIX}{n}")
}

/// The counter of a preset id, when the id has the expected shape.
fn id_counter(id: &str) -> Option<u64> {
    id.strip_prefix(ID_PREFIX)?.parse().ok()
}

/// The name as it is compared: trimmed and Unicode lowercased.
fn name_key(name: &str) -> String {
    name.trim().to_lowercase()
}

/// Checks a name and gives the trimmed name back.
///
/// The name is display text. It may hold any letter of any script, it is
/// never a path part and it is never used to build one.
fn check_name(name: &str) -> Result<String, PresetError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(PresetError::InvalidName {
            reason: String::from("the name is empty"),
        });
    }
    if trimmed.chars().count() > MAX_NAME_SCALARS {
        return Err(PresetError::InvalidName {
            reason: format!("the name is longer than {MAX_NAME_SCALARS} characters"),
        });
    }
    if let Some(c) = trimmed.chars().find(|c| c.is_control()) {
        return Err(PresetError::InvalidName {
            reason: format!("the name holds a control character (U+{:04X})", c as u32),
        });
    }
    Ok(trimmed.to_string())
}

/// True for a field kind a preset may store.
fn kind_is_savable(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Int | Kind::Percent | Kind::Px | Kind::PxPair | Kind::Enum | Kind::Bool | Kind::Float
    )
}

/// True for an action a preset may be saved for in this version: a
/// built-in image action, except the one that builds a PDF.
fn scope_allows(action: &ActionDef) -> bool {
    action.engine == "image" && action.op != "to-pdf" && action.output.mode != OutputMode::Combined
}

/// The reason an action is out of the saved scope.
fn scope_reason(action: &ActionDef) -> String {
    if action.engine != "image" {
        return format!("the action `{}` is not an image action", action.id);
    }
    if action.op == "to-pdf" || action.output.mode == OutputMode::Combined {
        return format!("the action `{}` writes one combined file", action.id);
    }
    String::new()
}

/// The option semantics of one field, without the translated labels.
#[derive(Clone, Debug, PartialEq)]
struct FieldSemantics {
    id: String,
    param: String,
    kind: Kind,
    min: Option<f64>,
    max: Option<f64>,
    step: Option<f64>,
    clears: Vec<String>,
    choices: Vec<String>,
}

/// The option semantics of one field.
fn field_semantics(field: &Field) -> FieldSemantics {
    let mut clears = field.clears.clone();
    clears.sort();
    let mut choices: Vec<String> = field.values.iter().map(|c| c.id.clone()).collect();
    choices.sort();
    FieldSemantics {
        id: field.id.clone(),
        param: field.param_key().to_string(),
        kind: field.kind,
        min: field.min,
        max: field.max,
        step: field.step,
        clears,
        choices,
    }
}

/// The option semantics of an action, sorted by field id.
fn action_fields(action: &ActionDef) -> Vec<FieldSemantics> {
    let Some(spec) = &action.options else {
        return Vec::new();
    };
    let mut out: Vec<FieldSemantics> = spec.fields.iter().map(field_semantics).collect();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// The parts of the output the action itself decides.
///
/// The output folder is a setting, not the action, so it is left out.
fn output_signature(action: &ActionDef) -> (OutputMode, &str, &str) {
    (
        action.output.mode,
        action.output.ext.as_str(),
        action.output.suffix.as_str(),
    )
}

/// True when two definitions describe the same action: the same engine, the
/// same operation, the same output and the same option fields.
///
/// The base params are not part of it: the settings move the base quality
/// of an image action, and that does not make a different action.
fn same_action(a: &ActionDef, b: &ActionDef) -> bool {
    a.engine == b.engine
        && a.op == b.op
        && output_signature(a) == output_signature(b)
        && action_fields(a) == action_fields(b)
}

/// The name of an output mode, as the store writes it.
fn output_mode_name(mode: OutputMode) -> &'static str {
    match mode {
        OutputMode::PerFile => "per-file",
        OutputMode::Combined => "combined",
    }
}

/// The name of a field kind, as the store writes it.
fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Percent => "percent",
        Kind::Px => "px",
        Kind::PxPair => "px-pair",
        Kind::Enum => "enum",
        Kind::Bool => "bool",
        Kind::Int => "int",
        Kind::Float => "float",
        Kind::String => "string",
    }
}

/// Writes a string with its length, so the canonical text cannot be
/// changed by a value that holds a separator.
fn put_string(out: &mut String, text: &str) {
    out.push_str(&text.len().to_string());
    out.push(':');
    out.push_str(text);
}

/// Writes a number, or nothing when there is none.
fn put_number(out: &mut String, value: Option<f64>) {
    match value {
        None => out.push_str("none"),
        Some(n) => {
            out.push_str("num");
            out.push_str(&format!("{n:?}"));
        }
    }
}

/// Writes a value in a canonical shape.
fn put_value(out: &mut String, value: &toml::Value) {
    match value {
        toml::Value::String(text) => {
            out.push_str("str");
            put_string(out, text);
        }
        toml::Value::Integer(n) => {
            out.push_str("int");
            out.push_str(&n.to_string());
        }
        toml::Value::Float(n) => {
            out.push_str("float");
            out.push_str(&format!("{n:?}"));
        }
        toml::Value::Boolean(b) => {
            out.push_str("bool");
            out.push_str(if *b { "1" } else { "0" });
        }
        toml::Value::Datetime(d) => {
            out.push_str("date");
            put_string(out, &d.to_string());
        }
        toml::Value::Array(items) => {
            out.push_str("list");
            out.push_str(&items.len().to_string());
            for item in items {
                put_value(out, item);
            }
        }
        toml::Value::Table(table) => {
            out.push_str("table");
            out.push_str(&table.len().to_string());
            let mut keys: Vec<&String> = table.keys().collect();
            keys.sort();
            for key in keys {
                put_string(out, key);
                put_value(out, &table[key]);
            }
        }
    }
}

/// The canonical text of the parts of an action that decide what it does:
/// the engine, the operation, the output, the base params and the option
/// fields.
///
/// The translated labels, the units, the readout, the defaults and the
/// output folder are not part of it: they do not change what the action
/// does.
fn canonical_semantics(action: &ActionDef) -> String {
    let mut out = String::from("kawaro-presets-semantics-v1\n");
    put_string(&mut out, "engine");
    put_string(&mut out, &action.engine);
    put_string(&mut out, "op");
    put_string(&mut out, &action.op);
    put_string(&mut out, "output.mode");
    put_string(&mut out, output_mode_name(action.output.mode));
    put_string(&mut out, "output.ext");
    put_string(&mut out, &action.output.ext);
    put_string(&mut out, "output.suffix");
    put_string(&mut out, &action.output.suffix);
    put_string(&mut out, "params");
    let mut keys: Vec<&String> = action.params.keys().collect();
    keys.sort();
    for key in keys {
        put_string(&mut out, key);
        put_value(&mut out, &action.params[key]);
    }
    put_string(&mut out, "fields");
    for field in action_fields(action) {
        put_string(&mut out, &field.id);
        put_string(&mut out, &field.param);
        put_string(&mut out, kind_name(field.kind));
        put_number(&mut out, field.min);
        put_number(&mut out, field.max);
        put_number(&mut out, field.step);
        put_string(&mut out, &field.clears.len().to_string());
        for clear in &field.clears {
            put_string(&mut out, clear);
        }
        put_string(&mut out, &field.choices.len().to_string());
        for choice in &field.choices {
            put_string(&mut out, choice);
        }
    }
    out
}

/// The canonical text of the parts of an action that decide whether a
/// preset may be applied at all: the engine, the operation and the output.
fn canonical_compatibility(action: &ActionDef) -> String {
    let mut out = String::from("kawaro-presets-compatibility-v1\n");
    put_string(&mut out, "engine");
    put_string(&mut out, &action.engine);
    put_string(&mut out, "op");
    put_string(&mut out, &action.op);
    put_string(&mut out, "output.mode");
    put_string(&mut out, output_mode_name(action.output.mode));
    put_string(&mut out, "output.ext");
    put_string(&mut out, &action.output.ext);
    put_string(&mut out, "output.suffix");
    put_string(&mut out, &action.output.suffix);
    out
}

/// SHA-256 of the bytes, as lowercase hex.
fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The two digests of a fingerprint.
struct Fingerprint {
    compatibility: String,
    full: String,
}

/// The fingerprint of the action `action` describes.
///
/// The trusted built-in definition wins when the id belongs to a built-in
/// action, so a configured base quality or a chosen output folder does not
/// change the fingerprint.
///
/// The string holds two digests. The first one covers the engine, the
/// operation and the output: a change there makes a preset unavailable,
/// because the preset was saved for another action. The second one covers
/// the whole semantics, the base params and the option fields: a change
/// there asks the user for a review.
pub fn fingerprint(action: &ActionDef) -> String {
    let source = builtin_action(&action.id).unwrap_or(action);
    let compatibility = sha256_hex(canonical_compatibility(source).as_bytes());
    let full = sha256_hex(canonical_semantics(source).as_bytes());
    format!("{compatibility}-{full}")
}

/// The two digests of a fingerprint, when the string has the expected shape.
fn parse_fingerprint(value: &str) -> Option<Fingerprint> {
    let (compatibility, full) = value.split_once('-')?;
    let hex = |text: &str| text.len() == 64 && text.chars().all(|c| c.is_ascii_hexdigit());
    if !hex(compatibility) || !hex(full) {
        return None;
    }
    Some(Fingerprint {
        compatibility: compatibility.to_ascii_lowercase(),
        full: full.to_ascii_lowercase(),
    })
}

/// The effective values of the declared fields, keyed by field id.
///
/// `resolved` is the table the option resolver returned for the displayed
/// form values after the configured base settings were applied. A field
/// whose param is not in the table was cleared or was never set, so it is
/// left out. Every param key belongs to one field, so the mapping happens
/// once. Saving those resolved values never writes `Config` or activates
/// remember-last behavior.
pub fn field_values(spec: &OptionsSpec, resolved: &toml::Table) -> toml::Table {
    let mut out = toml::Table::new();
    for field in &spec.fields {
        let Some(value) = resolved.get(field.param_key()) else {
            continue;
        };
        out.insert(field.id.clone(), value.clone());
    }
    out
}

/// Rejects two fields that clear each other: the saved table must not be
/// able to activate both.
fn check_clears(spec: &OptionsSpec, values: &toml::Table) -> Result<(), PresetError> {
    for field in &spec.fields {
        for cleared in &field.clears {
            let Some(other) = spec
                .fields
                .iter()
                .find(|f| f.param_key() == cleared.as_str() || &f.id == cleared)
            else {
                continue;
            };
            let both_active = values.contains_key(&field.id) && values.contains_key(&other.id);
            if both_active {
                return Err(PresetError::InvalidValues {
                    reason: format!(
                        "the options `{}` and `{}` exclude each other",
                        field.id, other.id
                    ),
                });
            }
        }
    }
    Ok(())
}

/// Rejects a float that is not finite.
///
/// The TOML format allows `nan` and `inf`, and the option validator does
/// not see them as out of range, so they are refused here.
fn check_finite(key: &str, value: &toml::Value) -> Result<(), String> {
    if let toml::Value::Float(n) = value
        && !n.is_finite()
    {
        return Err(format!("the option `{key}` is not a finite number"));
    }
    Ok(())
}

/// Checks the saved values against the current schema of the action.
///
/// The types, the ranges and the enum values are checked by the existing
/// option resolver, so there is one validator and not two.
fn check_values(
    spec: &OptionsSpec,
    action: &ActionDef,
    values: &toml::Table,
) -> Result<(), String> {
    if values.len() > MAX_SAVED_FIELDS {
        return Err(format!(
            "the preset holds more than {MAX_SAVED_FIELDS} fields"
        ));
    }
    for (key, value) in values {
        let Some(field) = spec.fields.iter().find(|f| &f.id == key) else {
            return Err(format!("the option `{key}` is not a field of the action"));
        };
        if !kind_is_savable(field.kind) {
            return Err(format!("the option `{key}` is free text and is not stored"));
        }
        check_finite(key, value)?;
    }
    spec.resolve(&action.params, values)
        .map_err(|e| e.to_string())?;
    check_clears(spec, values).map_err(|e| e.to_string())
}

/// The effective values of the declared fields, ready to be saved.
///
/// `resolved` is the table the option resolver returned for the form
/// values, before the settings moved the base quality and before the
/// output folder was set. Only the built-in image actions, except the one
/// that builds a PDF, and only the numeric, boolean, enum and pixel pair
/// fields are saved. A path, an output name, a command or a secret is not
/// a field kind, so none of them can be stored.
pub fn canonical_values(
    action: &ActionDef,
    resolved: &toml::Table,
) -> Result<toml::Table, PresetError> {
    if action.options.is_none() {
        return Err(PresetError::UnsupportedScope {
            reason: format!("the action `{}` takes no options", action.id),
        });
    }
    if !scope_allows(action) {
        return Err(PresetError::UnsupportedScope {
            reason: scope_reason(action),
        });
    }
    let Some(trusted) = builtin_action(&action.id) else {
        return Err(PresetError::MissingAction {
            action_id: action.id.clone(),
        });
    };
    if !same_action(action, trusted) {
        return Err(PresetError::UnsupportedScope {
            reason: format!("the action `{}` is not the built-in definition", action.id),
        });
    }
    let Some(trusted_spec) = trusted.options.as_ref() else {
        return Err(PresetError::UnsupportedScope {
            reason: format!("the action `{}` takes no options", trusted.id),
        });
    };
    let values = field_values(trusted_spec, resolved);
    if let Err(reason) = check_values(trusted_spec, trusted, &values) {
        return Err(PresetError::InvalidValues { reason });
    }
    Ok(values)
}

/// The state of a preset against the action as it is now.
///
/// `action` is the action the caller found in the registry, when it found
/// one. A missing action, an action that is not the built-in definition
/// any more, a value that does not fit the current schema or a fingerprint
/// this build cannot read make the preset unavailable. A changed schema
/// with values that still fit asks for a review.
pub fn validate_preset(preset: &UserPreset, action: Option<&ActionDef>) -> PresetStatus {
    let Some(trusted) = builtin_action(&preset.action_id) else {
        return PresetStatus::Unavailable(format!(
            "the action `{}` is not available",
            preset.action_id
        ));
    };
    let Some(found) = action else {
        return PresetStatus::Unavailable(format!(
            "the action `{}` is not in the current registry",
            preset.action_id
        ));
    };
    if found.id != preset.action_id || !same_action(found, trusted) {
        return PresetStatus::Unavailable(format!(
            "the action `{}` is not the built-in definition",
            preset.action_id
        ));
    }
    if !scope_allows(trusted) {
        return PresetStatus::Unavailable(scope_reason(trusted));
    }
    let Some(spec) = trusted.options.as_ref() else {
        return PresetStatus::Unavailable(format!("the action `{}` takes no options", trusted.id));
    };
    if let Err(reason) = check_values(spec, trusted, &preset.values) {
        return PresetStatus::Unavailable(reason);
    }
    let Some(saved) = parse_fingerprint(&preset.schema_fingerprint) else {
        return PresetStatus::Unavailable(String::from("the saved fingerprint is not readable"));
    };
    let Some(current) = parse_fingerprint(&fingerprint(trusted)) else {
        return PresetStatus::Unavailable(String::from(
            "the fingerprint of the action is not readable",
        ));
    };
    if saved.compatibility != current.compatibility {
        return PresetStatus::Unavailable(String::from(
            "the action changed: the engine, the operation or the output is not the one it was saved for",
        ));
    }
    if saved.full != current.full {
        return PresetStatus::NeedsReview(String::from(
            "the options of the action changed since the preset was saved",
        ));
    }
    PresetStatus::Ready
}

/// The params table the action runs with when the preset is applied.
///
/// The preset has to be ready. A preset that asks for a review is not
/// applied on trust: the user has to look at it and save it again.
pub fn apply_preset(preset: &UserPreset, action: &ActionDef) -> Result<toml::Table, PresetError> {
    match validate_preset(preset, Some(action)) {
        PresetStatus::Ready => {}
        PresetStatus::NeedsReview(reason) => {
            return Err(PresetError::InvalidValues {
                reason: format!("the preset needs a review: {reason}"),
            });
        }
        PresetStatus::Unavailable(reason) => {
            return Err(PresetError::InvalidValues { reason });
        }
    }
    let Some(spec) = action.options.as_ref() else {
        return Err(PresetError::UnsupportedScope {
            reason: format!("the action `{}` takes no options", action.id),
        });
    };
    match spec.resolve(&action.params, &preset.values) {
        Ok(params) => Ok(params),
        Err(e) => Err(PresetError::InvalidValues {
            reason: e.to_string(),
        }),
    }
}

impl PresetDocument {
    /// A new empty store.
    pub fn empty() -> Self {
        PresetDocument {
            version: STORE_VERSION,
            next_id: 1,
            presets: Vec::new(),
        }
    }

    /// The preset with this id.
    pub fn get(&self, id: &str) -> Option<&UserPreset> {
        self.presets.iter().find(|p| p.id == id)
    }

    /// The presets of one action, in the order of the store.
    pub fn for_action(&self, action_id: &str) -> Vec<&UserPreset> {
        self.presets
            .iter()
            .filter(|p| p.action_id == action_id)
            .collect()
    }

    /// The id the next preset gets.
    pub fn next_preset_id(&self) -> Result<String, PresetError> {
        if self.next_id == 0 {
            return Err(PresetError::Exhausted {
                reason: String::from("the counter is zero"),
            });
        }
        Ok(preset_id(self.next_id))
    }

    /// True when no preset of this action carries the name.
    fn name_is_free(&self, action_id: &str, name: &str, except_id: Option<&str>) -> bool {
        let key = name_key(name);
        !self.presets.iter().any(|p| {
            p.action_id == action_id && name_key(&p.name) == key && Some(p.id.as_str()) != except_id
        })
    }

    /// Adds a preset for `action` under `name`, with the values the form
    /// holds.
    ///
    /// The id is allocated here, so a caller never writes one. The name is
    /// checked, the values are checked against the current schema of the
    /// action and the fingerprint of the action is written with the
    /// preset.
    pub fn insert(
        &mut self,
        action: &ActionDef,
        name: &str,
        values: &toml::Table,
    ) -> Result<UserPreset, PresetError> {
        check_shape(self)?;
        if self.presets.len() >= MAX_PRESETS {
            return Err(PresetError::InvalidValues {
                reason: format!("the store already holds {MAX_PRESETS} presets"),
            });
        }
        let name = check_name(name)?;
        if !scope_allows(action) {
            return Err(PresetError::UnsupportedScope {
                reason: scope_reason(action),
            });
        }
        let Some(trusted) = builtin_action(&action.id) else {
            return Err(PresetError::MissingAction {
                action_id: action.id.clone(),
            });
        };
        if !same_action(action, trusted) {
            return Err(PresetError::UnsupportedScope {
                reason: format!("the action `{}` is not the built-in definition", action.id),
            });
        }
        let Some(spec) = trusted.options.as_ref() else {
            return Err(PresetError::UnsupportedScope {
                reason: format!("the action `{}` takes no options", trusted.id),
            });
        };
        if let Err(reason) = check_values(spec, trusted, values) {
            return Err(PresetError::InvalidValues { reason });
        }
        if !self.name_is_free(&trusted.id, &name, None) {
            return Err(PresetError::InvalidName {
                reason: format!(
                    "the action `{}` already has a preset named `{name}`",
                    trusted.id
                ),
            });
        }
        let next = self.next_id.checked_add(1).ok_or(PresetError::Exhausted {
            reason: String::from("the counter cannot give out another id"),
        })?;
        let preset = UserPreset {
            id: preset_id(self.next_id),
            name,
            action_id: trusted.id.clone(),
            schema_fingerprint: fingerprint(trusted),
            values: values.clone(),
        };
        self.presets.push(preset.clone());
        self.next_id = next;
        Ok(preset)
    }

    /// Gives a preset another name. The name stays free inside the action.
    pub fn rename(&mut self, id: &str, name: &str) -> Result<(), PresetError> {
        let name = check_name(name)?;
        let index =
            self.presets
                .iter()
                .position(|p| p.id == id)
                .ok_or(PresetError::InvalidValues {
                    reason: format!("the preset `{id}` is not in the store"),
                })?;
        let action_id = self.presets[index].action_id.clone();
        if !self.name_is_free(&action_id, &name, Some(id)) {
            return Err(PresetError::InvalidName {
                reason: format!("the action `{action_id}` already has a preset named `{name}`"),
            });
        }
        self.presets[index].name = name;
        Ok(())
    }

    /// Takes a preset out of the store.
    ///
    /// The id counter does not go back, so the id is never given out
    /// again.
    pub fn remove(&mut self, id: &str) -> Result<UserPreset, PresetError> {
        let index =
            self.presets
                .iter()
                .position(|p| p.id == id)
                .ok_or(PresetError::InvalidValues {
                    reason: format!("the preset `{id}` is not in the store"),
                })?;
        Ok(self.presets.remove(index))
    }

    /// Saves the values the user reviewed into a preset and writes the
    /// fingerprint of the action as it is now.
    ///
    /// This is the way out of a review: the values are checked against the
    /// current schema again, and the preset gets the fingerprint of the
    /// action it belongs to.
    pub fn refresh(
        &mut self,
        id: &str,
        action: &ActionDef,
        values: &toml::Table,
    ) -> Result<(), PresetError> {
        let index =
            self.presets
                .iter()
                .position(|p| p.id == id)
                .ok_or(PresetError::InvalidValues {
                    reason: format!("the preset `{id}` is not in the store"),
                })?;
        let Some(trusted) = builtin_action(&action.id) else {
            return Err(PresetError::MissingAction {
                action_id: action.id.clone(),
            });
        };
        if self.presets[index].action_id != action.id {
            return Err(PresetError::InvalidValues {
                reason: format!(
                    "the preset `{id}` belongs to the action `{}`",
                    self.presets[index].action_id
                ),
            });
        }
        if !same_action(action, trusted) {
            return Err(PresetError::UnsupportedScope {
                reason: format!("the action `{}` is not the built-in definition", action.id),
            });
        }
        let Some(spec) = trusted.options.as_ref() else {
            return Err(PresetError::UnsupportedScope {
                reason: format!("the action `{}` takes no options", trusted.id),
            });
        };
        if let Err(reason) = check_values(spec, trusted, values) {
            return Err(PresetError::InvalidValues { reason });
        }
        self.presets[index].values = values.clone();
        self.presets[index].schema_fingerprint = fingerprint(trusted);
        Ok(())
    }
}

/// Checks the shape of a store: the version, the counters, the ids and the
/// names.
///
/// The values of a preset are not checked here: a preset whose action is
/// gone stays in the store, and the caller sees it as unavailable.
fn check_shape(document: &PresetDocument) -> Result<(), PresetError> {
    if document.version != STORE_VERSION {
        return Err(PresetError::UnsupportedVersion {
            version: document.version,
        });
    }
    if document.presets.len() > MAX_PRESETS {
        return Err(PresetError::InvalidValues {
            reason: format!("the store holds more than {MAX_PRESETS} presets"),
        });
    }
    if document.next_id == 0 {
        return Err(PresetError::InvalidValues {
            reason: String::from("the preset id counter is zero"),
        });
    }
    let mut ids: BTreeSet<&str> = BTreeSet::new();
    let mut names: BTreeSet<(&str, String)> = BTreeSet::new();
    let mut highest = 0u64;
    for preset in &document.presets {
        let Some(counter) = id_counter(&preset.id) else {
            return Err(PresetError::InvalidValues {
                reason: format!("the preset id `{}` is not `preset-{{counter}}`", preset.id),
            });
        };
        if counter == 0 || preset.id != preset_id(counter) {
            return Err(PresetError::InvalidValues {
                reason: format!("the preset id `{}` is not canonical", preset.id),
            });
        }
        if !ids.insert(preset.id.as_str()) {
            return Err(PresetError::InvalidValues {
                reason: format!("the preset id `{}` is used twice", preset.id),
            });
        }
        highest = highest.max(counter);
        let name = check_name(&preset.name)?;
        if !names.insert((preset.action_id.as_str(), name_key(&name))) {
            return Err(PresetError::InvalidName {
                reason: format!(
                    "the action `{}` has two presets named `{name}`",
                    preset.action_id
                ),
            });
        }
        if parse_fingerprint(&preset.schema_fingerprint).is_none() {
            return Err(PresetError::InvalidValues {
                reason: format!(
                    "the preset `{}` has a fingerprint this build cannot read",
                    preset.id
                ),
            });
        }
    }
    if document.next_id <= highest {
        return Err(PresetError::InvalidValues {
            reason: format!(
                "the preset id counter {} is not above the ids in the store ({highest})",
                document.next_id
            ),
        });
    }
    Ok(())
}

/// Validate new or changed payloads, preserving unavailable saved records.
/// Renaming or deleting an old preset does not silently repair its values.
fn check_changed_values(
    document: &PresetDocument,
    previous: Option<&PresetDocument>,
) -> Result<(), PresetError> {
    for preset in &document.presets {
        let unchanged = previous
            .and_then(|old| old.get(&preset.id))
            .is_some_and(|old| {
                old.action_id == preset.action_id
                    && old.schema_fingerprint == preset.schema_fingerprint
                    && old.values == preset.values
            });
        if unchanged {
            continue;
        }
        let Some(action) = builtin_action(&preset.action_id) else {
            return Err(PresetError::MissingAction {
                action_id: preset.action_id.clone(),
            });
        };
        match validate_preset(preset, Some(action)) {
            PresetStatus::Ready => {}
            PresetStatus::NeedsReview(reason) | PresetStatus::Unavailable(reason) => {
                return Err(PresetError::InvalidValues { reason });
            }
        }
    }
    Ok(())
}

/// Reads the bytes of a store and checks them, without touching the disk.
///
/// The version is read from the table before the document is built, so a
/// file of a version this build does not know is never read as a store of
/// this version.
fn parse_store(bytes: &[u8]) -> Result<PresetDocument, PresetError> {
    if bytes.len() > MAX_STORE_BYTES {
        return Err(PresetError::Corrupt {
            reason: format!("the store is larger than {MAX_STORE_BYTES} bytes"),
        });
    }
    if bytes.is_empty() {
        return Err(PresetError::Corrupt {
            reason: String::from("the existing store is empty and has no version"),
        });
    }
    let text = std::str::from_utf8(bytes).map_err(|_| PresetError::Corrupt {
        reason: String::from("the store is not UTF-8 text"),
    })?;
    let table: toml::Table = toml::from_str(text).map_err(|e| PresetError::Corrupt {
        reason: e.to_string(),
    })?;
    let version = match table.get("version") {
        None => {
            return Err(PresetError::Corrupt {
                reason: String::from("the store has no version"),
            });
        }
        Some(value) => {
            let number = value.as_integer().ok_or(PresetError::Corrupt {
                reason: String::from("the store version is not a whole number"),
            })?;
            if number < 0 {
                return Err(PresetError::Corrupt {
                    reason: String::from("the store version is negative"),
                });
            }
            u32::try_from(number).map_err(|_| PresetError::Corrupt {
                reason: format!("the store version {number} exceeds the supported range"),
            })?
        }
    };
    if version > STORE_VERSION {
        return Err(PresetError::UnsupportedVersion { version });
    }
    if version < STORE_VERSION {
        return Err(PresetError::Corrupt {
            reason: format!(
                "the store has version {version}, this build reads version {STORE_VERSION}"
            ),
        });
    }
    let document: PresetDocument =
        table
            .try_into()
            .map_err(|e: toml::de::Error| PresetError::Corrupt {
                reason: e.to_string(),
            })?;
    check_shape(&document).map_err(|e| PresetError::Corrupt {
        reason: e.to_string(),
    })?;
    Ok(document)
}

/// Reads the store.
///
/// A missing file is a new empty store. An existing empty file is corrupt.
/// A file of a
/// version this build does not know is refused before it is read as a
/// document, so it is never read as a store of this version. A broken file
/// is refused as well, and nothing on disk is changed.
pub fn load(path: &Path) -> Result<(PresetDocument, StoreRevision), PresetError> {
    let Some(bytes) = read_store_bytes(path)? else {
        return Ok((PresetDocument::empty(), missing_revision()));
    };
    let revision = StoreRevision(sha256_hex(&bytes));
    let document = parse_store(&bytes).map_err(|e| match e {
        PresetError::UnsupportedVersion { version } => PresetError::UnsupportedVersion { version },
        other => PresetError::Corrupt {
            reason: other.to_string(),
        },
    })?;
    Ok((document, revision))
}

/// The bytes of the store, as they are written.
fn serialize(document: &PresetDocument) -> Result<Vec<u8>, PresetError> {
    let text = toml::to_string_pretty(document).map_err(|e| PresetError::InvalidValues {
        reason: e.to_string(),
    })?;
    Ok(text.into_bytes())
}

/// The bytes of the store file, when it is there.
fn read_store_bytes(path: &Path) -> Result<Option<Vec<u8>>, PresetError> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_file() || metadata.len() > MAX_STORE_BYTES as u64 {
        return Err(PresetError::Corrupt {
            reason: String::from("the store is not a regular file within the size limit"),
        });
    }
    let file = std::fs::File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(PresetError::Corrupt {
            reason: String::from("the store is not a regular file"),
        });
    }
    let mut bytes = Vec::new();
    file.take(MAX_STORE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_STORE_BYTES {
        return Err(PresetError::Corrupt {
            reason: String::from("the store exceeded the size limit while reading"),
        });
    }
    Ok(Some(bytes))
}

/// A temporary file in the same folder as the store.
///
/// The name is created with `create_new`, so another writer never takes
/// the same name, and the file holds the mode of a private file on Unix.
fn create_temp(path: &Path) -> Result<(PathBuf, std::fs::File), PresetError> {
    let dir = path
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let stem = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| String::from("presets"));
    for attempt in 0..64u32 {
        let candidate = dir.join(format!("{stem}.{}.{attempt}.tmp", std::process::id()));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&candidate) {
            Ok(file) => return Ok((candidate, file)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(PresetError::Io(e)),
        }
    }
    Err(PresetError::Io(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "no free temporary file name for the preset store",
    )))
}

/// Writes the store.
///
/// The daemon is the only writer of the file. `expected` is the revision
/// the writer read: when the file on disk is not that revision, the save
/// is refused and nothing is written. A file that cannot be read as a
/// store of this version is never overwritten.
///
/// The revision is compared first, before the document is looked at. A
/// writer that read an older file is told that the store changed, and not
/// that its own document does not fit: the file moved on, so the document
/// is compared against what is on disk only when both are the same.
///
/// The bytes go to a temporary file in the same folder, are flushed and
/// synced, and the file is read again before the temporary file is moved
/// over it. A change made by another program between that last read and
/// the move is not caught: this is a compare and replace, not a
/// transaction, and no shared lock is taken.
pub fn save(
    path: &Path,
    expected: &StoreRevision,
    document: &PresetDocument,
) -> Result<StoreRevision, PresetError> {
    check_shape(document)?;
    let bytes = serialize(document)?;
    if bytes.len() > MAX_STORE_BYTES {
        return Err(PresetError::InvalidValues {
            reason: format!("the store would be larger than {MAX_STORE_BYTES} bytes"),
        });
    }
    let current = read_store_bytes(path)?;
    let current_revision = match &current {
        Some(existing) => StoreRevision(sha256_hex(existing)),
        None => missing_revision(),
    };
    // The revision comes first: a stale writer is refused with a conflict
    // before the counter of the document is compared with the one on disk.
    if &current_revision != expected {
        return Err(PresetError::Conflict {
            reason: format!(
                "the store on disk is not the revision that was read ({})",
                current_revision.0
            ),
        });
    }
    let previous = current.as_deref().map(parse_store).transpose()?;
    if let Some(previous) = &previous
        && document.next_id < previous.next_id
    {
        return Err(PresetError::InvalidValues {
            reason: String::from("the preset id counter cannot move backwards"),
        });
    }
    check_changed_values(document, previous.as_ref())?;
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    let (temp, mut file) = create_temp(path)?;
    let written = (|| -> std::io::Result<()> {
        file.write_all(&bytes)?;
        file.flush()?;
        file.sync_all()?;
        Ok(())
    })();
    drop(file);
    if let Err(e) = written {
        let _ = std::fs::remove_file(&temp);
        return Err(PresetError::Io(e));
    }
    let again = read_store_bytes(path).inspect_err(|_| {
        let _ = std::fs::remove_file(&temp);
    })?;
    let again_revision = match &again {
        Some(existing) => StoreRevision(sha256_hex(existing)),
        None => missing_revision(),
    };
    if &again_revision != expected {
        let _ = std::fs::remove_file(&temp);
        return Err(PresetError::Conflict {
            reason: String::from("the store changed while it was written"),
        });
    }
    std::fs::rename(&temp, path).map_err(|error| {
        let _ = std::fs::remove_file(&temp);
        PresetError::Io(error)
    })?;
    Ok(StoreRevision(sha256_hex(&bytes)))
}
