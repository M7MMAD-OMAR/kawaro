//! Candidate-only production options capture. No baseline writes or job submission.
//! options_snapshot OUT.png --action image.resize-50 --lang en --frame initial
//! Frames: initial, focused, invalid, bottom. Crop numeric-invalid is N/A.
//! Input uses Slint WindowEvents, not OS keyboard injection or geometry setters.
use i_slint_backend_testing::{ElementHandle, ElementRoot};
use kawaro_core::action::Registry;
use kawaro_ui::{Hive, HiveCell, Lang, stage_two::StageTwoState};
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, Model};
use std::{cell::RefCell, path::PathBuf, rc::Rc, time::Duration};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn find_named(ui: &kawaro_ui::HiveWindow, id: &str) -> Option<ElementHandle> {
    let id = id.to_owned();
    let expected = id.clone();
    ui.root_element()
        .query_descendants()
        .match_predicate(move |e| {
            e.id()
                .is_some_and(|s| s == expected.as_str() || s.ends_with(&format!("::{expected}")))
        })
        .find_first()
}
fn named(ui: &kawaro_ui::HiveWindow, id: &str) -> ElementHandle {
    find_named(ui, id).unwrap_or_else(|| panic!("missing actual element {id}"))
}
fn centre(e: &ElementHandle) -> LogicalPosition {
    let p = e.absolute_position();
    let s = e.size();
    LogicalPosition::new(p.x + s.width / 2., p.y + s.height / 2.)
}
fn key(ui: &kawaro_ui::HiveWindow, text: slint::SharedString) {
    ui.window()
        .dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    ui.window()
        .dispatch_event(WindowEvent::KeyReleased { text });
}
fn wheel(ui: &kawaro_ui::HiveWindow, delta_y: f32) {
    let position = centre(&named(ui, "options"));
    ui.window()
        .dispatch_event(WindowEvent::PointerMoved { position });
    ui.window().dispatch_event(WindowEvent::PointerScrolled {
        position,
        delta_x: 0.,
        delta_y,
    });
}
fn prepare(hive: &Hive, frame: &str, crop: bool) -> Result<()> {
    let ui = hive.testing_window();
    if frame == "bottom" {
        wheel(&ui, -100_000.);
    } else if frame == "focused" || frame == "invalid" {
        let kind = if crop { "RadioButton" } else { "TextField" };
        let field = ui
            .root_element()
            .query_descendants()
            .match_inherits(kind)
            .find_first()
            .ok_or("missing actual Material control")?;
        // Scroll the real body until this control is inside its viewport.
        let viewport = named(&ui, "options");
        let p = centre(&field);
        let v = viewport.absolute_position();
        if p.y < v.y || p.y > v.y + viewport.size().height {
            wheel(&ui, centre(&viewport).y - p.y);
        }
        let position = centre(&field);
        ui.window()
            .dispatch_event(WindowEvent::PointerMoved { position });
        ui.window().dispatch_event(WindowEvent::PointerPressed {
            position,
            button: PointerEventButton::Left,
        });
        ui.window().dispatch_event(WindowEvent::PointerReleased {
            position,
            button: PointerEventButton::Left,
        });
        if frame == "invalid" {
            ui.window().dispatch_event(WindowEvent::KeyPressed {
                text: Key::Control.into(),
            });
            key(&ui, "a".into());
            ui.window().dispatch_event(WindowEvent::KeyReleased {
                text: Key::Control.into(),
            });
            key(&ui, Key::Backspace.into());
            key(&ui, "invalid".into());
            let rows = ui.get_stage_options();
            assert_eq!(
                rows.iter().filter(|r| r.text == "invalid").count(),
                1,
                "exactly one raw typed invalid draft required"
            );
            assert!(
                !ui.get_stage_error().is_empty(),
                "typed draft must show a real error"
            );
            println!("typed invalid, error={}", ui.get_stage_error());
        }
    }
    Ok(())
}
fn capture(hive: &Hive, out: &PathBuf, compare: Option<&PathBuf>, frame: &str) -> Result<()> {
    let ui = hive.testing_window();
    let buf = ui.window().take_snapshot()?;
    let (w, h) = (buf.width(), buf.height());
    // Preserve the entire rendered window. Never crop, scale or pad it to pass.
    assert_eq!((w, h), (393, 362), "unexpected production surface size");
    let logical = ui.window().size().to_logical(ui.window().scale_factor());
    assert!((logical.width - 393.).abs() < 0.5 && (logical.height - 362.).abs() < 0.5);
    let footer = named(
        &ui,
        if ui.get_rtl() {
            "stage-footer-ar"
        } else {
            "stage-footer-en"
        },
    );
    let (p, s) = (footer.absolute_position(), footer.size());
    assert!(
        p.x >= 0. && p.y >= 0. && p.x + s.width <= 393.5 && p.y + s.height <= 362.5,
        "footer must be in the full frame"
    );
    let viewport = named(&ui, "options");
    let bottom = viewport.absolute_position().y + viewport.size().height;
    if frame == "bottom" {
        let body = named(&ui, "body");
        assert!(
            body.absolute_position().y + body.size().height <= bottom + 1.,
            "native wheel must reach the actual body bottom"
        );
    } else if frame == "invalid" {
        assert_eq!(
            ui.get_stage_options()
                .iter()
                .filter(|r| r.text == "invalid")
                .count(),
            1,
            "raw typed invalid draft must survive until capture"
        );
        assert!(!ui.get_stage_error().is_empty(), "typed error must persist");
        let error = named(&ui, "stage-error-text");
        assert!(
            error.absolute_position().y >= viewport.absolute_position().y - 1.
                && error.absolute_position().y + error.size().height <= bottom + 1.,
            "actual typed error must be visible"
        );
    }
    for (name, rect, glyphs) in hive.stage_geometry() {
        println!("geometry {name}: {rect:?}, glyphs={glyphs:?}");
    }
    let file = std::fs::File::create_new(out)?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()?.write_image_data(buf.as_bytes())?;
    println!("wrote {} ({w}x{h})", out.display());
    if let Some(reference) = compare {
        let mut reader = png::Decoder::new(std::fs::File::open(reference)?).read_info()?;
        let mut rgba = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut rgba)?;
        assert_eq!((info.width, info.height), (w, h));
        assert_eq!(info.color_type, png::ColorType::Rgba);
        assert_eq!(info.bit_depth, png::BitDepth::Eight);
        let bad = buf
            .as_bytes()
            .as_chunks::<4>()
            .0
            .iter()
            .zip(rgba[..info.buffer_size()].as_chunks::<4>().0.iter())
            .filter(|(a, b)| a.iter().zip(*b).any(|(x, y)| x.abs_diff(*y) > 8))
            .count();
        println!("diff {bad} pixels against {}", reference.display());
        if bad as f64 / (w as f64 * h as f64) > 0.001 {
            return Err("snapshot differs, candidate retained".into());
        }
    }
    Ok(())
}
fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(args.next().ok_or("expected OUT.png")?);
    let (mut action, mut lang, mut frame, mut compare) = (
        "image.resize-50".to_string(),
        Lang::En,
        "initial".to_string(),
        None,
    );
    let mut fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/images/kawaro-sample.png");
    while let Some(arg) = args.next() {
        let value = args.next().ok_or("option needs a value")?;
        match arg.as_str() {
            "--action" => action = value,
            "--lang" if value == "en" || value == "ar" => lang = Lang::parse(&value),
            "--frame" => frame = value,
            "--compare" => compare = Some(PathBuf::from(value)),
            "--file" => fixture = PathBuf::from(value),
            _ => return Err(format!("unknown option {arg}").into()),
        }
    }
    assert!(
        [
            "image.resize-50",
            "image.resize-to",
            "image.crop",
            "image.compress-to"
        ]
        .contains(&action.as_str())
    );
    assert!(["initial", "focused", "invalid", "bottom"].contains(&frame.as_str()));
    assert!(
        action != "image.crop" || frame != "invalid",
        "Crop numeric-invalid is N/A"
    );
    assert!(fixture.is_file(), "source fixture must exist");
    kawaro_ui::init()?;
    let hive = Rc::new(Hive::new(lang)?);
    let registry = Registry::builtin();
    let definition = registry.get(&action).ok_or("unknown builtin action")?;
    let state = StageTwoState::build(definition, &[fixture], lang, lang == Lang::Ar)
        .ok_or("action has no options")?;
    let cells = vec![HiveCell {
        id: action.clone(),
        code: action.clone(),
        label: action.clone(),
        icon: "resize".into(),
        ring: 2,
        enabled: true,
        reason: String::new(),
        customize: true,
    }];
    hive.present_options(cells, Rc::new(state))?;
    let result = Rc::new(RefCell::new(Ok::<(), String>(())));
    let (h, r) = (hive.clone(), result.clone());
    slint::Timer::single_shot(Duration::from_millis(900), move || {
        println!("before-input geometry {:?}", h.stage_geometry());
        if let Err(e) = prepare(&h, &frame, action == "image.crop") {
            *r.borrow_mut() = Err(e.to_string());
            slint::quit_event_loop().ok();
            return;
        }
        // Conditional Slint items materialize after input, on the event loop.
        slint::Timer::single_shot(Duration::from_millis(100), move || {
            if frame == "invalid" {
                let ui = h.testing_window();
                // ElementQuery omits clipped items. Use real wheel input first,
                // so the error below the controls can be found in the live tree.
                let error = (0..32).find_map(|_| {
                    let error = find_named(&ui, "stage-error-text");
                    if error.is_none() {
                        wheel(&ui, -named(&ui, "options").size().height / 2.);
                    }
                    error
                });
                let Some(error) = error else {
                    *r.borrow_mut() = Err("missing actual typed error after event turn".into());
                    slint::quit_event_loop().ok();
                    return;
                };
                // Real wheel input only, after the conditional error has settled.
                wheel(&ui, centre(&named(&ui, "options")).y - centre(&error).y);
            }
            slint::Timer::single_shot(Duration::from_millis(400), move || {
                *r.borrow_mut() =
                    capture(&h, &out, compare.as_ref(), &frame).map_err(|e| e.to_string());
                slint::quit_event_loop().ok();
            });
        });
    });
    slint::run_event_loop_until_quit()?;
    hive.close();
    let final_result = result.borrow_mut().clone();
    final_result.map_err(Into::into)
}
