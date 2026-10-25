# ADR 0002: Settings window components

Status: accepted
Date: 2026-10-20

## Context

`CLAUDE.md` (UI section) requires settings windows built from Slint's official Material 3 library, with colours from `design/tokens.json` passed once to `MaterialPalette.schemes`, and no shadows. The first fallback was Sleek-ui, the last Slint's standard widgets with the fluent style. Slint is pinned at 1.18.1.

What we tried:

1. **The standalone repo `slint-ui/material-components`:** archived since September 2025, no releases or tags, and its README says development moved into the Slint repo. Not used, because it matches no specific version.
2. **The library inside the Slint repo:** `ui-libraries/material/src` at the `v1.18.1` tag, so it matches the pinned version. Downloaded from the tag archive (`codeload.github.com/slint-ui/slint/tar.gz/refs/tags/v1.18.1`), MIT licensed. The settings window built on it with no compiler error or warning.

Sleek-ui and fluent were not needed.

## Decision

- Use the Material 3 library from the Slint repo at v1.18.1, vendored in `crates/ui/material/` with its licence and a README giving the source, the hash and every change.
- Import it as `@material` (a library path in `crates/ui/build.rs`). The settings window compiles in its own Rust file (`settings_ui.rs`), included by `src/settings.rs` in a private module, because `include_modules!()` takes only the last compiled file, which is the hive.
- Components used: `MaterialWindow`, `SegmentedButton` (language, theme, output location, quality), `TextField` (folder name), `Switch`, `FilledButton` (save), `TextButton` (cancel). No library component was rebuilt; `Choice` and `SwitchRow` in `settings.slint` only group library components with a title or label.

### Changes to the library

1. **Shadows:** `elevation.slint` is a rounded rectangle with no shadow, keeping its properties so the rest of the library is unchanged. Our files never set `level`.
2. **Text field:** it was a bottom line with rounded top corners, a one sided border. It is now a full rounded border that thickens on focus, plus a `horizontal_alignment` property to right align text and label in Arabic.
3. **Segmented buttons:** removed the side divider between segments (a side accent line); the selected segment shows by its surface and check mark. The selected end segment rounds its own corners, because the software renderer does not clip children to the parent's rounded bounds and the colour leaked past the edge.

### Colours

`build.rs` reads the `MaterialScheme` fields from the vendored file and fails the build unless the `MATERIAL_ROLES` table matches them field by field, so no role is left without a colour. It then writes `material-tokens.slint`, which the window assigns once to `MaterialPalette.schemes` in `init`.

| Token | Material roles |
| --- | --- |
| `accent` | primary, surfaceTint, tertiary, primaryFixed and siblings |
| `accent-ink` | onPrimary, onTertiary, onPrimaryFixed |
| `surface-0` | background, surface, surfaceBright, surfaceContainerLowest, primaryContainer (switch handle) |
| `surface-1` | surfaceContainer, surfaceContainerLow, surfaceDim |
| `surface-2` | surfaceContainerHigh, surfaceVariant, tertiaryContainer, errorContainer |
| `surface-3` | surfaceContainerHighest, secondaryContainer (selected segment) |
| `text-primary` | onSurface, onBackground, on*Container, shadow, scrim |
| `text-secondary` | outline, secondary, onSurfaceVariant |
| `border` | outlineVariant |
| `danger` | error, onErrorContainer |
| The same token from the other theme | inverseSurface, inverseOnSurface, inversePrimary |

outline maps to `text-secondary`, not `border`, because outline draws the off switch, the segmented buttons and the text field, and `border` is too faint to see on them.

Theme: the window follows the standard `Palette.color-scheme`, which `MaterialPalette` reads. `KAWARO_COLOR_SCHEME` wins first (as in the hive), then the theme setting (light or dark); auto follows the system. So the window is rebuilt on every `open`, and no forced theme survives a return to auto. Changing the theme inside the window does not restyle it before save.

### Design lint

The vendored library has hand written colours and lengths, so `scripts/design-lint.sh` skips `crates/ui/material/` for the colour and length checks only, matching the full path rather than the word `material` so our files are not hidden. Shadow, gradient and dash checks still run on the library, which proves the shadows are gone. An en dash check was added beside the em dash check.

## Consequences

- Font weights in `MaterialTypography` (300, 600) do not match our tokens (400, 500, 600); the bundled font picks the nearest weight. Changing them needs a fourth library change, deferred.
- The check mark in the selected segment stays left of the text in Arabic, because its order is inside the library button. The order of segments is fully mirrored.
- The window and the hive share one process and one event loop. The focus handler in `lib.rs` receives focus events for every window; on Wayland focus leaves the old window before entering the new one, and `present` clears the focus state, so the settings window should not close the hive. Not tested on a real compositor.
- Screenshots come from a private headless KWin; the window has not been opened on real GNOME or KDE.
