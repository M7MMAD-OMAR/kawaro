//! S-067 (backlog/S-067-review-fixes-update-drop-notify.md): the shared
//! update pieces in core. Written by the tester from the acceptance criteria;
//! the implementer follows this contract and does not edit the tests.
//! Nothing here starts `pkexec` or touches the network.
//!
//! Contract (where the story leaves a signature open, the simplest one):
//!
//! `kawaro_core::update::apply`, all `pub`:
//! - `const INSTALL_PRIVILEGED_FLAG: &str = "--install-privileged"`, used by
//!   `privileged_argv` (its `argv[3]`).
//! - `fn pkexec_target(argv: &[String]) -> Option<&Path>`: `Some(argv[1])`
//!   whenever `argv[0] == PKEXEC` and there is a program word, whatever
//!   the rest of the command (the privileged step and a repository
//!   upgrade alike); `None` for anything else. Every program `pkexec`
//!   would run as root goes through the trust check, nothing is exempt.
//! - `struct ExeStat { program: Option<(u32, u32)>, parents:
//!   Vec<Option<(u32, u32)>> }` (`Debug + Clone + PartialEq + Eq +
//!   Default`): the (owner uid, mode) of the program and of every parent
//!   directory (closest first), `None` where a component is not a plain
//!   file or a real directory. A symlink is never trusted.
//! - `fn path_is_trusted(exe: &Path, stat: &ExeStat) -> bool`: `exe` is
//!   absolute (a bare name would be found through `PATH` at exec time),
//!   the program is `Some` and passes `exe_is_trusted`, and every parent
//!   entry is `Some` and passes `exe_is_trusted`.
//! - `fn exe_stat(exe: &Path) -> std::io::Result<ExeStat>`: the live
//!   `symlink_metadata` walk behind `SystemRunner` (on Unix; elsewhere
//!   the stat gives an error), tested against real files in
//!   `s054b_hardening_unix.rs`.
//! - `struct SystemRunner;` (a unit struct) with `impl CommandRunner`: it
//!   runs `argv[0]` with `argv[1..]` and gives the exit code, and on Unix it
//!   refuses with `UpdateError::Io` when `pkexec_target` is `Some(p)` and `p`
//!   is not `path_is_trusted`. Built on every platform.
//! - `fn run_checked(argv: &[String], stat: &dyn Fn(&Path) ->
//!   std::io::Result<ExeStat>, spawn: &mut dyn FnMut(&[String]) ->
//!   Result<i32, UpdateError>) -> Result<i32, UpdateError>` (story, manager
//!   decision D1): `stat` runs on `pkexec_target(argv)` only when that is
//!   `Some`; a failing `stat` or a path that is not `path_is_trusted` is
//!   `Err(UpdateError::Io)` with no `spawn` call; otherwise `spawn(argv)`.
//!   `SystemRunner::run` passes the live `exe_stat` walk and the real
//!   `Command`. The refusal tests use fakes, so no test here ever starts
//!   `pkexec`.
//! - `fn is_repo_file(name: &str) -> bool`.
//! - `fn clean_exe_path(exe: &Path) -> PathBuf` (moved from the daemon).
//! - `fn probe_facts(installed_kind: Option<PkgKind>) -> Facts`: the live
//!   probe. `has_dnf` is `/usr/bin/dnf` exists, `has_apt` is
//!   `/usr/bin/apt-get` exists, `has_pkexec` is `PKEXEC` exists,
//!   `repo_configured` is any entry of `/etc/yum.repos.d` or
//!   `/etc/apt/sources.list.d` passing `is_repo_file`, and `installed_kind`
//!   is the argument, unchanged.
//!
//! `kawaro_core::update::flow`, all `pub`:
//! - `fn part_path(final_path: &Path) -> PathBuf`: the same folder, the file
//!   name with `.part` appended (`k.rpm` gives `k.rpm.part`).
//! - `fn ready_on_disk(cache_dir: &Path, current: &semver::Version) ->
//!   Option<semver::Version>`: looks in `<cache_dir>/updates/<version>/`.
//!
//! `kawaro_core::update::Channel::parse(text: &str) -> Option<Channel>`:
//! `stable` and `beta` exactly; anything else is `None`.

use std::fs;
use std::path::{Path, PathBuf};

use kawaro_core::update::Channel;
use kawaro_core::update::apply::{
    CommandRunner, Facts, INSTALL_PRIVILEGED_FLAG, PKEXEC, PkgKind, Plan, clean_exe_path,
    is_repo_file, linux_plan, pkexec_target, privileged_argv,
};
#[cfg(unix)]
use kawaro_core::update::apply::ExeStat;
use kawaro_core::update::flow::{part_path, ready_on_disk};
use semver::Version;

fn words(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_string).collect()
}

/// The argv `apply_now` runs for a repository upgrade (`repo_command` is
/// private, so it is read from the plan).
fn repo_argv(kind: PkgKind) -> Vec<String> {
    let facts = Facts {
        has_dnf: kind == PkgKind::Rpm,
        has_apt: kind == PkgKind::Deb,
        has_pkexec: true,
        repo_configured: true,
        installed_kind: Some(kind),
    };
    match linux_plan(&facts) {
        Plan::RepoUpgrade(cmd) => words(&cmd),
        other => panic!("a repository with pkexec is a repo upgrade, got {other:?}"),
    }
}

// ---------------------------------------------------------------- AC1

#[test]
fn ac1_pkexec_target_is_the_program_pkexec_would_run_as_root() {
    let argv = privileged_argv(
        Path::new("/usr/bin/kawaro"),
        Path::new("/c/updates/0.4.0/k.rpm"),
        "ab",
        PkgKind::Rpm,
    );
    assert_eq!(pkexec_target(&argv), Some(Path::new("/usr/bin/kawaro")));

    let rpm = repo_argv(PkgKind::Rpm);
    assert_eq!(rpm[0], PKEXEC);
    assert_eq!(rpm[1], "/usr/bin/dnf", "the manager is an absolute path");
    assert_eq!(
        pkexec_target(&rpm),
        Some(Path::new("/usr/bin/dnf")),
        "pkexec dnf upgrade: {rpm:?}"
    );
    let deb = repo_argv(PkgKind::Deb);
    assert_eq!(deb[1], "/usr/bin/apt-get");
    assert_eq!(
        pkexec_target(&deb),
        Some(Path::new("/usr/bin/apt-get")),
        "pkexec apt-get: {deb:?}"
    );
    // A bare name is still the program pkexec would run, resolved through
    // the caller's PATH at exec time. It must reach the trust check.
    assert_eq!(
        pkexec_target(&words("/usr/bin/pkexec dnf upgrade -y kawaro")),
        Some(Path::new("dnf"))
    );

    // Not pkexec at all, even with the privileged words after it.
    let mut sudo = argv.clone();
    sudo[0] = "/usr/bin/sudo".into();
    assert_eq!(pkexec_target(&sudo), None);
    assert_eq!(
        pkexec_target(&words("/usr/bin/dnf upgrade -y kawaro")),
        None
    );
    // Every word after the program is the command's own business: any
    // argv that starts with pkexec names the program it runs as root,
    // and that program is always trust checked.
    let mut wrong_cmd = argv.clone();
    wrong_cmd[2] = "drop".into();
    assert_eq!(
        pkexec_target(&wrong_cmd),
        Some(Path::new("/usr/bin/kawaro"))
    );
    let mut wrong_flag = argv.clone();
    wrong_flag[3] = "--check".into();
    assert_eq!(
        pkexec_target(&wrong_flag),
        Some(Path::new("/usr/bin/kawaro"))
    );
    assert_eq!(
        pkexec_target(&argv[..3]),
        Some(Path::new("/usr/bin/kawaro"))
    );
    assert_eq!(
        pkexec_target(&argv[..1]),
        None,
        "pkexec alone names nothing"
    );
    assert_eq!(pkexec_target(&[]), None, "empty");
}

#[cfg(unix)]
#[test]
fn ac1_system_runner_runs_a_plain_command_without_checking_its_arguments() {
    use kawaro_core::update::apply::SystemRunner;
    // `dnf` is no file here: only the privileged step's program is checked.
    assert_eq!(
        SystemRunner
            .run(&["/bin/sh".into(), "-c".into(), "exit 3".into()])
            .ok(),
        Some(3)
    );
    assert_eq!(
        SystemRunner.run(&words("/bin/sh -c true dnf")).ok(),
        Some(0),
        "an argument that is no file is never stat'ed"
    );
}

/// The privileged step of a local package, as `apply_now` runs it.
#[cfg(unix)]
fn privileged_step() -> Vec<String> {
    privileged_argv(
        Path::new("/usr/bin/kawaro"),
        Path::new("/c/k.rpm"),
        "ab",
        PkgKind::Rpm,
    )
}

/// The fake stat of a trusted program: a root owned plain file under
/// root owned directories that nobody but root can write.
#[cfg(unix)]
fn trusted() -> ExeStat {
    ExeStat {
        program: Some((0, 0o100755)),
        parents: vec![Some((0, 0o40755)), Some((0, 0o40755))],
    }
}

/// Runs `run_checked` with a fake `stat` giving `stat_result` and a `spawn`
/// that records its calls and starts nothing. Returns the result, the paths
/// `stat` saw and the argv `spawn` saw.
#[cfg(unix)]
fn checked(
    argv: &[String],
    stat_result: std::io::Result<ExeStat>,
) -> (
    Result<i32, kawaro_core::update::UpdateError>,
    Vec<PathBuf>,
    Vec<Vec<String>>,
) {
    use kawaro_core::update::apply::run_checked;
    use std::cell::RefCell;
    let stat_result = RefCell::new(Some(stat_result));
    let stat_calls = RefCell::new(Vec::new());
    let stat = |p: &Path| -> std::io::Result<ExeStat> {
        stat_calls.borrow_mut().push(p.to_path_buf());
        stat_result
            .borrow_mut()
            .take()
            .expect("stat is called at most once")
    };
    let mut spawn_calls: Vec<Vec<String>> = Vec::new();
    let mut spawn = |a: &[String]| {
        spawn_calls.push(a.to_vec());
        Ok(7)
    };
    let result = run_checked(argv, &stat, &mut spawn);
    (result, stat_calls.into_inner(), spawn_calls)
}

#[cfg(unix)]
#[test]
fn ac1_run_checked_refuses_a_privileged_step_whose_program_root_does_not_own() {
    use kawaro_core::update::UpdateError;
    let argv = privileged_step();
    let (result, stats, spawns) = checked(
        &argv,
        Ok(ExeStat {
            program: Some((1000, 0o100755)),
            ..trusted()
        }),
    );
    assert_eq!(result, Err(UpdateError::Io));
    assert_eq!(stats, vec![PathBuf::from("/usr/bin/kawaro")]);
    assert!(spawns.is_empty(), "nothing starts: {spawns:?}");
}

#[cfg(unix)]
#[test]
fn ac1_run_checked_refuses_a_privileged_step_whose_program_is_group_writable() {
    use kawaro_core::update::UpdateError;
    let argv = privileged_step();
    let (result, stats, spawns) = checked(
        &argv,
        Ok(ExeStat {
            program: Some((0, 0o100775)),
            ..trusted()
        }),
    );
    assert_eq!(result, Err(UpdateError::Io));
    assert_eq!(stats, vec![PathBuf::from("/usr/bin/kawaro")]);
    assert!(spawns.is_empty(), "nothing starts: {spawns:?}");
}

#[cfg(unix)]
#[test]
fn ac1_run_checked_refuses_a_privileged_step_whose_program_cannot_be_read() {
    use kawaro_core::update::UpdateError;
    let argv = privileged_step();
    let gone = std::io::Error::from(std::io::ErrorKind::NotFound);
    let (result, stats, spawns) = checked(&argv, Err(gone));
    assert_eq!(result, Err(UpdateError::Io));
    assert_eq!(stats, vec![PathBuf::from("/usr/bin/kawaro")]);
    assert!(spawns.is_empty(), "nothing starts: {spawns:?}");
}

#[cfg(unix)]
#[test]
fn ac1_run_checked_refuses_a_program_that_is_no_plain_file() {
    use kawaro_core::update::UpdateError;
    // The live stat reports a symlink (or a directory, or a fifo) as no
    // program; proved against real files in s054b_hardening_unix.rs.
    // The parent chain is trusted, so only the file kind refuses here.
    let argv = privileged_step();
    let (result, stats, spawns) = checked(
        &argv,
        Ok(ExeStat {
            program: None,
            ..trusted()
        }),
    );
    assert_eq!(result, Err(UpdateError::Io));
    assert_eq!(stats, vec![PathBuf::from("/usr/bin/kawaro")]);
    assert!(spawns.is_empty(), "nothing starts: {spawns:?}");
}

/// A root owned program in a group or world writable directory can be
/// swapped by rename between the check and the exec, so the run is
/// refused. The stat is a fake: a root owned file in a writable directory
/// cannot be created by the test user.
#[cfg(unix)]
#[test]
fn ac1_run_checked_refuses_an_exe_under_a_writable_directory() {
    use kawaro_core::update::UpdateError;
    let argv = privileged_step();
    let stat = ExeStat {
        program: Some((0, 0o100755)),
        parents: vec![Some((0, 0o40777)), Some((0, 0o40755))],
    };
    let (result, stats, spawns) = checked(&argv, Ok(stat));
    assert_eq!(result, Err(UpdateError::Io));
    assert_eq!(stats, vec![PathBuf::from("/usr/bin/kawaro")]);
    assert!(spawns.is_empty(), "nothing starts: {spawns:?}");
}

#[cfg(unix)]
#[test]
fn ac1_run_checked_refuses_an_exe_whose_parent_chain_is_untrusted() {
    use kawaro_core::update::UpdateError;
    let argv = privileged_step();
    for parents in [
        Vec::new(),
        vec![None],
        vec![Some((1000, 0o40755))],
        vec![Some((0, 0o40755)), None],
    ] {
        let stat = ExeStat {
            program: Some((0, 0o100755)),
            parents,
        };
        let (result, stats, spawns) = checked(&argv, Ok(stat.clone()));
        assert_eq!(result, Err(UpdateError::Io), "{stat:?}");
        assert_eq!(stats, vec![PathBuf::from("/usr/bin/kawaro")]);
        assert!(spawns.is_empty(), "nothing starts: {spawns:?}");
    }
}

#[cfg(unix)]
#[test]
fn ac1_run_checked_runs_a_privileged_step_whose_program_root_owns() {
    let argv = privileged_step();
    let (result, stats, spawns) = checked(&argv, Ok(trusted()));
    assert_eq!(result, Ok(7), "the spawn's exit code comes back");
    assert_eq!(stats, vec![PathBuf::from("/usr/bin/kawaro")]);
    assert_eq!(spawns, vec![argv]);
}

#[cfg(unix)]
#[test]
fn ac1_run_checked_checks_the_repository_program_too() {
    use kawaro_core::update::UpdateError;
    for kind in [PkgKind::Rpm, PkgKind::Deb] {
        let argv = repo_argv(kind);
        let name = PathBuf::from(&argv[1]);
        // The manager is not exempt: an untrusted program refuses the run.
        let (result, stats, spawns) = checked(
            &argv,
            Ok(ExeStat {
                program: Some((1000, 0o100755)),
                ..trusted()
            }),
        );
        assert_eq!(result, Err(UpdateError::Io), "untrusted manager: {argv:?}");
        assert_eq!(stats, vec![name.clone()], "the manager is stat'ed");
        assert!(spawns.is_empty(), "nothing starts: {spawns:?}");
        let (result, stats, spawns) = checked(&argv, Ok(trusted()));
        assert_eq!(result, Ok(7), "{argv:?}");
        assert_eq!(stats, vec![name]);
        assert_eq!(spawns, vec![argv]);
    }
}

#[cfg(unix)]
#[test]
fn ac1_run_checked_refuses_a_bare_program_name() {
    use kawaro_core::update::UpdateError;
    // pkexec would resolve `dnf` through the caller's PATH at exec time,
    // so the bare name is refused even when the stat is fully trusted.
    let argv = words("/usr/bin/pkexec dnf upgrade -y kawaro");
    let (result, stats, spawns) = checked(&argv, Ok(trusted()));
    assert_eq!(result, Err(UpdateError::Io), "a bare name is refused");
    assert_eq!(stats, vec![PathBuf::from("dnf")]);
    assert!(spawns.is_empty(), "nothing starts: {spawns:?}");
}

// ---------------------------------------------------------------- AC2

#[test]
fn ac2_is_repo_file_matches_the_kawaro_repository_files_only() {
    for name in ["kawaro.repo", "Kawaro-Beta.REPO", "kawaro.sources"] {
        assert!(is_repo_file(name), "{name} is a Kawaro repository file");
    }
    for name in [
        "kawaro.repo.bak",
        "other.repo",
        "kawaro.list",
        "my-kawaro.sources",
    ] {
        assert!(
            !is_repo_file(name),
            "{name} is not a Kawaro repository file"
        );
    }
}

#[test]
fn ac2_clean_exe_path_strips_only_a_trailing_deleted_mark() {
    assert_eq!(
        clean_exe_path(Path::new("/usr/bin/kawarod (deleted)")),
        PathBuf::from("/usr/bin/kawarod")
    );
    assert_eq!(
        clean_exe_path(Path::new("/usr/bin/kawarod")),
        PathBuf::from("/usr/bin/kawarod")
    );
    assert_eq!(
        clean_exe_path(Path::new("/usr/bin/(deleted)/kawarod")),
        PathBuf::from("/usr/bin/(deleted)/kawarod")
    );
}

#[cfg(unix)]
#[test]
fn ac2_probe_facts_reads_this_machine() {
    use kawaro_core::update::apply::probe_facts;
    let exists = |p: &str| Path::new(p).exists();
    let repo = ["/etc/yum.repos.d", "/etc/apt/sources.list.d"]
        .iter()
        .filter_map(|d| fs::read_dir(d).ok())
        .flatten()
        .flatten()
        .any(|e| is_repo_file(&e.file_name().to_string_lossy()));
    for kind in [Some(PkgKind::Rpm), Some(PkgKind::Deb), None] {
        let f = probe_facts(kind);
        assert_eq!(f.has_dnf, exists("/usr/bin/dnf"));
        assert_eq!(f.has_apt, exists("/usr/bin/apt-get"));
        assert_eq!(f.has_pkexec, exists(PKEXEC));
        assert_eq!(f.repo_configured, repo);
        assert_eq!(f.installed_kind, kind);
    }
}

// ---------------------------------------------------------------- AC3

fn finished(cache: &Path, version: &str, name: &str) {
    let dir = cache.join("updates").join(version);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(name), b"x").unwrap();
}

fn v(text: &str) -> Version {
    Version::parse(text).unwrap()
}

#[test]
fn ac3_part_path_appends_part_to_the_file_name() {
    assert_eq!(
        part_path(Path::new("/c/updates/0.4.0/kawaro-0.4.0-1.x86_64.rpm")),
        PathBuf::from("/c/updates/0.4.0/kawaro-0.4.0-1.x86_64.rpm.part")
    );
    assert_eq!(part_path(Path::new("k.msi")), PathBuf::from("k.msi.part"));
}

#[test]
fn ac3_ready_on_disk_gives_the_highest_finished_version_in_either_order() {
    for order in [["0.5.0", "0.6.0"], ["0.6.0", "0.5.0"]] {
        let t = tempfile::tempdir().unwrap();
        for version in order {
            finished(t.path(), version, &format!("kawaro-{version}.rpm"));
        }
        assert_eq!(
            ready_on_disk(t.path(), &v("0.4.0")),
            Some(v("0.6.0")),
            "created in the order {order:?}"
        );
    }
    // Many folders: the highest wins whatever the directory order.
    let t = tempfile::tempdir().unwrap();
    for minor in [6, 1, 5, 2, 4, 3] {
        finished(t.path(), &format!("0.{minor}.0"), "k.rpm");
    }
    finished(t.path(), "0.6.1-beta.1", "k.rpm");
    assert_eq!(
        ready_on_disk(t.path(), &v("0.0.9")),
        Some(v("0.6.1-beta.1"))
    );
}

#[test]
fn ac3_ready_on_disk_ignores_parts_older_versions_and_other_entries() {
    let t = tempfile::tempdir().unwrap();
    assert_eq!(ready_on_disk(t.path(), &v("0.4.0")), None, "no updates/");
    finished(t.path(), "0.6.0", "x.rpm.part");
    assert_eq!(ready_on_disk(t.path(), &v("0.4.0")), None, "only a .part");
    finished(t.path(), "0.3.0", "old.rpm");
    finished(t.path(), "0.4.0", "same.rpm");
    finished(t.path(), "notes", "x.rpm");
    fs::create_dir_all(t.path().join("updates/0.7.0/sub")).unwrap();
    assert_eq!(
        ready_on_disk(t.path(), &v("0.4.0")),
        None,
        "older, equal, not a version, or a folder only"
    );
    finished(t.path(), "0.6.0", "x.rpm");
    assert_eq!(ready_on_disk(t.path(), &v("0.4.0")), Some(v("0.6.0")));
    assert_eq!(ready_on_disk(t.path(), &v("0.6.0")), None, "not newer");
}

// ---------------------------------------------------------------- AC8

#[test]
fn ac8_channel_parse_knows_stable_and_beta_only() {
    assert_eq!(Channel::parse("stable"), Some(Channel::Stable));
    assert_eq!(Channel::parse("beta"), Some(Channel::Beta));
    for other in ["", "nightly", "Stable", " beta"] {
        assert_eq!(Channel::parse(other), None, "{other:?}");
    }
}

#[test]
fn ac8_install_privileged_flag_is_the_word_privileged_argv_uses() {
    assert_eq!(INSTALL_PRIVILEGED_FLAG, "--install-privileged");
    let argv = privileged_argv(
        Path::new("/usr/bin/kawaro"),
        Path::new("/c/k.deb"),
        "ab",
        PkgKind::Deb,
    );
    assert_eq!(argv[3], INSTALL_PRIVILEGED_FLAG);
}

/// Every runner here is the core one; this guards that the trait stays
/// usable as `&dyn CommandRunner` by the CLI and the daemon.
#[test]
fn ac1_system_runner_is_a_command_runner_object() {
    use kawaro_core::update::apply::SystemRunner;
    let runner: &dyn CommandRunner = &SystemRunner;
    let _ = runner;
}
