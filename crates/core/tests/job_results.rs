//! S-144: the platform-free job result ledger and the safe retry proposal.
//!
//! These cases are source only. They are never executed in this pass.

mod common;

use common::action;
use kawaro_core::ActionDef;
use kawaro_core::action::OutputMode::{Combined, PerFile};
use kawaro_core::engine::SkipReason;
use kawaro_core::job_report::{JobDetail, JobState, JobSummary, UnitState};
use kawaro_core::job_results::{
    ActionSnapshot, InputIdentity, JobManifest, JobStatus, ResultLedger, RetryRefusal, TaskUnitId,
    TaskUnitManifest, UnitOutcome,
};
use kawaro_core::options::{Field, Kind, OptionsKind, OptionsSpec};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const A: &[&str] = &["a.png"];
const B: &[&str] = &["b.png"];
const C: &[&str] = &["c.png"];
const AB: &[&str] = &["a.png", "b.png"];
const ABC: &[&str] = &["a.png", "b.png", "c.png"];

fn stamp() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1)
}

fn identity(path: &str, len: u64) -> InputIdentity {
    InputIdentity {
        path: PathBuf::from(path),
        len,
        modified: Some(stamp()),
        regular_file: true,
    }
}

/// A manifest with the given units, without touching the file system.
///
/// The units follow the production planner: a per-file action has one
/// unit per file, and a combined action has one unit holding the whole
/// ordered group. A group of a per-file action is therefore split into one
/// unit per path, the way `plan` splits the selection of a per-file
/// action.
fn manifest(act: &ActionDef, groups: &[&[&str]]) -> JobManifest {
    let combined = matches!(act.output.mode, Combined);
    let mut units: Vec<TaskUnitManifest> = Vec::new();
    for group in groups {
        if combined {
            let inputs = group.iter().map(|p| identity(p, 10)).collect();
            let id = units.len() as TaskUnitId;
            units.push(TaskUnitManifest { id, inputs });
        } else {
            for path in group.iter() {
                let id = units.len() as TaskUnitId;
                units.push(TaskUnitManifest {
                    id,
                    inputs: vec![identity(path, 10)],
                });
            }
        }
    }
    JobManifest {
        action: act.clone(),
        snapshot: ActionSnapshot::of(act),
        units,
    }
}

/// The identity of a source that did not change.
fn fresh(path: &Path) -> Option<InputIdentity> {
    Some(InputIdentity {
        path: path.to_path_buf(),
        len: 10,
        modified: Some(stamp()),
        regular_file: true,
    })
}

fn failed() -> UnitOutcome {
    UnitOutcome::Failed {
        error: String::from("boom"),
    }
}

fn ok() -> UnitOutcome {
    UnitOutcome::Success {
        output: Some(PathBuf::from("out.webp")),
        detail: None,
    }
}

/// An action with the given typed options and no params.
fn with_options(fields: Vec<Field>) -> ActionDef {
    let mut act = action("fake", PerFile, "webp", "");
    act.options = Some(OptionsSpec {
        kind: OptionsKind::Form,
        readout: vec![],
        remember: false,
        suffix_template: String::new(),
        fields,
        presets: vec![],
    });
    act
}

/// One option field of a kind.
fn field(id: &str, kind: Kind, clears: &[&str]) -> Field {
    Field {
        id: String::from(id),
        kind,
        label: kawaro_core::action::Localized {
            ar: String::from("ع"),
            en: String::from(id),
        },
        default: None,
        min: None,
        max: None,
        step: None,
        unit: None,
        values: vec![],
        maps_to: None,
        clears: clears.iter().map(|c| String::from(*c)).collect(),
        no_enlarge: false,
    }
}

// ---------- retention ----------

#[test]
fn eviction_removes_the_oldest_whole_job() {
    let act = action("fake", PerFile, "webp", "");
    let mut ledger = ResultLedger::new();
    for job in 1..=101u64 {
        ledger.start(job, manifest(&act, &[AB]));
        ledger.record_unit(job, 0, ok(), Duration::from_millis(1));
        ledger.record_unit(job, 1, ok(), Duration::from_millis(1));
        ledger.finish(job, Duration::from_millis(2), false);
    }
    assert_eq!(ledger.completed_jobs_count(), 100);
    // The oldest job leaves whole, never a part of it.
    assert!(ledger.job(1).is_none());
    assert!(ledger.job(2).is_some());
    assert_eq!(ledger.completed_units(), 200);
}

#[test]
fn the_unit_cap_evicts_whole_jobs() {
    let act = action("fake", PerFile, "webp", "");
    let groups: Vec<&[&str]> = (0..400).map(|_| A).collect();
    let mut ledger = ResultLedger::new();
    for job in 1..=3u64 {
        ledger.start(job, manifest(&act, &groups));
        for unit in 0..400u64 {
            ledger.record_unit(job, unit, ok(), Duration::from_millis(1));
        }
        ledger.finish(job, Duration::from_millis(2), false);
    }
    // 1200 units do not fit, so the oldest job leaves whole.
    assert_eq!(ledger.completed_units(), 800);
    assert!(ledger.job(1).is_none());
    assert!(ledger.job(2).is_some());
    assert!(ledger.job(3).is_some());
}

#[test]
fn active_jobs_are_exempt_from_the_limits() {
    let act = action("fake", PerFile, "webp", "");
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&act, &[AB]));
    for job in 2..=102u64 {
        ledger.start(job, manifest(&act, &[A]));
        ledger.record_unit(job, 0, ok(), Duration::from_millis(1));
        ledger.finish(job, Duration::from_millis(2), false);
    }
    assert_eq!(ledger.completed_jobs_count(), 100);
    let active = ledger.job(1).expect("the active job stays");
    assert_eq!(active.status, kawaro_core::job_results::JobStatus::Active);
    assert_eq!(active.units.len(), 0);
}

#[test]
fn an_oversized_job_keeps_a_summary_only() {
    let act = action("fake", PerFile, "webp", "");
    let groups: Vec<&[&str]> = (0..1001).map(|_| A).collect();
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&act, &groups));
    ledger.record_unit(1, 0, failed(), Duration::from_millis(1));
    ledger.finish(1, Duration::from_millis(2), false);
    let record = ledger.job(1).expect("the job stays");
    assert!(record.summary_only);
    assert_eq!(record.units.len(), 0);
    assert!(record.manifest.is_none());
    assert_eq!(record.counts.total, 1001);
    assert_eq!(record.counts.failed, 1);
    // No unit error text is kept for a summary only job.
    assert!(!format!("{ledger:?}").contains("boom"));
    // And such a job is never retried.
    let refusal = ledger
        .retry_plan(1, Some(&act), &|_: &str| true, &fresh)
        .expect_err("no retry");
    assert_eq!(refusal, RetryRefusal::SummaryOnly);
}

#[test]
fn clearing_the_history_keeps_the_active_jobs() {
    let act = action("fake", PerFile, "webp", "");
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&act, &[A]));
    ledger.record_unit(1, 0, ok(), Duration::from_millis(1));
    ledger.finish(1, Duration::from_millis(2), false);
    ledger.start(2, manifest(&act, &[A]));
    ledger.clear_completed();
    assert!(ledger.job(1).is_none());
    assert!(ledger.job(2).is_some());
}

// ---------- the unit order ----------

#[test]
fn the_next_unit_follows_the_manifest_order() {
    let act = action("fake", PerFile, "webp", "");
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&act, &[A, B, C]));
    assert_eq!(ledger.next_unit(1), Some(0));
    ledger.record_unit(1, 0, ok(), Duration::from_millis(1));
    assert_eq!(ledger.next_unit(1), Some(1));
    ledger.record_unit(1, 1, failed(), Duration::from_millis(1));
    assert_eq!(ledger.next_unit(1), Some(2));
    ledger.record_unit(1, 2, ok(), Duration::from_millis(1));
    assert_eq!(ledger.next_unit(1), None);
}

// ---------- the retry proposal ----------

#[test]
fn a_retry_offers_only_the_failed_units() {
    let act = action("fake", PerFile, "webp", "");
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&act, &[A, B, C]));
    ledger.record_unit(1, 0, ok(), Duration::from_millis(1));
    ledger.record_unit(1, 1, failed(), Duration::from_millis(1));
    ledger.record_unit(
        1,
        2,
        UnitOutcome::Skipped {
            reason: SkipReason::AlreadyOptimized,
        },
        Duration::from_millis(1),
    );
    ledger.finish(1, Duration::from_millis(3), false);
    let proposal = ledger
        .retry_plan(1, Some(&act), &|_: &str| true, &fresh)
        .expect("a retry");
    assert_eq!(proposal.parent_job, 1);
    assert_eq!(proposal.inputs, [PathBuf::from("b.png")]);
    assert_eq!(proposal.original_units, [1]);
}

#[test]
fn a_combined_retry_keeps_the_whole_ordered_group() {
    let act = action("fake", Combined, "pdf", "");
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&act, &[ABC]));
    ledger.record_unit(1, 0, failed(), Duration::from_millis(1));
    ledger.finish(1, Duration::from_millis(2), false);
    let proposal = ledger
        .retry_plan(1, Some(&act), &|_: &str| true, &fresh)
        .expect("a retry");
    assert_eq!(
        proposal.inputs,
        [
            PathBuf::from("a.png"),
            PathBuf::from("b.png"),
            PathBuf::from("c.png")
        ]
    );
    assert_eq!(proposal.original_units, [0]);
}

#[test]
fn a_retry_keeps_the_action_as_it_ran() {
    let mut act = action("fake", PerFile, "webp", "");
    act.params
        .insert(String::from("quality"), toml::Value::Integer(60));
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&act, &[A]));
    ledger.record_unit(1, 0, failed(), Duration::from_millis(1));
    ledger.finish(1, Duration::from_millis(2), false);
    let proposal = ledger
        .retry_plan(1, Some(&act), &|_: &str| true, &fresh)
        .expect("a retry");
    assert_eq!(
        proposal
            .action
            .params
            .get("quality")
            .and_then(toml::Value::as_integer),
        Some(60)
    );
}

#[test]
fn a_changed_source_blocks_the_retry() {
    let act = action("fake", PerFile, "webp", "");
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&act, &[A]));
    ledger.record_unit(1, 0, failed(), Duration::from_millis(1));
    ledger.finish(1, Duration::from_millis(2), false);
    let changed = |_: &Path| {
        Some(InputIdentity {
            path: PathBuf::from("a.png"),
            len: 11,
            modified: Some(stamp()),
            regular_file: true,
        })
    };
    let refusal = ledger
        .retry_plan(1, Some(&act), &|_: &str| true, &changed)
        .expect_err("no retry");
    assert_eq!(refusal, RetryRefusal::InputChanged(String::from("a.png")));
}

#[test]
fn a_missing_source_blocks_the_retry() {
    let act = action("fake", PerFile, "webp", "");
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&act, &[A]));
    ledger.record_unit(1, 0, failed(), Duration::from_millis(1));
    ledger.finish(1, Duration::from_millis(2), false);
    let gone = |_: &Path| -> Option<InputIdentity> { None };
    let refusal = ledger
        .retry_plan(1, Some(&act), &|_: &str| true, &gone)
        .expect_err("no retry");
    assert_eq!(
        refusal,
        RetryRefusal::InputUnavailable(String::from("a.png"))
    );
}

#[test]
fn a_changed_action_blocks_the_retry() {
    let act = action("fake", PerFile, "webp", "");
    let mut other = action("fake", PerFile, "webp", "");
    other.op = String::from("convert");
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&act, &[A]));
    ledger.record_unit(1, 0, failed(), Duration::from_millis(1));
    ledger.finish(1, Duration::from_millis(2), false);
    let refusal = ledger
        .retry_plan(1, Some(&other), &|_: &str| true, &fresh)
        .expect_err("no retry");
    assert!(
        matches!(refusal, RetryRefusal::ActionChanged(_)),
        "{refusal:?}"
    );
}

#[test]
fn a_missing_action_blocks_the_retry() {
    let act = action("fake", PerFile, "webp", "");
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&act, &[A]));
    ledger.record_unit(1, 0, failed(), Duration::from_millis(1));
    ledger.finish(1, Duration::from_millis(2), false);
    let refusal = ledger
        .retry_plan(1, None, &|_: &str| true, &fresh)
        .expect_err("no retry");
    assert_eq!(
        refusal,
        RetryRefusal::ActionMissing(String::from("test.copy"))
    );
}

#[test]
fn an_unavailable_engine_blocks_the_retry() {
    let act = action("fake", PerFile, "webp", "");
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&act, &[A]));
    ledger.record_unit(1, 0, failed(), Duration::from_millis(1));
    ledger.finish(1, Duration::from_millis(2), false);
    let refusal = ledger
        .retry_plan(1, Some(&act), &|_: &str| false, &fresh)
        .expect_err("no retry");
    assert_eq!(
        refusal,
        RetryRefusal::EngineUnavailable(String::from("fake"))
    );
}

#[test]
fn a_cancelled_unit_is_not_retried() {
    let act = action("fake", PerFile, "webp", "");
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&act, &[A, B]));
    ledger.record_unit(1, 0, UnitOutcome::Cancelled, Duration::from_millis(1));
    ledger.finish(1, Duration::from_millis(2), true);
    let refusal = ledger
        .retry_plan(1, Some(&act), &|_: &str| true, &fresh)
        .expect_err("no retry");
    assert_eq!(refusal, RetryRefusal::NoEligibleFailures);
}

#[test]
fn a_running_job_is_not_retried() {
    let act = action("fake", PerFile, "webp", "");
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&act, &[A]));
    ledger.record_unit(1, 0, failed(), Duration::from_millis(1));
    let refusal = ledger
        .retry_plan(1, Some(&act), &|_: &str| true, &fresh)
        .expect_err("no retry");
    assert_eq!(refusal, RetryRefusal::NotCompleted);
}

#[test]
fn a_job_without_a_failed_unit_is_not_retried() {
    let act = action("fake", PerFile, "webp", "");
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&act, &[A]));
    ledger.record_unit(1, 0, ok(), Duration::from_millis(1));
    ledger.finish(1, Duration::from_millis(2), false);
    let refusal = ledger
        .retry_plan(1, Some(&act), &|_: &str| true, &fresh)
        .expect_err("no retry");
    assert_eq!(refusal, RetryRefusal::NoEligibleFailures);
}

// ---------- the record validation ----------

#[test]
fn a_duplicate_unit_is_refused() {
    let act = action("fake", PerFile, "webp", "");
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&act, &[A]));
    assert!(ledger.record_unit(1, 0, ok(), Duration::from_millis(1)));
    // The same unit never counts twice.
    assert!(!ledger.record_unit(1, 0, failed(), Duration::from_millis(1)));
    ledger.finish(1, Duration::from_millis(2), false);
    let record = ledger.job(1).expect("the job stays");
    assert_eq!(record.counts.succeeded, 1);
    assert_eq!(record.counts.failed, 0);
    assert_eq!(record.units.len(), 1);
}

#[test]
fn a_unit_outside_the_manifest_is_refused() {
    let act = action("fake", PerFile, "webp", "");
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&act, &[A]));
    assert!(!ledger.record_unit(1, 5, ok(), Duration::from_millis(1)));
    let record = ledger.job(1).expect("the job stays");
    assert_eq!(record.counts.succeeded, 0);
    assert!(record.diagnostic.is_some(), "{record:?}");
}

#[test]
fn the_cancelled_units_keep_their_own_ids() {
    let act = action("fake", PerFile, "webp", "");
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&act, &[ABC]));
    // The outcome of the last unit arrived first.
    ledger.record_unit(1, 2, ok(), Duration::from_millis(1));
    ledger.finish(1, Duration::from_millis(2), true);
    let record = ledger.job(1).expect("the job stays");
    assert_eq!(record.status, JobStatus::Completed);
    assert_eq!(record.counts.cancelled, 2);
    let mut ids: Vec<TaskUnitId> = record.units.iter().map(|u| u.unit).collect();
    ids.sort();
    assert_eq!(ids, [0, 1, 2]);
    assert!(matches!(
        record.units[0].outcome,
        UnitOutcome::Success { .. }
    ));
    // The unit that never reported is cancelled, with its own id.
    let cancelled: Vec<TaskUnitId> = record
        .units
        .iter()
        .filter(|u| matches!(u.outcome, UnitOutcome::Cancelled))
        .map(|u| u.unit)
        .collect();
    assert_eq!(cancelled, [0, 1]);
}

#[test]
fn a_job_with_a_missing_outcome_is_inconsistent() {
    let act = action("fake", PerFile, "webp", "");
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&act, &[AB]));
    ledger.record_unit(1, 0, ok(), Duration::from_millis(1));
    ledger.finish(1, Duration::from_millis(2), false);
    let record = ledger.job(1).expect("the job stays");
    assert_eq!(record.status, JobStatus::Inconsistent);
    assert!(record.diagnostic.is_some(), "{record:?}");
    // Such a job is never retried.
    let refusal = ledger
        .retry_plan(1, Some(&act), &|_: &str| true, &fresh)
        .expect_err("no retry");
    assert_eq!(refusal, RetryRefusal::Inconsistent);
}

// ---------- the retry proposal, again ----------

#[test]
fn a_retry_keeps_the_order_of_the_original_manifest() {
    let act = action("fake", PerFile, "webp", "");
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&act, &[ABC]));
    // The outcomes arrived in another order than the manifest.
    ledger.record_unit(1, 2, failed(), Duration::from_millis(1));
    ledger.record_unit(1, 0, failed(), Duration::from_millis(1));
    ledger.record_unit(1, 1, ok(), Duration::from_millis(1));
    ledger.finish(1, Duration::from_millis(3), false);
    let proposal = ledger
        .retry_plan(1, Some(&act), &|_: &str| true, &fresh)
        .expect("a retry");
    assert_eq!(
        proposal.inputs,
        [PathBuf::from("a.png"), PathBuf::from("c.png")]
    );
    assert_eq!(proposal.original_units, [0, 2]);
}

#[test]
fn another_action_id_blocks_the_retry() {
    let act = action("fake", PerFile, "webp", "");
    let mut other = act.clone();
    other.id = String::from("test.other");
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&act, &[A]));
    ledger.record_unit(1, 0, failed(), Duration::from_millis(1));
    ledger.finish(1, Duration::from_millis(2), false);
    let refusal = ledger
        .retry_plan(1, Some(&other), &|_: &str| true, &fresh)
        .expect_err("no retry");
    assert!(
        matches!(refusal, RetryRefusal::ActionChanged(_)),
        "{refusal:?}"
    );
}

// ---------- the stored options ----------

#[test]
fn the_whole_override_table_is_validated_together() {
    let mut scale = field("scale", Kind::Percent, &["max_side"]);
    scale.min = Some(1.0);
    scale.max = Some(1000.0);
    let mut max_side = field("max_side", Kind::Px, &["scale"]);
    max_side.min = Some(16.0);
    max_side.max = Some(20000.0);
    let mut current = with_options(vec![scale, max_side]);
    current
        .params
        .insert(String::from("scale"), toml::Value::Integer(50));
    // The run activated both options at once.
    let mut stored = current.clone();
    stored
        .params
        .insert(String::from("scale"), toml::Value::Integer(25));
    stored
        .params
        .insert(String::from("max_side"), toml::Value::Integer(1920));
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&stored, &[A]));
    ledger.record_unit(1, 0, failed(), Duration::from_millis(1));
    ledger.finish(1, Duration::from_millis(2), false);
    let refusal = ledger
        .retry_plan(1, Some(&current), &|_: &str| true, &fresh)
        .expect_err("no retry");
    assert!(
        matches!(refusal, RetryRefusal::InvalidOptions(_)),
        "{refusal:?}"
    );
}

#[test]
fn a_removed_option_schema_does_not_accept_an_old_override() {
    // The action had a quality option and the run used 60.
    let mut stored = with_options(vec![field("quality", Kind::Int, &[])]);
    stored
        .params
        .insert(String::from("quality"), toml::Value::Integer(60));
    // The schema no longer knows the option and the action file holds 85.
    let mut current = with_options(vec![]);
    current
        .params
        .insert(String::from("quality"), toml::Value::Integer(85));
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&stored, &[A]));
    ledger.record_unit(1, 0, failed(), Duration::from_millis(1));
    ledger.finish(1, Duration::from_millis(2), false);
    let refusal = ledger
        .retry_plan(1, Some(&current), &|_: &str| true, &fresh)
        .expect_err("no retry");
    assert!(
        matches!(refusal, RetryRefusal::InvalidOptions(_)),
        "{refusal:?}"
    );
}

#[test]
fn an_unchanged_action_file_default_survives_a_removed_schema() {
    // The run used the action file default of 85.
    let mut stored = with_options(vec![field("quality", Kind::Int, &[])]);
    stored
        .params
        .insert(String::from("quality"), toml::Value::Integer(85));
    // The schema no longer knows the option and the action file still holds
    // the same default.
    let mut current = with_options(vec![]);
    current
        .params
        .insert(String::from("quality"), toml::Value::Integer(85));
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&stored, &[A]));
    ledger.record_unit(1, 0, failed(), Duration::from_millis(1));
    ledger.finish(1, Duration::from_millis(2), false);
    let proposal = ledger
        .retry_plan(1, Some(&current), &|_: &str| true, &fresh)
        .expect("a retry");
    assert_eq!(proposal.inputs, [PathBuf::from("a.png")]);
}

// ---------- every stored typed value is revalidated ----------

#[test]
fn a_default_outside_a_narrower_range_is_refused() {
    // The run used the action file value of 90, which was valid then.
    let mut stored = with_options(vec![field("quality", Kind::Int, &[])]);
    stored
        .params
        .insert(String::from("quality"), toml::Value::Integer(90));
    // The schema now allows at most 50 and the action file value did not
    // move, so the value is unchanged and an older validator would let it
    // through.
    let mut narrowed = field("quality", Kind::Int, &[]);
    narrowed.max = Some(50.0);
    let mut current = with_options(vec![narrowed]);
    current
        .params
        .insert(String::from("quality"), toml::Value::Integer(90));
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&stored, &[A]));
    ledger.record_unit(1, 0, failed(), Duration::from_millis(1));
    ledger.finish(1, Duration::from_millis(2), false);
    let refusal = ledger
        .retry_plan(1, Some(&current), &|_: &str| true, &fresh)
        .expect_err("no retry");
    assert!(
        matches!(refusal, RetryRefusal::InvalidOptions(_)),
        "{refusal:?}"
    );
    let RetryRefusal::InvalidOptions(reason) = refusal else {
        unreachable!()
    };
    assert!(reason.contains("quality"), "{reason}");
}

#[test]
fn an_unchanged_typed_default_inside_the_range_is_accepted() {
    // The value equals the action file default and the current range holds
    // it, so the retry is not refused for a value that did not change.
    let mut stored = with_options(vec![field("quality", Kind::Int, &[])]);
    stored
        .params
        .insert(String::from("quality"), toml::Value::Integer(85));
    let mut current = with_options(vec![field("quality", Kind::Int, &[])]);
    current
        .params
        .insert(String::from("quality"), toml::Value::Integer(85));
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&stored, &[A]));
    ledger.record_unit(1, 0, failed(), Duration::from_millis(1));
    ledger.finish(1, Duration::from_millis(2), false);
    let proposal = ledger
        .retry_plan(1, Some(&current), &|_: &str| true, &fresh)
        .expect("a retry");
    assert_eq!(proposal.inputs, [PathBuf::from("a.png")]);
}

#[test]
fn a_resolved_table_with_a_cleared_option_is_accepted() {
    // The run set the longest side, so the resolved table holds no scale.
    let mut scale = field("scale", Kind::Percent, &["max_side"]);
    scale.min = Some(1.0);
    scale.max = Some(1000.0);
    let mut max_side = field("max_side", Kind::Px, &["scale"]);
    max_side.min = Some(16.0);
    max_side.max = Some(20000.0);
    let mut current = with_options(vec![scale, max_side]);
    current
        .params
        .insert(String::from("scale"), toml::Value::Integer(50));
    let mut stored = current.clone();
    assert!(stored.params.remove("scale").is_some());
    stored
        .params
        .insert(String::from("max_side"), toml::Value::Integer(1920));
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&stored, &[A]));
    ledger.record_unit(1, 0, failed(), Duration::from_millis(1));
    ledger.finish(1, Duration::from_millis(2), false);
    let proposal = ledger
        .retry_plan(1, Some(&current), &|_: &str| true, &fresh)
        .expect("a retry");
    assert_eq!(proposal.inputs, [PathBuf::from("a.png")]);
}

// ---------- the bounded wire view ----------

#[test]
fn the_summary_carries_counts_and_no_raw_action() {
    let mut act = action("fake", PerFile, "webp", "");
    act.params
        .insert(String::from("quality"), toml::Value::Integer(60));
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&act, &[A, B]));
    ledger.record_unit(1, 0, ok(), Duration::from_millis(1200));
    ledger.record_unit(1, 1, failed(), Duration::from_millis(800));
    ledger.finish(1, Duration::from_millis(2000), false);
    let record = ledger.job(1).expect("the job stays");
    let summary = JobSummary::of(record, "Compress to WebP");
    assert_eq!(summary.job, 1);
    assert_eq!(summary.action_label, "Compress to WebP");
    assert_eq!(summary.state, JobState::Completed);
    assert_eq!(summary.counts.total, 2);
    assert_eq!(summary.counts.succeeded, 1);
    assert_eq!(summary.counts.failed, 1);
    assert_eq!(summary.elapsed_ms, 2000);
    assert!(summary.retryable);
    // The wire carries no action definition, no params and no manifest.
    let line = serde_json::to_string(&summary).unwrap();
    assert!(!line.contains("quality"), "{line}");
    assert!(!line.contains("params"), "{line}");
    assert!(!line.contains("fake"), "{line}");
}

#[test]
fn the_detail_lists_the_units_with_their_paths_and_states() {
    let act = action("fake", PerFile, "webp", "");
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&act, &[A, B, C]));
    ledger.record_unit(1, 0, ok(), Duration::from_millis(1));
    ledger.record_unit(
        1,
        1,
        UnitOutcome::Skipped {
            reason: SkipReason::AlreadyOptimized,
        },
        Duration::from_millis(1),
    );
    ledger.record_unit(
        1,
        2,
        UnitOutcome::Failed {
            error: String::from("boom"),
        },
        Duration::from_millis(1),
    );
    ledger.finish(1, Duration::from_millis(3), false);
    let record = ledger.job(1).expect("the job stays");
    let detail = JobDetail::of(record, "Compress to WebP");
    assert_eq!(detail.units.len(), 3);
    assert_eq!(detail.units[0].inputs, [PathBuf::from("a.png")]);
    assert_eq!(detail.units[0].state, UnitState::Succeeded);
    assert_eq!(detail.units[0].output, Some(PathBuf::from("out.webp")));
    assert_eq!(detail.units[1].state, UnitState::Skipped);
    assert_eq!(detail.units[1].reason.as_deref(), Some("already-optimized"));
    assert_eq!(detail.units[2].state, UnitState::Failed);
    assert_eq!(detail.units[2].reason.as_deref(), Some("boom"));
    assert_eq!(detail.retry_units, [2]);
    // The unit inputs are paths, not identities, and no manifest is exposed.
    let line = serde_json::to_string(&detail).unwrap();
    assert!(!line.contains("regular_file"), "{line}");
    assert!(!line.contains("modified"), "{line}");
}

#[test]
fn a_summary_only_job_has_no_unit_detail_and_no_retry() {
    let act = action("fake", PerFile, "webp", "");
    let groups: Vec<&[&str]> = (0..1001).map(|_| A).collect();
    let mut ledger = ResultLedger::new();
    ledger.start(1, manifest(&act, &groups));
    ledger.record_unit(1, 0, failed(), Duration::from_millis(1));
    ledger.finish(1, Duration::from_millis(2), false);
    let record = ledger.job(1).expect("the job stays");
    let summary = JobSummary::of(record, "Compress to WebP");
    assert!(summary.summary_only);
    assert!(!summary.retryable);
    let detail = JobDetail::of(record, "Compress to WebP");
    assert!(detail.units.is_empty());
    assert!(detail.retry_units.is_empty());
    assert!(!detail.retryable);
    assert_eq!(detail.counts.total, 1001);
    assert_eq!(detail.counts.failed, 1);
    // No unit error text is on the wire.
    let line = serde_json::to_string(&detail).unwrap();
    assert!(!line.contains("boom"), "{line}");
}

#[test]
fn a_job_with_no_session_record_is_unknown_and_invents_nothing() {
    let detail = JobDetail::unknown(9);
    assert_eq!(detail.state, JobState::Unknown);
    assert_eq!(detail.counts.total, 0);
    assert!(detail.units.is_empty());
    assert!(detail.action_label.is_empty());
    assert!(!detail.retryable);
}
