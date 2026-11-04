//! Actions are data. Each `actions/*.toml` file holds `[[action]]` tables;
//! the hive is built from whatever matches the current selection.
//!
//! ```toml
//! [[action]]
//! id = "image.to-webp"
//! engine = "image"
//! op = "convert"
//! ring = 1                 # 1 = inner ring, 2 = outer ring
//! order = 10               # position inside the ring, ascending
//! accepts = ["image"]      # MediaKind values
//! skip_ext = ["webp"]      # hide when every selected file already has one of these
//! min_files = 1
//! max_files = 0            # 0 = no limit
//! output = { mode = "per-file", ext = "webp" }   # or mode = "combined"
//! code = { ar = "WEBP", en = "WEBP" }            # short text inside the cell
//! label = { ar = "تحويل إلى WebP", en = "Convert to WebP" }
//! icon = "image"           # name from the hive icon set (crates/ui/icons/);
//!                          # empty or left out means the code shows alone
//! [action.params]
//! quality = 85
//! ```

use crate::media::MediaKind;
use crate::options::{OptionsError, OptionsSpec};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputMode {
    /// One task and one output per input file.
    PerFile,
    /// One task for the whole selection, one output named after the first input.
    Combined,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputSpec {
    pub mode: OutputMode,
    /// Output extension without the dot. Empty keeps the input extension.
    #[serde(default)]
    pub ext: String,
    /// Added before the extension, for example "-small" gives `photo-small.jpg`.
    #[serde(default)]
    pub suffix: String,
    /// Subfolder, relative to the first input's folder, that receives the
    /// output. Empty means next to the input. Set by the daemon from the
    /// user's settings; action files normally leave it out.
    #[serde(default)]
    pub dir: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Localized {
    pub ar: String,
    pub en: String,
}

impl Localized {
    pub fn get(&self, lang: crate::i18n::Lang) -> &str {
        match lang {
            crate::i18n::Lang::Ar => &self.ar,
            crate::i18n::Lang::En => &self.en,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActionDef {
    pub id: String,
    pub engine: String,
    pub op: String,
    pub ring: u8,
    #[serde(default)]
    pub order: i32,
    pub accepts: Vec<MediaKind>,
    #[serde(default)]
    pub skip_ext: Vec<String>,
    #[serde(default = "one")]
    pub min_files: usize,
    #[serde(default)]
    pub max_files: usize,
    pub output: OutputSpec,
    pub code: Localized,
    pub label: Localized,
    /// Icon drawn above the code, a name from the fixed icon set in
    /// `crates/ui/icons/` (for example "image", "pdf", "rotate"). Empty means
    /// no icon: the cell shows the code alone.
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub params: toml::Table,
    /// Typed options the engine takes (S-117); absent when the action has
    /// none, which is most of them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<OptionsSpec>,
}

fn one() -> usize {
    1
}

impl ActionDef {
    /// This action with the explicit option values applied to a copy of its
    /// params (S-120). The registry entry is never changed: the caller gets a
    /// new action for one run.
    ///
    /// An action without `[action.options]` takes no explicit value, so its
    /// first key is reported as unknown rather than dropped.
    pub fn with_params(&self, explicit: &toml::Table) -> Result<ActionDef, OptionsError> {
        let params = match &self.options {
            Some(options) => options.resolve_params(&self.params, explicit)?,
            None => match explicit.iter().next() {
                Some((key, _)) => return Err(OptionsError::Unknown(key.clone())),
                None => self.params.clone(),
            },
        };
        Ok(ActionDef {
            params,
            ..self.clone()
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("{file}: {reason}")]
    Parse { file: String, reason: String },
    #[error("duplicate action id `{0}`")]
    Duplicate(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Clone, Debug, Default)]
pub struct Registry {
    actions: Vec<ActionDef>,
}

impl Registry {
    /// Parses one TOML document with `[[action]]` tables. `name` is used in errors.
    pub fn add_toml(&mut self, name: &str, text: &str) -> Result<(), RegistryError> {
        crate::action_impl::add_toml(&mut self.actions, name, text)
    }

    /// Loads every `*.toml` file in `dir`, sorted by file name. A missing dir is not an error.
    pub fn add_dir(&mut self, dir: &Path) -> Result<(), RegistryError> {
        crate::action_impl::add_dir(self, dir)
    }

    /// The built-in actions shipped in the repository `actions/` folder.
    pub fn builtin() -> Self {
        crate::action_impl::builtin()
    }

    pub fn all(&self) -> &[ActionDef] {
        &self.actions
    }

    pub fn get(&self, id: &str) -> Option<&ActionDef> {
        self.actions.iter().find(|a| a.id == id)
    }

    /// Actions offered for this selection, sorted by (ring, order, id).
    /// `engine_ok` filters out actions whose engine is unavailable.
    pub fn matching(&self, files: &[&Path], engine_ok: &dyn Fn(&str) -> bool) -> Vec<&ActionDef> {
        crate::action_impl::matching(&self.actions, files, engine_ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::{Field, Kind, OptionsKind};

    fn localized(en: &str) -> Localized {
        Localized {
            ar: String::from("ع"),
            en: en.to_string(),
        }
    }

    fn base(params: &[(&str, toml::Value)], options: Option<OptionsSpec>) -> ActionDef {
        let table: toml::Table = params
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.clone()))
            .collect();
        ActionDef {
            id: String::from("x.test"),
            engine: String::from("image"),
            op: String::from("resize"),
            ring: 1,
            order: 0,
            accepts: vec![MediaKind::Image],
            skip_ext: vec![],
            min_files: 1,
            max_files: 0,
            output: OutputSpec {
                mode: OutputMode::PerFile,
                ext: String::new(),
                suffix: String::new(),
                dir: String::new(),
            },
            code: localized("T"),
            label: localized("Test"),
            icon: String::new(),
            params: table,
            options,
        }
    }

    fn quality_spec() -> OptionsSpec {
        OptionsSpec {
            kind: OptionsKind::Form,
            readout: vec![],
            remember: false,
            suffix_template: String::new(),
            fields: vec![Field {
                id: String::from("quality"),
                kind: Kind::Int,
                label: localized("Quality"),
                default: Some(toml::Value::Integer(85)),
                min: Some(1.0),
                max: Some(100.0),
                step: None,
                unit: Some(String::from("%")),
                values: vec![],
                maps_to: Some(String::from("quality")),
                clears: vec![],
                no_enlarge: false,
            }],
            presets: vec![],
        }
    }

    fn table(entries: &[(&str, toml::Value)]) -> toml::Table {
        entries
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.clone()))
            .collect()
    }

    #[test]
    fn s120_with_params_resolves_onto_a_copy() {
        let action = base(
            &[("quality", toml::Value::Integer(85))],
            Some(quality_spec()),
        );
        let explicit = table(&[("quality", toml::Value::Integer(60))]);
        let run = action.with_params(&explicit).unwrap();
        assert_eq!(
            run.params.get("quality").and_then(toml::Value::as_integer),
            Some(60)
        );
        // The source action is untouched: the registry entry never changes.
        assert_eq!(
            action
                .params
                .get("quality")
                .and_then(toml::Value::as_integer),
            Some(85)
        );
        // Every other field of the action is carried over.
        assert_eq!(run.id, action.id);
        assert_eq!(run.output, action.output);
    }

    #[test]
    fn s120_with_params_without_options_refuses_a_key() {
        let action = base(&[("quality", toml::Value::Integer(85))], None);
        let explicit = table(&[("quality", toml::Value::Integer(60))]);
        assert_eq!(
            action.with_params(&explicit).unwrap_err(),
            OptionsError::Unknown(String::from("quality"))
        );
    }

    #[test]
    fn s120_with_params_without_options_and_nothing_explicit_copies() {
        let action = base(&[("quality", toml::Value::Integer(85))], None);
        let run = action.with_params(&toml::Table::new()).unwrap();
        assert_eq!(run, action);
    }

    #[test]
    fn s120_with_params_reports_an_unknown_key_and_a_wrong_type() {
        let action = base(
            &[("quality", toml::Value::Integer(85))],
            Some(quality_spec()),
        );
        assert_eq!(
            action
                .with_params(&table(&[("nope", toml::Value::Integer(1))]))
                .unwrap_err(),
            OptionsError::Unknown(String::from("nope"))
        );
        match action
            .with_params(&table(&[(
                "quality",
                toml::Value::String(String::from("high")),
            )]))
            .unwrap_err()
        {
            OptionsError::Type { name, expected } => {
                assert_eq!(name, "quality");
                assert_eq!(expected, "integer");
            }
            other => panic!("expected Type, got {other:?}"),
        }
    }
}
