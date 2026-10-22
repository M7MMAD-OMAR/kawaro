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

A full check after a change:

```
scripts/windows/build.sh
scripts/windows/check-dlls.sh /home/sbarah/.cache/sbarhive-win-target/x86_64-pc-windows-gnu/release/sbarhive.exe
scripts/windows/check.sh
```

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

فحص كامل بعد أي تغيير: الأوامر الثلاثة أعلاه بالترتيب نفسه.
