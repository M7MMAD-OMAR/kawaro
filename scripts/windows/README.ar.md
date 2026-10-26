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

## نسخة اختبار موقّعة

يوقّع `sign-local.sh` إصداراً لأجهزة ويندوز الخاصة بك بشهادة اختبار ذاتية التوقيع. ليست شهادة عامة: تبقى SmartScreen وأجهزة الآخرين تعامل الملفات كأنها غير موقّعة. أما على جهاز تستورد عليه الشهادة فيعرض ويندوز الناشر «Kawaro test signing» ويتحقق من أن الملفات لم تتغير.

```
scripts/windows/sign-local.sh test kawaro-0.4.0-beta.1-windows-x64.zip dist-signed
```

المصدر الأول هو مصدر الشهادة (S-083): `test` هي شهادة الاختبار الذاتية
في `~/.config/kawaro-signing/windows/`، و`pkcs12:PATH` هي شهادة توقيع حقيقية
في ملف PKCS#12 مع عبارة المرور في `KAWARO_WIN_SIGN_PASS`، وهذا هو المسار
الذي تستعمله خطوط الإصدار. يوقّع `--sign FILE...` الملفات في مكانها،
ويفحص `--verify FILE...` الملفات فقط. كل توقيع يحمل ختماً زمنياً من RFC 3161
ويُتحقق منه فور التوقيع؛ و`KAWARO_WIN_SIGN_TS` هي خادمة الختم الزمني ولا
يصح أن تكون فارغة.

يحتاج إلى `osslsigncode` و`openssl` و`wixl` و`zip` و`unzip`، ونسخة من المستودع بنفس إصدار الملف المضغوط. يبقى المفتاح مشفّراً في `~/.config/kawaro-signing/windows/`، ولا تنسخه أبداً إلى جهاز ويندوز.

على جهاز ويندوز انسخ `kawaro-test-signing.cer` وحزمة MSI من ذاكرة USB أو مجلد مشترك محلي (التنزيل من المتصفح يضيف علامة «منزّل من الإنترنت» التي تفحصها SmartScreen)، ثم في PowerShell بصلاحيات المسؤول:

```
certutil -addstore Root kawaro-test-signing.cer
certutil -addstore TrustedPublisher kawaro-test-signing.cer
```

احذفها بعد الاختبار بالأمر `certutil -delstore Root "Kawaro test signing (Muhmad Omar Haj Hmdo)"` ومثله لـ `TrustedPublisher`. لا تستطيع الشهادة إلا توقيع البرامج (`CA:FALSE` واستخدام توقيع الشيفرة)، فلا يمكنها إصدار شهادات لمواقع الويب.
