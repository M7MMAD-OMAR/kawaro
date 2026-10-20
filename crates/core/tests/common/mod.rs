//! Fake engine shared by the queue tests.
#![allow(dead_code)]

use sbarhive_core::MediaKind;
use sbarhive_core::action::{ActionDef, Localized, OutputMode, OutputSpec};
use sbarhive_core::engine::{CancelToken, Engine, EngineError, Progress, Task};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Barrier, Mutex};
use std::time::{Duration, Instant};

/// Writes `task.output` with the input names joined by newlines.
/// Inputs whose file name contains `fail_on` fail after writing a partial
/// output; `panic_on` panics after writing. `slow` keeps each task running
/// for that long (polling cancel every few ms) before finishing; it is
/// bounded so a broken cancel fails an assertion instead of hanging.
#[derive(Default)]
pub struct FakeEngine {
    pub id: &'static str,
    pub unavailable: bool,
    pub fail_on: Option<&'static str>,
    pub panic_on: Option<&'static str>,
    pub slow: Option<Duration>,
    pub barrier: Option<Barrier>,
    pub started: Mutex<Option<Sender<PathBuf>>>,
    pub calls: AtomicUsize,
    pub running: AtomicUsize,
    pub max_running: AtomicUsize,
}

impl FakeEngine {
    pub fn new() -> Self {
        FakeEngine {
            id: "fake",
            ..Default::default()
        }
    }
    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

fn matches(path: &Path, pat: Option<&str>) -> bool {
    pat.is_some_and(|p| path.file_name().unwrap().to_str().unwrap().contains(p))
}

impl Engine for FakeEngine {
    fn id(&self) -> &'static str {
        self.id
    }
    fn available(&self) -> bool {
        !self.unavailable
    }
    fn capabilities(&self) -> Vec<(&'static str, Vec<MediaKind>)> {
        vec![("copy", vec![MediaKind::Image])]
    }
    fn run(
        &self,
        task: &Task,
        progress: Progress,
        cancel: &CancelToken,
    ) -> Result<(), EngineError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let now = self.running.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_running.fetch_max(now, Ordering::SeqCst);
        let r = self.work(task, progress, cancel);
        self.running.fetch_sub(1, Ordering::SeqCst);
        r
    }
}

impl FakeEngine {
    fn work(
        &self,
        task: &Task,
        progress: Progress,
        cancel: &CancelToken,
    ) -> Result<(), EngineError> {
        if let Some(tx) = self.started.lock().unwrap().as_ref() {
            let _ = tx.send(task.inputs[0].clone());
        }
        let body: Vec<String> = task
            .inputs
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        std::fs::write(&task.output, body.join("\n"))?;
        if let Some(b) = &self.barrier {
            b.wait();
        }
        if matches(&task.inputs[0], self.panic_on) {
            panic!("fake engine panic");
        }
        if matches(&task.inputs[0], self.fail_on) {
            return Err(EngineError::Failed("fake failure".into()));
        }
        progress(0.5);
        if let Some(d) = self.slow {
            let end = Instant::now() + d;
            while Instant::now() < end {
                cancel.check()?;
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        cancel.check()?;
        progress(1.0);
        Ok(())
    }
}

pub fn action(engine: &str, mode: OutputMode, ext: &str, suffix: &str) -> ActionDef {
    ActionDef {
        id: "test.copy".into(),
        engine: engine.into(),
        op: "copy".into(),
        ring: 1,
        order: 0,
        accepts: vec![MediaKind::Image],
        skip_ext: vec![],
        min_files: 1,
        max_files: 0,
        output: OutputSpec {
            mode,
            ext: ext.into(),
            suffix: suffix.into(),
            dir: String::new(),
        },
        code: Localized {
            ar: "ن".into(),
            en: "N".into(),
        },
        label: Localized {
            ar: "نسخ".into(),
            en: "Copy".into(),
        },
        icon: String::new(),
        params: toml::Table::new(),
    }
}

pub fn files(dir: &Path, names: &[&str]) -> Vec<PathBuf> {
    names
        .iter()
        .map(|n| {
            let p = dir.join(n);
            std::fs::write(&p, b"input").unwrap();
            p
        })
        .collect()
}

/// Sorted file names in `dir`.
pub fn listing(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}
