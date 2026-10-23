//! User settings in `config.toml` inside [`crate::paths::config_dir`]
//! (`~/.config/sbarhive` on Linux, `%APPDATA%\SbarHive` on Windows). Every
//! field has a default, unknown keys are ignored, and a broken file falls
//! back to the defaults with a warning instead of stopping the daemon.

use crate::i18n::Lang;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LangSetting {
    #[default]
    Auto,
    Ar,
    En,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    #[default]
    Auto,
    Light,
    Dark,
}

/// Where results go.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputPlace {
    /// Next to the original file.
    #[default]
    Beside,
    /// In a subfolder next to the original, named by `output_folder`.
    Subfolder,
}

/// Quality preset applied on top of action parameters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Quality {
    /// Smaller files, faster.
    Light,
    #[default]
    Balanced,
    /// Best quality, slower and larger.
    Max,
}

/// Where the hive is drawn (S-019a): `auto` follows the compositor and the
/// screen reader rule, `layer` and `window` override both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HiveSurface {
    #[default]
    Auto,
    Layer,
    Window,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Config {
    pub language: LangSetting,
    pub theme: Theme,
    pub output: OutputPlace,
    pub output_folder: String,
    pub quality: Quality,
    /// Open the result folder when a job ends.
    pub open_folder_when_done: bool,
    /// Desktop notification when a job ends.
    pub notify: bool,
    /// Spelled with an underscore in the file, as S-019 AC1 writes it.
    #[serde(rename = "hive_surface")]
    pub hive_surface: HiveSurface,
    /// The long press threshold of `sbarhive drop --hold` in milliseconds
    /// (S-019e AC18), [`HOLD_MS_MIN`] to [`HOLD_MS_MAX`]. Spelled with an
    /// underscore, as `hive_surface` is.
    #[serde(rename = "hold_ms", deserialize_with = "hold_ms_in_range")]
    pub hold_ms: u16,
    /// Register the drop shortcut with the GlobalShortcuts portal (S-014e1):
    /// at start, and live when the setting changes (S-014f2). The daemon
    /// writes `false` after the user denies the portal's dialog, so it does
    /// not return at every login. Spelled with an underscore, as `hold_ms`
    /// is.
    #[serde(rename = "portal_shortcut")]
    pub portal_shortcut: bool,
}

/// The default long press threshold.
pub const HOLD_MS_DEFAULT: u16 = 400;
/// The shortest accepted threshold: below it a press is a tap.
pub const HOLD_MS_MIN: u16 = 100;
/// The longest accepted threshold.
pub const HOLD_MS_MAX: u16 = 2000;

/// `hold_ms` as a whole number in range; anything else is an error, so the
/// file falls back to the defaults with a warning, as for every other key.
fn hold_ms_in_range<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u16, D::Error> {
    let v = i64::deserialize(d)?;
    u16::try_from(v)
        .ok()
        .filter(|v| (HOLD_MS_MIN..=HOLD_MS_MAX).contains(v))
        .ok_or_else(|| {
            serde::de::Error::custom(format!(
                "hold_ms = {v}: expected {HOLD_MS_MIN} to {HOLD_MS_MAX} milliseconds"
            ))
        })
}

impl Default for Config {
    fn default() -> Self {
        Self {
            language: LangSetting::Auto,
            theme: Theme::Auto,
            output: OutputPlace::Beside,
            output_folder: "SbarHive".into(),
            quality: Quality::Balanced,
            open_folder_when_done: false,
            notify: true,
            hive_surface: HiveSurface::Auto,
            hold_ms: HOLD_MS_DEFAULT,
            portal_shortcut: true,
        }
    }
}

impl Config {
    pub fn path() -> Option<PathBuf> {
        crate::paths::config_dir().map(|d| d.join("config.toml"))
    }

    /// The folder of the user's own actions, `actions/` next to `config.toml`.
    pub fn actions_dir() -> Option<PathBuf> {
        crate::paths::config_dir().map(|d| d.join("actions"))
    }

    /// Defaults when the file is missing; defaults plus a warning when it is broken.
    pub fn load_from(path: &Path) -> (Self, Option<String>) {
        match std::fs::read_to_string(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Self::default(), None),
            Err(e) => (Self::default(), Some(format!("{}: {e}", path.display()))),
            Ok(text) => match toml::from_str(&text) {
                Ok(c) => (c, None),
                Err(e) => (Self::default(), Some(format!("{}: {e}", path.display()))),
            },
        }
    }

    pub fn load() -> (Self, Option<String>) {
        match Self::path() {
            Some(p) => Self::load_from(&p),
            None => (Self::default(), None),
        }
    }

    /// Writes atomically: temp file in the same directory, then rename.
    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = toml::to_string_pretty(self).map_err(std::io::Error::other)?;
        let tmp = path.with_extension(format!("toml.{}.tmp", std::process::id()));
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, path)
    }

    pub fn lang(&self) -> Lang {
        match self.language {
            LangSetting::Auto => Lang::detect(),
            LangSetting::Ar => Lang::Ar,
            LangSetting::En => Lang::En,
        }
    }

    /// Directory the results of `input` go to. Creating it is the caller's job.
    pub fn output_dir(&self, input: &Path) -> PathBuf {
        let dir = input.parent().unwrap_or(Path::new("/")).to_path_buf();
        match self.output {
            OutputPlace::Beside => dir,
            OutputPlace::Subfolder => {
                let name = self.output_folder.trim();
                let safe = !name.is_empty() && !name.contains('/') && name != "." && name != "..";
                if safe {
                    dir.join(name)
                } else {
                    dir.join("SbarHive")
                }
            }
        }
    }

    /// Adjusts a lossy `quality` parameter for the preset. Other params pass through.
    pub fn apply_quality(&self, params: &mut toml::Table) {
        let Some(q) = params.get("quality").and_then(|v| v.as_integer()) else {
            return;
        };
        let adjusted = match self.quality {
            Quality::Light => q - 15,
            Quality::Balanced => q,
            Quality::Max => q + 10,
        }
        .clamp(1, 100);
        params.insert("quality".into(), adjusted.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_gives_defaults_without_warning() {
        let dir = tempfile::tempdir().unwrap();
        let (c, warn) = Config::load_from(&dir.path().join("none.toml"));
        assert_eq!(c, Config::default());
        assert!(warn.is_none());
    }

    #[test]
    fn partial_file_keeps_other_defaults_and_ignores_unknown_keys() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("config.toml");
        std::fs::write(&p, "language = \"ar\"\nquality = \"max\"\nfuture-key = 3\n").unwrap();
        let (c, warn) = Config::load_from(&p);
        assert!(warn.is_none());
        assert_eq!(c.language, LangSetting::Ar);
        assert_eq!(c.quality, Quality::Max);
        assert_eq!(c.output, OutputPlace::Beside);
        assert!(c.notify);
    }

    #[test]
    fn broken_file_falls_back_with_warning() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("config.toml");
        std::fs::write(&p, "language = [").unwrap();
        let (c, warn) = Config::load_from(&p);
        assert_eq!(c, Config::default());
        assert!(warn.unwrap().contains("config.toml"));
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("sub").join("config.toml");
        let c = Config {
            theme: Theme::Dark,
            output: OutputPlace::Subfolder,
            output_folder: "محوّل".into(),
            ..Config::default()
        };
        c.save_to(&p).unwrap();
        assert_eq!(Config::load_from(&p), (c, None));
        let leftovers: Vec<_> = std::fs::read_dir(p.parent().unwrap()).unwrap().collect();
        assert_eq!(leftovers.len(), 1, "temp file left behind");
    }

    #[test]
    fn output_dir_follows_setting_and_rejects_unsafe_names() {
        let input = Path::new("/home/u/pics/a.png");
        let mut c = Config::default();
        assert_eq!(c.output_dir(input), PathBuf::from("/home/u/pics"));
        c.output = OutputPlace::Subfolder;
        c.output_folder = "out".into();
        assert_eq!(c.output_dir(input), PathBuf::from("/home/u/pics/out"));
        for bad in ["", "  ", "..", ".", "a/b"] {
            c.output_folder = bad.into();
            assert_eq!(
                c.output_dir(input),
                PathBuf::from("/home/u/pics/SbarHive"),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn quality_preset_shifts_and_clamps() {
        let mut c = Config::default();
        let mut p = toml::Table::new();
        p.insert("quality".into(), 85.into());
        c.apply_quality(&mut p);
        assert_eq!(p["quality"].as_integer(), Some(85));
        c.quality = Quality::Light;
        c.apply_quality(&mut p);
        assert_eq!(p["quality"].as_integer(), Some(70));
        c.quality = Quality::Max;
        p.insert("quality".into(), 95.into());
        c.apply_quality(&mut p);
        assert_eq!(p["quality"].as_integer(), Some(100));
        p.insert("quality".into(), 5.into());
        c.quality = Quality::Light;
        c.apply_quality(&mut p);
        assert_eq!(p["quality"].as_integer(), Some(1));
        let mut none = toml::Table::new();
        c.apply_quality(&mut none);
        assert!(none.is_empty());
    }

    #[test]
    fn max_preset_adds_ten() {
        let c = Config {
            quality: Quality::Max,
            ..Config::default()
        };
        let mut p = toml::Table::new();
        p.insert("quality".into(), 85.into());
        c.apply_quality(&mut p);
        assert_eq!(p["quality"].as_integer(), Some(95));
    }

    #[test]
    fn unreadable_file_warns_unlike_a_missing_one() {
        let dir = tempfile::tempdir().unwrap();
        // A directory where the file should be: a read error that is not NotFound.
        let p = dir.path().join("config.toml");
        std::fs::create_dir(&p).unwrap();
        let (c, warn) = Config::load_from(&p);
        assert_eq!(c, Config::default());
        assert!(warn.unwrap().contains("config.toml"));
    }

    #[test]
    fn path_and_load_follow_xdg_then_home() {
        // The only test in this binary that touches these variables.
        let saved = (
            std::env::var_os("XDG_CONFIG_HOME"),
            std::env::var_os("HOME"),
        );
        let dir = tempfile::tempdir().unwrap();
        let xdg = dir.path().join("xdg");
        unsafe { std::env::set_var("XDG_CONFIG_HOME", &xdg) };
        assert_eq!(
            Config::path(),
            Some(xdg.join("sbarhive").join("config.toml"))
        );
        Config {
            quality: Quality::Light,
            ..Config::default()
        }
        .save_to(&xdg.join("sbarhive").join("config.toml"))
        .unwrap();
        assert_eq!(Config::load().0.quality, Quality::Light);
        assert!(Config::load().1.is_none());

        unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
        unsafe { std::env::set_var("HOME", dir.path()) };
        let home_path = dir
            .path()
            .join(".config")
            .join("sbarhive")
            .join("config.toml");
        assert_eq!(Config::path(), Some(home_path));
        assert_eq!(Config::load(), (Config::default(), None));

        unsafe { std::env::remove_var("HOME") };
        assert_eq!(Config::path(), None);
        assert_eq!(Config::load(), (Config::default(), None));

        for (k, v) in [("XDG_CONFIG_HOME", saved.0), ("HOME", saved.1)] {
            match v {
                Some(v) => unsafe { std::env::set_var(k, v) },
                None => unsafe { std::env::remove_var(k) },
            }
        }
    }

    #[test]
    fn explicit_language_overrides_locale() {
        let c = Config {
            language: LangSetting::Ar,
            ..Config::default()
        };
        assert_eq!(c.lang(), Lang::Ar);
        let c = Config {
            language: LangSetting::En,
            ..Config::default()
        };
        assert_eq!(c.lang(), Lang::En);
    }
}
