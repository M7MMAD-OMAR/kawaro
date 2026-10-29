//! User settings in `config.toml` inside [`crate::paths::config_dir`]
//! (`~/.config/kawaro` on Linux, `%APPDATA%\Kawaro` on Windows). Every
//! field has a default, unknown keys are ignored, and a broken file falls
//! back to the defaults with a warning instead of stopping the daemon.

use crate::i18n::Lang;
use crate::update::Channel;
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

/// What the daemon does with an update it found (ADR 0006).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateMode {
    #[default]
    Ask,
    Auto,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    /// Limit job completion notifications to jobs lasting at least 10 seconds.
    #[serde(default)]
    pub notify_long_only: bool,
    /// Spelled with an underscore in the file, as S-019 AC1 writes it.
    #[serde(rename = "hive_surface")]
    pub hive_surface: HiveSurface,
    /// The long press threshold of `kawaro drop --hold` in milliseconds
    /// (S-019e AC18), [`HOLD_MS_MIN`] to [`HOLD_MS_MAX`]. Spelled with an
    /// underscore, as `hive_surface` is.
    #[serde(rename = "hold_ms", deserialize_with = "hold_ms_in_range")]
    pub hold_ms: u16,
    /// The longest a held drop key waits for a drag or a release, in
    /// milliseconds (S-095 AC4). Default 10000.
    #[serde(rename = "hold_max_ms", default = "hold_max_default")]
    pub hold_max_ms: u32,
    /// Register the drop shortcut with the GlobalShortcuts portal (S-014e1):
    /// at start, and live when the setting changes (S-014f2). The daemon
    /// writes `false` after the user denies the portal's dialog, so it does
    /// not return at every login. Spelled with an underscore, as `hold_ms`
    /// is.
    #[serde(rename = "portal_shortcut")]
    pub portal_shortcut: bool,
    /// The global shortcut that opens the hive on Windows (S-048), as
    /// `hotkey::parse` reads it.
    pub shortcut: String,
    /// The drag key of the settings picker (S-076). Resolved by
    /// [`crate::drop_key::resolve`] (S-084 D1 and D6): a missing key is the
    /// platform default and a stored Alt on Windows re-resolves to the
    /// Windows default on read, with no migration write. Spelled with an
    /// underscore, as `hold_ms` is; written only when the user chose one.
    #[serde(rename = "drop_key", default, skip_serializing_if = "Option::is_none")]
    pub drop_key: Option<String>,
    /// Check for updates in the background (S-053a). A manual check ignores it.
    #[serde(rename = "update_check")]
    pub update_check: bool,
    /// The release line to follow, `stable` or `beta`.
    #[serde(rename = "update_channel")]
    pub update_channel: Channel,
    /// `ask` or `auto` for what happens when an update is ready.
    #[serde(rename = "update_mode")]
    pub update_mode: UpdateMode,
    /// The one-shot first-run marker (S-087 AC1, D5): written only as
    /// `welcome_seen = true`, by the welcome window after it is actually
    /// shown, and by no other write path. Skipped while false, so a config
    /// written by a portal denial save, a settings save or an updater write
    /// never hides the welcome. Spelled with an underscore, as `hold_ms` is.
    #[serde(rename = "welcome_seen", default, skip_serializing_if = "is_false")]
    pub welcome_seen: bool,
    /// True when the loaded file had an `update_channel` key. It lets the
    /// beta default for a prerelease build survive a save from the settings
    /// window (S-054b). Not part of the file and not part of equality.
    #[serde(skip)]
    pub update_channel_present: bool,
}

impl PartialEq for Config {
    /// Every setting; `update_channel_present` only describes where the
    /// values came from, so two configs with the same settings are equal.
    fn eq(&self, o: &Self) -> bool {
        [
            self.language == o.language,
            self.theme == o.theme,
            self.output == o.output,
            self.output_folder == o.output_folder,
            self.quality == o.quality,
            self.open_folder_when_done == o.open_folder_when_done,
            self.notify == o.notify,
            self.notify_long_only == o.notify_long_only,
            self.hive_surface == o.hive_surface,
            self.hold_ms == o.hold_ms,
            self.hold_max_ms == o.hold_max_ms,
            self.portal_shortcut == o.portal_shortcut,
            self.shortcut == o.shortcut,
            self.drop_key == o.drop_key,
            self.update_check == o.update_check,
            self.update_channel == o.update_channel,
            self.update_mode == o.update_mode,
            self.welcome_seen == o.welcome_seen,
        ]
        .iter()
        .all(|same| *same)
    }
}

/// The default long press threshold.
pub const HOLD_MS_DEFAULT: u16 = 400;
/// The default `hold_max_ms`.
pub const HOLD_MAX_DEFAULT: u32 = 10_000;
fn hold_max_default() -> u32 {
    HOLD_MAX_DEFAULT
}
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

/// `welcome_seen` is written only as `true` (S-087 D5): the key is absent
/// from the file while the welcome has not been shown.
fn is_false(value: &bool) -> bool {
    !*value
}

impl Default for Config {
    fn default() -> Self {
        Self {
            language: LangSetting::Auto,
            theme: Theme::Auto,
            output: OutputPlace::Beside,
            output_folder: "Kawaro".into(),
            quality: Quality::Balanced,
            open_folder_when_done: false,
            notify: true,
            notify_long_only: false,
            hive_surface: HiveSurface::Auto,
            hold_ms: HOLD_MS_DEFAULT,
            hold_max_ms: HOLD_MAX_DEFAULT,
            portal_shortcut: true,
            shortcut: "Ctrl+Alt+H".into(),
            drop_key: None,
            update_check: true,
            update_channel: Channel::Stable,
            update_mode: UpdateMode::Ask,
            welcome_seen: false,
            update_channel_present: false,
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
            Ok(text) => match toml::from_str::<Config>(&text) {
                Ok(c) => (
                    Config {
                        update_channel_present: Self::explicit_update_channel(&text).is_some(),
                        ..c
                    },
                    None,
                ),
                Err(e) => (Self::default(), Some(format!("{}: {e}", path.display()))),
            },
        }
    }

    /// The `update_channel` written in a config file text; `None` when the
    /// key is missing, is not `stable` or `beta`, or the text is not TOML.
    pub fn explicit_update_channel(text: &str) -> Option<Channel> {
        let table: toml::Table = toml::from_str(text).ok()?;
        Channel::parse(table.get("update_channel")?.as_str()?)
    }

    /// The channel the user chose: the file's value when it had the key or
    /// it is not the default, otherwise none.
    pub fn configured_update_channel(&self) -> Option<Channel> {
        (self.update_channel_present || self.update_channel != Channel::Stable)
            .then_some(self.update_channel)
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
        let mut table = toml::Table::try_from(self).map_err(std::io::Error::other)?;
        // An unset channel stays unset, so the beta default of a prerelease
        // build survives a save (S-054b).
        if !self.update_channel_present && self.update_channel == Channel::Stable {
            table.remove("update_channel");
        }
        let text = toml::to_string_pretty(&table).map_err(std::io::Error::other)?;
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
                    dir.join("Kawaro")
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
        assert!(!c.notify_long_only);
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
            notify_long_only: true,
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
                PathBuf::from("/home/u/pics/Kawaro"),
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
        #[cfg(unix)]
        {
            // The only test in this binary that touches these variables.
            let saved = (
                std::env::var_os("XDG_CONFIG_HOME"),
                std::env::var_os("HOME"),
            );
            let dir = tempfile::tempdir().unwrap();
            let xdg = dir.path().join("xdg");
            unsafe { std::env::set_var("XDG_CONFIG_HOME", &xdg) };
            assert_eq!(Config::path(), Some(xdg.join("kawaro").join("config.toml")));
            Config {
                quality: Quality::Light,
                ..Config::default()
            }
            .save_to(&xdg.join("kawaro").join("config.toml"))
            .unwrap();
            assert_eq!(Config::load().0.quality, Quality::Light);
            assert!(Config::load().1.is_none());

            unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
            unsafe { std::env::set_var("HOME", dir.path()) };
            let home_path = dir
                .path()
                .join(".config")
                .join("kawaro")
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

        // Windows has no XDG or HOME: the folder is %APPDATA%\Kawaro (S-058).
        #[cfg(windows)]
        {
            let saved = std::env::var_os("APPDATA");
            let dir = tempfile::tempdir().unwrap();
            let appdata = dir.path().join("AppData");
            unsafe { std::env::set_var("APPDATA", &appdata) };
            let expected = appdata.join("Kawaro").join("config.toml");
            assert_eq!(Config::path(), Some(expected.clone()));
            Config {
                quality: Quality::Light,
                ..Config::default()
            }
            .save_to(&expected)
            .unwrap();
            assert_eq!(Config::load().0.quality, Quality::Light);
            assert!(Config::load().1.is_none());

            // An unset APPDATA falls back to the roaming known folder.
            unsafe { std::env::remove_var("APPDATA") };
            let fallback = Config::path().expect("the known folder gives a path");
            assert!(fallback.ends_with(std::path::Path::new("Kawaro").join("config.toml")));
            assert!(!fallback.starts_with(dir.path()));

            match saved {
                Some(v) => unsafe { std::env::set_var("APPDATA", v) },
                None => unsafe { std::env::remove_var("APPDATA") },
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
