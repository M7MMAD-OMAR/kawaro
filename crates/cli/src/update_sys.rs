//! The live side of `kawaro update` (S-054c): the facts of this machine,
//! the system curl and the real installer runner. Nothing is decided here;
//! the command is `update.rs` and the rules are in `kawaro_core::update`.
//! It reads `/usr/bin`, `/etc` and the running program and starts `pkexec`,
//! so it is covered by `tests/s054c_update_bin.rs` and the S-052 live check
//! and left out of the mutants run (`.cargo/mutants.toml`).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use kawaro_core::paths::{CURRENT, cache_dir_with, config_dir};
use kawaro_core::update::UpdateError;
use kawaro_core::update::apply::{CommandRunner, Facts as SystemFacts, PKEXEC, PkgKind};
use kawaro_core::update::fetch::CurlFetcher;
use kawaro_core::update::flow::platform_with;
use kawaro_core::update::verify::UPDATE_KEYS;

use crate::update::{self, Facts};

/// Runs a program and gives its exit code. A `pkexec` command is refused
/// unless root owns the program it would run as root (S-054a).
struct Runner;

impl CommandRunner for Runner {
    fn run(&self, argv: &[String]) -> Result<i32, UpdateError> {
        #[cfg(unix)]
        if argv[0] == PKEXEC {
            use std::os::unix::fs::MetadataExt;
            let meta = std::fs::metadata(&argv[1]).map_err(|_| UpdateError::Io)?;
            if !kawaro_core::update::apply::exe_is_trusted(meta.uid(), meta.mode()) {
                return Err(UpdateError::Io);
            }
        }
        std::process::Command::new(&argv[0])
            .args(&argv[1..])
            .status()
            .map_err(|_| UpdateError::Io)?
            .code()
            .ok_or(UpdateError::Io)
    }
}

/// A file matching `*kawaro*.repo` or `kawaro.sources` exists.
fn repo_configured() -> bool {
    ["/etc/yum.repos.d", "/etc/apt/sources.list.d"]
        .iter()
        .filter_map(|d| std::fs::read_dir(d).ok())
        .flatten()
        .flatten()
        .any(|e| {
            let name = e.file_name().to_string_lossy().to_lowercase();
            (name.contains("kawaro") && name.ends_with(".repo")) || name == "kawaro.sources"
        })
}

/// This program's path, without the ` (deleted)` Linux adds after a package
/// replaced the file.
fn exe() -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("/usr/bin/kawaro"));
    let text = exe.to_string_lossy();
    match text.strip_suffix(" (deleted)") {
        Some(clean) => PathBuf::from(clean),
        None => exe,
    }
}

/// `kawaro update ARGS` on this machine.
pub fn run(args: &[String]) -> ExitCode {
    let exists = |p: &str| Path::new(p).exists();
    let platform = platform_with(CURRENT, std::env::consts::ARCH, &exists);
    let installed_kind = PkgKind::parse(&platform.pkg);
    let cache = cache_dir_with(CURRENT, &|k| std::env::var_os(k), &|| None)
        .unwrap_or_else(|| std::env::temp_dir().join("kawaro-cache"));
    let facts = Facts {
        system: SystemFacts {
            has_dnf: exists("/usr/bin/dnf"),
            has_apt: exists("/usr/bin/apt-get"),
            has_pkexec: exists(PKEXEC),
            repo_configured: repo_configured(),
            installed_kind,
        },
        installed: env!("CARGO_PKG_VERSION").to_string(),
        keys: UPDATE_KEYS.iter().map(|k| k.to_string()).collect(),
        platform,
        config_dir: config_dir().unwrap_or_default(),
        cache_dir: cache,
        exe: exe(),
        runner: &Runner,
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
