//! S-132 AC3: a structured result detail rides the engine and queue.
//! Contract pinned by this test (tester): kawaro_core::engine::TaskDetail
//! { width, height, source_width, source_height, bytes, downscaled } with
//! Clone, Debug and PartialEq; Engine::result_detail(&self, task) -> Option<TaskDetail>
//! defaulting to None; JobEvent::TaskDone gains detail: Option<TaskDetail>
//! filled by the queue from that method.

use kawaro_core::action::{ActionDef, Localized, OutputSpec};
use kawaro_core::engine::{Engine, EngineError, Task, TaskDetail, detail_line};
use kawaro_core::i18n::Lang;
use kawaro_core::queue::{JobEvent, run_job};
use kawaro_core::{CancelToken, MediaKind, OutputMode, Progress};
use std::sync::mpsc::channel;

struct Fake {
    detail: Option<TaskDetail>,
}

impl Engine for Fake {
    fn id(&self) -> &'static str {
        "fake"
    }
    fn capabilities(&self) -> Vec<(&'static str, Vec<MediaKind>)> {
        Vec::new()
    }
    fn run(
        &self,
        task: &Task,
        progress: Progress,
        cancel: &CancelToken,
    ) -> Result<(), EngineError> {
        progress(0.0);
        cancel.check()?;
        std::fs::write(&task.output, b"out").map_err(EngineError::Io)?;
        progress(1.0);
        Ok(())
    }
    fn result_detail(&self, _task: &Task) -> Option<TaskDetail> {
        self.detail.clone()
    }
}

struct DefaultDetail;

impl Engine for DefaultDetail {
    fn id(&self) -> &'static str {
        "default-detail"
    }
    fn capabilities(&self) -> Vec<(&'static str, Vec<MediaKind>)> {
        Vec::new()
    }
    fn run(
        &self,
        task: &Task,
        progress: Progress,
        cancel: &CancelToken,
    ) -> Result<(), EngineError> {
        progress(0.0);
        cancel.check()?;
        std::fs::write(&task.output, b"out").map_err(EngineError::Io)?;
        progress(1.0);
        Ok(())
    }
    // result_detail is not overridden: the trait default is None.
}

fn action() -> ActionDef {
    ActionDef {
        id: "image.compress-to".to_string(),
        engine: "fake".to_string(),
        op: "compress-to".to_string(),
        ring: 2,
        order: 130,
        accepts: vec![MediaKind::Image],
        skip_ext: Vec::new(),
        min_files: 1,
        max_files: 0,
        output: OutputSpec {
            mode: OutputMode::PerFile,
            ext: "jpg".to_string(),
            suffix: "-small".to_string(),
            dir: String::new(),
        },
        code: Localized {
            ar: "حجم".to_string(),
            en: "SIZE".to_string(),
        },
        label: Localized {
            ar: "ضغط إلى حجم محدد".to_string(),
            en: "Compress to a size".to_string(),
        },
        icon: "compress".to_string(),
        params: toml::from_str("target_bytes = 50000").unwrap(),
        options: None,
    }
}

fn dummy_task(dir: &std::path::Path) -> Task {
    let input = dir.join("in.jpg");
    std::fs::write(&input, b"in").unwrap();
    Task {
        action_id: "image.compress-to".to_string(),
        op: "compress-to".to_string(),
        params: toml::from_str("").unwrap(),
        inputs: vec![input],
        output: dir.join("out.jpg"),
    }
}

fn details(engine: &dyn Engine, task: &Task) -> Vec<Option<TaskDetail>> {
    let (tx, rx) = channel();
    run_job(0, &action(), &task.inputs, engine, &CancelToken::new(), &tx);
    drop(tx);
    rx.into_iter()
        .filter_map(|event| match event {
            JobEvent::TaskDone { detail, .. } => Some(detail),
            _ => None,
        })
        .collect()
}

#[test]
fn ac3_the_engine_detail_reaches_the_task_done_event() {
    let dir = tempfile::tempdir().unwrap();
    let expected = TaskDetail {
        width: 1536,
        height: 1024,
        source_width: 2400,
        source_height: 1600,
        bytes: 4242,
        downscaled: true,
    };
    let engine = Fake {
        detail: Some(expected.clone()),
    };
    let task = dummy_task(dir.path());
    assert_eq!(details(&engine, &task), vec![Some(expected)]);
}

#[test]
fn ac3_the_default_result_detail_is_none() {
    let dir = tempfile::tempdir().unwrap();
    let engine = DefaultDetail;
    let task = dummy_task(dir.path());
    assert_eq!(details(&engine, &task), vec![None]);
}

#[test]
fn ac3_detail_line_is_english() {
    let detail = TaskDetail {
        width: 1536,
        height: 1024,
        source_width: 2400,
        source_height: 1600,
        bytes: 4242,
        downscaled: true,
    };
    let line = detail_line(&detail, Lang::En);
    assert_eq!(
        line, "1536 x 1024, 4242 bytes (downscaled from 2400 x 1600)",
        "the English line the CLI prints and the daemon notification uses"
    );
}

#[test]
fn ac3_detail_line_omits_the_source_size_when_not_downscaled() {
    let detail = TaskDetail {
        width: 2400,
        height: 1600,
        source_width: 2400,
        source_height: 1600,
        bytes: 90_000,
        downscaled: false,
    };
    let line = detail_line(&detail, Lang::En);
    assert_eq!(line, "2400 x 1600, 90000 bytes");
}

#[test]
fn ac3_detail_line_is_localized_in_arabic_without_long_dashes() {
    let detail = TaskDetail {
        width: 1536,
        height: 1024,
        source_width: 2400,
        source_height: 1600,
        bytes: 4242,
        downscaled: true,
    };
    let english = detail_line(&detail, Lang::En);
    let arabic = detail_line(&detail, Lang::Ar);
    assert_ne!(english, arabic, "the daemon localizes the line");
    assert!(
        arabic
            .chars()
            .any(|c| ('\u{0600}'..='\u{06FF}').contains(&c)),
        "the Arabic line carries an Arabic phrase: {arabic}"
    );
    for value in ["1536", "1024", "4242", "2400", "1600"] {
        assert!(arabic.contains(value), "the numbers stay: {arabic}");
    }
    for line in [&english, &arabic] {
        assert!(
            !line.contains('\u{2013}') && !line.contains('\u{2014}'),
            "{line}"
        );
    }
}

#[test]
fn ac3_task_detail_fields_compare_and_debug() {
    let detail = TaskDetail {
        width: 1920,
        height: 1280,
        source_width: 2400,
        source_height: 1600,
        bytes: 90_000,
        downscaled: true,
    };
    let same = TaskDetail {
        downscaled: true,
        ..detail.clone()
    };
    assert_eq!(detail, same);
    let text = format!("{detail:?}");
    assert!(
        text.contains("1920") && text.contains("downscaled"),
        "{text}"
    );
}
