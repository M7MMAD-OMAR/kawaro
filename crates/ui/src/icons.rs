//! The hive cell icons: a fixed set of monoline SVGs in `crates/ui/icons/`,
//! embedded in the binary. Actions name one with `icon = "..."` in
//! `actions/*.toml`; the cell draws it above the code, coloured like the code
//! (the SVGs are black only and Slint colourises them).

use std::cell::RefCell;
use std::collections::HashMap;

macro_rules! icons {
    ($($name:literal),* $(,)?) => {
        [$(($name, include_bytes!(concat!("../icons/", $name, ".svg")) as &[u8])),*]
    };
}

/// Every icon, by name. The only list: add a file to `crates/ui/icons/` and
/// its name here.
const ICONS: [(&str, &[u8]); 22] = icons![
    "audio",
    "clean",
    "compress",
    "convert",
    "document",
    "file",
    "fit",
    "flip",
    "gif",
    "grayscale",
    "icon",
    "image",
    "merge",
    "pages",
    "pdf",
    "resize",
    "rotate",
    "sheet",
    "slides",
    "split",
    "text",
    "video",
];

thread_local! {
    /// Parsed icons, so a repeated `present` does not parse the SVGs again.
    static CACHE: RefCell<HashMap<&'static str, slint::Image>> = RefCell::new(HashMap::new());
}

/// Names of the built-in cell icons, sorted.
pub fn icon_names() -> impl Iterator<Item = &'static str> {
    ICONS.iter().map(|(n, _)| *n)
}

/// The SVG source of an icon, `None` for an empty or unknown name.
fn svg(name: &str) -> Option<(&'static str, &'static [u8])> {
    ICONS.iter().find(|(n, _)| *n == name).copied()
}

/// The icon as a Slint image, `None` for an empty or unknown name.
pub(crate) fn image(name: &str) -> Option<slint::Image> {
    let (key, data) = svg(name)?;
    CACHE.with(|c| {
        if let Some(img) = c.borrow().get(key) {
            return Some(img.clone());
        }
        let img = slint::Image::load_from_svg_data(data).ok()?;
        c.borrow_mut().insert(key, img.clone());
        Some(img)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::path::Path;

    fn icons_dir() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("icons")
    }

    #[test]
    fn the_list_matches_the_icons_folder() {
        let files: BTreeSet<String> = std::fs::read_dir(icons_dir())
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "svg"))
            .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
            .collect();
        let listed: BTreeSet<String> = icon_names().map(String::from).collect();
        assert_eq!(files, listed);
        let names: Vec<&str> = icon_names().collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted, "ICONS stays sorted");
    }

    #[test]
    fn each_name_gives_its_own_file() {
        for name in icon_names() {
            let (key, data) = svg(name).unwrap();
            assert_eq!(key, name);
            let on_disk = std::fs::read(icons_dir().join(format!("{name}.svg"))).unwrap();
            assert_eq!(data, on_disk.as_slice(), "{name}");
        }
    }

    #[test]
    fn empty_or_unknown_names_have_no_icon() {
        assert!(svg("").is_none());
        assert!(svg("no-such-icon").is_none());
        assert!(image("").is_none());
        assert!(image("no-such-icon").is_none());
    }

    #[test]
    fn every_icon_loads_at_its_24_px_viewbox() {
        for name in icon_names() {
            // Twice: the second call comes from the cache.
            for _ in 0..2 {
                let img = image(name).unwrap_or_else(|| panic!("{name} does not load"));
                let size = img.size();
                assert_eq!((size.width, size.height), (24, 24), "{name}");
            }
        }
    }

    #[test]
    fn every_builtin_action_has_a_known_icon() {
        let reg = kawaro_core::action::Registry::builtin();
        assert!(!reg.all().is_empty());
        let known: BTreeSet<&str> = icon_names().collect();
        for a in reg.all() {
            assert!(!a.icon.is_empty(), "{} has no icon", a.id);
            assert!(
                known.contains(a.icon.as_str()),
                "{} names unknown icon `{}`",
                a.id,
                a.icon
            );
        }
    }
}
