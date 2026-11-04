//! S-119 stage 2: the option panel model and its live preview.
//!
//! The panel is built from the action's [action.options] schema (S-117),
//! never from a hand written list per action: one row per field, one control
//! per field kind. Values resolve through `resolve_params`, the same core
//! function the CLI and MCP use, so the readout, the preview and the job all
//! agree.
//!
//! The preview decodes one draft on a worker thread; a generation counter
//! makes the newest request win and only one decode runs at a time. Nothing
//! here touches the hive or starts before the user opens the panel.

use crate::{SegmentChoice, StageKind, StageOption, StagePreset};
use kawaro_core::action::ActionDef;
use kawaro_core::i18n::Lang;
use kawaro_core::image_geom::{self, Mode};
use kawaro_core::media;
use kawaro_core::options::{Field, Kind, OptionsError};
use slint::{Image, Model as _, ModelRc, Rgba8Pixel, SharedPixelBuffer, VecModel};
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

/// Longest side of a panel draft, in pixels.
pub const MAX_DRAFT: u32 = 512;

/// The three footer labels of the panel, in the window language.
pub fn buttons(lang: Lang) -> (&'static str, &'static str, &'static str) {
    match lang {
        Lang::Ar => ("تشغيل", "إعادة الضبط", "رجوع"),
        Lang::En => ("Run", "Reset", "Back"),
    }
}

/// The hint that opens the panel, in the window language.
pub fn customize_hint(lang: Lang) -> &'static str {
    match lang {
        Lang::Ar => "تخصيص",
        Lang::En => "Customize",
    }
}

/// The state behind one open stage 2 panel.
pub struct StageTwoState {
    action: ActionDef,
    /// The values the user set, by param key. Empty means the base params.
    explicit: RefCell<toml::Table>,
    rows: Rc<VecModel<StageOption>>,
    presets: Rc<VecModel<StagePreset>>,
    /// Header dimensions of the first file, when it is an image.
    source: Cell<Option<(u32, u32)>>,
    /// True when the first file is an image, for the preview note.
    image: bool,
    path: PathBuf,
    lang: Lang,
    rtl: bool,
    preview: Preview,
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
            presets: Rc::new(VecModel::default()),
            source: Cell::new(source),
            image,
            path,
            lang,
            rtl,
            preview: Preview::new(),
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
        let field = field.clone();
        self.refresh_row(index, &field);
        // A cleared sibling changes the resolved table, so refresh every row
        // that the field names.
        self.refresh_cleared(&field);
        self.request_preview();
    }

    /// A preset was picked: its values name field ids, so they go into the
    /// explicit table under each field's param key.
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
                for clear in &field.clears {
                    explicit.remove(clear);
                }
                explicit.insert(field.param_key().to_string(), value.clone());
            }
        }
        self.refresh_all();
        self.request_preview();
    }

    /// Reset returns every control to the base [action.params] value.
    pub fn reset(&self) {
        self.explicit.borrow_mut().clear();
        self.refresh_all();
        self.request_preview();
    }

    /// The params a Run submits: the base table with the explicit values.
    pub fn resolved(&self) -> Result<toml::Table, OptionsError> {
        match &self.action.options {
            Some(options) => options.resolve_params(&self.action.params, &self.explicit.borrow()),
            None => Ok(self.action.params.clone()),
        }
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
    pub fn readout(&self) -> String {
        let params = self
            .resolved()
            .unwrap_or_else(|_| self.action.params.clone());
        let target = self.dimension_target(&params);
        let mut parts = Vec::new();
        if let Some(options) = &self.action.options {
            for token in &options.readout {
                match token.as_str() {
                    "dimensions" => {
                        if let Some(target) = target {
                            parts.push(format!("{} x {}", target.width, target.height));
                        }
                    }
                    "ratio" => {
                        if let Some(target) = target {
                            let divisor = gcd(target.width, target.height);
                            parts.push(format!(
                                "{}:{}",
                                target.width / divisor,
                                target.height / divisor
                            ));
                        }
                    }
                    "percent" => {
                        if let Some(target) = target {
                            parts.push(format!(
                                "{:+}%",
                                self.percent_change(target.width, target.height)
                            ));
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
        let extension = if self.action.output.ext.is_empty() {
            media::extension(&self.path)
        } else {
            self.action.output.ext.clone()
        };
        let mut out = format!("{stem}{}", self.action.output.suffix);
        if !extension.is_empty() {
            out.push('.');
            out.push_str(&extension);
        }
        Some(out)
    }

    /// Start (or refresh) the preview of the first file.
    pub fn request_preview(&self) {
        if self.preview_available() {
            self.preview.request(self.path.clone());
        }
    }

    /// The newest finished draft, ready to publish; None while nothing new.
    pub fn poll_preview(&self) -> Option<Image> {
        self.preview.poll().map(Image::from_rgba8)
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
            _ => String::new(),
        };
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

/// One masked draft, decoded on the worker thread.
fn render(path: &Path) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
    #[cfg(unix)]
    {
        let mut buffer = crate::preview_worker::decode_draft(path, MAX_DRAFT)?;
        let circumradius = (buffer.width() as f32 / 1.732_050_8).min(buffer.height() as f32 / 2.0);
        crate::preview_worker::mask_hexagon(&mut buffer, circumradius)?;
        Some(buffer)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        None
    }
}

/// The newest-wins preview decode: one render at a time, a newer request
/// replaces an older one, and only the newest result is published.
struct Preview {
    shared: Arc<Shared>,
}

struct Shared {
    /// (a render runs, the generation the UI wants).
    state: Mutex<(bool, u64)>,
    sender: Sender<(u64, Option<SharedPixelBuffer<Rgba8Pixel>>)>,
    receiver: Mutex<Receiver<(u64, Option<SharedPixelBuffer<Rgba8Pixel>>)>>,
}

impl Preview {
    fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Preview {
            shared: Arc::new(Shared {
                state: Mutex::new((false, 0)),
                sender,
                receiver: Mutex::new(receiver),
            }),
        }
    }

    fn request(&self, path: PathBuf) {
        let spawn = {
            let mut state = lock(&self.shared.state);
            state.1 += 1;
            if state.0 {
                false
            } else {
                state.0 = true;
                true
            }
        };
        if !spawn {
            return;
        }
        let shared = self.shared.clone();
        let started = std::thread::Builder::new()
            .name(String::from("kawaro-preview"))
            .spawn(move || {
                loop {
                    let generation = lock(&shared.state).1;
                    let image = render(&path);
                    let mut state = lock(&shared.state);
                    if state.1 == generation {
                        let _ = shared.sender.send((generation, image));
                        state.0 = false;
                        return;
                    }
                    // A newer value arrived while this render ran: run again.
                }
            });
        if started.is_err() {
            // No thread: the panel shows the dimensions and no picture.
            lock(&self.shared.state).0 = false;
        }
    }

    /// The newest finished draft, if one arrived since the last poll.
    fn poll(&self) -> Option<SharedPixelBuffer<Rgba8Pixel>> {
        let receiver = lock(&self.shared.receiver);
        let mut newest = None;
        while let Ok((_, image)) = receiver.try_recv() {
            newest = newest.or(image);
        }
        newest
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}
