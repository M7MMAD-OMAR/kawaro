[English](README.md) | [العربية](README.ar.md)

# Explorer right click menu (Windows)

`sbarhive integration install` adds SbarHive to the Explorer right click menu of files. `sbarhive integration uninstall` removes it. Neither needs administrator rights: only keys under `HKCU` are written, through `reg.exe`.

Run `install` from the folder where the zip file was unpacked. It checks that `sbarhived.exe` and `sbarhive.ico` are next to `sbarhive.exe`, then registers that folder. Add `--dry-run` to print the `reg.exe` commands without running them.

## Keys written

| Key | Value | Data |
| --- | --- | --- |
| `HKCU\Software\Classes\*\shell\SbarHive` | `MUIVerb` | `SbarHive` (`صبار هايف` when the language is Arabic) |
| | `Icon` | `<folder>\sbarhive.ico` |
| | `MultiSelectModel` | `Player` (one call for all selected files) |
| `HKCU\Software\Classes\*\shell\SbarHive\command` | default | `"<folder>\sbarhived.exe" open "%1"` |
| `HKCU\Software\Classes\AppUserModelId\io.github.m7mmad_omar.SbarHive` | `DisplayName` | `SbarHive` |
| | `IconUri` | `<folder>\sbarhive.ico` |

`uninstall` deletes the two trees `HKCU\Software\Classes\*\shell\SbarHive` and `HKCU\Software\Classes\AppUserModelId\io.github.m7mmad_omar.SbarHive`, and nothing else.

## Windows 11

The item shows under "Show more options" (or press Shift+F10 on the selected files). The first level menu of Windows 11 needs a signed package and is not part of this beta.

## Status

The command lines are unit tested on Linux. Checking the menu on a live Windows system is tracked in S-052.
