//! `sbarhive`: thin client for the daemon, plus in-process conversion.

mod client;
mod clipboard;

use clap::{Parser, Subcommand};
use sbarhive_core::i18n::Lang;
use sbarhive_core::ipc::{Event, Request, Response};
use sbarhive_core::queue::{self, JobEvent};
use sbarhive_core::{ActionDef, CancelToken, Engine, Registry};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::mpsc;

#[derive(Parser)]
#[command(
    name = "sbarhive",
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
    /// Report which engines and helper tools this system has.
    Doctor,
    /// Start the daemon in the foreground.
    Daemon,
    /// Check that the daemon answers.
    Ping,
    /// Stop the daemon.
    Quit,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.cmd {
        Cmd::Open {
            token,
            from_clipboard,
            files,
        } => open(token, from_clipboard, files),
        Cmd::Convert { to, quality, files } => convert(&to, quality, &files),
        Cmd::Run {
            action,
            wait,
            files,
        } => run(&action, wait, &files),
        Cmd::Actions { json, files } => actions(json, &files),
        Cmd::Doctor => {
            doctor();
            Ok(())
        }
        Cmd::Daemon => client::exec_daemon(),
        Cmd::Ping => client::request(&Request::Ping).map(|_| println!("pong")),
        Cmd::Quit => client::request(&Request::Quit).map(|_| ()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("sbarhive: {e}");
            ExitCode::FAILURE
        }
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
    let token = token
        .or_else(|| std::env::var("XDG_ACTIVATION_TOKEN").ok())
        .or_else(|| std::env::var("DESKTOP_STARTUP_ID").ok())
        .filter(|t| !t.is_empty());
    client::ensure_daemon()?;
    client::request(&Request::Open { files, token }).map(|_| ())
}

fn engines() -> Vec<Arc<dyn Engine>> {
    sbarhive_engines::all()
}

fn engine_for(id: &str) -> Res<Arc<dyn Engine>> {
    engines()
        .into_iter()
        .find(|e| e.id() == id && e.available())
        .ok_or_else(|| format!("engine `{id}` is not available"))
}

fn registry() -> Registry {
    let mut reg = Registry::builtin();
    if let Some(dir) = user_actions_dir()
        && let Err(e) = reg.add_dir(&dir)
    {
        eprintln!("sbarhive: ignoring user actions: {e}");
    }
    reg
}

fn user_actions_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("sbarhive").join("actions"))
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
        println!("{}", o.display());
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
                Event::TaskDone { output, .. } => println!("{}", output.display()),
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
    println!("engines");
    for e in engines() {
        println!("  {} {}", mark(e.available()), e.id());
    }
    println!("tools");
    let tools: [(&str, &str); 8] = [
        ("ffmpeg", "video and audio"),
        ("ffprobe", "video and audio progress"),
        ("soffice", "documents"),
        ("gs", "PDF compression and pages to images"),
        ("wl-paste", "open from clipboard on Wayland"),
        ("xclip", "open from clipboard on X11"),
        ("xdg-open", "open folder from notifications"),
        ("sbarhived", "background service"),
    ];
    for (tool, why) in tools {
        println!("  {} {tool:<10} {why}", mark(on_path(tool)));
    }
    println!("service");
    println!(
        "  {} sbarhived answering on {}",
        mark(client::daemon_up()),
        sbarhive_core::ipc::socket_path().display()
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
        println!(
            "{}",
            serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?
        );
    } else {
        for a in list {
            println!(
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
