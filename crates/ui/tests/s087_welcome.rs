//! S-087 (backlog/S-087-first-run-welcome.md): the welcome window's assets
//! and static rules. AC2's frame set, budget and reduced motion, AC4's
//! bundled sample, AC5's forbidden words and AC6's static snapshot checks.
//! The rendered snapshots are compared in the private compositor by
//! `crates/ui/tests/welcome_snapshots.sh`; CI checks only the static rows.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    let path = root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} must exist: {e}", path.display()))
}

// ------------------------------------------------------------ AC2

// S-094c replaced the raster frame set by a live vector film, so the old
// frame set, budget and size tests became these.

#[test]
fn ac2_the_film_is_a_live_scene_and_embeds_no_frame_images() {
    let welcome = read("crates/ui/ui/welcome.slint");
    assert!(
        !welcome.contains("resources/welcome/frame-"),
        "welcome.slint must not embed raster frames"
    );
    let film = read("crates/ui/ui/film.slint");
    assert!(
        !film.contains("@image-url"),
        "the film scene is vectors, not images"
    );
    let sample = root().join("tests/fixtures/images/welcome-sample.png");
    let size = std::fs::metadata(&sample)
        .expect("the sample image exists")
        .len();
    assert!(
        size <= 2 * 1024 * 1024,
        "the embedded sample is {size} bytes, over the 2 MB of AC2"
    );
}

#[test]
fn ac2_reduced_motion_shows_one_frame_and_still_steps_the_captions() {
    use kawaro_ui::welcome::demo_plan;
    let moving = demo_plan(16, false);
    assert_eq!(moving.frames, 16, "the full set loops");
    assert!(
        moving.frame_interval_ms > 0,
        "the loop has a frame interval"
    );
    assert_eq!(moving.caption_steps, 5, "five caption steps");

    let still = demo_plan(16, true);
    assert_eq!(still.frames, 1, "reduced motion shows one static frame");
    assert_eq!(still.frame_interval_ms, 0, "and never loops");
    assert_eq!(still.caption_steps, 5, "the caption steps stay");
    assert!(still.caption_interval_ms > 0, "the captions still step");
}

#[test]
fn ac2_no_frame_asset_is_committed_without_a_generator() {
    let dir = root().join("crates/ui/resources/welcome");
    let frames: Vec<_> = std::fs::read_dir(&dir)
        .expect("resources/welcome")
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("frame-"))
        .collect();
    assert!(
        frames.is_empty(),
        "raster frames need a generator and a 1680x945 size; there are none now"
    );
    assert!(
        !root().join("scripts/gen-welcome-frames.sh").exists(),
        "the frame generator was removed with the frames"
    );
}

// ------------------------------------------------------------ AC4

#[test]
fn ac4_the_sample_installs_into_a_writable_folder() {
    let dir = tempfile::tempdir().unwrap();
    let path = kawaro_ui::welcome::install_sample(dir.path()).expect("the sample installs");
    assert_eq!(path, dir.path().join("sample.png"));
    assert_eq!(
        std::fs::read(&path).unwrap(),
        kawaro_ui::welcome::sample_bytes(),
        "the installed file is the bundled sample"
    );
    let decoder = png::Decoder::new(std::fs::File::open(&path).unwrap());
    decoder.read_info().expect("the sample is a real PNG");
    // A second click writes the same untouched copy again.
    let again = kawaro_ui::welcome::install_sample(dir.path()).unwrap();
    assert_eq!(again, path);
    assert_eq!(
        std::fs::read(&path).unwrap(),
        kawaro_ui::welcome::sample_bytes(),
        "the sample is rewritten from the bundle, never from disk"
    );
}

// ------------------------------------------------------------ AC5

#[test]
fn ac5_no_forbidden_word_in_a_welcome_slint_string() {
    let src = read("crates/ui/ui/welcome.slint");
    let mut rest = src.as_str();
    while let Some(i) = rest.find('"') {
        let tail = &rest[i + 1..];
        let Some(end) = tail.find('"') else { break };
        let literal = &tail[..end];
        rest = &tail[end + 1..];
        // Import and image paths are structural, not visible strings: the
        // generated `tokens.slint` file name itself contains "token".
        if literal.contains('/')
            || literal.ends_with(".slint")
            || literal.ends_with(".otf")
            || literal.ends_with(".png")
        {
            continue;
        }
        let lower = literal.to_lowercase();
        for word in ["portal", "daemon", "registry", "config", "ipc", "token"] {
            assert!(
                !lower.contains(word),
                "the forbidden word {word:?} appears in the literal {literal:?}"
            );
        }
    }
}

// ------------------------------------------------------------ AC6

#[test]
fn ac6_the_four_snapshots_exist_as_pngs() {
    for name in [
        "welcome-en-light.png",
        "welcome-en-dark.png",
        "welcome-ar-light.png",
        "welcome-ar-dark.png",
    ] {
        let p = root().join("crates/ui/tests/snapshots").join(name);
        let bytes = std::fs::read(&p).unwrap_or_else(|e| panic!("{} must exist: {e}", p.display()));
        assert!(
            bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]),
            "{name} is not a PNG"
        );
    }
}

#[test]
fn ac6_rtl_mirrors_the_button_row_in_arabic() {
    let src = read("crates/ui/ui/welcome.slint");
    assert!(
        src.contains("if root.rtl"),
        "the welcome mirrors its element order for Arabic"
    );
    assert!(
        src.contains("if !root.rtl"),
        "the welcome keeps reading order for English"
    );
}

#[test]
fn ac4_both_buttons_carry_descriptive_accessible_labels() {
    let src = read("crates/ui/ui/welcome.slint");
    assert!(
        src.contains("FilledButton"),
        "the try button is a Material button"
    );
    assert!(
        src.contains("TextButton"),
        "the settings button is a Material button"
    );
    let labels = src.matches("accessible-label").count();
    assert!(
        labels >= 2,
        "every interactive element has a label; found {labels}"
    );
}
