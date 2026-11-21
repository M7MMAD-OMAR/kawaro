# Kawaro (كوارو): a local, open source file converter with a honeycomb UI

The name is fixed: Kawaro in Latin script, «كوارو» in Arabic (from «كوّارة», an old Arabic word for a beehive; `docs/adr/0008-name-kawaro.md`). The logo is a saguaro cactus built from many hive cells on the honeycomb lattice (`design/brand/kawaro-cells.svg`); the single hexagon with the cactus cut out is only the app icon and favicon. The name changed to Kawaro, the mark did not (ADR 0008). Releases and the plan are in `docs/PLAN.md`, changes in `CHANGELOG.md`.

## What it is

A free GPLv3 tool that converts and processes files entirely on the user's machine. The user selects one or more files in the file manager, a hexagonal hive shows the actions that fit their type and count, and the result is saved as a new file next to the original.

Platform order: Linux on Wayland first (GNOME and KDE), then X11, then Windows, then macOS. GNOME, Fedora and Ubuntu default to Wayland and GNOME's X11 session is gone from current distributions. Hyprland is more permissive than GNOME and KDE, so testing on it alone is not enough: anything about windows and focus must also be tested on GNOME and KDE.

## Non-negotiable principles

- No file leaves the user's machine. No processing servers, and no telemetry in the app (the website only counts visits in aggregate, without cookies; `PRIVACY.md`).
- The hive appears less than 150 ms after it is invoked. On the long press path (`kawaro drop --hold`) "invoked" means the `hold_ms` threshold has elapsed, not the key press.
- The base install is light. Extra engines are downloaded only when needed, or taken from the system when present.
- The original file is never modified. Write to a temporary file, then rename atomically to the final name.
- Arabic and English from day one, with full RTL.

## Language policy

- English is the primary language of all repo text: code, comments, internal docs, ADRs, commit messages.
- Public user docs have an English main file and an Arabic copy beside it (`NAME.md` and `NAME.ar.md`). Each starts with the language switch line `[English](NAME.md) | [العربية](NAME.ar.md)`.
- Backlog stories may stay in Arabic. New stories and new log entries are written in English.
- `CHANGELOG.md`: one line per story, in English and Arabic like the existing entries.
- App UI strings stay in Arabic and English, with RTL.

## Stack

- **Language:** Rust, a multi crate workspace.
- **UI:** Slint (GPLv3) with the software renderer (`docs/adr/0001-ui-toolkit.md`). The Slint version is pinned; upgrading it is a separate task.
- **IPC:** Unix socket on Linux, named pipe on Windows.
- **Heavy engines run as separate processes** (FFmpeg, LibreOffice, Ghostscript).

## Repository layout

```
crates/
  core/      job model, format and action registry, queue, cancel and progress
             (device and engine probing in core::probe until it grows into a crate)
  engines/   one adapter per engine behind a shared trait
  daemon/    background service, tray icon, request handling
  ui/        the hive, the settings window (Slint); hand built parts in components/
  cli/       command line, uses core directly
actions/     action definitions in TOML; the hive is built from them at runtime
integrations/linux/{nautilus,dolphin,nemo,thunar,hyprland}/
integrations/windows/{explorer-command,sparse-package}/   planned for 0.6.0
design/tokens.json   the single source of truth for design
docs/        PLAN.md, architecture.md, design-system.md, engines.md, adr/
backlog/     one story file per task, from backlog/_TEMPLATE.md
tests/       end to end and compositor tests, small fixtures in tests/fixtures
spikes/      throwaway feasibility experiments, outside the workspace, never merged as is
site/        landing page
```

## Engine contract

Every engine implements one trait:

- `capabilities()`: (input type, output format) pairs with the options they support.
- `estimate(file, device)`: expected time and memory.
- `run(job, progress, cancel)`: execution with progress and cancellation.

A new format is a TOML file in `actions/` plus a small adapter. Never put action logic in the UI.

## Design rules (mandatory)

- No `box-shadow` and no shadows of any kind.
- No gradients.
- No one sided borders and no side accent lines.
- No em dash or en dash in any text, comment or doc. Use a comma, colon, semicolon or a new sentence.
- Elevation through surface tokens only: `surface-0` to `surface-3`.
- Transparency only from the glass tokens (`glass-2`, `glass-3`) and only over a blur the compositor draws. Where there is no blur (GNOME) surfaces stay opaque.
- Every colour, spacing, radius and motion duration comes from `design/tokens.json`. No hand written values.
- Cells are pointy top hexagons in axial coordinates, growing ring by ring.
- A cell shows an icon with the format code (WEBP, PDF). The full Arabic or English name shows in the centre cell on hover, never inside the small cell.
- Respect the system's reduced motion setting.

## UI

- Components come from Slint's official Material 3 library, vendored at v1.18.1 in `crates/ui/material/` (MIT, `docs/adr/0002-settings-components.md`). Colours and fonts are set from tokens only. Never rebuild a component the library has.
- The only hand built components are the hexagonal cell and the centre cell, in `crates/ui/components/`.
- Colours are passed once to `MaterialPalette.schemes` (light and dark) under Material Theme Builder field names, generated from `tokens.json`.
- Shadows live only in the library's `elevation.slint`, which we keep as a modified copy without shadows. Do not use elevation levels to show height; use surface tokens.
- RTL: Slint shapes Arabic and handles bidi text. Layout mirroring is not automatic: reverse element order by UI language.
- Do not change the UI library without an ADR.
- No work blocks the UI. Job progress shows in the tray icon tooltip with one notification when each job finishes (`docs/PLAN.md`, decision of 2026-10-20), never in a window that takes focus.

## Wayland and Windows: facts to respect

- Wayland gives no window positioning and no pointer position outside our surfaces. On the layer path (Hyprland, KDE, wlroots) the hive appears at the pointer or the drag position and its shape adapts to the screen edges. On the window path (GNOME, or a running screen reader) it is centred. On X11 and Windows it appears at the pointer.
- **Focus stealing prevention is the top risk.** A window opened by a background service does not get focus by default on GNOME and KDE, and an "app is ready" notification may show instead. The fix: the file manager integration requests an xdg-activation token and passes it with the IPC request, and the service activates the hive with it. The hive does not count as working on a compositor where focus was not tested.
- The hive is a `zwlr_layer_shell_v1` surface where the compositor offers it (Hyprland, KDE Plasma since 5.20, Sway and wlroots), and an ordinary window on GNOME, X11 and Windows, and when a screen reader is actually running (not the flag alone). Never assume layer-shell exists (`docs/adr/0004-hive-overlay.md`).
- The global shortcut on Wayland goes through the xdg-desktop-portal GlobalShortcuts portal, reading copied files from the clipboard.
- The guaranteed entry point is the file manager's right click menu.
- Windows 11: the modern menu needs IExplorerCommand and a signed sparse package. The legacy menu through registry keys works unsigned.

## Active programme (2026-11-15)

- The owner explicitly authorized full source implementation on 2026-11-15; build/test/runtime/CI evidence remains deferred. The full scope is in `docs/specs/2026-11-15-next-features.md`, current evidence in `docs/research/2026-11-15-next-features-evidence.md`, and exact implementation packets in `docs/superpowers/plans/2026-11-15-next-features.md`.
- Order: S-142 transformed image preview; S-143 named user presets; S-144 batch outcomes and safe retry; S-145 unavailable-action explanations; S-146 optional local image OCR; S-147 searchable-PDF feasibility only. On 2026-11-21 S-142 to S-150 reached source implemented and parent-reviewed (executors MiniMax-M3.1-Flash and Gemini 4 Neon); ADR 0017 accepted. The owner then approved compile checks and the Linux and Windows cross test stages on the second machine; visual, compositor, real desktop and physical Windows checks are not claimed. Records: `docs/development/2026-11-15-execution-status.md`, `docs/development/2026-11-21-review-findings.md`. Next free story: **S-151**.
- No builds, tests, benchmarks, compositors, installers, model inference, factory or CI during planning. Future focused cases are designs, not executed evidence. Full release gates stay deferred; no automatic tagging/publication.
- Other-system adapters and qualification come after stable feature behavior. Keep portable core boundaries now, preserve existing source, and make no new GNOME/KDE/Windows/X11/macOS acceptance claim. S-138 remains draft; S-141 remains separate publication work. Public release 0.5.0, candidate source 0.5.1.
- Selected research/implementation agent: Pi/MiniMax-M3.1-Flash. Parent owns plans, decisions and review. Old research baselines may predate implemented options, crop and MCP; do not reimplement them from historical prose.

## Status (2026-11-05)

- **Current release: 0.5.0** (tag v0.5.0, 2026-10-31). The first official release was 0.4.0 (2026-10-27); every release has a tag, packages and a `CHANGELOG.md` entry. RPM and DEB packages are on the release pages from 0.2.1 on.
- **Shipped since 0.4.3:** X11 support (S-113 hive window at the pointer, S-114 private session checks, S-115 drop cell, S-116 placement; checked only in a private virtual X session with xfwm4), Alt chords and the `kawaro doctor` drop key line (S-102), Hyprland 0.57 support (S-104) and its first click fix (S-105), the hive explains why a file has no actions (S-096, S-108), the About page actions (S-094b), one notification choice (S-109), `kawaro open --at` (S-110), `kawaro doctor --repair` and the generated Hyprland bind (S-111), AppStream metadata and one install command source (S-112; the signed checksums job ran on the 0.4.8 and 0.5.0 tags), Windows foreground, cleanup and names (S-106, S-107, checked in a Windows 11 virtual machine), the distribution pipeline (S-103), and a flaky update poll test fixed in test code.
- **Not checked:** X11 on a real GNOME or KDE X11 desktop (only a virtual X server with xfwm4 was used), Shift as a move request reaching the drag source and Nautilus or Dolphin as X11 drag sources, the S-094b real clipboard checks on desktops, the S-110 and S-111 nested Hyprland and live session checks, the S-108 manual checks M1 to M3 (screenshot, 150 ms timing, Windows Explorer mixed selection), the S-105 KWin scripts and drop budget script, physical Windows hardware, and the focus and activation test on real GNOME and KDE (S-082, needs a virtual machine; private compositor evidence does not close it).
- **Distribution:** apt and Scoop publish on every tag. WinGet [microsoft/winget-pkgs#462805](https://github.com/microsoft/winget-pkgs/pull/462805) passed all automated checks and the CLA is signed; it waits since 2026-10-27 for a volunteer moderator, and the winget channel job stays off until the package exists. AUR and COPR need an account only a person can create (`docs/distribution-channels/aur.md`, `copr.md`), so they stay skipped. Details: `docs/distribution-status.md`.
- **Signing:** the maintainer has no code signing certificate and no identity document usable for the Microsoft Store verification. The SignPath Foundation declined on 2026-10-27 for lack of public visibility. The Windows build stays self-signed (SmartScreen warns); the Store, SignPath and a paid certificate are parked, not planned. `scripts/windows/sign-local.sh` signs with the self-signed test certificate.
- **Consolidation pass (2026-11-15):** every non-`main` branch and every extra worktree was consolidated into `main`. Implementation was done by the selected Pi implementation agent (`pi/minimax/MiniMax-M3.1-Flash`); the parent agent owned the planning and the final review. No software factory job was used. The 0.5.1 candidate source, the restored spike sources (`spikes/alt-portal/`, `spikes/gnome-alt-extension/`) and the historical goldfish review were preserved and pushed to `main` with a `[skip ci]` commit marker. The full local gate set and the remote CI were deferred by owner order for this one consolidation push, so no gate result is claimed for it. The public release stays **0.5.0**; the 0.5.1 tree stays a candidate with no tag, no packages and no channel publication. Evidence: `docs/maintenance/2026-11-15-consolidation.md`.
- **0.5.1 candidate (S-141):** the release cut includes accepted S-117 through S-140 except S-138. Typed options, customization with a source-draft preview (not transformed output), local opt-in/read-only-first MCP, PDF/image operations and drag-session fixes are included. S-139/S-140 source and the CI repair in PR #11 are public on main `c0444b0858204370f40478eaec3726b612264ace`; all three CI jobs passed in run `42058502635` (Lead-confirmed). Current public release remains **0.5.0** until tagging and publication verification. Candidate metadata does not certify a 0.5.1 tag, packages, signatures, feeds or channels. Lead owns fresh version snapshots, four public General captures, full Linux and both complete Windows stages including Release DLL/resources, and fresh review before a candidate PR push; exact public main CI must be green before tagging. Historical S-139/S-140 private native layer/window and installed Release evidence remains scoped to those reviewed builds, not new-version acceptance. S-138 remains reserved DRAFT [#9](https://github.com/M7MMAD-OMAR/kawaro/issues/9) for whole-output/final-position coverage. Real GNOME/KDE focus stays the top open risk [#1](https://github.com/M7MMAD-OMAR/kawaro/issues/1), not closed by this release. Fresh show/preview timing, IME, physical hotplug, legacy Hyprlang runtime, separate compositor Closed injection, the new Windows UI runtime and physical Windows remain unverified; Windows stays self-signed with SmartScreen warnings, macOS unsupported. Next free story: **S-151**; 0.6.0 remains reserved for Windows native work.
- **Remaining:** S-082, a real desktop X11 check, 0.6.0 Windows native work, S-078 (AUR half; AppStream shipped in S-112), S-016 and S-017 (glass hive), S-077b (KDE and GNOME pointer source), S-075b (native KDE and GNOME Alt paths). The blocked items are listed in `docs/PLAN.md` under "Blocked on the owner or outside resources". Details in `docs/PLAN.md`. The next free story number is **S-151**; S-138 remains reserved DRAFT.
- Measured 2026-10-26: Alt-only is not a usable GlobalShortcuts trigger on GNOME 50 and KWin 6.7.5; Super+Z passes on both. No story may claim Alt through the portal.

## Working rules

- Write an ADR before any major architectural decision.
- Tests for every engine adapter, with small test files in `tests/fixtures`.
- Check the licence of any new library before adding it and record it in `docs/engines.md`.
- After every push to main, read `scripts/ci-status.sh --wait`. Red CI blocks the next story.
- At every version bump run `crates/ui/tests/settings_snapshots.sh --update` (the header label follows the workspace version), then recapture `docs/screenshots/s094/A-general-*-default.png` and refresh the hashes in `docs/launch/assets.md`; `scripts/release-gates.sh` fails on a stale label.

## Agent chain (mandatory for every task)

All work in `crates/` goes through this chain, one agent per step in `.claude/agents/`. Spikes in `spikes/` are exempt; their result is an ADR, not merged code.

1. **planner:** writes the story in `backlog/` from `backlog/_TEMPLATE.md`.
2. **tester:** writes tests from the acceptance criteria only, runs them and shows they fail.
3. **implementer:** writes code until the tests pass. Editing tests is forbidden.
4. **reviewer:** in a fresh context, runs the quality gates and returns PASS or REWORK.

The maintainer has delegated approvals (stories, screenshots) to the manager, so the chain does not stop for approval. After two REWORKs the story stops and goes to the maintainer.

### Quality gates

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all
scripts/design-lint.sh                       # no hand written colours, shadows, gradients or long dashes
```

Plus: screenshots match, the performance budget holds, UI strings in Arabic and English, a descriptive label on every interactive element, and one line in `CHANGELOG.md`.

### Fast loop (owner decision 2026-10-30)

Do not rerun the whole suite at every development step. Locally run only fmt, clippy and the tests of the touched crates or files; CI runs the full suite after the push, and a red CI is fixed first. The reviewer follows the same scope.

Publication and CI-repair exception (owner decision 2026-11-05): no further partial main/publication pushes. First pass the complete local Linux CI equivalent, both complete Windows cross stages, and fresh review, serialized under the resource limits below. Keep every real test and assertion active; fix platform availability and resource-identity checks rather than suppressing warnings or skipping failures. Prefer a checked candidate PR before merging to main, then require green CI on the exact main commit before creating a release tag. The old scripts/release-gates.sh mutation/copy and parallel-job procedure is not an authorized gate; mutation is not a release requirement. Record external infrastructure failures honestly instead of promising they cannot occur.

### Mutation testing

Not a story gate (decision 2026-10-30: it cost hours per story). Run it outside the chain, nightly or weekly, on changed code only, with `--re` to limit mutants and `--test-tool=nextest` if it measures faster. Survivors become fix tasks and never block a push.

### Running heavy work

- Builds, tests, mutation runs and compositor scripts run inside `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=100%` with `nice -n 15`, `CARGO_BUILD_JOBS=1`, `cargo test -- --test-threads=2`, one cargo job at a time, and temp files under `$HOME` (never fill the `/tmp` tmpfs, it is small). On 2026-10-21 systemd-oomd killed the whole Claude app over mutant builds, and on 2026-10-30 the owner said the machine must never be loaded that hard again. Rust stories run one after another, never in parallel worktrees (each would build its own 70 GB `target/`).
- Put the commands in a script file and run that, because systemd-run substitutes `$` in its arguments.
- `cargo mutants` always runs with `--in-place`, on the changed file only. Copy mode copies the 76 GB `target/` into tmpfs.
- GUI tests run only in private compositors (`tests/hyprland/repro.sh`, the KWin scripts such as `tests/overlay/kwin.sh`), never on the live desktop.
