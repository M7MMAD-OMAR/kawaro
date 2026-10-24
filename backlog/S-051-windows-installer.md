# S-051 Beta packages for Windows: portable zip and a per-user MSI
Status: approved (manager, by delegation, 2026-10-24)

Builds on ADR 0005 section 8 and on S-044 (build scripts), S-045 (`sbarhived.exe`), S-048 (icon) and S-050 (keys). For the Windows beta tonight. Build tools only on the maintainer machine, nothing shipped. One work day or less. Order inside the story: the zip first (AC1 to AC3), then the MSI (AC4 to AC7). If `wixl` cannot build a per-user package with these keys, the zip is the beta artefact and the MSI moves to a new story S-051b; the result is written in the review log.

## Story
As a Windows user, I want to download one file, install SbarHive for my account without administrator rights, and remove it again from "Apps", so that I can try the beta safely.

## Files
`scripts/windows/{make-zip.sh,make-msi.sh}`, `installer/windows/sbarhive.wxs`, `installer/windows/README-windows.txt`, `docs/engines.md` (msitools, zip), `README.md` (unsigned warning, English and Arabic), `CHANGELOG.md`.

## Acceptance criteria
- AC1: `scripts/windows/make-zip.sh` (no cargo call; reads the release build in `CARGO_TARGET_DIR`) writes `sbarhive-<version>-windows-x64.zip`, where `<version>` is the workspace version. `unzip -l` lists `sbarhive.exe`, `sbarhived.exe`, `sbarhive.ico`, `README-windows.txt` and `SHA256SUMS`, and the action TOML files at the path the daemon reads them from (the implementer records the path). Exit 2 with a message when `sbarhived.exe` is missing.
- AC2: `SHA256SUMS` in the zip has one line per file and each hash equals `sha256sum` of the extracted file. A second run on unchanged inputs produces a zip with the same SHA-256 (fixed timestamps and order).
- AC3: `README-windows.txt` (English, then Arabic) names the three steps: unzip, run `sbarhive integration install`, right click then "Show more options"; it names the SmartScreen steps ("More info", "Run anyway") and has no long dash.
- AC4: `scripts/windows/make-msi.sh` runs `wixl` and writes `sbarhive-<version>-windows-x64.msi`; exit 0, and `file` says it is a composite document or MSI. `msiinfo export <msi> Property` shows `ProductVersion` equal to the version and no `ALLUSERS` row with value `1`; the install folder row targets `LocalAppDataFolder\Programs\SbarHive`.
- AC5: `msiinfo export <msi> Registry` contains the same keys as S-050 `install_commands`, with the install folder token in place of `<dir>`: the `MultiSelectModel` value `Player`, the command value `"[INSTALLDIR]sbarhived.exe" open "%1"`, the `AppUserModelId` key with `DisplayName`, and a `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` value `SbarHive` = `"[INSTALLDIR]sbarhived.exe"`. A test compares the verb and AUMID rows with the S-050 output for a dir of `[INSTALLDIR]`, so the two sources cannot drift.
- AC6: `msiinfo export <msi> File` lists the four files of the zip (exes, icon, actions), and `Shortcut` has a Start menu shortcut named `SbarHive`.
- AC7: `docs/engines.md` records `msitools` (version from `rpm -q`, licences) and states that it is a build tool only. `README.md` has the unsigned warning in English and Arabic, with the SHA-256 note. `scripts/design-lint.sh` clean; one CHANGELOG line; no Rust changes, so no mutants.

## Check commands
Read-only tools (`unzip`, `msiinfo`, `sha256sum`, `file`) run outside the scope. `make-zip.sh` and `make-msi.sh` run `scripts/windows/build.sh` first when the release files are missing (inside the 6G scope, `-j 2`).

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1, AC2 | Shell test on a fake release tree (small fixture exes) and on the real build: `unzip -l`, extract, `sha256sum -c`, double run compared | integration |
| AC3, AC7 | Test greps the files for the phrases and for long dashes | unit |
| AC4, AC5, AC6 | Shell test running `msiinfo export` for `Property`, `Registry`, `File`, `Shortcut` and comparing the rows | integration |

## Live check deferred to S-052
- Install by double click (SmartScreen), the menu item and toast name appear, daemon starts at the next logon, uninstall from "Apps" removes the files, the keys and the Run value.

## Out of scope
- Signing (`osslsigncode`), WiX 4, NSIS (fallback only), uploading to a GitHub release, arm64.

## Review log
- 2026-10-24, planner: sizes the beta so the zip alone is shippable; the registry rows are compared with S-050 to keep one source of truth.
