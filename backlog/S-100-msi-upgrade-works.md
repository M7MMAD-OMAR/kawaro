# S-100 MSI upgrade really replaces the files
Status: in progress (2026-10-30, manager, urgent release blocker)

## Context
S-099 live check on Windows 11 (`docs/screenshots/s099-windows/NOTES.md`): upgrading Kawaro 0.4.2 to a newer build fails, by `msiexec /i` and by the in-app update alike. msiexec exits 1603 and rolls back, so the old version stays. The same happens from 0.4.0. S-097 passed its check only because the rollback restores the old files.

## Cause (corrected by measurement)
The S-099 note blamed the shared ComponentGuid. A build with new GUIDs alone still failed (`docs/screenshots/s100/vm-before-fix-guid-only.txt`): 1603 in all four scenarios (0.4.2 to 0.4.3 quiet, 0.4.0 to 0.4.3, double click, 0.4.3 to 0.4.4), and afterwards only `actions` was left, since the failed attempt removed the old exes. The decisive log line (`msi-log-before-fix-excerpt.txt`): `Disallowing installation of component ... since the same component with higher versioned keyfile exists`, so `ProgramFiles` is costed `Action: Null` while `ActionFiles` (same GUID situation) is `Local`. The difference between the two components is the file type: wixl cannot write File.Version, so `kawaro.exe` and `kawarod.exe` are unversioned, and Windows Installer keeps an installed unversioned file whose modification time is later than its creation time (it assumes a user edit). The cabinet carries the build machine's local time, so an exe built in an earlier time zone than the installing machine looks modified forever; here the build host is 11 hours ahead of the VM clock. Nothing is scheduled for the exes, `StartDaemon` (FileKey `DaemonExe`) fails with 2753, the install rolls back (1603). Moving `RemoveExistingProducts` after `InstallInitialize` instead deleted the skipped files and emptied the folder (S-097 era measurement).

## Approaches considered
- (a) ComponentGuid that changes with every build, `RemoveExistingProducts` at the wixl default: alone it does not fix the Null costing (measured). Kept, because the old product is then removed first and the new files are written fresh, and every release no longer depends on a shared component refcount.
- (b) Sequencing alone with fixed GUIDs: measured to fail (1603, or an emptied folder).
- (c) Helper that uninstalls first: does not help a double click. Not needed.
- (d) File.Version on the two exes, written after wixl with `msibuild` (`scripts/windows/msi-fixup.sh`): a versioned package file always replaces an unversioned installed one, whatever its modification time. This is the fix for the cause. Chosen together with (a).

## Fix
- `scripts/windows/msi-fixup.sh MSI VERSION`: sets File.Version `x.y.z.0` on `CliExe` and `DaemonExe`; `make-msi.sh` runs it on every MSI.
- `installer/windows/kawaro.wxs`: the last 12 hex digits of every ComponentGuid come from `$(var.GuidTail)`; the S-097 override of `RemoveExistingProducts` is removed.
- `scripts/windows/guid-tail.sh`: derives the tail from the version and the content of every packaged file, so two different builds never share component ids and the same inputs give the same tail.
- `scripts/windows/make-msi.sh` passes `-D GuidTail=...`.
- User data lives outside the MSI (`%APPDATA%\Kawaro`, `%LOCALAPPDATA%\Kawaro`), so no component owns it and no removal touches it.

## Acceptance criteria
- AC1: two MSIs built from the project's .wxs with different tails share no ComponentId (test `tests/release/s100_msi_upgrade_works.sh`).
- AC2: `RemoveExistingProducts` is scheduled before `InstallInitialize` in the built MSI (not after `InstallFinalize`).
- AC3: `guid-tail.sh` is deterministic, is 12 hex digits, and changes when the version or any file changes; `make-msi.sh` uses it.
- AC3b: both exes carry File.Version `x.y.z.0` in the built MSI.
- AC4: Windows 11, clean 0.4.2 then the candidate 0.4.3, by `msiexec /i` and by double click: exit 0, new `kawaro.exe` and `kawarod.exe` present, the new daemon running, the config in `%APPDATA%\Kawaro` kept, uninstall leaves nothing (evidence in `docs/screenshots/s100/`).
- AC5: the same from the 0.4.0 MSI.

## Review log
- 2026-10-30: measured on the Windows 11 VM with `docs/screenshots/s100/`. Before (GUIDs only): rc 1603 x4, folder emptied. After (GUIDs plus File.Version): rc 0 x4 for 0.4.2 to 0.4.3 (`msiexec /i /qn`), 0.4.0 to 0.4.3, 0.4.2 to 0.4.3 by ShellExecute (double click equivalent), 0.4.3 to 0.4.4; kawaro.exe and kawarod.exe have exactly the new package hashes, `actions` and the icon present, the sentinel file in `%APPDATA%\Kawaro` kept, menu key, Run key and Start shortcut present; uninstall removes the folder and all three registrations. The base installs were the real 0.4.0 and 0.4.2 MSIs (0.4.2 had the exes that failed before).
- Not proven: that the NEW daemon is the running process. A daemon (pid 6196) left by an earlier elevated test run survived every taskkill (access denied from a standard token), so no live process could be attributed. The log shows StopDaemon and StartDaemon executed in every upgrade. After the VM reboot that cleared the stale process the Windows lock screen asks for the owner's PIN, so the live daemon check needs the owner to sign in (rerun `S100.CMD`).
