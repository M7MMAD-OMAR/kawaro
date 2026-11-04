//! The `[mcp]` section of config.toml (S-121).
//!
//! The server is off by default: a host configuration alone must not be able
//! to start it. Every key falls back to its default when it is missing or
//! has the wrong type, so a broken section never stops the CLI from
//! printing the disabled error the user needs to see.

use std::path::{Path, PathBuf};

/// Settings of the MCP server, read from the `[mcp]` table.
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    /// Serve at all. Off by default.
    pub enabled: bool,
    /// Refuse every tool that writes a file. On by default.
    pub read_only: bool,
    /// Folders a tool may read from or write into, before `~` expansion and
    /// canonicalisation. Empty means no path passes.
    pub allowed_roots: Vec<PathBuf>,
    /// Most files one call may take.
    pub max_files: usize,
    /// Append one JSON line per `tools/call`.
    pub audit: bool,
    /// Longest side of the PNG preview of a result, in pixels.
    pub preview_px: u32,
    /// Where the audit lines go. Not read from the TOML: it is a seam for the
    /// tests, which must not write into the user's config folder.
    pub audit_path: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: false,
            read_only: true,
            allowed_roots: Vec::new(),
            max_files: 50,
            audit: true,
            preview_px: 512,
            audit_path: None,
        }
    }
}

impl Config {
    /// Reads the `[mcp]` table. The keys are the snake case names of the
    /// contract (S-121): no aliases, so the file and the ADR agree. Unknown
    /// keys and missing keys keep the defaults, and a key of the wrong type
    /// is ignored.
    pub fn from_table(t: &toml::Table) -> Self {
        let mut config = Self::default();
        let Some(mcp) = t.get("mcp").and_then(toml::Value::as_table) else {
            return config;
        };
        if let Some(v) = mcp.get("enabled").and_then(toml::Value::as_bool) {
            config.enabled = v;
        }
        if let Some(v) = mcp.get("read_only").and_then(toml::Value::as_bool) {
            config.read_only = v;
        }
        if let Some(v) = mcp.get("allowed_roots").and_then(toml::Value::as_array) {
            config.allowed_roots = v
                .iter()
                .filter_map(toml::Value::as_str)
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
                .collect();
        }
        if let Some(v) = mcp.get("max_files").and_then(toml::Value::as_integer)
            && v > 0
        {
            config.max_files = usize::try_from(v).unwrap_or(usize::MAX);
        }
        if let Some(v) = mcp.get("audit").and_then(toml::Value::as_bool) {
            config.audit = v;
        }
        if let Some(v) = mcp.get("preview_px").and_then(toml::Value::as_integer)
            && v > 0
        {
            config.preview_px = u32::try_from(v).unwrap_or(u32::MAX);
        }
        config
    }

    /// Reads a config.toml file. A missing file is the defaults without a
    /// warning; an unreadable or broken one is the defaults plus a warning.
    pub fn from_file(path: &Path) -> (Self, Option<String>) {
        match std::fs::read_to_string(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Self::default(), None),
            Err(e) => (Self::default(), Some(format!("{}: {e}", path.display()))),
            Ok(text) => match toml::from_str::<toml::Table>(&text) {
                Ok(table) => (Self::from_table(&table), None),
                Err(e) => (Self::default(), Some(format!("{}: {e}", path.display()))),
            },
        }
    }

    /// The user's config.toml.
    pub fn default_path() -> Option<PathBuf> {
        kawaro_core::config::Config::path()
    }

    /// The allowed roots that exist on this machine: `~` expanded with HOME,
    /// made absolute, canonicalised, folders only. Order is kept.
    pub fn resolved_roots(&self) -> Vec<PathBuf> {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        self.allowed_roots
            .iter()
            .filter_map(|root| resolve_root(root, home.as_deref()))
            .collect()
    }
}

/// One root: `~` and `~/` expand with HOME, then absolute, canonical and a
/// folder check. A root that does not resolve to a folder is dropped.
fn resolve_root(root: &Path, home: Option<&Path>) -> Option<PathBuf> {
    let text = root.to_string_lossy();
    let expanded = if text == "~" {
        home?.to_path_buf()
    } else if let Some(rest) = text.strip_prefix("~/") {
        home?.join(rest)
    } else {
        root.to_path_buf()
    };
    let absolute = std::path::absolute(&expanded).ok()?;
    let canonical = std::fs::canonicalize(&absolute).ok()?;
    canonical.is_dir().then_some(canonical)
}
