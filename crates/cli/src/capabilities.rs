//! S-145 CLI capability surface: reports every unavailable action with its
//! typed reason and curated help text for `kawaro doctor`. Read-only: never
//! runs an installer, never invokes `sudo`, and never starts a conversion job.

use kawaro_core::i18n::Lang;
use kawaro_core::unsupported::{
    CapabilityReport, UnavailableReason, capability_reports, default_engine_reason, help_message,
    reason_message,
};
use kawaro_core::{ActionDef, CancelToken, Engine, Registry};
use kawaro_engines::ocr::OcrEngine;
use kawaro_engines::ocrmypdf::OcrmypdfEngine;
use std::sync::Arc;

/// Evaluates every registered action against the current engine list, OCR
/// language probe and OCRmyPDF version probe, returning the shared S-145
/// [`CapabilityReport`] list.
pub(crate) fn evaluate(
    reg: &Registry,
    engines: &[Arc<dyn Engine>],
    ocr: &OcrEngine,
) -> Vec<CapabilityReport> {
    let ocrmypdf = OcrmypdfEngine::discover();
    evaluate_with(reg, engines, ocr, &ocrmypdf)
}

/// Evaluates every registered action with explicit `ocr` and `ocrmypdf`
/// handles, so tests can inject fake executables.
pub(crate) fn evaluate_with(
    reg: &Registry,
    engines: &[Arc<dyn Engine>],
    ocr: &OcrEngine,
    ocrmypdf: &OcrmypdfEngine,
) -> Vec<CapabilityReport> {
    let cancel = CancelToken::new();
    capability_reports(reg, &[], &|action: &ActionDef| {
        let available = engines
            .iter()
            .any(|e| e.id() == action.engine && e.available());
        if !available {
            return default_engine_reason(&action.engine, false);
        }
        if action.engine == "ocr" {
            let lang = action
                .params
                .get("lang")
                .and_then(|v| v.as_str())
                .unwrap_or("ara+eng");
            return ocr.check_language(lang, &cancel).err();
        }
        if action.engine == "ocrmypdf" {
            let lang = action
                .params
                .get("lang")
                .and_then(|v| v.as_str())
                .unwrap_or("ara+eng");
            if ocrmypdf.program().is_none() {
                return Some(UnavailableReason::MissingExecutable {
                    program: String::from("ocrmypdf"),
                });
            }
            if let Some(reason) = ocrmypdf.probe_version(&cancel).into_reason("ocrmypdf") {
                return Some(reason);
            }
            return ocr.check_language(lang, &cancel).err();
        }
        None
    })
}

/// Formats the read-only `unavailable actions` block for `kawaro doctor`.
pub(crate) fn format_unavailable(reports: &[CapabilityReport], lang: Lang) -> Vec<String> {
    let unavailable: Vec<&CapabilityReport> = reports.iter().filter(|r| !r.runnable).collect();
    let mut lines = Vec::new();
    lines.push(String::from("unavailable actions"));
    if unavailable.is_empty() {
        lines.push(String::from("  none"));
        return lines;
    }
    for report in unavailable {
        let Some(reason) = report.reason.as_ref() else {
            continue;
        };
        let why = reason_message(reason, lang);
        lines.push(format!("  {:<24} {why}", report.action_id));
        if let Some(help_id) = report.help_id.as_deref()
            && let Some(help) = help_message(help_id, reason, lang)
        {
            lines.push(format!("    help ({help_id}): {help}"));
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s145_doctor_formats_missing_tool_missing_language_and_policy_distinctly() {
        let reports = vec![
            CapabilityReport {
                action_id: String::from("image.to-webp"),
                runnable: true,
                reason: None,
                help_id: None,
            },
            CapabilityReport {
                action_id: String::from("pdf.linearize"),
                runnable: false,
                reason: Some(UnavailableReason::MissingExecutable {
                    program: String::from("qpdf"),
                }),
                help_id: Some(String::from("qpdf")),
            },
            CapabilityReport {
                action_id: String::from("image.to-text"),
                runnable: false,
                reason: Some(UnavailableReason::MissingLanguageData {
                    language: String::from("ara"),
                }),
                help_id: Some(String::from("ocr-lang")),
            },
            CapabilityReport {
                action_id: String::from("mcp.action"),
                runnable: false,
                reason: Some(UnavailableReason::DisabledByPolicy),
                help_id: None,
            },
        ];

        let text = format_unavailable(&reports, Lang::En).join("\n");
        assert!(text.starts_with("unavailable actions\n"), "{text}");
        assert!(!text.contains("image.to-webp"), "{text}");
        assert!(text.contains("pdf.linearize"), "{text}");
        assert!(text.contains("https://qpdf.readthedocs.io/"), "{text}");
        assert!(text.contains("image.to-text"), "{text}");
        assert!(text.contains("ara"), "{text}");
        assert!(
            text.contains("https://tesseract-ocr.github.io/tessdoc/"),
            "{text}"
        );
        assert!(text.contains("mcp.action"), "{text}");
        assert!(text.contains("Disabled by configuration policy"), "{text}");
        assert!(!text.contains("sudo"), "{text}");
        assert!(
            !text.contains('\u{2013}') && !text.contains('\u{2014}'),
            "{text}"
        );
    }

    #[test]
    fn s145_doctor_prints_none_when_every_action_is_runnable() {
        let reports = vec![CapabilityReport {
            action_id: String::from("image.to-webp"),
            runnable: true,
            reason: None,
            help_id: None,
        }];
        assert_eq!(
            format_unavailable(&reports, Lang::En),
            vec![String::from("unavailable actions"), String::from("  none")]
        );
    }

    #[test]
    fn s150_doctor_formats_ocrmypdf_missing_and_probe_unknown_with_curated_help() {
        let reports = vec![
            CapabilityReport {
                action_id: String::from("pdf.make-searchable"),
                runnable: false,
                reason: Some(UnavailableReason::MissingExecutable {
                    program: String::from("ocrmypdf"),
                }),
                help_id: Some(String::from("ocrmypdf")),
            },
            CapabilityReport {
                action_id: String::from("pdf.make-searchable"),
                runnable: false,
                reason: Some(UnavailableReason::ProbeUnknown),
                help_id: Some(String::from("ocrmypdf")),
            },
        ];
        for lang in [Lang::En, Lang::Ar] {
            let text = format_unavailable(&reports, lang).join("\n");
            assert!(text.contains("pdf.make-searchable"), "{text}");
            assert!(text.contains("ocrmypdf"), "{text}");
            assert!(text.contains("14.4.0"), "{text}");
            assert!(text.contains("Fedora"), "{text}");
            assert!(text.contains("Debian"), "{text}");
            assert!(
                text.contains("https://ocrmypdf.readthedocs.io/en/stable/installation.html"),
                "{text}"
            );
            assert!(!text.contains("sudo"), "{text}");
            assert!(
                !text.contains('\u{2013}') && !text.contains('\u{2014}'),
                "{text}"
            );
        }
    }
}
