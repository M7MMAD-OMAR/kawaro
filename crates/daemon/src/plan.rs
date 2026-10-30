//! S-108: what the hive shows for a selection, as plain data. Pure: no I/O,
//! no process, no network. `server::open` builds a [`HivePlan`] and then
//! presents it; [`on_choice`] decides what a chosen cell does.

use crate::state::{State, common_kind};
use crate::{text, ui};
use kawaro_core::i18n::Lang;
use kawaro_core::media;
use kawaro_core::unsupported::{
    self, Outcome, more_label, request_hover, request_label, request_url, skipped_label,
};
use kawaro_ui::HiveCell;
use std::path::PathBuf;

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
    }
}

/// The reason one skipped file has no action.
fn reason(file: &PathBuf, state: &State, lang: Lang) -> String {
    match state.classify(std::slice::from_ref(file)) {
        Outcome::MissingEngine { tool } => unsupported::missing_engine_message(tool, lang),
        _ => unsupported::unsupported_message(lang).to_string(),
    }
}

/// The greyed cells, `+K` and the request cell that fit in `room` cells.
fn extras(skipped: &[PathBuf], room: usize, state: &State, lang: Lang) -> Vec<HiveCell> {
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
            let why = reason(f, state, lang);
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
    let lang = state.lang;
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
        state
            .matching(supported)
            .iter()
            .map(|a| ui::cell(a, lang))
            .collect()
    };
    cells.extend(extras(
        skipped,
        MAX_CELLS.saturating_sub(cells.len()),
        state,
        lang,
    ));
    let (centre, message) = match outcome {
        Outcome::Unsupported { .. } => (
            unsupported::centre_text(files, lang),
            unsupported::unsupported_message(lang).to_string(),
        ),
        Outcome::MissingEngine { tool } => (
            unsupported::centre_text(files, lang),
            unsupported::missing_engine_message(tool, lang),
        ),
        _ => (
            text::selection(lang, supported.len(), common_kind(supported)),
            String::new(),
        ),
    };
    let suggestion = match outcome {
        Outcome::Unsupported { .. } => {
            let ok = |e: &str| state.available.iter().any(|a| a == e);
            unsupported::suggestion(files, &state.registry, &ok, lang)
        }
        _ => None,
    };
    let pending = (!supported.is_empty()).then(|| supported.to_vec());
    HivePlan {
        centre,
        message,
        suggestion,
        cells,
        pending,
    }
}
