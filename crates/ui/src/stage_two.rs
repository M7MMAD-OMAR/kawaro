//! S-119 stage 2: the option panel model, named presets and live preview.
//!
//! The panel is built from the action's [action.options] schema (S-117),
//! never from a hand written list per action: one row per field, one control
//! per field kind. Values resolve through `resolve_params`, the same core
//! function the CLI and MCP use, so the readout, the preview and the job all
//! agree.
//!
//! S-142 adds the transformed preview. The panel never calls an engine: it
//! asks the hive's scheduler, which owns one worker thread and one provider
//! injected by the daemon. The automatic draft waits for its debounce; the
//! encoded result is only ever asked for by the explicit control. A request
//! carries the resolved action, so the preview and the run describe the same
//! work, and every result carries the stamp of its request, so a result of a
//! closed panel or of an older set of values never reaches this panel.
//!
//! S-143-C adds named presets through an injected asynchronous service seam.
//! The daemon checks every command against its current registry action. The
//! UI never waits for the store, never submits a job on Apply, and drops the
//! poller when this panel closes or is replaced.
//!
//! Nothing here touches the hive or starts before the user opens the panel.

use crate::preview_schedule::{Failure, Outcome, Output, Publication, Scheduler};
use crate::saved_presets::{
    ChannelPoller, PresetApplied, PresetFailure, PresetItem, PresetPoller, PresetReply,
    PresetRequest, PresetSaved, PresetService, PresetSnapshot,
};
use crate::{SegmentChoice, StageKind, StageOption, StagePreset};
use kawaro_core::CancelToken;
use kawaro_core::action::ActionDef;
use kawaro_core::i18n::Lang;
use kawaro_core::image_geom::{self, Mode};
use kawaro_core::image_preview::{ImagePreviewMode, ImagePreviewRequest, PreviewStamp};
use kawaro_core::media;
use kawaro_core::options::{Field, Kind, OptionsError, OptionsSpec};
use kawaro_core::presets::StoreRevision;
use slint::{Model as _, ModelRc, Rgba8Pixel, SharedPixelBuffer, VecModel};
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::mpsc::TryRecvError;
use std::time::Instant;

/// Longest side of a panel draft, in pixels.
pub const MAX_DRAFT: u32 = 512;

/// The three footer labels of the panel, in the window language.
pub fn buttons(lang: Lang) -> (&'static str, &'static str, &'static str) {
    match lang {
        Lang::Ar => ("تشغيل", "إعادة الضبط", "رجوع"),
        Lang::En => ("Run", "Reset", "Back"),
    }
}

/// Named preset control labels, in the window language.
pub fn preset_labels(lang: Lang) -> PresetLabels {
    match lang {
        Lang::Ar => PresetLabels {
            name: String::from("اسم الإعداد"),
            save: String::from("حفظ الحالي"),
            apply: String::from("تطبيق"),
            rename: String::from("إعادة تسمية"),
            delete: String::from("حذف"),
            review: String::from("مراجعة وحفظ"),
        },
        Lang::En => PresetLabels {
            name: String::from("Preset name"),
            save: String::from("Save current"),
            apply: String::from("Apply"),
            rename: String::from("Rename"),
            delete: String::from("Delete"),
            review: String::from("Review and save"),
        },
    }
}

/// Bilingual labels of the secondary named preset controls.
pub struct PresetLabels {
    pub name: String,
    pub save: String,
    pub apply: String,
    pub rename: String,
    pub delete: String,
    pub review: String,
}

/// The hint that opens the panel, in the window language.
pub fn customize_hint(lang: Lang) -> &'static str {
    match lang {
        Lang::Ar => "تخصيص",
        Lang::En => "Customize",
    }
}

/// One Slint row for a named user preset. The row type is the one the hive's
/// `[NamedPresetRow]` property takes, so the panel hands its rows over
/// without a copy.
pub use crate::NamedPresetRow;

/// What one poll of the named-preset panel changes.
#[derive(Clone, Debug)]
pub struct PresetUpdate {
    pub rows: Vec<NamedPresetRow>,
    pub available: bool,
    pub selected_id: String,
    /// The name the panel's name field shows.
    pub name: String,
    pub status: String,
    pub read_only: bool,
    pub pending: bool,
    pub can_save: bool,
}

/// The state behind one open stage 2 panel.
pub struct StageTwoState {
    pub(crate) action: ActionDef,
    /// The values the user set, by param key. Empty means the configured
    /// base params from the current registry action.
    pub(crate) explicit: RefCell<toml::Table>,
    pub(crate) rows: Rc<VecModel<StageOption>>,
    /// Raw numeric drafts, including invalid and empty edits.
    pub(crate) drafts: RefCell<std::collections::BTreeMap<String, String>>,
    pub(crate) presets: Rc<VecModel<StagePreset>>,
    pub(crate) named_presets: Rc<VecModel<NamedPresetRow>>,
    pub(crate) preset_service: RefCell<Option<Arc<dyn PresetService>>>,
    pub(crate) preset_revision: RefCell<Option<StoreRevision>>,
    pub(crate) preset_items: RefCell<Vec<PresetItem>>,
    pub(crate) selected_preset: RefCell<Option<String>>,
    pub(crate) preset_read_only: Cell<bool>,
    pub(crate) preset_status: RefCell<String>,
    pub(crate) preset_name: RefCell<String>,
    pub(crate) pending_preset: RefCell<Option<Rc<dyn PresetPoller>>>,
    pub(crate) pending_preset_label: RefCell<Option<&'static str>>,
    pub(crate) preset_dirty: Cell<bool>,
    pub(crate) preset_request_counter: Cell<u64>,
    /// The session and panel stamps of the running service, so the same
    /// panel never asks the store twice for the same list.
    pub(crate) preset_attached: Cell<Option<(u64, u64)>>,
    /// The preset id whose delete is armed and waits for a second click.
    pub(crate) armed_delete: RefCell<Option<String>>,
    /// Header dimensions of the first file, when it is an image.
    pub(crate) source: Cell<Option<(u32, u32)>>,
    /// True when the first file is an image, for the preview note.
    pub(crate) image: bool,
    pub(crate) path: PathBuf,
    pub(crate) lang: Lang,
    pub(crate) rtl: bool,
    /// The preview link: the scheduler, the session, the panel id and the
    /// token of the panel's work. It is None until the hive opens the panel.
    pub(crate) link: RefCell<Option<Rc<PreviewLink>>>,
    /// What the panel shows for the preview.
    pub(crate) shown: RefCell<Shown>,
    /// True when the shown preview changed since the last poll.
    pub(crate) dirty: Cell<bool>,
}

impl StageTwoState {
    /// The panel for `action`, or None when it declares no options.
    pub fn build(
        action: &ActionDef,
        files: &[PathBuf],
        lang: Lang,
        rtl: bool,
    ) -> Option<StageTwoState> {
        action.options.as_ref()?;
        let path = files.first().cloned().unwrap_or_default();
        let image = media::detect(&path) == media::MediaKind::Image;
        let source = if image { probe(&path) } else { None };
        let state = StageTwoState {
            action: action.clone(),
            explicit: RefCell::new(toml::Table::new()),
            rows: Rc::new(VecModel::default()),
            drafts: RefCell::default(),
            presets: Rc::new(VecModel::default()),
            named_presets: Rc::new(VecModel::default()),
            preset_service: RefCell::new(None),
            preset_revision: RefCell::new(None),
            preset_items: RefCell::new(Vec::new()),
            selected_preset: RefCell::new(None),
            preset_read_only: Cell::new(false),
            preset_status: RefCell::new(String::new()),
            preset_name: RefCell::new(String::new()),
            pending_preset: RefCell::new(None),
            pending_preset_label: RefCell::new(None),
            preset_dirty: Cell::new(false),
            preset_request_counter: Cell::new(0),
            preset_attached: Cell::new(None),
            armed_delete: RefCell::new(None),
            source: Cell::new(source),
            image,
            path,
            lang,
            rtl,
            link: RefCell::new(None),
            shown: RefCell::new(Shown::default()),
            dirty: Cell::new(false),
        };
        state.rebuild();
        Some(state)
    }

    pub fn action_id(&self) -> &str {
        &self.action.id
    }

    pub fn lang(&self) -> Lang {
        self.lang
    }

    pub fn rtl(&self) -> bool {
        self.rtl
    }

    /// The action label, the panel title.
    pub fn title(&self) -> String {
        self.action.label.get(self.lang).to_string()
    }

    pub fn rows(&self) -> ModelRc<StageOption> {
        ModelRc::new(self.rows.clone())
    }

    pub fn presets(&self) -> ModelRc<StagePreset> {
        ModelRc::new(self.presets.clone())
    }

    pub fn named_presets(&self) -> ModelRc<NamedPresetRow> {
        ModelRc::new(self.named_presets.clone())
    }

    pub fn preset_status(&self) -> String {
        self.preset_status.borrow().clone()
    }

    pub fn preset_name(&self) -> String {
        self.preset_name.borrow().clone()
    }

    pub fn selected_preset(&self) -> Option<String> {
        self.selected_preset.borrow().clone()
    }

    pub fn set_preset_name(&self, name: &str) {
        *self.preset_name.borrow_mut() = name.to_string();
        // Another action drops the delete confirmation.
        self.disarm_delete();
        self.preset_dirty.set(true);
    }

    /// Selects one named preset row. The name field takes the name of that
    /// preset, so the rename control has a value to send.
    pub fn select_preset(&self, id: &str) {
        let name = {
            let items = self.preset_items.borrow();
            let Some(item) = items.iter().find(|item| item.id == id) else {
                return;
            };
            item.name.clone()
        };
        *self.selected_preset.borrow_mut() = Some(id.to_string());
        *self.preset_name.borrow_mut() = name;
        self.disarm_delete();
        *self.preset_status.borrow_mut() = String::new();
        self.refresh_named_rows();
        self.preset_dirty.set(true);
    }

    /// True when a named preset service is installed and the current action
    /// is in the initial saved scope.
    pub fn named_presets_available(&self) -> bool {
        self.preset_service.borrow().is_some()
            && kawaro_core::presets::validate_preset(
                &kawaro_core::presets::UserPreset {
                    id: String::from("preset-probe"),
                    name: String::from("probe"),
                    action_id: self.action.id.clone(),
                    schema_fingerprint: kawaro_core::presets::fingerprint(&self.action),
                    values: toml::Table::new(),
                },
                Some(&self.action),
            )
            .is_ready()
    }

    pub fn preset_pending(&self) -> bool {
        self.pending_preset.borrow().is_some()
    }

    /// True when the store cannot be written to, so the controls that change
    /// it stay disabled.
    pub fn preset_read_only(&self) -> bool {
        self.preset_read_only.get()
    }

    /// True when the displayed form resolves and a nonempty name is present.
    pub fn can_save_preset(&self) -> bool {
        !self.preset_pending()
            && !self.preset_read_only.get()
            && self.named_presets_available()
            && !self.preset_name.borrow().trim().is_empty()
            && self.resolved().is_ok()
    }

    /// Installs the injectable service and starts one stamped List request.
    /// Without a service, the named preset controls remain omitted.
    ///
    /// The same panel is attached twice in production: the hive attaches the
    /// service when the panel opens and the daemon confirms it with the same
    /// session and panel stamps. The second attach keeps the running list
    /// and asks the store only once.
    pub fn attach_preset_service(&self, service: Arc<dyn PresetService>, session: u64, panel: u64) {
        *self.preset_service.borrow_mut() = Some(service);
        if self.preset_attached.get() == Some((session, panel)) {
            return;
        }
        *self.preset_items.borrow_mut() = Vec::new();
        *self.selected_preset.borrow_mut() = None;
        *self.preset_revision.borrow_mut() = None;
        self.preset_read_only.set(false);
        *self.preset_status.borrow_mut() = String::new();
        self.pending_preset.borrow_mut().take();
        self.pending_preset_label.borrow_mut().take();
        self.rebuild_named_rows();
        self.preset_attached.set(Some((session, panel)));
        if self.named_presets_available() {
            self.request_preset(PresetRequest::List {
                session,
                panel,
                request: self.next_preset_request(),
                action: self.action.clone(),
            });
        } else {
            self.preset_dirty.set(true);
        }
    }

    /// Ends the panel: cancel preview and preset work, clear the pictures
    /// and retire the panel instance.
    pub fn end_stage(&self) {
        self.detach_preview();
        self.detach_preset_service();
        self.dirty.set(true);
    }

    /// Drops the service and poller with the closing panel. A late reply is
    /// then disconnected and cannot mutate a replacement.
    pub fn detach_preset_service(&self) {
        if let Some(pending) = self.pending_preset.borrow_mut().take() {
            pending.detach();
        }
        *self.preset_service.borrow_mut() = None;
        *self.pending_preset_label.borrow_mut() = None;
        *self.preset_items.borrow_mut() = Vec::new();
        *self.selected_preset.borrow_mut() = None;
        *self.preset_revision.borrow_mut() = None;
        *self.preset_name.borrow_mut() = String::new();
        *self.armed_delete.borrow_mut() = None;
        self.preset_attached.set(None);
        self.preset_read_only.set(false);
        *self.preset_status.borrow_mut() = String::new();
        self.named_presets.set_vec(Vec::new());
    }

    /// Saves the exact displayed resolved form values under a trimmed name.
    pub fn save_current_preset(&self, session: u64, panel: u64) {
        if self.pending_preset.borrow().is_some() || self.preset_read_only.get() {
            return;
        }
        self.disarm_delete();
        if !self.named_presets_available() {
            self.set_preset_failure(&PresetFailure::Unsupported(String::from(
                "the current action cannot be saved as a preset",
            )));
            return;
        }
        let name = self.preset_name.borrow().trim().to_string();
        let can_save = self.can_save_preset();
        if !can_save {
            let failure = if self.preset_read_only.get() {
                PresetFailure::StoreReadOnly(String::from("the preset store is read only"))
            } else if self.preset_pending() {
                PresetFailure::Busy
            } else if name.is_empty() {
                PresetFailure::Invalid(String::from("the preset name is empty"))
            } else if self.resolved().is_err() {
                PresetFailure::Invalid(String::from("the form values do not resolve"))
            } else {
                PresetFailure::Unsupported(String::from(
                    "the current action cannot be saved as a preset",
                ))
            };
            self.set_preset_failure(&failure);
            return;
        }
        let params = self.resolved().expect("can_save_preset checked resolution");
        let mut action = self.action.clone();
        action.params = params;
        let expected = self
            .preset_revision
            .borrow()
            .clone()
            .unwrap_or_else(kawaro_core::presets::missing_revision);
        self.request_preset(PresetRequest::SaveCurrent {
            session,
            panel,
            request: self.next_preset_request(),
            action,
            action_id: self.action.id.clone(),
            name,
            expected,
        });
    }

    /// Renames one stable preset id using the panel snapshot revision.
    pub fn rename_preset(&self, session: u64, panel: u64, id: &str, name: &str) {
        if !self.can_mutate_named() {
            return;
        }
        self.disarm_delete();
        let name = name.trim();
        if name.is_empty() {
            self.set_preset_failure(&PresetFailure::Invalid(String::from(
                "the preset name is empty",
            )));
            return;
        }
        let Some(expected) = self.preset_revision.borrow().clone() else {
            self.set_preset_failure(&PresetFailure::Conflict(String::from(
                "the store revision is not loaded",
            )));
            return;
        };
        self.request_preset(PresetRequest::Rename {
            session,
            panel,
            request: self.next_preset_request(),
            action_id: self.action.id.clone(),
            id: id.to_string(),
            name: name.to_string(),
            expected,
        });
    }

    /// Asks for explicit local confirmation. The first click arms the id and
    /// the second click on the same id in the same panel confirms it.
    pub fn delete_preset(&self, session: u64, panel: u64, id: &str) -> bool {
        if !self.can_mutate_named() {
            return false;
        }
        let armed = self.armed_delete.borrow().clone();
        if armed.as_deref() != Some(id) {
            // The first click only arms the confirmation.
            *self.armed_delete.borrow_mut() = Some(id.to_string());
            *self.preset_status.borrow_mut() = delete_prompt(self.lang).into();
            self.preset_dirty.set(true);
            return false;
        }
        let Some(expected) = self.preset_revision.borrow().clone() else {
            self.set_preset_failure(&PresetFailure::Conflict(String::from(
                "the store revision is not loaded",
            )));
            return false;
        };
        *self.armed_delete.borrow_mut() = None;
        *self.preset_status.borrow_mut() = String::new();
        self.request_preset(PresetRequest::Delete {
            session,
            panel,
            request: self.next_preset_request(),
            action_id: self.action.id.clone(),
            id: id.to_string(),
            expected,
        });
        true
    }

    /// Any other named preset action drops the armed delete confirmation.
    pub(crate) fn disarm_delete(&self) {
        if self.armed_delete.borrow().is_none() {
            return;
        }
        *self.armed_delete.borrow_mut() = None;
        if self.preset_status.borrow().eq(&delete_prompt(self.lang)) {
            *self.preset_status.borrow_mut() = String::new();
        }
        self.preset_dirty.set(true);
    }

    /// Explicit review: refresh only the displayed compatible values for
    /// this preset. The daemon checks the current registry and the snapshot
    /// revision before it writes anything.
    pub fn review_preset(&self, session: u64, panel: u64, id: &str) {
        if !self.can_mutate_named() {
            return;
        }
        self.disarm_delete();
        let Some(item) = self
            .preset_items
            .borrow()
            .iter()
            .find(|item| item.id == id)
            .cloned()
        else {
            return;
        };
        if !matches!(
            item.status,
            kawaro_core::presets::PresetStatus::NeedsReview(_)
        ) {
            self.set_preset_failure(&PresetFailure::Invalid(String::from(
                "only a preset needing review can be refreshed",
            )));
            return;
        }
        let Ok(params) = self.resolved() else {
            self.set_preset_failure(&PresetFailure::Invalid(String::from(
                "the form values do not resolve",
            )));
            return;
        };
        let mut action = self.action.clone();
        action.params = params;
        let Some(expected) = self.preset_revision.borrow().clone() else {
            self.set_preset_failure(&PresetFailure::Conflict(String::from(
                "the store revision is not loaded",
            )));
            return;
        };
        self.request_preset(PresetRequest::Refresh {
            session,
            panel,
            request: self.next_preset_request(),
            action,
            action_id: self.action.id.clone(),
            id: id.to_string(),
            expected,
        });
    }

    /// Applies a Ready named preset to the form and asks for exactly one
    /// transformed draft. It never submits or runs a job.
    pub fn apply_named_preset(&self, session: u64, panel: u64, id: &str) {
        if self.pending_preset.borrow().is_some() || !self.named_presets_available() {
            return;
        }
        self.disarm_delete();
        let exists = self
            .preset_items
            .borrow()
            .iter()
            .any(|item| item.id == id && item.can_apply());
        if !exists {
            self.set_preset_failure(&PresetFailure::ActionUnavailable(String::from(
                "the preset is unavailable in the current registry",
            )));
            return;
        }
        self.request_preset(PresetRequest::Apply {
            session,
            panel,
            request: self.next_preset_request(),
            action: self.action.clone(),
            action_id: self.action.id.clone(),
            id: id.to_string(),
        });
    }

    /// Polls one finished typed reply. Ownership and action stamps are checked
    /// again here, so a retired poller or another action cannot publish.
    pub fn poll_preset(&self, session: u64, panel: u64) -> Option<PresetUpdate> {
        let Some(current) = self.pending_preset.borrow().clone() else {
            return self
                .preset_dirty
                .replace(false)
                .then(|| self.preset_update());
        };
        if !current.is_current(session, panel) || current.request().action_id() != self.action.id {
            self.drop_pending_preset();
            self.preset_dirty.set(true);
            return Some(self.preset_update());
        }
        let reply = match current.try_recv() {
            Ok(reply) => reply,
            Err(TryRecvError::Empty) => {
                return self
                    .preset_dirty
                    .replace(false)
                    .then(|| self.preset_update());
            }
            Err(TryRecvError::Disconnected) => {
                self.drop_pending_preset();
                *self.preset_status.borrow_mut() =
                    preset_failure_text(self.lang, &PresetFailure::ChannelClosed);
                self.preset_dirty.set(true);
                return Some(self.preset_update());
            }
        };
        let request = current.request();
        self.drop_pending_preset();
        self.deliver_preset_reply(request, reply, session, panel);
        self.preset_dirty.set(true);
        Some(self.preset_update())
    }

    fn deliver_preset_reply(
        &self,
        current: PresetRequest,
        reply: PresetReply,
        session: u64,
        panel: u64,
    ) {
        if !current.owns(session, panel) || current.action_id() != self.action.id {
            return;
        }
        match (current, reply) {
            (PresetRequest::List { action, .. }, PresetReply::List(snapshot))
                if action.id == self.action.id =>
            {
                self.apply_preset_snapshot(snapshot);
            }
            (
                PresetRequest::SaveCurrent { action, name, .. },
                PresetReply::Saved(PresetSaved { preset, revision }),
            ) if action.id == self.action.id => {
                if preset.name == name {
                    self.apply_saved_revision(revision);
                    self.refresh_preset_list(session, panel);
                    self.set_preset_success(saved_text(self.lang));
                } else {
                    self.set_preset_failure(&PresetFailure::ChannelClosed);
                }
            }
            (PresetRequest::Rename { action_id, id, .. }, PresetReply::Renamed(saved))
                if action_id == self.action.id && saved.preset.id == id =>
            {
                self.apply_saved_revision(saved.revision);
                self.refresh_preset_list(session, panel);
                self.set_preset_success(renamed_text(self.lang));
            }
            (PresetRequest::Delete { action_id, id, .. }, PresetReply::Deleted(saved))
                if action_id == self.action.id && saved.preset.id == id =>
            {
                self.apply_saved_revision(saved.revision);
                self.choose_after_delete(&id);
                self.refresh_preset_list(session, panel);
                self.set_preset_success(deleted_text(self.lang));
            }
            (
                PresetRequest::Refresh {
                    action,
                    action_id,
                    id,
                    ..
                },
                PresetReply::Saved(saved),
            ) if action.id == self.action.id
                && action_id == self.action.id
                && saved.preset.id == id =>
            {
                self.apply_saved_revision(saved.revision);
                self.refresh_preset_list(session, panel);
                self.set_preset_success(refreshed_text(self.lang));
            }
            (
                PresetRequest::Apply {
                    action,
                    action_id,
                    id,
                    ..
                },
                PresetReply::Applied(applied),
            ) if action.id == self.action.id && action_id == self.action.id => {
                if let Err(error) = self.apply_resolved_named(&id, applied) {
                    self.set_preset_failure(&PresetFailure::Invalid(error.to_string()));
                }
            }
            (_, PresetReply::Failed(PresetFailure::Conflict(_))) => {
                self.set_preset_failure(&PresetFailure::Conflict(String::from(
                    "the saved preset store changed",
                )));
                self.refresh_preset_list(session, panel);
            }
            (_, PresetReply::Failed(failure)) => {
                self.set_preset_failure(&failure);
            }
            _ => self.set_preset_failure(&PresetFailure::ChannelClosed),
        }
    }

    pub(crate) fn request_preset(&self, request: PresetRequest) {
        // Every request carries the stamps of the live panel. A zero stamp is
        // never sent: the service would refuse it.
        if request.session() == 0 || request.panel() == 0 || request.request() == 0 {
            self.set_preset_failure(&PresetFailure::Invalid(String::from(
                "the preset request is not stamped",
            )));
            return;
        }
        let action_id = request.action_id().to_string();
        let service = self.preset_service.borrow().clone();
        let Some(service) = service else {
            self.set_preset_failure(&PresetFailure::ServiceUnavailable(String::from(
                "no preset service is installed",
            )));
            return;
        };
        if action_id != self.action.id {
            return;
        }
        match service.submit(request.clone()) {
            Ok(receiver) => {
                self.drop_pending_preset();
                let label = pending_label(self.lang, &request);
                *self.pending_preset_label.borrow_mut() = Some(label);
                *self.preset_status.borrow_mut() = label.into();
                *self.pending_preset.borrow_mut() =
                    Some(Rc::new(ChannelPoller::new(request, receiver)));
                self.preset_dirty.set(true);
            }
            Err(failure) => self.set_preset_failure(&failure),
        }
    }

    pub(crate) fn drop_pending_preset(&self) {
        if let Some(pending) = self.pending_preset.borrow_mut().take() {
            pending.detach();
        }
        *self.pending_preset_label.borrow_mut() = None;
    }

    pub(crate) fn apply_preset_snapshot(&self, snapshot: PresetSnapshot) {
        *self.preset_revision.borrow_mut() = Some(snapshot.revision);
        self.preset_read_only.set(snapshot.read_only.is_some());
        let selected = self.valid_selected_id();
        *self.preset_items.borrow_mut() = snapshot.items;
        if let Some(selected) = selected {
            *self.selected_preset.borrow_mut() = Some(selected);
        } else {
            self.choose_default_preset();
        }
        self.refresh_named_rows();
        if let Some(error) = snapshot.read_only {
            *self.preset_status.borrow_mut() = preset_failure_text(self.lang, &error);
        } else {
            *self.preset_status.borrow_mut() = String::new();
        }
    }

    pub(crate) fn apply_saved_revision(&self, revision: StoreRevision) {
        *self.preset_revision.borrow_mut() = Some(revision);
    }

    pub(crate) fn refresh_preset_list(&self, session: u64, panel: u64) {
        self.request_preset(PresetRequest::List {
            session,
            panel,
            request: self.next_preset_request(),
            action: self.action.clone(),
        });
    }

    pub(crate) fn refresh_named_rows(&self) {
        self.rebuild_named_rows();
    }

    pub(crate) fn apply_resolved_named(
        &self,
        id: &str,
        applied: PresetApplied,
    ) -> Result<(), OptionsError> {
        if applied.action.id != self.action.id {
            return Err(OptionsError::Unknown(String::from(
                "the preset action changed",
            )));
        }
        let options = self
            .action
            .options
            .as_ref()
            .ok_or_else(|| OptionsError::Unknown(String::from("the action has no options")))?;
        let values = kawaro_core::presets::field_values(options, &applied.action.params);
        self.replace_explicit_with_fields(options, &values)?;
        let resolved = self.resolved_action()?;
        if resolved.params != applied.action.params {
            return Err(OptionsError::Value {
                name: self.action.id.clone(),
                reason: String::from("the preset did not resolve to the current form values"),
            });
        }
        *self.selected_preset.borrow_mut() = Some(id.to_string());
        self.refresh_all();
        self.request_preview();
        Ok(())
    }

    pub(crate) fn replace_explicit_with_fields(
        &self,
        options: &OptionsSpec,
        values: &toml::Table,
    ) -> Result<(), OptionsError> {
        self.explicit.borrow_mut().clear();
        self.drafts.borrow_mut().clear();
        {
            let mut explicit = self.explicit.borrow_mut();
            for (field_id, value) in values {
                let field = options
                    .fields
                    .iter()
                    .find(|field| &field.id == field_id)
                    .ok_or_else(|| OptionsError::Unknown(field_id.clone()))?;
                for key in &field.clears {
                    explicit.remove(key);
                }
                explicit.insert(field.param_key().to_string(), value.clone());
            }
        }
        self.drafts.borrow_mut().clear();
        Ok(())
    }

    pub(crate) fn choose_after_delete(&self, deleted: &str) {
        let items = self.preset_items.borrow();
        let deleted_position = items.iter().position(|item| item.id == deleted);
        let next_id = deleted_position
            .and_then(|position| items.get(position).or_else(|| items.last()))
            .or_else(|| items.first())
            .map(|item| item.id.clone());
        drop(items);
        *self.selected_preset.borrow_mut() = next_id;
    }

    pub(crate) fn valid_selected_id(&self) -> Option<String> {
        let items = self.preset_items.borrow();
        let selected = self.selected_preset.borrow();
        selected
            .as_ref()
            .filter(|id| items.iter().any(|item| &item.id == *id))
            .cloned()
            .or_else(|| items.first().map(|item| item.id.clone()))
    }

    pub(crate) fn choose_default_preset(&self) {
        let selected = self
            .preset_items
            .borrow()
            .first()
            .map(|item| item.id.clone());
        *self.selected_preset.borrow_mut() = selected;
    }

    pub(crate) fn can_mutate_named(&self) -> bool {
        !self.pending_preset.borrow().is_some()
            && !self.preset_read_only.get()
            && self.named_presets_available()
    }

    pub(crate) fn set_preset_failure(&self, failure: &PresetFailure) {
        // A store that cannot be read or written keeps every control that
        // changes it disabled.
        if matches!(failure, PresetFailure::StoreReadOnly(_)) {
            self.preset_read_only.set(true);
        }
        *self.preset_status.borrow_mut() = preset_failure_text(self.lang, failure);
        self.preset_dirty.set(true);
    }

    pub(crate) fn set_preset_success(&self, message: &str) {
        *self.preset_status.borrow_mut() = message.to_string();
        self.preset_dirty.set(true);
    }

    pub(crate) fn next_preset_request(&self) -> u64 {
        let value = self.preset_request_counter.get().wrapping_add(1);
        self.preset_request_counter.set(value);
        value
    }

    pub(crate) fn rebuild_named_rows(&self) {
        let selected = self.selected_preset.borrow().clone();
        let rows: Vec<NamedPresetRow> = self
            .preset_items
            .borrow()
            .iter()
            .map(|item| {
                let selected_row = selected.as_deref() == Some(item.id.as_str());
                let status = match &item.status {
                    kawaro_core::presets::PresetStatus::Ready => ready_text(self.lang),
                    kawaro_core::presets::PresetStatus::NeedsReview(reason) => {
                        review_text(self.lang, reason)
                    }
                    kawaro_core::presets::PresetStatus::Unavailable(reason) => {
                        unavailable_text(self.lang, reason)
                    }
                };
                // The selected row says so in its detail line, so the chip
                // needs no checked state of its own.
                let detail = if selected_row {
                    selected_text(self.lang, &status)
                } else if matches!(&item.status, kawaro_core::presets::PresetStatus::Ready) {
                    String::new()
                } else {
                    status
                };
                NamedPresetRow {
                    id: item.id.as_str().into(),
                    name: item.name.as_str().into(),
                    detail: detail.as_str().into(),
                    can_apply: item.can_apply(),
                    can_review: matches!(
                        item.status,
                        kawaro_core::presets::PresetStatus::NeedsReview(_)
                    ),
                }
            })
            .collect();
        self.named_presets.set_vec(rows);
    }

    /// A read only snapshot of the named controls for Slint or an async poll.
    pub fn preset_update(&self) -> PresetUpdate {
        self.rebuild_named_rows();
        PresetUpdate {
            rows: self.named_presets.iter().collect(),
            available: self.named_presets_available(),
            selected_id: self.selected_preset.borrow().clone().unwrap_or_default(),
            name: self.preset_name.borrow().clone(),
            status: self.preset_status.borrow().clone(),
            read_only: self.preset_read_only.get(),
            pending: self.pending_preset.borrow().is_some(),
            can_save: self.can_save_preset(),
        }
    }

    /// True when the first file is an image with readable dimensions, so a
    /// picture is expected (the decode may still fail softly).
    pub fn preview_available(&self) -> bool {
        self.source.get().is_some()
    }

    /// A value changed in the panel: write it to the explicit table, prune
    /// the keys the field clears, and refresh the row and the preview.
    pub fn set_value(&self, index: i32, numeric: f32, text: &str) {
        let Some(field) = self.field(index) else {
            return;
        };
        let Some(value) = self.value_of(field, numeric, text) else {
            return;
        };
        {
            let mut explicit = self.explicit.borrow_mut();
            for key in &field.clears {
                explicit.remove(key);
            }
            explicit.insert(field.param_key().to_string(), value);
        }
        self.drafts.borrow_mut().remove(field.param_key());
        for key in &field.clears {
            self.drafts.borrow_mut().remove(key);
        }
        let field = field.clone();
        self.refresh_row(index, &field);
        // A cleared sibling changes the resolved table, so refresh every row
        // that the field names.
        self.refresh_cleared(&field);
        self.request_preview();
    }

    /// Preserve raw text independently of the last resolved number. Invalid
    /// drafts are explicit values too, so Run cannot reuse an older value.
    pub fn set_draft(&self, index: i32, text: &str) {
        let Some(field) = self.field(index).cloned() else {
            return;
        };
        let value = match field.kind {
            Kind::Int | Kind::Percent | Kind::Px => {
                text.parse::<i64>().ok().map(toml::Value::Integer)
            }
            Kind::Float => text
                .parse::<f64>()
                .ok()
                .filter(|n| n.is_finite())
                .map(toml::Value::Float),
            _ => None,
        }
        .unwrap_or_else(|| toml::Value::String(text.to_string()));
        {
            let mut explicit = self.explicit.borrow_mut();
            let mut drafts = self.drafts.borrow_mut();
            for key in &field.clears {
                explicit.remove(key);
                drafts.remove(key);
            }
            drafts.insert(field.param_key().into(), text.into());
            explicit.insert(field.param_key().into(), value);
        }
        self.refresh_row(index, &field);
        self.refresh_cleared(&field);
        self.request_preview();
    }

    /// An action defined preset was picked: its values name field ids, so
    /// they go into the explicit table under each field's param key. The
    /// named user presets have their own path and never come here.
    pub fn apply_preset(&self, index: i32) {
        let Some(options) = &self.action.options else {
            return;
        };
        let Some(preset) = options.presets.get(usize::try_from(index).unwrap_or(0)) else {
            return;
        };
        let values = preset.values.clone();
        {
            let mut explicit = self.explicit.borrow_mut();
            for (key, value) in &values {
                let Some(field) = options.fields.iter().find(|f| &f.id == key) else {
                    continue;
                };
                self.drafts.borrow_mut().remove(field.param_key());
                for clear in &field.clears {
                    explicit.remove(clear);
                    self.drafts.borrow_mut().remove(clear);
                }
                explicit.insert(field.param_key().to_string(), value.clone());
            }
        }
        self.refresh_all();
        self.request_preview();
    }

    /// Reset returns every control to the configured base of the current
    /// registry action, never a saved or last-used preset.
    pub fn reset(&self) {
        self.explicit.borrow_mut().clear();
        self.drafts.borrow_mut().clear();
        self.refresh_all();
        self.request_preview();
    }

    /// The params a Run submits: the configured base with explicit values.
    pub fn resolved(&self) -> Result<toml::Table, OptionsError> {
        self.resolved_action().map(|action| action.params)
    }

    /// The current action with only the explicit form values resolved. The
    /// daemon validates this exact action against the current registry and
    /// submits it with `jobs::submit_resolved`, so global quality cannot
    /// overwrite explicit choices again.
    pub(crate) fn resolved_action(&self) -> Result<ActionDef, OptionsError> {
        match &self.action.options {
            Some(options) => {
                let explicit = self.explicit.borrow();
                let params = options.resolve_params(&self.action.params, &explicit)?;
                for field in &options.fields {
                    let number = match explicit.get(field.param_key()) {
                        Some(toml::Value::Integer(n)) => *n as f64,
                        Some(toml::Value::Float(n)) => *n,
                        _ => continue,
                    };
                    let invalid_step = field.step.filter(|s| *s > 0.).is_some_and(|step| {
                        let steps = (number - field.min.unwrap_or(0.)) / step;
                        (steps - steps.round()).abs() > 1e-7
                    });
                    if !number.is_finite() || invalid_step {
                        return Err(OptionsError::Value {
                            name: field.id.clone(),
                            reason: if invalid_step {
                                format!("expected steps of {}", field.step.unwrap())
                            } else {
                                "expected a finite number".into()
                            },
                        });
                    }
                }
                Ok(ActionDef {
                    params,
                    ..self.action.clone()
                })
            }
            None => Ok(self.action.clone()),
        }
    }

    /// The explicit form overrides, keyed by engine param key. The daemon
    /// validates them once more against the current registry action before
    /// it submits the resolved action.
    pub fn explicit_params(&self) -> toml::Table {
        self.explicit.borrow().clone()
    }

    /// One readout line: dimensions, ratio and percent for an image resize,
    /// else the resolved params as short key: value pairs.
    /// "Preview not available" when an image source has no usable picture.
    pub fn preview_note(&self) -> Option<&'static str> {
        if !self.image || self.preview_available() {
            return None;
        }
        Some(match self.lang {
            Lang::Ar => "المعاينة غير متاحة",
            Lang::En => "Preview not available",
        })
    }

    /// One readout line from the action's readout tokens, in their order:
    /// dimensions, ratio, percent and the output name. The line is wrapped in
    /// the bidi isolates U+2068 and U+2069 so digits and Arabic keep their
    /// order. Without tokens it falls back to the resolved params.
    ///
    /// The size is the intended output of the last result, read from the
    /// engine artifact. The display proxy never replaces it.
    pub fn readout(&self) -> String {
        let Ok(params) = self.resolved() else {
            return String::new();
        };
        let target = self.dimension_target(&params);
        let size = self
            .output_size()
            .or_else(|| target.map(|target| (target.width, target.height)));
        let mut parts = Vec::new();
        if let Some(options) = &self.action.options {
            for token in &options.readout {
                match token.as_str() {
                    "dimensions" => {
                        if let Some((width, height)) = size {
                            parts.push(format!("{width} x {height}"));
                        }
                    }
                    "ratio" => {
                        if let Some((width, height)) = size {
                            let divisor = gcd(width, height);
                            parts.push(format!("{}:{}", width / divisor, height / divisor));
                        }
                    }
                    "percent" => {
                        if let Some((width, height)) = size {
                            parts.push(format!("{:+}%", self.percent_change(width, height)));
                        }
                    }
                    "output_name" => {
                        if let Some(name) = self.output_name() {
                            parts.push(name);
                        }
                    }
                    _ => {}
                }
            }
        }
        if parts.is_empty() {
            for (key, value) in &params {
                parts.push(format!("{key}: {}", display(value)));
            }
        }
        format!("\u{2068}{}\u{2069}", parts.join(", "))
    }

    /// The name the output takes next to the original: the stem, the action's
    /// suffix and its extension (or the input's). A combined action names it
    /// after the first input.
    fn output_name(&self) -> Option<String> {
        let name = self.path.file_name()?.to_str()?;
        let stem = Path::new(name).file_stem()?.to_str()?;
        let extension = match self.output_format() {
            Some(format) if !format.is_empty() => format,
            _ => {
                if self.action.output.ext.is_empty() {
                    media::extension(&self.path)
                } else {
                    self.action.output.ext.clone()
                }
            }
        };
        let mut out = format!("{stem}{}", self.action.output.suffix);
        if !extension.is_empty() {
            out.push('.');
            out.push_str(&extension);
        }
        Some(out)
    }

    /// Attaches the preview scheduler to this panel (S-142). The hive calls
    /// it when the panel opens, so the panel and the hive share one worker
    /// and one provider. A panel without a scheduler asks for nothing and
    /// shows no picture.
    pub fn attach_preview(&self, scheduler: Rc<Scheduler>, session: u64, panel: u64) {
        *self.link.borrow_mut() = Some(Rc::new(PreviewLink {
            scheduler,
            session,
            panel,
            token: RefCell::new(CancelToken::new()),
            stamp: Cell::new(None),
            mode: Cell::new(ImagePreviewMode::TransformedDraft),
        }));
        *self.shown.borrow_mut() = Shown::default();
        self.dirty.set(false);
    }

    /// The panel closes: its work is cancelled and its result is dropped.
    /// A result that arrives later belongs to a stamp this panel no longer
    /// has, so it is never published.
    pub(crate) fn detach_preview(&self) {
        let link = self.link.borrow_mut().take();
        if let Some(link) = link {
            link.token.borrow().cancel();
            link.scheduler.cancel_panel(link.session, link.panel);
        }
        *self.shown.borrow_mut() = Shown::default();
        self.dirty.set(false);
    }

    /// Asks for a preview of the current values (S-142).
    ///
    /// The automatic draft waits for its debounce, so a slider drag does not
    /// decode a picture per step.
    pub fn request_preview(&self) {
        self.request(ImagePreviewMode::TransformedDraft);
    }

    /// Asks for the encoded result of the current values (S-142).
    ///
    /// This is the explicit control. It never runs on its own, and it never
    /// runs the byte target search of `compress-to` without the user asking.
    pub fn request_encoded(&self) {
        self.request(ImagePreviewMode::EncodedResult);
    }

    /// Queues one preview request for the current values.
    fn request(&self, mode: ImagePreviewMode) {
        let Some(link) = self.link.borrow().clone() else {
            return;
        };
        if !link.scheduler.has_provider() {
            // No engine behind the panel: the legacy source draft, a bounded
            // local decode. It is a picture of the source, never a result.
            #[cfg(unix)]
            {
                if self.preview_available() {
                    let stamp = link.scheduler.stamp(link.session, link.panel);
                    link.stamp.set(Some(stamp));
                    link.scheduler.request_source(
                        self.path.clone(),
                        stamp,
                        link.token.borrow().clone(),
                    );
                }
            }
            return;
        }
        if !self.image || self.action.engine != "image" {
            return;
        }
        let Ok(action) = self.resolved_action() else {
            // An invalid draft: the result is cleared and nothing is asked
            // for, so an old picture never stays on the panel.
            self.stop_work();
            return;
        };
        let stamp = link.scheduler.stamp(link.session, link.panel);
        link.stamp.set(Some(stamp));
        link.mode.set(mode);
        let request = ImagePreviewRequest {
            stamp,
            input: self.path.clone(),
            resolved_action: action,
            mode,
        };
        let token = link.token.borrow().clone();
        match mode {
            ImagePreviewMode::TransformedDraft => link.scheduler.request_draft(request, token),
            ImagePreviewMode::EncodedResult => link.scheduler.request_encoded(request, token),
        }
    }

    /// Stops the panel's current work and clears its result.
    ///
    /// The panel keeps its label, so the preview area still says what it
    /// would show, but it shows no picture and no byte count: the source is
    /// not a result.
    fn stop_work(&self) {
        let link = self.link.borrow().clone();
        let mode = link
            .as_ref()
            .map(|link| link.mode.get())
            .unwrap_or(ImagePreviewMode::TransformedDraft);
        if let Some(link) = link {
            link.token.borrow().cancel();
            link.scheduler.cancel_panel(link.session, link.panel);
            // A new token, so the stopped work never stops a later request.
            *link.token.borrow_mut() = CancelToken::new();
            // A new stamp, so the cancelled result is never published here.
            link.stamp
                .set(Some(link.scheduler.stamp(link.session, link.panel)));
        }
        let label = self.label_for(mode);
        self.dirty.set(true);
        let mut shown = self.shown.borrow_mut();
        shown.image = None;
        shown.bytes = None;
        shown.output = None;
        shown.failure = None;
        shown.label = label;
    }

    /// The newest finished preview of this panel, ready to publish.
    ///
    /// Only the result whose stamp is the newest request of this panel is
    /// taken: a result of a closed panel, of another session or of an older
    /// set of values is dropped.
    pub fn poll_preview(&self) -> Option<PreviewUpdate> {
        let link = self.link.borrow().clone()?;
        link.scheduler.tick(Instant::now());
        let mut newest: Option<Outcome> = None;
        while let Some(outcome) = link.scheduler.poll() {
            if Some(outcome.stamp()) != link.stamp.get() {
                continue;
            }
            newest = Some(outcome);
        }
        if let Some(outcome) = newest {
            self.apply(outcome, link.mode.get());
            self.dirty.set(true);
        }
        if !self.dirty.replace(false) {
            return None;
        }
        Some(self.update())
    }

    /// Puts one finished outcome into the shown state.
    fn apply(&self, outcome: Outcome, mode: ImagePreviewMode) {
        match outcome {
            Outcome::Ready(publication) => {
                let label = self.label_of(&publication);
                let bytes = publication
                    .output
                    .as_ref()
                    .and_then(|output| output.final_encoded_bytes);
                let output = publication.output.clone();
                let mut shown = self.shown.borrow_mut();
                shown.image = Some(publication.image);
                shown.label = label;
                shown.bytes = bytes;
                shown.failure = None;
                shown.output = output;
            }
            Outcome::Failed { kind, .. } => {
                let label = self.label_for(mode);
                let mut shown = self.shown.borrow_mut();
                // A failure never leaves an old picture on the panel: the
                // source is not a result.
                shown.image = None;
                shown.bytes = None;
                shown.output = None;
                shown.label = label;
                shown.failure = Some(kind);
            }
        }
    }

    /// The shown state, ready for the interface.
    fn update(&self) -> PreviewUpdate {
        let shown = self.shown.borrow();
        PreviewUpdate {
            image: shown.image.clone(),
            label: shown.label.clone(),
            bytes: shown.bytes,
            failure: shown.failure,
            output: shown.output.clone(),
        }
    }

    /// The label of the preview area before any result: the mode that is
    /// being asked for.
    pub(crate) fn preview_label(&self) -> String {
        let link = self.link.borrow().clone();
        match link {
            Some(link) if !link.scheduler.has_provider() => String::from(source_label(self.lang)),
            Some(link) => self.label_for(link.mode.get()),
            None => self.label_for(ImagePreviewMode::TransformedDraft),
        }
    }

    /// True when the explicit encoded control is offered: an image action
    /// with an engine behind it.
    pub(crate) fn encode_offered(&self) -> bool {
        let link = self.link.borrow().clone();
        let Some(link) = link else {
            return false;
        };
        link.scheduler.has_provider() && self.image && self.action.engine == "image"
    }

    /// The label of a mode: the picture names what it is.
    pub(crate) fn label_for(&self, mode: ImagePreviewMode) -> String {
        match mode {
            ImagePreviewMode::EncodedResult => match self.lang {
                Lang::Ar => String::from("النتيجة المرمّزة"),
                Lang::En => String::from("Encoded result"),
            },
            ImagePreviewMode::TransformedDraft => match self.lang {
                Lang::Ar => String::from("مسودة النتيجة"),
                Lang::En => String::from("Transformed draft"),
            },
        }
    }

    /// The label of a published picture.
    fn label_of(&self, publication: &Publication) -> String {
        if publication.source {
            return String::from(source_label(self.lang));
        }
        let mode = publication
            .output
            .as_ref()
            .map(|output| output.mode)
            .unwrap_or(ImagePreviewMode::TransformedDraft);
        self.label_for(mode)
    }

    /// The output size of the last result, when the engine gave one. It is
    /// the intended output, never the size of the display proxy.
    fn output_size(&self) -> Option<(u32, u32)> {
        let shown = self.shown.borrow();
        let output = shown.output.as_ref()?;
        (output.width > 0 && output.height > 0).then_some((output.width, output.height))
    }

    /// The output format of the last result, when the engine gave one.
    fn output_format(&self) -> Option<String> {
        let shown = self.shown.borrow();
        shown.output.as_ref().map(|output| output.format.clone())
    }

    fn field(&self, index: i32) -> Option<&Field> {
        self.action
            .options
            .as_ref()?
            .fields
            .get(usize::try_from(index).ok()?)
    }

    fn value_of(&self, field: &Field, numeric: f32, text: &str) -> Option<toml::Value> {
        Some(match field.kind {
            Kind::Bool => toml::Value::Boolean(numeric > 0.5),
            Kind::Enum => {
                let choice = field.values.get(numeric.max(0.0) as usize)?;
                toml::Value::String(choice.id.clone())
            }
            Kind::String | Kind::PxPair => toml::Value::String(text.to_string()),
            Kind::Float => toml::Value::Float(f64::from(numeric)),
            Kind::Int | Kind::Percent | Kind::Px => toml::Value::Integer(numeric.round() as i64),
        })
    }

    /// The output target of an image resize, when the source is known.
    fn dimension_target(&self, params: &toml::Table) -> Option<image_geom::Target> {
        let source = self.source.get()?;
        let scale = params.get("scale").and_then(toml::Value::as_integer);
        let max_side = params.get("max_side").and_then(toml::Value::as_integer);
        let mode = match (scale, max_side) {
            (Some(scale), _) => Mode::Percent(u32::try_from(scale.max(0)).unwrap_or(u32::MAX)),
            (_, Some(max)) => Mode::LongestSide(u32::try_from(max.max(0)).unwrap_or(u32::MAX)),
            _ => return None,
        };
        image_geom::target(source, mode)
    }

    fn percent_change(&self, width: u32, height: u32) -> i64 {
        let Some((sw, sh)) = self.source.get() else {
            return 0;
        };
        let source = f64::from(sw) * f64::from(sh);
        if source <= 0.0 {
            return 0;
        }
        let target = f64::from(width) * f64::from(height);
        (target / source * 100.0 - 100.0).round() as i64
    }

    fn rebuild(&self) {
        let rows: Vec<StageOption> = match &self.action.options {
            Some(options) => options.fields.iter().map(|field| self.row(field)).collect(),
            None => Vec::new(),
        };
        self.rows.set_vec(rows);
        let presets: Vec<StagePreset> = match &self.action.options {
            Some(options) => options
                .presets
                .iter()
                .map(|preset| StagePreset {
                    id: preset.id.as_str().into(),
                    // A preset carries no label in the schema, so its id is
                    // the label until S-117 grows one.
                    label: preset.id.as_str().into(),
                })
                .collect(),
            None => Vec::new(),
        };
        self.presets.set_vec(presets);
    }

    fn refresh_all(&self) {
        let Some(options) = &self.action.options else {
            return;
        };
        for (index, field) in options.fields.iter().enumerate() {
            self.rows.set_row_data(index, self.row(field));
        }
    }

    fn refresh_row(&self, index: i32, field: &Field) {
        if let Ok(index) = usize::try_from(index) {
            self.rows.set_row_data(index, self.row(field));
        }
    }

    /// The rows of the fields this one clears follow the resolved table.
    fn refresh_cleared(&self, field: &Field) {
        let Some(options) = &self.action.options else {
            return;
        };
        for (index, other) in options.fields.iter().enumerate() {
            if field.clears.iter().any(|key| key == other.param_key()) {
                self.rows.set_row_data(index, self.row(other));
            }
        }
    }

    /// The formatted value of a row: the number with its unit, the selected
    /// choice label, the on or off word, or the text.
    fn display_of(&self, field: &Field, base: Option<&toml::Value>) -> String {
        match field.kind {
            Kind::Bool => {
                let on = matches!(base, Some(toml::Value::Boolean(true)));
                match (self.lang, on) {
                    (Lang::Ar, true) => String::from("مفعّل"),
                    (Lang::Ar, false) => String::from("معطّل"),
                    (Lang::En, true) => String::from("On"),
                    (Lang::En, false) => String::from("Off"),
                }
            }
            Kind::Enum => base
                .and_then(toml::Value::as_str)
                .and_then(|id| field.values.iter().find(|choice| choice.id == id))
                .map(|choice| choice.label.get(self.lang).to_string())
                .unwrap_or_default(),
            Kind::String | Kind::PxPair => base
                .and_then(toml::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            Kind::Int | Kind::Float | Kind::Percent | Kind::Px => {
                let number = number_of(base, field).unwrap_or(0.0);
                let value = if number.fract() == 0.0 {
                    format!("{}", number as i64)
                } else {
                    format!("{number}")
                };
                match field.unit.as_deref() {
                    Some(unit) if !unit.is_empty() => format!("{value} {unit}"),
                    _ => value,
                }
            }
        }
    }

    /// One row from the explicit value, else the base params, else the field
    /// default, else the kind default.
    fn row(&self, field: &Field) -> StageOption {
        let explicit = self.explicit.borrow();
        let base = explicit
            .get(field.param_key())
            .or_else(|| self.action.params.get(field.param_key()))
            .or(field.default.as_ref());
        let kind = match field.kind {
            Kind::Bool => StageKind::Bool,
            Kind::Enum => StageKind::Enum,
            Kind::String | Kind::PxPair => StageKind::Text,
            Kind::Int | Kind::Float | Kind::Percent | Kind::Px => StageKind::Number,
        };
        let choices: Vec<SegmentChoice> = field
            .values
            .iter()
            .map(|choice| SegmentChoice {
                text: choice.label.get(self.lang).into(),
            })
            .collect();
        let selected = match (field.kind, base) {
            (Kind::Enum, Some(toml::Value::String(id))) => field
                .values
                .iter()
                .position(|choice| &choice.id == id)
                .map_or(-1, |index| index as i32),
            (Kind::Enum, _) => 0,
            _ => -1,
        };
        let value = number_of(base, field).unwrap_or(0.0);
        let text = match base {
            Some(toml::Value::String(text)) => text.clone(),
            Some(toml::Value::Integer(n)) => n.to_string(),
            Some(toml::Value::Float(n)) => n.to_string(),
            _ => String::new(),
        };
        let text = self
            .drafts
            .borrow()
            .get(field.param_key())
            .cloned()
            .unwrap_or(text);
        let display = self.display_of(field, base);
        StageOption {
            id: field.id.as_str().into(),
            label: field.label.get(self.lang).into(),
            unit: field.unit.clone().unwrap_or_default().into(),
            display: display.as_str().into(),
            kind,
            value,
            text: text.as_str().into(),
            min: field.min.map(|v| v as f32).unwrap_or(0.0),
            max: field.max.map(|v| v as f32).unwrap_or(100.0),
            step: field.step.map(|v| v as f32).unwrap_or(1.0),
            choices: ModelRc::new(VecModel::from(choices)),
            selected,
        }
    }
}

/// The numeric value of a field: a bool as 0 or 1, an enum index, a number.
fn number_of(base: Option<&toml::Value>, field: &Field) -> Option<f32> {
    let value = base?;
    Some(match (field.kind, value) {
        (Kind::Bool, toml::Value::Boolean(on)) => f32::from(u8::from(*on)),
        (Kind::Enum, toml::Value::String(id)) => {
            field.values.iter().position(|choice| &choice.id == id)? as f32
        }
        (_, toml::Value::Integer(n)) => *n as f32,
        (_, toml::Value::Float(n)) => *n as f32,
        _ => return None,
    })
}

/// A short key: value form for the readout.
fn display(value: &toml::Value) -> String {
    match value {
        toml::Value::String(text) => text.clone(),
        toml::Value::Integer(n) => n.to_string(),
        toml::Value::Float(n) => n.to_string(),
        toml::Value::Boolean(on) => on.to_string(),
        other => other.to_string(),
    }
}

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a.max(1)
}

/// The header dimensions of `path`, when readable.
fn probe(path: &Path) -> Option<(u32, u32)> {
    #[cfg(unix)]
    {
        crate::preview_worker::probe_dimensions(path)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        None
    }
}

/// The preview link of one open panel: the scheduler of the hive, the
/// session, the panel id and the token of the panel's work.
pub(crate) struct PreviewLink {
    /// The scheduler of the hive: one worker for every panel.
    scheduler: Rc<Scheduler>,
    /// The session the panel belongs to.
    session: u64,
    /// The panel id, so a result of another panel is never published here.
    panel: u64,
    /// The token of the panel's current work. A cancelled token is replaced
    /// at once, so a stopped request never stops a later one.
    token: RefCell<CancelToken>,
    /// The stamp of the newest request of this panel.
    stamp: Cell<Option<PreviewStamp>>,
    /// The mode of the newest request of this panel.
    mode: Cell<ImagePreviewMode>,
}

/// What the panel shows for the preview.
#[derive(Clone, Default)]
pub(crate) struct Shown {
    /// The picture of the last result, when there is one.
    image: Option<SharedPixelBuffer<Rgba8Pixel>>,
    /// The label of the shown picture: the mode it belongs to.
    label: String,
    /// The exact length of the encoded artifact, when the picture is one.
    bytes: Option<u64>,
    /// The typed reason of the last failure.
    failure: Option<Failure>,
    /// The facts of the last output.
    output: Option<Output>,
}

/// What one poll of the panel gives to the interface.
pub struct PreviewUpdate {
    /// The picture of the last result. None when the last request failed or
    /// when nothing arrived yet.
    pub image: Option<SharedPixelBuffer<Rgba8Pixel>>,
    /// The label of the shown picture.
    pub label: String,
    /// The exact length of the encoded artifact, when there is one. A draft
    /// has none.
    pub bytes: Option<u64>,
    /// The typed reason of the last failure.
    pub failure: Option<Failure>,
    /// The facts of the last output.
    pub output: Option<Output>,
}

/// The label of the legacy source draft: a picture of the source, never a
/// picture of the result.
fn source_label(lang: Lang) -> &'static str {
    match lang {
        Lang::Ar => "مسودة المصدر",
        Lang::En => "Source draft",
    }
}

/// The typed status of a failed preview, in the window language.
pub fn preview_status(lang: Lang, failure: Failure) -> &'static str {
    match (lang, failure) {
        (Lang::Ar, Failure::Cancelled) => "أُلغيت المعاينة",
        (Lang::Ar, Failure::SourceChanged) => "تغيّر الملف المصدر",
        (Lang::Ar, Failure::PolicyLimit) => "الملف أكبر من حدود المعاينة",
        (Lang::Ar, Failure::Unsupported) => "لا معاينة لهذا الإجراء",
        (Lang::Ar, Failure::Failed) => "فشلت المعاينة",
        (Lang::En, Failure::Cancelled) => "The preview was cancelled",
        (Lang::En, Failure::SourceChanged) => "The source file changed",
        (Lang::En, Failure::PolicyLimit) => "The file is over the preview limits",
        (Lang::En, Failure::Unsupported) => "No preview for this action",
        (Lang::En, Failure::Failed) => "The preview failed",
    }
}

/// The exact length of an encoded artifact, in the window language. A draft
/// never encodes, so it shows no byte count.
pub fn preview_bytes(lang: Lang, bytes: Option<u64>) -> String {
    let Some(bytes) = bytes else {
        return String::new();
    };
    match lang {
        Lang::Ar => format!("\u{2068}{bytes} بايت\u{2069}"),
        Lang::En => format!("\u{2068}{bytes} bytes\u{2069}"),
    }
}

/// The label of the explicit encoded control, in the window language.
pub fn encode_hint(lang: Lang) -> &'static str {
    match lang {
        Lang::Ar => "عرض النتيجة المرمّزة",
        Lang::En => "Show encoded result",
    }
}

/// Text for a named preset that is ready to apply.
pub fn ready_text(lang: Lang) -> String {
    match lang {
        Lang::Ar => String::from("جاهز"),
        Lang::En => String::from("Ready"),
    }
}

/// Text for a named preset whose option schema changed.
pub fn review_text(lang: Lang, _reason: &str) -> String {
    match lang {
        Lang::Ar => String::from("يحتاج مراجعة"),
        Lang::En => String::from("Needs review"),
    }
}

/// Text for a preset whose action is missing, changed or has invalid values.
pub fn unavailable_text(lang: Lang, _reason: &str) -> String {
    match lang {
        Lang::Ar => String::from("غير متاح في السجل الحالي"),
        Lang::En => String::from("Unavailable in the current registry"),
    }
}

/// The detail of the selected row: the marker and the status of the preset,
/// in the window language.
pub fn selected_text(lang: Lang, status: &str) -> String {
    match lang {
        Lang::Ar => format!("محدد، {status}"),
        Lang::En => format!("Selected, {status}"),
    }
}

/// The local confirmation line before an explicit Delete.
pub fn delete_prompt(lang: Lang) -> &'static str {
    match lang {
        Lang::Ar => "اضغط حذف مرة أخرى للتأكيد",
        Lang::En => "Press Delete again to confirm",
    }
}

pub fn saved_text(lang: Lang) -> &'static str {
    match lang {
        Lang::Ar => "تم حفظ الإعداد",
        Lang::En => "Preset saved",
    }
}

pub fn renamed_text(lang: Lang) -> &'static str {
    match lang {
        Lang::Ar => "تم تغيير الاسم",
        Lang::En => "Preset renamed",
    }
}

pub fn deleted_text(lang: Lang) -> &'static str {
    match lang {
        Lang::Ar => "تم حذف الإعداد",
        Lang::En => "Preset deleted",
    }
}

pub fn refreshed_text(lang: Lang) -> &'static str {
    match lang {
        Lang::Ar => "تمت مراجعة الإعداد وحفظه",
        Lang::En => "Preset reviewed and saved",
    }
}

pub fn preset_failure_text(lang: Lang, failure: &PresetFailure) -> String {
    crate::saved_presets::failure_text(lang, failure)
}

fn pending_label(lang: Lang, request: &PresetRequest) -> &'static str {
    match (lang, request) {
        (Lang::Ar, PresetRequest::List { .. }) => "جار تحميل الإعدادات",
        (Lang::Ar, PresetRequest::SaveCurrent { .. }) => "جار حفظ الإعداد",
        (Lang::Ar, PresetRequest::Rename { .. }) => "جار تغيير الاسم",
        (Lang::Ar, PresetRequest::Delete { .. }) => "جار حذف الإعداد",
        (Lang::Ar, PresetRequest::Refresh { .. }) => "جار مراجعة الإعداد",
        (Lang::Ar, PresetRequest::Apply { .. }) => "جار تطبيق الإعداد",
        (Lang::En, PresetRequest::List { .. }) => "Loading presets",
        (Lang::En, PresetRequest::SaveCurrent { .. }) => "Saving preset",
        (Lang::En, PresetRequest::Rename { .. }) => "Renaming preset",
        (Lang::En, PresetRequest::Delete { .. }) => "Deleting preset",
        (Lang::En, PresetRequest::Refresh { .. }) => "Reviewing preset",
        (Lang::En, PresetRequest::Apply { .. }) => "Applying preset",
    }
}
