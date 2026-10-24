//! State shared between the socket server, the event pump, the tray and the hive.

use sbarhive_core::config::{Config, OutputPlace};
use sbarhive_core::i18n::Lang;
use sbarhive_core::ipc::{ActionSummary, Event, Response};
use sbarhive_core::queue::JobId;
use sbarhive_core::transport;
use sbarhive_core::{ActionDef, Engine, MediaKind, Registry, media};
use std::collections::HashMap;
use std::io::Write;
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
    pub config: Config,
    pub lang: Lang,
    pub registry: Registry,
    pub available: Vec<String>,
    /// Files of the hive currently shown, consumed by the choice.
    pub pending: Option<Vec<PathBuf>>,
    pub jobs: HashMap<JobId, JobInfo>,
    pub subscribers: HashMap<JobId, Vec<transport::Stream>>,
}

pub type Shared = Arc<Mutex<State>>;

/// Locks ignoring poison, for the state and the daemon's other mutexes.
pub fn lock<T>(s: &Mutex<T>) -> MutexGuard<'_, T> {
    s.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn load_registry() -> Registry {
    let mut reg = Registry::builtin();
    if let Some(dir) = Config::actions_dir()
        && let Err(e) = reg.add_dir(&dir)
    {
        eprintln!("sbarhived: ignoring user actions: {e}");
    }
    reg
}

impl State {
    pub fn new(config: Config, engines: &[Arc<dyn Engine>]) -> Self {
        Self {
            lang: config.lang(),
            config,
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

    /// The action as it should run under the current settings: output
    /// folder and quality preset applied.
    pub fn configured(&self, mut action: ActionDef) -> ActionDef {
        if self.config.output == OutputPlace::Subfolder {
            let name = self.config.output_folder.trim();
            let safe = !name.is_empty() && !name.contains('/') && name != "." && name != "..";
            action.output.dir = if safe {
                name.to_string()
            } else {
                "SbarHive".into()
            };
        }
        self.config.apply_quality(&mut action.params);
        action
    }

    /// Re-reads settings and action files.
    pub fn reload(&mut self) {
        let (config, warning) = Config::load();
        if let Some(w) = warning {
            eprintln!("sbarhived: settings: {w}");
        }
        self.lang = config.lang();
        self.config = config;
        self.registry = load_registry();
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
