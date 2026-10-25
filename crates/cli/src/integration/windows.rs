//! The only code of S-050 that starts `reg.exe`; the live check is S-052.

use kawaro_core::i18n::Lang;
use kawaro_core::registry;
use std::path::Path;
use std::process::{Command, ExitCode};

fn run_all(commands: &[Vec<String>]) -> ExitCode {
    for c in commands {
        match Command::new("reg.exe").args(c).status() {
            Ok(s) if s.success() => {}
            Ok(s) => {
                eprintln!(
                    "kawaro: reg.exe {} failed with exit code {}",
                    c.join(" "),
                    s.code().unwrap_or(-1)
                );
                return ExitCode::FAILURE;
            }
            Err(e) => {
                eprintln!("kawaro: cannot start reg.exe: {e}");
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}

pub fn install(dir: &str, lang: Lang) -> ExitCode {
    let missing = registry::missing_files(dir, |p| Path::new(p).is_file());
    if !missing.is_empty() {
        eprintln!(
            "kawaro: missing next to the executable: {}",
            missing.join(", ")
        );
        return ExitCode::FAILURE;
    }
    match registry::install_commands(dir, lang) {
        Ok(commands) => run_all(&commands),
        Err(e) => {
            eprintln!("kawaro: {e}");
            ExitCode::FAILURE
        }
    }
}

pub fn uninstall() -> ExitCode {
    run_all(&registry::uninstall_commands())
}
