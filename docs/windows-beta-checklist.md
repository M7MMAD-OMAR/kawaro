# Kawaro Windows beta checklist

Run on a real Windows machine and mark every step PASS or FAIL. Steps for features that are not built yet are listed as skipped, not failed.

## Before you start

- Windows 10 22H2 or Windows 11, 64 bit, with a mouse. Do the steps in order.
- Create the free folder `%USERPROFILE%\kawaro-beta-test`. Everything you save for the report goes there.
- Copy these files to the machine: `kawaro-<version>-windows-x64.msi` or `kawaro-<version>-windows-x64.zip` (S-051), `scripts\windows\beta-smoke.cmd`, `scripts\windows\collect-logs.cmd`, and `scripts\windows\run-tests.cmd` with the ISO from S-044.
- Install route: the MSI (double click), or unzip the zip and run `beta-smoke.cmd` from that folder. Use sample files only.
- Window rule: every test window you open has a bounded size (for example 1000 by 700), floating, never maximized. Close only the exact windows the test opened, never your other windows or unsaved work.
- Not in this build, so skipped: the drop hive (S-047), the tray icon, shortcut and icon (S-048), notifications (S-049). There is no tray icon yet: stop the service from Task Manager (end `kawarod.exe`) or uninstall.

## Steps

W01 Install with SmartScreen
Do: double click the MSI (or unzip the zip). Windows SmartScreen appears: click More info, then Run anyway.
Expect: install finishes with no administrator prompt; Start menu has a Kawaro entry; the folder holds kawaro.exe and kawarod.exe.
Result: PASS | FAIL
W02 Smoke script
Do: run `beta-smoke.cmd` from the install or unzipped folder; open `%USERPROFILE%\kawaro-beta-test\smoke.txt`.
Expect: 4 lines of `<command> <errorlevel>` all ending 0, then `total 4 failed 0`; no console window stays open from kawarod.exe.
Result: PASS | FAIL
W03 Explorer menu
Do: right click a .png file. On Windows 11 click Show more options first; on Windows 10 the menu shows it directly.
Expect: a Kawaro entry is in the list (under Show more options on Windows 11).
Result: PASS | FAIL
W04 Hive timing
Do: record the screen at 60 fps, click the Kawaro entry, count frames from the click to the first hive frame. Three trials.
Expect: 9 frames or fewer (150 ms) in each trial; all three must pass.
Result: PASS | FAIL
W05 Hive look
Do: open the hive with the pointer in the middle, then near each screen edge, and look at the corners and the border.
Expect: hive centre is at the pointer; near an edge it stays fully on screen; transparent corners show the desktop; no shadow and no frame; no taskbar button.
Result: PASS | FAIL
W06 Focus, Esc, Narrator
Do: open the hive, press a letter key without clicking, then press Esc. Start Narrator (Win+Ctrl+Enter) and open the hive again.
Expect: keyboard focus is in the hive at once and Esc closes it; Narrator reads a label for each cell and the centre cell.
Result: PASS | FAIL
W07 Ten files and 150%
Do: set display scaling to 150% in Settings, select ten files, right click, choose Kawaro.
Expect: one hive for all ten files (not ten hives), sharp at 150% and inside the screen; set scaling back to 100% afterwards.
Result: PASS | FAIL
W08 Convert
Do: convert a .png to WEBP from the hive.
Expect: a new .webp file appears next to the original within 10 s; the original is unchanged (same size and date).
Result: PASS | FAIL
W09 Arabic
Do: in `%APPDATA%\Kawaro\config.toml` set the line `language = "ar"`, then restart the service (end `kawarod.exe` in Task Manager, start it again from the Start menu or by opening a file). Open the hive, then convert a file whose name is Arabic.
Expect: Arabic labels, layout mirrored right to left, the result is saved next to the original with the Arabic name intact.
Result: PASS | FAIL
W10 LibreOffice and cancel
Do: install LibreOffice yourself, convert a .docx to PDF; then start a second conversion and cancel it. Check Task Manager.
Expect: the PDF opens; after cancel there is no soffice.bin process within 5 s; without LibreOffice the error names it.
Result: PASS | FAIL
W11 ffmpeg and Ghostscript
Do: put ffmpeg.exe in the `ffmpeg\bin` folder next to kawaro.exe and install Ghostscript; convert an .mp4 to .mp3 and a .pdf to .png.
Expect: both results open and play or show correctly; with an engine missing the error names ffmpeg or Ghostscript.
Result: PASS | FAIL
W12 Log on and upgrade
Do: sign out and sign in again, check Task Manager; then install a newer MSI over the old one.
Expect: kawarod.exe runs after the log on with one process; the upgrade keeps the Explorer entry and leaves no second install.
Result: PASS | FAIL
W13 Uninstall
Do: uninstall from Settings, Apps. Right click a .png again; look at Task Manager.
Expect: the Kawaro entry is gone from the menu and no kawarod.exe runs; files you converted are still there.
Result: PASS | FAIL
W14 Not in this build (skipped, not failed)
Do: nothing. This step is skipped, not a failure: the drag-drop hive (S-047), tray icon, shortcut and icon (S-048) and toast notifications (S-049) are not built. Write SKIPPED beside this id.
Expect: nothing to check; do not mark FAIL.
Result: PASS | FAIL

## Report

Run `scripts\windows\collect-logs.cmd`, then send back from `%USERPROFILE%\kawaro-beta-test`:

- the PASS or FAIL list for W01 to W14 (SKIPPED for W14),
- `smoke.txt`,
- the log, `report\kawarod.log`, and `report\config.toml`,
- screenshots only of the hive; nothing else on screen.

Before you send anything, open `kawarod.log` and `config.toml` and glance for personal paths, user names or machine names; remove them or leave the file out.

Send no other files, and no personal names or machine names.
