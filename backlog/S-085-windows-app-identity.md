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
`crates/cli/build.rs`, `crates/daemon/build.rs` (new), `crates/cli/Cargo.toml`, `crates/daemon/Cargo.toml`, `crates/daemon/src/` (the identity sync of D1), `crates/core/src/registry.rs`, `installer/windows/kawaro.wxs` (ASCII strings only), `crates/core/tests/s051_msi_registry.rs`, `tests/windows/test_s051.sh` (the D2 test updates), `docs/engines.md`, `scripts/windows/make-msi.sh` (only if the icon flow needs it).

## Acceptance criteria
- AC1: both shipped exes carry an embedded Windows icon resource built from `design/brand/kawaro.ico` with 16, 32, 48 and 256 px images (256 for the taskbar and Alt+Tab at 150% scaling), and version info whose ProductName and FileDescription read "Kawaro | كوارو" (the resource compiler, not wixl, carries these strings). A test in the windows-cross job lists the icon resources of both built exes and fails when the resource is missing.
- AC2: the Start menu entry shows that icon and its name is "Kawaro | كوارو". Measured constraint (2026-10-26): wixl segfaults on any non-ASCII string in the wxs source, including XML numeric entities (msitools 0.106.58, the fedora:44 line CI installs), so every MSI table string stays ASCII. The bilingual name is therefore applied after install by the daemon's identity sync (D1): it renames the Start shortcut `.lnk` and writes the AUMID DisplayName through Unicode-safe Windows calls. A test proves the rename and registry output bytes; the owner checklist step observes the name in Start.
- AC3: `core::registry::install_commands` writes the same bilingual DisplayName for the portable path (its `reg.exe` calls already carry Arabic for the menu verb today), and the s051 parity test compares the MSI Registry table against an ASCII projection of `install_commands` (D2). The `UpgradeCode` of `kawaro.wxs` is byte-for-byte unchanged (it is what makes a newer package replace an older one), a static test pins its value, and the owner checklist keeps its MSI upgrade step to prove a rename never strands an existing install.
- AC4: the Explorer menu verb stays language-localized exactly as today (Kawaro in English, كوارو in Arabic), the tray icon still loads and the tray tooltip stays localized; no behavior of S-048 changes. The bilingual compound name is a single fixed string "Kawaro | كوارو" with the Latin part first in every language (it is a brand string like the release titles `Kawaro X | كوارو X`, not a sentence, so it is never reordered per locale); the story records this policy in one line of the user docs.
- AC5: the build dependency added for the icon and version resource has its licence recorded in `docs/engines.md`; `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings` and `scripts/design-lint.sh` pass (the same commands as the CLAUDE.md quality gates); one CHANGELOG line in both languages.

## Manager decisions for the tester
- D1: the bilingual name cannot live in the MSI tables (wixl segfaults, measured 2026-10-26). The daemon applies it on start and on `kawaro install` through one idempotent identity sync: rename the Start shortcut `.lnk` to "Kawaro | كوارو.lnk" if it exists under the user's Start Menu Programs folder, and write the AUMID DisplayName registry value. The MSI tables keep ASCII "Kawaro" everywhere. The Add/Remove Programs name keeps the MSI Product name unless a measurement shows a Unicode-safe way to change it; the limitation is documented in the user docs, never hidden.
- D2: the s051 parity test keeps guarding the MSI Registry rows, projected against the ASCII output of `install_commands` (the tester updates the two `"Kawaro"` literals in `crates/core/tests/s051_msi_registry.rs` and the shortcut-name match in `tests/windows/test_s051.sh` as mechanical test updates; the implementer may not edit tests). A second test covers the identity sync output (the bilingual bytes).

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | windows-cross script lists icon resources of `kawaro.exe` and `kawarod.exe` and reads ProductName from the version info | integration |
| AC2, AC3 | identity-sync tests (the bilingual `.lnk` name and AUMID bytes) on the Windows path; MSI Shortcut table export proves the ASCII name and icon reference; the s051 registry parity test against the ASCII projection; the UpgradeCode pin test | unit, integration |
| AC4 | existing S-048 tray and registry tests unchanged and green | integration |
| AC5 | static gates and the engines licence note | static |

## Out of scope
- The Linux icon theme and desktop file names (already correct), macOS.
- Code signing of these exes (S-083) and anything that changes the mark itself (ADR 0008 fixed it).
- Live "looks right in Start" confirmation, which belongs to the S-079 checklist, not to a test here.

## Review log
- 2026-10-26, planner: written from the owner Windows beta report; three goldfish reviews (design, readiness, zoom-out) applied to the text.
