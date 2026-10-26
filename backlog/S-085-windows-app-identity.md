# S-085 Windows app identity: the icon everywhere and the name Kawaro | كوارو
Status: approved under manager delegation (2026-10-26)

## Story
As a Windows user, I want the app to show its cactus icon and the name "Kawaro | كوارو" wherever the system shows an app, so that I can find and recognize Kawaro in Start, in the taskbar and in the apps list.

## Current state (verified in the tree, 2026-10-26)
- `design/brand/kawaro.ico` exists (16, 32 and 48 px, checked by `crates/daemon/tests/s048_sources.rs`) and the MSI installs it next to the exes; the Explorer menu Icon and the AUMID IconUri point at it. That is why the right click menu has an icon.
- Neither `kawaro.exe` nor `kawarod.exe` embeds an icon resource: there is no Windows resource step in any `build.rs`. Windows therefore draws the generic executable icon in Start search, in the shortcut and in Alt+Tab. This is the owner's "there is no icon at all" report.
- The MSI Start shortcut (`installer/windows/kawaro.wxs`, `StartShortcut`) has no `Icon` attribute and no `Icon` element, and its name is "Kawaro" only. The AUMID DisplayName in both the wxs and `core::registry::install_commands` is "Kawaro", so toasts say "Kawaro" only.
- The adopted bilingual name is "Kawaro | كوارو" (ADR 0008; release titles already read `Kawaro X | كوارو X`). The goal is Arabic and English together on every OS-level surface.

## Files
`crates/cli/build.rs`, `crates/daemon/build.rs` (new), `crates/cli/Cargo.toml`, `crates/daemon/Cargo.toml`, `installer/windows/kawaro.wxs`, `crates/core/src/registry.rs`, `crates/core/tests/s051_msi_registry.rs` (parity follows the rows), `docs/engines.md`, `scripts/windows/make-msi.sh` (only if the icon flow needs it).

## Acceptance criteria
- AC1: both shipped exes carry an embedded Windows icon resource built from `design/brand/kawaro.ico` with 16, 32 and 48 px images, and version info whose ProductName and FileDescription read "Kawaro | كوارو". A test in the windows-cross job lists the icon resources of both built exes and fails when the resource is missing.
- AC2: the MSI Start menu entry shows that icon and its name is "Kawaro | كوارو", proved by a test that exports the MSI Shortcut table (the s051 pattern with `msiinfo export`) and checks the name and the icon reference.
- AC3: the AUMID DisplayName is "Kawaro | كوارو" in both `installer/windows/kawaro.wxs` and `core::registry::install_commands`, the MSI Product name is "Kawaro | كوارو", and the s051 registry parity test still passes, proving the two sources cannot drift. The `UpgradeCode` of `kawaro.wxs` is byte-for-byte unchanged (it is what makes a newer package replace an older one), a static test pins its value, and the owner checklist keeps its MSI upgrade step to prove a rename never strands an existing install.
- AC4: the Explorer menu verb stays language-localized exactly as today (Kawaro in English, كوارو in Arabic), the tray icon still loads and the tray tooltip stays localized; no behavior of S-048 changes.
- AC5: the build dependency added for the icon and version resource has its licence recorded in `docs/engines.md`; `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `scripts/design-lint.sh` pass; one CHANGELOG line in both languages.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | windows-cross script lists icon resources of `kawaro.exe` and `kawarod.exe` and reads ProductName from the version info | integration |
| AC2, AC3 | MSI Shortcut and Registry table export tests in the s051 pattern; registry parity test | integration, unit |
| AC4 | existing S-048 tray and registry tests unchanged and green | integration |
| AC5 | static gates and the engines licence note | static |

## Out of scope
- The Linux icon theme and desktop file names (already correct), macOS.
- Code signing of these exes (S-083) and anything that changes the mark itself (ADR 0008 fixed it).
- Live "looks right in Start" confirmation, which belongs to the S-079 checklist, not to a test here.
