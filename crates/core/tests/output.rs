//! S-002 AC7 to AC9: output naming, temp paths and finalisation.

use sbarhive_core::action::{OutputMode, OutputSpec};
use sbarhive_core::output::{final_path, finalize, temp_path};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

fn spec(ext: &str, suffix: &str) -> OutputSpec {
    OutputSpec {
        mode: OutputMode::PerFile,
        ext: ext.into(),
        suffix: suffix.into(),
        dir: String::new(),
    }
}

fn touch(p: &Path) {
    fs::write(p, b"x").unwrap();
}

#[test]
fn ac7_free_name_uses_stem_suffix_ext() {
    let d = tempfile::tempdir().unwrap();
    let input = d.path().join("photo.jpg");
    touch(&input);
    assert_eq!(
        final_path(&input, &spec("webp", ""), d.path()),
        d.path().join("photo.webp")
    );
    assert_eq!(
        final_path(&input, &spec("", "-small"), d.path()),
        d.path().join("photo-small.jpg")
    );
    assert_eq!(
        final_path(&input, &spec("png", "-gray"), d.path()),
        d.path().join("photo-gray.png")
    );
}

#[test]
fn ac7_out_dir_is_respected() {
    let d = tempfile::tempdir().unwrap();
    let out = d.path().join("out");
    fs::create_dir(&out).unwrap();
    let input = d.path().join("a.png");
    assert_eq!(
        final_path(&input, &spec("webp", ""), &out),
        out.join("a.webp")
    );
}

#[test]
fn ac7_existing_names_get_numbers() {
    let d = tempfile::tempdir().unwrap();
    let input = d.path().join("photo.jpg");
    touch(&input);
    touch(&d.path().join("photo.webp"));
    assert_eq!(
        final_path(&input, &spec("webp", ""), d.path()),
        d.path().join("photo (1).webp")
    );
    touch(&d.path().join("photo (1).webp"));
    assert_eq!(
        final_path(&input, &spec("webp", ""), d.path()),
        d.path().join("photo (2).webp")
    );
}

#[test]
fn ac7_output_equal_to_input_starts_numbering() {
    let d = tempfile::tempdir().unwrap();
    // The input need not exist: equality alone forces numbering.
    let input = d.path().join("photo.png");
    assert_eq!(
        final_path(&input, &spec("", ""), d.path()),
        d.path().join("photo (1).png")
    );
    assert_eq!(
        final_path(&input, &spec("png", ""), d.path()),
        d.path().join("photo (1).png")
    );
}

#[test]
fn ac7_input_without_extension() {
    let d = tempfile::tempdir().unwrap();
    let input = d.path().join("README");
    touch(&input);
    assert_eq!(
        final_path(&input, &spec("", "-x"), d.path()),
        d.path().join("README-x")
    );
    assert_eq!(
        final_path(&input, &spec("", ""), d.path()),
        d.path().join("README (1)")
    );
    assert_eq!(
        final_path(&input, &spec("pdf", ""), d.path()),
        d.path().join("README.pdf")
    );
}

#[cfg(unix)]
#[test]
fn ac7_dangling_symlink_counts_as_taken() {
    let d = tempfile::tempdir().unwrap();
    let input = d.path().join("a.png");
    std::os::unix::fs::symlink(d.path().join("nowhere"), d.path().join("a.webp")).unwrap();
    assert_eq!(
        final_path(&input, &spec("webp", ""), d.path()),
        d.path().join("a (1).webp")
    );
}

#[test]
fn ac8_temp_paths_are_hidden_local_same_ext_and_unique() {
    let d = tempfile::tempdir().unwrap();
    let fin = d.path().join("photo (1).webp");
    let mut seen = HashSet::new();
    for _ in 0..1000 {
        let t = temp_path(&fin);
        assert_eq!(t.parent(), Some(d.path()));
        assert_eq!(t.extension().unwrap(), "webp");
        let name = t.file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with('.'), "{name}");
        assert!(name.contains("photo (1)"), "{name}");
        assert!(seen.insert(t));
    }
}

#[test]
fn ac8_temp_path_without_extension_has_none() {
    let d = tempfile::tempdir().unwrap();
    let t = temp_path(&d.path().join("README"));
    assert_eq!(t.parent(), Some(d.path()));
    assert_eq!(t.extension(), None);
    assert!(t.file_name().unwrap().to_str().unwrap().starts_with('.'));
}

#[test]
fn ac8_temp_paths_unique_across_threads() {
    let d = tempfile::tempdir().unwrap();
    let fin = d.path().join("x.png");
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let fin = fin.clone();
            std::thread::spawn(move || (0..250).map(|_| temp_path(&fin)).collect::<Vec<_>>())
        })
        .collect();
    let mut seen = HashSet::new();
    for h in handles {
        for t in h.join().unwrap() {
            assert!(seen.insert(t));
        }
    }
}

#[test]
fn ac9_finalize_moves_temp_to_free_name() {
    let d = tempfile::tempdir().unwrap();
    let fin = d.path().join("a.webp");
    let tmp = temp_path(&fin);
    fs::write(&tmp, b"new").unwrap();
    let used = finalize(&tmp, &fin).unwrap();
    assert_eq!(used, fin);
    assert_eq!(fs::read(&fin).unwrap(), b"new");
    assert!(!tmp.exists());
    assert_eq!(fs::read_dir(d.path()).unwrap().count(), 1);
}

#[test]
fn ac9_finalize_never_overwrites() {
    let d = tempfile::tempdir().unwrap();
    let fin = d.path().join("a.webp");
    fs::write(&fin, b"old").unwrap();
    fs::write(d.path().join("a (1).webp"), b"old1").unwrap();
    let tmp = temp_path(&fin);
    fs::write(&tmp, b"new").unwrap();
    let used = finalize(&tmp, &fin).unwrap();
    assert_eq!(used, d.path().join("a (2).webp"));
    assert_eq!(fs::read(&fin).unwrap(), b"old");
    assert_eq!(fs::read(d.path().join("a (1).webp")).unwrap(), b"old1");
    assert_eq!(fs::read(&used).unwrap(), b"new");
    assert!(!tmp.exists());
}

#[test]
fn ac9_finalize_continues_numbering_of_a_numbered_target() {
    let d = tempfile::tempdir().unwrap();
    let fin = d.path().join("a (3).webp");
    fs::write(&fin, b"old").unwrap();
    let tmp = temp_path(&fin);
    fs::write(&tmp, b"new").unwrap();
    assert_eq!(finalize(&tmp, &fin).unwrap(), d.path().join("a (4).webp"));
}

#[test]
fn ac9_finalize_target_without_extension() {
    let d = tempfile::tempdir().unwrap();
    let fin = d.path().join("notes");
    fs::write(&fin, b"old").unwrap();
    let tmp = temp_path(&fin);
    fs::write(&tmp, b"new").unwrap();
    assert_eq!(finalize(&tmp, &fin).unwrap(), d.path().join("notes (1)"));
}

#[test]
fn ac9_finalize_missing_temp_is_error_and_creates_nothing() {
    let d = tempfile::tempdir().unwrap();
    let fin = d.path().join("a.webp");
    let tmp = temp_path(&fin);
    assert!(finalize(&tmp, &fin).is_err());
    assert_eq!(fs::read_dir(d.path()).unwrap().count(), 0);
}
