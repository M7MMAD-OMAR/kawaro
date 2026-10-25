[English](README.md) | [العربية](README.ar.md)

# سكربتات بناء ويندوز

تبني هذه السكربتات برامج ويندوز وتفحصها على لينكس، للهدف
`x86_64-pc-windows-gnu`. تحتاج `mingw64-gcc` وهدف Rust
(`rustup target add x86_64-pc-windows-gnu`). خارج CI (`CI=true`) يعيد
سكربتا البناء والفحص تشغيل نفسيهما داخل
`systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200%` بمهمتين،
ويطبعان `cgroup: <المسار>` أول سطر. مجلد البناء الافتراضي
`${XDG_CACHE_HOME:-$HOME/.cache}/kawaro-win-target`، وإن ضبطت `CARGO_TARGET_DIR`
بقي كما هو.

| الأمر | ما يفعله |
| --- | --- |
| `scripts/windows/build.sh` | يبني `x86_64-pc-windows-gnu/release/kawaro.exe` |
| `scripts/windows/check-dlls.sh <exe>` | يقرأ المكتبات المستوردة بـ `x86_64-w64-mingw32-objdump -p`، ويفشل (رمز 1) إن وُجدت أي `lib*.dll` لأن مكتبات mingw لا تُشحن، ويطبع `dll check: ok <name> <N> imports` إن كان الملف نظيفاً. ويقرأ `--text <file>` مخرج objdump محفوظاً بدلاً من ذلك |
| `scripts/windows/check.sh` | يشغّل clippy لويندوز على core وengines وسطر الأوامر، بكل الأهداف، والتحذيرات أخطاء |
| `scripts/windows/make-iso.sh` | لا يبني شيئاً؛ يجمع `kawaro.exe` من بناء الإصدار (`bin/`) وبرامج الاختبار من `debug/deps` (`tests/`) و`run-tests.cmd` و`SHA256SUMS` في صورة `kawaro-win-tests.iso` داخل مجلد البناء بـ `genisoimage -J -R` |
| `scripts/windows/run-tests.cmd` | على ويندوز يشغّل كل `tests\*.exe` من القرص بـ `--test-threads=1`، ويكتب `results.txt` (سطر `<exe> <errorlevel>` لكل برنامج، ثم `total N failed F`) و`log.txt` في `%USERPROFILE%\kawaro-tests` |

فحص كامل بعد أي تغيير:

```
scripts/windows/build.sh
scripts/windows/check-dlls.sh ${XDG_CACHE_HOME:-$HOME/.cache}/kawaro-win-target/x86_64-pc-windows-gnu/release/kawaro.exe
scripts/windows/check.sh
```

لآلة اختبار ويندوز: ابنِ برامج الاختبار، ثم اجمعها في الصورة، ثم أرفق الصورة
بالآلة `win11` قرصاً مضغوطاً للقراءة فقط (السكربتات لا تشغّل هذا الأمر):

```
systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_TARGET_DIR=${XDG_CACHE_HOME:-$HOME/.cache}/kawaro-win-target cargo test --no-run -j 2 --target x86_64-pc-windows-gnu -p kawaro-core -p kawaro-engines -p kawaro-cli
scripts/windows/make-iso.sh
virsh -c qemu:///session attach-disk win11 ${XDG_CACHE_HOME:-$HOME/.cache}/kawaro-win-target/kawaro-win-tests.iso sdb --type cdrom --mode readonly
```

ثم شغّل `D:\run-tests.cmd` (أو حرف محرك الأقراص المضغوطة) داخل الآلة.
