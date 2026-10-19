//! State shared between the socket server, the event pump, the tray and the hive.

use sbarhive_core::i18n::Lang;
use sbarhive_core::ipc::{ActionSummary, Event, Response};
use sbarhive_core::queue::JobId;
use sbarhive_core::{ActionDef, Engine, MediaKind, Registry, media};
use std::collections::HashMap;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

pub struct JobInfo {
    pub label: String,
    pub tasks: usize,
    pub fraction: f32,
    pub ok: usize,
    pub failed: usize,
    pub first_output: Option<PathBuf>,
    pub first_error: Option<String>,
}

pub struct State {
    pub lang: Lang,
    pub registry: Registry,
    pub available: Vec<String>,
    /// Files of the hive currently shown, consumed by the choice.
    pub pending: Option<Vec<PathBuf>>,
    pub jobs: HashMap<JobId, JobInfo>,
    pub subscribers: HashMap<JobId, Vec<UnixStream>>,
}

pub type Shared = Arc<Mutex<State>>;

pub fn lock(s: &Shared) -> MutexGuard<'_, State> {
    s.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn load_registry() -> Registry {
    let mut reg = Registry::builtin();
    if let Some(dir) = user_actions_dir()
        && let Err(e) = reg.add_dir(&dir)
    {
        eprintln!("sbarhived: ignoring user actions: {e}");
    }
    reg
}

fn user_actions_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("sbarhive").join("actions"))
}

impl State {
    pub fn new(lang: Lang, engines: &[Arc<dyn Engine>]) -> Self {
        Self {
            lang,
            registry: load_registry(),
            available: engines
                .iter()
                .filter(|e| e.available())
                .map(|e| e.id().to_string())
                .collect(),
            pending: None,
            jobs: HashMap::new(),
            subscribers: HashMap::new(),
        }
    }

    pub fn matching(&self, files: &[PathBuf]) -> Vec<ActionDef> {
        let paths: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
        let ok = |e: &str| self.available.iter().any(|a| a == e);
        self.registry
            .matching(&paths, &ok)
            .into_iter()
            .cloned()
            .collect()
    }

    pub fn summaries(&self, files: &[PathBuf]) -> Vec<ActionSummary> {
        self.matching(files)
            .into_iter()
            .map(|a| ActionSummary {
                id: a.id.clone(),
                ring: a.ring,
                code: a.code.get(self.lang).to_string(),
                label: a.label.get(self.lang).to_string(),
            })
            .collect()
    }

    /// Sends an event to the clients waiting on `job`, dropping broken ones.
    pub fn publish(&mut self, job: JobId, event: Event) {
        let finished = matches!(event, Event::Finished { .. });
        if let Some(subs) = self.subscribers.get_mut(&job) {
            let line = serde_json::to_string(&Response::Event { event }).unwrap_or_default();
            subs.retain_mut(|s| writeln!(s, "{line}").is_ok());
        }
        if finished {
            self.subscribers.remove(&job);
        }
    }
}

/// The common kind of a selection, or None when it is mixed.
pub fn common_kind(files: &[PathBuf]) -> Option<MediaKind> {
    let mut kinds = files.iter().map(|f| media::detect(f));
    let first = kinds.next()?;
    kinds.all(|k| k == first).then_some(first)
}
