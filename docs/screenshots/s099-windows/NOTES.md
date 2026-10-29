# S-099 Windows live check (2026-10-30), 0.4.3 candidate

Setup: win11 VM, base = current main built as 0.4.2 (has S-099), candidate = same tree as 0.4.3 with
docs/whats-new/0.4.3.json. Manifest signed with the real update key; the VM reaches a local HTTPS
server as github.com (hosts entry, test CA in the VM root store, CRL served locally).
Note: the released 0.4.2 MSI predates S-099, so it cannot show the new update UI.

## Result: the in-app update does not complete

1. Updates page shows `Installed version: 0.4.2` (01).
2. Check for updates: the line stays `Checking for updates.` for the whole download (about 65 s with a throttled
   server), never `Downloading X%`; Check and Update now are greyed (02). The check also downloads and installs
   on its own, no Update now click.
3. A taskkill console flashes (MSI StopDaemon) and the settings window closes (03, 04). No `Installing` line was
   ever shown.
4. msiexec exits 1603 and rolls back. kawaro.exe stays 0.4.2, no reopen, no `Updated to 0.4.3`, no What is new (05).
   Only the Arabic toast `تعذّر التحديث 0.4.3. النسخة الحالية لم تتغير.` and `install failed: Failed(Msi(Failed))`
   in kawarod.log.

## Root cause (installer/windows/kawaro.wxs, unchanged since v0.4.2)

msi-log-excerpt.txt: the component ProgramFiles is `Installed: Absent; Request: Local; Action: Null` when a
different product code already owns it, so the new files are not marked for installation. StartDaemon
(FileKey=DaemonExe, After InstallFinalize, after RemoveExistingProducts) then fails with error 2753
`The File 'DaemonExe' is not marked for installation`, return value 3, rollback.

Clean manual runs, base 0.4.2 then `msiexec /i` candidate, same machine:

| wxs variant | rc | kawaro.exe after |
| --- | --- | --- |
| HEAD as is | 1603 | 0.4.2 (rolled back) |
| StartDaemon by Directory + ExeCommand | 0 | 0.4.2 (silent no op, files not replaced) |
| RemoveExistingProducts After InstallInitialize | 1603 | install folder nearly empty (1 entry) |

Also 0.4.0 to my 0.4.2 build: 1603 with the same 2753. So the S-097 check (files remain) passed only because the
rollback restores the old version. Program files do remain, but only as the old version.
