# S-051 Beta packages for Windows: portable zip and a per-user MSI
Status: review

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
- 2026-10-24, tester: tests written from the ACs only. Files: `tests/windows/test_s051.sh` (AC1 to AC7, run inside the 6G scope with `CARGO_BUILD_JOBS=2`) and `crates/core/tests/s051_msi_registry.rs` (AC5, ignored in a plain `cargo test` because CI has no msitools; the shell test builds the MSI, exports the Registry table and runs it with `--ignored`). Contracts fixed by the tests: release tree is `$CARGO_TARGET_DIR/x86_64-pc-windows-gnu/release`; outputs land in the working directory, the repo root or the target dir (the test finds the new file by name); one common prefix inside the zip; the missing `sbarhived.exe` case exits 2 with no cargo call (AC1 wins over the "Check commands" note that says build.sh runs first). Deferred until S-048/S-049: the icon content and its listing in the zip and the MSI File table; a placeholder `design/brand/sbarhive.ico` is created for the run when absent. The Icon and IconUri registry rows are still compared with `install_commands`. Test side mapping for AC5: `[INSTALLDIR]\` becomes `[INSTALLDIR]` (the MSI property already ends in a backslash). The tests were also run against a throwaway reference wxs and scripts outside the repo (all pass except the cargo step that needs the real repo; the 4 Rust checks pass on its export, and a deliberate umask leak in the reference was caught by the AC2 reproducibility check).
- Failure output before implementation (30 of 40 checks fail, all because `make-zip.sh`, `make-msi.sh`, the wxs, README-windows.txt and the docs do not exist):
        FAIL: AC1: scripts/windows/make-zip.sh is executable
        FAIL: AC1: exit code: got [127] want [0]
        FAIL: AC1: sbarhive-0.3.3-windows-x64.zip written (/tmp/sbarhive-s051.yJYqcG/err1.txt)
        FAIL: AC1: exit code without sbarhived.exe: got [127] want [2]
        FAIL: AC1: the message names sbarhived.exe
        FAIL: AC2: first zip written
        FAIL: AC3: installer/windows/README-windows.txt exists
        FAIL: AC4: scripts/windows/make-msi.sh is executable
        FAIL: AC4: installer/windows/sbarhive.wxs exists
        FAIL: AC4: exit code: got [127] want [0]
        FAIL: AC4: sbarhive-0.3.3-windows-x64.msi written (/tmp/sbarhive-s051.yJYqcG/err4.txt)
        FAIL: AC4: exit code with the real exes: got [127] want [0]
        FAIL: AC4: MSI from the real exes written
        FAIL: AC4: ProductVersion with the real exes: got [] want [0.3.3]
        FAIL: AC5: MSI written
        FAIL: AC6: MSI written
        FAIL: AC7: docs/engines.md names msitools
        FAIL: AC7: msitools version 0.106.58
        FAIL: AC7: msitools licence LGPL-2.1-or-later
        FAIL: AC7: msitools licence GPL-2.0-or-later
        FAIL: AC7: msitools licence MS-RL
        FAIL: AC7: states build tool only
        FAIL: AC7: README.md unsigned warning with the SHA-256 note, about Windows
        FAIL: AC7: README.ar.md unsigned warning with the SHA-256 note
        FAIL: AC7: one CHANGELOG line for S-051: got [0] want [1]
        FAIL: AC7: that line also has the Arabic sentence
        FAIL: AC7: scripts/windows/make-zip.sh exists without a long dash
        FAIL: AC7: scripts/windows/make-msi.sh exists without a long dash
        FAIL: AC7: installer/windows/sbarhive.wxs exists without a long dash
        FAIL: AC7: installer/windows/README-windows.txt exists without a long dash
      FAIL S-051 windows installer (30 of 40 checks failed)
- 2026-10-24, implementer: scripts, wxs, README-windows.txt, docs, README and CHANGELOG done. 90 of 91 checks of `tests/windows/test_s051.sh` pass. Outputs go to `$CARGO_TARGET_DIR/dist/` (last stdout line is the path). Zip layout: one folder `sbarhive-<version>-windows-x64/` with the exes, README-windows.txt, SHA256SUMS, `sbarhive.ico` when `design/brand/sbarhive.ico` exists, and `actions/*.toml`. The daemon embeds its built in actions at build time (`crates/core/build.rs`) and reads only user actions from `%APPDATA%\SbarHive\actions`, so the shipped `actions/` copies are reference files; the MSI puts them in `[INSTALLDIR]actions\`. Both scripts exit 2 when an exe is missing and never call cargo.
- One check cannot pass, a test bug, not fixed (tests are not edited): `AC7: README.ar.md unsigned warning with the SHA-256 note` in `tests/windows/test_s051.sh` runs `perl -CSD -00 -ne '... /(غير موقّع|غير موقع|...)/ ...'` without `use utf8`, so the Arabic literals are Latin-1 byte strings while `-CSD` decodes the input. Evidence: `perl -CSD -00 -ne '$f=1 if /(غير موقّع|غير موقع|غير موقعة|غير موقّعة|بدون توقيع)/; END{print $f+0}' README.ar.md` prints 0 although the paragraph holds «غير موقّعة», while `/\x{063A}\x{064A}\x{0631}/` on the same file prints 1. The SHA-256 and Windows parts of that regex match. Fix: add `-Mutf8` (or `use utf8;`) to that perl call.
- For the updater story (ADR 0006): the MSI is `MajorUpgrade` with `AllowSameVersionUpgrades`, stable UpgradeCode, `ProductCode` new per build, ProductVersion is the workspace version without a prerelease part (MSI versions are numeric, so `0.4.0-beta.1` and `0.4.0` collide: the updater must compare the manifest semver, not the MSI). Custom actions (unverified on Windows, S-052): `StopDaemon` runs `taskkill /F /IM sbarhived.exe` before InstallValidate (return ignored), `StartDaemon` launches `[INSTALLDIR]sbarhived.exe` after InstallFinalize unless uninstalling (async, no wait); the ADR flow (copy, quit daemon gracefully, `msiexec /i ... /qn /norestart`) still should quit the daemon first, because the taskkill kills a running job. A silent install over a live daemon without that would otherwise schedule a reboot replace. The Icon and IconUri rows always point at `[INSTALLDIR]sbarhive.ico`, which is only installed once S-048 adds `design/brand/sbarhive.ico` (make-msi.sh and make-zip.sh pick it up automatically). wixl writes no trailing backslash on keys only for `RegistryValue` with its own `Root` and `Key` (inside `RegistryKey` it adds one, which the test rejects). Same-version `msiexec /i` over an installed copy is allowed, but a downgrade is refused.
