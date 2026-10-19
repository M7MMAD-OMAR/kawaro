//! Registry logic. Owned by the core track; see `action.rs` for the contract.
//!
//! One load (one `add_toml` document, or every file of one `add_dir` call)
//! must not repeat an id. A later load may reuse an id: the new definition
//! replaces the old one in place, which is how user files override
//! built-in actions. A failed load leaves the registry unchanged.
use crate::action::{ActionDef, Registry, RegistryError};
use crate::media;
use serde::Deserialize;
use std::path::Path;

/// Files in the repository `actions/` folder, in file name order.
const BUILTIN: &[(&str, &str)] = &[("images.toml", include_str!("../../../actions/images.toml"))];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    #[serde(default)]
    action: Vec<ActionDef>,
}

fn parse_error(name: &str, reason: impl Into<String>) -> RegistryError {
    RegistryError::Parse {
        file: name.to_string(),
        reason: reason.into(),
    }
}

fn validate(name: &str, a: &ActionDef) -> Result<(), RegistryError> {
    if a.ring != 1 && a.ring != 2 {
        return Err(parse_error(
            name,
            format!("action `{}`: ring must be 1 or 2", a.id),
        ));
    }
    if a.accepts.is_empty() {
        return Err(parse_error(
            name,
            format!("action `{}`: accepts is empty", a.id),
        ));
    }
    if a.max_files != 0 && a.max_files < a.min_files {
        return Err(parse_error(
            name,
            format!("action `{}`: max_files is below min_files", a.id),
        ));
    }
    Ok(())
}

/// Parses one document and appends its actions to `staged`, rejecting ids
/// already present in `staged`.
fn stage(staged: &mut Vec<ActionDef>, name: &str, text: &str) -> Result<(), RegistryError> {
    let doc: Document = toml::from_str(text).map_err(|e| parse_error(name, e.to_string()))?;
    for a in doc.action {
        validate(name, &a)?;
        if staged.iter().any(|s| s.id == a.id) {
            return Err(RegistryError::Duplicate(a.id));
        }
        staged.push(a);
    }
    Ok(())
}

/// Merges a successful load: same id replaces in place, new ids append.
fn merge(actions: &mut Vec<ActionDef>, staged: Vec<ActionDef>) {
    for a in staged {
        match actions.iter_mut().find(|e| e.id == a.id) {
            Some(slot) => *slot = a,
            None => actions.push(a),
        }
    }
}

pub(crate) fn add_toml(
    actions: &mut Vec<ActionDef>,
    name: &str,
    text: &str,
) -> Result<(), RegistryError> {
    let mut staged = Vec::new();
    stage(&mut staged, name, text)?;
    merge(actions, staged);
    Ok(())
}

pub(crate) fn add_dir(reg: &mut Registry, dir: &Path) -> Result<(), RegistryError> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry?.path();
        if media::extension(&path) == "toml" {
            paths.push(path);
        }
    }
    paths.sort();
    let mut docs = Vec::new();
    for path in paths {
        docs.push((path.display().to_string(), std::fs::read_to_string(&path)?));
    }
    add_all(reg, &docs)
}

/// Checks every document as one load, then merges them in order. The
/// registry field is private to `action.rs`, so the merge goes through the
/// public `add_toml`, which cannot fail on documents that passed the check.
fn add_all<N: AsRef<str>, T: AsRef<str>>(
    reg: &mut Registry,
    docs: &[(N, T)],
) -> Result<(), RegistryError> {
    let mut staged = Vec::new();
    for (name, text) in docs {
        stage(&mut staged, name.as_ref(), text.as_ref())?;
    }
    for (name, text) in docs {
        reg.add_toml(name.as_ref(), text.as_ref())?;
    }
    Ok(())
}

pub(crate) fn builtin() -> Registry {
    let mut reg = Registry::default();
    // The files ship with the binary and are checked by the tests.
    add_all(&mut reg, BUILTIN).expect("built-in actions are valid");
    reg
}

fn shown(a: &ActionDef, files: &[&Path], engine_ok: &dyn Fn(&str) -> bool) -> bool {
    let n = files.len();
    if n == 0 || n < a.min_files || (a.max_files != 0 && n > a.max_files) {
        return false;
    }
    if !files.iter().all(|f| a.accepts.contains(&media::detect(f))) {
        return false;
    }
    let skipped = files.iter().all(|f| {
        let ext = media::extension(f);
        a.skip_ext.iter().any(|s| s.eq_ignore_ascii_case(&ext))
    });
    !skipped && engine_ok(&a.engine)
}

pub(crate) fn matching<'a>(
    actions: &'a [ActionDef],
    files: &[&Path],
    engine_ok: &dyn Fn(&str) -> bool,
) -> Vec<&'a ActionDef> {
    let mut out: Vec<&ActionDef> = actions
        .iter()
        .filter(|a| shown(a, files, engine_ok))
        .collect();
    out.sort_by(|x, y| (x.ring, x.order, &x.id).cmp(&(y.ring, y.order, &y.id)));
    out
}
