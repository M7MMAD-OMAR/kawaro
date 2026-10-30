# S-106 Fix four small Windows defects from the 0.4.4 VM run
Status: draft

## Story
As a Windows user, I want the settings window to come to the front after an update, the update to leave no installer files behind, the notification header to read Kawaro, and every window title to read Kawaro, so that the app looks finished and does not slowly fill my disk.

## Evidence and causes (read in the code, VM facts from the 0.4.4 run)
- D1 (window behind others after an update): the helper copy starts the new daemon detached (`update_sys.rs`, `apply/windows.rs` `start_daemon`) and the reopen goes through `show_settings` then `raise_welcome` then `shell_open::foreground`, which is one bare `SetForegroundWindow`. The new process holds no foreground right, so the call is refused (the `kawarod: foreground refused` log line exists, S-046).
- D2 (disk grows): downloads go to `<cache>/updates/<version>/` (package, `.log`) and nothing ever deletes them (no cleanup code exists in `crates/core/src/update` or the daemon). The helper copy folder `%TEMP%\kawaro-update-helper-<pid>` is also never removed. The copy Windows Installer itself keeps in `%WINDIR%\Installer` is owned by the system and is NOT touched.
- D3 (toast header shows the AUMID): the registry `DisplayName` is already written twice (`kawaro.wxs` `AumidName`, `identity/windows.rs` `sync`), so the cause is not a missing value. The Start shortcut made by wixl carries no `System.AppUserModel.ID` property, and the identity sync only renames it. S-106c records first, on the VM, the registry value and the shortcut property; the fix below is the expected one.
- D4 (titles): `settings.slint:72` sets the title to `Kawaro settings` / `إعدادات كوارو`, `welcome.slint:110` to `Welcome to Kawaro` / `أهلاً بك في كوارو`; `hive.slint:64` already reads `Kawaro`. Those two strings are also the in-page headings, so only the window title binding changes.

## Acceptance criteria
- A1 (D1, pure): `foreground_ladder(try_step)` tries these steps in order and stops at the first that returns true: plain `SetForegroundWindow`, restore plus topmost toggle (`HWND_TOPMOST` then `HWND_NOTOPMOST`), taskbar flash. It returns the step that worked, or `Flash` last, and the call count never exceeds 3. `shell_open::foreground` uses it and logs `kawarod: foreground refused` only when step 1 failed.
- A2 (D1): after `start_daemon` the helper calls `AllowSetForegroundWindow(child_pid)` for the daemon it spawned (source scan of `apply/windows.rs`: the call follows `.spawn()` and uses the returned `id()`).
- A3 (D2, pure): `stale_update_dirs(entries, installed)` returns exactly the `<updates>/<version>` folders whose version is less than or equal to `installed`. Input `0.4.2, 0.4.3, 0.4.4, 0.5.0, notes.txt, 0.4.4-x` with installed `0.4.4` returns `0.4.2, 0.4.3, 0.4.4`. Newer and non version names are kept.
- A4 (D2): the daemon runs that cleanup at start, and the helper deletes its own `<version>` folder content (msi and log) after `Outcome::Installed` only; after `Failed` the MSI stays for the retry. With 3 fake version folders of 1 MB each, one cleanup leaves 0 bytes for versions at or below installed and the 5 MB of a newer one untouched.
- A5 (D2): at daemon start, `%TEMP%\kawaro-update-helper-*` folders older than 600 s are removed, younger ones and other names are kept (pure `stale_helper_dirs(entries, now, 600)`). No code path names `%WINDIR%\Installer` (source scan: the string `Installer` as a path part is absent from the cleanup module).
- A6 (D3, pure): the identity plan returns a shortcut property list equal to `[(System.AppUserModel.ID, "io.github.m7mmad_omar.Kawaro")]`, with the value taken from `APP_ID` (never a second literal). `identity/windows.rs` applies it through `IPropertyStore` to both `Kawaro.lnk` and the bilingual shortcut name, and `cargo clippy --target x86_64-pc-windows-gnu` (`scripts/windows/check.sh`) passes.
- A7 (D4): the window title of `SettingsUi`, `WelcomeUi` and `HiveWindow` is the literal `Kawaro` in `.slint` (source scan: exactly one top level `title:` per window component, equal to `"Kawaro"`), in English and Arabic. The in-page headings still read `Kawaro settings`, `Welcome to Kawaro` and the Arabic pair (existing tests keep passing, no assertion edited).
- A8 (VM only, release step): on Windows 11 after an update 0.4.3 to the next build, the settings window is the foreground window within 3 s of the reopen (A1/A2); the `updates` folder of the Kawaro cache directory shows 0 version folders at or below the installed version and no `kawaro-update-helper-*` older than 10 min after the next start (A4/A5); the toast header reads `Kawaro`, not `io.github...` (A6); the title bar of the three windows reads `Kawaro` (A7). Screenshots in `docs/screenshots/s106/windows/`. If the header still shows the AUMID, the recorded registry and shortcut facts decide the next step and this AC reopens the story.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| A1 | Fake closure fails step 1, passes step 2: result is step 2, calls are 2; fails 1 and 2: result `Flash`, calls 3; passes step 1: calls 1 and no refused log line | unit (daemon) |
| A2 | Read `apply/windows.rs`: `AllowSetForegroundWindow` appears after `spawn()` and takes the child id | source scan |
| A3 | The six names above in, exact three paths out, in order; installed `0.4.1` returns an empty list | unit (core) |
| A4 | Temp dir with 3 folders of a 1 MB file plus a 0.5.0 folder of 5 MB: after cleanup total bytes under `updates` equal 5 MB; helper `run_helper` with a fake `Installed` removes the msi, with a fake `Failed` keeps it | integration (core) |
| A5 | Entries aged 599 s, 601 s, and a foreign name: only the 601 s one is returned; scan finds no `WINDIR` or `Installer` path in the cleanup source | unit, source scan |
| A6 | Plan equals the exact pair; scan of `windows.rs` finds `PKEY_AppUserModel_ID` or `IPropertyStore` and no second app id literal; `scripts/windows/check.sh` exit 0 | unit, source scan, cross build |
| A7 | Scan of the three `.slint` files for the window `title:` value; existing `s087_welcome` and settings heading tests stay green | source scan |
| A8 | The VM run above, recorded in the review log | manual, release step |

## Out of scope
Signing the Windows build, the Microsoft Store identity, the S-105 Hyprland click, GNOME and KDE focus (S-082), changing the in-page headings, cleaning the system copy in `%WINDIR%\Installer`, the Linux package cache.

## Review log
- Tester, 2026-10-30. Tests: `crates/core/tests/s106_windows_small.rs` (A3, A4, A5 pure, A6 plan), `crates/core/tests/s106_sources.rs` (A2, A4 daemon start, A5 and A6 and A7 scans), `crates/daemon/tests/s106_foreground.rs` (A1). A8 is VM only, no test. The hooks the implementer must create are pinned in the header comments of each file: `update::cleanup::{stale_update_dirs, stale_helper_dirs, clean_updates}`, `HelperOps::cleanup_download`, `identity::shortcut_properties`, `crates/daemon/src/foreground_logic.rs` (`Step`, `foreground_ladder`).
- Failing output (all for the right reason, nothing implemented):
  - `s106_sources`: 5 failed. A2 "AllowSetForegroundWindow must follow .spawn()"; A4 "daemon must call clean_updates"; A5 "crates/core/src/update/cleanup.rs must exist"; A6 "the property store must be used"; A7 `settings.slint: window title left: ["title: root.text.title;"] right: ["title: \"Kawaro\";"]`.
  - `s106_windows_small`: compile errors E0432 `unresolved import kawaro_core::update::cleanup`, E0407 `cleanup_download is not a member of trait HelperOps`, E0425 `cannot find function shortcut_properties in module identity`.
  - `s106_foreground`: `couldn't find file crates/daemon/tests/../src/foreground_logic.rs`.
- Not covered by a test: the `scripts/windows/check.sh` cross build of A6 (run it at the gate).
