# S-048 Tray icon, menu in Arabic and English, and the global shortcut on Windows
Status: approved (manager, by delegation, 2026-10-24)

Builds on ADR 0005 section 5 (shell thread, tray, shortcut) and on S-045 (empty tray module). For the Windows beta. Pure logic is in `crates/daemon/src/shell_logic.rs` and `crates/core/src/hotkey.rs` (Linux tests); Win32 is in `crates/daemon/src/tray/windows.rs` and `shell.rs`, excluded from mutation. COM is not initialised on the main thread. One work day or less.

## Story
As a Windows user, I want a tray icon that shows job progress and offers Cancel all, Settings and Quit, and a shortcut that opens the hive, so that I can control SbarHive without a window.

## Files
`crates/core/src/hotkey.rs`, `crates/core/src/config.rs` (field `shortcut`), `crates/daemon/src/{shell_logic.rs,shell.rs,tray/windows.rs}`, `design/brand/sbarhive.ico`, `scripts/windows/make-ico.sh`, `.cargo/mutants.toml`, `CHANGELOG.md`.

## Acceptance criteria
- AC1: `hotkey::parse(&str) -> Result<Hotkey, String>` is pure, with `Hotkey { mods: u32, vk: u32 }` and Win32 values `MOD_ALT=1, MOD_CONTROL=2, MOD_SHIFT=4, MOD_WIN=8`. `"Ctrl+Alt+H"` gives `mods 3, vk 0x48`; `"shift+f9"` gives `mods 4, vk 0x78` (case-insensitive); `"Ctrl+Alt+7"` gives `vk 0x37`; `"H"` (no modifier) gives `Err`; `"Ctrl+Alt+"` and `"Ctrl+Alt+Nope"` give `Err`. The default string in `Config` is `"Ctrl+Alt+H"`. Constants are pinned against `windows_sys` at compile time for the target.
- AC2: `shell_logic::tooltip(active: &[(String, f32)], lang: Lang) -> String` is pure and never exceeds 127 UTF-16 units. Idle gives `text::idle(lang)`. Two jobs `("a.png", 0.5)`, `("b.pdf", 0.25)` give two lines `a.png: 50%` and `b.pdf: 25%`. Twenty jobs with 30-character labels give a string of at most 127 UTF-16 units that ends with `…`, and an Arabic label is never cut inside a surrogate pair (a test with an astral character at the cut point).
- AC3: `shell_logic::menu(lang, active_count) -> Vec<Item>` gives the three actions of the Linux tray in order: Cancel all (disabled when `active_count == 0`), Settings, a separator, Quit, with labels from `text::cancel_all`, `text::settings`, `text::quit` for Arabic and English. `shell_logic::menu_right_to_left(lang) -> bool` is `true` for Arabic only (the Windows code sets `MFT_RIGHTORDER` from it).
- AC4: `shell_logic::HoldPoll` is pure. After `press()`, `tick(key_down: bool) -> Poll`: `tick(true)` repeatedly gives `Poll::Held`; the first `tick(false)` gives `Poll::Released`, and every later tick gives `Poll::Idle`. The poll interval constant `POLL_MS` is `16`.
- AC5: the shell thread owns a hidden top-level window (not `HWND_MESSAGE`), registers `RegisterHotKey` with `MOD_NOREPEAT (0x4000)` OR the parsed mods, and re-adds the tray icon when it receives the registered `TaskbarCreated` message. A text test over `shell.rs` finds `RegisterHotKey`, `TaskbarCreated` and `Shell_NotifyIconW`, and no `HWND_MESSAGE`. On `WM_HOTKEY` the daemon behaves as `Drop { hold: true }` and polls `GetAsyncKeyState` through `HoldPoll`, then as `DropRelease`.
- AC6: `design/brand/sbarhive.ico` exists, `file` reports `MS Windows icon resource` with at least 3 images, and its sizes include 16, 32 and 48. The tray loads it from the folder of the executable with `LR_LOADFROMFILE`; `scripts/windows/make-ico.sh` regenerates it from `design/brand/sbarhive.svg` and a re-run produces an identical file.
- AC7: a registration failure (hotkey taken) writes one log line `sbarhived: shortcut <string> unavailable` and the daemon keeps running; pure helper `shortcut_error_line(s: &str, ok: bool) -> Option<String>` is tested for both values.
- AC8: `W` (`scripts/windows/check.sh`) exits 0; `C`, `L` clean with baseline plus new tests; `M` has no surviving mutant in `hotkey.rs` and `shell_logic.rs`; one CHANGELOG line.

## Check commands
`W`, `L`, `C`, `M` as in S-045 (6G scope, `-j 2`).

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1, AC2, AC3, AC4, AC7 | Unit tests with the exact values above | unit |
| AC5 | Text test over `shell.rs`; `W` | unit, build |
| AC6 | `file` on the icon; run `make-ico.sh` twice and compare SHA-256 | integration |

## Live check deferred to S-052
- Icon appears, tooltip shows progress during a job, menu is right to left in Arabic, icon returns after restarting Explorer, the shortcut opens the hive, and works during a drag.

## Out of scope
- A settings UI for the shortcut; `--at` for Windows (the daemon reads the pointer itself).

## Review log
- 2026-10-24, planner: default shortcut avoids the Win key because many Win combinations are reserved (ADR 0005 open question).
- 2026-10-24, tester: tests written from AC1 to AC8. Files: `crates/core/tests/s048_hotkey.rs` (AC1), `crates/daemon/src/shell_logic_tests.rs` (AC2, AC3, AC4, AC7; registered in main.rs as `#[cfg(test)] mod shell_logic_tests;`), `crates/daemon/tests/s048_sources.rs` (AC5, AC6, AC8). Failure output before implementation: core test does not compile (`unresolved import sbarhive_core::hotkey`, `no field shortcut on type Config`); daemon unit tests do not compile (`unresolved import crate::shell_logic`); `s048_sources` 11 of 12 fail (no shell.rs, no sbarhive.ico, no make-ico.sh, no mutants exclusion, no CHANGELOG line); the twelfth (COM not on main thread) was made to also require shell.rs so it fails too.
