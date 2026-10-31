//! `kawaro`: thin client for the daemon, plus in-process conversion.

/// Like `println!`, but a closed stdout (for example `| head`) is not a panic.
macro_rules! out {
    ($($t:tt)*) => {{
        use std::io::Write as _;
        let _ = writeln!(std::io::stdout(), $($t)*);
    }};
}

mod client;
mod clipboard;
mod integration;
mod spawn;
mod update;
mod update_sys;

use clap::{Parser, Subcommand};
use kawaro_core::hold::{Outcome, doctor_line_for};
use kawaro_core::i18n::Lang;
use kawaro_core::ipc::{Event, Request, Response};
use kawaro_core::queue::{self, JobEvent};
use kawaro_core::update::apply::{APPLY_MSI_FLAG, INSTALL_PRIVILEGED_FLAG, PkgKind};
use kawaro_core::{ActionDef, CancelToken, Engine, Registry};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::mpsc;

#[derive(Parser)]
#[command(
    name = "kawaro",
    version,
    about = "Local file conversion from your file manager"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Show the hive for these files (starts the daemon when needed).
    Open {
        /// xdg-activation token or startup id from the caller.
        #[arg(long)]
        token: Option<String>,
        /// Read the files from the clipboard (text/uri-list).
        #[arg(long)]
        from_clipboard: bool,
        /// The pointer position "X,Y" in global logical coordinates, for
        /// example from `hyprctl cursorpos`.
        #[arg(long, value_parser = parse_at, allow_hyphen_values = true)]
        at: Option<(f64, f64)>,
        files: Vec<PathBuf>,
    },
    /// Show the hive waiting for files dragged onto it, for a keyboard
    /// shortcut pressed while dragging (starts the daemon when needed).
    Drop {
        /// xdg-activation token or startup id from the caller.
        #[arg(long)]
        token: Option<String>,
        /// The pointer position "X,Y" in global logical coordinates, for
        /// example from `hyprctl cursorpos`.
        #[arg(long, value_parser = parse_at, allow_hyphen_values = true)]
        at: Option<(f64, f64)>,
        /// The press of a held key: the daemon shows the hive once the key
        /// has been held for `hold_ms` (config.toml, 400 ms by default).
        #[arg(long, conflicts_with = "release")]
        hold: bool,
        /// The release of that key: closes a hive shown by `--hold`, or
        /// cancels it before it shows. Never starts the daemon.
        #[arg(long)]
        release: bool,
        /// Another key went down while the drop key is held (S-102): ends a
        /// press that has not shown yet. Never starts the daemon.
        #[arg(long = "other-key", conflicts_with_all = ["release", "hold"])]
        other_key: bool,
    },
    /// Check for an update, download and verify it, and install it
    /// (`kawaro update --help` lists the options).
    #[command(disable_help_flag = true)]
    Update {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Convert in this process, without the daemon.
    Convert {
        /// Target extension, for example webp.
        #[arg(long)]
        to: String,
        /// Quality 1..100 for lossy formats.
        #[arg(long)]
        quality: Option<i64>,
        #[arg(required = true)]
        files: Vec<PathBuf>,
    },
    /// Run one action by id. Uses the daemon when it runs, otherwise runs here.
    Run {
        action: String,
        /// Wait for the daemon to finish and print the results.
        #[arg(long)]
        wait: bool,
        #[arg(required = true)]
        files: Vec<PathBuf>,
    },
    /// List the actions offered for these files.
    Actions {
        /// Print JSON instead of a table.
        #[arg(long)]
        json: bool,
        #[arg(required = true)]
        files: Vec<PathBuf>,
    },
    /// Open the settings window (starts the daemon when needed). This is
    /// what the app launcher entry runs.
    Settings,
    /// Report which engines and helper tools this system has.
    Doctor {
        /// Hyprland: write or update the Kawaro bind file and remove stale
        /// launchers in `~/.local/bin`.
        #[arg(long)]
        repair: bool,
    },
    /// Add or remove the Explorer right click menu (Windows).
    Integration {
        #[command(subcommand)]
        action: IntegrationCmd,
    },
    /// Start the daemon in the foreground.
    Daemon,
    /// Check that the daemon answers.
    Ping,
    /// Make the daemon re-read settings and action files.
    Reload,
    /// Stop the daemon.
    Quit,
}

#[derive(Subcommand)]
enum IntegrationCmd {
    /// Add the right click menu item for the folder this program is in.
    Install {
        /// Print the reg.exe commands instead of running them.
        #[arg(long)]
        dry_run: bool,
        /// Program folder to register (default: the executable's folder).
        #[arg(long, hide = true)]
        dir: Option<PathBuf>,
    },
    /// Remove the right click menu item.
    Uninstall {
        /// Print the reg.exe commands instead of running them.
        #[arg(long)]
        dry_run: bool,
    },
}

/// The privileged install step that `pkexec` runs (root only), S-054a.
#[derive(Parser)]
#[command(name = "update", bin_name = "kawaro update")]
struct Privileged {
    /// Install this verified package as root.
    #[arg(long, value_name = "FILE")]
    install_privileged: PathBuf,
    /// The signed SHA-256 of the package, lowercase hex.
    #[arg(long)]
    sha256: String,
    /// The package format.
    #[arg(long, value_parser = parse_kind)]
    kind: PkgKind,
}

fn parse_kind(text: &str) -> Result<PkgKind, String> {
    PkgKind::parse(text).ok_or_else(|| "expected rpm or deb".to_string())
}

/// The entry of the detached Windows update helper
/// (`update --apply-msi <msi>`), S-054a AC7.
#[derive(Parser)]
#[command(name = "update", bin_name = "kawaro update")]
struct ApplyMsi {
    /// The downloaded MSI the helper installs with msiexec.
    #[arg(long, value_name = "MSI")]
    apply_msi: PathBuf,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if let Cmd::Integration { action } = cli.cmd {
        return match action {
            IntegrationCmd::Install { dry_run, dir } => integration::install(dry_run, dir),
            IntegrationCmd::Uninstall { dry_run } => integration::uninstall(dry_run),
        };
    }
    if let Cmd::Update { args } = cli.cmd {
        // `--install-privileged FILE` or `--install-privileged=FILE`.
        let privileged = |a: &String| {
            a.strip_prefix(INSTALL_PRIVILEGED_FLAG)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('='))
        };
        if args.iter().any(privileged) {
            let words = std::iter::once("update".to_string()).chain(args);
            let p = Privileged::parse_from(words);
            return update(p.install_privileged, p.sha256, p.kind);
        }
        // `--apply-msi FILE` or `--apply-msi=FILE`: the entry of the
        // detached Windows update helper copy.
        let msi_flag = |a: &String| {
            a.strip_prefix(APPLY_MSI_FLAG)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('='))
        };
        if args.iter().any(msi_flag) {
            let words = std::iter::once("update".to_string()).chain(args);
            let p = ApplyMsi::parse_from(words);
            return apply_msi(p.apply_msi);
        }
        return update_sys::run(&args);
    }
    let result = match cli.cmd {
        Cmd::Open {
            token,
            from_clipboard,
            at,
            files,
        } => open(token, from_clipboard, at, files),
        Cmd::Drop { release: true, .. } => drop_release(),
        Cmd::Drop {
            other_key: true, ..
        } => drop_other_key(),
        Cmd::Drop {
            token, at, hold, ..
        } => drop_hive(token, at, hold),
        Cmd::Convert { to, quality, files } => convert(&to, quality, &files),
        Cmd::Run {
            action,
            wait,
            files,
        } => run(&action, wait, &files),
        Cmd::Actions { json, files } => actions(json, &files),
        Cmd::Settings => client::ensure_daemon()
            .and_then(|()| client::request(&Request::Settings))
            .map(|_| ()),
        Cmd::Doctor { repair: true } => repair(),
        Cmd::Doctor { repair: false } => {
            doctor();
            Ok(())
        }
        Cmd::Daemon => client::exec_daemon(),
        Cmd::Ping => client::request(&Request::Ping).map(|_| out!("pong")),
        Cmd::Reload => client::request(&Request::Reload).map(|_| ()),
        Cmd::Quit => client::request(&Request::Quit).map(|_| ()),
        Cmd::Integration { .. } | Cmd::Update { .. } => unreachable!("handled above"),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            // An empty message means the line was already printed.
            if !e.is_empty() {
                eprintln!("kawaro: {e}");
            }
            ExitCode::FAILURE
        }
    }
}

/// `update --apply-msi <msi>`: the entry of the detached helper copy. It
/// waits for the daemon to exit, runs `msiexec` on the MSI, writes the
/// result and starts a daemon again (`helper_main`), and the exit code
/// reports the outcome. The MSI apply exists on Windows only (S-054a AC7).
fn apply_msi(msi: PathBuf) -> ExitCode {
    #[cfg(windows)]
    {
        use kawaro_core::update::apply::helper_main;
        use kawaro_core::update::result::Outcome;
        let result = helper_main(&msi);
        if result.outcome == Outcome::Installed {
            println!("kawaro: update: installed {}", result.version);
            ExitCode::SUCCESS
        } else {
            eprintln!("kawaro: update: install failed: {:?}", result.outcome);
            ExitCode::FAILURE
        }
    }
    #[cfg(not(windows))]
    {
        let _ = msi;
        eprintln!("kawaro: update: the MSI apply exists on Windows only");
        ExitCode::FAILURE
    }
}

/// `update --install-privileged`: refuses unless root (exit 77, before the
/// file is looked at), then installs the copy of the package.
fn update(file: PathBuf, sha256: String, kind: PkgKind) -> ExitCode {
    #[cfg(unix)]
    {
        use kawaro_core::update::apply::{Programs, effective_uid, install_privileged, root_gate};
        // An unreadable status counts as not root.
        let uid = std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|s| effective_uid(&s))
            .unwrap_or(u32::MAX);
        if let Err(code) = root_gate(uid) {
            eprintln!("kawaro: update --install-privileged must run as root");
            return ExitCode::from(code as u8);
        }
        match install_privileged(
            &file,
            &sha256,
            kind,
            Path::new("/var/tmp"),
            &Programs::system(),
        ) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("kawaro: update: {e}");
                ExitCode::FAILURE
            }
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (file, sha256, kind);
        eprintln!("kawaro: update: the privileged install exists on Linux only");
        ExitCode::FAILURE
    }
}

type Res<T> = Result<T, String>;

fn absolute(files: Vec<PathBuf>) -> Res<Vec<PathBuf>> {
    files
        .into_iter()
        .map(|f| std::path::absolute(&f).map_err(|e| format!("{}: {e}", f.display())))
        .collect()
}

fn existing(files: &[PathBuf]) -> Res<()> {
    match files.iter().find(|f| !f.is_file()) {
        Some(f) => Err(format!("not a file: {}", f.display())),
        None => Ok(()),
    }
}

fn open(
    token: Option<String>,
    from_clipboard: bool,
    at: Option<(f64, f64)>,
    mut files: Vec<PathBuf>,
) -> Res<()> {
    if from_clipboard {
        files.extend(clipboard::read_files()?);
    }
    let files = absolute(files)?;
    if files.is_empty() {
        return Err("no files".into());
    }
    let token = resolve_token(token, &|k| std::env::var(k).ok());
    client::ensure_daemon()?;
    client::request(&Request::Open { files, token, at }).map(|_| ())
}

fn drop_hive(token: Option<String>, at: Option<(f64, f64)>, hold: bool) -> Res<()> {
    let token = resolve_token(token, &|k| std::env::var(k).ok());
    if let Err(e) = client::ensure_daemon() {
        if hold {
            // S-102 AC2: a press that reached no daemon is logged.
            eprintln!("{}", Outcome::DaemonDown.log_line());
        }
        return Err(e);
    }
    client::request(&Request::Drop { token, at, hold }).map(|_| ())
}

/// Another key went down with the drop key held. With no daemon there is no
/// press to end, so that is not an error.
fn drop_other_key() -> Res<()> {
    if client::daemon_up() {
        client::request(&Request::DropOtherKey)?;
    }
    Ok(())
}

/// A key release whose press reached no daemon has nothing to close, so a
/// daemon that does not answer is not an error here.
fn drop_release() -> Res<()> {
    if client::daemon_up() {
        client::request(&Request::DropRelease)?;
    }
    Ok(())
}

/// `X,Y` with optional spaces after the comma, as `hyprctl cursorpos`
/// prints it.
fn parse_at(s: &str) -> Result<(f64, f64), String> {
    let bad = || format!("expected X,Y (two numbers), got {s:?}");
    let (x, y) = s.split_once(',').ok_or_else(bad)?;
    let num = |v: &str| v.trim().parse::<f64>().ok().filter(|n| n.is_finite());
    match (num(x), num(y)) {
        (Some(x), Some(y)) => Ok((x, y)),
        _ => Err(bad()),
    }
}

/// `--token` first, then the variables launchers set: Wayland's
/// XDG_ACTIVATION_TOKEN, then X11's DESKTOP_STARTUP_ID. Empty values are skipped.
fn resolve_token(arg: Option<String>, env: &dyn Fn(&str) -> Option<String>) -> Option<String> {
    arg.into_iter()
        .chain(env("XDG_ACTIVATION_TOKEN"))
        .chain(env("DESKTOP_STARTUP_ID"))
        .find(|t| !t.is_empty())
}

fn engines() -> Vec<Arc<dyn Engine>> {
    kawaro_engines::all()
}

fn engine_for(id: &str) -> Res<Arc<dyn Engine>> {
    engines()
        .into_iter()
        .find(|e| e.id() == id && e.available())
        .ok_or_else(|| format!("engine `{id}` is not available"))
}

fn registry() -> Registry {
    let mut reg = Registry::builtin();
    if let Some(dir) = kawaro_core::config::Config::actions_dir()
        && let Err(e) = reg.add_dir(&dir)
    {
        eprintln!("kawaro: ignoring user actions: {e}");
    }
    reg
}

fn available_ids() -> Vec<String> {
    engines()
        .iter()
        .filter(|e| e.available())
        .map(|e| e.id().to_string())
        .collect()
}

fn matching(reg: &Registry, files: &[PathBuf]) -> Vec<ActionDef> {
    let ids = available_ids();
    let paths: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
    reg.matching(&paths, &|e| ids.iter().any(|i| i == e))
        .into_iter()
        .cloned()
        .collect()
}

/// Runs one action here, printing outputs and errors. Fails when any file failed.
fn run_local(action: &ActionDef, files: &[PathBuf]) -> Res<()> {
    let engine = engine_for(&action.engine)?;
    let (tx, rx) = mpsc::channel();
    let outputs = queue::run_job(0, action, files, engine.as_ref(), &CancelToken::new(), &tx);
    drop(tx);
    let mut failed = 0;
    for ev in rx {
        if let JobEvent::TaskFailed { input, error, .. } = ev {
            eprintln!("failed: {}: {error}", input.display());
            failed += 1;
        }
    }
    for o in &outputs {
        out!("{}", o.display());
    }
    if failed > 0 {
        Err(format!("{failed} file(s) failed"))
    } else {
        Ok(())
    }
}

fn convert(to: &str, quality: Option<i64>, files: &[PathBuf]) -> Res<()> {
    let files = absolute(files.to_vec())?;
    existing(&files)?;
    let to = to.trim_start_matches('.').to_ascii_lowercase();
    let reg = registry();
    let candidates = matching(&reg, &files);
    // A plain format conversion: same extension, no suffix, per file.
    let mut action = candidates
        .iter()
        .find(|a| a.output.ext == to && a.output.suffix.is_empty())
        .cloned()
        .ok_or_else(|| {
            let exts: Vec<&str> = candidates
                .iter()
                .filter(|a| a.output.suffix.is_empty() && !a.output.ext.is_empty())
                .map(|a| a.output.ext.as_str())
                .collect();
            format!(
                "cannot convert these files to `{to}`; available: {}",
                exts.join(", ")
            )
        })?;
    if let Some(q) = quality {
        if !(1..=100).contains(&q) {
            return Err("quality must be between 1 and 100".into());
        }
        action.params.insert("quality".into(), q.into());
    }
    run_local(&action, &files)
}

fn run(id: &str, wait: bool, files: &[PathBuf]) -> Res<()> {
    let files = absolute(files.to_vec())?;
    existing(&files)?;
    let reg = registry();
    let action = reg
        .get(id)
        .ok_or_else(|| format!("unknown action `{id}`"))?
        .clone();
    if !client::daemon_up() {
        return run_local(&action, &files);
    }
    let req = Request::Run {
        action: id.to_string(),
        files,
        wait,
    };
    if !wait {
        return client::request(&req).map(|_| ());
    }
    let mut failed = 0;
    client::stream(&req, |resp| {
        if let Response::Event { event } = resp {
            match event {
                Event::TaskDone { output, .. } => out!("{}", output.display()),
                Event::TaskFailed { input, error, .. } => {
                    eprintln!("failed: {}: {error}", input.display());
                    failed += 1;
                }
                Event::Finished { .. } => return false,
                _ => {}
            }
        }
        true
    })?;
    if failed > 0 {
        Err(format!("{failed} file(s) failed"))
    } else {
        Ok(())
    }
}

fn on_path(tool: &str) -> bool {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d.join(tool).is_file()))
        .unwrap_or(false)
}

fn doctor() {
    let mark = |ok: bool| if ok { "ok     " } else { "missing" };
    out!("engines");
    for e in engines() {
        out!("  {} {}", mark(e.available()), e.id());
    }
    out!("tools");
    use kawaro_engines::locate::Program;
    // The four engine tools resolve through the engines' shared lookup so
    // doctor and the engines never disagree (S-080).
    let engine_tools: [(&str, &str, Program); 4] = [
        ("ffmpeg", "video and audio", Program::Ffmpeg),
        ("ffprobe", "video and audio progress", Program::Ffprobe),
        ("soffice", "documents", Program::Office),
        (
            "gs",
            "PDF compression and pages to images",
            Program::Ghostscript,
        ),
    ];
    for (tool, why, program) in engine_tools {
        let path = kawaro_engines::locate::find(program).map(|p| p.to_string_lossy().into_owned());
        out!(
            "  {} {tool:<10} {why} {}",
            mark(path.is_some()),
            path.as_deref().unwrap_or("")
        );
    }
    // The plain PATH tools probe PATH directly.
    let plain_tools: [(&str, &str); 4] = [
        ("wl-paste", "open from clipboard on Wayland"),
        ("xclip", "open from clipboard on X11"),
        ("xdg-open", "open folder from notifications"),
        ("kawarod", "background service"),
    ];
    for (tool, why) in plain_tools {
        let path = on_path(tool).then(|| tool.to_string());
        out!(
            "  {} {tool:<10} {why} {}",
            mark(path.is_some()),
            path.as_deref().unwrap_or("")
        );
    }
    out!("service");
    let up = client::daemon_up();
    out!(
        "  {} kawarod answering on {}",
        mark(up),
        kawaro_core::transport::endpoint()
    );
    let last = match client::request(&Request::HoldStatus) {
        Ok(Response::Hold { last, ago_ms }) => {
            kawaro_core::hold::last_from_status(last.as_deref(), ago_ms)
        }
        _ => None,
    };
    let binds = read_binds();
    let bind = drop_bind(binds.as_deref());
    out!("{}", doctor_line_for(&bind, up, last));
    if let Some(line) = drop_key_state_line(binds.as_deref(), bind) {
        out!("{line}");
    }
}

/// The `drop key state:` line (S-111), or none when nothing is known.
fn drop_key_state_line(binds: Option<&str>, bind: kawaro_core::hold::BindState) -> Option<String> {
    use kawaro_core::hyprland_bind::{
        StateInput, drop_key_state, fallback_key, foreign_on, message, state_in,
    };
    let denied = kawaro_core::config::Config::path()
        .map(|p| !kawaro_core::config::Config::load_from(&p).0.portal_shortcut)
        .unwrap_or(false);
    let input = StateInput {
        key: "Alt".into(),
        bind,
        foreign: binds.is_some_and(|b| foreign_on(b, "Alt")),
        denied,
        fallback: binds.and_then(fallback_key),
    };
    let lang = Lang::detect();
    drop_key_state(&input)
        .map(|s| format!("{} {}", message(lang, "state.prefix"), state_in(lang, &s)))
}

/// `kawaro doctor --repair` (S-111): Hyprland only. Writes the Kawaro owned
/// bind file, removes stale launchers, reloads once when something changed.
fn repair() -> Res<()> {
    use kawaro_core::hyprland_bind::{
        MergeError, Syntax, foreign_on, hyprland_bind_text, merge_block, message, stale_launchers,
    };
    let lang = Lang::detect();
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none() {
        out!("{}", message(lang, "repair.not_hyprland"));
        return Ok(());
    }
    if read_binds().is_some_and(|b| foreign_on(&b, "Alt")) {
        out!(
            "{}",
            message(lang, "repair.conflict").replace("{key}", "Alt")
        );
        return Err(String::new());
    }
    let dir = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .ok_or("no config folder")?
        .join("hypr");
    let (syntax, name) = if dir.join("hyprland.lua").is_file() {
        (Syntax::Lua, "kawaro.lua")
    } else {
        (Syntax::Hyprlang, "kawaro.conf")
    };
    let file = dir.join(name);
    let bin = if on_path("kawaro") {
        "kawaro"
    } else {
        "/usr/bin/kawaro"
    };
    let block = hyprland_bind_text("Alt", syntax, bin);
    let existing = std::fs::read_to_string(&file).ok();
    let old = existing.clone().unwrap_or_default();
    let merged = merge_block(&old, &block).map_err(|e| match e {
        MergeError::Unbalanced => format!("{}: kawaro markers do not match", file.display()),
        MergeError::Duplicate => format!("{}: more than one kawaro block", file.display()),
    })?;
    if existing.is_some() && merged == old {
        out!("{}", message(lang, "repair.nothing"));
    } else {
        let had_block = old.contains("kawaro:begin");
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let tmp = dir.join(format!(".{name}.tmp"));
        std::fs::write(&tmp, &merged)
            .and_then(|()| std::fs::rename(&tmp, &file))
            .map_err(|e| format!("{}: {e}", file.display()))?;
        let key = if had_block {
            "repair.updated"
        } else {
            "repair.wrote"
        };
        out!("{}", message(lang, key));
        let reloaded = hyprctl().arg("reload").status().is_ok_and(|s| s.success());
        if !reloaded {
            eprintln!("kawaro: hyprctl reload failed");
        }
    }
    // Stale launchers in ~/.local/bin that a packaged install replaces.
    let local = std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/bin"));
    if let Some(local) = local.filter(|d| d.is_dir()) {
        let names: Vec<String> = std::fs::read_dir(&local)
            .map(|r| {
                r.flatten()
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        let listing: Vec<&str> = names.iter().map(String::as_str).collect();
        let installed: Vec<&str> = ["kawaro", "kawarod"]
            .into_iter()
            .filter(|n| installed_outside(n, &local))
            .collect();
        let plan = stale_launchers(&listing, &installed);
        for n in &plan.remove {
            let path = local.join(n);
            std::fs::remove_file(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            out!(
                "{}",
                message(lang, "launcher.removed").replace("{path}", &path.display().to_string())
            );
        }
        if !plan.keep.is_empty() {
            out!("{}", message(lang, "launcher.keep"));
        }
    }
    Ok(())
}

/// A binary named `name` on PATH in a folder other than `skip`. `/usr/bin`
/// counts because it is on PATH.
fn installed_outside(name: &str, skip: &std::path::Path) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|p| std::env::split_paths(&p).any(|d| d != skip && d.join(name).is_file()))
}

/// A `hyprctl` command whose PATH also holds the system folders, so a
/// `hyprctl` that is a shell script finds its tools in a trimmed session.
fn hyprctl() -> std::process::Command {
    let mut paths: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    paths.extend(["/usr/bin", "/bin"].map(PathBuf::from));
    let mut c = std::process::Command::new("hyprctl");
    if let Ok(joined) = std::env::join_paths(paths) {
        c.env("PATH", joined);
    }
    c
}

/// `hyprctl binds -j` output, or `None` when it is absent or failed.
fn read_binds() -> Option<String> {
    hyprctl()
        .args(["binds", "-j"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
}

/// The drop key bind of this Hyprland session
/// (`kawaro_core::hold::bind_state` says what each outcome means).
fn drop_bind(binds: Option<&str>) -> kawaro_core::hold::BindState {
    use kawaro_core::hold::{Hyprctl, bind_state};
    bind_state(binds.map_or(Hyprctl::Failed, Hyprctl::Json))
}

fn actions(json: bool, files: &[PathBuf]) -> Res<()> {
    let files = absolute(files.to_vec())?;
    let lang = Lang::detect();
    let reg = registry();
    let list = matching(&reg, &files);
    if json {
        let v: Vec<_> = list
            .iter()
            .map(|a| {
                serde_json::json!({
                    "id": a.id, "ring": a.ring,
                    "code": a.code.get(lang), "label": a.label.get(lang),
                })
            })
            .collect();
        out!(
            "{}",
            serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?
        );
    } else {
        for a in list {
            out!(
                "{}\t{}\t{}\t{}",
                a.id,
                a.ring,
                a.code.get(lang),
                a.label.get(lang)
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::resolve_token;

    #[test]
    fn token_comes_from_the_flag_then_wayland_then_x11() {
        let env = |vars: &'static [(&'static str, &'static str)]| {
            move |k: &str| {
                vars.iter()
                    .find(|(n, _)| *n == k)
                    .map(|(_, v)| v.to_string())
            }
        };
        let both = env(&[
            ("XDG_ACTIVATION_TOKEN", "wl"),
            ("DESKTOP_STARTUP_ID", "x11"),
        ]);
        assert_eq!(
            resolve_token(Some("flag".into()), &both).as_deref(),
            Some("flag")
        );
        assert_eq!(resolve_token(None, &both).as_deref(), Some("wl"));
        let x11 = env(&[("DESKTOP_STARTUP_ID", "x11")]);
        assert_eq!(resolve_token(None, &x11).as_deref(), Some("x11"));
        let blank = env(&[("XDG_ACTIVATION_TOKEN", ""), ("DESKTOP_STARTUP_ID", "x11")]);
        assert_eq!(
            resolve_token(Some(String::new()), &blank).as_deref(),
            Some("x11")
        );
        assert_eq!(resolve_token(None, &env(&[])), None);
    }
}
