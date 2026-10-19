//! PDF engines (S-010).
//!
//! `available()` is per engine, not per op, so the PDF work is split in two:
//!
//! - [`PdfEngine`] (id "pdf"): merge, rotate, split. Pure Rust on `lopdf`,
//!   always available.
//! - [`GhostscriptEngine`] (id "ghostscript"): compress, to-images. Runs `gs`
//!   as a child process and is unavailable without it.
//!
//! An engine task writes exactly one file, so `split` and `to-images` write
//! one ZIP (stored, no recompression: PDF pages and PNG are already
//! compressed) holding `page-001.pdf` or `page-001.png` and so on.

use crate::office::{
    failure_message, find_program, input_error, is_nonempty_file, move_file, path_dirs, run_child,
    single_input, timeout_param, work_dir,
};
use lopdf::{Document, Object, ObjectId, dictionary};
use sbarhive_core::{CancelToken, Engine, EngineError, MediaKind, Progress, Task};
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

/// Page attributes a page inherits from its ancestors in the page tree.
const INHERITABLE: [&str; 4] = ["MediaBox", "CropBox", "Resources", "Rotate"];

pub struct PdfEngine;

impl Engine for PdfEngine {
    fn id(&self) -> &'static str {
        "pdf"
    }

    fn capabilities(&self) -> Vec<(&'static str, Vec<MediaKind>)> {
        ["merge", "rotate", "split"]
            .into_iter()
            .map(|op| (op, vec![MediaKind::Pdf]))
            .collect()
    }

    fn run(
        &self,
        task: &Task,
        progress: Progress,
        cancel: &CancelToken,
    ) -> Result<(), EngineError> {
        progress(0.0);
        cancel.check()?;
        match task.op.as_str() {
            "merge" => merge(task, progress, cancel),
            "rotate" => rotate(task, cancel),
            "split" => split(task, progress, cancel),
            op => Err(EngineError::UnsupportedOp(op.to_string())),
        }?;
        progress(1.0);
        Ok(())
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
        let pages: Vec<ObjectId> = doc.get_pages().into_values().collect();
        let root = doc.trailer.get(b"Root").and_then(Object::as_reference).ok();
        for (id, object) in doc.objects {
            // The old catalog and page tree nodes are replaced by new ones.
            let is_tree = object.type_name().is_ok_and(|t| t == b"Pages");
            if Some(id) != root && !is_tree {
                next_id = next_id.max(id.0 + 1);
                out.objects.insert(id, object);
            }
        }
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
    out.max_id = next_id - 1;
    out.prune_objects();
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

fn split(task: &Task, progress: Progress, cancel: &CancelToken) -> Result<(), EngineError> {
    let input = single_input(task)?;
    let doc = load_pdf(input)?;
    let pages: Vec<ObjectId> = doc.get_pages().into_values().collect();
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
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&task.output)?);
    for (n, page) in pages.iter().enumerate() {
        cancel.check()?;
        let mut one = doc.clone();
        if let Ok(dict) = one.get_dictionary_mut(root_pages) {
            dict.set("Kids", vec![Object::Reference(*page)]);
            dict.set("Count", 1);
        }
        if let Ok(dict) = one.get_dictionary_mut(*page) {
            dict.set("Parent", root_pages);
        }
        if let Ok(dict) = one.get_dictionary_mut(catalog_id) {
            // These point at every page and would keep them all alive.
            for key in ["Outlines", "StructTreeRoot", "OpenAction"] {
                dict.remove(key.as_bytes());
            }
        }
        one.prune_objects();
        let mut bytes = Vec::new();
        one.save_to(&mut bytes)
            .map_err(|e| EngineError::Failed(format!("cannot write page {}: {e}", n + 1)))?;
        zip.start_file(page_name(n + 1, pages.len(), "pdf"), zip_options())
            .map_err(zip_error)?;
        zip.write_all(&bytes)?;
        progress((n + 1) as f32 / pages.len() as f32 * 0.99);
    }
    zip.finish().map_err(zip_error)?;
    Ok(())
}

fn zip_error(e: zip::result::ZipError) -> EngineError {
    EngineError::Failed(format!("cannot write the ZIP: {e}"))
}

pub struct GhostscriptEngine {
    program: Option<PathBuf>,
    temp_root: Option<PathBuf>,
}

impl Default for GhostscriptEngine {
    /// Uses `gs` (`gswin64c` or `gswin32c` on Windows) from `PATH`, looked up once.
    fn default() -> Self {
        static FOUND: OnceLock<Option<PathBuf>> = OnceLock::new();
        let names: &[&str] = if cfg!(windows) {
            &["gswin64c.exe", "gswin32c.exe"]
        } else {
            &["gs"]
        };
        let program = FOUND
            .get_or_init(|| find_program(names, &path_dirs()))
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
            .ok_or_else(|| EngineError::Unavailable("Ghostscript (gs)".into()))?;
        let mut cmd = Command::new(&program);
        cmd.args(["-dSAFER", "-dBATCH", "-dNOPAUSE"])
            .args(args)
            .arg(input);
        let mut total = 0usize;
        let outcome = run_child(
            &mut cmd,
            cancel,
            timeout,
            &mut |line| match parse_gs_line(line) {
                GsLine::Total(n) => total = n,
                GsLine::Page(n) if total > 0 => progress(n.min(total) as f32 / total as f32 * 0.9),
                _ => {}
            },
        )
        .map_err(|e| match e {
            EngineError::Io(err) if err.kind() == std::io::ErrorKind::NotFound => {
                EngineError::Unavailable(format!("{} not found", program.display()))
            }
            other => other,
        })?;
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
    ) -> Result<(), EngineError> {
        let preset = preset_param(task)?;
        let input = absolute_input(task)?;
        let work = work_dir(self.temp_root.as_deref(), "sbarhive-gs-")?;
        let out = work.path().join("out.pdf");
        let args = [
            "-sDEVICE=pdfwrite".to_string(),
            "-dCompatibilityLevel=1.5".to_string(),
            format!("-dPDFSETTINGS=/{preset}"),
            format!("-sOutputFile={}", gs_path(&out)),
        ];
        self.gs(&args, &input, task, progress, cancel)?;
        if !is_nonempty_file(&out) {
            return Err(EngineError::Failed("Ghostscript wrote no PDF".into()));
        }
        let in_len = std::fs::metadata(&input)?.len();
        let out_len = std::fs::metadata(&out)?.len();
        if keep_original(in_len, out_len) {
            std::fs::copy(&input, &task.output)?;
        } else {
            move_file(&out, &task.output)?;
        }
        work.close()?;
        Ok(())
    }

    fn to_images(
        &self,
        task: &Task,
        progress: Progress,
        cancel: &CancelToken,
    ) -> Result<(), EngineError> {
        let dpi = dpi_param(task)?;
        let input = absolute_input(task)?;
        let work = work_dir(self.temp_root.as_deref(), "sbarhive-gs-")?;
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
    ) -> Result<(), EngineError> {
        progress(0.0);
        cancel.check()?;
        match task.op.as_str() {
            "compress" => self.compress(task, progress, cancel),
            "to-images" => self.to_images(task, progress, cancel),
            op => Err(EngineError::UnsupportedOp(op.to_string())),
        }?;
        progress(1.0);
        Ok(())
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
    if ["screen", "ebook", "printer"].contains(&preset) {
        Ok(preset)
    } else {
        Err(EngineError::BadParam {
            name: "preset".into(),
            reason: "must be screen, ebook or printer".into(),
        })
    }
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
        assert!(preset_param(&task("preset = \"prepress\"")).is_err());
        assert_eq!(dpi_param(&task("")).unwrap(), 150);
        assert_eq!(dpi_param(&task("dpi = 36")).unwrap(), 36);
        assert_eq!(dpi_param(&task("dpi = 600")).unwrap(), 600);
        assert!(dpi_param(&task("dpi = 35")).is_err());
        assert!(dpi_param(&task("dpi = 601")).is_err());
        assert!(dpi_param(&task("dpi = 150.0")).is_err());
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
}
