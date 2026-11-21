//! S-108 and S-145: what the hive shows for a selection, as plain data.
//! `server::open` builds a [`HivePlan`] once per hive show using the shared
//! S-145 `capability_reports` and cached OCR probes, then presents it;
//! [`on_choice`] decides what a chosen cell does.

use crate::state::{State, common_kind};
use crate::{text, ui};
use kawaro_core::i18n::Lang;
use kawaro_core::unsupported::{
    self, CapabilityReport, Outcome, UnavailableReason, more_label, request_hover, request_label,
    request_url, skipped_label,
};
use kawaro_core::{ActionDef, media};
use kawaro_ui::HiveCell;
use std::path::{Path, PathBuf};

/// The hive limit, and how many greyed cells show before `+K`.
const MAX_CELLS: usize = 18;
const MAX_SKIPPED: usize = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HivePlan {
    pub centre: String,
    pub message: String,
    pub suggestion: Option<String>,
    pub cells: Vec<HiveCell>,
    /// The files a real action runs on: the supported ones.
    pub pending: Option<Vec<PathBuf>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Choice {
    Run(String),
    Request(String),
    Ignore,
}

/// What a chosen cell id means: a skipped or `+K` cell does nothing, the
/// request cell gives the request link, any other id runs that action.
pub fn choice(id: &str) -> Choice {
    if id == "more" || id.starts_with("skipped:") {
        Choice::Ignore
    } else if let Some(ext) = id.strip_prefix("request:") {
        Choice::Request(request_url(ext))
    } else {
        Choice::Run(id.to_string())
    }
}

/// [`choice`], and one call of `open` with the link for a request.
pub fn on_choice(id: &str, open: &dyn Fn(&str)) -> Choice {
    let c = choice(id);
    if let Choice::Request(url) = &c {
        open(url);
    }
    c
}

fn extra(id: String, code: String, label: String, reason: String, enabled: bool) -> HiveCell {
    HiveCell {
        id,
        code,
        label,
        icon: String::new(),
        ring: 2,
        enabled,
        reason,
        customize: false,
    }
}

/// Checks whether the OCR `language` option is ready from the cached probe
/// only, so a hive show never waits for a child.
fn check_ocr_language(state: &State, language: &str) -> Option<UnavailableReason> {
    let components: Option<&[&str]> = match language {
        "eng" => Some(&["eng"]),
        "ara" => Some(&["ara"]),
        "ara+eng" => Some(&["ara", "eng"]),
        _ => None,
    };
    let Some(parts) = components else {
        return Some(UnavailableReason::InvalidOptions);
    };
    let Some(program) = state.ocr.program() else {
        return Some(UnavailableReason::MissingExecutable {
            program: String::from("tesseract"),
        });
    };
    // The hive show path never runs a probe child: it reads the daemon cache,
    // then the engine cache. A cold cache counts as runnable here, the
    // background warm-up fills it, and the adapter preflights the language
    // again before any OCR runs.
    state
        .probe_cache
        .get(program)
        .or_else(|| state.ocr.cached_language_probe())
        .and_then(|cached| cached.check_language("tesseract", parts))
}

/// Evaluates why `action` is unavailable under `state`, or `None` when runnable.
fn action_reason(state: &State, action: &ActionDef) -> Option<UnavailableReason> {
    let ok = |e: &str| state.available.iter().any(|a| a == e);
    if !ok(&action.engine) {
        return unsupported::default_engine_reason(&action.engine, false);
    }
    if action.engine == "ocr" {
        let ocr_lang = action
            .params
            .get("lang")
            .and_then(|v| v.as_str())
            .unwrap_or("ara+eng");
        return check_ocr_language(state, ocr_lang);
    }
    None
}

fn is_unavailable_matching(report: &CapabilityReport) -> bool {
    !report.runnable
        && report
            .reason
            .as_ref()
            .is_some_and(|r| !matches!(r, UnavailableReason::UnsupportedInput))
}

/// Formats one non-runnable [`CapabilityReport`] as a single-line explanation
/// for the hive centre or a greyed cell (S-145).
fn typed_unavailable_message(report: &CapabilityReport, lang: Lang) -> String {
    let Some(reason) = report.reason.as_ref() else {
        return unsupported::unsupported_message(lang).to_string();
    };
    match reason {
        UnavailableReason::MissingExecutable { .. } => report
            .help_id
            .as_deref()
            .and_then(|help_id| unsupported::help_message(help_id, reason, lang))
            .unwrap_or_else(|| unsupported::reason_message(reason, lang)),
        UnavailableReason::UnsupportedInput => unsupported::unsupported_message(lang).to_string(),
        UnavailableReason::MissingLanguageData { .. }
        | UnavailableReason::InvalidOptions
        | UnavailableReason::DisabledByPolicy
        | UnavailableReason::ProbeUnknown => unsupported::reason_message(reason, lang),
    }
}

/// The reason one skipped file has no action.
fn reason(file: &Path, state: &State, lang: Lang) -> String {
    let path = [file];
    let reports =
        unsupported::capability_reports(&state.registry, &path, &|a| action_reason(state, a));
    match reports.iter().find(|r| is_unavailable_matching(r)) {
        Some(report) => typed_unavailable_message(report, lang),
        None => unsupported::unsupported_message(lang).to_string(),
    }
}

/// The greyed cells, `+K` and the request cell that fit in `room` cells.
fn extras(
    skipped: &[PathBuf],
    room: usize,
    state: &State,
    lang: Lang,
    single_override: Option<&str>,
) -> Vec<HiveCell> {
    let total = skipped.len();
    if total == 0 || room == 0 {
        return Vec::new();
    }
    let shown = (1..=total.min(MAX_SKIPPED))
        .rev()
        .find(|&n| n + usize::from(total > n) < room)
        .unwrap_or(0);
    let mut cells: Vec<HiveCell> = skipped
        .iter()
        .take(shown)
        .enumerate()
        .map(|(i, f)| {
            let ext = media::extension(f);
            let why = match single_override {
                Some(override_reason) if total == 1 => override_reason.to_string(),
                _ => reason(f, state, lang),
            };
            let code = if ext.is_empty() {
                "?".to_string()
            } else {
                ext.to_uppercase()
            };
            extra(
                format!("skipped:{i}"),
                code,
                skipped_label(&ext, &why, lang),
                why,
                false,
            )
        })
        .collect();
    let rest = total - shown;
    if rest > 0 {
        let label = more_label(rest, lang);
        cells.push(extra(
            "more".into(),
            format!("+{rest}"),
            label.clone(),
            label,
            false,
        ));
    }
    if cells.len() < room {
        let ext = media::extension(&skipped[0]);
        cells.push(extra(
            format!("request:{ext}"),
            "REQ".into(),
            request_label(lang).into(),
            request_hover(lang).into(),
            true,
        ));
    }
    cells
}

pub fn plan(outcome: &Outcome, files: &[PathBuf], state: &State) -> HivePlan {
    let target: &[PathBuf] = match outcome {
        Outcome::Supported => files,
        Outcome::Mixed { supported, .. } => supported,
        Outcome::Unsupported { .. } | Outcome::MissingEngine { .. } => files,
        Outcome::NoSelection => &[],
    };
    let reports = if target.is_empty() {
        Vec::new()
    } else {
        let refs: Vec<&Path> = target.iter().map(PathBuf::as_path).collect();
        unsupported::capability_reports(&state.registry, &refs, &|a| action_reason(state, a))
    };
    plan_with_reports(outcome, files, state, &reports)
}

/// Builds a [`HivePlan`] from precomputed S-145 [`CapabilityReport`] entries
/// for the selection, keeping actionable cells limited to runnable actions.
pub fn plan_with_reports(
    outcome: &Outcome,
    files: &[PathBuf],
    state: &State,
    reports: &[CapabilityReport],
) -> HivePlan {
    let lang = state.lang;
    if matches!(outcome, Outcome::NoSelection) {
        return HivePlan {
            centre: text::selection(lang, 0, None),
            message: String::new(),
            suggestion: None,
            cells: Vec::new(),
            pending: None,
        };
    }
    let (supported, skipped): (&[PathBuf], &[PathBuf]) = match outcome {
        Outcome::Supported => (files, &[]),
        Outcome::Mixed {
            supported,
            unsupported,
        } => (supported, unsupported),
        Outcome::Unsupported { .. } | Outcome::MissingEngine { .. } => (&[], files),
        Outcome::NoSelection => (&[], &[]),
    };
    let mut cells: Vec<HiveCell> = if supported.is_empty() {
        Vec::new()
    } else {
        reports
            .iter()
            .filter(|r| r.runnable)
            .filter_map(|r| state.registry.get(&r.action_id))
            .map(|a| ui::cell(a, lang))
            .collect()
    };
    let unavailable_count = reports
        .iter()
        .filter(|r| is_unavailable_matching(r))
        .count();
    let first_unavailable = reports.iter().find(|r| is_unavailable_matching(r));

    if !cells.is_empty() {
        cells.extend(extras(
            skipped,
            MAX_CELLS.saturating_sub(cells.len()),
            state,
            lang,
            None,
        ));
        let centre = text::selection(lang, supported.len(), common_kind(supported));
        let message = unsupported::unavailable_actions_hint(unavailable_count, lang);
        return HivePlan {
            centre,
            message,
            suggestion: None,
            cells,
            pending: Some(supported.to_vec()),
        };
    }

    let zero_skipped = if skipped.is_empty() { files } else { skipped };
    let message = match first_unavailable {
        Some(report) => typed_unavailable_message(report, lang),
        None => match outcome {
            Outcome::MissingEngine { tool } => unsupported::missing_engine_message(tool, lang),
            _ => unsupported::unsupported_message(lang).to_string(),
        },
    };
    let single_override = (zero_skipped.len() == 1).then_some(message.as_str());
    cells.extend(extras(
        zero_skipped,
        MAX_CELLS,
        state,
        lang,
        single_override,
    ));
    let centre = unsupported::centre_text(files, lang);
    let suggestion =
        if matches!(outcome, Outcome::Unsupported { .. }) && first_unavailable.is_none() {
            let ok = |e: &str| state.available.iter().any(|a| a == e);
            unsupported::suggestion(files, &state.registry, &ok, lang)
        } else {
            None
        };
    HivePlan {
        centre,
        message,
        suggestion,
        cells,
        pending: None,
    }
}
