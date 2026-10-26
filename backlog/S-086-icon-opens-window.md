# S-086 Clicking the app icon opens a window instead of nothing
Status: approved under manager delegation (2026-10-26)

## Story
As a Windows user, I want clicking Kawaro in Start or search to open its Settings window at once, so that the app never looks dead or broken.

## Root cause (verified in the tree, 2026-10-26)
The MSI Start shortcut targets `kawarod.exe` with no arguments, which is daemon mode. The `Run` registry value already starts that daemon at every logon, so the shortcut's instance reaches `Bound::AlreadyRunning`, logs `kawarod: already running` and exits 0 with zero visible UI (`crates/daemon/src/main.rs`). Even a cold click only starts a background service with a tray icon. This is a design gap, not a security or SmartScreen effect. On Linux the desktop entry runs `kawaro settings` and opens the settings window (`packaging/io.github.m7mmad_omar.Kawaro.desktop`); Windows has no equivalent because `kawaro.exe` is a console binary and cannot be a shortcut target without a console flash.

## Files
`crates/daemon/src/args.rs`, `crates/daemon/src/args_tests.rs`, `crates/daemon/src/main.rs`, `installer/windows/kawaro.wxs`, `docs/windows-beta-checklist.md` (one live step), `crates/core/tests/s051_msi_registry.rs` (only if the shortcut command row is covered).

## Acceptance criteria
- AC1: `kawarod.exe settings` with a running daemon opens the existing Settings window through the same `Request::Settings` path as `kawaro settings` and exits 0; with no daemon running it starts the service and opens the window; an unknown argument still exits 2 with the message of today.
- AC2: the MSI Start shortcut runs `kawarod.exe settings`; clicking it shows a visible window within 1 s on the S-079 reference machine (the owner's Windows test machine, cold start included), no console window appears or flashes (`kawarod` keeps `windows_subsystem = "windows"`), and a second click re-focuses the same window instead of opening another. The re-focus mechanism is stated: `ui::open_settings` currently early-returns when the window exists (`crates/daemon/src/ui.rs`), so this story adds a raise on the existing Slint window (the Windows foreground path the hive already uses for `open`) instead of that silent return.
- AC3: autostart is unchanged: the `Run` value still starts the silent daemon with no window, and ten clicks of the shortcut leave exactly one window of whatever the `settings` entry currently opens (the S-086 Settings window today, the S-087 welcome window after S-087 lands) and one daemon process.
- AC4: `kawarod open <files>`, the Explorer menu command and the named-pipe request routing are unchanged; the args tests cover the new mode and the existing modes together.
- AC5: `docs/windows-beta-checklist.md` step W01 (which already ends with "open the Start menu entry") gains the click procedure in its Do and the 1 s observable expectation in its Expect; no new step is added and the S-079 D1 cap of 40 steps and 160 nonblank lines stays satisfied. One CHANGELOG line in both languages; no long dashes.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1, AC4 | args parsing tests for `settings`, `open` and an unknown argument; server routing tests with a fake listener proving `Request::Settings` reaches `ui::open_settings` | unit, integration |
| AC2, AC3 | checklist step text check; a Windows shell test asserting one window and one daemon after ten launches (live portion stays owner-run) | static, live checklist |
| AC5 | static checks | static |

## Out of scope
- What the opened window contains: S-087 makes the `settings` entry open the welcome window on the first run by adding a routing branch inside the same request path, not by retargeting the shortcut; the shortcut command of this story is unchanged by S-087 and every AC here stays valid.
- First-run auto-popup after install (the hive must never appear without a real invocation, and focus stealing is the top Wayland risk; the window appears only from a user click).
- The tray menu, which already offers Settings.

## Review log
- 2026-10-26, planner: written from the owner Windows beta report; three goldfish reviews (design, readiness, zoom-out) applied to the text.
