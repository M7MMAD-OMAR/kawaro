//! Applying a verified update (S-054a, ADR 0006 sections 6 and 7). Every
//! decision is pure and tested here: the `msiexec` command and its exit
//! codes, the Linux plan, the privileged install, the timing rules and the
//! order of the Windows helper. The only code that talks to the system is
//! `install_privileged` (fake package managers in the tests), `exe_stat`
//! (a read only `lstat` walk) and `apply/windows.rs`, which is checked live
//! in S-052.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::UpdateError;
use super::result::{Failure, Outcome, UpdateResult};
use crate::config::UpdateMode;
use crate::paths::{EnvFn, Os};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{apply_msi, helper_main};

/// The page that lists the packages, shown when nothing else can install.
pub const RELEASE_PAGE_URL: &str = "https://github.com/M7MMAD-OMAR/kawaro/releases/latest";

/// `pkexec` by absolute path, never found through `PATH`.
pub const PKEXEC: &str = "/usr/bin/pkexec";

/// The `kawaro update` flag that runs the privileged step.
pub const INSTALL_PRIVILEGED_FLAG: &str = "--install-privileged";

/// The exit code of the privileged step when it is not run by root.
pub const EXIT_NOT_ROOT: i32 = 77;

/// How long a pending apply waits before it asks again.
pub const RETRY_INTERVAL: Duration = Duration::from_secs(30);

/// How long the Windows helper waits for the daemon to exit.
pub const DAEMON_EXIT_TIMEOUT: Duration = Duration::from_secs(60);

// ---------------------------------------------------------------- Windows

/// What `msiexec` answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MsiResult {
    Installed,
    InstalledRebootPending,
    Cancelled,
    Busy,
    Failed,
}

/// The quiet, per-user install: `/i <msi> /qn /norestart /l*v <log>`.
pub fn msiexec_args(msi: &Path, log: &Path) -> Vec<String> {
    vec![
        "/i".to_string(),
        msi.to_string_lossy().into_owned(),
        "/qn".to_string(),
        "/norestart".to_string(),
        "/l*v".to_string(),
        log.to_string_lossy().into_owned(),
    ]
}

/// The repair of an install whose files are missing: `/fa <msi> /qn
/// /norestart /l*v <log>`.
pub fn msiexec_repair_args(msi: &Path, log: &Path) -> Vec<String> {
    let mut args = msiexec_args(msi, log);
    args[0] = "/fa".to_string();
    args
}

/// The meaning of an `msiexec` exit code.
pub fn classify_msi(code: i32) -> MsiResult {
    match code {
        0 => MsiResult::Installed,
        3010 => MsiResult::InstalledRebootPending,
        1602 => MsiResult::Cancelled,
        1618 => MsiResult::Busy,
        _ => MsiResult::Failed,
    }
}

/// The `kawaro update` flag that runs the Windows helper entry.
pub const APPLY_MSI_FLAG: &str = "--apply-msi";

/// The arguments of the helper copy of `kawaro.exe`.
pub fn helper_args(msi: &Path) -> Vec<String> {
    vec![
        "update".to_string(),
        APPLY_MSI_FLAG.to_string(),
        msi.to_string_lossy().into_owned(),
    ]
}

/// The program the helper copy is staged from: `kawaro.exe` beside the
/// running program. The copy runs `update --apply-msi`, which only the
/// `kawaro` binary handles: the caller is the daemon (`kawarod.exe`), whose
/// command line has no `update`, so a copy of the daemon renamed to
/// `kawaro.exe` could never run `helper_main` (security audit D1).
pub fn helper_exe(program_dir: &Path) -> PathBuf {
    program_dir.join("kawaro.exe")
}

/// `msiexec.exe` by absolute path under `SystemRoot`, never through `PATH`.
pub fn msiexec_path(env: EnvFn) -> Option<PathBuf> {
    let root = env("SystemRoot").filter(|r| !r.is_empty())?;
    Some(PathBuf::from(root).join("System32").join("msiexec.exe"))
}

/// Which daemon the helper starts at the end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Daemon {
    New,
    Old,
}

/// The system calls of the Windows helper, so its order is testable.
pub trait HelperOps {
    /// True when the daemon exited within `timeout`.
    fn wait_for_daemon_exit(&self, timeout: Duration) -> bool;
    /// The exit code of `msiexec` run with `args`.
    fn run_msiexec(&self, args: &[String]) -> Result<i32, UpdateError>;
    /// True when the installed `kawarod.exe` exists.
    fn daemon_present(&self) -> bool;
    fn write_result(&self, r: &UpdateResult) -> Result<(), UpdateError>;
    fn start_daemon(&self, which: Daemon);
}

/// The daemon after an install: present, or present after one repair.
fn present_after_repair(ops: &dyn HelperOps, msi: &Path, log: &Path) -> bool {
    ops.daemon_present()
        || (ops.run_msiexec(&msiexec_repair_args(msi, log)).is_ok() && ops.daemon_present())
}

/// Wait for the daemon, install, write the result, start a daemon. Without
/// the daemon's exit nothing is installed and no daemon is started (the old
/// one still runs). A result that cannot be written never stops the daemon.
/// When the install worked but `kawarod.exe` is missing, one repair runs and,
/// if the file is still missing, the failure names the MSI and no daemon starts.
pub fn run_helper(
    ops: &dyn HelperOps,
    msi: &Path,
    log: &Path,
    version: &str,
    now: u64,
) -> UpdateResult {
    let exited = ops.wait_for_daemon_exit(DAEMON_EXIT_TIMEOUT);
    let mut missing = false;
    let outcome = if !exited {
        Outcome::Failed(Failure::Text("the daemon did not exit in time".to_string()))
    } else {
        match ops.run_msiexec(&msiexec_args(msi, log)) {
            Ok(code) => {
                let outcome = Outcome::from_msi(classify_msi(code));
                missing = outcome == Outcome::Installed && !present_after_repair(ops, msi, log);
                if missing {
                    Outcome::Failed(Failure::Text(format!(
                        "kawarod.exe is missing after the update. Run {} by hand.",
                        msi.display()
                    )))
                } else {
                    outcome
                }
            }
            Err(_) => Outcome::Failed(Failure::Text("msiexec could not start".to_string())),
        }
    };
    let result = UpdateResult {
        version: version.to_string(),
        outcome,
        at: now,
    };
    let _ = ops.write_result(&result);
    if exited && !missing {
        let which = if result.outcome == Outcome::Installed {
            Daemon::New
        } else {
            Daemon::Old
        };
        ops.start_daemon(which);
    }
    result
}

// ------------------------------------------------------------------ Linux

/// The package format of the running system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PkgKind {
    Rpm,
    Deb,
}

impl PkgKind {
    /// The format named by a `--kind` text.
    pub fn parse(text: &str) -> Option<PkgKind> {
        match text {
            "rpm" => Some(PkgKind::Rpm),
            "deb" => Some(PkgKind::Deb),
            _ => None,
        }
    }

    /// The `--kind` text.
    pub fn as_str(self) -> &'static str {
        match self {
            PkgKind::Rpm => "rpm",
            PkgKind::Deb => "deb",
        }
    }
}

/// What the daemon found out about the system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    pub has_dnf: bool,
    pub has_apt: bool,
    pub has_pkexec: bool,
    /// A file matching `*kawaro*.repo` or `kawaro.sources` exists.
    pub repo_configured: bool,
    /// The format Kawaro was installed from; it picks the manager when
    /// both exist, and `None` prefers dnf.
    pub installed_kind: Option<PkgKind>,
}

/// How the update is installed on Linux.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Plan {
    /// Upgrade through the configured repository: the command to run.
    RepoUpgrade(String),
    /// Install the verified local package through the privileged step.
    Local(PkgKind),
    /// No `pkexec`: tell the user this command (`{file}` is the package).
    ShowCommand(String),
    /// No supported package manager: show this page.
    ShowRelease(String),
}

/// The repository upgrade command. `{elevate}` is [`PKEXEC`] (the argv
/// `apply_now` runs) or `sudo` (the text shown to the user). The package
/// manager is named exactly as [`Programs::system`] holds it: by absolute
/// path, never found through `PATH`, so `pkexec` cannot be pointed at a
/// planted program name in the caller's `PATH`.
fn repo_command(elevate: &str, kind: PkgKind) -> String {
    let programs = Programs::system();
    match kind {
        PkgKind::Rpm => format!("{elevate} {} upgrade -y kawaro", programs.dnf.display()),
        PkgKind::Deb => format!(
            "{elevate} {} install -y --only-upgrade kawaro",
            programs.apt_get.display()
        ),
    }
}

fn local_command(kind: PkgKind) -> String {
    match kind {
        PkgKind::Rpm => "sudo dnf install -y {file}".to_string(),
        PkgKind::Deb => "sudo apt-get install -y {file}".to_string(),
    }
}

/// Decides the install from the facts (ADR 0006 section 7).
pub fn linux_plan(f: &Facts) -> Plan {
    let kind = match (f.has_dnf, f.has_apt) {
        (false, false) => return Plan::ShowRelease(RELEASE_PAGE_URL.to_string()),
        (true, false) => PkgKind::Rpm,
        (false, true) => PkgKind::Deb,
        (true, true) => f.installed_kind.unwrap_or(PkgKind::Rpm),
    };
    match (f.repo_configured, f.has_pkexec) {
        (true, true) => Plan::RepoUpgrade(repo_command(PKEXEC, kind)),
        (true, false) => Plan::ShowCommand(repo_command("sudo", kind)),
        (false, true) => Plan::Local(kind),
        (false, false) => Plan::ShowCommand(local_command(kind)),
    }
}

/// What the daemon does once a verified update is ready.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutoAction {
    ApplyWhenIdle,
    NotifyReady,
}

/// Auto install exists only on Windows; Linux always waits for the user.
pub fn auto_action(mode: UpdateMode, os: Os) -> AutoAction {
    match (mode, os) {
        (UpdateMode::Auto, Os::Windows) => AutoAction::ApplyWhenIdle,
        _ => AutoAction::NotifyReady,
    }
}

/// No apply while a job runs or the hive is open.
pub fn can_apply(running_jobs: usize, hive_open: bool) -> bool {
    running_jobs == 0 && !hive_open
}

/// `Ok` for root, otherwise the exit code to refuse with.
pub fn root_gate(euid: u32) -> Result<(), i32> {
    if euid == 0 {
        Ok(())
    } else {
        Err(EXIT_NOT_ROOT)
    }
}

/// The effective user id in the text of `/proc/self/status` (the second
/// number of the `Uid:` line).
pub fn effective_uid(status: &str) -> Option<u32> {
    let line = status.lines().find_map(|l| l.strip_prefix("Uid:"))?;
    line.split_whitespace().nth(1)?.parse().ok()
}

/// Runs a program and gives its exit code; `argv[0]` is the program.
pub trait CommandRunner {
    fn run(&self, argv: &[String]) -> Result<i32, UpdateError>;
}

/// The command that asks `pkexec` to run the privileged step.
pub fn privileged_argv(exe: &Path, file: &Path, sha256: &str, kind: PkgKind) -> Vec<String> {
    vec![
        PKEXEC.to_string(),
        exe.to_string_lossy().into_owned(),
        "update".to_string(),
        INSTALL_PRIVILEGED_FLAG.to_string(),
        file.to_string_lossy().into_owned(),
        "--sha256".to_string(),
        sha256.to_string(),
        "--kind".to_string(),
        kind.as_str().to_string(),
    ]
}

/// True when root owns the program and neither the group nor others can
/// write it, so a user cannot swap what `pkexec` runs as root.
pub fn exe_is_trusted(owner_uid: u32, mode: u32) -> bool {
    owner_uid == 0 && mode & 0o022 == 0
}

/// What the trust check reads about the path of the program `pkexec`
/// would run as root: the (owner uid, mode) of the program itself and of
/// every parent directory (closest first). An entry is `None` when that
/// component is not a plain file (or a real directory): a symlink is
/// never trusted, whatever it points at today.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExeStat {
    pub program: Option<(u32, u32)>,
    pub parents: Vec<Option<(u32, u32)>>,
}

/// True when the whole path is trusted: `exe` is absolute (a bare name
/// would be resolved through the caller's `PATH` at exec time), the
/// program is a plain file that passes [`exe_is_trusted`], and every
/// parent directory is a real directory that passes it too. A symlink
/// anywhere in the path, or one writable parent, refuses: between the
/// check and the exec nothing an attacker can write may name the program.
pub fn path_is_trusted(exe: &Path, stat: &ExeStat) -> bool {
    exe.is_absolute()
        && stat
            .program
            .is_some_and(|(uid, mode)| exe_is_trusted(uid, mode))
        && !stat.parents.is_empty()
        && stat
            .parents
            .iter()
            .all(|p| p.is_some_and(|(uid, mode)| exe_is_trusted(uid, mode)))
}

/// The [`ExeStat`] of `exe` from `symlink_metadata` on the program and on
/// every parent directory, so no symlink is ever followed and shows up as
/// `None` (Unix). Anywhere else the stat gives an error and every
/// privileged run is refused. The live input of [`path_is_trusted`],
/// passed by [`SystemRunner`] only; the refusal tests use fakes and never
/// start a program.
pub fn exe_stat(exe: &Path) -> std::io::Result<ExeStat> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let entry = |m: &std::fs::Metadata| (m.uid(), m.mode());
        let meta = std::fs::symlink_metadata(exe)?;
        let program = meta.file_type().is_file().then(|| entry(&meta));
        let mut parents = Vec::new();
        let mut dir = exe.parent();
        while let Some(d) = dir {
            let m = std::fs::symlink_metadata(d)?;
            parents.push(m.file_type().is_dir().then(|| entry(&m)));
            dir = d.parent();
        }
        Ok(ExeStat { program, parents })
    }
    #[cfg(not(unix))]
    {
        let _ = exe;
        Err(std::io::Error::from(std::io::ErrorKind::Unsupported))
    }
}

/// [`privileged_argv`] for an `exe` whose whole path passes
/// [`path_is_trusted`]; any other program (or one that cannot be read) is
/// refused with `Io`.
#[cfg(unix)]
pub fn privileged_argv_checked(
    exe: &Path,
    file: &Path,
    sha256: &str,
    kind: PkgKind,
) -> Result<Vec<String>, UpdateError> {
    let stat = exe_stat(exe).map_err(|_| UpdateError::Io)?;
    if !path_is_trusted(exe, &stat) {
        return Err(UpdateError::Io);
    }
    Ok(privileged_argv(exe, file, sha256, kind))
}

/// The program `pkexec` would run as root: `argv[1]` whenever `argv[0]`
/// is [`PKEXEC`], whatever the rest of the command. The privileged step
/// (`pkexec <exe> update --install-privileged ...`) and a repository
/// upgrade (`pkexec /usr/bin/dnf ...`) alike: every program `pkexec`
/// runs as root goes through the trust check, nothing is exempt.
pub fn pkexec_target(argv: &[String]) -> Option<&Path> {
    match argv {
        [pkexec, exe, ..] if pkexec == PKEXEC => Some(Path::new(exe)),
        _ => None,
    }
}

/// The trust check in front of a command (S-054a, S-067). For every
/// program `pkexec` would run as root, `stat` gives its [`ExeStat`]; the
/// run is refused with `Io`, and `spawn` is never called, when `stat`
/// fails or the path is not [`path_is_trusted`]. Any other command goes
/// to `spawn` without `stat`.
pub fn run_checked(
    argv: &[String],
    stat: &dyn Fn(&Path) -> std::io::Result<ExeStat>,
    spawn: &mut dyn FnMut(&[String]) -> Result<i32, UpdateError>,
) -> Result<i32, UpdateError> {
    if let Some(exe) = pkexec_target(argv) {
        let found = stat(exe).map_err(|_| UpdateError::Io)?;
        if !path_is_trusted(exe, &found) {
            return Err(UpdateError::Io);
        }
    }
    spawn(argv)
}

/// Runs a program and gives its exit code, through [`run_checked`] with
/// the live [`exe_stat`] walk and the real process. pkexec exists only on
/// Unix, so elsewhere the stat gives an error and every privileged run is
/// refused.
pub struct SystemRunner;

impl CommandRunner for SystemRunner {
    fn run(&self, argv: &[String]) -> Result<i32, UpdateError> {
        run_checked(argv, &exe_stat, &mut |argv: &[String]| {
            let (program, args) = argv.split_first().ok_or(UpdateError::Io)?;
            std::process::Command::new(program)
                .args(args)
                .status()
                .map_err(|_| UpdateError::Io)?
                .code()
                .ok_or(UpdateError::Io)
        })
    }
}

/// A Kawaro repository file: `*kawaro*.repo` (any case) or `kawaro.sources`.
pub fn is_repo_file(name: &str) -> bool {
    let name = name.to_lowercase();
    (name.contains("kawaro") && name.ends_with(".repo")) || name == "kawaro.sources"
}

/// `/proc/self/exe` ends in ` (deleted)` after a package replaced the file.
pub fn clean_exe_path(exe: &Path) -> PathBuf {
    let text = exe.to_string_lossy();
    PathBuf::from(text.strip_suffix(" (deleted)").unwrap_or(&text))
}

/// The facts of this machine: the package managers and `pkexec` by absolute
/// path, and a Kawaro repository file in the dnf or apt folders.
pub fn probe_facts(installed_kind: Option<PkgKind>) -> Facts {
    let exists = |p: &str| Path::new(p).exists();
    let repo_configured = ["/etc/yum.repos.d", "/etc/apt/sources.list.d"]
        .iter()
        .filter_map(|d| std::fs::read_dir(d).ok())
        .flatten()
        .flatten()
        .any(|e| is_repo_file(&e.file_name().to_string_lossy()));
    Facts {
        has_dnf: exists("/usr/bin/dnf"),
        has_apt: exists("/usr/bin/apt-get"),
        has_pkexec: exists(PKEXEC),
        repo_configured,
        installed_kind,
    }
}

/// Runs the plan after an explicit "Update now". The only place that builds
/// a `pkexec` command line; the show plans run nothing.
pub fn apply_now(
    plan: &Plan,
    exe: &Path,
    file: &Path,
    sha256: &str,
    runner: &dyn CommandRunner,
) -> Result<(), UpdateError> {
    let argv: Vec<String> = match plan {
        Plan::RepoUpgrade(cmd) => cmd.split_whitespace().map(str::to_string).collect(),
        Plan::Local(kind) => privileged_argv(exe, file, sha256, *kind),
        Plan::ShowCommand(_) | Plan::ShowRelease(_) => return Err(UpdateError::Unavailable),
    };
    match runner.run(&argv)? {
        0 => Ok(()),
        _ => Err(UpdateError::Io),
    }
}

/// The package managers `install_privileged` runs, by absolute path.
#[derive(Clone, Debug)]
pub struct Programs {
    pub dnf: PathBuf,
    pub apt_get: PathBuf,
}

impl Programs {
    /// The system's own programs, never found through `PATH`.
    pub fn system() -> Programs {
        Programs {
            dnf: PathBuf::from("/usr/bin/dnf"),
            apt_get: PathBuf::from("/usr/bin/apt-get"),
        }
    }
}

/// The privileged step: copy `file` into a fresh 0700 folder under
/// `root_dir`, hash the copy against the signed `sha256`, install the copy,
/// and remove the folder on every path. Run as root only (the caller checks).
#[cfg(unix)]
pub fn install_privileged(
    file: &Path,
    sha256: &str,
    kind: PkgKind,
    root_dir: &Path,
    programs: &Programs,
) -> Result<(), UpdateError> {
    use std::os::unix::fs::OpenOptionsExt;
    use std::process::Command;

    let name = file.file_name().ok_or(UpdateError::Io)?;
    let mut src = std::fs::File::open(file).map_err(|_| UpdateError::Io)?;
    let root = std::path::absolute(root_dir).map_err(|_| UpdateError::Io)?;
    let folder = make_folder(&root)?;
    let _cleanup = Cleanup(folder.clone());
    let copy = folder.join(name);
    let mut dst = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&copy)
        .map_err(|_| UpdateError::Io)?;
    std::io::copy(&mut src, &mut dst).map_err(|_| UpdateError::Io)?;
    drop(dst);
    // Hash the root-owned copy, so a swap of the original after the caller's
    // check changes nothing: what is hashed is what is installed.
    if super::flow::sha256_hex(&copy).map_err(|_| UpdateError::Io)? != sha256 {
        return Err(UpdateError::HashMismatch);
    }
    let status = match kind {
        PkgKind::Rpm => Command::new(&programs.dnf),
        PkgKind::Deb => Command::new(&programs.apt_get),
    }
    .args(["install", "-y"])
    .arg(&copy)
    .status()
    .map_err(|_| UpdateError::Io)?;
    if status.success() {
        Ok(())
    } else {
        Err(UpdateError::Io)
    }
}

#[cfg(unix)]
struct Cleanup(PathBuf);

#[cfg(unix)]
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A new `kawaro-update-*` folder with mode 0700 directly under `root`.
/// The name has the process id, the clock and a counter, and `create` fails
/// when it exists, so a planted folder is never reused.
#[cfg(unix)]
fn make_folder(root: &Path) -> Result<PathBuf, UpdateError> {
    use std::os::unix::fs::DirBuilderExt;
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let path = root.join(format!("kawaro-update-{}-{nanos}-{n}", std::process::id()));
    std::fs::DirBuilder::new()
        .mode(0o700)
        .create(&path)
        .map_err(|_| UpdateError::Io)?;
    Ok(path)
}
