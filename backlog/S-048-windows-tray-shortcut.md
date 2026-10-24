# S-048 Tray icon, menu in Arabic and English, and the global shortcut on Windows
Status: review

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
- Follow-up: saving from the Settings window resets `shortcut` to its default, because the save builds the config with `..Config::default()`; a user-edited `shortcut` in `config.toml` is lost on save. Fix in a later story by starting from the loaded config.

## Review log
- 2026-10-24, planner: default shortcut avoids the Win key because many Win combinations are reserved (ADR 0005 open question).
- 2026-10-24, tester: tests written from AC1 to AC8. Files: `crates/core/tests/s048_hotkey.rs` (AC1), `crates/daemon/src/shell_logic_tests.rs` (AC2, AC3, AC4, AC7; registered in main.rs as `#[cfg(test)] mod shell_logic_tests;`), `crates/daemon/tests/s048_sources.rs` (AC5, AC6, AC8). Failure output before implementation: core test does not compile (`unresolved import sbarhive_core::hotkey`, `no field shortcut on type Config`); daemon unit tests do not compile (`unresolved import crate::shell_logic`); `s048_sources` 11 of 12 fail (no shell.rs, no sbarhive.ico, no make-ico.sh, no mutants exclusion, no CHANGELOG line); the twelfth (COM not on main thread) was made to also require shell.rs so it fails too.
- 2026-10-24, reviewer: REWORK (first). Gates: fmt, clippy, cargo test --all, design-lint, scripts/windows/check.sh, tests/windows/test_s051.sh all pass. `cargo mutants --in-place --in-diff` over the whole `*.rs` diff: 35 mutants, 32 caught, 3 unviable, 0 missed. Icon: 6 images (16, 24, 32, 48, 64, 256), `make-ico.sh` run twice gives the identical SHA-256 (66e28039...), equal to the committed file. Reasons:
  1. HICON leak (tray/windows.rs `icon()`, called from shell.rs `icon_data()` on every `notify_icon`): each NIM_ADD and every NIM_MODIFY (one per progress event and per language change) runs `LoadImageW(LR_LOADFROMFILE)` again, reads the 305 KB file from disk, and never calls `DestroyIcon`. This leaks a GDI icon handle per tooltip update and does file I/O on the shell thread each time. Fix: load the icon once (cache the HICON, for example in the thread local or a `OnceLock<isize>` set on the shell thread) and reuse it for ADD, MODIFY and the TaskbarCreated re-add; destroy it on WM_DESTROY after NIM_DELETE (not for the shared `IDI_APPLICATION` fallback). Not a unit-testable pure function, so no new test is required; the shell.rs text test may stay.
  2. Decision on the manager's question (1): the loosening of `key_code` is acceptable, no REWORK for it. It only admits `F01`, `F001` and `F024` (leading zeros); `F+5` is rejected by the modifier check, and every value still lands in F1 to F24, so nothing unsafe or out of range is registered. AC1 and the docs do not forbid leading zeros. Keeping the guard would have needed a new tester test; not worth a round. Record only.
  3. Settings follow-up (2): confirmed recorded under Out of scope. `to_config` already resets `hive_surface` and `hold_ms` the same way, so Linux behaviour is unchanged; `shortcut` is read only by the Windows shell.
  Checked and fine: shell.rs and tray/windows.rs hold no pure logic (decisions in shell_logic.rs and hotkey.rs, both fully mutation-tested); message loop on the shell thread; hidden top-level window, no HWND_MESSAGE; TaskbarCreated re-adds; menu created and `DestroyMenu` after TrackPopupMenu, WM_NULL posted; hold timer killed on Released and Idle; RegisterHotKey failure and parse failure log `sbarhived: shortcut <s> unavailable` and run on; Quit goes ui::quit, then main calls `tray::stop`, SendMessage WM_CLOSE, DestroyWindow on the shell thread, NIM_DELETE, UnregisterHotKey, PostQuitMessage; no COM; Arabic uses MFT_RIGHTORDER and TPM_LAYOUTRTL. Linux tray path unchanged (tray/linux.rs untouched).
  For the S-052 live check (needs real Windows): icon shows at the right size and DPI; tooltip updates during a job; right-click menu appears, closes on outside click, is right to left in Arabic; Cancel all, Settings and Quit work and Quit leaves no ghost icon; icon returns after restarting Explorer; the shortcut opens the hive after hold_ms, releases on key up, and works during a drag; a taken combination logs the unavailable line; also check the handle count (GDI objects) stays flat over many progress updates after the fix.
