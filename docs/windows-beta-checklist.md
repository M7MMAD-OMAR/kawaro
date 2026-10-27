# Kawaro Windows beta checklist (S-079)

The owner runs every step below on a real Windows machine, in order, and marks each step PASS or FAIL. The beta ships the drop hive, the tray icon, the global shortcut, job notifications and the Windows updater, so every one of them gets a live procedure here.

## Before you start

- Windows 10 22H2 or Windows 11, 64 bit, with a mouse and a 60 Hz or faster display. Do the steps in order.
- Create the free folder `%USERPROFILE%\kawaro-beta-test`. Everything you save for the report goes there.
- Copy to the machine: `kawaro-<version>-windows-x64.msi` and `kawaro-<version>-windows-x64.zip` with `SHA256SUMS`, `scripts\windows\beta-smoke.cmd`, `scripts\windows\collect-logs.cmd`, and `scripts\windows\run-tests.cmd` with the S-044 ISO.
- Stage these prerequisites: a signed beta feed with a maintainer-supplied newer MSI behind it, a second Windows user account, a second machine on the LAN, an exFAT drive, LibreOffice and Ghostscript installed, sample files with Arabic names, and a screen recorder running at 60 fps.
- Window rule: every test window you open has a bounded size (for example 1000 by 700), floating, never maximized. Close only the exact windows the test opened, never your other windows or unsaved work.
- Record the exact commit SHA you are testing; the release readiness step checks CI against it.

## Steps

W01 Install packages
Do: Download `kawaro-<version>-windows-x64.zip` and `kawaro-<version>-windows-x64.msi` with `SHA256SUMS` from the release page and verify both with certutil -hashfile (or Get-FileHash) against SHA256SUMS, run the per-user MSI and pass SmartScreen with More info then Run anyway, let the daemon start once after install (it renames the Start shortcut and sets the app name), then click the Kawaro entry in the Start menu (it runs `kawarod.exe settings`) and time the click until its window is visible, then click the same entry nine more times and count the Kawaro windows and the kawarod.exe processes in Task Manager or with `tasklist /FI "IMAGENAME eq kawarod.exe"`.
Expect: The hashes match SHA256SUMS for both files; the unsigned per-user MSI installs without elevation; right after install the Start menu entry shows the cactus icon, and after the daemon sync it shows the cactus icon with the name Kawaro | كوارو (Latin part first) in Start and in Start search; the click opens one Kawaro window within 1 s (cold start included) with no console window appearing or flashing, the second click brings that same window back instead of opening a second one, and after ten clicks exactly one Kawaro window and one kawarod.exe process remain.
Result: PASS | FAIL

W02 Portable integration
Do: From the unpacked zip portable folder run kawaro.exe integration install and confirm the Explorer context menu entry, then run kawaro.exe integration uninstall and open the context menu again.
Expect: After integration install the Explorer context menu shows Kawaro entries and after integration uninstall they are removed from the menu.
Result: PASS | FAIL

W03 Portable integration faults
Do: Keep kawaro.exe in the portable folder and remove kawarod.exe (or kawaro.ico) beside it before running kawaro.exe integration install to hit the missing-file case, then restore the file, block the first registry value with reg.exe and run integration install again from the portable folder.
Expect: The missing-file run exits nonzero and prints the refusal message `kawaro: missing next to the executable: <name>`, naming the missing kawarod.exe or kawaro.ico the install preflight found beside the executable, and writes no registry value; the second run stops at the first failing registry value and writes no further values.
Result: PASS | FAIL

W04 Explorer selections
Do: In Explorer select one file and choose Kawaro from the right click menu and from Show more options, then select ten files and choose Kawaro again.
Expect: The one selection opens one hive for that file and the ten selection opens one hive covering all ten files.
Result: PASS | FAIL

W05 Hive placement and appearance
Do: Open the hive with the pointer in the middle and near a screen edge, at 150% display scaling; look at the corners, the border and the taskbar.
Expect: The hive centres on the pointer and stays fully on screen at the edge and at 150%, with transparent corners, no shadow, no frame, no taskbar button, and text stays sharp.
Result: PASS | FAIL

W06 Hive focus and accessibility
Do: Open the hive and press a letter key without clicking to test focus, press Esc, then open it again with Narrator running and walk the cells.
Expect: Keyboard focus lands in the hive from the key press, Esc closes the hive, and Narrator reads a label on every cell.
Result: PASS | FAIL

W07 Hive timing
Do: Record at 60 fps the hive appearing after the menu click, three trials on a 60 Hz display, and count frames from the click to the first full hive frame.
Expect: All three trials land within 150 ms and 9 frames or fewer from the click to the first full hive frame.
Result: PASS | FAIL

W08 Convert and preserve
Do: Record the hash or size and date of a PNG, convert it to WEBP from the hive, then check the folder and the original file again.
Expect: A new result file appears next to the original and the original hash or size and date is unchanged.
Result: PASS | FAIL

W09 Arabic
Do: Set `language = "ar"` in config.toml, restart the daemon, open the hive over a file with an Arabic name and convert it.
Expect: Arabic labels mirror right to left and the Arabic name stays intact in the result name.
Result: PASS | FAIL

W10 Pipe second account
Do: Sign in to a second Windows user account and from that session run kawaro open with runas against the daemon pipe, watching the daemon log.
Expect: The second account gets ERROR_ACCESS_DENIED and the daemon log shows no request from that account.
Result: PASS | FAIL

W11 Pipe remote client
Do: From a second machine connect a pipe client to the remote path `\\HOSTNAME\pipe\kawaro` and send one request.
Expect: The remote client is rejected with ERROR_ACCESS_DENIED and the daemon log records the remote rejection.
Result: PASS | FAIL

W12 Pipe foreign owner
Do: Build a client with a foreign owner SID from another account token and watch the daemon pipe with a monitor while it connects, confirming the rejection happens before any request bytes are read.
Expect: The foreign owner is rejected before request bytes are sent and the daemon logs the denial.
Result: PASS | FAIL

W13 Daemon instance storm
Do: Start a second daemon with kawarod.exe while the first runs, then launch ten concurrent kawaro open clients and watch the pipe server count.
Expect: One kawarod and one named-pipe server remain after the storm and every client connects to it.
Result: PASS | FAIL

W14 Pipe timeout and cancellation
Do: Point kawaro open at an unreachable pipe and wait 10 s, then leave a client blocked on a pending request and trigger CancelIoEx from the cancel path.
Expect: The client hits its 10 s timeout and exits while the daemon stays responsive; CancelIoEx completes and the client is not hung with the daemon responsive.
Result: PASS | FAIL

W15 Daemon start
Do: Run kawaro open from a terminal and close the terminal at once, then run it again inside a job that forbids breakaway and watch it retry, timing the daemon start within 500 ms of the first open.
Expect: kawarod survives the terminal close and keeps running, the retry starts one daemon within 500 ms of the open, and one kawarod serves the client.
Result: PASS | FAIL

W16 Settings and action paths
Do: Write config.toml under %APPDATA%\Kawaro from the settings UI and watch for a temporary file during the atomic save; drop a custom action into the user actions folder, restart the daemon, and read %LOCALAPPDATA%\Kawaro\logs\kawarod.log.
Expect: The save is atomic, the config stays valid and no temp file is left behind; the custom user action appears in kawarod.log and in the listing after the restart.
Result: PASS | FAIL

W17 Known folder fallback
Do: Empty APPDATA and remove LOCALAPPDATA from the environment, start the daemon, and compare the paths it writes with the actual known folders.
Expect: With APPDATA empty and LOCALAPPDATA unset the daemon writes to the actual known folder paths for roaming and local data.
Result: PASS | FAIL

W18 exFAT collision
Do: On an exFAT drive convert a file twice with output names already occupied, recording the hash or size and date of every existing file first.
Expect: The conversion takes the next free name, does not replace any occupied file, and every existing file is unchanged.
Result: PASS | FAIL

W19 Doctor paths
Do: Run kawaro doctor on the test machine and read every reported path against the real folders on disk.
Expect: doctor reports the named pipe endpoint and the config, logs and actions paths it uses.
Result: PASS | FAIL

W20 Engine discovery
Do: Install LibreOffice and Ghostscript in their default registry locations, keep FFmpeg bundled next to kawaro.exe and convert one file per engine; then rename one engine away and convert again.
Expect: LibreOffice, Ghostscript and FFmpeg are found at their discovered paths and each conversion runs; with an engine missing the message names the missing engine.
Result: PASS | FAIL

W21 Engine path and wait
Do: Install LibreOffice under `D:\Tools\Office pro العربية`, a path with spaces and Arabic, run kawaro on soffice.com and wait in the shell for its completion.
Expect: The result output lands with the Arabic path intact and soffice.com waits and does not return until the conversion is complete.
Result: PASS | FAIL

W22 Engine process tree and consoles
Do: Run one conversion per engine (FFmpeg, ffprobe, Ghostscript, soffice) through normal exit, cancel, timeout and daemon termination paths, watching the whole process tree in Process Explorer for grandchildren; record the screen and the daemon console for any flash.
Expect: After every exit no engine child or grandchild remains: no soffice.exe, no soffice.bin, no FFmpeg, no ffprobe and no Ghostscript; no console window or flash appears for the daemon, FFmpeg, ffprobe or Ghostscript.
Result: PASS | FAIL

W23 Explorer drop
Do: Drag files over the hive and watch the highlight, drop them and hash the originals; cancel a second drag mid-way, close the hive, then start a new drag with different files.
Expect: The highlight follows the drag and the drop writes a new result next to the original with the original unchanged; after the close the fresh drag lists only current files with no stale entries.
Result: PASS | FAIL

W24 Shortcut
Do: Record at 60 fps a press held past the hold_ms threshold and a release, start an OLE drag from the hive, repeat with swapped primary buttons, then register the same shortcut elsewhere as a taken conflict and hold again.
Expect: The hive opens only after the hold_ms threshold elapsed and within 150 ms and 9 frames of it, and closes on release; the OLE drag and the swapped primary buttons still drag the hive; the taken shortcut logs `shortcut <string> unavailable` and the daemon stays alive.
Result: PASS | FAIL

W25 Tray progress and resources
Do: Watch the tray tooltip during a long conversion job progress and during an updater download progress, then push 100 repeated progress updates while watching GDI handles in Task Manager.
Expect: The tray tooltip shows job progress and download progress as they move and the GDI handle count stays flat over the repeated progress updates.
Result: PASS | FAIL

W26 Tray menus and actions
Do: Open the tray menu in English and in Arabic, click outside the menu to close it, run Cancel all on two running jobs, open Settings from the menu and choose Quit.
Expect: The Arabic menu shows RTL, an outside click closes the menu, Cancel all cancels both jobs, Settings opens the settings window and Quit leaves no ghost icon behind.
Result: PASS | FAIL

W27 Tray lifecycle
Do: Restart Explorer from Task Manager while the daemon runs and watch the notification area afterwards.
Expect: The tray icon returns as one tray icon after the Explorer restart and its menu still opens.
Result: PASS | FAIL

W28 Job notifications
Do: Run a job until it finishes end to end with a different window focused and count the toasts through the whole run.
Expect: Exactly one Kawaro toast appears per finished job and the focus stays unchanged on the other window.
Result: PASS | FAIL

W29 Open folder
Do: Convert into a folder whose path contains spaces and use Open folder on the result; then point the shell at an invalid Explorer path and use Open folder again to force a spawn failure.
Expect: The first Open folder selects the result in Explorer; the second logs the spawn failure and reaps its child with no child left behind.
Result: PASS | FAIL

W30 Updater manual check
Do: Point the updater at the real signed beta feed with the maintainer-supplied newer MSI behind it, run kawaro update --check and let it download and verify the package signature.
Expect: The check reports a newer version available from the signed feed and the download completes with signature verification and a valid signature.
Result: PASS | FAIL

W31 Updater status and configuration
Do: Take the machine offline and run kawaro update --status, then set `update_check = false` in config.toml and run a manual check with kawaro update --check.
Expect: The status run makes no network call and reports the cached last result; the manual check still runs with automatic checks off and no automatic check starts on its own.
Result: PASS | FAIL

W32 Updater tray and deferral
Do: Set `update_check = true`, open the tray menu and use the enabled Update now action, then retry it during a busy job and again with an open hive.
Expect: Update now is enabled and runs the check and the download; with a busy job or an open hive the apply defers and waits for the job and the hive to finish.
Result: PASS | FAIL

W33 Updater apply modes
Do: Leave the updater in ask mode and watch the prompt path, then switch to idle auto mode and let it apply without a prompt.
Expect: In ask mode a prompt appears and the update waits for your answer before the apply; in idle auto mode the apply runs on its own while the machine is idle.
Result: PASS | FAIL

W34 Updater live apply
Do: Install the maintainer-supplied newer MSI through the updater, quit the daemon mid-apply and watch the copied helper survive it and find msiexec.exe through %SystemRoot%, then read the log and the one-shot result and check the temporary state is cleaned.
Expect: The copied helper survives the daemon quit, finds msiexec.exe through SystemRoot and installs the per-user MSI, then kawarod.exe restarts; the log records a one-shot result, temporary state is cleaned and one update notification appears.
Result: PASS | FAIL

W35 Updater MSI exit codes
Do: Force each MSI exit code on the helper in turn (0, 3010, 1602, 1618, 1603) by staging installs and record what the updater reports for each.
Expect: 0 maps to success, 3010 maps to success needing a restart, 1602 maps to cancelled, 1618 maps to another install busy with a retry, and 1603 maps to failure with an error in the log.
Result: PASS | FAIL

W36 Updater failure handling
Do: Let antivirus or SmartScreen block the copied helper and read the report, then take the feed offline and hang an update --check on it while using the daemon.
Expect: An antivirus or SmartScreen block of the copied helper is reported in the log with an error message and the daemon stays responsive; an offline or hung check times out and the daemon stays responsive to new requests.
Result: PASS | FAIL

W37 Native ISO runner
Do: Mount the S-044 ISO read-only and run run-tests.cmd from the disc on a Windows machine, keeping every output on the hard disk.
Expect: Each test exe writes one result per test exe with total N failed 0 and all logs land outside the disc.
Result: PASS | FAIL

W38 Release readiness
Do: Check the CI run for the exact commit SHA under test and confirm the windows-cross and windows-native jobs ran on it.
Expect: Both windows-cross and windows-native report success for the exact tested commit before the beta is tagged.
Result: PASS | FAIL

W39 MSI lifecycle
Do: Install the newer MSI as an upgrade, try the older MSI as a downgrade, sign out and sign in for the next logon start, then uninstall from Settings and compare files, registry keys and the Run value.
Expect: The upgrade keeps converted files, the downgrade is refused, one kawarod runs after the next logon, and the uninstall removes only Kawaro files and keys plus the Kawaro Run value while preserving converted files and other Run values.
Result: PASS | FAIL

W40 Smoke, logs, report and privacy
Do: Run `scripts\windows\beta-smoke.cmd` and `scripts\windows\collect-logs.cmd`, open smoke.txt and the report folder, then read kawarod.log and config.toml for personal paths before writing the report.
Expect: smoke.txt holds one line per command with its errorlevel and total N failed 0, the report folder holds kawarod.log and config.toml, and no personal paths, user names or machine names leave the machine.
Result: PASS | FAIL

## Report

Run `scripts\windows\collect-logs.cmd`, then send back from `%USERPROFILE%\kawaro-beta-test`:

- the PASS or FAIL list for W01 to W40,
- `smoke.txt`,
- `report\kawarod.log` and `report\config.toml`,
- hive screenshots only; nothing else on screen.

Before you send anything, open `kawarod.log` and `config.toml` and glance for personal paths, user names or machine names; remove them or leave the file out.

Send no other files.
