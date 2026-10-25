//! The Explorer right click menu as `reg.exe` argument lists (S-050).
//!
//! Everything here is pure, so each argument is checkable on any OS; the
//! Windows-only runner in the CLI feeds these lists to `reg.exe`. Only
//! `HKCU` is written, so no administrator rights are needed.

use crate::i18n::Lang;

const SHELL_KEY: &str = r"HKCU\Software\Classes\*\shell\Kawaro";
const COMMAND_KEY: &str = r"HKCU\Software\Classes\*\shell\Kawaro\command";
const AUMID_KEY: &str = r"HKCU\Software\Classes\AppUserModelId\io.github.m7mmad_omar.Kawaro";
const SERVICE_EXE: &str = "kawarod.exe";
const ICON: &str = "kawaro.ico";

fn args(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| s.to_string()).collect()
}

fn add_named(key: &str, name: &str, data: &str) -> Vec<String> {
    args(&["add", key, "/v", name, "/t", "REG_SZ", "/d", data, "/f"])
}

/// The `reg.exe` calls that add the menu item for the program folder `dir`,
/// in the order they must run.
pub fn install_commands(dir: &str, lang: Lang) -> Result<Vec<Vec<String>>, String> {
    if dir.contains('"') {
        return Err("path contains a quote".into());
    }
    let dir = dir.trim_end_matches('\\');
    let icon = format!(r"{dir}\{ICON}");
    let verb = match lang {
        Lang::En => "Kawaro",
        Lang::Ar => "كوارو",
    };
    let command = format!(r#""{dir}\{SERVICE_EXE}" open "%1""#);
    Ok(vec![
        add_named(SHELL_KEY, "MUIVerb", verb),
        add_named(SHELL_KEY, "Icon", &icon),
        add_named(SHELL_KEY, "MultiSelectModel", "Player"),
        args(&[
            "add",
            COMMAND_KEY,
            "/ve",
            "/t",
            "REG_SZ",
            "/d",
            &command,
            "/f",
        ]),
        add_named(AUMID_KEY, "DisplayName", "Kawaro"),
        add_named(AUMID_KEY, "IconUri", &icon),
    ])
}

/// The `reg.exe` calls that remove everything `install_commands` wrote.
pub fn uninstall_commands() -> Vec<Vec<String>> {
    vec![
        args(&["delete", SHELL_KEY, "/f"]),
        args(&["delete", AUMID_KEY, "/f"]),
    ]
}

/// The files `install` needs next to the executable that `exists` does not
/// find, in the order they are checked.
pub fn missing_files(dir: &str, exists: impl Fn(&str) -> bool) -> Vec<String> {
    let dir = dir.trim_end_matches('\\');
    [SERVICE_EXE, ICON]
        .into_iter()
        .filter(|name| !exists(&format!(r"{dir}\{name}")))
        .map(str::to_string)
        .collect()
}
