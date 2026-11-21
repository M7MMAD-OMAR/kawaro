//! The daemon side of the named user presets (S-143-B and S-143-C).
//!
//! The daemon is the only writer of `presets.toml`. This module owns one
//! worker thread for the lifetime of the daemon. Every store read and every
//! store write happens on that worker; the state mutex and the event loop
//! only move a cheap handle or a command.
//!
//! The core module owns the schema, the validation, the fingerprinting, the
//! stable ids and the atomic write. This module adds the serialization: one
//! command at a time, one reply per command, no request thread, and no
//! optimistic publish. A command that edits the store first loads the clean
//! file, compares the revision the caller read, and only then writes. A
//! failed save never replaces the last good in-memory document.
//!
//! The UI adapter recomputes List status against the actual current registry
//! action before it publishes a snapshot. The store compare and replace is
//! not a transaction against an arbitrary external writer. This service
//! serializes only Kawaro's own writes. No OS file lock is taken or
//! advertised.

use kawaro_core::action::ActionDef;
use kawaro_core::presets::{self, PresetDocument, PresetStatus, StoreRevision, UserPreset};
use kawaro_ui::saved_presets::{
    PresetApplied, PresetFailure, PresetItem, PresetReply as UiPresetReply, PresetRequest,
    PresetService as UiPresetService, PresetSnapshot,
};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};

/// A list of presets with their current status.
#[derive(Clone, Debug)]
pub struct PresetList {
    /// The presets of the requested action, or all of them.
    pub entries: Vec<PresetEntry>,
    /// The revision of the last good store. It is the missing revision when
    /// no store has been read yet.
    pub revision: StoreRevision,
    /// Set when the store file cannot be read. The entries are the last
    /// good in-memory list, and every mutation is refused until the file is
    /// repaired by hand.
    pub read_only: Option<PresetError>,
}

/// One preset and what it can do with the action as it is now.
#[derive(Clone, Debug)]
pub struct PresetEntry {
    /// The saved preset.
    pub preset: UserPreset,
    /// Ready, NeedsReview or Unavailable, decided by the core.
    pub status: PresetStatus,
}

/// A command for the preset worker.
#[derive(Clone, Debug)]
pub enum PresetCommand {
    /// The presets of one action, or all of them, with their statuses.
    List { action_id: Option<String> },
    /// Save the current resolved action under a name. The action carries the
    /// resolved values in its `params`, keyed by the engine param names.
    SaveCurrent {
        action: ActionDef,
        name: String,
        expected: StoreRevision,
    },
    /// Give a preset another name.
    Rename {
        id: String,
        name: String,
        expected: StoreRevision,
    },
    /// Take a preset out of the store.
    Delete { id: String, expected: StoreRevision },
    /// Explicitly review and save a preset again, with the current resolved
    /// action. Nothing refreshes on load.
    Refresh {
        id: String,
        action: ActionDef,
        expected: StoreRevision,
    },
    /// Apply a ready preset to the current action. This returns the action
    /// and its params; it never submits a job.
    Apply { id: String, action: ActionDef },
    /// Read the store again. `watch` is set for the file watcher, so a reload
    /// caused by this service's own successful save is skipped.
    Reload { watch: bool },
}

/// The reply to one command. Replies are rare, one per user action, so the
/// size of the largest variant does not matter.
#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum PresetReply {
    /// The answer to List and Reload.
    List(PresetList),
    /// A preset was saved.
    Saved {
        preset: UserPreset,
        revision: StoreRevision,
    },
    /// A preset was renamed.
    Renamed {
        preset: UserPreset,
        revision: StoreRevision,
    },
    /// A preset was deleted.
    Deleted {
        preset: UserPreset,
        revision: StoreRevision,
    },
    /// A preset was refreshed with the current action.
    Refreshed {
        preset: UserPreset,
        revision: StoreRevision,
    },
    /// The validated action with the preset values in its `params`.
    Applied { action: ActionDef },
    /// The command did not work. The high level helpers turn this into an
    /// error; a caller that polls the receiver sees it as a value.
    Failed(PresetError),
}

/// Why a preset command did not work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PresetError {
    /// The daemon has no config directory, so the store has no path.
    UnavailablePath,
    /// The action the command named is not a built-in action.
    MissingAction { action_id: String },
    /// The name, the values or the scope is not valid for a preset.
    Invalid { reason: String },
    /// The id counter cannot give out another id.
    Exhausted { reason: String },
    /// The store file has a version this build does not read.
    UnsupportedVersion { version: u32 },
    /// The store on disk is not the revision the caller read.
    Conflict { reason: String },
    /// The store file cannot be read as a store of this version.
    Corrupt { reason: String },
    /// A file or a thread could not be used.
    Io { reason: String },
    /// The worker is gone, usually because the daemon is shutting down.
    ChannelClosed,
    /// The bounded worker queue is full; the caller may try again later.
    Busy,
}

impl PresetError {
    /// The typed UI failure for this service error.
    pub fn ui_failure(&self) -> PresetFailure {
        match self {
            PresetError::UnavailablePath
            | PresetError::Corrupt { .. }
            | PresetError::UnsupportedVersion { .. }
            | PresetError::Io { .. } => PresetFailure::StoreReadOnly(self.to_string()),
            PresetError::Conflict { .. } => PresetFailure::Conflict(self.to_string()),
            PresetError::MissingAction { .. } => PresetFailure::ActionUnavailable(self.to_string()),
            PresetError::Invalid { .. } | PresetError::Exhausted { .. } => {
                PresetFailure::Invalid(self.to_string())
            }
            PresetError::ChannelClosed => PresetFailure::ChannelClosed,
            PresetError::Busy => PresetFailure::Busy,
        }
    }
}

/// The daemon adapter for the UI's typed preset seam.
#[derive(Clone)]
pub struct UiPresetAdapter {
    service: PresetService,
    state: crate::state::Shared,
}

impl UiPresetAdapter {
    /// Injects the daemon single writer and the live shared state.
    pub fn new(service: PresetService, state: crate::state::Shared) -> Self {
        Self { service, state }
    }

    fn unavailable(reason: impl Into<String>) -> PresetFailure {
        PresetFailure::ActionUnavailable(reason.into())
    }

    fn current_action(
        &self,
        action: &ActionDef,
        action_id: &str,
    ) -> Result<ActionDef, PresetFailure> {
        if action.id != action_id {
            return Err(Self::unavailable(format!(
                "the request action `{}` does not match `{action_id}`",
                action.id
            )));
        }
        let (current, configured) = {
            let st = crate::state::lock(&self.state);
            let Some(current) = st.registry.get(action_id).cloned() else {
                return Err(Self::unavailable(format!(
                    "the action `{action_id}` is not in the current registry"
                )));
            };
            (current.clone(), st.configured(current.clone()))
        };
        if !presets::validate_preset(
            &UserPreset {
                id: String::from("preset-probe"),
                name: String::from("probe"),
                action_id: action.id.clone(),
                schema_fingerprint: presets::fingerprint(&current),
                values: toml::Table::new(),
            },
            Some(&current),
        )
        .is_ready()
        {
            return Err(Self::unavailable(format!(
                "the action `{}` is not the built-in definition",
                action.id
            )));
        }
        if action.engine != current.engine
            || action.op != current.op
            || action.output.mode != current.output.mode
            || action.output.ext != current.output.ext
            || action.output.suffix != current.output.suffix
            || action.options != current.options
        {
            return Err(Self::unavailable(format!(
                "the action `{}` changed in the current registry",
                action.id
            )));
        }
        Ok(configured)
    }

    fn current_registry_action(&self, action_id: &str) -> Option<ActionDef> {
        let st = crate::state::lock(&self.state);
        st.registry.get(action_id).cloned()
    }

    fn command(&self, request: PresetRequest) -> Result<PresetCommand, PresetFailure> {
        match request {
            PresetRequest::List { action, .. } => {
                let action_id = action.id.clone();
                self.current_action(&action, &action_id)?;
                Ok(PresetCommand::List {
                    action_id: Some(action_id),
                })
            }
            PresetRequest::SaveCurrent {
                action,
                action_id,
                name,
                expected,
                ..
            } => {
                let configured = self.current_action(&action, &action_id)?;
                let saved = configured
                    .with_params(&action.params)
                    .map_err(|e| PresetFailure::Invalid(e.to_string()))?;
                Ok(PresetCommand::SaveCurrent {
                    action: saved,
                    name,
                    expected,
                })
            }
            PresetRequest::Rename {
                action_id,
                id,
                name,
                expected,
                ..
            } => {
                if self.current_registry_action(&action_id).is_none() {
                    return Err(Self::unavailable(format!(
                        "the action `{action_id}` is not in the current registry"
                    )));
                }
                Ok(PresetCommand::Rename { id, name, expected })
            }
            PresetRequest::Delete {
                action_id,
                id,
                expected,
                ..
            } => {
                if self.current_registry_action(&action_id).is_none() {
                    return Err(Self::unavailable(format!(
                        "the action `{action_id}` is not in the current registry"
                    )));
                }
                Ok(PresetCommand::Delete { id, expected })
            }
            PresetRequest::Refresh {
                action,
                action_id,
                id,
                expected,
                ..
            } => {
                let configured = self.current_action(&action, &action_id)?;
                let saved = configured
                    .with_params(&action.params)
                    .map_err(|e| PresetFailure::Invalid(e.to_string()))?;
                Ok(PresetCommand::Refresh {
                    id,
                    action: saved,
                    expected,
                })
            }
            PresetRequest::Apply {
                action,
                action_id,
                id,
                ..
            } => {
                let configured = self.current_action(&action, &action_id)?;
                Ok(PresetCommand::Apply {
                    id,
                    action: configured,
                })
            }
        }
    }

    fn snapshot(&self, list: PresetList, action_id: &str) -> PresetSnapshot {
        let current = self.current_registry_action(action_id);
        let read_only = list.read_only.as_ref().map(PresetError::ui_failure);
        let items = list
            .entries
            .into_iter()
            .map(|entry| {
                // The live registry decides: Ready, NeedsReview (offered for
                // review, never applied) or Unavailable with its reason.
                let status = match current.as_ref() {
                    Some(current) if current.id == entry.preset.action_id => entry.status,
                    _ => PresetStatus::Unavailable(format!(
                        "the action `{}` is not available in the current registry",
                        entry.preset.action_id
                    )),
                };
                PresetItem::new(entry.preset, status)
            })
            .collect();
        PresetSnapshot {
            revision: list.revision,
            items,
            read_only,
        }
    }

    fn map_reply(&self, request: &PresetRequest, reply: PresetReply) -> UiPresetReply {
        let action_id = request.action_id().to_string();
        match reply {
            PresetReply::List(list) => UiPresetReply::List(self.snapshot(list, &action_id)),
            PresetReply::Saved { preset, revision } => {
                UiPresetReply::Saved(kawaro_ui::saved_presets::PresetSaved { preset, revision })
            }
            PresetReply::Renamed { preset, revision } => {
                UiPresetReply::Renamed(kawaro_ui::saved_presets::PresetSaved { preset, revision })
            }
            PresetReply::Deleted { preset, revision } => {
                UiPresetReply::Deleted(kawaro_ui::saved_presets::PresetSaved { preset, revision })
            }
            PresetReply::Refreshed { preset, revision } => {
                UiPresetReply::Saved(kawaro_ui::saved_presets::PresetSaved { preset, revision })
            }
            PresetReply::Applied { action } => UiPresetReply::Applied(PresetApplied { action }),
            PresetReply::Failed(error) => UiPresetReply::Failed(error.ui_failure()),
        }
    }
}

impl UiPresetService for UiPresetAdapter {
    fn submit(&self, request: PresetRequest) -> Result<Receiver<UiPresetReply>, PresetFailure> {
        if request.session() == 0 || request.panel() == 0 || request.request() == 0 {
            return Err(PresetFailure::Invalid(String::from(
                "the preset request is not stamped",
            )));
        }
        let command = self.command(request.clone())?;
        let receiver = self.service.submit(command).map_err(|e| e.ui_failure())?;
        let this = self.clone();
        let (reply, mapped) = mpsc::channel();
        std::thread::spawn(move || {
            let reply_value = match receiver.recv() {
                Ok(value) => this.map_reply(&request, value),
                Err(_) => UiPresetReply::Failed(PresetFailure::ChannelClosed),
            };
            let _ = reply.send(reply_value);
        });
        Ok(mapped)
    }
}

impl fmt::Display for PresetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PresetError::UnavailablePath => {
                f.write_str("the preset store has no path on this system")
            }
            PresetError::MissingAction { action_id } => {
                write!(f, "the action `{action_id}` is not available")
            }
            PresetError::Invalid { reason } => write!(f, "invalid preset: {reason}"),
            PresetError::Exhausted { reason } => {
                write!(f, "the preset id counter is exhausted: {reason}")
            }
            PresetError::UnsupportedVersion { version } => write!(
                f,
                "the preset store has version {version}, this build reads an older one"
            ),
            PresetError::Conflict { reason } => write!(f, "the preset store changed: {reason}"),
            PresetError::Corrupt { reason } => {
                write!(f, "the preset store is not readable: {reason}")
            }
            PresetError::Io { reason } => write!(f, "the preset store I/O failed: {reason}"),
            PresetError::ChannelClosed => f.write_str("the preset worker is gone"),
            PresetError::Busy => f.write_str("the preset worker is busy; try again"),
        }
    }
}

impl std::error::Error for PresetError {}

impl From<kawaro_core::presets::PresetError> for PresetError {
    fn from(error: kawaro_core::presets::PresetError) -> Self {
        use kawaro_core::presets::PresetError as Core;
        match error {
            Core::InvalidName { reason } => PresetError::Invalid { reason },
            Core::InvalidValues { reason } => PresetError::Invalid { reason },
            Core::MissingAction { action_id } => PresetError::MissingAction { action_id },
            Core::UnsupportedScope { reason } => PresetError::Invalid { reason },
            Core::Exhausted { reason } => PresetError::Exhausted { reason },
            Core::UnsupportedVersion { version } => PresetError::UnsupportedVersion { version },
            Core::Conflict { reason } => PresetError::Conflict { reason },
            Core::Corrupt { reason } => PresetError::Corrupt { reason },
            Core::Io(error) => PresetError::Io {
                reason: error.to_string(),
            },
        }
    }
}

/// One command and the channel its reply goes back on.
struct Envelope {
    command: PresetCommand,
    reply: Sender<PresetReply>,
}

/// The cheap, cloneable handle to the one preset worker.
///
/// The worker starts on the first command, not at construction. A service
/// that is never used starts no thread and reads no file.
#[derive(Clone)]
pub struct PresetService {
    path: Option<PathBuf>,
    inner: Arc<Mutex<Option<SyncSender<Envelope>>>>,
}

// The synchronous wrappers serve the source tests and later callers; the
// production path uses `submit` and `reload_async`.
#[allow(dead_code)]
impl PresetService {
    /// A handle for `path`, or an unavailable handle when there is no path.
    pub fn new(path: Option<PathBuf>) -> Self {
        PresetService {
            path,
            inner: Arc::new(Mutex::new(None)),
        }
    }

    /// The store path, when the daemon has one.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Sends one command and returns the reply channel without waiting. The
    /// UI can poll this in a later packet.
    pub fn submit(&self, command: PresetCommand) -> Result<Receiver<PresetReply>, PresetError> {
        let (reply, receiver) = mpsc::channel();
        self.send_envelope(Envelope { command, reply })?;
        Ok(receiver)
    }

    /// Sends one command and waits for its reply. A worker failure is an
    /// error, not a reply value.
    pub fn send(&self, command: PresetCommand) -> Result<PresetReply, PresetError> {
        let receiver = self.submit(command)?;
        match receiver.recv() {
            Ok(PresetReply::Failed(error)) => Err(error),
            Ok(reply) => Ok(reply),
            Err(_) => Err(PresetError::ChannelClosed),
        }
    }

    /// The presets of one action, or all of them, with their statuses.
    pub fn list(&self, action_id: Option<String>) -> Result<PresetList, PresetError> {
        match self.send(PresetCommand::List { action_id })? {
            PresetReply::List(list) => Ok(list),
            _ => Err(PresetError::Invalid {
                reason: String::from("the preset worker gave an unexpected reply"),
            }),
        }
    }

    /// Saves the current resolved action under a name.
    pub fn save_current(
        &self,
        action: &ActionDef,
        name: &str,
        expected: StoreRevision,
    ) -> Result<(UserPreset, StoreRevision), PresetError> {
        match self.send(PresetCommand::SaveCurrent {
            action: action.clone(),
            name: name.to_string(),
            expected,
        })? {
            PresetReply::Saved { preset, revision } => Ok((preset, revision)),
            _ => Err(PresetError::Invalid {
                reason: String::from("the preset worker gave an unexpected reply"),
            }),
        }
    }

    /// Renames a preset.
    pub fn rename(
        &self,
        id: &str,
        name: &str,
        expected: StoreRevision,
    ) -> Result<(UserPreset, StoreRevision), PresetError> {
        match self.send(PresetCommand::Rename {
            id: id.to_string(),
            name: name.to_string(),
            expected,
        })? {
            PresetReply::Renamed { preset, revision } => Ok((preset, revision)),
            _ => Err(PresetError::Invalid {
                reason: String::from("the preset worker gave an unexpected reply"),
            }),
        }
    }

    /// Deletes a preset.
    pub fn delete(
        &self,
        id: &str,
        expected: StoreRevision,
    ) -> Result<(UserPreset, StoreRevision), PresetError> {
        match self.send(PresetCommand::Delete {
            id: id.to_string(),
            expected,
        })? {
            PresetReply::Deleted { preset, revision } => Ok((preset, revision)),
            _ => Err(PresetError::Invalid {
                reason: String::from("the preset worker gave an unexpected reply"),
            }),
        }
    }

    /// Explicitly saves a reviewed preset again against the current action.
    pub fn refresh(
        &self,
        id: &str,
        action: &ActionDef,
        expected: StoreRevision,
    ) -> Result<(UserPreset, StoreRevision), PresetError> {
        match self.send(PresetCommand::Refresh {
            id: id.to_string(),
            action: action.clone(),
            expected,
        })? {
            PresetReply::Refreshed { preset, revision } => Ok((preset, revision)),
            _ => Err(PresetError::Invalid {
                reason: String::from("the preset worker gave an unexpected reply"),
            }),
        }
    }

    /// Validates a ready preset against the current action and returns that
    /// action with the preset params. It never submits a job.
    pub fn apply(&self, id: &str, action: &ActionDef) -> Result<ActionDef, PresetError> {
        match self.send(PresetCommand::Apply {
            id: id.to_string(),
            action: action.clone(),
        })? {
            PresetReply::Applied { action } => Ok(action),
            _ => Err(PresetError::Invalid {
                reason: String::from("the preset worker gave an unexpected reply"),
            }),
        }
    }

    /// Reads the store again, waiting for the reply.
    pub fn reload(&self) -> Result<PresetList, PresetError> {
        match self.send(PresetCommand::Reload { watch: false })? {
            PresetReply::List(list) => Ok(list),
            _ => Err(PresetError::Invalid {
                reason: String::from("the preset worker gave an unexpected reply"),
            }),
        }
    }

    /// Queues a watcher reload without waiting. The reply is dropped: the
    /// watcher only needs the worker to see the new file.
    pub fn reload_async(&self) {
        let _ = self.submit(PresetCommand::Reload { watch: true });
    }

    /// The worker sender, started once.
    fn sender(&self) -> Result<SyncSender<Envelope>, PresetError> {
        let path = self.path.clone().ok_or(PresetError::UnavailablePath)?;
        let mut guard = crate::state::lock(&self.inner);
        if let Some(sender) = guard.as_ref() {
            return Ok(sender.clone());
        }
        let (sender, receiver) = mpsc::sync_channel(16);
        std::thread::Builder::new()
            .name(String::from("kawaro-presets"))
            .spawn(move || worker(path, receiver))
            .map_err(|error| PresetError::Io {
                reason: error.to_string(),
            })?;
        *guard = Some(sender.clone());
        Ok(sender)
    }

    fn send_envelope(&self, envelope: Envelope) -> Result<(), PresetError> {
        let sender = self.sender()?;
        sender.try_send(envelope).map_err(|error| match error {
            TrySendError::Full(_) => PresetError::Busy,
            TrySendError::Disconnected(_) => PresetError::ChannelClosed,
        })
    }
}

/// The one worker thread. It ends when every handle is dropped.
fn worker(path: PathBuf, receiver: Receiver<Envelope>) {
    let mut state = WorkerState::new(path);
    for envelope in receiver {
        let reply = state.handle(envelope.command);
        let _ = envelope.reply.send(reply);
    }
}

/// The worker's state: the last good store and the revision it wrote.
struct WorkerState {
    path: PathBuf,
    committed: Option<(PresetDocument, StoreRevision)>,
    last_saved: Option<StoreRevision>,
}

impl WorkerState {
    fn new(path: PathBuf) -> Self {
        WorkerState {
            path,
            committed: None,
            last_saved: None,
        }
    }

    fn handle(&mut self, command: PresetCommand) -> PresetReply {
        match command {
            PresetCommand::List { action_id } => PresetReply::List(self.list(action_id)),
            PresetCommand::SaveCurrent {
                action,
                name,
                expected,
            } => match self.save_current(action, name, expected) {
                Ok((preset, revision)) => PresetReply::Saved { preset, revision },
                Err(error) => PresetReply::Failed(error),
            },
            PresetCommand::Rename { id, name, expected } => match self.rename(id, name, expected) {
                Ok((preset, revision)) => PresetReply::Renamed { preset, revision },
                Err(error) => PresetReply::Failed(error),
            },
            PresetCommand::Delete { id, expected } => match self.delete(id, expected) {
                Ok((preset, revision)) => PresetReply::Deleted { preset, revision },
                Err(error) => PresetReply::Failed(error),
            },
            PresetCommand::Refresh {
                id,
                action,
                expected,
            } => match self.refresh(id, action, expected) {
                Ok((preset, revision)) => PresetReply::Refreshed { preset, revision },
                Err(error) => PresetReply::Failed(error),
            },
            PresetCommand::Apply { id, action } => match self.apply(id, action) {
                Ok(action) => PresetReply::Applied { action },
                Err(error) => PresetReply::Failed(error),
            },
            PresetCommand::Reload { watch } => PresetReply::List(self.reload(watch)),
        }
    }

    /// The presets of one action, or all of them, with their statuses.
    ///
    /// A missing file is an empty store. An existing empty, corrupt or
    /// future-version file is a read-only error state: the last good
    /// in-memory list stays, the bytes stay, and no mutation can replace
    /// them.
    fn list(&mut self, action_id: Option<String>) -> PresetList {
        match presets::load(&self.path) {
            Ok((document, revision)) => {
                self.committed = Some((document.clone(), revision.clone()));
                self.list_from(&document, revision, None, action_id.as_deref())
            }
            Err(error) => {
                let error = PresetError::from(error);
                let (document, revision) = match &self.committed {
                    Some((document, revision)) => (document.clone(), revision.clone()),
                    None => (PresetDocument::empty(), presets::missing_revision()),
                };
                self.list_from(&document, revision, Some(error), action_id.as_deref())
            }
        }
    }

    /// Builds a list from a document already in memory.
    fn list_from(
        &self,
        document: &PresetDocument,
        revision: StoreRevision,
        read_only: Option<PresetError>,
        action_id: Option<&str>,
    ) -> PresetList {
        let entries = document
            .presets
            .iter()
            .filter(|preset| action_id.is_none_or(|id| preset.action_id == id))
            .map(|preset| PresetEntry {
                status: presets::validate_preset(
                    preset,
                    presets::builtin_action(&preset.action_id),
                ),
                preset: (*preset).clone(),
            })
            .collect();
        PresetList {
            entries,
            revision,
            read_only,
        }
    }

    /// Loads the clean file and keeps it as the last good document.
    fn load_fresh(&mut self) -> Result<(PresetDocument, StoreRevision), PresetError> {
        let (document, revision) = presets::load(&self.path)?;
        self.committed = Some((document.clone(), revision.clone()));
        Ok((document, revision))
    }

    /// One serialized mutation: load, compare the revision, edit in memory,
    /// save, and only then publish the new document.
    fn mutate<F>(
        &mut self,
        expected: StoreRevision,
        edit: F,
    ) -> Result<(PresetDocument, StoreRevision), PresetError>
    where
        F: FnOnce(&mut PresetDocument) -> Result<(), PresetError>,
    {
        let (mut document, current) = self.load_fresh()?;
        if current != expected {
            return Err(PresetError::Conflict {
                reason: format!(
                    "the store on disk is not the revision that was read ({})",
                    current.0
                ),
            });
        }
        edit(&mut document)?;
        let revision = presets::save(&self.path, &expected, &document)?;
        self.committed = Some((document.clone(), revision.clone()));
        self.last_saved = Some(revision.clone());
        Ok((document, revision))
    }

    fn save_current(
        &mut self,
        action: ActionDef,
        name: String,
        expected: StoreRevision,
    ) -> Result<(UserPreset, StoreRevision), PresetError> {
        // The action carries the resolved values in its params, keyed by the
        // engine param names. The core maps them to field ids and validates
        // them once.
        let values = presets::canonical_values(&action, &action.params)?;
        let mut created = None;
        let (_, revision) = self.mutate(expected, |document| {
            match document.insert(&action, &name, &values) {
                Ok(preset) => {
                    created = Some(preset);
                    Ok(())
                }
                Err(error) => Err(PresetError::from(error)),
            }
        })?;
        let preset = created.ok_or_else(|| PresetError::Invalid {
            reason: String::from("the preset was not created"),
        })?;
        Ok((preset, revision))
    }

    fn rename(
        &mut self,
        id: String,
        name: String,
        expected: StoreRevision,
    ) -> Result<(UserPreset, StoreRevision), PresetError> {
        let (_, revision) = self.mutate(expected, |document| {
            document.rename(&id, &name).map_err(PresetError::from)
        })?;
        let preset = self.preset_after(&id)?;
        Ok((preset, revision))
    }

    fn delete(
        &mut self,
        id: String,
        expected: StoreRevision,
    ) -> Result<(UserPreset, StoreRevision), PresetError> {
        let mut removed = None;
        let (_, revision) = self.mutate(expected, |document| match document.remove(&id) {
            Ok(preset) => {
                removed = Some(preset);
                Ok(())
            }
            Err(error) => Err(PresetError::from(error)),
        })?;
        let preset = removed.ok_or_else(|| PresetError::Invalid {
            reason: format!("the preset `{id}` was not removed"),
        })?;
        Ok((preset, revision))
    }

    fn refresh(
        &mut self,
        id: String,
        action: ActionDef,
        expected: StoreRevision,
    ) -> Result<(UserPreset, StoreRevision), PresetError> {
        let values = presets::canonical_values(&action, &action.params)?;
        let (_, revision) = self.mutate(expected, |document| {
            document
                .refresh(&id, &action, &values)
                .map_err(PresetError::from)
        })?;
        let preset = self.preset_after(&id)?;
        Ok((preset, revision))
    }

    fn apply(&mut self, id: String, action: ActionDef) -> Result<ActionDef, PresetError> {
        let (document, _) = self.load_fresh()?;
        let preset = document.get(&id).ok_or_else(|| PresetError::Invalid {
            reason: format!("the preset `{id}` is not in the store"),
        })?;
        let params = presets::apply_preset(preset, &action)?;
        let mut applied = action;
        applied.params = params;
        Ok(applied)
    }

    fn preset_after(&self, id: &str) -> Result<UserPreset, PresetError> {
        self.committed
            .as_ref()
            .and_then(|(document, _)| document.get(id).cloned())
            .ok_or_else(|| PresetError::Invalid {
                reason: format!("the preset `{id}` is not in the store"),
            })
    }

    /// A watcher reload. When the file is the revision this worker last
    /// saved, the committed document is already current, so the reload is a
    /// no-op and the worker does not read the file again.
    fn reload(&mut self, watch: bool) -> PresetList {
        if watch {
            let same = match (&self.last_saved, presets::revision_of(&self.path)) {
                (Some(last), Ok(current)) => last == &current,
                _ => false,
            };
            if same && let Some((document, revision)) = self.committed.clone() {
                return self.list_from(&document, revision, None, None);
            }
        }
        self.list(None)
    }
}
