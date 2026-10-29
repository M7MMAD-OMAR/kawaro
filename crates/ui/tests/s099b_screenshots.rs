//! S-099b B8: the screenshot contract. 16 images (four states, English and
//! Arabic, light and dark) in `docs/screenshots/s099/`, made by
//! `crates/ui/tests/updates_screenshots.sh` through the `settings_snapshot`
//! example. Written from the acceptance criteria only.
//!
//! Pinned names:
//! - file `docs/screenshots/s099/updates-<lang>-<scheme>-<state>.png` with
//!   lang `en|ar`, scheme `light|dark`, state `downloading|ready|installing|updated`.
//! - the script renders all 16 when given the argument `s099`-style default
//!   or when it is run with no OUTDIR, and sets `KAWARO_COLOR_SCHEME` to each
//!   scheme; `settings_snapshot --update-state` accepts the four state words.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

const LANGS: [&str; 2] = ["en", "ar"];
const SCHEMES: [&str; 2] = ["light", "dark"];
const STATES: [&str; 4] = ["downloading", "ready", "installing", "updated"];

#[test]
fn ac_b8_the_script_renders_four_states_in_two_languages_and_two_schemes_to_s099() {
    let src = std::fs::read_to_string(root().join("crates/ui/tests/updates_screenshots.sh"))
        .expect("the script exists");
    assert!(
        src.contains("docs/screenshots/s099"),
        "default output folder is docs/screenshots/s099"
    );
    for word in STATES.iter().chain(SCHEMES.iter()).chain(LANGS.iter()) {
        assert!(src.contains(word), "script must mention {word}");
    }
    assert!(src.contains("KAWARO_COLOR_SCHEME"));
    assert!(src.contains("updates-"), "file name pattern updates-...");
    assert!(
        src.contains("$scheme") || src.contains("${scheme}"),
        "the scheme is in the loop and in the file name"
    );
    assert!(!src.contains('\u{2014}') && !src.contains('\u{2013}'));
}

#[test]
fn ac_b8_the_snapshot_example_accepts_the_new_state_words() {
    let src = std::fs::read_to_string(root().join("crates/ui/examples/settings_snapshot.rs"))
        .expect("example exists");
    for state in ["installing", "updated"] {
        assert!(
            src.contains(&format!("\"{state}\"")),
            "--update-state {state} is not handled"
        );
    }
}

#[test]
fn ac_b8_sixteen_png_screenshots_exist_with_one_size_each_language() {
    let dir = root().join("docs/screenshots/s099");
    let mut sizes = Vec::new();
    let mut hashes = Vec::new();
    for lang in LANGS {
        for scheme in SCHEMES {
            for state in STATES {
                let p = dir.join(format!("updates-{lang}-{scheme}-{state}.png"));
                let bytes = std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
                assert_eq!(
                    &bytes[..8],
                    b"\x89PNG\r\n\x1a\n",
                    "{} is a PNG",
                    p.display()
                );
                let w = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
                let h = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
                assert!(w >= 400 && h >= 300, "{}: {w}x{h}", p.display());
                sizes.push((w, h));
                hashes.push(bytes);
            }
        }
    }
    assert_eq!(sizes.len(), 16);
    assert!(sizes.iter().all(|s| *s == sizes[0]), "one window size");
    let mut uniq = hashes.clone();
    uniq.sort();
    uniq.dedup();
    assert_eq!(uniq.len(), 16, "every image differs");
    let count = std::fs::read_dir(&dir)
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .map(|e| e.path().extension().is_some_and(|x| x == "png"))
                .unwrap_or(false)
        })
        .count();
    assert_eq!(count, 16, "exactly 16 images");
}
