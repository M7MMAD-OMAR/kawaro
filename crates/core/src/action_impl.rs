//! Registry logic. Owned by the core track; see `action.rs` for the contract.
use crate::action::{ActionDef, Registry, RegistryError};
use std::path::Path;

pub(crate) fn add_toml(
    _actions: &mut Vec<ActionDef>,
    _name: &str,
    _text: &str,
) -> Result<(), RegistryError> {
    todo!("core track")
}

pub(crate) fn add_dir(_reg: &mut Registry, _dir: &Path) -> Result<(), RegistryError> {
    todo!("core track")
}

pub(crate) fn builtin() -> Registry {
    todo!("core track")
}

pub(crate) fn matching<'a>(
    _actions: &'a [ActionDef],
    _files: &[&Path],
    _engine_ok: &dyn Fn(&str) -> bool,
) -> Vec<&'a ActionDef> {
    todo!("core track")
}
