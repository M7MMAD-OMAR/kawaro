//! PDF engines (S-010, S-123, S-125).
//!
//! `available()` is per engine, not per op, so the PDF work is split in two:
//!
//! - [`PdfEngine`] (id "pdf"): merge, rotate, split, extract-pages,
//!   delete-pages, reorder-pages, metadata. Pure Rust on `lopdf`, always
//!   available.
//! - [`GhostscriptEngine`] (id "ghostscript"): compress, to-images. Runs `gs`
//!   as a child process and is unavailable without it.
//!
//! An engine task writes exactly one file, so `split` and `to-images` write
//! one ZIP (stored, no recompression: PDF pages and PNG are already
//! compressed) holding `page-001.pdf`, `page-002.pdf` or `page-001.png` and
//! so on. In `split` the number is the output file: one page, one `ranges`
//! range or `every` pages.

use crate::locate::{self, Program};
use crate::office::{
    failure_message, input_error, is_nonempty_file, missing, missing_on_not_found, move_file,
    run_child, single_input, timeout_param, work_dir,
};
use kawaro_core::engine::{SkipReason, TaskOutcome};
use kawaro_core::{CancelToken, Engine, EngineError, MediaKind, Progress, Task};
use lopdf::{Document, Object, ObjectId, dictionary};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// Default `timeout_s` for one Ghostscript run.
pub const GS_DEFAULT_TIMEOUT_S: f64 = 300.0;
/// Accepted `dpi` range for `to-images`. 600 dpi on A4 is already about
/// 5000 x 7000 pixels per page.
pub const DPI_RANGE: std::ops::RangeInclusive<i64> = 36..=600;
pub const DEFAULT_DPI: i64 = 150;
/// Where to get Ghostscript, for the message when it is missing.
const GS_URL: &str = "https://ghostscript.com/releases/";

/// Page attributes a page inherits from its ancestors in the page tree.
const INHERITABLE: [&str; 4] = ["MediaBox", "CropBox", "Resources", "Rotate"];

/// The only params the `metadata` op accepts.
const METADATA_PARAMS: [&str; 5] = ["title", "author", "subject", "keywords", "clear"];

/// The text params of `metadata` and the Info key each one sets.
const INFO_TEXT: [(&str, &str); 4] = [
    ("title", "Title"),
    ("author", "Author"),
    ("subject", "Subject"),
    ("keywords", "Keywords"),
];

pub struct PdfEngine;

impl Engine for PdfEngine {
    fn id(&self) -> &'static str {
        "pdf"
    }

    fn capabilities(&self) -> Vec<(&'static str, Vec<MediaKind>)> {
        [
            "merge",
            "rotate",
            "split",
            "extract-pages",
            "delete-pages",
            "reorder-pages",
            "metadata",
        ]
        .into_iter()
        .map(|op| (op, vec![MediaKind::Pdf]))
        .collect()
    }

    fn run(
        &self,
        task: &Task,
        progress: Progress,
        cancel: &CancelToken,
    ) -> Result<TaskOutcome, EngineError> {
        progress(0.0);
        cancel.check()?;
        match task.op.as_str() {
            "merge" => merge(task, progress, cancel),
            "rotate" => rotate(task, cancel),
            "split" => split(task, progress, cancel),
            "extract-pages" => extract_pages(task, progress, cancel),
            "delete-pages" => delete_pages(task, progress, cancel),
            "reorder-pages" => reorder_pages(task, progress, cancel),
            "metadata" => metadata(task, progress, cancel),
            op => Err(EngineError::UnsupportedOp(op.to_string())),
        }?;
        progress(1.0);
        Ok(TaskOutcome::Written)
    }
}

fn load_pdf(path: &Path) -> Result<Document, EngineError> {
    let mut doc = Document::load(path).map_err(|e| input_error(path, e))?;
    if doc.is_encrypted() {
        return Err(input_error(path, "the PDF is encrypted with a password"));
    }
    flatten_inherited(&mut doc);
    Ok(doc)
}

/// Copies inherited attributes onto every page, so a page keeps its size,
/// fonts and rotation when it moves to another page tree.
fn flatten_inherited(doc: &mut Document) {
    for page in doc.get_pages().into_values() {
        let found: Vec<_> = INHERITABLE
            .iter()
            .filter_map(|key| inherited(doc, page, key.as_bytes()).map(|v| (*key, v)))
            .collect();
        if let Ok(dict) = doc.get_dictionary_mut(page) {
            for (key, value) in found {
                dict.set(key, value);
            }
        }
    }
}

/// The value of `key` for `page` when the page itself lacks it, from the
/// nearest ancestor. None when the page has it or no ancestor does.
fn inherited(doc: &Document, page: ObjectId, key: &[u8]) -> Option<Object> {
    let mut dict = doc.get_dictionary(page).ok()?;
    if dict.has(key) {
        return None;
    }
    // Depth guard against malformed, cyclic Parent chains.
    for _ in 0..64 {
        let parent = dict.get(b"Parent").and_then(Object::as_reference).ok()?;
        dict = doc.get_dictionary(parent).ok()?;
        if let Ok(value) = dict.get(key) {
            return Some(value.clone());
        }
    }
    None
}

fn merge(task: &Task, progress: Progress, cancel: &CancelToken) -> Result<(), EngineError> {
    if task.inputs.is_empty() {
        return Err(EngineError::BadParam {
            name: "inputs".into(),
            reason: "merge needs at least one PDF".into(),
        });
    }
    let mut out = Document::with_version("1.7");
    let pages_id: ObjectId = (1, 0);
    let catalog_id: ObjectId = (2, 0);
    let mut next_id = 3;
    let mut kids = Vec::new();
    let total = task.inputs.len() as f32;
    for (n, path) in task.inputs.iter().enumerate() {
        cancel.check()?;
        let mut doc = load_pdf(path)?;
        doc.renumber_objects_with(next_id);
        next_id = doc.max_id + 1;
        let pages: Vec<ObjectId> = doc.get_pages().into_values().collect();
        // Everything is copied. The old catalog, page tree and cross-reference
        // stream become unreachable once the pages get the new Parent, and
        // prune_objects drops them below.
        out.objects.extend(doc.objects);
        for page in pages {
            if let Ok(dict) = out.get_dictionary_mut(page) {
                dict.set("Parent", pages_id);
                kids.push(Object::Reference(page));
            }
        }
        progress((n + 1) as f32 / total * 0.9);
    }
    out.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Count" => kids.len() as i64,
            "Kids" => kids,
        }),
    );
    out.objects.insert(
        catalog_id,
        Object::Dictionary(dictionary! { "Type" => "Catalog", "Pages" => pages_id }),
    );
    out.trailer.set("Root", catalog_id);
    out.prune_objects();
    // The highest id still in use, so the dropped objects reserve no ids.
    out.max_id = out.objects.keys().next_back().map_or(0, |id| id.0);
    save(&mut out, &task.output)
}

fn save(doc: &mut Document, path: &Path) -> Result<(), EngineError> {
    doc.save(path)
        .map(drop)
        .map_err(|e| EngineError::Failed(format!("cannot write {}: {e}", path.display())))
}

/// `degrees` param: 90 (default), 180 or 270.
fn degrees_param(task: &Task) -> Result<i64, EngineError> {
    let degrees = match task.params.get("degrees") {
        None => 90,
        Some(_) => task.param_i64("degrees").unwrap_or(0),
    };
    if [90, 180, 270].contains(&degrees) {
        Ok(degrees)
    } else {
        Err(EngineError::BadParam {
            name: "degrees".into(),
            reason: "must be 90, 180 or 270".into(),
        })
    }
}

/// New /Rotate value, normalised to 0, 90, 180 or 270.
fn rotated(current: i64, degrees: i64) -> i64 {
    (current + degrees).rem_euclid(360)
}

fn rotate(task: &Task, cancel: &CancelToken) -> Result<(), EngineError> {
    let degrees = degrees_param(task)?;
    let input = single_input(task)?;
    let mut doc = load_pdf(input)?;
    cancel.check()?;
    for page in doc.get_pages().into_values() {
        if let Ok(dict) = doc.get_dictionary_mut(page) {
            let current = dict.get(b"Rotate").and_then(Object::as_i64).unwrap_or(0);
            dict.set("Rotate", rotated(current, degrees));
        }
    }
    save(&mut doc, &task.output)
}

/// Zero padded entry name: `page-001.pdf`, wider when there are 1000 pages or more.
fn page_name(index: usize, total: usize, ext: &str) -> String {
    let width = total.to_string().len().max(3);
    format!("page-{index:0width$}.{ext}")
}

fn zip_options() -> zip::write::SimpleFileOptions {
    zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .large_file(true)
}

/// Pages named by a string like `1`, `1-3`, `1,3,5` or `1-3,7`, 1-based,
/// in the order they were written. Every page must exist and appear once.
fn parse_pages(text: &str, page_count: usize) -> Result<Vec<usize>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("the selection is empty".into());
    }
    let mut seen = vec![false; page_count + 1];
    let mut pages = Vec::new();
    for part in text.split(',') {
        let part = part.trim();
        if part.is_empty() {
            return Err(format!("`{text}` has an empty part"));
        }
        let (first, last) = match part.split_once('-') {
            Some((first, last)) => (
                page_number(first, page_count)?,
                page_number(last, page_count)?,
            ),
            None => {
                let page = page_number(part, page_count)?;
                (page, page)
            }
        };
        if last < first {
            return Err(format!("`{part}` goes backwards"));
        }
        for (page, already) in seen.iter_mut().enumerate().take(last + 1).skip(first) {
            if *already {
                return Err(format!("page {page} appears twice"));
            }
            *already = true;
            pages.push(page);
        }
    }
    Ok(pages)
}

/// One 1-based page number of a selection, inside `1..=page_count`.
fn page_number(text: &str, page_count: usize) -> Result<usize, String> {
    let text = text.trim();
    let page: usize = text
        .parse()
        .map_err(|_| format!("`{text}` is not a page number"))?;
    if page == 0 || page > page_count {
        return Err(format!("page {page} is not between 1 and {page_count}"));
    }
    Ok(page)
}

/// A string param, or `fallback` when the action leaves it out.
fn text_param<'a>(task: &'a Task, name: &str, fallback: &'a str) -> Result<&'a str, EngineError> {
    match task.params.get(name) {
        None => Ok(fallback),
        Some(value) => value.as_str().ok_or_else(|| EngineError::BadParam {
            name: name.to_string(),
            reason: "must be a string".into(),
        }),
    }
}

/// A page selection param, 1-based, in the order it was written.
fn pages_param(task: &Task, page_count: usize, name: &str) -> Result<Vec<usize>, EngineError> {
    let text = text_param(task, name, "1")?;
    parse_pages(text, page_count).map_err(|reason| EngineError::BadParam {
        name: name.to_string(),
        reason,
    })
}

/// The `order` param: every page exactly once, in the order the user wrote
/// it, so the list is a permutation of `1..=page_count`. A partial list is
/// refused; `extract-pages` is the op for a subset.
fn order_param(task: &Task, page_count: usize) -> Result<Vec<usize>, EngineError> {
    let order = pages_param(task, page_count, "order")?;
    let mut sorted = order.clone();
    sorted.sort_unstable();
    let expected: Vec<usize> = (1..=page_count).collect();
    if sorted != expected {
        return Err(EngineError::BadParam {
            name: "order".into(),
            reason: format!("must list every page from 1 to {page_count} once"),
        });
    }
    Ok(order)
}

/// The `every` param of `split`: whole pages per output, at least 1.
fn every_param(task: &Task) -> Result<usize, EngineError> {
    let every = match task.params.get("every") {
        None => 1,
        Some(_) => match task.param_i64("every") {
            Some(value) if value >= 1 => usize::try_from(value).unwrap_or(usize::MAX),
            _ => 0,
        },
    };
    if every == 0 {
        return Err(EngineError::BadParam {
            name: "every".into(),
            reason: "must be a whole number of pages, 1 or more".into(),
        });
    }
    Ok(every)
}

/// 1-based pages as 0-based page indices.
fn zero_based(pages: Vec<usize>) -> Vec<usize> {
    pages.into_iter().map(|page| page - 1).collect()
}

/// The page ids of `doc` in document order.
fn page_ids(doc: &Document) -> Vec<ObjectId> {
    doc.get_pages().into_values().collect()
}

/// The page tree root and the catalog, which a page subset copy rewrites.
fn page_tree_ids(doc: &Document, input: &Path) -> Result<(ObjectId, ObjectId), EngineError> {
    let catalog = doc.catalog().map_err(|e| input_error(input, e))?;
    let root_pages = catalog
        .get(b"Pages")
        .and_then(Object::as_reference)
        .map_err(|e| input_error(input, e))?;
    let catalog_id = doc
        .trailer
        .get(b"Root")
        .and_then(Object::as_reference)
        .map_err(|e| input_error(input, e))?;
    Ok((root_pages, catalog_id))
}

/// A copy of `doc` whose page tree holds `order` (0-based page indices) in
/// that order, under the same page tree root. Pages left out become
/// unreachable and are pruned, and the catalog drops the trees that point
/// at every page.
fn copy_with_pages(
    doc: &Document,
    pages: &[ObjectId],
    order: &[usize],
    root_pages: ObjectId,
    catalog_id: ObjectId,
) -> Document {
    let mut copy = doc.clone();
    let kids: Vec<Object> = order
        .iter()
        .map(|index| Object::Reference(pages[*index]))
        .collect();
    if let Ok(dict) = copy.get_dictionary_mut(root_pages) {
        dict.set("Kids", kids);
        dict.set("Count", order.len() as i64);
    }
    for index in order {
        if let Ok(dict) = copy.get_dictionary_mut(pages[*index]) {
            dict.set("Parent", root_pages);
        }
    }
    if let Ok(dict) = copy.get_dictionary_mut(catalog_id) {
        // These point at every page and would keep them all alive.
        for key in ["Outlines", "StructTreeRoot", "OpenAction"] {
            dict.remove(key.as_bytes());
        }
    }
    copy.prune_objects();
    copy
}

/// Writes one output PDF holding `order` (0-based) of the input pages.
fn save_pages(
    doc: &Document,
    input: &Path,
    pages: &[ObjectId],
    order: &[usize],
    task: &Task,
    progress: Progress,
) -> Result<(), EngineError> {
    let (root_pages, catalog_id) = page_tree_ids(doc, input)?;
    let mut out = copy_with_pages(doc, pages, order, root_pages, catalog_id);
    progress(0.9);
    save(&mut out, &task.output)
}

/// `extract-pages`: one PDF with the selected pages in document order.
fn extract_pages(task: &Task, progress: Progress, cancel: &CancelToken) -> Result<(), EngineError> {
    let input = single_input(task)?;
    let doc = load_pdf(input)?;
    let pages = page_ids(&doc);
    let mut selection = pages_param(task, pages.len(), "pages")?;
    selection.sort_unstable();
    cancel.check()?;
    save_pages(&doc, input, &pages, &zero_based(selection), task, progress)
}

/// `delete-pages`: one PDF with the selected pages left out.
fn delete_pages(task: &Task, progress: Progress, cancel: &CancelToken) -> Result<(), EngineError> {
    let input = single_input(task)?;
    let doc = load_pdf(input)?;
    let pages = page_ids(&doc);
    let selection = pages_param(task, pages.len(), "pages")?;
    let keep: Vec<usize> = (1..=pages.len())
        .filter(|page| !selection.contains(page))
        .map(|page| page - 1)
        .collect();
    if keep.is_empty() {
        return Err(EngineError::BadParam {
            name: "pages".into(),
            reason: "cannot delete every page".into(),
        });
    }
    cancel.check()?;
    save_pages(&doc, input, &pages, &keep, task, progress)
}

/// `reorder-pages`: one PDF with every page once, in the `order` order.
fn reorder_pages(task: &Task, progress: Progress, cancel: &CancelToken) -> Result<(), EngineError> {
    let input = single_input(task)?;
    let doc = load_pdf(input)?;
    let pages = page_ids(&doc);
    let order = order_param(task, pages.len())?;
    cancel.check()?;
    save_pages(&doc, input, &pages, &zero_based(order), task, progress)
}

/// `metadata`: a new PDF with the non-empty Info fields set, or with the
/// document information and the XMP stream cleared (S-125). The input is
/// never modified and no file is overwritten; the queue places the result.
fn metadata(task: &Task, progress: Progress, cancel: &CancelToken) -> Result<(), EngineError> {
    check_metadata_params(task)?;
    let clear = metadata_clear(task)?;
    let mut fields: Vec<(&str, &str, &str)> = Vec::new();
    for (name, key) in INFO_TEXT {
        let value = metadata_text(task, name)?;
        if !value.is_empty() {
            fields.push((name, key, value));
        }
    }
    // The options clear one another, but a raw params table can still ask
    // for both; the op refuses that (AC5).
    if clear && let Some(field) = fields.first() {
        let (name, _, _) = *field;
        return Err(EngineError::BadParam {
            name: "clear".into(),
            reason: format!("`{name}` cannot be combined with clear: drop the field or the switch"),
        });
    }
    let input = single_input(task)?;
    let mut doc = load_pdf(input)?;
    cancel.check()?;
    if clear {
        clear_metadata(&mut doc);
        // With the trailer and catalog entries gone, the Info dictionary and
        // the XMP stream are unreachable, so the prune drops their objects too.
        doc.prune_objects();
    } else if !fields.is_empty() {
        for (_, key, value) in fields {
            let id = info_id(&mut doc);
            if let Ok(dict) = doc.get_dictionary_mut(id) {
                dict.set(key, text_object(value));
            }
        }
        // A reader that prefers XMP would show the old values, so the Info
        // dictionary is the single source after a write (AC6, decision 3).
        if let Ok(catalog) = doc.catalog_mut() {
            catalog.remove(b"Metadata");
        }
        doc.prune_objects();
    }
    progress(0.9);
    save(&mut doc, &task.output)
}

/// A PDF text string: plain ASCII as a literal string, anything else as a
/// hexadecimal UTF-16BE string with the byte order mark (AC6).
fn text_object(value: &str) -> Object {
    if value.is_ascii() {
        return Object::string_literal(value);
    }
    let mut bytes = vec![0xFE, 0xFF];
    for unit in value.encode_utf16() {
        bytes.extend_from_slice(&unit.to_be_bytes());
    }
    Object::String(bytes, lopdf::StringFormat::Hexadecimal)
}

/// The op accepts only its five params. An unknown param or a wrong type is
/// BadParam naming the param.
fn check_metadata_params(task: &Task) -> Result<(), EngineError> {
    for name in task.params.keys() {
        if !METADATA_PARAMS.contains(&name.as_str()) {
            return Err(EngineError::BadParam {
                name: name.clone(),
                reason: "is not a metadata parameter".into(),
            });
        }
    }
    Ok(())
}

/// A text param of `metadata`, empty when it is left out.
fn metadata_text<'a>(task: &'a Task, name: &str) -> Result<&'a str, EngineError> {
    match task.params.get(name) {
        None => Ok(""),
        Some(value) => value.as_str().ok_or_else(|| EngineError::BadParam {
            name: name.to_string(),
            reason: "must be a string".into(),
        }),
    }
}

/// The `clear` param of `metadata`, false when it is left out.
fn metadata_clear(task: &Task) -> Result<bool, EngineError> {
    match task.params.get("clear") {
        None => Ok(false),
        Some(value) => value.as_bool().ok_or_else(|| EngineError::BadParam {
            name: "clear".into(),
            reason: "must be true or false".into(),
        }),
    }
}

/// The trailer Info dictionary, created when the document has none.
fn info_id(doc: &mut Document) -> ObjectId {
    if let Ok(id) = doc.trailer.get(b"Info").and_then(Object::as_reference) {
        return id;
    }
    let id = doc.add_object(dictionary! {});
    doc.trailer.set("Info", id);
    id
}

/// Drops the trailer `/Info` entry and the catalog `/Metadata` entry (AC4).
fn clear_metadata(doc: &mut Document) {
    doc.trailer.remove(b"Info");
    if let Ok(catalog) = doc.catalog_mut() {
        catalog.remove(b"Metadata");
    }
}

/// `split`: one ZIP. With a non-empty `ranges` string, one PDF per comma
/// range; otherwise one PDF per `every` pages. `every = 1` is one PDF per
/// page, the behaviour before S-123.
fn split(task: &Task, progress: Progress, cancel: &CancelToken) -> Result<(), EngineError> {
    let input = single_input(task)?;
    let doc = load_pdf(input)?;
    let pages = page_ids(&doc);
    let groups = split_groups(task, pages.len())?;
    let (root_pages, catalog_id) = page_tree_ids(&doc, input)?;
    let total = groups.len();
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&task.output)?);
    for (n, group) in groups.iter().enumerate() {
        cancel.check()?;
        let mut one = copy_with_pages(&doc, &pages, group, root_pages, catalog_id);
        let mut bytes = Vec::new();
        one.save_to(&mut bytes)
            .map_err(|e| EngineError::Failed(format!("cannot write part {}: {e}", n + 1)))?;
        zip.start_file(page_name(n + 1, total, "pdf"), zip_options())
            .map_err(zip_error)?;
        zip.write_all(&bytes)?;
        progress((n + 1) as f32 / total as f32 * 0.99);
    }
    zip.finish().map_err(zip_error)?;
    Ok(())
}

/// The page groups of `split`, as 0-based indices. `ranges` wins when it
/// is not empty; otherwise the pages are cut every `every` pages.
fn split_groups(task: &Task, page_count: usize) -> Result<Vec<Vec<usize>>, EngineError> {
    let ranges = text_param(task, "ranges", "")?;
    if !ranges.trim().is_empty() {
        return range_groups(ranges, page_count).map_err(|reason| EngineError::BadParam {
            name: "ranges".into(),
            reason,
        });
    }
    let every = every_param(task)?;
    let indices: Vec<usize> = (0..page_count).collect();
    Ok(indices.chunks(every).map(<[usize]>::to_vec).collect())
}

/// One group per comma range, each a page selection. A page may appear in
/// one range only.
fn range_groups(ranges: &str, page_count: usize) -> Result<Vec<Vec<usize>>, String> {
    let mut seen = vec![false; page_count + 1];
    let mut groups = Vec::new();
    for part in ranges.split(',') {
        let pages = parse_pages(part, page_count)?;
        for page in &pages {
            if seen[*page] {
                return Err(format!("page {page} appears in more than one range"));
            }
            seen[*page] = true;
        }
        groups.push(zero_based(pages));
    }
    Ok(groups)
}

fn zip_error(e: zip::result::ZipError) -> EngineError {
    EngineError::Failed(format!("cannot write the ZIP: {e}"))
}

pub struct GhostscriptEngine {
    program: Option<PathBuf>,
    temp_root: Option<PathBuf>,
}

impl Default for GhostscriptEngine {
    /// Uses Ghostscript as `locate::find` sees it (`gs`, or `gswin64c` or
    /// `gswin32c` on Windows), looked up once.
    fn default() -> Self {
        static FOUND: OnceLock<Option<PathBuf>> = OnceLock::new();
        let program = FOUND
            .get_or_init(|| locate::find(Program::Ghostscript))
            .clone();
        Self {
            program,
            temp_root: None,
        }
    }
}

impl GhostscriptEngine {
    /// Runs this program instead of the detected Ghostscript.
    pub fn with_program(program: impl Into<PathBuf>) -> Self {
        Self {
            program: Some(program.into()),
            temp_root: None,
        }
    }

    /// Directory for the private work directories. Default: the system temp dir.
    pub fn temp_root(mut self, dir: impl Into<PathBuf>) -> Self {
        self.temp_root = Some(dir.into());
        self
    }

    fn gs(
        &self,
        args: &[String],
        input: &Path,
        task: &Task,
        progress: Progress,
        cancel: &CancelToken,
    ) -> Result<(), EngineError> {
        let timeout = timeout_param(task, GS_DEFAULT_TIMEOUT_S)?;
        let program = self
            .program
            .clone()
            .ok_or_else(|| missing("Ghostscript", GS_URL, None))?;
        let mut cmd = Command::new(&program);
        cmd.args(["-dSAFER", "-dBATCH", "-dNOPAUSE"])
            .args(args)
            .arg(input);
        let mut pages = GsProgress::default();
        let outcome = run_child(&mut cmd, cancel, timeout, &mut |line| {
            if let Some(fraction) = pages.feed(line) {
                progress(fraction);
            }
        })
        .map_err(|e| missing_on_not_found(e, "Ghostscript", GS_URL, &program))?;
        if !outcome.status.success() {
            return Err(EngineError::Failed(failure_message(
                "Ghostscript failed",
                outcome.status,
                &outcome.stderr,
            )));
        }
        Ok(())
    }

    fn compress(
        &self,
        task: &Task,
        progress: Progress,
        cancel: &CancelToken,
    ) -> Result<TaskOutcome, EngineError> {
        let preset = preset_param(task)?;
        let dpi = compress_dpi_param(task)?;
        let input = absolute_input(task)?;
        let work = work_dir(self.temp_root.as_deref(), "kawaro-gs-")?;
        let out = work.path().join("out.pdf");
        let mut args = vec![
            "-sDEVICE=pdfwrite".to_string(),
            "-dCompatibilityLevel=1.5".to_string(),
            format!("-dPDFSETTINGS=/{preset}"),
        ];
        args.extend(downsample_args(dpi));
        args.push(format!("-sOutputFile={}", gs_path(&out)));
        self.gs(&args, &input, task, progress, cancel)?;
        if !is_nonempty_file(&out) {
            return Err(EngineError::Failed("Ghostscript wrote no PDF".into()));
        }
        let in_len = std::fs::metadata(&input)?.len();
        let out_len = std::fs::metadata(&out)?.len();
        if keep_original(in_len, out_len) {
            // Equal or larger: the original is kept and nothing is written
            // (S-133). The temp output goes with the work directory.
            work.close()?;
            return Ok(TaskOutcome::Skipped(SkipReason::AlreadyOptimized));
        }
        move_file(&out, &task.output)?;
        work.close()?;
        Ok(TaskOutcome::Written)
    }

    fn to_images(
        &self,
        task: &Task,
        progress: Progress,
        cancel: &CancelToken,
    ) -> Result<(), EngineError> {
        let dpi = dpi_param(task)?;
        let input = absolute_input(task)?;
        let work = work_dir(self.temp_root.as_deref(), "kawaro-gs-")?;
        let args = [
            "-sDEVICE=png16m".to_string(),
            format!("-r{dpi}"),
            "-dTextAlphaBits=4".to_string(),
            "-dGraphicsAlphaBits=4".to_string(),
            format!("-sOutputFile={}/p-%06d.png", gs_path(work.path())),
        ];
        self.gs(&args, &input, task, progress, cancel)?;
        let mut pngs: Vec<PathBuf> = std::fs::read_dir(work.path())?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "png"))
            .collect();
        pngs.sort();
        if pngs.is_empty() {
            return Err(EngineError::Failed("Ghostscript wrote no pages".into()));
        }
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&task.output)?);
        for (n, png) in pngs.iter().enumerate() {
            cancel.check()?;
            zip.start_file(page_name(n + 1, pngs.len(), "png"), zip_options())
                .map_err(zip_error)?;
            std::io::copy(&mut std::fs::File::open(png)?, &mut zip)?;
        }
        zip.finish().map_err(zip_error)?;
        work.close()?;
        Ok(())
    }
}

impl Engine for GhostscriptEngine {
    fn id(&self) -> &'static str {
        "ghostscript"
    }

    fn available(&self) -> bool {
        self.program.as_deref().is_some_and(Path::is_file)
    }

    fn capabilities(&self) -> Vec<(&'static str, Vec<MediaKind>)> {
        ["compress", "to-images"]
            .into_iter()
            .map(|op| (op, vec![MediaKind::Pdf]))
            .collect()
    }

    fn run(
        &self,
        task: &Task,
        progress: Progress,
        cancel: &CancelToken,
    ) -> Result<TaskOutcome, EngineError> {
        progress(0.0);
        cancel.check()?;
        let outcome = match task.op.as_str() {
            "compress" => self.compress(task, progress, cancel)?,
            "to-images" => {
                self.to_images(task, progress, cancel)?;
                TaskOutcome::Written
            }
            op => return Err(EngineError::UnsupportedOp(op.to_string())),
        };
        progress(1.0);
        Ok(outcome)
    }
}

/// The input as an absolute path, so Ghostscript never reads it as an option.
fn absolute_input(task: &Task) -> Result<PathBuf, EngineError> {
    let input = single_input(task)?;
    if !input.is_file() {
        return Err(input_error(input, "no such file"));
    }
    std::path::absolute(input).map_err(|e| input_error(input, e))
}

/// Ghostscript reads `%` in an output path as a format directive.
fn gs_path(path: &Path) -> String {
    path.to_string_lossy().replace('%', "%%")
}

/// The original stays when Ghostscript did not make the file smaller.
fn keep_original(in_len: u64, out_len: u64) -> bool {
    out_len >= in_len
}

fn preset_param(task: &Task) -> Result<&str, EngineError> {
    let preset = match task.params.get("preset") {
        None => "ebook",
        Some(_) => task.param_str("preset").unwrap_or(""),
    };
    if ["screen", "ebook", "printer", "prepress"].contains(&preset) {
        Ok(preset)
    } else {
        Err(EngineError::BadParam {
            name: "preset".into(),
            reason: "must be screen, ebook, printer or prepress".into(),
        })
    }
}

/// The `dpi` target of `compress`: 0 to 600, where 0 keeps the preset alone.
fn compress_dpi_param(task: &Task) -> Result<i64, EngineError> {
    match task.params.get("dpi") {
        None => Ok(0),
        Some(_) => match task.param_i64("dpi") {
            Some(dpi) if (0..=600).contains(&dpi) => Ok(dpi),
            _ => Err(EngineError::BadParam {
                name: "dpi".into(),
                reason: "must be a whole number from 0 to 600, 0 uses the preset".into(),
            }),
        },
    }
}

/// The image downsampling arguments of `compress` for a `dpi` target, empty
/// for 0 where the preset decides the image resolution. The names are the
/// documented Ghostscript parameters.
fn downsample_args(dpi: i64) -> Vec<String> {
    if dpi <= 0 {
        return Vec::new();
    }
    vec![
        "-dDownsampleColorImages=true".to_string(),
        "-dColorImageDownsampleType=/Bicubic".to_string(),
        format!("-dColorImageResolution={dpi}"),
        "-dDownsampleGrayImages=true".to_string(),
        "-dGrayImageDownsampleType=/Bicubic".to_string(),
        format!("-dGrayImageResolution={dpi}"),
        "-dDownsampleMonoImages=true".to_string(),
        "-dMonoImageDownsampleType=/Subsample".to_string(),
        format!("-dMonoImageResolution={dpi}"),
    ]
}

fn dpi_param(task: &Task) -> Result<i64, EngineError> {
    let dpi = match task.params.get("dpi") {
        None => DEFAULT_DPI,
        Some(_) => task.param_i64("dpi").unwrap_or(0),
    };
    if DPI_RANGE.contains(&dpi) {
        Ok(dpi)
    } else {
        Err(EngineError::BadParam {
            name: "dpi".into(),
            reason: format!(
                "must be a whole number from {} to {}",
                DPI_RANGE.start(),
                DPI_RANGE.end()
            ),
        })
    }
}

#[derive(Debug, PartialEq)]
enum GsLine {
    Total(usize),
    Page(usize),
    Other,
}

/// Ghostscript prints `Processing pages 1 through 3.` then `Page 1`, `Page 2`...
fn parse_gs_line(line: &str) -> GsLine {
    if let Some(rest) = line.strip_prefix("Processing pages ")
        && let Some((first, last)) = rest.trim_end_matches('.').split_once(" through ")
        && let (Ok(first), Ok(last)) = (first.parse::<usize>(), last.parse::<usize>())
    {
        return GsLine::Total(last.saturating_sub(first) + 1);
    }
    match line.strip_prefix("Page ").map(|n| n.trim().parse()) {
        Some(Ok(n)) => GsLine::Page(n),
        _ => GsLine::Other,
    }
}

/// Turns Ghostscript's output into progress fractions up to 0.9.
#[derive(Default)]
struct GsProgress {
    /// Page count from `Processing pages`, 0 until Ghostscript prints it.
    total: usize,
}

impl GsProgress {
    /// The fraction to report for this line, if any. Nothing is reported
    /// before the page count is known.
    fn feed(&mut self, line: &str) -> Option<f32> {
        match parse_gs_line(line) {
            GsLine::Total(n) => {
                self.total = n;
                None
            }
            GsLine::Page(n) if self.total > 0 => {
                Some(n.min(self.total) as f32 / self.total as f32 * 0.9)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(params: &str) -> Task {
        Task {
            action_id: "t".into(),
            op: "x".into(),
            params: toml::from_str(params).unwrap(),
            inputs: vec![],
            output: PathBuf::from("o"),
        }
    }

    #[test]
    fn pdf_keep_original_unless_strictly_smaller() {
        assert!(keep_original(100, 100));
        assert!(keep_original(100, 101));
        assert!(!keep_original(100, 99));
    }

    #[test]
    fn pdf_page_names_pad_to_the_page_count() {
        assert_eq!(page_name(1, 3, "pdf"), "page-001.pdf");
        assert_eq!(page_name(12, 999, "png"), "page-012.png");
        assert_eq!(page_name(7, 1000, "pdf"), "page-0007.pdf");
        assert_eq!(page_name(12345, 12345, "png"), "page-12345.png");
    }

    #[test]
    fn pdf_rotation_math_wraps() {
        assert_eq!(rotated(0, 90), 90);
        assert_eq!(rotated(270, 90), 0);
        assert_eq!(rotated(180, 270), 90);
        assert_eq!(rotated(-90, 90), 0);
        assert_eq!(rotated(-90, 0), 270);
    }

    #[test]
    fn pdf_params_default_and_validate() {
        assert_eq!(degrees_param(&task("")).unwrap(), 90);
        assert_eq!(degrees_param(&task("degrees = 270")).unwrap(), 270);
        assert!(degrees_param(&task("degrees = 360")).is_err());
        assert_eq!(preset_param(&task("")).unwrap(), "ebook");
        assert_eq!(
            preset_param(&task("preset = \"screen\"")).unwrap(),
            "screen"
        );
        assert_eq!(
            preset_param(&task("preset = \"prepress\"")).unwrap(),
            "prepress"
        );
        assert_eq!(dpi_param(&task("")).unwrap(), 150);
        assert_eq!(dpi_param(&task("dpi = 36")).unwrap(), 36);
        assert_eq!(dpi_param(&task("dpi = 600")).unwrap(), 600);
        assert!(dpi_param(&task("dpi = 35")).is_err());
        assert!(dpi_param(&task("dpi = 601")).is_err());
        assert!(dpi_param(&task("dpi = 150.0")).is_err());
    }

    #[test]
    fn s128_compress_dpi_builds_the_downsampling_arguments() {
        assert!(downsample_args(0).is_empty());
        assert_eq!(
            downsample_args(300),
            [
                "-dDownsampleColorImages=true",
                "-dColorImageDownsampleType=/Bicubic",
                "-dColorImageResolution=300",
                "-dDownsampleGrayImages=true",
                "-dGrayImageDownsampleType=/Bicubic",
                "-dGrayImageResolution=300",
                "-dDownsampleMonoImages=true",
                "-dMonoImageDownsampleType=/Subsample",
                "-dMonoImageResolution=300",
            ]
        );
        assert_eq!(compress_dpi_param(&task("")).unwrap(), 0);
        assert_eq!(compress_dpi_param(&task("dpi = 0")).unwrap(), 0);
        assert_eq!(compress_dpi_param(&task("dpi = 600")).unwrap(), 600);
        for bad in ["dpi = -1", "dpi = 601", "dpi = 72.5", "dpi = \"72\""] {
            assert!(compress_dpi_param(&task(bad)).is_err(), "{bad}");
        }
    }

    #[test]
    fn pdf_page_selections_parse_in_written_order() {
        assert_eq!(parse_pages("1", 5), Ok(vec![1]));
        assert_eq!(parse_pages(" 3 ", 5), Ok(vec![3]));
        assert_eq!(parse_pages("1-3", 5), Ok(vec![1, 2, 3]));
        assert_eq!(parse_pages("1,3,5", 5), Ok(vec![1, 3, 5]));
        assert_eq!(parse_pages("1-3,7", 7), Ok(vec![1, 2, 3, 7]));
        assert_eq!(parse_pages("5,1,2", 5), Ok(vec![5, 1, 2]));
        assert_eq!(parse_pages("1 - 2", 5), Ok(vec![1, 2]));
    }

    #[test]
    fn pdf_page_selections_reject_bad_input() {
        let bad = [
            "", "   ", "0", "6", "0-2", "1-6", "3-1", "1,1", "1-3,3", "1,,2", "1,", ",1", "a",
            "1-a", "1-", "-2", "1-2-3", "1.5",
        ];
        for text in bad {
            assert!(parse_pages(text, 5).is_err(), "{text:?} was accepted");
        }
    }

    #[test]
    fn pdf_page_params_default_and_validate() {
        assert_eq!(pages_param(&task(""), 5, "pages").unwrap(), vec![1]);
        assert_eq!(
            pages_param(&task("pages = \"2\""), 5, "pages").unwrap(),
            vec![2]
        );
        assert!(pages_param(&task("pages = 2"), 5, "pages").is_err());
        assert_eq!(order_param(&task(""), 1).unwrap(), vec![1]);
        assert_eq!(
            order_param(&task("order = \"5,1,2,3,4\""), 5).unwrap(),
            vec![5, 1, 2, 3, 4]
        );
        // Every page exactly once: partial lists, duplicates and pages
        // outside the document are errors.
        assert!(order_param(&task("order = \"5,1,2\""), 5).is_err());
        assert!(order_param(&task("order = \"4,2\""), 5).is_err());
        assert!(order_param(&task(""), 5).is_err());
        assert!(order_param(&task("order = \"1,1,2,3,4\""), 5).is_err());
        assert!(order_param(&task("order = \"6,1,2,3,4\""), 5).is_err());
        assert!(order_param(&task("order = \"\""), 5).is_err());
    }

    #[test]
    fn pdf_split_groups_follow_ranges_then_every() {
        assert_eq!(
            split_groups(&task(""), 3).unwrap(),
            vec![vec![0], vec![1], vec![2]]
        );
        assert_eq!(
            split_groups(&task("every = 2"), 5).unwrap(),
            vec![vec![0, 1], vec![2, 3], vec![4]]
        );
        assert_eq!(
            split_groups(&task("ranges = \"1-2,3-5\""), 5).unwrap(),
            vec![vec![0, 1], vec![2, 3, 4]]
        );
        // ranges wins over every
        assert_eq!(
            split_groups(&task("ranges = \"1-2,3-5\"\nevery = 4"), 5).unwrap(),
            vec![vec![0, 1], vec![2, 3, 4]]
        );
        assert_eq!(
            split_groups(&task("ranges = \"5\""), 5).unwrap(),
            vec![vec![4]]
        );
        assert!(split_groups(&task("ranges = \"1-2,2-3\""), 5).is_err());
        assert!(split_groups(&task("ranges = \"1-9\""), 5).is_err());
        assert!(split_groups(&task("every = 0"), 5).is_err());
        assert!(split_groups(&task("every = 2.5"), 5).is_err());
    }

    #[test]
    fn pdf_gs_lines_and_paths() {
        assert_eq!(
            parse_gs_line("Processing pages 1 through 3."),
            GsLine::Total(3)
        );
        assert_eq!(
            parse_gs_line("Processing pages 2 through 5."),
            GsLine::Total(4)
        );
        assert_eq!(parse_gs_line("Page 2"), GsLine::Page(2));
        assert_eq!(parse_gs_line("Page two"), GsLine::Other);
        assert_eq!(parse_gs_line("GPL Ghostscript 10.06.0"), GsLine::Other);
        assert_eq!(gs_path(Path::new("/t/50%/a")), "/t/50%%/a");
    }

    #[test]
    fn pdf_gs_progress_waits_for_the_page_count() {
        let mut p = GsProgress::default();
        assert_eq!(p.feed("GPL Ghostscript 10.06.0"), None);
        // A page line before the count reports nothing.
        assert_eq!(p.feed("Page 1"), None);
        assert_eq!(p.feed("Processing pages 1 through 4."), None);
        let mut seen = Vec::new();
        for line in ["Page 1", "Loading font", "Page 2", "Page 4", "Page 9"] {
            seen.extend(p.feed(line));
        }
        let want = [0.225, 0.45, 0.9, 0.9];
        assert_eq!(seen.len(), want.len(), "{seen:?}");
        for (got, want) in seen.iter().zip(want) {
            assert!((got - want).abs() < 1e-6, "{seen:?}");
        }
    }

    #[test]
    fn pdf_inherited_walks_up_and_skips_own_values() {
        let mut doc = Document::with_version("1.5");
        let root = doc.new_object_id();
        let mid = doc.new_object_id();
        let page =
            doc.add_object(dictionary! { "Type" => "Page", "Parent" => mid, "Rotate" => 90 });
        doc.objects.insert(
            mid,
            Object::Dictionary(dictionary! { "Type" => "Pages", "Parent" => root, "Kids" => vec![page.into()], "Count" => 1 }),
        );
        doc.objects.insert(
            root,
            Object::Dictionary(dictionary! {
                "Type" => "Pages", "Kids" => vec![mid.into()], "Count" => 1,
                "MediaBox" => vec![0.into(), 0.into(), 10.into(), 20.into()], "Rotate" => 180,
            }),
        );
        assert_eq!(inherited(&doc, page, b"Rotate"), None);
        assert!(inherited(&doc, page, b"MediaBox").is_some());
        assert_eq!(inherited(&doc, page, b"CropBox"), None);
        // A Parent cycle ends instead of looping forever.
        if let Ok(d) = doc.get_dictionary_mut(root) {
            d.set("Parent", mid);
        }
        assert_eq!(inherited(&doc, page, b"CropBox"), None);
    }

    // S-043b: an engine without a program names Ghostscript and its
    // download page.
    #[test]
    fn ac11_ghostscript_without_program_names_it_and_link() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("a.pdf");
        std::fs::write(&input, "%PDF-1.4\n").unwrap();
        let engine = GhostscriptEngine {
            program: None,
            ..GhostscriptEngine::with_program("unused")
        }
        .temp_root(dir.path());
        for op in ["compress", "to-images"] {
            let t = Task {
                action_id: format!("test.{op}"),
                op: op.into(),
                params: toml::from_str("").unwrap(),
                inputs: vec![input.clone()],
                output: dir.path().join("out"),
            };
            match engine.run(&t, &|_| {}, &CancelToken::new()) {
                Err(EngineError::Unavailable(m)) => {
                    assert!(m.contains("Ghostscript"), "{op}: {m}");
                    assert!(m.contains("https://ghostscript.com/releases/"), "{op}: {m}");
                }
                other => panic!("{op}: expected Unavailable, got {other:?}"),
            }
        }
    }
}
