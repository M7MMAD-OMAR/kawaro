//! The injectable named-preset service seam of the customization panel.
//!
//! This module is platform free and knows no daemon, engine or store type.
//! The daemon maps these typed requests to its one preset worker, checks
//! commands against the live registry, and returns a pollable reply. The UI
//! polls with `try_recv` from its existing panel timer, so no store I/O or
//! waiting runs on the event loop.
//!
//! Every request carries the action it belongs to plus the panel and request
//! stamps. A panel drops its poller on close or replacement. A late reply for
//! an old action, panel or request is then ignored.

use kawaro_core::action::ActionDef;
use kawaro_core::presets::{PresetStatus, StoreRevision, UserPreset};
use std::sync::mpsc::{Receiver, TryRecvError};

/// One saved preset as the panel shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresetItem {
    /// The stable store id, never a row index.
    pub id: String,
    /// The trimmed display name.
    pub name: String,
    /// The action the preset belongs to.
    pub action_id: String,
    /// Ready, NeedsReview(reason) or Unavailable(reason), decided from the
    /// current registry action.
    pub status: PresetStatus,
}

impl PresetItem {
    /// Builds a view item from the domain preset and its current status.
    pub fn new(preset: UserPreset, status: PresetStatus) -> Self {
        Self {
            id: preset.id,
            name: preset.name,
            action_id: preset.action_id,
            status,
        }
    }

    /// True when Apply is offered. Unavailable presets are never applied.
    pub fn can_apply(&self) -> bool {
        self.status.is_ready()
    }
}

/// A typed asynchronous command for the injected preset service.
#[derive(Clone, Debug)]
pub enum PresetRequest {
    /// Reads the current action's presets and snapshot revision.
    List {
        session: u64,
        panel: u64,
        request: u64,
        action: ActionDef,
    },
    /// Saves the exact displayed resolved action under a trimmed name.
    SaveCurrent {
        session: u64,
        panel: u64,
        request: u64,
        action: ActionDef,
        action_id: String,
        name: String,
        expected: StoreRevision,
    },
    /// Renames one stable preset id.
    Rename {
        session: u64,
        panel: u64,
        request: u64,
        action_id: String,
        id: String,
        name: String,
        expected: StoreRevision,
    },
    /// Deletes one stable preset id after local confirmation.
    Delete {
        session: u64,
        panel: u64,
        request: u64,
        action_id: String,
        id: String,
        expected: StoreRevision,
    },
    /// Explicitly refreshes one reviewed preset against the exact current
    /// registry action and displayed values.
    Refresh {
        session: u64,
        panel: u64,
        request: u64,
        action: ActionDef,
        action_id: String,
        id: String,
        expected: StoreRevision,
    },
    /// Validates and resolves one Ready preset against the current action.
    Apply {
        session: u64,
        panel: u64,
        request: u64,
        action: ActionDef,
        action_id: String,
        id: String,
    },
}

impl PresetRequest {
    /// The owning hive session.
    pub fn session(&self) -> u64 {
        match self {
            Self::List { session, .. }
            | Self::SaveCurrent { session, .. }
            | Self::Rename { session, .. }
            | Self::Delete { session, .. }
            | Self::Refresh { session, .. }
            | Self::Apply { session, .. } => *session,
        }
    }

    /// The panel instance that owns this request.
    pub fn panel(&self) -> u64 {
        match self {
            Self::List { panel, .. }
            | Self::SaveCurrent { panel, .. }
            | Self::Rename { panel, .. }
            | Self::Delete { panel, .. }
            | Self::Refresh { panel, .. }
            | Self::Apply { panel, .. } => *panel,
        }
    }

    /// The monotonically allocated request number inside that panel.
    pub fn request(&self) -> u64 {
        match self {
            Self::List { request, .. }
            | Self::SaveCurrent { request, .. }
            | Self::Rename { request, .. }
            | Self::Delete { request, .. }
            | Self::Refresh { request, .. }
            | Self::Apply { request, .. } => *request,
        }
    }

    /// The action id this request is scoped to.
    pub fn action_id(&self) -> &str {
        match self {
            Self::List { action, .. }
            | Self::SaveCurrent { action, .. }
            | Self::Refresh { action, .. }
            | Self::Apply { action, .. } => &action.id,
            Self::Rename { action_id, .. } | Self::Delete { action_id, .. } => action_id,
        }
    }

    /// True when all ownership stamps still belong to the live panel.
    pub fn owns(&self, session: u64, panel: u64) -> bool {
        self.session() == session && self.panel() == panel
    }
}

/// The snapshot of a successful List or reload.
#[derive(Clone, Debug)]
pub struct PresetSnapshot {
    /// The revision mutations must carry.
    pub revision: StoreRevision,
    /// The current action's presets in stable store order.
    pub items: Vec<PresetItem>,
    /// A read-only store error, such as a corrupt or future-version file.
    pub read_only: Option<PresetFailure>,
}

/// Why a named-preset command did not work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PresetFailure {
    /// The store file cannot be read. Mutations remain disabled.
    StoreReadOnly(String),
    /// Another change won the revision check. Reload before retrying.
    Conflict(String),
    /// The action is missing from the current registry or is not the
    /// built-in definition a saved preset may use.
    ActionUnavailable(String),
    /// A name or a typed value is invalid.
    Invalid(String),
    /// The action is outside the saved preset scope.
    Unsupported(String),
    /// The preset id is not in the current store.
    NotFound(String),
    /// The bounded preset worker queue is full. Try again after pending clears.
    Busy,
    /// The preset worker disconnected or gave an unexpected reply.
    ChannelClosed,
    /// The adapter could not reach the live daemon state.
    ServiceUnavailable(String),
}

/// Concise bilingual text for a preset failure.
pub fn failure_text(lang: kawaro_core::i18n::Lang, failure: &PresetFailure) -> String {
    use kawaro_core::i18n::Lang;
    match failure {
        PresetFailure::StoreReadOnly(_) => match lang {
            Lang::Ar => String::from("تعذر قراءة الإعدادات المحفوظة، لذا توقف التعديل"),
            Lang::En => String::from("Saved presets are read only until the store is repaired"),
        },
        PresetFailure::Conflict(_) => match lang {
            Lang::Ar => String::from("تغيرت الإعدادات المحفوظة، أعد التحميل ثم حاول مجددا"),
            Lang::En => String::from("Saved presets changed; reloaded, please try again"),
        },
        PresetFailure::ActionUnavailable(_) => match lang {
            Lang::Ar => String::from("هذا الإجراء لم يعد متاحا"),
            Lang::En => String::from("This action is no longer available"),
        },
        PresetFailure::Invalid(_) => match lang {
            Lang::Ar => String::from("الاسم أو القيم غير صالحة"),
            Lang::En => String::from("The name or values are invalid"),
        },
        PresetFailure::Unsupported(_) => match lang {
            Lang::Ar => String::from("لا يمكن حفظ هذا الإجراء ضمن الإعدادات"),
            Lang::En => String::from("This action cannot be saved as a preset"),
        },
        PresetFailure::NotFound(_) => match lang {
            Lang::Ar => String::from("الإعداد المحفوظ لم يعد موجودا"),
            Lang::En => String::from("The saved preset no longer exists"),
        },
        PresetFailure::Busy => match lang {
            Lang::Ar => String::from("الإعدادات المحفوظة مشغولة، حاول بعد قليل"),
            Lang::En => String::from("Saved presets are busy; try again shortly"),
        },
        PresetFailure::ChannelClosed | PresetFailure::ServiceUnavailable(_) => match lang {
            Lang::Ar => String::from("تعذر الوصول إلى خدمة الإعدادات المحفوظة"),
            Lang::En => String::from("The saved preset service is unavailable"),
        },
    }
}

/// A saved preset after a successful mutation.
#[derive(Clone, Debug)]
pub struct PresetSaved {
    /// The stable saved preset.
    pub preset: UserPreset,
    /// The new store revision.
    pub revision: StoreRevision,
}

/// A validated named preset and the action params to put in the form.
#[derive(Clone, Debug)]
pub struct PresetApplied {
    /// The current registry action after preset resolution.
    pub action: ActionDef,
}

/// One typed reply from the injected service. One per user action.
#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum PresetReply {
    /// The answer to List.
    List(PresetSnapshot),
    /// A SaveCurrent or Refresh succeeded.
    Saved(PresetSaved),
    /// A Rename succeeded.
    Renamed(PresetSaved),
    /// A Delete succeeded.
    Deleted(PresetSaved),
    /// An Apply succeeded. The UI still validates it before form refill.
    Applied(PresetApplied),
    /// A typed command failure.
    Failed(PresetFailure),
}

/// The daemon-side adapter for the UI's typed preset seam.
pub trait PresetService: Send + Sync {
    /// Sends one typed request and returns its pollable reply channel.
    fn submit(&self, request: PresetRequest) -> Result<Receiver<PresetReply>, PresetFailure>;
}

/// A pollable reply owned by one open customization panel.
pub trait PresetPoller: Send {
    /// The typed request this poller answers.
    fn request(&self) -> PresetRequest;
    /// Removes one finished reply without waiting.
    fn try_recv(&self) -> Result<PresetReply, TryRecvError>;
    /// True when the request still belongs to the given live panel.
    fn is_current(&self, session: u64, panel: u64) -> bool;
    /// Drops the reply channel and marks this poller retired.
    fn detach(&self);
}

/// The production poller over the standard reply receiver.
pub struct ChannelPoller {
    request: PresetRequest,
    receiver: std::sync::Mutex<Option<Receiver<PresetReply>>>,
}

impl ChannelPoller {
    /// Keeps one request and its receiver together.
    pub fn new(request: PresetRequest, receiver: Receiver<PresetReply>) -> Self {
        Self {
            request,
            receiver: std::sync::Mutex::new(Some(receiver)),
        }
    }
}

impl PresetPoller for ChannelPoller {
    fn request(&self) -> PresetRequest {
        self.request.clone()
    }

    fn try_recv(&self) -> Result<PresetReply, TryRecvError> {
        let guard = self
            .receiver
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(receiver) = guard.as_ref() else {
            return Err(TryRecvError::Disconnected);
        };
        receiver.try_recv()
    }

    fn is_current(&self, session: u64, panel: u64) -> bool {
        self.request.owns(session, panel)
    }

    fn detach(&self) {
        if let Ok(mut guard) = self.receiver.lock() {
            guard.take();
        }
    }
}
