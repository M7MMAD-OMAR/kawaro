//! Platform-free view model for the quiet session results surface (S-144-D).
//!
//! Every row is mapped directly from [`kawaro_core::job_report::JobDetail`],
//! the exact bounded projection the IPC surface uses. No second ledger
//! projection, no action execution logic and no Slint types live here.
//!
//! Output paths stay plain data (`PathBuf`): the view model checks only
//! whether an output file exists right now so the surface offers "Show in
//! folder" when present and reports unavailable when missing. A filename is
//! never treated as a URL or executed as a command.

use kawaro_core::i18n::Lang;
use kawaro_core::job_report::{JobDetail, JobState, OutcomeCounts, UnitResult, UnitState};
use kawaro_core::job_results::TaskUnitId;
use kawaro_core::queue::JobId;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Settings page index of the Results page inside the shared app shell (S-144).
/// Keeps Updates at 3 and About at 4 unchanged.
pub const RESULTS_PAGE: usize = 5;

/// All static strings of the results page in one language.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultsStrings {
    pub page_title: &'static str,
    pub empty_title: &'static str,
    pub empty_body: &'static str,
    pub clear_history: &'static str,
    pub clear_history_accessible: &'static str,
    pub retry: &'static str,
    pub retrying: &'static str,
    pub cancel: &'static str,
    pub show_in_folder: &'static str,
    pub output_unavailable: &'static str,
    pub expand_details: &'static str,
    pub collapse_details: &'static str,
    pub summary_only_reason: &'static str,
    pub retry_pending_reason: &'static str,
    pub inconsistent_reason: &'static str,
    pub state_queued: &'static str,
    pub state_active: &'static str,
    pub state_completed: &'static str,
    pub state_inconsistent: &'static str,
    pub state_unknown: &'static str,
    pub unit_pending: &'static str,
    pub unit_succeeded: &'static str,
    pub unit_skipped: &'static str,
    pub unit_failed: &'static str,
    pub unit_cancelled: &'static str,
    pub unit_interrupted: &'static str,
}

/// The static strings of the results page for `lang`.
pub fn strings(lang: Lang) -> ResultsStrings {
    match lang {
        Lang::En => ResultsStrings {
            page_title: "Results",
            empty_title: "No results yet",
            empty_body: "Active and completed jobs from this session appear here. History stays in memory only.",
            clear_history: "Clear history",
            clear_history_accessible: "Clear completed job history",
            retry: "Retry",
            retrying: "Retrying",
            cancel: "Cancel",
            show_in_folder: "Show in folder",
            output_unavailable: "Output unavailable",
            expand_details: "Show details",
            collapse_details: "Hide details",
            summary_only_reason: "Too many items to keep unit details; retry is unavailable.",
            retry_pending_reason: "Retry is already running.",
            inconsistent_reason: "Job ended with missing outcomes; retry is unavailable.",
            state_queued: "Queued",
            state_active: "Running",
            state_completed: "Completed",
            state_inconsistent: "Interrupted",
            state_unknown: "Unavailable",
            unit_pending: "Pending",
            unit_succeeded: "Succeeded",
            unit_skipped: "Skipped",
            unit_failed: "Failed",
            unit_cancelled: "Cancelled",
            unit_interrupted: "Interrupted",
        },
        Lang::Ar => ResultsStrings {
            page_title: "النتائج",
            empty_title: "لا نتائج بعد",
            empty_body: "تظهر هنا المهام الجارية والمكتملة في هذه الجلسة، ويبقى السجل في الذاكرة فقط.",
            clear_history: "مسح السجل",
            clear_history_accessible: "مسح سجل المهام المكتملة",
            retry: "إعادة المحاولة",
            retrying: "قيد إعادة المحاولة",
            cancel: "إلغاء",
            show_in_folder: "عرض في المجلد",
            output_unavailable: "الملف الناتج غير متوفر",
            expand_details: "عرض التفاصيل",
            collapse_details: "إخفاء التفاصيل",
            summary_only_reason: "عدد العناصر أكبر من حد السجل، فتُعرض الأعداد فقط ولا تتاح إعادة المحاولة.",
            retry_pending_reason: "إعادة المحاولة قيد التنفيذ.",
            inconsistent_reason: "انتهت المهمة دون تسجيل كل النتائج؛ إعادة المحاولة غير متاحة.",
            state_queued: "في الانتظار",
            state_active: "قيد التنفيذ",
            state_completed: "مكتملة",
            state_inconsistent: "متوقفة",
            state_unknown: "غير متوفرة",
            unit_pending: "في الانتظار",
            unit_succeeded: "مكتمل",
            unit_skipped: "متخطى",
            unit_failed: "تعذر",
            unit_cancelled: "ملغى",
            unit_interrupted: "متوقف",
        },
    }
}

/// Aggregate counts shown on one job row, keeping interrupted units distinct
/// from units still running in an active job.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OutcomeViewCounts {
    pub total: usize,
    pub succeeded: usize,
    pub skipped: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub interrupted: usize,
}

impl OutcomeViewCounts {
    /// Builds the view counts from a bounded job projection.
    pub fn from_detail(detail: &JobDetail) -> Self {
        let OutcomeCounts {
            total,
            succeeded,
            failed,
            skipped,
            cancelled,
        } = detail.counts;
        let recorded = succeeded
            .saturating_add(failed)
            .saturating_add(skipped)
            .saturating_add(cancelled);
        let interrupted = if detail.state == JobState::Inconsistent {
            total.saturating_sub(recorded)
        } else {
            0
        };
        OutcomeViewCounts {
            total,
            succeeded,
            skipped,
            failed,
            cancelled,
            interrupted,
        }
    }
}

/// The terminal or in-progress status of one unit row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitViewStatus {
    /// The parent job is still active and this unit has not reported yet.
    Pending,
    Succeeded,
    Skipped,
    Failed,
    Cancelled,
    /// The parent job ended without reporting an outcome for this unit.
    Interrupted,
}

/// Whether a recorded output path is currently available on disk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutputLinkState {
    /// No output file was recorded for this unit or job.
    None,
    /// The recorded output file currently exists and can be revealed in its folder.
    Available {
        path: PathBuf,
        display_name: String,
        action_label: String,
        accessible_label: String,
    },
    /// An output path was recorded, but the file is no longer present.
    Unavailable { display_name: String, label: String },
}

impl OutputLinkState {
    /// True when "Show in folder" should be offered.
    pub fn can_show_in_folder(&self) -> bool {
        matches!(self, OutputLinkState::Available { .. })
    }

    /// The path to reveal when available.
    pub fn available_path(&self) -> Option<&Path> {
        match self {
            OutputLinkState::Available { path, .. } => Some(path.as_path()),
            OutputLinkState::None | OutputLinkState::Unavailable { .. } => None,
        }
    }
}

/// Whether Retry is enabled for a job and why it is disabled when not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RetryState {
    Enabled {
        label: String,
        accessible_label: String,
    },
    Disabled {
        label: String,
        accessible_label: String,
        reason: Option<String>,
    },
}

impl RetryState {
    /// True when the Retry control is enabled.
    pub fn is_enabled(&self) -> bool {
        matches!(self, RetryState::Enabled { .. })
    }

    /// The visible label of the Retry button.
    pub fn label(&self) -> &str {
        match self {
            RetryState::Enabled { label, .. } | RetryState::Disabled { label, .. } => label,
        }
    }

    /// The accessible label of the Retry button.
    pub fn accessible_label(&self) -> &str {
        match self {
            RetryState::Enabled {
                accessible_label, ..
            }
            | RetryState::Disabled {
                accessible_label, ..
            } => accessible_label,
        }
    }

    /// The explanation shown when Retry is disabled for a notable reason
    /// (summary-only oversized job, pending retry, or interrupted job).
    pub fn reason(&self) -> Option<&str> {
        match self {
            RetryState::Enabled { .. } => None,
            RetryState::Disabled { reason, .. } => reason.as_deref(),
        }
    }
}

/// One expandable unit row under a job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitRow {
    pub unit: TaskUnitId,
    /// Display names of the unit's inputs, in manifest order.
    pub input_display: String,
    pub status: UnitViewStatus,
    pub status_label: String,
    /// Localized skip reason or failure detail; absent for success, cancel and pending.
    pub reason: Option<String>,
    pub output: OutputLinkState,
}

/// One job row in the results list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobRow {
    pub job: JobId,
    pub action_id: String,
    pub action_label: String,
    pub state: JobState,
    pub state_label: String,
    pub counts: OutcomeViewCounts,
    pub counts_summary: String,
    pub summary_only: bool,
    pub summary_only_notice: Option<String>,
    /// The first output of the job, useful for summary-only jobs that keep no unit rows.
    pub first_output: OutputLinkState,
    pub expanded: bool,
    pub can_expand: bool,
    pub expand_label: String,
    pub expand_accessible_label: String,
    pub retry: RetryState,
    /// True when Cancel is available for this job alone.
    pub can_cancel: bool,
    pub cancel_label: String,
    pub cancel_accessible_label: String,
    pub units: Vec<UnitRow>,
}

/// Complete, platform-free snapshot rendered by the results page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultsViewModel {
    pub lang: Lang,
    pub rtl: bool,
    pub strings: ResultsStrings,
    pub jobs: Vec<JobRow>,
    /// True when at least one completed or inconsistent job can be cleared.
    pub can_clear_history: bool,
}

impl ResultsViewModel {
    /// Builds the view model from bounded daemon projections and an injected
    /// output-existence predicate (for pure testing without touching the disk).
    pub fn build(
        details: &[JobDetail],
        pending_retries: &HashSet<JobId>,
        expanded: &HashSet<JobId>,
        lang: Lang,
        output_exists: &dyn Fn(&Path) -> bool,
    ) -> Self {
        let s = strings(lang);
        let jobs: Vec<JobRow> = details
            .iter()
            .map(|detail| build_job_row(detail, pending_retries, expanded, lang, &s, output_exists))
            .collect();
        let can_clear_history = details.iter().any(|detail| is_clearable(detail.state));
        ResultsViewModel {
            lang,
            rtl: lang.rtl(),
            strings: s,
            jobs,
            can_clear_history,
        }
    }

    /// Builds the view model checking real files on disk via [`Path::is_file`].
    pub fn build_fs(
        details: &[JobDetail],
        pending_retries: &HashSet<JobId>,
        expanded: &HashSet<JobId>,
        lang: Lang,
    ) -> Self {
        Self::build(details, pending_retries, expanded, lang, &|path| {
            path.is_file()
        })
    }
}

/// Stateful, platform-free UI controller for the results page.
///
/// Tracks per-job expansion and optimistic pending-retry guards so a double
/// click on Retry is idempotent even before the next daemon event arrives.
#[derive(Clone, Debug, Default)]
pub struct ResultsViewState {
    details: Vec<JobDetail>,
    daemon_pending: HashSet<JobId>,
    local_pending: HashSet<JobId>,
    expanded: HashSet<JobId>,
}

impl ResultsViewState {
    /// Creates an empty results state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces the displayed projections from the daemon's latest snapshot.
    ///
    /// Keeps expansion state for jobs still present and clears optimistic
    /// pending flags once the daemon's pending set reconciles.
    pub fn update(&mut self, details: Vec<JobDetail>, daemon_pending: HashSet<JobId>) {
        let live: HashSet<JobId> = details.iter().map(|d| d.job).collect();
        self.expanded.retain(|id| live.contains(id));
        self.local_pending
            .retain(|id| live.contains(id) && daemon_pending.contains(id));
        self.daemon_pending = daemon_pending;
        self.details = details;
    }

    /// Toggles the expanded state of one expandable job and returns the new state.
    pub fn toggle_expanded(&mut self, job: JobId) -> bool {
        let expandable = self
            .details
            .iter()
            .any(|d| d.job == job && !d.summary_only && !d.units.is_empty());
        if !expandable {
            self.expanded.remove(&job);
            return false;
        }
        if self.expanded.remove(&job) {
            false
        } else {
            self.expanded.insert(job);
            true
        }
    }

    /// Begins a Retry submission for `job`.
    ///
    /// Returns `true` only when the job is currently eligible for retry and no
    /// retry submission is already pending, then marks `job` pending so a
    /// duplicate trigger returns `false`.
    pub fn begin_retry(&mut self, job: JobId) -> bool {
        if self.is_retry_pending(job) {
            return false;
        }
        let eligible = self.details.iter().any(|d| {
            d.job == job && d.retryable && !d.summary_only && d.state == JobState::Completed
        });
        if !eligible {
            return false;
        }
        self.local_pending.insert(job);
        true
    }

    /// Releases the local optimistic pending lock when the daemon refuses a retry.
    pub fn finish_retry_refusal(&mut self, job: JobId) {
        self.local_pending.remove(&job);
    }

    /// Clears finished jobs from the local snapshot while keeping active and
    /// queued jobs intact. Mirrors [`kawaro_core::job_results::ResultLedger::clear_completed`].
    pub fn clear_completed_local(&mut self) {
        self.details.retain(|d| !is_clearable(d.state));
        let live: HashSet<JobId> = self.details.iter().map(|d| d.job).collect();
        self.expanded.retain(|id| live.contains(id));
        self.local_pending.retain(|id| live.contains(id));
        self.daemon_pending.retain(|id| live.contains(id));
    }

    /// True when a retry for `job` is pending locally or in the daemon.
    pub fn is_retry_pending(&self, job: JobId) -> bool {
        self.daemon_pending.contains(&job) || self.local_pending.contains(&job)
    }

    /// Resolves a recorded output path for "Show in folder" from stable ids
    /// rather than trusting a path string from the UI.
    pub fn output_for_folder(
        &self,
        job: JobId,
        unit: Option<TaskUnitId>,
        output_exists: &dyn Fn(&Path) -> bool,
    ) -> Option<PathBuf> {
        let detail = self.details.iter().find(|d| d.job == job)?;
        let path = match unit {
            Some(unit_id) => detail
                .units
                .iter()
                .find(|u| u.unit == unit_id)
                .and_then(|u| u.output.as_ref()),
            None => detail.first_output.as_ref(),
        }?;
        output_exists(path.as_path()).then(|| path.clone())
    }

    /// Builds a view model for `lang` with an injected existence predicate.
    pub fn view(&self, lang: Lang, output_exists: &dyn Fn(&Path) -> bool) -> ResultsViewModel {
        let pending = self.effective_pending();
        ResultsViewModel::build(&self.details, &pending, &self.expanded, lang, output_exists)
    }

    /// Builds a view model for `lang` using [`Path::is_file`].
    pub fn view_fs(&self, lang: Lang) -> ResultsViewModel {
        self.view(lang, &|path| path.is_file())
    }

    fn effective_pending(&self) -> HashSet<JobId> {
        self.daemon_pending
            .union(&self.local_pending)
            .copied()
            .collect()
    }
}

/// True when a job record is removed by "Clear history".
fn is_clearable(state: JobState) -> bool {
    matches!(
        state,
        JobState::Completed | JobState::Inconsistent | JobState::Unknown
    )
}

/// Maps one [`JobDetail`] into a [`JobRow`].
fn build_job_row(
    detail: &JobDetail,
    pending_retries: &HashSet<JobId>,
    expanded: &HashSet<JobId>,
    lang: Lang,
    s: &ResultsStrings,
    output_exists: &dyn Fn(&Path) -> bool,
) -> JobRow {
    let counts = OutcomeViewCounts::from_detail(detail);
    let action_label = if detail.action_label.trim().is_empty() {
        detail.action_id.clone()
    } else {
        detail.action_label.clone()
    };
    // Only an expanded job shows its unit outputs, so only its files are
    // checked on disk; a collapsed job costs no filesystem call per unit.
    let not_checked = |_: &Path| false;
    let unit_exists: &dyn Fn(&Path) -> bool = if expanded.contains(&detail.job) {
        output_exists
    } else {
        &not_checked
    };
    let units: Vec<UnitRow> = if detail.summary_only {
        Vec::new()
    } else {
        detail
            .units
            .iter()
            .map(|unit| build_unit_row(unit, detail.state, lang, s, unit_exists))
            .collect()
    };
    let can_expand = !detail.summary_only && !units.is_empty();
    let is_expanded = can_expand && expanded.contains(&detail.job);
    let is_pending = pending_retries.contains(&detail.job);
    let retry = build_retry_state(detail, &action_label, is_pending, lang, s);
    let can_cancel = matches!(detail.state, JobState::Queued | JobState::Active);
    let first_output = build_output_state(detail.first_output.as_deref(), lang, s, output_exists);

    let expand_accessible = expand_accessible_label(is_expanded, &action_label, lang);
    let cancel_accessible = cancel_accessible_label(&action_label, lang);

    JobRow {
        job: detail.job,
        action_id: detail.action_id.clone(),
        action_label,
        state: detail.state,
        state_label: job_state_label(detail.state, s).to_string(),
        counts,
        counts_summary: format_counts(counts, lang, s),
        summary_only: detail.summary_only,
        summary_only_notice: detail
            .summary_only
            .then(|| s.summary_only_reason.to_string()),
        first_output,
        expanded: is_expanded,
        can_expand,
        expand_label: if is_expanded {
            s.collapse_details.to_string()
        } else {
            s.expand_details.to_string()
        },
        expand_accessible_label: expand_accessible,
        retry,
        can_cancel,
        cancel_label: s.cancel.to_string(),
        cancel_accessible_label: cancel_accessible,
        units,
    }
}

/// Maps one [`UnitResult`] into a [`UnitRow`].
fn build_unit_row(
    unit: &UnitResult,
    job_state: JobState,
    lang: Lang,
    s: &ResultsStrings,
    output_exists: &dyn Fn(&Path) -> bool,
) -> UnitRow {
    let status = match unit.state {
        UnitState::Succeeded => UnitViewStatus::Succeeded,
        UnitState::Skipped => UnitViewStatus::Skipped,
        UnitState::Failed => UnitViewStatus::Failed,
        UnitState::Cancelled => UnitViewStatus::Cancelled,
        UnitState::Unreported => {
            if matches!(job_state, JobState::Queued | JobState::Active) {
                UnitViewStatus::Pending
            } else {
                UnitViewStatus::Interrupted
            }
        }
    };
    let status_label = match status {
        UnitViewStatus::Pending => s.unit_pending,
        UnitViewStatus::Succeeded => s.unit_succeeded,
        UnitViewStatus::Skipped => s.unit_skipped,
        UnitViewStatus::Failed => s.unit_failed,
        UnitViewStatus::Cancelled => s.unit_cancelled,
        UnitViewStatus::Interrupted => s.unit_interrupted,
    };
    let reason = unit
        .reason
        .as_deref()
        .map(|raw| localize_reason(status, raw, lang));
    let output = build_output_state(unit.output.as_deref(), lang, s, output_exists);
    UnitRow {
        unit: unit.unit,
        input_display: format_input_names(&unit.inputs, lang),
        status,
        status_label: status_label.to_string(),
        reason,
        output,
    }
}

/// Computes the Retry button state and reason from the bounded projection.
fn build_retry_state(
    detail: &JobDetail,
    action_label: &str,
    pending: bool,
    lang: Lang,
    s: &ResultsStrings,
) -> RetryState {
    let accessible = retry_accessible_label(action_label, lang);
    if pending {
        return RetryState::Disabled {
            label: s.retrying.to_string(),
            accessible_label: accessible,
            reason: Some(s.retry_pending_reason.to_string()),
        };
    }
    if detail.summary_only {
        return RetryState::Disabled {
            label: s.retry.to_string(),
            accessible_label: accessible,
            reason: Some(s.summary_only_reason.to_string()),
        };
    }
    if detail.state == JobState::Inconsistent {
        return RetryState::Disabled {
            label: s.retry.to_string(),
            accessible_label: accessible,
            reason: Some(s.inconsistent_reason.to_string()),
        };
    }
    if detail.retryable && detail.state == JobState::Completed {
        RetryState::Enabled {
            label: s.retry.to_string(),
            accessible_label: accessible,
        }
    } else {
        RetryState::Disabled {
            label: s.retry.to_string(),
            accessible_label: accessible,
            reason: None,
        }
    }
}

/// Maps an optional output path to an [`OutputLinkState`].
fn build_output_state(
    output: Option<&Path>,
    lang: Lang,
    s: &ResultsStrings,
    output_exists: &dyn Fn(&Path) -> bool,
) -> OutputLinkState {
    let Some(path) = output else {
        return OutputLinkState::None;
    };
    let display_name = file_display_name(path);
    if output_exists(path) {
        OutputLinkState::Available {
            path: path.to_path_buf(),
            accessible_label: show_in_folder_accessible_label(&display_name, lang),
            display_name,
            action_label: s.show_in_folder.to_string(),
        }
    } else {
        OutputLinkState::Unavailable {
            display_name,
            label: s.output_unavailable.to_string(),
        }
    }
}

/// Formats aggregate counts in reading order with Western digits.
fn format_counts(counts: OutcomeViewCounts, lang: Lang, s: &ResultsStrings) -> String {
    let sep = match lang {
        Lang::En => ", ",
        Lang::Ar => "، ",
    };
    let pairs = [
        (counts.succeeded, s.unit_succeeded),
        (counts.skipped, s.unit_skipped),
        (counts.failed, s.unit_failed),
        (counts.cancelled, s.unit_cancelled),
        (counts.interrupted, s.unit_interrupted),
    ];
    let parts: Vec<String> = pairs
        .into_iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, word)| format!("{n} {word}"))
        .collect();
    if parts.is_empty() {
        match lang {
            Lang::En => format!("0 of {}", counts.total),
            Lang::Ar => format!("0 من {}", counts.total),
        }
    } else {
        parts.join(sep)
    }
}

/// Localized job state word.
fn job_state_label(state: JobState, s: &ResultsStrings) -> &'static str {
    match state {
        JobState::Queued => s.state_queued,
        JobState::Active => s.state_active,
        JobState::Completed => s.state_completed,
        JobState::Inconsistent => s.state_inconsistent,
        JobState::Unknown => s.state_unknown,
    }
}

/// Translates known skip reason words into the UI language while leaving
/// engine diagnostic strings intact.
fn localize_reason(status: UnitViewStatus, raw: &str, lang: Lang) -> String {
    if status == UnitViewStatus::Skipped && raw == "already-optimized" {
        return match lang {
            Lang::En => String::from("Already optimized, no new file was written"),
            Lang::Ar => String::from("محسّن مسبقاً، لم يُكتب ملف جديد"),
        };
    }
    raw.to_string()
}

/// Displays the file names of a unit's inputs in order, never executing or
/// interpreting them as URLs.
fn format_input_names(inputs: &[PathBuf], lang: Lang) -> String {
    let sep = match lang {
        Lang::En => ", ",
        Lang::Ar => "، ",
    };
    inputs
        .iter()
        .map(|p| file_display_name(p))
        .collect::<Vec<_>>()
        .join(sep)
}

/// Short display name for a file path (basename when available).
fn file_display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.display().to_string())
}

fn retry_accessible_label(action_label: &str, lang: Lang) -> String {
    match lang {
        Lang::En => format!("Retry failed items in {action_label}"),
        Lang::Ar => format!("إعادة محاولة العناصر التي تعذرت في {action_label}"),
    }
}

fn cancel_accessible_label(action_label: &str, lang: Lang) -> String {
    match lang {
        Lang::En => format!("Cancel {action_label}"),
        Lang::Ar => format!("إلغاء {action_label}"),
    }
}

fn expand_accessible_label(expanded: bool, action_label: &str, lang: Lang) -> String {
    match (lang, expanded) {
        (Lang::En, false) => format!("Show details for {action_label}"),
        (Lang::En, true) => format!("Hide details for {action_label}"),
        (Lang::Ar, false) => format!("عرض تفاصيل {action_label}"),
        (Lang::Ar, true) => format!("إخفاء تفاصيل {action_label}"),
    }
}

fn show_in_folder_accessible_label(display_name: &str, lang: Lang) -> String {
    match lang {
        Lang::En => format!("Show {display_name} in folder"),
        Lang::Ar => format!("عرض {display_name} في المجلد"),
    }
}
