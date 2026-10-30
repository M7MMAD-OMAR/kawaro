# Kawaro (كوارو): a local, open source file converter with a honeycomb UI

The name is fixed: Kawaro in Latin script, «كوارو» in Arabic (from «كوّارة», an old Arabic word for a beehive; `docs/adr/0008-name-kawaro.md`). The logo is a saguaro cactus built from many hive cells on the honeycomb lattice (`design/brand/kawaro-cells.svg`); the single hexagon with the cactus cut out is only the app icon and favicon. The name changed to Kawaro, the mark did not (ADR 0008). Releases and the plan are in `docs/PLAN.md`, changes in `CHANGELOG.md`.

## What it is

A free GPLv3 tool that converts and processes files entirely on the user's machine. The user selects one or more files in the file manager, a hexagonal hive shows the actions that fit their type and count, and the result is saved as a new file next to the original.

Platform order: Linux on Wayland first (GNOME and KDE), then X11, then Windows, then macOS. GNOME, Fedora and Ubuntu default to Wayland and GNOME's X11 session is gone from current distributions. Hyprland is more permissive than GNOME and KDE, so testing on it alone is not enough: anything about windows and focus must also be tested on GNOME and KDE.

## Non-negotiable principles

- No file leaves the user's machine. No processing servers, no telemetry.
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

## Status (2026-10-30)

- Released and tagged on GitHub: 0.1.0, 0.2.0 to 0.2.2, 0.3.0, 0.3.1 (S-021), 0.3.2 (S-022), 0.3.3 (S-014e, S-027), 0.4.0-beta.1, **0.4.0** (the first official release, 2026-10-27) 0.4.1 (S-094, S-090, the simpler website and README) 0.4.2 (urgent Windows fix S-097) 0.4.3 (cut 2026-10-30: the MSI upgrade fix S-100, the update experience S-099 and S-101, the Alt and late drag fix S-095, the Welcome film S-094c; known limits: the settings window can open behind another window after an update on Windows, S-102 stray hive on Alt chords on Hyprland). and **0.4.4** (cut 2026-10-30: Alt chords and the `kawaro doctor` drop key line S-102, Hyprland 0.57 support S-104, the signed apt repository and the Scoop bucket S-103; known limits: on Hyprland 0.57 `kawaro drop` without `--at` can swallow the first click S-105, the settings window can open behind another window after an update on Windows, the Windows build is self-signed). RPM and DEB packages are on the release pages from 0.2.1 on.
- Windows (ADR 0005) and in-app updates (ADR 0006): S-045 to S-055 are done on main, with S-047 (drop hive), S-049 (notifications), S-054c (`kawaro update`) and S-067 (their review fixes). The 0.4.0 release carries the Windows build, installer and Explorer menu (S-045 to S-051), the beta feedback block (S-079 to S-088) and the portal shortcut (S-014e, S-014f). The update signing key is in the GitHub environment `release-signing`. The 0.4.0 Windows binaries are test-signed with a self-signed certificate, so SmartScreen warns (More info, then Run anyway); public signing still needs the maintainer's OV certificate. `scripts/windows/sign-local.sh` signs a release with the self-signed test certificate (key in `~/.config/kawaro-signing/windows/`).
- The owner ran the published 0.4.0 MSI in a Windows 11 virtual machine on 2026-10-27: the welcome window appeared, the sample image action produced a notification, and the settings window opened. Every other Windows result is owner pending until the S-079 checklist report exists.
- The **WinGet** submission for `Sbarah.Kawaro` 0.4.0 is open and ready for review at [microsoft/winget-pkgs#462805](https://github.com/microsoft/winget-pkgs/pull/462805), with all twelve automated checks passing. The only remaining gate is one approving review from a Microsoft moderator. Details in `docs/distribution-status.md` and `docs/winget-pkgs-merge-requirements.md`.
- The SignPath Foundation **declined** the application on 2026-10-27, because the program needs public visibility the project does not have yet, not because of the project quality. The free path to a trusted publisher name is therefore the Microsoft Store, where Microsoft re-signs the package; the individual developer registration is not complete yet, because the identity verification rejected the name mismatch between the Arabic identity document and the Latin account name. See `docs/signpath-application.md` and `docs/distribution-status.md`.
- Next after 0.4.0: the portal follow-ups S-014f1 to S-014f4 are done. Remaining: signed repositories (S-016) and the glass hive (S-017), plus the S-072 to S-082 block: S-074 done (no hive plate), S-075 re-baselined (Alt-drag out of the box on a per-desktop measured contract, the maintainer's top priority), S-076b approved and pending, S-077 approved, S-078 in progress (the winget half is submitted, AppStream and AUR are not), and S-080 and S-081 done (the two confirmed Windows defects from S-079). Then 0.5.0 for X11 (S-015) and 0.6.0 for Windows. Details in `docs/PLAN.md`.
- Measured 2026-10-26: Alt-only is not a usable GlobalShortcuts trigger on GNOME 50 and KWin 6.7.5. The spike disproved it; Super+Z passes on both. S-075 registers Alt only through measured native per-desktop paths, with the portal as a named non-Alt fallback. No story may claim Alt through the portal.
- The review of everything since 0.3.3 is done (S-028 fixes, S-029 cleanup, S-030 one process tree for every engine child). The next free story number is **S-104**.
- Still open from phase 0: the focus and activation test on real GNOME and KDE, with tokens from Nautilus and Dolphin (needs a virtual machine). It is owned by S-082 and is not closed by private compositor evidence.

## Working rules

- Write an ADR before any major architectural decision.
- Tests for every engine adapter, with small test files in `tests/fixtures`.
- Check the licence of any new library before adding it and record it in `docs/engines.md`.
- After every push to main, read `scripts/ci-status.sh --wait`. Red CI blocks the next story.

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

### Mutation testing

Not a story gate (decision 2026-10-30: it cost hours per story). Run it outside the chain, nightly or weekly, on changed code only, with `--re` to limit mutants and `--test-tool=nextest` if it measures faster. Survivors become fix tasks and never block a push.

### Running heavy work

- Builds, tests, mutation runs and compositor scripts run inside `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=100%` with `nice -n 15`, `CARGO_BUILD_JOBS=1`, `cargo test -- --test-threads=2`, one cargo job at a time, and temp files under `$HOME` (never fill the `/tmp` tmpfs, it is small). On 2026-10-21 systemd-oomd killed the whole Claude app over mutant builds, and on 2026-10-30 the owner said the machine must never be loaded that hard again. Rust stories run one after another, never in parallel worktrees (each would build its own 70 GB `target/`).
- Put the commands in a script file and run that, because systemd-run substitutes `$` in its arguments.
- `cargo mutants` always runs with `--in-place`, on the changed file only. Copy mode copies the 76 GB `target/` into tmpfs.
- GUI tests run only in private compositors (`tests/hyprland/repro.sh`, the KWin scripts such as `tests/overlay/kwin.sh`), never on the live desktop.
