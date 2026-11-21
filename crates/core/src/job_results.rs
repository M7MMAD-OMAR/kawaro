//! Platform-free records for batch outcomes and safe retry (S-144).
//!
//! The ledger lives in memory only. Nothing is written to disk, no history
//! survives a restart and no work is replayed after a crash.
//!
//! The identity of a source file is its size and its modification time. It
//! is best effort: it is not a content hash and it does not remove the race
//! between reading the identity and running the engine.
//!
//! The daemon records what the queue reports and asks the ledger for a retry
//! proposal. The proposal is not a submission: the caller submits it
//! through the normal queue path, so the output naming and the safe commit
//! stay in one place.

use crate::action::{ActionDef, OutputMode};
use crate::engine::{SkipReason, TaskDetail};
use crate::queue::JobId;
use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// At most this many completed jobs are kept.
pub const MAX_COMPLETED_JOBS: usize = 100;
/// At most this many completed units are kept, over all completed jobs.
pub const MAX_COMPLETED_UNITS: usize = 1000;

/// The identity of one unit inside a job: its ordinal in the ordered
/// manifest.
pub type TaskUnitId = u64;

/// The best effort identity of one source file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputIdentity {
    /// The path the file was read from.
    pub path: PathBuf,
    /// The size in bytes.
    pub len: u64,
    /// The modification time, when the file system reports one.
    pub modified: Option<SystemTime>,
    /// False when the path is missing or is not a regular file.
    pub regular_file: bool,
}

impl InputIdentity {
    /// Reads the identity of `path`, best effort. A missing file gives a
    /// zero length, no time and no regular file.
    pub fn capture(path: &Path) -> Self {
        match std::fs::metadata(path) {
            Ok(m) => InputIdentity {
                path: path.to_path_buf(),
                len: m.len(),
                modified: m.modified().ok(),
                regular_file: m.is_file(),
            },
            Err(_) => InputIdentity {
                path: path.to_path_buf(),
                len: 0,
                modified: None,
                regular_file: false,
            },
        }
    }
}

/// The current identity of a source file, or nothing when it is not there.
///
/// A file that is missing or is not a regular file has no identity, so a
/// retry never runs on it.
pub fn current_identity(path: &Path) -> Option<InputIdentity> {
    let identity = InputIdentity::capture(path);
    if identity.regular_file {
        Some(identity)
    } else {
        None
    }
}

/// The identity of the action that ran: the engine, the operation and the
/// output mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionSnapshot {
    pub engine: String,
    pub op: String,
    pub output_mode: OutputMode,
}

impl ActionSnapshot {
    pub fn of(action: &ActionDef) -> Self {
        ActionSnapshot {
            engine: action.engine.clone(),
            op: action.op.clone(),
            output_mode: action.output.mode,
        }
    }
}

/// One execution unit: the inputs of one task, in order.
#[derive(Clone, Debug, PartialEq)]
pub struct TaskUnitManifest {
    /// The ordinal of the unit in the job.
    pub id: TaskUnitId,
    /// The source files of the unit, in the order the task receives them.
    pub inputs: Vec<InputIdentity>,
}

/// What one job is made of: the resolved action and its ordered units.
#[derive(Clone, Debug, PartialEq)]
pub struct JobManifest {
    /// The action with its options already resolved, as it ran.
    pub action: ActionDef,
    /// The identity of that action.
    pub snapshot: ActionSnapshot,
    /// The units, in the order the job runs them.
    pub units: Vec<TaskUnitManifest>,
}

impl JobManifest {
    /// Builds the manifest of one job from its ordered execution units. A
    /// per-file action has one unit per file, a combined action has one unit
    /// holding the whole ordered group.
    pub fn new(action: ActionDef, units: Vec<Vec<PathBuf>>) -> Self {
        let snapshot = ActionSnapshot::of(&action);
        let units = units
            .into_iter()
            .enumerate()
            .map(|(ordinal, inputs)| TaskUnitManifest {
                id: ordinal as TaskUnitId,
                inputs: inputs
                    .into_iter()
                    .map(|p| InputIdentity::capture(&p))
                    .collect(),
            })
            .collect();
        JobManifest {
            action,
            snapshot,
            units,
        }
    }

    /// The number of source files in the job.
    pub fn input_count(&self) -> usize {
        self.units.iter().map(|u| u.inputs.len()).sum()
    }
}

/// What one unit ended as.
#[derive(Clone, Debug)]
pub enum UnitOutcome {
    /// The unit wrote its output. The detail is the structured result when
    /// the engine has one.
    Success {
        output: Option<PathBuf>,
        detail: Option<TaskDetail>,
    },
    /// The unit failed.
    Failed { error: String },
    /// The unit deliberately wrote nothing.
    Skipped { reason: SkipReason },
    /// The unit was cancelled before it finished.
    Cancelled,
}

/// One unit and its terminal outcome.
#[derive(Clone, Debug)]
pub struct UnitRecord {
    /// The unit this outcome belongs to.
    pub unit: TaskUnitId,
    /// The terminal outcome.
    pub outcome: UnitOutcome,
    /// The time the unit took.
    pub elapsed: Duration,
}

/// The aggregate counts of one job.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct JobCounts {
    /// The number of units in the job.
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub skipped: usize,
    pub cancelled: usize,
}

/// The state of one job in the ledger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobStatus {
    /// The job is still running.
    Active,
    /// The job finished and every unit has an outcome.
    Completed,
    /// The job finished but some unit never reported. Such a job is not
    /// retried.
    Inconsistent,
}

/// One job as the ledger keeps it.
#[derive(Clone, Debug)]
pub struct JobRecord {
    /// The queue job id.
    pub job: JobId,
    /// The id of the action that ran.
    pub action_id: String,
    /// The manifest. It is absent for a summary only job, which cannot be
    /// retried.
    pub manifest: Option<JobManifest>,
    pub status: JobStatus,
    /// The unit outcomes, in the order they were recorded. Empty for a
    /// summary only job.
    pub units: Vec<UnitRecord>,
    pub counts: JobCounts,
    /// The time the job took.
    pub elapsed: Duration,
    /// The job has more units than the ledger keeps, so only its counts stay.
    /// Such a job is never retried.
    pub summary_only: bool,
    /// The first output of the job, when there is one.
    pub first_output: Option<PathBuf>,
    /// A local note about an inconsistency. It is never a user message.
    pub diagnostic: Option<String>,
    /// The units that already have an outcome. It keeps the ledger from
    /// counting one unit twice and it names the units that never reported.
    seen: HashSet<TaskUnitId>,
}

/// The in-memory ledger of job outcomes.
///
/// Active jobs are exempt from the retention limits. Completed jobs leave
/// whole, oldest first, so a job is never left with a part of its units.
#[derive(Debug, Default)]
pub struct ResultLedger {
    active: Vec<JobRecord>,
    completed: VecDeque<JobRecord>,
}

impl ResultLedger {
    /// A new, empty ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records the manifest of a job that is about to run.
    ///
    /// The manifest stays until the job finishes, so every task event can be
    /// attached to the unit it belongs to.
    pub fn start(&mut self, job: JobId, manifest: JobManifest) {
        if self.active.iter().any(|r| r.job == job) {
            return;
        }
        let total = manifest.units.len();
        let action_id = manifest.action.id.clone();
        self.active.push(JobRecord {
            job,
            action_id,
            manifest: Some(manifest),
            status: JobStatus::Active,
            units: Vec::new(),
            counts: JobCounts {
                total,
                ..JobCounts::default()
            },
            elapsed: Duration::ZERO,
            summary_only: false,
            first_output: None,
            diagnostic: None,
            seen: HashSet::new(),
        });
    }

    /// The next unit of a job that has no outcome yet, in manifest order.
    ///
    /// This is a convenience for the tests. The production path uses the
    /// unit ordinal the queue puts in every task event, so the unit of an
    /// event never depends on the order the events arrive in.
    pub fn next_unit(&self, job: JobId) -> Option<TaskUnitId> {
        let record = self.active.iter().find(|r| r.job == job)?;
        let total = record.manifest.as_ref()?.units.len() as TaskUnitId;
        let next = record.seen.len() as TaskUnitId;
        if next < total { Some(next) } else { None }
    }

    /// Records the terminal outcome of one unit.
    ///
    /// A unit is recorded once. A unit outside the manifest and a unit that
    /// already has an outcome are refused, so the counts of a job never
    /// exceed the number of its units. The method says whether the outcome
    /// was recorded.
    pub fn record_unit(
        &mut self,
        job: JobId,
        unit: TaskUnitId,
        outcome: UnitOutcome,
        elapsed: Duration,
    ) -> bool {
        let Some(record) = self.active.iter_mut().find(|r| r.job == job) else {
            return false;
        };
        let Some(total) = record.manifest.as_ref().map(|m| m.units.len()) else {
            return false;
        };
        if unit >= total as TaskUnitId {
            append_note(
                record,
                format!("the unit {unit} of the job {job} is outside the manifest"),
            );
            return false;
        }
        if !record.seen.insert(unit) {
            append_note(
                record,
                format!("the unit {unit} of the job {job} reported twice"),
            );
            return false;
        }
        // A job with more units than the ledger keeps is counted, never
        // detailed, so no unit error text piles up.
        let oversized = total > MAX_COMPLETED_UNITS;
        match &outcome {
            UnitOutcome::Success { output, .. } => {
                record.counts.succeeded += 1;
                if record.first_output.is_none() {
                    record.first_output = output.clone();
                }
            }
            UnitOutcome::Failed { .. } => record.counts.failed += 1,
            UnitOutcome::Skipped { .. } => record.counts.skipped += 1,
            UnitOutcome::Cancelled => record.counts.cancelled += 1,
        }
        if !oversized {
            record.units.push(UnitRecord {
                unit,
                outcome,
                elapsed,
            });
        }
        true
    }

    /// Finishes a job. `cancelled` says the queue reported a cancel.
    ///
    /// The units that never reported become cancelled when the job was
    /// cancelled, each with its own unit id. A job that finished without a
    /// cancel and without every outcome is inconsistent: it is not retried
    /// and it keeps a local diagnostic.
    pub fn finish(&mut self, job: JobId, elapsed: Duration, cancelled: bool) {
        let Some(index) = self.active.iter().position(|r| r.job == job) else {
            return;
        };
        let mut record = self.active.remove(index);
        record.elapsed = elapsed;
        let total = record
            .manifest
            .as_ref()
            .map(|m| m.units.len())
            .unwrap_or(record.counts.total);
        record.counts.total = total;
        // The units that never reported, with their own ids.
        let seen = &record.seen;
        let missing: Vec<TaskUnitId> = match record.manifest.as_ref() {
            Some(manifest) => manifest
                .units
                .iter()
                .map(|u| u.id)
                .filter(|id| !seen.contains(id))
                .collect(),
            None => Vec::new(),
        };
        let oversized = total > MAX_COMPLETED_UNITS;
        if cancelled {
            record.counts.cancelled += missing.len();
            if !oversized {
                for id in &missing {
                    record.units.push(UnitRecord {
                        unit: *id,
                        outcome: UnitOutcome::Cancelled,
                        elapsed: Duration::ZERO,
                    });
                }
            }
            record.seen.extend(missing.iter().copied());
            record.status = JobStatus::Completed;
        } else if !missing.is_empty() {
            record.status = JobStatus::Inconsistent;
            append_note(
                &mut record,
                format!("{} of {total} units reported no outcome", missing.len()),
            );
        } else {
            record.status = JobStatus::Completed;
        }
        // The counts of a finished job always add up to its total.
        let sum = record.counts.succeeded
            + record.counts.failed
            + record.counts.skipped
            + record.counts.cancelled;
        if record.status == JobStatus::Completed && sum != total {
            record.status = JobStatus::Inconsistent;
            append_note(
                &mut record,
                format!("the counts add up to {sum} of {total} units"),
            );
        }
        if record.diagnostic.is_some() {
            record.status = JobStatus::Inconsistent;
        }
        if oversized {
            // The whole job summary stays, no part of it.
            record.summary_only = true;
            record.units.clear();
            record.manifest = None;
        }
        self.completed.push_back(record);
        self.enforce_limits();
    }

    /// Removes every completed job. Active jobs keep running and no output
    /// file is touched.
    pub fn clear_completed(&mut self) {
        self.completed.clear();
    }

    /// One job by its id.
    pub fn job(&self, job: JobId) -> Option<&JobRecord> {
        self.active
            .iter()
            .chain(self.completed.iter())
            .find(|r| r.job == job)
    }

    /// Every job, the active ones first.
    pub fn jobs(&self) -> impl Iterator<Item = &JobRecord> {
        self.active.iter().chain(self.completed.iter())
    }

    /// The completed jobs, oldest first.
    pub fn completed_jobs(&self) -> impl Iterator<Item = &JobRecord> {
        self.completed.iter()
    }

    /// The number of unit records kept over all completed jobs.
    pub fn completed_units(&self) -> usize {
        self.completed.iter().map(|r| r.units.len()).sum()
    }

    /// The number of completed jobs kept.
    pub fn completed_jobs_count(&self) -> usize {
        self.completed.len()
    }

    /// Keeps the ledger inside its limits: at most `MAX_COMPLETED_JOBS`
    /// completed jobs and at most `MAX_COMPLETED_UNITS` completed units.
    /// The oldest completed job leaves whole.
    fn enforce_limits(&mut self) {
        while self.completed.len() > MAX_COMPLETED_JOBS {
            if self.completed.pop_front().is_none() {
                break;
            }
        }
        while self.completed_units() > MAX_COMPLETED_UNITS {
            if self.completed.pop_front().is_none() {
                break;
            }
        }
    }

    /// Builds the retry proposal of a finished job.
    ///
    /// `current` is the action the registry holds now, looked up by the
    /// stored action id. It is the action as it would run now, so the
    /// settings the daemon applies are already in it. `engine_ok` says
    /// whether the engine is available. `identity` reads the current
    /// identity of a source file.
    ///
    /// Only failed units are eligible. A per-file action retries its failed
    /// files, a combined action retries the whole original ordered group or
    /// it is refused. The returned proposal is submitted by the caller with
    /// `Queue::submit`, and it holds the action as it ran, not the current
    /// one.
    pub fn retry_plan(
        &self,
        job: JobId,
        current: Option<&ActionDef>,
        engine_ok: &dyn Fn(&str) -> bool,
        identity: &dyn Fn(&Path) -> Option<InputIdentity>,
    ) -> Result<RetryProposal, RetryRefusal> {
        let record = self.job(job).ok_or(RetryRefusal::MissingJob)?;
        // A job that is kept as a summary only is never retried, whatever
        // its status is.
        if record.summary_only {
            return Err(RetryRefusal::SummaryOnly);
        }
        match record.status {
            JobStatus::Active => return Err(RetryRefusal::NotCompleted),
            JobStatus::Inconsistent => return Err(RetryRefusal::Inconsistent),
            JobStatus::Completed => {}
        }
        let manifest = record.manifest.as_ref().ok_or(RetryRefusal::SummaryOnly)?;
        let stored = &manifest.action;
        let current = current.ok_or_else(|| RetryRefusal::ActionMissing(stored.id.clone()))?;
        // The action id must be the same. The registry is looked up by the
        // stored id, so a different id means the caller passed another
        // action and the retry would run the wrong thing.
        if current.id != stored.id {
            return Err(RetryRefusal::ActionChanged(format!(
                "the action id changed from `{}` to `{}`",
                stored.id, current.id
            )));
        }
        // The engine, the operation and the output mode must be the same. A
        // changed registry action is never used silently.
        if current.engine != stored.engine {
            return Err(RetryRefusal::ActionChanged(format!(
                "the engine changed from `{}` to `{}`",
                stored.engine, current.engine
            )));
        }
        if current.op != stored.op {
            return Err(RetryRefusal::ActionChanged(format!(
                "the operation changed from `{}` to `{}`",
                stored.op, current.op
            )));
        }
        if current.output.mode != stored.output.mode {
            return Err(RetryRefusal::ActionChanged(format!(
                "the output mode changed from `{:?}` to `{:?}",
                stored.output.mode, current.output.mode
            )));
        }
        if !engine_ok(&stored.engine) {
            return Err(RetryRefusal::EngineUnavailable(stored.engine.clone()));
        }
        check_params(current, stored).map_err(RetryRefusal::InvalidOptions)?;

        let failed = record
            .units
            .iter()
            .filter(|u| matches!(u.outcome, UnitOutcome::Failed { .. }))
            .count();
        if failed == 0 {
            return Err(RetryRefusal::NoEligibleFailures);
        }

        let mut inputs: Vec<PathBuf> = Vec::new();
        let mut original_units: Vec<TaskUnitId> = Vec::new();
        match stored.output.mode {
            OutputMode::Combined => {
                // A combined action is one task over the whole group. It is
                // never retried as a part of the group.
                if manifest.units.len() != 1 {
                    return Err(RetryRefusal::GroupIncomplete);
                }
                let unit = &manifest.units[0];
                for input in &unit.inputs {
                    inputs.push(input.path.clone());
                }
                original_units.push(unit.id);
            }
            OutputMode::PerFile => {
                // The failed units keep the order of the original manifest,
                // not the order their outcomes arrived in.
                for source in &manifest.units {
                    let is_failed = record.units.iter().any(|u| {
                        u.unit == source.id && matches!(u.outcome, UnitOutcome::Failed { .. })
                    });
                    if !is_failed {
                        continue;
                    }
                    if source.inputs.len() != 1 {
                        return Err(RetryRefusal::GroupIncomplete);
                    }
                    inputs.push(source.inputs[0].path.clone());
                    original_units.push(source.id);
                }
            }
        }

        for input in &inputs {
            let stored_identity = manifest
                .units
                .iter()
                .flat_map(|u| u.inputs.iter())
                .find(|i| &i.path == input)
                .ok_or_else(|| RetryRefusal::InputUnavailable(input.display().to_string()))?;
            let fresh = identity(input.as_path())
                .ok_or_else(|| RetryRefusal::InputUnavailable(input.display().to_string()))?;
            if !fresh.regular_file {
                return Err(RetryRefusal::InputUnavailable(input.display().to_string()));
            }
            if stored_identity.modified.is_none() {
                return Err(RetryRefusal::IdentityIncomplete(
                    input.display().to_string(),
                ));
            }
            if fresh.len != stored_identity.len || fresh.modified != stored_identity.modified {
                return Err(RetryRefusal::InputChanged(input.display().to_string()));
            }
        }

        Ok(RetryProposal {
            parent_job: job,
            action: stored.clone(),
            inputs,
            original_units,
        })
    }
}

/// A retry the caller submits through the normal queue path.
#[derive(Clone, Debug)]
pub struct RetryProposal {
    /// The job this retry comes from.
    pub parent_job: JobId,
    /// The action as it ran, with its options already resolved.
    pub action: ActionDef,
    /// The inputs to submit, in the original order. For a combined action
    /// this is the whole original group.
    pub inputs: Vec<PathBuf>,
    /// The units that failed.
    pub original_units: Vec<TaskUnitId>,
}

/// Why a retry is refused.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RetryRefusal {
    #[error("the job is not in the ledger")]
    MissingJob,
    #[error("the job has not finished")]
    NotCompleted,
    #[error("the job ended with missing outcomes and is not retried")]
    Inconsistent,
    #[error("the job is kept as a summary only and cannot be retried")]
    SummaryOnly,
    #[error("no failed unit can be retried")]
    NoEligibleFailures,
    #[error("the original group of the job is incomplete")]
    GroupIncomplete,
    #[error("the action `{0}` is no longer in the registry")]
    ActionMissing(String),
    #[error("the action changed since the job ran: {0}")]
    ActionChanged(String),
    #[error("the engine `{0}` is not available")]
    EngineUnavailable(String),
    #[error("the stored options no longer fit the action: {0}")]
    InvalidOptions(String),
    #[error("the source `{0}` is missing or is not a regular file")]
    InputUnavailable(String),
    #[error("the source `{0}` changed since the job ran")]
    InputChanged(String),
    #[error("the source `{0}` has no recorded modification time")]
    IdentityIncomplete(String),
}

/// Checks that the stored parameters still fit the current action.
///
/// Every stored value of a field the current schema still knows is
/// revalidated, including a value that is equal to the current action file
/// default: a schema whose range shrank must refuse the old value too. The
/// complete typed table is resolved once through the current
/// `OptionsSpec`, so the clears and the dependent fields are seen together
/// and never one parameter at a time. No value check is written a second
/// time: the schema answers what it knows and what it clears.
///
/// A stored parameter the current schema no longer knows is an action file
/// parameter, and it may only stay when it is the unchanged action file
/// default. A changed one is refused, so a removed option schema never
/// accepts an old override silently.
fn check_params(current: &ActionDef, stored: &ActionDef) -> Result<(), String> {
    let Some(options) = current.options.as_ref() else {
        // Without typed options every stored parameter is an action file
        // parameter, and only an unchanged one may stay.
        return unchanged_action_params(current, stored);
    };
    options
        .validate()
        .map_err(|e| format!("the current options are invalid: {e}"))?;
    // The stored value of every field the current schema still knows, keyed
    // by the param key, in the order the schema declares its fields.
    let mut typed = toml::Table::new();
    for field in &options.fields {
        let key = field.param_key();
        if let Some(value) = stored.params.get(key) {
            typed.insert(key.to_string(), value.clone());
        }
    }
    // One resolve of the whole table.
    let resolved = options
        .resolve_params(&toml::Table::new(), &typed)
        .map_err(|e| format!("the stored options no longer fit the action: {e}"))?;
    // The resolved table must reproduce every stored value. A value that a
    // stored field clears is a combination the current schema does not
    // allow, so the table never resolves to itself and is refused.
    for (key, value) in typed.iter() {
        if resolved.get(key) != Some(value) {
            return Err(format!("the parameter `{key}` no longer maps to itself"));
        }
    }
    unchanged_action_params(current, stored)
}

/// The stored parameters the current schema does not know: they are action
/// file parameters, and a changed one is refused.
fn unchanged_action_params(current: &ActionDef, stored: &ActionDef) -> Result<(), String> {
    let typed = |key: &str| {
        current
            .options
            .as_ref()
            .is_some_and(|options| options.fields.iter().any(|f| f.param_key() == key))
    };
    for (key, value) in stored.params.iter() {
        if typed(key) {
            continue;
        }
        if current.params.get(key) != Some(value) {
            return Err(format!(
                "the parameter `{key}` changed in the action file since the job ran"
            ));
        }
    }
    Ok(())
}

/// Adds a note to the local diagnostic of a job.
fn append_note(record: &mut JobRecord, text: String) {
    let mut note = String::new();
    if let Some(existing) = &record.diagnostic {
        note.push_str(existing);
        note.push_str("; ");
    }
    note.push_str(&text);
    record.diagnostic = Some(note);
}
