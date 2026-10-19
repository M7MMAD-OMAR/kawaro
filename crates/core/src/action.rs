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
//! [action.params]
//! quality = 85
//! ```

use crate::media::MediaKind;
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
    #[serde(default)]
    pub params: toml::Table,
}

fn one() -> usize {
    1
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
