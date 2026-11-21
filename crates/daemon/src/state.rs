//! State shared between the socket server, the event pump, the tray and the hive.

use kawaro_core::config::{Config, OutputPlace};
use kawaro_core::i18n::Lang;
use kawaro_core::ipc::{ActionSummary, Event, Response};
use kawaro_core::job_results::{ResultLedger, RetryRefusal as LedgerRefusal};
use kawaro_core::queue::JobId;
use kawaro_core::transport;
use kawaro_core::unsupported::ProbeCache;
use kawaro_core::{ActionDef, Engine, MediaKind, Registry, media};
use kawaro_engines::ocr::OcrEngine;
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
    /// The engine's structured result of the last finished task (S-132).
    pub detail: Option<kawaro_core::engine::TaskDetail>,
    /// Tasks that deliberately wrote nothing (S-133).
    pub skipped: usize,
    /// The first skipped input and its reason, for the notification (S-133).
    pub first_skipped: Option<(PathBuf, kawaro_core::engine::SkipReason)>,
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
    /// What happened to every unit of every job of this session (S-144).
    /// It is filled from the queue events, never from a guessed manifest.
    pub results: ResultLedger,
    /// The parent of every retry child job (S-144).
    pub retry_parents: HashMap<JobId, JobId>,
    /// The parents whose retry is submitted and whose child has not
    /// finished yet. A second retry of the same parent is refused while one
    /// is pending, so a double trigger never starts two child jobs.
    pub pending_retries: HashMap<JobId, JobId>,
    /// The single writer of the named user preset store (S-143-B). The
    /// handle is cheap to clone; the worker starts on the first command and
    /// every store read and write happens there, never under this mutex.
    pub presets: crate::presets::PresetService,
    /// Cached read-only tool probe outcomes, cleared on reload (S-145).
    pub probe_cache: ProbeCache,
    /// System Tesseract handle used for read-only OCR language probes (S-145).
    pub ocr: OcrEngine,
}

/// Why the daemon refuses a retry (S-144).
///
/// The refusal is typed so a later surface can tell a missing action from a
/// changed source from a retry that is already running.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RetryRefusal {
    /// The ledger refuses the retry.
    Ledger(LedgerRefusal),
    /// A retry of this job is already submitted and has not finished.
    AlreadySubmitting,
}

impl RetryRefusal {
    /// The readable reason of the refusal.
    pub fn reason(&self) -> String {
        match self {
            RetryRefusal::Ledger(refusal) => refusal.to_string(),
            RetryRefusal::AlreadySubmitting => {
                String::from("a retry of this job is already running")
            }
        }
    }
}

impl std::fmt::Display for RetryRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.reason())
    }
}

impl From<LedgerRefusal> for RetryRefusal {
    fn from(refusal: LedgerRefusal) -> Self {
        RetryRefusal::Ledger(refusal)
    }
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
        eprintln!("kawarod: ignoring user actions: {e}");
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
            results: ResultLedger::new(),
            retry_parents: HashMap::new(),
            pending_retries: HashMap::new(),
            presets: crate::presets::PresetService::new(kawaro_core::presets::path()),
            probe_cache: ProbeCache::new(),
            ocr: OcrEngine::default(),
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
                "Kawaro".into()
            };
        }
        self.config.apply_quality(&mut action.params);
        action
    }

    /// Re-reads settings and action files, and invalidates cached tool probes (S-145).
    pub fn reload(&mut self) {
        let (config, warning) = Config::load();
        if let Some(w) = warning {
            eprintln!("kawarod: settings: {w}");
        }
        self.lang = config.lang();
        self.config = config;
        self.registry = load_registry();
        self.probe_cache.invalidate_all();
        OcrEngine::invalidate_language_cache();
        self.ocr = OcrEngine::discover();
        self.ocr.warm_language_probe();
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

    /// Why a selection has no actions, or that it has (S-096).
    pub fn classify(&self, files: &[PathBuf]) -> kawaro_core::unsupported::Outcome {
        let ok = |e: &str| self.available.iter().any(|a| a == e);
        kawaro_core::unsupported::classify(files, &self.registry, &ok)
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
