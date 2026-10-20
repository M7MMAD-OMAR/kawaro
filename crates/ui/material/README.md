# Vendored: Slint Material 3 components

- Library: the official Material Design 3 component set for Slint
  (https://material.slint.dev).
- Version: Slint 1.18.1, the same version as the pinned `slint` crate. The
  standalone repository github.com/slint-ui/material-components is archived
  (it has no releases or tags); the library now lives in the Slint repository
  under `ui-libraries/material/src`.
- Source: https://codeload.github.com/slint-ui/slint/tar.gz/refs/tags/v1.18.1,
  directory `slint-1.18.1/ui-libraries/material/src`.
  SHA-256 of that tarball:
  `fe485305ed303215e76c04918ee9aefbffbe229f18f979098ec36c7fa1dab28b`.
- License: MIT, `LICENSE.md` in this directory (Copyright SixtyFPS GmbH).
  The upstream README is kept as `UPSTREAM-README.md`.
- Imported as `@material` by `crates/ui/ui/settings.slint` (library path set
  in `crates/ui/build.rs`).

## What was modified

Everything else is byte for byte as in the tarball.

1. `ui/components/elevation.slint`: every drop shadow removed. The component
   keeps its properties (`background`, `border_radius`, `level`) so the
   library still compiles, but `level` has no effect. Our files never set an
   elevation level; height comes from surface tones.
2. `ui/components/text_field.slint`: the bottom-only active indicator and the
   top-only corner radius are replaced by a full rounded border that thickens
   on focus (no one-sided borders), and a `horizontal_alignment` property is
   added so right-to-left forms align the text and label to the right.
3. `ui/components/segmented_button.slint`: the 1px side divider between
   segments is removed (no side lines), and the selected segment rounds its
   own outer corners, because the software renderer does not clip children to
   a rounded parent.

The colours are not edited here: `crates/ui/build.rs` maps
`design/tokens.json` to `MaterialPalette.schemes`, which the settings window
assigns once at start.

## Updating

Upgrading Slint is its own task. Download the matching tag, replace this
directory, reapply the three changes above, and run `scripts/design-lint.sh`
(it still checks this directory for shadows, gradients and dashes) and
`crates/ui/tests/settings_snapshots.sh`. The build fails if the Material
colour roles change, until `MATERIAL_ROLES` in `crates/ui/build.rs` matches.
