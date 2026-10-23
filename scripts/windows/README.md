# Windows build scripts / سكربتات بناء ويندوز

## English

These scripts cross build and check the Windows programs on Linux, for the
`x86_64-pc-windows-gnu` target. They need `mingw64-gcc` and the Rust target
(`rustup target add x86_64-pc-windows-gnu`). Outside CI (`CI=true`), the
build scripts run themselves inside
`systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200%` with two jobs,
and print `cgroup: <path>` as their first line. The target dir defaults to
`/home/sbarah/.cache/sbarhive-win-target`; a `CARGO_TARGET_DIR` you set is
kept.

| command | what it does |
| --- | --- |
| `scripts/windows/build.sh` | builds `x86_64-pc-windows-gnu/release/sbarhive.exe` |
| `scripts/windows/check-dlls.sh <exe>` | reads the DLL imports with `x86_64-w64-mingw32-objdump -p` and fails (exit 1) when any `lib*.dll` is imported, since mingw DLLs are not shipped; prints `dll check: ok <name> <N> imports` when clean. `--text <file>` reads saved objdump output instead |
| `scripts/windows/check.sh` | runs clippy for Windows on core, engines and the CLI, all targets, warnings as errors |
| `scripts/windows/make-iso.sh` | builds nothing; packs the release `sbarhive.exe` (`bin/`), the test programs from `debug/deps` (`tests/`), `run-tests.cmd` and `SHA256SUMS` into `sbarhive-win-tests.iso` in the target dir with `genisoimage -J -R` |
| `scripts/windows/run-tests.cmd` | on Windows, runs every `tests\*.exe` on the disc with `--test-threads=1` and writes `results.txt` (one `<exe> <errorlevel>` line each, then `total N failed F`) and `log.txt` in `%USERPROFILE%\sbarhive-tests` |

A full check after a change:

```
scripts/windows/build.sh
scripts/windows/check-dlls.sh /home/sbarah/.cache/sbarhive-win-target/x86_64-pc-windows-gnu/release/sbarhive.exe
scripts/windows/check.sh
```

For the Windows test machine, build the test programs, pack them, and attach
the ISO to the `win11` VM as a read only CD (not run by these scripts):

```
systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_TARGET_DIR=/home/sbarah/.cache/sbarhive-win-target cargo test --no-run -j 2 --target x86_64-pc-windows-gnu -p sbarhive-core -p sbarhive-engines -p sbarhive-cli
scripts/windows/make-iso.sh
virsh -c qemu:///session attach-disk win11 /home/sbarah/.cache/sbarhive-win-target/sbarhive-win-tests.iso sdb --type cdrom --mode readonly
```

Then run `D:\run-tests.cmd` (or the CD drive letter) in the VM.

## العربية

تبني هذه السكربتات برامج ويندوز وتفحصها على لينكس، للهدف
`x86_64-pc-windows-gnu`. تحتاج `mingw64-gcc` وهدف Rust
(`rustup target add x86_64-pc-windows-gnu`). خارج CI (`CI=true`) يعيد
سكربتا البناء والفحص تشغيل نفسيهما داخل
`systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200%` بمهمتين،
ويطبعان `cgroup: <المسار>` أول سطر. مجلد البناء الافتراضي
`/home/sbarah/.cache/sbarhive-win-target`، وإن ضبطت `CARGO_TARGET_DIR`
بقي كما هو.

| الأمر | ما يفعله |
| --- | --- |
| `scripts/windows/build.sh` | يبني `x86_64-pc-windows-gnu/release/sbarhive.exe` |
| `scripts/windows/check-dlls.sh <exe>` | يقرأ المكتبات المستوردة بـ `x86_64-w64-mingw32-objdump -p`، ويفشل (رمز 1) إن وُجدت أي `lib*.dll` لأن مكتبات mingw لا تُشحن، ويطبع `dll check: ok <name> <N> imports` إن كان الملف نظيفاً. ويقرأ `--text <file>` مخرج objdump محفوظاً بدلاً من ذلك |
| `scripts/windows/check.sh` | يشغّل clippy لويندوز على core وengines وسطر الأوامر، بكل الأهداف، والتحذيرات أخطاء |
| `scripts/windows/make-iso.sh` | لا يبني شيئاً؛ يجمع `sbarhive.exe` من بناء الإصدار (`bin/`) وبرامج الاختبار من `debug/deps` (`tests/`) و`run-tests.cmd` و`SHA256SUMS` في صورة `sbarhive-win-tests.iso` داخل مجلد البناء بـ `genisoimage -J -R` |
| `scripts/windows/run-tests.cmd` | على ويندوز يشغّل كل `tests\*.exe` من القرص بـ `--test-threads=1`، ويكتب `results.txt` (سطر `<exe> <errorlevel>` لكل برنامج، ثم `total N failed F`) و`log.txt` في `%USERPROFILE%\sbarhive-tests` |

فحص كامل بعد أي تغيير: `build.sh` ثم `check-dlls.sh` على `sbarhive.exe` الناتج ثم `check.sh`، كما في القسم الإنكليزي.

لآلة اختبار ويندوز: ابنِ برامج الاختبار، ثم اجمعها في الصورة، ثم أرفق الصورة
بالآلة `win11` قرصاً مضغوطاً للقراءة فقط (السكربتات لا تشغّل هذا الأمر):

```
systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_TARGET_DIR=/home/sbarah/.cache/sbarhive-win-target cargo test --no-run -j 2 --target x86_64-pc-windows-gnu -p sbarhive-core -p sbarhive-engines -p sbarhive-cli
scripts/windows/make-iso.sh
virsh -c qemu:///session attach-disk win11 /home/sbarah/.cache/sbarhive-win-target/sbarhive-win-tests.iso sdb --type cdrom --mode readonly
```

ثم شغّل `D:\run-tests.cmd` (أو حرف محرك الأقراص المضغوطة) داخل الآلة.
