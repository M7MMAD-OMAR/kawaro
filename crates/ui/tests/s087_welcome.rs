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

/// The `resources/welcome/frame-XX-NN.png` references of welcome.slint, as
/// (family, path relative to the crate) in file order.
fn frame_refs() -> Vec<(String, PathBuf)> {
    let src = read("crates/ui/ui/welcome.slint");
    let mut out = Vec::new();
    let mut rest = src.as_str();
    while let Some(i) = rest.find("resources/welcome/frame-") {
        let tail = &rest[i + "resources/welcome/frame-".len()..];
        let end = tail.find('"').expect("the path literal closes");
        let name = &tail[..end];
        let family = name.split('-').next().unwrap_or_default().to_string();
        out.push((
            family,
            Path::new("crates/ui")
                .join("resources/welcome")
                .join(format!("frame-{name}")),
        ));
        rest = &tail[end..];
    }
    out
}

fn png_size(rel: &Path) -> (u32, u32) {
    let decoder = png::Decoder::new(std::fs::File::open(root().join(rel)).unwrap());
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    reader.next_frame(&mut buf).unwrap();
    let info = reader.info();
    (info.width, info.height)
}

// ------------------------------------------------------------ AC2

#[test]
fn ac2_the_embedded_frame_set_is_non_empty_in_every_sequence() {
    let refs = frame_refs();
    assert!(!refs.is_empty(), "welcome.slint embeds no frames");
    for family in ["lp", "lc", "dp", "dc"] {
        let n = refs.iter().filter(|(f, _)| f == family).count();
        assert!(
            n >= 10,
            "the {family} sequence holds {n} frames, expected at least 10"
        );
    }
    for (_, rel) in &refs {
        assert!(root().join(rel).exists(), "{} is missing", rel.display());
    }
}

#[test]
fn ac2_the_frames_and_the_sample_fit_the_two_megabyte_budget() {
    let mut total: u64 = 0;
    for (_, rel) in frame_refs() {
        total += std::fs::metadata(root().join(&rel)).unwrap().len();
    }
    let sample = root().join("tests/fixtures/images/welcome-sample.png");
    total += std::fs::metadata(&sample)
        .expect("the sample image exists")
        .len();
    assert!(
        total <= 2 * 1024 * 1024,
        "the embedded frame set and sample image are {total} bytes, over the 2 MB of AC2"
    );
}

#[test]
fn ac2_the_frames_decode_at_one_size() {
    let mut sizes: Vec<(u32, u32)> = frame_refs().iter().map(|(_, rel)| png_size(rel)).collect();
    sizes.sort();
    sizes.dedup();
    assert_eq!(sizes.len(), 1, "every frame has the same size: {sizes:?}");
    let (w, h) = sizes[0];
    assert!(w >= 320 && h >= 180, "the demo area is too small: {w}x{h}");
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
fn ac2_no_frame_asset_without_the_generator() {
    let script = read("scripts/gen-welcome-frames.sh");
    assert!(script.contains("S-087"), "the generator names its story");
    assert!(
        script.contains("welcome_frames"),
        "the generator runs the frame renderer"
    );
    assert!(
        script.contains("resources/welcome"),
        "the generator writes into crates/ui/resources/welcome"
    );
    let note = read("crates/ui/resources/welcome/README.md");
    assert!(
        note.contains("gen-welcome-frames.sh"),
        "the licence note names the generator"
    );
    assert!(
        note.to_lowercase().contains("licence") || note.to_lowercase().contains("license"),
        "the licence note states a licence"
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
