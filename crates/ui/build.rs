//! Generates the design tokens from `design/tokens.json` and compiles the UI.
//!
//! Generated files in OUT_DIR:
//! - `tokens.slint`: the exported global `Tokens`, the only .slint file that
//!   may hold raw colours, lengths and durations (scripts/design-lint.sh
//!   skips it by name).
//! - `tokens.rs`: the same numbers as Rust constants, for the hex geometry.
//! - `material-tokens.slint`: the token colours under Material 3 role names
//!   (`MATERIAL_ROLES`), for the settings window.
//! - `settings_ui.rs`: the compiled settings window, included by
//!   src/settings.rs in its own module.
//!
//! Colours pick light or dark from `Palette.color-scheme`, so the theme follows
//! the system. Durations collapse to zero when `Tokens.reduced-motion` is set.

use serde_json::Value;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

const COLOUR_KEYS: [&str; 18] = [
    "surface-0",
    "surface-1",
    "surface-2",
    "surface-3",
    "border",
    "text-primary",
    "text-secondary",
    "accent",
    "accent-ink",
    "success",
    "warning",
    "danger",
    "progress",
    "hive-plate",
    "hive-edge",
    "hex-fill",
    "hex-fill-centre",
    "hex-ink",
];

fn main() {
    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let tokens_path = manifest.join("../../design/tokens.json");
    println!("cargo:rerun-if-changed={}", tokens_path.display());
    let raw = std::fs::read_to_string(&tokens_path).expect("read design/tokens.json");
    let t: Value = serde_json::from_str(&raw).expect("parse design/tokens.json");

    // The shell logos are copies of the brand files (S-094), so the crate
    // builds without the rest of the repository. A stale copy fails here.
    for name in ["kawaro-cells.svg", "kawaro-cells-dark.svg"] {
        let original = manifest.join("../../design/brand").join(name);
        if let Ok(brand) = std::fs::read(&original) {
            let copy = std::fs::read(manifest.join("resources/brand").join(name))
                .unwrap_or_else(|e| panic!("resources/brand/{name} must exist: {e}"));
            assert_eq!(
                brand, copy,
                "resources/brand/{name} differs from design/brand/{name}: copy it again"
            );
            println!("cargo:rerun-if-changed={}", original.display());
        }
    }

    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    std::fs::write(out.join("tokens.slint"), slint_tokens(&t)).unwrap();
    std::fs::write(out.join("tokens.rs"), rust_tokens(&t)).unwrap();

    // The settings window is compiled first, into its own Rust file that
    // src/settings.rs includes in a separate module. The hive goes last:
    // `include_modules!()` picks up the last `compile_with_config` output.
    let material = manifest.join("material");
    let schemes = std::fs::read_to_string(material.join("ui/styling/material_schemes.slint"))
        .expect("read material/ui/styling/material_schemes.slint");
    check_material_roles(&schemes);
    std::fs::write(out.join("material-tokens.slint"), material_tokens(&t)).unwrap();
    let settings_config = ui_config(&out).with_library_paths(std::collections::HashMap::from([(
        "material".to_string(),
        material.join("material.slint"),
    )]));
    let deps = slint_build::compile_with_output_path(
        manifest.join("ui/settings.slint"),
        out.join("settings_ui.rs"),
        settings_config,
    )
    .expect("compile ui/settings.slint");
    for dep in deps {
        println!("cargo:rerun-if-changed={}", dep.display());
    }

    // The welcome window (S-087), compiled into its own Rust file like the
    // settings window. Its film is a live vector scene (ui/film.slint, S-094c),
    // so no frame images are embedded; the sample image is embedded by
    // welcome_resources.rs below.
    let welcome_config = ui_config(&out).with_library_paths(std::collections::HashMap::from([(
        "material".to_string(),
        material.join("material.slint"),
    )]));
    let deps = slint_build::compile_with_output_path(
        manifest.join("ui/welcome.slint"),
        out.join("welcome_ui.rs"),
        welcome_config,
    )
    .expect("compile ui/welcome.slint");
    for dep in deps {
        println!("cargo:rerun-if-changed={}", dep.display());
    }
    let sample = manifest.join("../../tests/fixtures/images/welcome-sample.png");
    println!("cargo:rerun-if-changed={}", sample.display());
    std::fs::write(out.join("welcome_resources.rs"), welcome_resources(&sample)).unwrap();

    slint_build::compile_with_config(manifest.join("ui/hive.slint"), ui_config(&out))
        .expect("compile ui/hive.slint");
    rerun_dir(&manifest.join("ui"));
    rerun_dir(&manifest.join("components"));
    rerun_dir(&manifest.join("fonts"));
    rerun_dir(&manifest.join("icons"));
    rerun_dir(&manifest.join("resources"));
    rerun_dir(&material);
}

/// The embedded sample image of the first-run welcome (S-087 D6), one copy
/// in the shipped binary: `welcome::install_sample` writes it out for the
/// Try on an image button. Sourced from `tests/fixtures/images/`, the same
/// fixture the S-088 download button generates its site asset from.
fn welcome_resources(sample: &Path) -> String {
    let mut s = String::new();
    s.push_str("// Generated by crates/ui/build.rs. Do not edit.\n");
    s.push_str("/// The bundled sample image of the first-run welcome (S-087).\n");
    writeln!(
        s,
        "pub static SAMPLE: &[u8] = include_bytes!(r#\"{}\"#);",
        sample.display()
    )
    .unwrap();
    s
}

fn ui_config(out: &Path) -> slint_build::CompilerConfiguration {
    slint_build::CompilerConfiguration::new()
        .with_style("fluent".into())
        .with_include_paths(vec![out.to_path_buf()])
        .embed_resources(slint_build::EmbedResourcesKind::EmbedFiles)
        // Element debug info lets the behaviour tests find elements by their
        // accessible label in every profile (`cargo test --release` too).
        // It only adds element names and ids to the generated code.
        .with_debug_info(true)
}

/// Material 3 colour roles and the token each one takes, as (role, light,
/// dark). A token name starting with `~` is read from the other scheme: the
/// inverse roles show the opposite theme. Every role of `MaterialScheme` must
/// be listed; the build fails otherwise (see `check_material_roles`), so no
/// role can fall back to an unset colour.
const MATERIAL_ROLES: [(&str, &str); 49] = [
    ("primary", "accent"),
    ("surfaceTint", "accent"),
    ("onPrimary", "accent-ink"),
    // The switch handle when on and the like: a surface on the accent track.
    ("primaryContainer", "surface-0"),
    ("onPrimaryContainer", "text-primary"),
    ("secondary", "text-secondary"),
    ("onSecondary", "surface-0"),
    // Selected segment.
    ("secondaryContainer", "surface-3"),
    ("onSecondaryContainer", "text-primary"),
    ("tertiary", "accent"),
    ("onTertiary", "accent-ink"),
    ("tertiaryContainer", "surface-2"),
    ("onTertiaryContainer", "text-primary"),
    ("error", "danger"),
    ("onError", "surface-0"),
    ("errorContainer", "surface-2"),
    ("onErrorContainer", "danger"),
    ("background", "surface-0"),
    ("onBackground", "text-primary"),
    ("surface", "surface-0"),
    ("onSurface", "text-primary"),
    ("surfaceVariant", "surface-2"),
    ("onSurfaceVariant", "text-secondary"),
    // Outlines of controls need contrast, so the secondary text tone; the
    // softer border token is the variant.
    ("outline", "text-secondary"),
    ("outlineVariant", "border"),
    // Shadows are removed from the library; scrim dims behind modals only.
    ("shadow", "text-primary"),
    ("scrim", "text-primary"),
    ("inverseSurface", "~surface-1"),
    ("inverseOnSurface", "~text-primary"),
    ("inversePrimary", "~accent"),
    ("primaryFixed", "accent"),
    ("onPrimaryFixed", "accent-ink"),
    ("primaryFixedDim", "accent"),
    ("onPrimaryFixedVariant", "accent-ink"),
    ("secondaryFixed", "surface-2"),
    ("onSecondaryFixed", "text-primary"),
    ("secondaryFixedDim", "surface-3"),
    ("onSecondaryFixedVariant", "text-secondary"),
    ("tertiaryFixed", "accent"),
    ("onTertiaryFixed", "accent-ink"),
    ("tertiaryFixedDim", "accent"),
    ("onTertiaryFixedVariant", "accent-ink"),
    ("surfaceDim", "surface-1"),
    ("surfaceBright", "surface-0"),
    ("surfaceContainerLowest", "surface-0"),
    ("surfaceContainerLow", "surface-1"),
    ("surfaceContainer", "surface-1"),
    ("surfaceContainerHigh", "surface-2"),
    ("surfaceContainerHighest", "surface-3"),
];

/// Fails the build when the vendored `MaterialScheme` and `MATERIAL_ROLES`
/// disagree, for example after a library update.
fn check_material_roles(schemes_slint: &str) {
    let body = schemes_slint
        .split("export struct MaterialScheme {")
        .nth(1)
        .and_then(|s| s.split('}').next())
        .expect("MaterialScheme struct in material_schemes.slint");
    let mut fields: Vec<&str> = body
        .split(',')
        .filter_map(|f| f.split(':').next())
        .map(str::trim)
        .filter(|f| !f.is_empty())
        .collect();
    let mut ours: Vec<&str> = MATERIAL_ROLES.iter().map(|(r, _)| *r).collect();
    fields.sort_unstable();
    ours.sort_unstable();
    assert_eq!(
        fields, ours,
        "MATERIAL_ROLES in build.rs must list every MaterialScheme field"
    );
}

fn material_tokens(t: &Value) -> String {
    let mut s = String::new();
    s.push_str("// Generated by crates/ui/build.rs from design/tokens.json. Do not edit.\n");
    s.push_str("import { MaterialSchemes } from \"@material\";\n\n");
    s.push_str("// Token colours under Material 3 role names, assigned once to\n");
    s.push_str("// MaterialPalette.schemes by the settings window.\n");
    s.push_str("export global SbarMaterial {\n");
    s.push_str("    out property <MaterialSchemes> schemes: {\n");
    for (scheme, other) in [("light", "dark"), ("dark", "light")] {
        writeln!(s, "        {scheme}: {{").unwrap();
        for (role, token) in MATERIAL_ROLES {
            let value = match token.strip_prefix('~') {
                Some(key) => text(t, &["color", other, key]),
                None => text(t, &["color", scheme, token]),
            };
            writeln!(s, "            {role}: {value},").unwrap();
        }
        s.push_str("        },\n");
    }
    s.push_str("    };\n}\n");
    s
}

fn rerun_dir(dir: &Path) {
    println!("cargo:rerun-if-changed={}", dir.display());
}

fn num(t: &Value, path: &[&str]) -> f64 {
    let mut v = t;
    for p in path {
        v = &v[*p];
    }
    v.as_f64()
        .unwrap_or_else(|| panic!("tokens.json: {} is not a number", path.join(".")))
}

fn text<'a>(t: &'a Value, path: &[&str]) -> &'a str {
    let mut v = t;
    for p in path {
        v = &v[*p];
    }
    v.as_str()
        .unwrap_or_else(|| panic!("tokens.json: {} is not a string", path.join(".")))
}

fn slint_tokens(t: &Value) -> String {
    // The easing is a keyword in .slint, not a value, so ui/hive.slint writes
    // it literally. Fail the build if the tokens ever ask for another one.
    assert_eq!(
        text(t, &["motion", "hive-open", "easing"]),
        "ease-out",
        "ui/hive.slint hard-codes ease-out; update it together with tokens.json"
    );
    let mut s = String::new();
    s.push_str("// Generated by crates/ui/build.rs from design/tokens.json. Do not edit.\n");
    s.push_str("import { Palette } from \"std-widgets.slint\";\n\n");
    s.push_str("export global Tokens {\n");
    s.push_str("    in-out property <bool> dark: Palette.color-scheme == ColorScheme.dark;\n");
    s.push_str("    in-out property <bool> reduced-motion: false;\n");
    for key in COLOUR_KEYS {
        let light = text(t, &["color", "light", key]);
        let dark = text(t, &["color", "dark", key]);
        writeln!(
            s,
            "    out property <color> {key}: dark ? {dark} : {light};"
        )
        .unwrap();
    }
    // Add glass colors from the glass section
    for glass_key in ["glass-2", "glass-3"] {
        let light = text(t, &["glass", glass_key, "light"]);
        let dark = text(t, &["glass", glass_key, "dark"]);
        writeln!(
            s,
            "    out property <color> {glass_key}: dark ? {dark} : {light};"
        )
        .unwrap();
    }
    for size in ["sm", "md", "lg"] {
        let v = num(t, &["hex", "size", size]);
        writeln!(s, "    out property <length> hex-{size}: {v}px;").unwrap();
    }
    writeln!(
        s,
        "    out property <length> hex-gap: {}px;",
        num(t, &["hex", "gap"])
    )
    .unwrap();
    for k in [
        "icon-size",
        "icon-gap",
        "preview-icon-size",
        "stack-offset",
        "badge-size",
    ] {
        writeln!(
            s,
            "    out property <length> hex-{k}: {}px;",
            num(t, &["hex", k])
        )
        .unwrap();
    }
    for k in ["1", "2", "3", "4", "5", "6", "7", "8"] {
        writeln!(
            s,
            "    out property <length> space-{k}: {}px;",
            num(t, &["space", k])
        )
        .unwrap();
    }
    // The S-094 window and shell geometry (tokens.json `layout`).
    for k in [
        "window-width",
        "window-height",
        "window-min-width",
        "window-min-height",
        "header-height",
        "header-logo",
        "rail-width",
        "content-max-width",
        "row-breakpoint",
        "label-column",
        "film-max-width",
    ] {
        writeln!(
            s,
            "    out property <length> layout-{k}: {}px;",
            num(t, &["layout", k])
        )
        .unwrap();
    }
    for k in ["sm", "md", "lg"] {
        writeln!(
            s,
            "    out property <length> radius-{k}: {}px;",
            num(t, &["radius", k])
        )
        .unwrap();
    }
    for k in ["default", "focus", "hex"] {
        writeln!(
            s,
            "    out property <length> border-{k}: {}px;",
            num(t, &["border-width", k])
        )
        .unwrap();
    }
    writeln!(
        s,
        "    out property <string> font-family: \"{}\";",
        text(t, &["font", "family"])
    )
    .unwrap();
    for k in ["label", "body", "title", "cell"] {
        writeln!(
            s,
            "    out property <length> font-{k}: {}px;",
            num(t, &["font", "size", k])
        )
        .unwrap();
    }
    for k in ["regular", "medium", "semibold"] {
        writeln!(
            s,
            "    out property <int> weight-{k}: {};",
            num(t, &["font", "weight", k])
        )
        .unwrap();
    }
    let open = num(t, &["motion", "hive-open", "duration-ms"]);
    let from = num(t, &["motion", "hive-open", "scale-from"]);
    let to = num(t, &["motion", "hive-open", "scale-to"]);
    let hover = num(t, &["motion", "hover", "duration-ms"]);
    let ring_delay = num(t, &["motion", "ring-delay-ms"]);
    let highlight_move_duration = num(t, &["motion", "highlight-move", "duration-ms"]);
    let press_duration = num(t, &["motion", "press", "duration-ms"]);
    let press_scale = num(t, &["motion", "press", "scale-to"]);
    let hive_close_duration = num(t, &["motion", "hive-close", "duration-ms"]);
    let hive_close_scale = num(t, &["motion", "hive-close", "scale-to"]);
    let hive_close_opacity = num(t, &["motion", "hive-close", "opacity-to"]);

    // Read and validate easing values from tokens.json
    let hive_open_easing = text(t, &["motion", "hive-open", "easing"]);
    validate_easing(hive_open_easing);
    let highlight_move_easing = text(t, &["motion", "highlight-move", "easing"]);
    validate_easing(highlight_move_easing);
    let hive_close_easing = text(t, &["motion", "hive-close", "easing"]);
    validate_easing(hive_close_easing);

    writeln!(
        s,
        "    out property <duration> hive-open: reduced-motion ? 0ms : {open}ms;"
    )
    .unwrap();
    writeln!(
        s,
        "    out property <float> hive-scale-from: reduced-motion ? {to} : {from};"
    )
    .unwrap();
    writeln!(s, "    out property <float> hive-scale-to: {to};").unwrap();
    writeln!(
        s,
        "    out property <duration> hover: reduced-motion ? 0ms : {hover}ms;"
    )
    .unwrap();
    writeln!(
        s,
        "    out property <duration> ring-delay: reduced-motion ? 0ms : {ring_delay}ms;"
    )
    .unwrap();
    writeln!(
        s,
        "    out property <duration> highlight-move: reduced-motion ? 0ms : {highlight_move_duration}ms;"
    )
    .unwrap();
    writeln!(
        s,
        "    out property <duration> press: reduced-motion ? 0ms : {press_duration}ms;"
    )
    .unwrap();
    writeln!(s, "    out property <float> press-scale-to: {press_scale};").unwrap();
    writeln!(
        s,
        "    out property <duration> hive-close: reduced-motion ? 0ms : {hive_close_duration}ms;"
    )
    .unwrap();
    writeln!(
        s,
        "    out property <float> hive-close-scale-to: {hive_close_scale};"
    )
    .unwrap();
    writeln!(
        s,
        "    out property <float> hive-close-opacity-to: {hive_close_opacity};"
    )
    .unwrap();
    writeln!(
        s,
        "    out property <easing> hive-open-easing: {hive_open_easing};\n"
    )
    .unwrap();
    writeln!(
        s,
        "    out property <easing> highlight-move-easing: {highlight_move_easing};\n"
    )
    .unwrap();
    writeln!(
        s,
        "    out property <easing> hive-close-easing: {hive_close_easing};\n"
    )
    .unwrap();
    s.push_str("}\n");
    s
}

fn rust_tokens(t: &Value) -> String {
    let mut s = String::new();
    s.push_str("// Generated by crates/ui/build.rs from design/tokens.json. Do not edit.\n");
    writeln!(
        s,
        "/// Flat-to-flat width of a hive cell, tokens hex.size.md.\npub const HEX_SIZE_MD: f32 = {:.1};",
        num(t, &["hex", "size", "md"])
    )
    .unwrap();
    writeln!(
        s,
        "/// Gap between neighbouring cells, tokens hex.gap.\npub const HEX_GAP: f32 = {:.1};",
        num(t, &["hex", "gap"])
    )
    .unwrap();
    writeln!(
        s,
        "/// Window padding around the outer ring, tokens space.1.\npub const PAD: f32 = {:.1};",
        num(t, &["space", "1"])
    )
    .unwrap();
    writeln!(
        s,
        "/// Cells in ring 1, tokens hex.ring-1-count.\npub const RING1: usize = {};",
        num(t, &["hex", "ring-1-count"]) as usize
    )
    .unwrap();
    writeln!(
        s,
        "/// Cells in ring 2, tokens hex.ring-2-count.\npub const RING2: usize = {};",
        num(t, &["hex", "ring-2-count"]) as usize
    )
    .unwrap();
    writeln!(
        s,
        "/// Close animation length, tokens motion.hive-close.duration-ms.\npub const HIVE_CLOSE_MS: u64 = {};",
        num(t, &["motion", "hive-close", "duration-ms"]) as u64
    )
    .unwrap();
    s
}

/// Validate an easing value and panic if invalid.
/// Valid easing = known name (ease-in, ease-out, ease-in-out, linear) or
/// well-formed `cubic-bezier(a, b, c, d)` with exactly four numeric arguments.
fn validate_easing(value: &str) {
    match value {
        "ease-in" | "ease-out" | "ease-in-out" | "linear" => {}
        s if s.starts_with("cubic-bezier(") && s.ends_with(')') => {
            let args = &s[13..s.len() - 1];
            let parts: Vec<&str> = args.split(',').map(|s| s.trim()).collect();
            if parts.len() != 4 {
                panic!(
                    "Invalid cubic-bezier: expected 4 arguments, got {} in '{}'",
                    parts.len(),
                    value
                );
            }
            for part in parts {
                part.parse::<f32>().unwrap_or_else(|_| {
                    panic!("Invalid cubic-bezier argument: '{}' in '{}'", part, value)
                });
            }
        }
        _ => panic!("Unknown easing: '{}'", value),
    }
}
