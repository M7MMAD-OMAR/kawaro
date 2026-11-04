//! The six tools: their descriptions for `tools/list` and the calls.
//!
//! Every write tool runs an engine in this process, through the same
//! `queue::run_job` the CLI uses, so the no overwrite rule and the output
//! naming stay in kawaro-core.

use crate::config::Config;
use crate::preview;
use crate::protocol::DISCOVER_VERSIONS;
use kawaro_core::action::{ActionDef, Registry};
use kawaro_core::engine::{SkipReason, TaskDetail, detail_line};
use kawaro_core::i18n::Lang;
use kawaro_core::queue::{self, JobEvent};
use kawaro_core::{CancelToken, Engine};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc;

/// What a read only server answers for every write tool.
const READ_ONLY_MESSAGE: &str =
    "the server is read only; set [mcp] read_only = false in config.toml to allow writes";

/// The `kawaro_pdf` ops and the action each runs. The op enum of the tool
/// schema is built from this table, so a new op cannot drift from the tool.
const PDF_OPS: [(&str, &str); 10] = [
    ("merge", "pdf.merge"),
    ("split", "pdf.split"),
    ("rotate", "pdf.rotate"),
    ("compress", "pdf.compress"),
    ("to_images", "pdf.to-images"),
    ("extract_pages", "pdf.extract-pages"),
    ("delete_pages", "pdf.delete-pages"),
    ("reorder_pages", "pdf.reorder-pages"),
    ("metadata", "pdf.metadata"),
    ("to_text", "pdf.to-text"),
];

/// The `kawaro_image` ops and the action each runs.
const IMAGE_OPS: [(&str, &str); 5] = [
    ("resize", "image.resize-50"),
    ("rotate", "image.rotate-90"),
    ("flip", "image.flip-h"),
    ("grayscale", "image.grayscale"),
    ("compress", "image.compress"),
];

/// `tools/list`: six tools, in a fixed order so a host's cache is
/// deterministic.
pub(crate) fn list() -> Value {
    let pdf_ops: Vec<&'static str> = PDF_OPS.iter().map(|(name, _)| *name).collect();
    json!({
        "resultType": "complete",
        "tools": [
            tool(
                "kawaro_formats",
                "Kawaro formats and engines",
                "List the file actions, formats and engines this machine can use. Reads no file and writes none.",
                json!({"type": "object", "properties": {}, "additionalProperties": false}),
                false,
            ),
            tool(
                "kawaro_plan",
                "Plan a conversion",
                "Show which Kawaro actions fit these files and the exact path each result would take. Writes nothing.",
                json!({
                    "type": "object",
                    "properties": {"paths": paths_schema()},
                    "required": ["paths"],
                    "additionalProperties": false,
                }),
                false,
            ),
            tool(
                "kawaro_convert",
                "Run a Kawaro action",
                "Run one Kawaro action id on local files. Saves one new file next to each original. The originals are never changed and no file is overwritten.",
                json!({
                    "type": "object",
                    "properties": {
                        "action": {"type": "string", "description": "Action id from kawaro_plan or kawaro_formats."},
                        "paths": paths_schema(),
                        "options": options_schema(),
                        "preview": preview_schema(),
                    },
                    "required": ["action", "paths"],
                    "additionalProperties": false,
                }),
                true,
            ),
            tool(
                "kawaro_images_to_pdf",
                "Combine images into one PDF",
                "Combine local image files into one PDF, saved next to the first image. The images are never changed.",
                json!({
                    "type": "object",
                    "properties": {
                        "paths": paths_schema(),
                        "options": options_schema(),
                        "preview": preview_schema(),
                    },
                    "required": ["paths"],
                    "additionalProperties": false,
                }),
                true,
            ),
            tool(
                "kawaro_pdf",
                "Work on PDF files",
                "Work on local PDF files: merge, split, extract, delete or reorder pages, rotate, compress, set or clear metadata, or turn pages into text or PNG images in one ZIP. Saves new files next to the originals.",
                json!({
                    "type": "object",
                    "properties": {
                        "op": {"type": "string", "enum": pdf_ops},
                        "paths": paths_schema(),
                        "options": options_schema(),
                        "preview": preview_schema(),
                    },
                    "required": ["op", "paths"],
                    "additionalProperties": false,
                }),
                true,
            ),
            tool(
                "kawaro_image",
                "Work on image files",
                "Work on local image files: resize, rotate, flip, grayscale or compress. The file format is kept and the result goes next to the original.",
                json!({
                    "type": "object",
                    "properties": {
                        "op": {"type": "string", "enum": ["resize", "rotate", "flip", "grayscale", "compress"]},
                        "paths": paths_schema(),
                        "options": options_schema(),
                        "preview": preview_schema(),
                    },
                    "required": ["op", "paths"],
                    "additionalProperties": false,
                }),
                true,
            ),
        ],
        "ttlMs": 300000,
        "cacheScope": "private",
    })
}

fn paths_schema() -> Value {
    json!({"type": "array", "items": {"type": "string"}, "minItems": 1})
}

fn options_schema() -> Value {
    json!({
        "type": "object",
        "description": "Values for the action options, shaped by the JSON Schema kawaro_plan reports for the action.",
    })
}

fn preview_schema() -> Value {
    json!({
        "type": "boolean",
        "default": true,
        "description": "Add a small PNG of the first image result to the answer.",
    })
}

fn tool(name: &str, title: &str, description: &str, schema: Value, writes: bool) -> Value {
    json!({
        "name": name,
        "title": title,
        "description": description,
        "inputSchema": schema,
        "annotations": {
            "readOnlyHint": !writes,
            "destructiveHint": false,
            "idempotentHint": !writes,
            "openWorldHint": false,
        },
    })
}

/// Calls one tool. `None` is an unknown tool name.
pub(crate) fn call(name: &str, args: &Value, config: &Config) -> Option<Value> {
    match name {
        "kawaro_formats" => Some(formats()),
        "kawaro_plan" => Some(plan(args, config)),
        // The action id is read inside run_action, after the paths and the
        // read only check, so the refusal order holds.
        "kawaro_convert" => Some(run_action(None, args, config)),
        "kawaro_images_to_pdf" => Some(run_action(Some("image.to-pdf"), args, config)),
        "kawaro_pdf" => Some(op_action("pdf", args, config, &PDF_OPS)),
        "kawaro_image" => Some(op_action("image", args, config, &IMAGE_OPS)),
        _ => None,
    }
}

/// The top level arguments each tool takes. The input schema is host side
/// only, so the server rejects anything else with BAD_OPTIONS.
fn accepted_arguments(tool: &str) -> Option<&'static [&'static str]> {
    match tool {
        "kawaro_formats" => Some(&[]),
        "kawaro_plan" => Some(&["paths"]),
        "kawaro_convert" => Some(&["action", "options", "paths", "preview"]),
        "kawaro_images_to_pdf" => Some(&["options", "paths", "preview"]),
        "kawaro_pdf" | "kawaro_image" => Some(&["op", "options", "paths", "preview"]),
        _ => None,
    }
}

/// A call that carries an argument the tool does not take, as a BAD_OPTIONS
/// tool failure. `None` when every key is known, or for an unknown tool.
pub(crate) fn reject_unknown_arguments(tool: &str, args: &Value) -> Option<Value> {
    let accepted = accepted_arguments(tool)?;
    let arguments = args.as_object()?;
    let unknown = arguments
        .keys()
        .find(|key| !accepted.contains(&key.as_str()))?;
    let names = if accepted.is_empty() {
        String::from("none")
    } else {
        accepted.join(", ")
    };
    Some(failure(
        "BAD_OPTIONS",
        &format!("unknown argument `{unknown}`; accepted: {names}"),
    ))
}

/// The `paths` argument as strings, for the audit line.
pub(crate) fn requested_paths(args: &Value) -> Vec<String> {
    args.get("paths")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// The output paths in a tool result, for the audit line.
pub(crate) fn output_paths(result: &Value) -> Vec<String> {
    result
        .pointer("/structuredContent/outputs")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|output| output.get("path"))
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// The registry the server reports and runs: the built-in actions, plus the
/// user's own action files when that folder exists.
fn registry() -> Registry {
    let mut registry = Registry::builtin();
    if let Some(dir) = kawaro_core::config::Config::actions_dir()
        && let Err(e) = registry.add_dir(&dir)
    {
        eprintln!(
            "kawaro mcp: ignoring user actions in {}: {e}",
            dir.display()
        );
    }
    registry
}

/// `kawaro_formats`: version, revisions, engines and the action catalog.
fn formats() -> Value {
    let registry = registry();
    let engines: Vec<Value> = kawaro_engines::all()
        .iter()
        .map(|engine| json!({"id": engine.id(), "available": engine.available()}))
        .collect();
    let actions: Vec<Value> = registry
        .all()
        .iter()
        .map(|action| {
            json!({
                "id": action.id,
                "engine": action.engine,
                "ring": action.ring,
                "label": action.label.en,
                "has_options": action.options.is_some(),
            })
        })
        .collect();
    let text = format!(
        "Kawaro {}: {} actions, {} engines on this machine.",
        env!("CARGO_PKG_VERSION"),
        registry.all().len(),
        engines.len()
    );
    success(
        text,
        json!({
            "version": env!("CARGO_PKG_VERSION"),
            "protocolVersions": DISCOVER_VERSIONS,
            "engines": engines,
            "actions": actions,
        }),
    )
}

/// `kawaro_plan`: the actions that fit the files, with their option
/// schema and the path each result would take. Writes nothing.
fn plan(args: &Value, config: &Config) -> Value {
    let files = match files(args, config) {
        Ok(files) => files,
        Err(value) => return value,
    };
    let registry = registry();
    let engines = kawaro_engines::all();
    let available: Vec<String> = engines
        .iter()
        .filter(|engine| engine.available())
        .map(|engine| engine.id().to_string())
        .collect();
    let refs: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
    let matching = registry.matching(&refs, &|id| available.iter().any(|known| known == id));
    let actions: Vec<Value> = matching
        .iter()
        .map(|action| {
            let outputs: Vec<String> = queue::plan(action, &files)
                .into_iter()
                .map(|(_, final_path)| final_path.to_string_lossy().into_owned())
                .collect();
            json!({
                "id": action.id,
                "ring": action.ring,
                "label": action.label.en,
                "engine": action.engine,
                "options": action
                    .options
                    .as_ref()
                    .map_or(Value::Null, |spec| spec.params_schema()),
                "outputs": outputs,
            })
        })
        .collect();
    let text = if actions.is_empty() {
        format!("No Kawaro action fits these {} files.", files.len())
    } else {
        format!("{} actions fit these {} files.", actions.len(), files.len())
    };
    success(text, json!({"actions": actions}))
}

/// `kawaro_convert`: the action comes from the arguments.
///
/// The refusal order is fixed (S-121 AC9 and the reviewer note): paths, read
/// only, action id, action lookup, matching, engine, options. A caller with
/// a bad path hears OUTSIDE_ROOTS even in read only mode, and an unknown
/// action is never reported before a path problem.
fn run_action(action_id: Option<&str>, args: &Value, config: &Config) -> Value {
    let files = match files(args, config) {
        Ok(files) => files,
        Err(value) => return value,
    };
    if config.read_only {
        return failure("READ_ONLY", READ_ONLY_MESSAGE);
    }
    let chosen = action_id.or_else(|| action_arg(args));
    let Some(action_id) = chosen.filter(|id| !id.is_empty()) else {
        return failure(
            "BAD_OPTIONS",
            "the action argument is missing or is empty; accepted: action",
        );
    };
    let registry = registry();
    let Some(action) = registry.get(action_id).cloned() else {
        return failure("UNKNOWN_ACTION", &format!("unknown action `{action_id}`"));
    };
    // Matching answers whether the action fits the files. The engine check
    // below reports a missing engine, so the engine is not filtered here.
    let refs: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
    let matching = registry.matching(&refs, &|_| true);
    if !matching.iter().any(|found| found.id == action.id) {
        let ids: Vec<&str> = matching
            .iter()
            .take(5)
            .map(|found| found.id.as_str())
            .collect();
        let hint = if ids.is_empty() {
            "no action fits these files".to_string()
        } else {
            format!("available: {}", ids.join(", "))
        };
        return failure(
            "NO_ACTION",
            &format!("action `{}` does not fit these files; {hint}", action.id),
        );
    }
    let engines = kawaro_engines::all();
    let Some(engine) = engines.iter().find(|engine| engine.id() == action.engine) else {
        return failure(
            "ENGINE_MISSING",
            &format!("engine `{}` is not part of this build", action.engine),
        );
    };
    if !engine.available() {
        return failure(
            "ENGINE_MISSING",
            &format!(
                "engine `{}` is not available on this machine",
                action.engine
            ),
        );
    }
    let options = match options(args) {
        Ok(options) => options,
        Err(value) => return value,
    };
    let mut action = action;
    if let Some(spec) = &action.options {
        match spec.resolve_params(&action.params, &options) {
            Ok(resolved) => action.params = resolved,
            Err(e) => return failure("BAD_OPTIONS", &e.to_string()),
        }
    } else if !options.is_empty() {
        return failure("BAD_OPTIONS", "this action takes no options");
    }
    run(&action, engine, &files, preview_wanted(args), config)
}

/// `kawaro_pdf` and `kawaro_image`: the op argument names the action.
///
/// The op is checked before the read only flag and the paths, so an unknown
/// op is BAD_OPTIONS even on a read only server.
fn op_action(tool: &str, args: &Value, config: &Config, ops: &[(&str, &str)]) -> Value {
    let accepted: Vec<&str> = ops.iter().map(|(name, _)| *name).collect();
    let accepted = accepted.join(", ");
    let op = args.get("op").and_then(Value::as_str);
    let Some((_, action_id)) = op.and_then(|op| ops.iter().find(|(name, _)| *name == op)) else {
        let op = op.unwrap_or("<missing>");
        return failure(
            "BAD_OPTIONS",
            &format!("unknown op {op} for {tool}; accepted: {accepted}"),
        );
    };
    run_action(Some(action_id), args, config)
}

/// Runs the action with the resolved params and builds the tool result.
fn run(
    action: &ActionDef,
    engine: &Arc<dyn Engine>,
    files: &[PathBuf],
    want_preview: bool,
    config: &Config,
) -> Value {
    let started = std::time::Instant::now();
    let (tx, rx) = mpsc::channel();
    let _ = queue::run_job(0, action, files, engine.as_ref(), &CancelToken::new(), &tx);
    drop(tx);
    let mut outputs: Vec<(PathBuf, Option<TaskDetail>)> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut skipped: Vec<(PathBuf, SkipReason)> = Vec::new();
    for event in rx {
        match event {
            JobEvent::TaskDone { output, detail, .. } => outputs.push((output, detail)),
            JobEvent::TaskFailed { error, .. } => errors.push(error),
            JobEvent::TaskSkipped { input, reason, .. } => skipped.push((input, reason)),
            _ => {}
        }
    }
    let elapsed_ms = started.elapsed().as_millis() as u64;
    if outputs.is_empty() && errors.is_empty() && !skipped.is_empty() {
        // S-133: a skip is a success with no output, never an error.
        let names: Vec<String> = skipped.iter().map(|(input, _)| file_name(input)).collect();
        let text = format!(
            "No file was written: {} is already optimized (the compressed result is not smaller, the original is kept).",
            names.join(", ")
        );
        return json!({
            "resultType": "complete",
            "content": [{"type": "text", "text": text}],
            "structuredContent": {
                "action": action.id,
                "engine": action.engine,
                "params": serde_json::to_value(&action.params).unwrap_or(json!({})),
                "outputs": [],
                "skipped": skipped_entries(&skipped),
                "elapsed_ms": elapsed_ms,
                "overwritten": false,
            },
            "isError": false,
        });
    }
    if outputs.is_empty() && errors.is_empty() {
        return failure("FAILED", "No file was written.");
    }
    if let Some(first) = errors.first() {
        return json!({
            "resultType": "complete",
            "content": [{"type": "text", "text": format!("ERROR [FAILED]: {first}")}],
            "structuredContent": {
                "error": {"code": "FAILED", "message": first},
                "outputs": outputs.iter().map(|(path, detail)| output_entry(path.as_path(), detail.as_ref())).collect::<Vec<Value>>(),
            },
            "isError": true,
        });
    }
    let params = match serde_json::to_value(&action.params) {
        Ok(params) => params,
        Err(_) => json!({}),
    };
    let mut content = vec![json!({"type": "text", "text": summary(&outputs)})];
    if want_preview
        && let Some(block) = outputs
            .iter()
            .find(|(path, _)| preview::is_image(path.as_path()))
            .and_then(|(path, _)| preview::block(path, config.preview_px))
    {
        content.push(block);
    }
    for (path, _) in &outputs {
        content.push(json!({
            "type": "resource_link",
            "uri": preview::file_uri(path),
            "name": file_name(path),
            "mimeType": preview::mime_type(path),
        }));
    }
    let mut structured = json!({
        "action": action.id,
        "engine": action.engine,
        "params": params,
        "outputs": outputs.iter().map(|(path, detail)| output_entry(path.as_path(), detail.as_ref())).collect::<Vec<Value>>(),
        "elapsed_ms": elapsed_ms,
        "overwritten": false,
    });
    if !skipped.is_empty() {
        structured["skipped"] = json!(skipped_entries(&skipped));
    }
    json!({
        "resultType": "complete",
        "content": content,
        "structuredContent": structured,
        "isError": false,
    })
}

/// The text block of a finished call: every absolute output path, its size,
/// and where it went.
fn summary(outputs: &[(PathBuf, Option<TaskDetail>)]) -> String {
    if outputs.is_empty() {
        return "No file was written.".to_string();
    }
    let entries: Vec<String> = outputs
        .iter()
        .map(|(path, detail)| match detail {
            Some(detail) => format!("{} ({})", path.display(), detail_line(detail, Lang::En)),
            None => format!("{} ({} bytes)", path.display(), bytes(path)),
        })
        .collect();
    if entries.len() == 1 {
        format!("Created {} next to the originals.", entries[0])
    } else {
        format!(
            "Created {} files next to the originals: {}.",
            entries.len(),
            entries.join(", ")
        )
    }
}

fn output_entry(path: &Path, detail: Option<&TaskDetail>) -> Value {
    let mut entry = json!({
        "path": path.to_string_lossy().into_owned(),
        "bytes": bytes(path),
    });
    if let Some(detail) = detail {
        entry["width"] = json!(detail.width);
        entry["height"] = json!(detail.height);
        entry["source_width"] = json!(detail.source_width);
        entry["source_height"] = json!(detail.source_height);
        entry["downscaled"] = json!(detail.downscaled);
    }
    entry
}

fn bytes(path: &Path) -> u64 {
    std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}

/// The skipped list of a result (S-133), the reason serialized kebab-case.
fn skipped_entries(skipped: &[(PathBuf, SkipReason)]) -> Vec<Value> {
    skipped
        .iter()
        .map(|(input, reason)| {
            json!({
                "path": input.to_string_lossy().into_owned(),
                "reason": serde_json::to_value(reason).unwrap_or(Value::Null),
            })
        })
        .collect()
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The `paths` argument, checked: at most `max_files`, made absolute,
/// canonicalised, a file, and inside one of the allowed roots.
fn files(args: &Value, config: &Config) -> Result<Vec<PathBuf>, Value> {
    let Some(list) = args.get("paths").and_then(Value::as_array) else {
        return Err(failure(
            "FAILED",
            "the paths argument is missing or is not an array of strings",
        ));
    };
    if list.is_empty() {
        return Err(failure("FAILED", "no paths were given"));
    }
    if list.len() > config.max_files.max(1) {
        return Err(failure(
            "TOO_MANY_FILES",
            &format!(
                "{} files were given, the limit is {}",
                list.len(),
                config.max_files
            ),
        ));
    }
    let roots = config.resolved_roots();
    let mut files = Vec::with_capacity(list.len());
    for entry in list {
        let Some(text) = entry.as_str() else {
            return Err(failure("FAILED", "every path must be a string"));
        };
        let path = Path::new(text);
        let absolute = match std::path::absolute(path) {
            Ok(absolute) => absolute,
            Err(_) => return Err(failure("OUTSIDE_ROOTS", &not_allowed(path))),
        };
        let canonical = match std::fs::canonicalize(&absolute) {
            Ok(canonical) => canonical,
            Err(_) => return Err(failure("OUTSIDE_ROOTS", &not_allowed(path))),
        };
        if !canonical.is_file() {
            return Err(failure("OUTSIDE_ROOTS", &not_allowed(&canonical)));
        }
        if !roots.iter().any(|root| canonical.starts_with(root)) {
            return Err(failure("OUTSIDE_ROOTS", &not_allowed(&canonical)));
        }
        files.push(canonical);
    }
    Ok(files)
}

/// A path refusal: the file is missing, is not a regular file, or is not
/// inside one of the allowed roots (S-121 AC9).
fn not_allowed(path: &Path) -> String {
    format!(
        "{}: not a file inside the allowed roots (set [mcp] allowed_roots in config.toml)",
        path.display()
    )
}

/// The `options` argument as a TOML table. JSON null is an empty table.
fn options(args: &Value) -> Result<toml::Table, Value> {
    let map = match args.get("options") {
        None | Some(Value::Null) => return Ok(toml::Table::new()),
        Some(Value::Object(map)) => map,
        Some(_) => return Err(failure("BAD_OPTIONS", "options must be an object")),
    };
    let mut table = toml::Table::new();
    for (key, value) in map {
        let Some(value) = json_to_toml(value) else {
            return Err(failure(
                "BAD_OPTIONS",
                &format!("option `{key}` is not a TOML value"),
            ));
        };
        table.insert(key.clone(), value);
    }
    Ok(table)
}

/// A JSON value as a TOML value. JSON null has no TOML form.
fn json_to_toml(value: &Value) -> Option<toml::Value> {
    match value {
        Value::Null => None,
        Value::Bool(inner) => Some(toml::Value::Boolean(*inner)),
        Value::Number(number) => match number.as_i64() {
            Some(inner) => Some(toml::Value::Integer(inner)),
            None => number.as_f64().map(toml::Value::Float),
        },
        Value::String(inner) => Some(toml::Value::String(inner.clone())),
        Value::Array(items) => {
            let items: Option<Vec<toml::Value>> = items.iter().map(json_to_toml).collect();
            items.map(toml::Value::Array)
        }
        Value::Object(map) => {
            let mut table = toml::Table::new();
            for (key, inner) in map {
                table.insert(key.clone(), json_to_toml(inner)?);
            }
            Some(toml::Value::Table(table))
        }
    }
}

fn preview_wanted(args: &Value) -> bool {
    args.get("preview").and_then(Value::as_bool).unwrap_or(true)
}

fn action_arg(args: &Value) -> Option<&str> {
    args.get("action").and_then(Value::as_str)
}

/// A tool result with a text block and the structured content.
fn success(text: String, structured: Value) -> Value {
    json!({
        "resultType": "complete",
        "content": [{"type": "text", "text": text}],
        "structuredContent": structured,
        "isError": false,
    })
}

/// A failed tool call: `ERROR [CODE]: message` in the text block and the
/// same code in the structured content.
fn failure(code: &str, message: &str) -> Value {
    json!({
        "resultType": "complete",
        "content": [{"type": "text", "text": format!("ERROR [{code}]: {message}")}],
        "structuredContent": {"error": {"code": code, "message": message}},
        "isError": true,
    })
}
