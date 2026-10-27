# S-089 First run and settings recovery
Status: draft, implementation and live verification pending.

## Story
As a new Kawaro user, I want to choose Arabic or English before the introduction, understand the real workflow from a useful video, and control settings in one reliable window, so I can use and trust the application.

## Evidence and limits
- Owner Windows report on 2026-10-27: the introduction was poor, the window appeared behind other applications, the app icon was missing, settings opened separately, and choosing Arabic broke the interface and did not update it immediately. The supplied screenshots show the separate windows and generic title bar icon. They do not establish the root causes.
- `backlog/S-087-first-run-welcome.md` deliberately substituted generated frames for video and records visual approval and live Windows verification as pending. That decision is superseded by the owner's request for a real embedded video. Avoid asserting that existing frames are video.
- `crates/ui/src/welcome.rs` and `crates/ui/src/settings.rs` fix language at construction. `crates/daemon/src/ui.rs` rebuilds windows asynchronously after a saved language change. Reproduce the reported break before identifying its cause.
- `backlog/S-085-windows-app-identity.md` specifies icon resources and `backlog/S-086-icon-opens-window.md` specifies foreground behavior. Their earlier automated results do not prove the installed 0.4.0 appearance or focus behavior. Check the shipped MSI and newly built installer separately.
- `Config` contains `notify` and `notify_long_only`. The current settings view exposes only `notify`. Changes to job timing and notification handling may already be in progress in the working tree. Inspect and preserve them, do not assume they were released or tested.
- Existing acceptance and test conventions are in `backlog/_TEMPLATE.md`, S-076, S-079, S-085, S-086 and S-087. Existing local modifications belong to the ongoing work and must not be discarded.

## Decisions and acceptance criteria
- AC1: A clean first run opens a bilingual language choice before any introduction. Arabic and English are visible in their own scripts, operable by mouse and keyboard, and select immediately. Persist the choice only after a successful save; a save error is visible and retryable. Existing installations retain their selected language and do not lose settings. Reopening the app does not repeat onboarding after completion; closing before completion resumes safely.
- AC2: One application window contains introduction and settings pages with labeled navigation icons. Selecting a page does not open a second top-level settings window. Tray and shortcut entry points focus the same window and select the intended page. Preserve the live sample flow and the original file.
- AC3: Introduction uses an actual bundled, locally playable video of the real workflow, with accurate platform-specific trigger wording, visible bilingual captions and accessible textual instructions. No network connection is needed. Provide play, pause and replay, with a static explanation when playback fails or reduced motion is enabled. Document source, license, generation and asset size. Evaluate supported Windows and Linux playback and binary size before choosing decoder and format. Do not silently ship the existing loop as a video or promise an unsupported codec.
- AC4: The first and subsequent explicit app launches show or raise one window in front of ordinary windows on Windows. Respect OS foreground restrictions and never force permanent always-on-top. Concurrent launches must not create duplicate windows or daemons. Autostart remains silent. Record actual outcomes for Windows focus rules and other desktops, rather than claiming unsupported focus guarantees.
- AC5: The correct branded icon appears in installed executable, Start shortcut, taskbar, title bar and Alt+Tab at normal and high DPI. Inspect resource and shortcut links plus real installed UI. Investigate runtime window identity if resources exist but the title bar is generic. Test upgrades and removal without deleting unrelated shortcuts.
- AC6: Changing language in the open window updates all visible labels, navigation, captions and RTL layout immediately without a crash, lost edits, duplicated window or restart. Both directions and system automatic language are supported. Failure to persist does not falsely claim success. Resolve the lifecycle and callback issue rather than hiding it with a restart.
- AC7: Notification choices are Off, All completed jobs, and Long jobs only. Long means at least 10 seconds of actual processing time, including a documented fallback when timing is unavailable. Preferences survive restart and migrate existing `notify` and `notify_long_only` values without changing behavior. Define how failures and multi-file batches are notified, keep notifications separate from in-app errors, and never take focus.
- AC8: Provide comprehensive customization for existing supported behavior, including language, theme, output location, quality and notifications, with clear labels, current values, reset and save or cancel semantics. Do not invent unsupported controls. Inventory remaining existing config fields and explain which are safe for ordinary users, with advanced controls only where behavior and validation exist. Preserve unknown configuration keys and prior values on save.
- AC9: Arabic and English, light and dark, scaling, keyboard navigation, screen reader labels, contrast, reduced motion, resize, small screens and long strings are checked visually and interactively. No technical implementation jargon in user-facing copy. Match design tokens and existing lint rules.

## Implementation slices and dependencies
1. Reproduce and isolate: record installed version, fresh versus existing profile, screenshots, logs, launch route, language change steps and result. Inspect MSI resources and window handle identity. Keep a separate test profile and do not change the user's files. Record findings before changing code.
2. Language and state model: define onboarding completion separately from language choice and existing `welcome_seen`; preserve existing installs. Introduce one source of truth for current language, direction, selected page and unsaved settings. Specify migration and persistence failure behavior in core tests before wiring UI.
3. Unified window: refactor `crates/ui/ui/welcome.slint`, `crates/ui/ui/settings.slint`, `crates/ui/src/welcome.rs`, `crates/ui/src/settings.rs` and `crates/daemon/src/ui.rs` around a single owner of the top-level window. Keep entry points and sample actions functional. Avoid stale callbacks, timer leaks and teardown of a window during its own callback. Implement immediate language updates and page navigation.
4. Video and accessibility: produce a genuine demonstration from an actual supported workflow, document asset origin and reproducible production, embed and decode it on supported platforms, and implement controls and fallback. Verify caption timing, sizes and licensing. If a platform cannot decode it, record the failure and use the accessible fallback while resolving support; do not mark AC3 complete.
5. Focus and icon: trace shortcut to daemon request and foreground grant, then inspect window icon and installer resources. Repair the discovered cause across executable, native window and shortcut as applicable. Validate a clean install and upgrade on Windows, preserving the daemon's silent startup.
6. Settings and notification policies: inventory `Config`, align visible controls to supported behavior, add migration-safe notification selection, apply timing policy at the job completion boundary and test failure and batch cases. Retain existing settings during unrelated edits and make save failures visible.
7. Verification and release: run targeted core, daemon and UI tests, snapshots and repository gates. Install the newly built MSI on Windows, perform the matrix below and record build hash, OS version, profile, screenshots and outcomes. Update bilingual guides, changelog, S-079 checklist and distribution status only with verified facts. Release only after real evidence and independent review.

## Test matrix
| Criterion | Scenario and expected result | Evidence |
| --- | --- | --- |
| AC1 | Fresh profile, pick each language, close before completion, reopen, save failure and retry; preexisting profile keeps all preferences | Core state and migration tests, Windows interactive run |
| AC2 | Navigate intro and settings, tray and shortcut while open or hidden, ten repeated launches; exactly one top-level app window | UI integration and Windows window count |
| AC3 | Offline playback, pause, replay, captions in both languages, reduced motion, missing or corrupt asset and unsupported decoder; instructions remain usable | UI tests and live Windows and Linux recordings |
| AC4 | Cold and warm launches behind Explorer and other applications, minimized window, repeated launch, silent login startup | Windows live focus observations with timestamps |
| AC5 | Clean install and upgrade; inspect exe icon resources and shortcut; observe Start, title, taskbar and Alt+Tab at 100% and 150% scaling | Windows resource tests, MSI inspection and screenshots |
| AC6 | Switch English to Arabic and back while on every page; change theme and other values before switching; save, cancel and reopen; verify no crash, immediate text and correct RTL | UI tests, accessibility tree and live Windows run |
| AC7 | Off, all and long policies at 9.999 and 10 seconds; absent timing, failed and mixed batches; restart and migration from each old boolean combination | Unit tests with fake clock and notification sink, live notification run |
| AC8 | Change each offered setting, save, cancel, reset, restart, invalid or unwritable config, unknown key preservation | Core serialization and UI integration tests |
| AC9 | Arabic and English in light and dark, long strings, narrow window, 100% and 150% scale, keyboard only, screen reader and reduced motion | Reviewed snapshots and manual accessibility checklist |
| Regression | Try sample, validate actual output decodes, original hash unchanged; real file conversion and Explorer integration still work | Integration test and Windows live run |

## Completion and review
- [ ] Every acceptance criterion has named test evidence and no unverified live or visual claim is presented as passing.
- [ ] Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --no-fail-fast`, `env KAWARO_WIN_SCOPE=1 scripts/windows/check.sh` and `scripts/design-lint.sh` using the repository resource limits. Record failures and environment restrictions.
- [ ] Check `git diff --check`, inspect all changed files and preserve unrelated working-tree changes.
- [ ] Independent reviewer checks behavior, migrations, accessibility and Windows evidence against this story. Record unresolved issues explicitly.

## Out of scope
Publishing or claiming store approval, changing file conversion semantics, and collecting personal data are not required by this story.

## Review log
- 2026-10-27: drafted from the owner report, current source and existing story records. No new test or visual approval is claimed by this draft.
