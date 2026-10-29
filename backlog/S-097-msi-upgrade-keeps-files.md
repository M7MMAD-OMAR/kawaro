# S-097 MSI upgrade keeps the program files
Status: done (2026-10-29, manager, urgent defect fix). Superseded by S-100: the `After InstallFinalize` order is gone; the upgrade only appeared to work because a failed install rolled back to the old files.

## Context
Owner test on Windows 11 after the in-app update from 0.4.0 to 0.4.1: the Start shortcut said `kawarod.exe` "has been changed or moved". Reproduced with a script on the same virtual machine: install the 0.4.0 MSI, then the 0.4.1 MSI. Result: only the `actions` folder is left; `kawaro.exe`, `kawarod.exe` and `kawaro.ico` are gone. A fresh install and the same upgrade with the fix keep all files.

## Cause
wixl schedules `RemoveExistingProducts` right after `InstallValidate` (sequence 1401). Costing already ran with the old files present, so the new install skipped them; then the removal of the old product deleted them.

## Fix
`installer/windows/kawaro.wxs`: `<Custom Action="RemoveExistingProducts" After="InstallFinalize">`. The shared components (same GUIDs) keep their files because the new product already owns them.

## Acceptance criteria
- AC1: the MSI built from the project's .wxs has `RemoveExistingProducts` after `InstallFinalize` in `InstallExecuteSequence` (test `tests/release/s097_msi_upgrade_order.sh`, fails without the fix).
- AC2: on Windows 11, installing 0.4.0 then the fixed MSI leaves `kawaro.exe`, `kawarod.exe`, `kawaro.ico` and `actions` (measured 2026-10-29).
- AC3: the in-app update from the released fixed version is checked live before the next release (owner pending).

## Review log
- 2026-10-29: measured before and after on the Windows 11 virtual machine with the script from this story's context: before, only `actions` remained; after, all four entries remained. Released as 0.4.2.
