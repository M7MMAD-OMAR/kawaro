//! S-119 stage 1: the stage 2 model, built from the action option schema.
//!
//! No window is created here: the rows, the readout and the resolved params
//! are pure data, and the preview thread is asked but never awaited, so a
//! failing decoder stays a soft failure.

use kawaro_core::action::{ActionDef, Localized, OutputSpec};
use kawaro_core::action::{OutputMode, Registry};
use kawaro_core::i18n::Lang;
use kawaro_core::media::MediaKind;
use kawaro_core::options::{Field, Kind, OptionsKind, OptionsSpec, Preset};
use kawaro_ui::stage_two::{StageTwoState, buttons, customize_hint};
use slint::Model;
use std::path::{Path, PathBuf};

/// A tiny JPEG whose SOF dimensions are patched, so the header probe reports
/// the wanted size without the test writing 36 MB of pixels.
fn fake_jpeg(path: &Path, width: u32, height: u32) {
    let mut bytes = Vec::new();
    {
        let image = image::RgbImage::from_fn(32, 32, |x, y| image::Rgb([x as u8, y as u8, 9]));
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 50);
        encoder
            .encode_image(&image::DynamicImage::ImageRgb8(image))
            .unwrap();
    }
    let sof = bytes
        .windows(2)
        .position(|w| (w[0] == 0xFF && w[1] == 0xC0) || (w[0] == 0xFF && w[1] == 0xC2))
        .expect("a JPEG has a start of frame marker");
    bytes[sof + 5..sof + 7].copy_from_slice(&(height as u16).to_be_bytes());
    bytes[sof + 7..sof + 9].copy_from_slice(&(width as u16).to_be_bytes());
    std::fs::write(path, bytes).unwrap();
}

fn builtin(id: &str) -> ActionDef {
    Registry::builtin()
        .get(id)
        .unwrap_or_else(|| panic!("missing built-in action {id}"))
        .clone()
}

fn text(text: &str) -> Localized {
    Localized {
        ar: text.into(),
        en: text.into(),
    }
}

/// A small action with one integer field and one preset, for the preset path
/// (no built-in declares presets yet).
fn preset_action() -> ActionDef {
    ActionDef {
        id: "x.preset".into(),
        engine: "image".into(),
        op: "resize".into(),
        ring: 2,
        order: 0,
        accepts: vec![MediaKind::Image],
        skip_ext: vec![],
        min_files: 1,
        max_files: 0,
        output: OutputSpec {
            mode: OutputMode::PerFile,
            ext: String::new(),
            suffix: String::new(),
            dir: String::new(),
        },
        code: text("PRESET"),
        label: text("Preset"),
        icon: String::new(),
        params: {
            let mut params = toml::Table::new();
            params.insert("quality".into(), 85.into());
            params
        },
        options: Some(OptionsSpec {
            kind: OptionsKind::Form,
            readout: vec![],
            remember: false,
            suffix_template: String::new(),
            fields: vec![Field {
                id: "quality".into(),
                kind: Kind::Int,
                label: text("Quality"),
                default: Some(85.into()),
                min: Some(1.0),
                max: Some(100.0),
                step: None,
                unit: Some("%".into()),
                values: vec![],
                maps_to: Some("quality".into()),
                clears: vec![],
                no_enlarge: false,
            }],
            presets: vec![Preset {
                id: "low".into(),
                values: {
                    let mut values = toml::Table::new();
                    values.insert("quality".into(), 40.into());
                    values
                },
            }],
        }),
    }
}

#[test]
fn resize_builds_number_rows_from_the_base_params() {
    let dir = tempfile::tempdir().unwrap();
    let photo = dir.path().join("photo.jpg");
    fake_jpeg(&photo, 4000, 3000);

    let state = StageTwoState::build(
        &builtin("image.resize-50"),
        std::slice::from_ref(&photo),
        Lang::En,
        false,
    )
    .expect("image.resize-50 has options");

    let rows = state.rows();
    assert_eq!(rows.row_count(), 3, "one row per field");
    let scale = rows.row_data(0).unwrap();
    assert_eq!(scale.id.as_str(), "scale");
    assert_eq!(scale.kind, kawaro_ui::StageKind::Number);
    assert_eq!(scale.value, 50.0, "the base params carry the value");
    assert_eq!(scale.min, 1.0);
    assert_eq!(scale.max, 1000.0);
    // The schema leaves the scale unit empty (the value is a percent anyway);
    // the quality fields of the lossy actions carry "%".
    assert_eq!(scale.unit.as_str(), "");
    let max_side = rows.row_data(1).unwrap();
    assert_eq!(max_side.id.as_str(), "max_side");
    assert_eq!(max_side.value, 1920.0);
    assert_eq!(max_side.min, 16.0);
    let keep = rows.row_data(2).unwrap();
    assert_eq!(keep.id.as_str(), "keep_metadata");
    assert_eq!(keep.kind, kawaro_ui::StageKind::Bool);
    assert_eq!(keep.value, 0.0, "the default is false");
    assert_eq!(state.presets().row_count(), 0);
    assert!(state.preview_available(), "the dimensions are known");
    assert_eq!(state.title(), "Resize to 50%");
    assert_eq!(state.action_id(), "image.resize-50");

    let resolved = state.resolved().unwrap();
    assert_eq!(resolved, builtin("image.resize-50").params);
}

#[test]
fn readout_uses_the_shared_image_geometry() {
    let dir = tempfile::tempdir().unwrap();
    let photo = dir.path().join("photo.jpg");
    fake_jpeg(&photo, 4000, 3000);
    let state = StageTwoState::build(
        &builtin("image.resize-50"),
        std::slice::from_ref(&photo),
        Lang::En,
        false,
    )
    .unwrap();
    // The action's readout tokens: dimensions, ratio, percent and the output
    // name, wrapped in the bidi isolates U+2068 and U+2069.
    assert_eq!(
        state.readout(),
        "\u{2068}2000 x 1500, 4:3, -75%, photo-50.jpg\u{2069}"
    );
}

#[test]
fn setting_max_side_clears_scale_and_changes_the_readout() {
    let dir = tempfile::tempdir().unwrap();
    let photo = dir.path().join("photo.jpg");
    fake_jpeg(&photo, 4000, 3000);
    let state = StageTwoState::build(
        &builtin("image.resize-50"),
        std::slice::from_ref(&photo),
        Lang::En,
        false,
    )
    .unwrap();

    state.set_value(1, 1920.0, "");
    let resolved = state.resolved().unwrap();
    assert_eq!(
        resolved.get("max_side").and_then(toml::Value::as_integer),
        Some(1920)
    );
    assert!(
        !resolved.contains_key("scale"),
        "scale is cleared: {resolved:?}"
    );
    // 4000 x 3000 fit to a 1920 long side, 4:3, area fell by 77 percent.
    assert_eq!(
        state.readout(),
        "\u{2068}1920 x 1440, 4:3, -77%, photo-50.jpg\u{2069}"
    );

    // Back to scale: max_side is cleared the other way.
    state.set_value(0, 25.0, "");
    let resolved = state.resolved().unwrap();
    assert_eq!(
        resolved.get("scale").and_then(toml::Value::as_integer),
        Some(25)
    );
    assert!(!resolved.contains_key("max_side"), "{resolved:?}");
}

#[test]
fn enum_rows_carry_the_choices_and_resolve() {
    let pdf = PathBuf::from("/tmp/example.pdf");
    let state = StageTwoState::build(
        &builtin("pdf.compress"),
        std::slice::from_ref(&pdf),
        Lang::En,
        false,
    )
    .expect("pdf.compress has options");
    let rows = state.rows();
    assert_eq!(rows.row_count(), 2, "preset and dpi");
    let preset = rows.row_data(0).unwrap();
    assert_eq!(preset.id.as_str(), "preset");
    assert_eq!(preset.kind, kawaro_ui::StageKind::Enum);
    assert_eq!(preset.selected, 1, "the base params say ebook");
    let choices: Vec<String> = (0..preset.choices.row_count())
        .map(|index| preset.choices.row_data(index).unwrap().text.to_string())
        .collect();
    assert_eq!(choices, ["Screen", "Ebook", "Printer", "Prepress"]);
    assert!(!state.preview_available(), "a PDF has no picture");
    assert_eq!(state.readout(), "\u{2068}dpi: 0, preset: ebook\u{2069}");

    state.set_value(0, 2.0, "");
    assert_eq!(
        state
            .resolved()
            .unwrap()
            .get("preset")
            .and_then(toml::Value::as_str),
        Some("printer")
    );
    assert_eq!(state.readout(), "\u{2068}dpi: 0, preset: printer\u{2069}");
    assert_eq!(state.rows().row_data(0).unwrap().selected, 2);
}

#[test]
fn bool_and_text_rows_resolve_and_respect_clears() {
    let pdf = PathBuf::from("/tmp/example.pdf");
    let state = StageTwoState::build(
        &builtin("pdf.metadata"),
        std::slice::from_ref(&pdf),
        Lang::En,
        false,
    )
    .expect("pdf.metadata has options");
    let rows = state.rows();
    assert_eq!(rows.row_count(), 5);
    assert_eq!(rows.row_data(0).unwrap().kind, kawaro_ui::StageKind::Bool);
    assert_eq!(rows.row_data(1).unwrap().kind, kawaro_ui::StageKind::Text);

    // Clear metadata: the base params false becomes true.
    state.set_value(0, 1.0, "");
    assert!(
        state
            .resolved()
            .unwrap()
            .get("clear")
            .and_then(toml::Value::as_bool)
            .unwrap()
    );

    // A title clears the clear switch, by the schema's clears list.
    state.set_value(1, 0.0, "A title");
    let resolved = state.resolved().unwrap();
    assert_eq!(
        resolved.get("title").and_then(toml::Value::as_str),
        Some("A title")
    );
    assert!(!resolved.contains_key("clear"), "{resolved:?}");
    assert!(state.readout().contains("title: A title"));
}

#[test]
fn presets_apply_their_values() {
    let dir = tempfile::tempdir().unwrap();
    let photo = dir.path().join("photo.jpg");
    fake_jpeg(&photo, 400, 300);
    let action = preset_action();
    let state = StageTwoState::build(&action, std::slice::from_ref(&photo), Lang::Ar, true)
        .expect("the preset action has options");
    assert_eq!(state.presets().row_count(), 1);
    assert_eq!(state.presets().row_data(0).unwrap().id.as_str(), "low");
    assert_eq!(state.rows().row_data(0).unwrap().value, 85.0);

    state.apply_preset(0);
    assert_eq!(
        state
            .resolved()
            .unwrap()
            .get("quality")
            .and_then(toml::Value::as_integer),
        Some(40)
    );
    assert_eq!(state.rows().row_data(0).unwrap().value, 40.0);

    // Reset returns to the base params.
    state.reset();
    assert_eq!(
        state
            .resolved()
            .unwrap()
            .get("quality")
            .and_then(toml::Value::as_integer),
        Some(85)
    );
}

#[test]
fn an_action_without_options_has_no_panel() {
    assert!(StageTwoState::build(&builtin("image.rotate-90"), &[], Lang::En, false).is_none());
}

#[test]
fn a_missing_picture_is_a_soft_failure() {
    let missing = PathBuf::from("/tmp/kawaro-s119-missing-photo.jpg");
    let state = StageTwoState::build(
        &builtin("image.to-webp"),
        std::slice::from_ref(&missing),
        Lang::En,
        false,
    )
    .expect("image.to-webp has options");
    assert!(!state.preview_available(), "no dimensions, no picture");
    state.request_preview();
    // Nothing to publish and no panic while the worker starts.
    assert!(!state.preview_available());
    // The readout falls back to the prompt params rather than inventing a size.
    assert!(
        state.readout().contains("quality: 85"),
        "{}",
        state.readout()
    );
}

#[test]
fn an_image_source_without_dimensions_says_the_preview_is_not_available() {
    let missing = PathBuf::from("/tmp/kawaro-s119-no-dimensions.jpg");
    let state = StageTwoState::build(
        &builtin("image.to-webp"),
        std::slice::from_ref(&missing),
        Lang::En,
        false,
    )
    .expect("image.to-webp has options");
    assert_eq!(state.preview_note(), Some("Preview not available"));
    assert!(!state.preview_available());
}

#[test]
fn the_two_languages_carry_their_own_strings() {
    let (run, reset, back) = buttons(Lang::En);
    assert_eq!((run, reset, back), ("Run", "Reset", "Back"));
    let (run, reset, back) = buttons(Lang::Ar);
    for word in [run, reset, back, customize_hint(Lang::Ar)] {
        assert!(!word.is_empty());
        assert!(
            word.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c)),
            "{word} must be Arabic"
        );
        assert!(
            !word.chars().any(|c| ('\u{0660}'..='\u{0669}').contains(&c)),
            "{word} must not use Arabic-Indic digits"
        );
    }
    assert_eq!(customize_hint(Lang::En), "Customize");
}
/// S-119 stage 5: the panel's callbacks in a headless Slint window. The
/// controls are wired to these callbacks, so firing them is what a Run, a
/// value change and a preset do.
#[test]
fn the_panel_callbacks_fire_in_a_headless_window() {
    i_slint_backend_testing::init_no_event_loop();
    let ui = kawaro_ui::HiveWindow::new().expect("a hive window");
    let runs = std::rc::Rc::new(std::cell::Cell::new(0));
    let changed = std::rc::Rc::new(std::cell::RefCell::new((0, 0.0, String::new())));
    ui.on_stage_run({
        let runs = runs.clone();
        move || runs.set(runs.get() + 1)
    });
    ui.on_option_changed({
        let changed = changed.clone();
        move |index, numeric, text| {
            *changed.borrow_mut() = (index, numeric, text.to_string());
        }
    });

    let choice = |text: &str| kawaro_ui::SegmentChoice { text: text.into() };
    ui.set_stage_options(slint::ModelRc::new(slint::VecModel::from(vec![
        kawaro_ui::StageOption {
            id: "scale".into(),
            label: "Scale".into(),
            unit: "%".into(),
            display: "50 %".into(),
            kind: kawaro_ui::StageKind::Number,
            value: 50.0,
            text: "".into(),
            min: 1.0,
            max: 1000.0,
            step: 1.0,
            choices: slint::ModelRc::new(slint::VecModel::from(vec![choice("a"), choice("b")])),
            selected: -1,
        },
    ])));
    ui.set_stage_two(true);
    assert!(ui.get_stage_two(), "the panel is open");
    assert_eq!(ui.get_stage_options().row_count(), 1);

    ui.invoke_stage_run();
    ui.invoke_option_changed(0, 60.0, "x".into());
    assert_eq!(runs.get(), 1, "Run fires once");
    assert_eq!(
        *changed.borrow(),
        (0, 60.0, String::from("x")),
        "option-changed carries the row, the number and the text"
    );
}

#[test]
fn an_out_of_range_option_surfaces_in_the_stage_error_text() {
    i_slint_backend_testing::init_no_event_loop();
    let dir = tempfile::tempdir().unwrap();
    let photo = dir.path().join("photo.jpg");
    fake_jpeg(&photo, 4000, 3000);
    let state = StageTwoState::build(
        &builtin("image.resize-50"),
        std::slice::from_ref(&photo),
        Lang::En,
        false,
    )
    .expect("image.resize-50 has options");

    let hive = kawaro_ui::Hive::new(Lang::En).expect("a hive window");
    let cells = vec![kawaro_ui::HiveCell {
        id: "image.resize-50".into(),
        code: "50%".into(),
        label: "Resize".into(),
        icon: "resize".into(),
        ring: 2,
        enabled: true,
        reason: String::new(),
        customize: true,
    }];
    hive.present_options(cells, std::rc::Rc::new(state))
        .expect("the panel shows");
    assert_eq!(hive.stage_error(), "", "no error before a bad value");

    // max_side's row accepts 20001, the shared resolver refuses it against
    // the schema maximum, and the wired panel writes the resolver text into
    // the stage-error line.
    hive.set_option(1, 20001.0, "");
    let error = hive.stage_error();
    assert!(error.contains("max_side"), "{error}");
    assert!(error.contains("above"), "{error}");

    // A valid value clears the line again.
    hive.set_option(1, 1920.0, "");
    assert_eq!(hive.stage_error(), "", "a valid value clears the error");
}
