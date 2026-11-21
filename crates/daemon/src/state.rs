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
use kawaro_engines::ocrmypdf::OcrmypdfEngine;
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
    /// The engines this state was built with. A reload refreshes them, so a
    /// tool installed after the daemon started is seen without a restart
    /// (S-145).
    engines: Vec<Arc<dyn Engine>>,
    /// Files of the hive currently shown, consumed by the choice.
    pub pending: Option<Vec<PathBuf>>,
    pub jobs: HashMap<JobId, JobInfo>,
    pub subscribers: HashMap<JobId, Vec<transport::Stream>>,
    /// What happened to every unit of every job of this session (S-144).
    /// It is filled from the queue events, never from a guessed manifest.
    pub results: ResultLedger,
    /// The parent of every retry child job (S-144).
    pub retry_parents: HashMap<JobId, JobId>,
    /// The parents whose retry is prepared or submitted and whose child has
    /// not finished yet. A second retry of the same parent is refused while
    /// one is pending, so a double trigger never starts two child jobs.
    ///
    /// `None` is a claim: the retry is prepared and its child is not known
    /// yet. `Some(child)` is a submitted retry, and the child job is the one
    /// that ends the claim (S-151).
    pub pending_retries: HashMap<JobId, Option<JobId>>,
    /// The single writer of the named user preset store (S-143-B). The
    /// handle is cheap to clone; the worker starts on the first command and
    /// every store read and write happens there, never under this mutex.
    pub presets: crate::presets::PresetService,
    /// Cached read-only tool probe outcomes, cleared on reload (S-145).
    pub probe_cache: ProbeCache,
    /// System Tesseract handle used for read-only OCR language probes (S-145).
    ///
    /// It is re-created on every reload with the same lookup the engine
    /// refresh does, so the probe cache is keyed by the path of the engine
    /// the queue runs (S-151).
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
        let ocr = OcrEngine::default();
        let available = available_ids(engines);
        Self {
            lang: config.lang(),
            config,
            registry: load_registry(),
            available,
            engines: engines.to_vec(),
            pending: None,
            jobs: HashMap::new(),
            subscribers: HashMap::new(),
            results: ResultLedger::new(),
            retry_parents: HashMap::new(),
            pending_retries: HashMap::new(),
            presets: crate::presets::PresetService::new(kawaro_core::presets::path()),
            probe_cache: ProbeCache::new(),
            ocr,
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

    /// Re-reads settings and action files, refreshes the engines, and
    /// invalidates cached tool probes (S-145).
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
        // S-145: every engine looks its program up again, so a tool
        // installed after the daemon started is seen. The lookup starts no
        // child process and uses no network.
        for engine in &self.engines {
            engine.refresh();
        }
        // S-151: the availability comes from the engines themselves, after
        // their refresh, so the hive offers exactly what the queue can run.
        self.available = available_ids(&self.engines);
        // The OCRmyPDF version probe is warmed after the refresh: a tool
        // below the version floor is then shown as unavailable with its
        // reason, instead of being offered and failing (S-151).
        OcrmypdfEngine::invalidate_version_cache();
        OcrmypdfEngine::default().warm_version_probe();
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

/// The ids of the available engines, in the order of the engine list.
///
/// Every engine answers for itself, after its own refresh: the daemon
/// keeps no second copy of an engine, so the hive never offers an action
/// the queue cannot run (S-151).
fn available_ids(engines: &[Arc<dyn Engine>]) -> Vec<String> {
    engines
        .iter()
        .filter(|engine| engine.available())
        .map(|engine| engine.id().to_string())
        .collect()
}

/// The common kind of a selection, or None when it is mixed.
pub fn common_kind(files: &[PathBuf]) -> Option<MediaKind> {
    let mut kinds = files.iter().map(|f| media::detect(f));
    let first = kinds.next()?;
    kinds.all(|k| k == first).then_some(first)
}

#[cfg(test)]
mod s151_tests {
    use super::*;
    use kawaro_core::engine::{CancelToken, EngineError, Estimate, Progress, Task, TaskOutcome};
    use std::sync::atomic::{AtomicBool, Ordering};

    /// A fake engine whose tool is gone after its refresh (S-151).
    struct FakeEngine {
        id: &'static str,
        ready: AtomicBool,
    }

    impl FakeEngine {
        fn engine(id: &'static str) -> Arc<dyn Engine> {
            Arc::new(FakeEngine {
                id,
                ready: AtomicBool::new(true),
            })
        }
    }

    impl Engine for FakeEngine {
        fn id(&self) -> &'static str {
            self.id
        }

        fn available(&self) -> bool {
            self.ready.load(Ordering::Acquire)
        }

        fn refresh(&self) {
            self.ready.store(false, Ordering::Release);
        }

        fn capabilities(&self) -> Vec<(&'static str, Vec<MediaKind>)> {
            Vec::new()
        }

        fn estimate(&self, _task: &Task) -> Estimate {
            Estimate::default()
        }

        fn run(
            &self,
            _task: &Task,
            _progress: Progress,
            _cancel: &CancelToken,
        ) -> Result<TaskOutcome, EngineError> {
            Err(EngineError::Failed(String::from(
                "the fake engine runs nothing",
            )))
        }
    }

    /// S-151: the availability comes from the engines themselves, after
    /// their refresh. The daemon keeps no second copy of the OCR engine,
    /// so a refresh that loses the tool is seen at once.
    #[test]
    fn the_availability_follows_the_engines_after_their_refresh() {
        let engines: Vec<Arc<dyn Engine>> =
            vec![FakeEngine::engine("ocr"), FakeEngine::engine("fake")];
        let mut st = State::new(Config::default(), &engines);
        let ids: Vec<&str> = st.available.iter().map(String::as_str).collect();
        assert_eq!(ids, vec!["ocr", "fake"]);
        // The refresh looks the programs up again and finds nothing.
        st.reload();
        assert!(st.available.is_empty(), "{:?}", st.available);
    }
}
