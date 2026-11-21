//! `kawaro jobs`, `kawaro results`, `kawaro retry` and `kawaro cancel`:
//! the session results and the explicit retry (S-144-C).
//!
//! Every command reads the running daemon. None of them starts one: the
//! history is session memory, so there is nothing to show without it. A
//! daemon older than this kawaro answers no job results capability, and the
//! command refuses instead of converting or replaying anything.
//!
//! A job reference may name the daemon instance it was taken from
//! (S-148 AC3): `kawaro cancel INSTANCE:JOB` is checked by the daemon, and a
//! reference of an earlier daemon is refused instead of reaching a job
//! number that the restart reused.
//!
//! The output is English, like the rest of the CLI. Paths are printed as
//! plain data: no command opens a file, a folder or a window.

use crate::JobRef;
use crate::Res;
use crate::client;
use kawaro_core::ipc::{Request, Response};
use kawaro_core::job_report::{JobState, UnitState};

/// The refusal of a daemon that does not answer the results commands.
const NO_JOBS: &str =
    "the running daemon is older than this kawaro and has no job results: restart kawarod";

/// The refusal of a daemon that has no instance to check a reference with
/// (S-148 AC3). The request is never sent to such a daemon.
const NO_CHECKED: &str =
    "this daemon does not support checked job references: run `kawaro jobs` again";

/// What the running daemon says about itself before a job command runs.
struct DaemonInfo {
    /// The instance of the daemon that answers, when it sends one (S-148).
    instance: Option<String>,
}

/// The version handshake, the same one S-120 uses for `run --set`. A new
/// command is refused when the running daemon does not advertise it, so an
/// old daemon never looks like an empty history.
fn require_jobs() -> Res<DaemonInfo> {
    match client::request(&Request::Version)? {
        Response::Version {
            job_results: true,
            instance,
            ..
        } => Ok(DaemonInfo { instance }),
        Response::Version { .. } => Err(String::from(NO_JOBS)),
        other => Err(unexpected(&other)),
    }
}

/// The instance to send with a job request (S-148 AC3).
///
/// A plain job number names the daemon that answers now, so nothing is sent
/// and the current session is meant. A checked reference needs a daemon that
/// sends an instance: an older daemon refuses it here, before the request
/// is sent.
fn instance_for(info: &DaemonInfo, job: &JobRef) -> Res<Option<String>> {
    match &job.instance {
        None => Ok(None),
        Some(instance) => {
            if info.instance.is_none() {
                return Err(String::from(NO_CHECKED));
            }
            Ok(Some(instance.clone()))
        }
    }
}

/// `kawaro jobs [--json]`: one bounded summary per job of this session.
pub fn list(json: bool) -> Res<()> {
    require_jobs()?;
    let (instance, jobs) = match client::request(&Request::Jobs)? {
        Response::Jobs { jobs, instance } => (instance, jobs),
        other => return Err(unexpected(&other)),
    };
    if json {
        // The instance travels with the jobs, so a machine reader keeps the
        // reference of the daemon the numbers belong to.
        let document = serde_json::json!({
            "instance": instance,
            "jobs": jobs,
        });
        out!(
            "{}",
            serde_json::to_string_pretty(&document).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    // S-148 AC3: the instance is in the header, once, so a job number of
    // this list can be used later as a checked reference.
    let name = instance.as_deref().unwrap_or("unknown");
    out!("daemon instance: {name}");
    if jobs.is_empty() {
        out!("no jobs in this session");
        return Ok(());
    }
    out!(
        "{:<6} {:<12} {:>9} {:>6} {:>7} {:>9}  {}",
        "JOB",
        "STATE",
        "SUCCEEDED",
        "FAILED",
        "SKIPPED",
        "CANCELLED",
        "ACTION"
    );
    for job in &jobs {
        out!(
            "{:<6} {:<12} {:>9} {:>6} {:>7} {:>9}  {}",
            job.job,
            state_word(job.state),
            job.counts.succeeded,
            job.counts.failed,
            job.counts.skipped,
            job.counts.cancelled,
            job.action_label
        );
    }
    Ok(())
}

/// `kawaro results JOB [--json]`: what happened to one job.
pub fn show(job: JobRef, json: bool) -> Res<()> {
    let info = require_jobs()?;
    let instance = instance_for(&info, &job)?;
    let number = job.job;
    let detail = match client::request(&Request::JobResults {
        job: number,
        instance,
    })? {
        Response::JobResults { job: detail } => detail,
        other => return Err(unexpected(&other)),
    };
    if json {
        out!(
            "{}",
            serde_json::to_string_pretty(&detail).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    if detail.state == JobState::Unknown {
        out!("no session record of job {number}: it never started here, or its history is gone");
        return Ok(());
    }
    out!(
        "job {}: {} ({}), {}",
        detail.job,
        detail.action_label,
        state_word(detail.state),
        elapsed(detail.elapsed_ms)
    );
    out!(
        "  {} succeeded, {} failed, {} skipped, {} cancelled",
        detail.counts.succeeded,
        detail.counts.failed,
        detail.counts.skipped,
        detail.counts.cancelled
    );
    if detail.summary_only {
        out!(
            "  summary only: this job has more units than the ledger keeps, so it has no unit list and no retry"
        );
        return Ok(());
    }
    for unit in &detail.units {
        out!(
            "  {:<5} {:<11} {}",
            unit.unit,
            unit_state_word(unit.state),
            display_paths(&unit.inputs)
        );
        if let Some(output) = &unit.output {
            out!("        output: {}", output.display());
        }
        if let Some(reason) = &unit.reason {
            out!("        reason: {reason}");
        }
    }
    if !detail.retry_units.is_empty() {
        let units: Vec<String> = detail.retry_units.iter().map(|u| u.to_string()).collect();
        out!(
            "retry: kawaro retry {} (failed units: {})",
            detail.job,
            units.join(", ")
        );
    }
    Ok(())
}

/// `kawaro results --clear`: forget the completed jobs of this session.
pub fn clear() -> Res<()> {
    require_jobs()?;
    match client::request(&Request::ClearJobResults)? {
        Response::Ok => {}
        other => return Err(unexpected(&other)),
    }
    out!("cleared the session job history; active jobs keep running");
    Ok(())
}

/// `kawaro retry JOB`: submit the failed units of a finished job as a new
/// job. The daemon runs the full preflight, so a refusal is printed as an
/// error.
pub fn retry(job: JobRef) -> Res<()> {
    let info = require_jobs()?;
    let instance = instance_for(&info, &job)?;
    let number = job.job;
    match client::request(&Request::RetryJob {
        job: number,
        instance,
    })? {
        Response::Retry { job: child } => {
            out!("retrying job {number} as job {child}");
            Ok(())
        }
        other => Err(unexpected(&other)),
    }
}

/// `kawaro cancel JOB`: ask the queue to cancel one job.
pub fn cancel(job: JobRef) -> Res<()> {
    let info = require_jobs()?;
    let instance = instance_for(&info, &job)?;
    let number = job.job;
    match client::request(&Request::CancelJob {
        job: number,
        instance,
    })? {
        Response::Ok => {}
        other => return Err(unexpected(&other)),
    }
    out!("cancel requested for job {number}");
    Ok(())
}

/// The job state in the words the table shows.
fn state_word(state: JobState) -> &'static str {
    match state {
        JobState::Queued => "queued",
        JobState::Active => "active",
        JobState::Completed => "completed",
        JobState::Inconsistent => "inconsistent",
        JobState::Unknown => "unknown",
    }
}

/// The unit state in the words the detail shows.
fn unit_state_word(state: UnitState) -> &'static str {
    match state {
        UnitState::Succeeded => "succeeded",
        UnitState::Failed => "failed",
        UnitState::Skipped => "skipped",
        UnitState::Cancelled => "cancelled",
        UnitState::Unreported => "unreported",
    }
}

/// A duration in the shortest readable form.
fn elapsed(ms: u64) -> String {
    if ms < 1000 {
        format!("{ms}ms")
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        format!("{}m {:02}s", ms / 60_000, (ms % 60_000) / 1000)
    }
}

/// The input paths of one unit, as plain data.
fn display_paths(paths: &[std::path::PathBuf]) -> String {
    paths
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

/// The one reply the command did not expect.
fn unexpected(reply: &Response) -> String {
    format!("the daemon answered {reply:?}, expected a job results reply")
}
