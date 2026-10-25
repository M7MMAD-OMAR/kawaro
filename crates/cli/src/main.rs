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

use clap::{Parser, Subcommand};
use kawaro_core::i18n::Lang;
use kawaro_core::ipc::{Event, Request, Response};
use kawaro_core::queue::{self, JobEvent};
use kawaro_core::update::apply::PkgKind;
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
    },
    /// Update Kawaro. For now only the privileged install step that
    /// `pkexec` runs (root only); the rest of the command comes later.
    Update {
        /// Install this verified package as root.
        #[arg(long, value_name = "FILE", requires_all = ["sha256", "kind"])]
        install_privileged: Option<PathBuf>,
        /// The signed SHA-256 of the package, lowercase hex.
        #[arg(long)]
        sha256: Option<String>,
        /// The package format.
        #[arg(long, value_parser = parse_kind)]
        kind: Option<PkgKind>,
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
    Doctor,
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

fn parse_kind(text: &str) -> Result<PkgKind, String> {
    PkgKind::parse(text).ok_or_else(|| "expected rpm or deb".to_string())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if let Cmd::Integration { action } = cli.cmd {
        return match action {
            IntegrationCmd::Install { dry_run, dir } => integration::install(dry_run, dir),
            IntegrationCmd::Uninstall { dry_run } => integration::uninstall(dry_run),
        };
    }
    if let Cmd::Update {
        install_privileged,
        sha256,
        kind,
    } = cli.cmd
    {
        return update(install_privileged, sha256, kind);
    }
    let result = match cli.cmd {
        Cmd::Open {
            token,
            from_clipboard,
            files,
        } => open(token, from_clipboard, files),
        Cmd::Drop { release: true, .. } => drop_release(),
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
        Cmd::Doctor => {
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
            eprintln!("kawaro: {e}");
            ExitCode::FAILURE
        }
    }
}

/// `update --install-privileged`: refuses unless root (exit 77, before the
/// file is looked at), then installs the copy of the package.
fn update(file: Option<PathBuf>, sha256: Option<String>, kind: Option<PkgKind>) -> ExitCode {
    let (Some(file), Some(sha256), Some(kind)) = (file, sha256, kind) else {
        eprintln!("kawaro: update: only --install-privileged is available so far");
        return ExitCode::FAILURE;
    };
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

fn open(token: Option<String>, from_clipboard: bool, mut files: Vec<PathBuf>) -> Res<()> {
    if from_clipboard {
        files.extend(clipboard::read_files()?);
    }
    let files = absolute(files)?;
    if files.is_empty() {
        return Err("no files".into());
    }
    let token = resolve_token(token, &|k| std::env::var(k).ok());
    client::ensure_daemon()?;
    client::request(&Request::Open { files, token }).map(|_| ())
}

fn drop_hive(token: Option<String>, at: Option<(f64, f64)>, hold: bool) -> Res<()> {
    let token = resolve_token(token, &|k| std::env::var(k).ok());
    client::ensure_daemon()?;
    client::request(&Request::Drop { token, at, hold }).map(|_| ())
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
    let tools: [(&str, &str); 8] = [
        ("ffmpeg", "video and audio"),
        ("ffprobe", "video and audio progress"),
        ("soffice", "documents"),
        ("gs", "PDF compression and pages to images"),
        ("wl-paste", "open from clipboard on Wayland"),
        ("xclip", "open from clipboard on X11"),
        ("xdg-open", "open folder from notifications"),
        ("kawarod", "background service"),
    ];
    for (tool, why) in tools {
        out!("  {} {tool:<10} {why}", mark(on_path(tool)));
    }
    out!("service");
    out!(
        "  {} kawarod answering on {}",
        mark(client::daemon_up()),
        kawaro_core::transport::endpoint()
    );
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
