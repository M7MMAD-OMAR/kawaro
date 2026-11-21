//! S-144-D: view model cases for the quiet results surface.
//!
//! These cases test the platform-free view model mapped from the bounded
//! `kawaro_core::job_report::JobDetail` projections that the IPC surface uses.
//!
//! These cases were written and never executed under the owner's deferred
//! check policy. No runtime acceptance is claimed by them.

use kawaro_core::action::Registry;
use kawaro_core::engine::SkipReason;
use kawaro_core::i18n::Lang;
use kawaro_core::job_report::{JobDetail, JobState};
use kawaro_core::job_results::{JobManifest, MAX_COMPLETED_UNITS, ResultLedger, UnitOutcome};
use kawaro_ui::results_view::{
    OutputLinkState, RESULTS_PAGE, ResultsViewModel, ResultsViewState, RetryState, UnitViewStatus,
    strings,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

fn webp_action() -> kawaro_core::ActionDef {
    Registry::builtin()
        .get("image.to-webp")
        .expect("the built-in image.to-webp action")
        .clone()
}

fn manifest_for(files: Vec<PathBuf>) -> JobManifest {
    let units = files.into_iter().map(|p| vec![p]).collect();
    JobManifest::new(webp_action(), units)
}

#[test]
fn mixed_outcomes_map_counts_and_unit_details_in_both_languages() {
    let mut ledger = ResultLedger::new();
    let inputs = vec![
        PathBuf::from("/work/ok.png"),
        PathBuf::from("/work/small.png"),
        PathBuf::from("/work/broken.png"),
        PathBuf::from("/work/stopped.png"),
    ];
    ledger.start(10, manifest_for(inputs));
    assert!(ledger.record_unit(
        10,
        0,
        UnitOutcome::Success {
            output: Some(PathBuf::from("/work/ok.webp")),
            detail: None,
        },
        Duration::from_millis(120),
    ));
    assert!(ledger.record_unit(
        10,
        1,
        UnitOutcome::Skipped {
            reason: SkipReason::AlreadyOptimized,
        },
        Duration::from_millis(40),
    ));
    assert!(ledger.record_unit(
        10,
        2,
        UnitOutcome::Failed {
            error: String::from("corrupt image header"),
        },
        Duration::from_millis(80),
    ));
    assert!(ledger.record_unit(10, 3, UnitOutcome::Cancelled, Duration::ZERO));
    ledger.finish(10, Duration::from_millis(240), false);

    let detail = JobDetail::of(ledger.job(10).expect("job 10 in ledger"), "Convert to WebP");
    let expanded = HashSet::from([10]);
    let pending = HashSet::new();

    for lang in [Lang::En, Lang::Ar] {
        let vm = ResultsViewModel::build(
            std::slice::from_ref(&detail),
            &pending,
            &expanded,
            lang,
            &|p| p == Path::new("/work/ok.webp"),
        );
        assert_eq!(vm.rtl, lang.rtl());
        assert!(vm.can_clear_history);
        assert_eq!(vm.jobs.len(), 1);

        let row = &vm.jobs[0];
        assert_eq!(row.job, 10);
        assert_eq!(row.action_label, "Convert to WebP");
        assert_eq!(row.state, JobState::Completed);
        assert_eq!(row.counts.total, 4);
        assert_eq!(row.counts.succeeded, 1);
        assert_eq!(row.counts.skipped, 1);
        assert_eq!(row.counts.failed, 1);
        assert_eq!(row.counts.cancelled, 1);
        assert_eq!(row.counts.interrupted, 0);
        assert!(row.expanded);
        assert!(row.can_expand);
        assert!(!row.can_cancel);
        assert!(row.retry.is_enabled());
        assert_eq!(row.units.len(), 4);

        assert_eq!(row.units[0].status, UnitViewStatus::Succeeded);
        assert_eq!(row.units[0].reason, None);
        assert!(row.units[0].output.can_show_in_folder());
        assert_eq!(
            row.units[0].output.available_path(),
            Some(Path::new("/work/ok.webp"))
        );

        assert_eq!(row.units[1].status, UnitViewStatus::Skipped);
        assert!(row.units[1].reason.is_some());
        assert_eq!(row.units[1].output, OutputLinkState::None);

        assert_eq!(row.units[2].status, UnitViewStatus::Failed);
        assert_eq!(row.units[2].reason.as_deref(), Some("corrupt image header"));
        assert_eq!(row.units[2].output, OutputLinkState::None);

        assert_eq!(row.units[3].status, UnitViewStatus::Cancelled);
        assert_eq!(row.units[3].reason, None);
        assert_eq!(row.units[3].output, OutputLinkState::None);
    }
}

#[test]
fn retry_is_disabled_while_pending_and_is_idempotent() {
    let mut ledger = ResultLedger::new();
    ledger.start(20, manifest_for(vec![PathBuf::from("/work/a.png")]));
    assert!(ledger.record_unit(
        20,
        0,
        UnitOutcome::Failed {
            error: String::from("decode error"),
        },
        Duration::from_millis(50),
    ));
    ledger.finish(20, Duration::from_millis(50), false);

    let detail = JobDetail::of(ledger.job(20).expect("job 20 in ledger"), "Convert to WebP");

    let mut state = ResultsViewState::new();
    state.update(vec![detail.clone()], HashSet::new());

    let before = state.view(Lang::En, &|_| false);
    assert!(before.jobs[0].retry.is_enabled());
    assert_eq!(before.jobs[0].retry.reason(), None);

    // First click accepts and marks pending; second click is ignored.
    assert!(state.begin_retry(20));
    assert!(!state.begin_retry(20));

    let during = state.view(Lang::En, &|_| false);
    assert!(!during.jobs[0].retry.is_enabled());
    assert_eq!(
        during.jobs[0].retry.reason(),
        Some(strings(Lang::En).retry_pending_reason)
    );
    assert_eq!(during.jobs[0].retry.label(), strings(Lang::En).retrying);

    // Daemon snapshot confirming pending keeps Retry disabled.
    state.update(vec![detail.clone()], HashSet::from([20]));
    let confirmed = state.view(Lang::Ar, &|_| false);
    assert!(!confirmed.jobs[0].retry.is_enabled());
    assert_eq!(
        confirmed.jobs[0].retry.reason(),
        Some(strings(Lang::Ar).retry_pending_reason)
    );

    // Once the child job finishes and leaves pending_retries, Retry is eligible again.
    state.update(vec![detail], HashSet::new());
    let after = state.view(Lang::En, &|_| false);
    assert!(after.jobs[0].retry.is_enabled());
}

#[test]
fn retry_is_disabled_for_summary_only_oversized_jobs_with_reason() {
    let mut ledger = ResultLedger::new();
    let count = MAX_COMPLETED_UNITS + 1;
    let inputs: Vec<PathBuf> = (0..count)
        .map(|i| PathBuf::from(format!("/work/file_{i}.png")))
        .collect();
    ledger.start(30, manifest_for(inputs));
    for unit in 0..count as u64 {
        let outcome = if unit == 0 {
            UnitOutcome::Failed {
                error: String::from("broken"),
            }
        } else {
            UnitOutcome::Success {
                output: Some(PathBuf::from(format!("/work/file_{unit}.webp"))),
                detail: None,
            }
        };
        assert!(ledger.record_unit(30, unit, outcome, Duration::from_millis(1)));
    }
    ledger.finish(30, Duration::from_secs(2), false);

    let detail = JobDetail::of(ledger.job(30).expect("job 30 in ledger"), "Convert to WebP");
    assert!(detail.summary_only);
    assert!(!detail.retryable);
    assert!(detail.units.is_empty());

    let mut state = ResultsViewState::new();
    state.update(vec![detail], HashSet::new());
    assert!(!state.begin_retry(30));
    assert!(!state.toggle_expanded(30));

    for lang in [Lang::En, Lang::Ar] {
        let vm = state.view(lang, &|p| p == Path::new("/work/file_1.webp"));
        let row = &vm.jobs[0];
        assert!(row.summary_only);
        assert_eq!(row.counts.total, count);
        assert_eq!(row.counts.failed, 1);
        assert_eq!(row.counts.succeeded, MAX_COMPLETED_UNITS);
        assert!(!row.can_expand);
        assert!(row.units.is_empty());
        assert!(!row.retry.is_enabled());
        assert_eq!(row.retry.reason(), Some(strings(lang).summary_only_reason));
        assert_eq!(
            row.summary_only_notice.as_deref(),
            Some(strings(lang).summary_only_reason)
        );
        assert!(row.first_output.can_show_in_folder());
    }
}

#[test]
fn missing_output_shows_unavailable_and_existing_output_offers_show_in_folder() {
    let dir = tempfile::tempdir().expect("tempdir");
    let present = dir.path().join("present.webp");
    let missing = dir.path().join("missing.webp");
    std::fs::write(&present, b"webp-bytes").expect("write output");

    let mut ledger = ResultLedger::new();
    ledger.start(
        40,
        manifest_for(vec![
            dir.path().join("present.png"),
            dir.path().join("missing.png"),
        ]),
    );
    assert!(ledger.record_unit(
        40,
        0,
        UnitOutcome::Success {
            output: Some(present.clone()),
            detail: None,
        },
        Duration::from_millis(10),
    ));
    assert!(ledger.record_unit(
        40,
        1,
        UnitOutcome::Success {
            output: Some(missing.clone()),
            detail: None,
        },
        Duration::from_millis(10),
    ));
    ledger.finish(40, Duration::from_millis(20), false);

    let detail = JobDetail::of(ledger.job(40).expect("job 40 in ledger"), "Convert to WebP");
    let mut state = ResultsViewState::new();
    state.update(vec![detail], HashSet::new());

    let vm = state.view_fs(Lang::En);
    let row = &vm.jobs[0];
    assert!(row.units[0].output.can_show_in_folder());
    assert_eq!(
        row.units[0].output.available_path(),
        Some(present.as_path())
    );
    assert!(!row.units[1].output.can_show_in_folder());
    assert_eq!(
        row.units[1].output,
        OutputLinkState::Unavailable {
            display_name: String::from("missing.webp"),
            label: strings(Lang::En).output_unavailable.to_string(),
        }
    );

    assert_eq!(
        state.output_for_folder(40, Some(0), &|p| p.is_file()),
        Some(present)
    );
    assert_eq!(state.output_for_folder(40, Some(1), &|p| p.is_file()), None);
}

#[test]
fn clear_history_keeps_active_and_queued_jobs_and_preserves_outputs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output = dir.path().join("done.webp");
    std::fs::write(&output, b"kept").expect("write output");

    let mut ledger = ResultLedger::new();
    // Job 50: completed.
    ledger.start(50, manifest_for(vec![dir.path().join("done.png")]));
    assert!(ledger.record_unit(
        50,
        0,
        UnitOutcome::Success {
            output: Some(output.clone()),
            detail: None,
        },
        Duration::from_millis(10),
    ));
    ledger.finish(50, Duration::from_millis(10), false);

    // Job 51: active.
    ledger.start(51, manifest_for(vec![dir.path().join("running.png")]));

    // Job 52: queued (no manifest in ledger yet).
    let queued = JobDetail::queued(52, "Convert to WebP");
    let active = JobDetail::of(ledger.job(51).expect("active job"), "Convert to WebP");
    let completed = JobDetail::of(ledger.job(50).expect("completed job"), "Convert to WebP");

    let mut state = ResultsViewState::new();
    state.update(vec![active, queued, completed], HashSet::new());

    let before = state.view_fs(Lang::En);
    assert!(before.can_clear_history);
    assert_eq!(before.jobs.len(), 3);
    assert!(before.jobs[0].can_cancel);
    assert!(before.jobs[1].can_cancel);
    assert!(!before.jobs[2].can_cancel);

    // Clearing history in the ledger and view keeps active and queued jobs only.
    ledger.clear_completed();
    assert!(ledger.job(50).is_none());
    assert!(ledger.job(51).is_some());

    state.clear_completed_local();
    let after = state.view_fs(Lang::En);
    assert!(!after.can_clear_history);
    assert_eq!(after.jobs.len(), 2);
    assert_eq!(after.jobs[0].job, 51);
    assert_eq!(after.jobs[0].state, JobState::Active);
    assert_eq!(after.jobs[1].job, 52);
    assert_eq!(after.jobs[1].state, JobState::Queued);
    assert!(output.is_file(), "clearing history never deletes outputs");
}

#[test]
fn inconsistent_job_counts_interrupted_units_and_disables_retry() {
    let mut ledger = ResultLedger::new();
    ledger.start(
        60,
        manifest_for(vec![
            PathBuf::from("/work/one.png"),
            PathBuf::from("/work/two.png"),
        ]),
    );
    assert!(ledger.record_unit(
        60,
        0,
        UnitOutcome::Success {
            output: Some(PathBuf::from("/work/one.webp")),
            detail: None,
        },
        Duration::from_millis(10),
    ));
    // Finish without cancellation while unit 1 never reported.
    ledger.finish(60, Duration::from_millis(15), false);

    let detail = JobDetail::of(ledger.job(60).expect("inconsistent job"), "Convert to WebP");
    assert_eq!(detail.state, JobState::Inconsistent);

    let vm = ResultsViewModel::build(
        std::slice::from_ref(&detail),
        &HashSet::new(),
        &HashSet::from([60]),
        Lang::En,
        &|_| false,
    );
    let row = &vm.jobs[0];
    assert_eq!(row.counts.succeeded, 1);
    assert_eq!(row.counts.interrupted, 1);
    assert_eq!(row.units[1].status, UnitViewStatus::Interrupted);
    assert!(!row.retry.is_enabled());
    assert_eq!(
        row.retry.reason(),
        Some(strings(Lang::En).inconsistent_reason)
    );

    // By contrast, an active job's unreported unit is Pending, not Interrupted.
    let mut active_ledger = ResultLedger::new();
    active_ledger.start(61, manifest_for(vec![PathBuf::from("/work/live.png")]));
    let active_detail = JobDetail::of(
        active_ledger.job(61).expect("active job"),
        "Convert to WebP",
    );
    let active_vm = ResultsViewModel::build(
        std::slice::from_ref(&active_detail),
        &HashSet::new(),
        &HashSet::from([61]),
        Lang::En,
        &|_| false,
    );
    assert_eq!(active_vm.jobs[0].counts.interrupted, 0);
    assert_eq!(active_vm.jobs[0].units[0].status, UnitViewStatus::Pending);
    assert!(matches!(
        active_vm.jobs[0].retry,
        RetryState::Disabled { reason: None, .. }
    ));
}

#[test]
fn bilingual_strings_and_accessible_labels_have_no_long_dashes() {
    for lang in [Lang::En, Lang::Ar] {
        let s = strings(lang);
        let all = [
            s.page_title,
            s.empty_title,
            s.empty_body,
            s.clear_history,
            s.clear_history_accessible,
            s.retry,
            s.retrying,
            s.cancel,
            s.show_in_folder,
            s.output_unavailable,
            s.expand_details,
            s.collapse_details,
            s.summary_only_reason,
            s.retry_pending_reason,
            s.inconsistent_reason,
            s.state_queued,
            s.state_active,
            s.state_completed,
            s.state_inconsistent,
            s.state_unknown,
            s.unit_pending,
            s.unit_succeeded,
            s.unit_skipped,
            s.unit_failed,
            s.unit_cancelled,
            s.unit_interrupted,
        ];
        for text in all {
            assert!(!text.trim().is_empty(), "{lang:?}: empty string");
            assert!(
                !text.contains('\u{2014}') && !text.contains('\u{2013}'),
                "{lang:?}: long dash in {text:?}"
            );
        }
    }
    assert_eq!(strings(Lang::En).page_title, "Results");
    assert_eq!(strings(Lang::Ar).page_title, "النتائج");
    assert_eq!(RESULTS_PAGE, 5);
}

#[test]
fn settings_and_welcome_windows_open_directly_on_results_page_with_zero_unlabelled_controls() {
    use kawaro_core::config::Config;
    use kawaro_core::welcome::Captions;
    use kawaro_ui::SettingsWindow;
    use kawaro_ui::welcome::WelcomeWindow;

    thread_local! {
        static INIT: std::sync::Once = const { std::sync::Once::new() };
    }
    INIT.with(|once| once.call_once(i_slint_backend_testing::init_no_event_loop));

    let mut ledger = ResultLedger::new();
    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path().join("done.webp");
    std::fs::write(&out, b"webp").expect("write output");
    ledger.start(
        70,
        manifest_for(vec![
            dir.path().join("done.png"),
            dir.path().join("bad.png"),
        ]),
    );
    assert!(ledger.record_unit(
        70,
        0,
        UnitOutcome::Success {
            output: Some(out),
            detail: None,
        },
        Duration::from_millis(10),
    ));
    assert!(ledger.record_unit(
        70,
        1,
        UnitOutcome::Failed {
            error: String::from("decode error"),
        },
        Duration::from_millis(10),
    ));
    ledger.finish(70, Duration::from_millis(20), false);
    let detail = JobDetail::of(ledger.job(70).expect("job 70"), "Convert to WebP");
    let mut state = ResultsViewState::new();
    state.update(vec![detail], HashSet::new());
    state.toggle_expanded(70);

    for lang in [Lang::En, Lang::Ar] {
        let vm = state.view_fs(lang);
        let sw = SettingsWindow::new(lang).expect("settings window");
        sw.open_results(&Config::default()).expect("open results");
        sw.set_results(&vm);
        assert!(sw.results_visible());
        assert_eq!(sw.page(), RESULTS_PAGE);
        assert_eq!(sw.unlabelled_controls(), 0, "{lang:?} settings results");
        sw.close();

        let ww = WelcomeWindow::new(lang).expect("welcome window");
        let captions = Captions {
            steps: std::array::from_fn(|i| format!("step {i}")),
        };
        ww.open_results(&captions, false, &Config::default())
            .expect("open welcome results");
        ww.set_results(&vm);
        assert!(ww.results_visible());
        assert_eq!(ww.page(), RESULTS_PAGE + 1);
        assert_eq!(ww.unlabelled_controls(), 0, "{lang:?} welcome results");
        ww.close();
    }
}
