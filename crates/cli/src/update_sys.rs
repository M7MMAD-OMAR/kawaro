//! The live side of `kawaro update` (S-054c): the facts of this machine,
//! the system curl and the real installer runner. Nothing is decided here;
//! the command is `update.rs` and the rules are in `kawaro_core::update`.
//! It reads `/usr/bin`, `/etc` and the running program and starts `pkexec`,
//! so it is covered by `tests/s054c_update_bin.rs` and the S-052 live check
//! and left out of the mutants run (`.cargo/mutants.toml`).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use kawaro_core::paths::{CURRENT, cache_dir_with, config_dir};
use kawaro_core::update::apply::{PkgKind, SystemRunner, clean_exe_path, probe_facts};
use kawaro_core::update::fetch::CurlFetcher;
use kawaro_core::update::flow::platform_with;
use kawaro_core::update::verify::UPDATE_KEYS;

use crate::update::{self, Facts};

/// This program's path, without the ` (deleted)` Linux adds after a package
/// replaced the file.
fn exe() -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("/usr/bin/kawaro"));
    clean_exe_path(&exe)
}

/// `kawaro update ARGS` on this machine.
pub fn run(args: &[String]) -> ExitCode {
    let exists = |p: &str| Path::new(p).exists();
    let platform = platform_with(CURRENT, std::env::consts::ARCH, &exists);
    let installed_kind = PkgKind::parse(&platform.pkg);
    let cache = cache_dir_with(CURRENT, &|k| std::env::var_os(k), &|| None)
        .unwrap_or_else(|| std::env::temp_dir().join("kawaro-cache"));
    let facts = Facts {
        system: probe_facts(installed_kind),
        installed: env!("CARGO_PKG_VERSION").to_string(),
        keys: UPDATE_KEYS.iter().map(|k| k.to_string()).collect(),
        platform,
        config_dir: config_dir().unwrap_or_default(),
        cache_dir: cache,
        exe: exe(),
        runner: &SystemRunner,
    };
    let code = update::run(
        args,
        &CurlFetcher::system(),
        &facts,
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    );
    ExitCode::from(u8::try_from(code).unwrap_or(1))
}
