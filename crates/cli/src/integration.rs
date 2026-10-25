//! `kawaro integration install|uninstall`: the Explorer right click menu
//! (S-050). The command lines come from `kawaro_core::registry`; only the
//! Windows runner in `integration/windows.rs` calls `reg.exe`.

use kawaro_core::i18n::Lang;
use kawaro_core::registry;
use std::path::PathBuf;
use std::process::ExitCode;

#[cfg(windows)]
mod windows;

fn print_all(commands: &[Vec<String>]) {
    for c in commands {
        out!("reg.exe {}", c.join(" "));
    }
}

/// Exit code 2: this OS has no Explorer menu to change.
#[cfg(not(windows))]
fn windows_only(what: &str) -> ExitCode {
    eprintln!("kawaro: integration {what}: available on Windows only");
    ExitCode::from(2)
}

fn program_dir(dir: Option<PathBuf>) -> Result<String, String> {
    let dir = match dir {
        Some(d) => d,
        None => std::env::current_exe()
            .map_err(|e| e.to_string())?
            .parent()
            .map(PathBuf::from)
            .ok_or("the executable has no folder")?,
    };
    Ok(dir.display().to_string())
}

pub fn install(dry_run: bool, dir: Option<PathBuf>) -> ExitCode {
    let lang = Lang::detect();
    let dir = match program_dir(dir) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("kawaro: {e}");
            return ExitCode::FAILURE;
        }
    };
    if dry_run {
        return match registry::install_commands(&dir, lang) {
            Ok(commands) => {
                print_all(&commands);
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("kawaro: {e}");
                ExitCode::FAILURE
            }
        };
    }
    #[cfg(windows)]
    return windows::install(&dir, lang);
    #[cfg(not(windows))]
    return windows_only("install");
}

pub fn uninstall(dry_run: bool) -> ExitCode {
    if dry_run {
        print_all(&registry::uninstall_commands());
        return ExitCode::SUCCESS;
    }
    #[cfg(windows)]
    return windows::uninstall();
    #[cfg(not(windows))]
    return windows_only("uninstall");
}
