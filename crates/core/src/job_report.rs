//! Bounded, serialisable views of the job results (S-144-C).
//!
//! The wire carries no [`crate::ActionDef`], no resolved params, no whole
//! manifest and no internal diagnostic. A summary carries the job id, the
//! action label, the state, the counts, the elapsed time and the first
//! output. The detail of one job adds its ordered units with their own input
//! paths, terminal state, output and reason.
//!
//! The types are a read only projection of
//! [`crate::job_results::JobRecord`]. They add no second record, no second
//! status word and no second outcome type, so the wire and the ledger cannot
//! drift.

use crate::job_results::{
    InputIdentity, JobRecord, JobStatus, MAX_COMPLETED_UNITS, TaskUnitId, UnitOutcome,
};
use crate::queue::JobId;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// The state of a job as the results view shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JobState {
    /// Submitted and not started yet: the daemon holds it and the ledger has
    /// no manifest for it.
    Queued,
    /// The job is running.
    Active,
    /// The job finished and every unit has an outcome.
    Completed,
    /// The job finished but a unit never reported. It is not retried.
    Inconsistent,
    /// The daemon has no session record of this job: it never started, or its
    /// history left the ledger. Nothing is invented for it.
    Unknown,
}

/// What one unit ended as, in the words the surfaces show.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnitState {
    Succeeded,
    Failed,
    Skipped,
    Cancelled,
    /// The job ended without an outcome for this unit. It is a missing
    /// report, never a cancellation and never a success.
    Unreported,
}

/// The aggregate counts of one job, as the wire carries them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutcomeCounts {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub skipped: usize,
    pub cancelled: usize,
}

impl OutcomeCounts {
    fn of(record: &JobRecord) -> Self {
        let counts = record.counts;
        OutcomeCounts {
            total: counts.total,
            succeeded: counts.succeeded,
            failed: counts.failed,
            skipped: counts.skipped,
            cancelled: counts.cancelled,
        }
    }
}

/// One job of the session, without its units.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobSummary {
    pub job: JobId,
    /// The id of the action that ran, empty when the daemon does not know it
    /// yet.
    pub action_id: String,
    /// The label of the action in the daemon's language, or the label the
    /// submission carried.
    pub action_label: String,
    pub state: JobState,
    pub counts: OutcomeCounts,
    pub elapsed_ms: u64,
    /// The first output of the job, when it wrote one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_output: Option<PathBuf>,
    /// The job has more units than the ledger keeps, so only its counts stay.
    /// Such a job has no unit detail and is never retried.
    pub summary_only: bool,
    /// The ledger offers a retry of this job: a finished, detailed job with at
    /// least one failed unit. The daemon still runs the full preflight, so a
    /// refusal is still possible.
    pub retryable: bool,
}

impl JobSummary {
    /// The summary of a job the ledger holds.
    pub fn of(record: &JobRecord, action_label: &str) -> Self {
        let state = state_of(record);
        JobSummary {
            job: record.job,
            action_id: record.action_id.clone(),
            action_label: action_label.to_string(),
            state,
            counts: OutcomeCounts::of(record),
            elapsed_ms: millis(record.elapsed),
            first_output: record.first_output.clone(),
            summary_only: summary_only(record),
            retryable: state == JobState::Completed
                && !summary_only(record)
                && !failed_units(record).is_empty(),
        }
    }

    /// The summary of a job the queue holds and the ledger has no manifest
    /// for. The action id is not known yet; the label is the one the
    /// submission carried.
    pub fn queued(job: JobId, action_label: &str) -> Self {
        JobSummary {
            job,
            action_id: String::new(),
            action_label: action_label.to_string(),
            state: JobState::Queued,
            counts: OutcomeCounts::default(),
            elapsed_ms: 0,
            first_output: None,
            summary_only: false,
            retryable: false,
        }
    }
}

/// One execution unit of a job.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitResult {
    pub unit: TaskUnitId,
    /// The source files of the unit, in the order the task received them.
    pub inputs: Vec<PathBuf>,
    pub state: UnitState,
    /// The final output path, when the unit wrote one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<PathBuf>,
    /// Why the unit failed or was skipped: the engine error of a failure, the
    /// skip reason word of a skip. None for a success and a cancellation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// The detail of one job: its units, in the order the job runs them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobDetail {
    pub job: JobId,
    pub action_id: String,
    pub action_label: String,
    pub state: JobState,
    pub counts: OutcomeCounts,
    pub elapsed_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_output: Option<PathBuf>,
    pub summary_only: bool,
    pub retryable: bool,
    /// The units a retry would submit, in manifest order. Empty for a job
    /// that is not retried.
    pub retry_units: Vec<TaskUnitId>,
    /// The units, empty for a summary only job and for a job the ledger does
    /// not hold.
    pub units: Vec<UnitResult>,
}

impl JobDetail {
    /// The detail of a job the ledger holds.
    pub fn of(record: &JobRecord, action_label: &str) -> Self {
        let state = state_of(record);
        let retry_units = if state == JobState::Completed && !summary_only(record) {
            failed_units(record)
        } else {
            Vec::new()
        };
        let units: Vec<UnitResult> = record
            .manifest
            .as_ref()
            .filter(|_| !summary_only(record))
            .map(|manifest| {
                manifest
                    .units
                    .iter()
                    .map(|unit| unit_result(record, unit.id, &unit.inputs))
                    .collect()
            })
            .unwrap_or_default();
        JobDetail {
            job: record.job,
            action_id: record.action_id.clone(),
            action_label: action_label.to_string(),
            state,
            counts: OutcomeCounts::of(record),
            elapsed_ms: millis(record.elapsed),
            first_output: record.first_output.clone(),
            summary_only: summary_only(record),
            retryable: !retry_units.is_empty(),
            retry_units,
            units,
        }
    }

    /// The detail of a job the queue holds and the ledger has no manifest
    /// for. It has no units: the queue has not planned them yet.
    pub fn queued(job: JobId, action_label: &str) -> Self {
        JobDetail {
            job,
            action_id: String::new(),
            action_label: action_label.to_string(),
            state: JobState::Queued,
            counts: OutcomeCounts::default(),
            elapsed_ms: 0,
            first_output: None,
            summary_only: false,
            retryable: false,
            retry_units: Vec::new(),
            units: Vec::new(),
        }
    }

    /// The detail of a job the daemon has no session record of. It invents
    /// no action, no count and no terminal state.
    pub fn unknown(job: JobId) -> Self {
        JobDetail {
            job,
            action_id: String::new(),
            action_label: String::new(),
            state: JobState::Unknown,
            counts: OutcomeCounts::default(),
            elapsed_ms: 0,
            first_output: None,
            summary_only: false,
            retryable: false,
            retry_units: Vec::new(),
            units: Vec::new(),
        }
    }
}

/// The ledger state of a record as a wire state.
fn state_of(record: &JobRecord) -> JobState {
    match record.status {
        JobStatus::Active => JobState::Active,
        JobStatus::Completed => JobState::Completed,
        JobStatus::Inconsistent => JobState::Inconsistent,
    }
}

/// One unit of a job, with the outcome the ledger recorded for it.
fn unit_result(record: &JobRecord, unit: TaskUnitId, inputs: &[InputIdentity]) -> UnitResult {
    let recorded = record.units.iter().find(|r| r.unit == unit);
    let (state, output, reason) = match recorded.map(|r| &r.outcome) {
        Some(UnitOutcome::Success { output, .. }) => (UnitState::Succeeded, output.clone(), None),
        Some(UnitOutcome::Failed { error }) => (UnitState::Failed, None, Some(error.clone())),
        Some(UnitOutcome::Skipped { reason }) => (
            UnitState::Skipped,
            None,
            Some(reason_word(*reason).to_string()),
        ),
        Some(UnitOutcome::Cancelled) => (UnitState::Cancelled, None, None),
        None => (UnitState::Unreported, None, None),
    };
    UnitResult {
        unit,
        inputs: inputs.iter().map(|i| i.path.clone()).collect(),
        state,
        output,
        reason,
    }
}

/// The stable word of a skip reason. The closed enum serializes kebab-case,
/// which is the word every surface already uses.
fn reason_word(reason: crate::engine::SkipReason) -> &'static str {
    match reason {
        crate::engine::SkipReason::AlreadyOptimized => "already-optimized",
    }
}

/// The units a retry would submit, in the order the job runs them.
fn failed_units(record: &JobRecord) -> Vec<TaskUnitId> {
    let Some(manifest) = record.manifest.as_ref() else {
        return Vec::new();
    };
    manifest
        .units
        .iter()
        .map(|unit| unit.id)
        .filter(|id| {
            record
                .units
                .iter()
                .any(|r| r.unit == *id && matches!(r.outcome, UnitOutcome::Failed { .. }))
        })
        .collect()
}

/// A duration as whole milliseconds, saturating instead of wrapping.
fn millis(elapsed: Duration) -> u64 {
    u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
}
/// Large active jobs follow the same wire detail limit as completed jobs.
fn summary_only(record: &JobRecord) -> bool {
    record.summary_only
        || record
            .manifest
            .as_ref()
            .is_some_and(|manifest| manifest.units.len() > MAX_COMPLETED_UNITS)
}
