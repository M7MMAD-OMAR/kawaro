//! The outcome of an install that ran in another process (S-054a AC6,
//! ADR 0006 section 6): the Windows helper writes `update-result.json`, and
//! the next daemon reports it once.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::UpdateError;
use super::apply::MsiResult;

/// The file name, next to `config.toml`.
pub const RESULT_FILE: &str = "update-result.json";

/// Why an install failed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Failure {
    Msi(MsiResult),
    Text(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Installed,
    Failed(Failure),
}

impl Outcome {
    /// A pending reboot still means the new version is installed.
    pub fn from_msi(r: MsiResult) -> Outcome {
        match r {
            MsiResult::Installed | MsiResult::InstalledRebootPending => Outcome::Installed,
            other => Outcome::Failed(Failure::Msi(other)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateResult {
    pub version: String,
    pub outcome: Outcome,
    pub at: u64,
}

/// Writes the result atomically: a temporary file, then a rename.
pub fn write_result(dir: &Path, result: &UpdateResult) -> Result<(), UpdateError> {
    fs::create_dir_all(dir).map_err(|_| UpdateError::Io)?;
    let text = serde_json::to_string(result).map_err(|_| UpdateError::Io)?;
    let tmp = dir.join(format!("{RESULT_FILE}.tmp"));
    let done = fs::write(&tmp, text).and_then(|()| fs::rename(&tmp, dir.join(RESULT_FILE)));
    if done.is_err() {
        let _ = fs::remove_file(&tmp);
        return Err(UpdateError::Io);
    }
    Ok(())
}

/// The stored result; an unreadable or incomplete file is `None`.
pub fn read_result(dir: &Path) -> Option<UpdateResult> {
    let text = fs::read_to_string(dir.join(RESULT_FILE)).ok()?;
    serde_json::from_str(&text).ok()
}

/// Reads the result and deletes the file, so it is reported once.
pub fn take_result(dir: &Path) -> Option<UpdateResult> {
    let result = read_result(dir);
    let _ = fs::remove_file(dir.join(RESULT_FILE));
    result
}
