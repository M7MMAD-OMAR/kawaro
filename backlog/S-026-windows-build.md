# S-026 البناء المتقاطع لويندوز وبوابته وCI وصورة الاختبار
الحالة: معتمدة للتنفيذ على فرع windows/0.6.0 بطلب محمد (2026-10-22)؛ ADR 0005 ما زال مقترحاً، ولا دمج في main

يبني على ADR 0005 القسمين 1 و9، وعلى S-022 إلى S-025 (قبلها لا يُبنى `sbarhive.exe` للهدف). للإصدار 0.6.0. لا يلمس `crates/ui`.

## القصة
بصفتي مطوّراً على Fedora دون ويندوز، أريد أمراً واحداً يبني `sbarhive.exe` وملفات الاختبار لويندوز داخل نطاق ذاكرة محدود ويرفض أي مكتبة mingw مشحونة، وأن يفحص CI الشيفرة على ويندوز حقيقي، حتى لا يصل الآلة إلا بناء نظيف.

## التقسيم (بالترتيب، كل قصة فرعية يوم عمل أو أقل)
ADR يجعلها قصة واحدة، وهي أكبر من يوم، فقُسمت:
| القصة | المحتوى | المعايير | الملفات |
| --- | --- | --- | --- |
| S-026a | سكربت البناء، وفحص `objdump`، وبوابة clippy للهدف بكل الأهداف، وحصر اختبارات Unix | AC1 إلى AC7 | `scripts/windows/{build.sh,check-dlls.sh,check.sh,README.md}`، `tests/windows/fixtures/{objdump-ok.txt,objdump-mingw.txt}`، `crates/cli/tests/{cli.rs,settings.rs}`، `crates/core/tests/{output.rs,contracts.rs}`، `crates/engines/src/office.rs` (وحدة الاختبار)، `crates/engines/tests/{office_engine.rs,pdf_ghostscript.rs}`، وربما `.cargo/config.toml` (AC2)، `docs/engines.md`، `CHANGELOG.md` |
| S-026b | وظيفتا CI، وسكربت ISO، و`run-tests.cmd` | AC8 إلى AC11 | `.github/workflows/ci.yml`، `scripts/windows/{make-iso.sh,run-tests.cmd,README.md}`، `CHANGELOG.md` |

## معايير القبول
S-026a:
- AC1: `scripts/windows/build.sh` خارج نطاق يعيد تشغيل نفسه داخل `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200%`، وداخل CI (`CI=true`) يعمل مباشرة. يطبع أول سطر `cgroup: <المسار>`، وقيمة `memory.max` لذلك المسار تساوي `6442450944`. يبني بـ `-j 2` و`CARGO_TARGET_DIR=/home/sbarah/.cache/sbarhive-win-target` افتراضياً، وينتج `x86_64-pc-windows-gnu/release/sbarhive.exe`، ويخرج بـ 0.
- AC2: `scripts/windows/check-dlls.sh <exe>` يقرأ أسطر `DLL Name` من `x86_64-w64-mingw32-objdump -p`، ويقبل `--text <file>` للاختبار. `objdump-ok.txt` (مكتبات النظام وحدها) يعطي 0 وسطر `dll check: ok <name> <N> imports`. و`objdump-mingw.txt` (فيه `libwinpthread-1.dll` و`libgcc_s_seh-1.dll`) يعطي 1 ويسرد الاسمين. أي اسم يطابق `^lib.*\.dll$` دون حساسية للحالة مرفوض. على `sbarhive.exe` الحقيقي يعطي 0، وإن لم يعطه يُضاف `.cargo/config.toml` بـ `-C target-feature=+crt-static` للهدف وحده، ويُسجَّل ذلك هنا.
- AC3: `scripts/windows/check.sh` داخل النطاق نفسه يشغّل `cargo clippy -j 2 --target x86_64-pc-windows-gnu -p sbarhive-core -p sbarhive-engines -p sbarhive-cli --all-targets -- -D warnings` ويخرج بـ 0.
- AC4: اختبارات Unix وحدها تُحصر بـ `cfg(unix)` تهيئةً لا تغييراً في التأكيدات: على مستوى الملف `crates/cli/tests/cli.rs` و`settings.rs` (خوادم `UnixListener` و`PermissionsExt`)، وعلى مستوى الدالة اختبار الرابط المعلّق في `core/tests/output.rs:108`، و`socket_path_lives_in_the_runtime_dir` في `core/tests/contracts.rs:228`، واختبار `PermissionsExt` في `engines/src/office.rs`، واختبارات البرامج الوهمية بـ `sh` في `office_engine.rs` و`pdf_ghostscript.rs`. كل سطر مضاف في `git diff` لهذه الملفات سطر `cfg`، ولا سطر محذوف. وعدد الاختبارات الناجحة في `cargo test --workspace` على لينكس قبل التعديل وبعده متساوٍ.
- AC5: `cargo test --no-run -j 2 --target x86_64-pc-windows-gnu -p sbarhive-core -p sbarhive-engines -p sbarhive-cli` داخل النطاق ينجح، وعدد ملفات الاختبار `.exe` التي يسردها يساوي عدد أهداف الاختبار في الحزم الثلاث (يُسجَّل العدد هنا).
- AC6: لا حزمة جديدة: عدد `[[package]]` في `Cargo.lock` يبقى 671، ولا تغيير في `Cargo.lock` في هذه القصة (`git diff --quiet -- Cargo.lock`).
- AC7: `docs/engines.md` يسجّل أدوات البناء التي لا تُشحن: mingw64-gcc 16.2.1 وgenisoimage (مثبتان)، ورخصتيهما. `scripts/windows/README.md` يشرح الأوامر بالعربية والإنكليزية.

S-026b:
- AC8: `ci.yml` يضيف وظيفتين ولا يغيّر الوظيفة `check`: `git diff -U0 main...HEAD -- .github/workflows/ci.yml` فيه أسطر `+` فقط. `windows-cross` على `ubuntu-24.04` تثبّت `mingw-w64` وهدف `x86_64-pc-windows-gnu` ثم تشغّل `check.sh` و`build.sh` و`check-dlls.sh`. `windows-native` على `windows-latest` تثبّت FFmpeg (`choco install ffmpeg -y`)، لأن `ffmpeg_engine.rs:123` يؤكد وجوده ولا يتخطاه، ثم تشغّل `cargo test -p sbarhive-core -p sbarhive-engines -p sbarhive-cli`. اختبارات LibreOffice وGhostscript تتخطى نفسها حين يغيب البرنامج (`office_engine.rs:37`، `pdf_ghostscript.rs:91`)، فلا تثبيت لهما.
- AC9: `scripts/windows/make-iso.sh` يبني بـ `genisoimage -J -R` صورة `/home/sbarah/.cache/sbarhive-win-target/sbarhive-win-tests.iso` فيها `bin/sbarhive.exe`، وملفات الاختبار في `tests/`، و`run-tests.cmd`، و`SHA256SUMS`. `isoinfo -J -f -i <iso>` يسرد كل هذه، وعدد أسطر `SHA256SUMS` يساوي عدد الملفات التنفيذية، و`sha256sum` لـ `run-tests.cmd` المستخرج بـ `isoinfo -J -x` يساوي الأصل.
- AC10: `run-tests.cmd` بنهايات CRLF (`file` يقول `with CRLF line terminators`). يشغّل كل `tests\*.exe` بـ `--test-threads=1`، ويكتب في `%USERPROFILE%\sbarhive-tests\results.txt` سطراً `<exe> <errorlevel>` لكل ملف، وآخر سطر `total N failed F`، والمخرج الكامل في `log.txt` بجانبه. لا يكتب على القرص المضغوط.
- AC11: `scripts/windows/README.md` يضيف بالعربية والإنكليزية أمر الإرفاق `virsh -c qemu:///session attach-disk win11 <iso> sdb --type cdrom --mode readonly` دون تشغيله في هذه القصة.

لكل قصة فرعية:
- سطر واحد للقصة كلها في `CHANGELOG.md` تحت `[Unreleased]`، بالإنكليزية مع نص عربي بين «»: تضيفه S-026a، وS-026b تعدّل السطر نفسه.
- `git diff --stat main...HEAD -- crates/ui` فارغ، و`C` بلا تحذير على لينكس.
- `scripts/design-lint.sh` نظيف على الملفات الجديدة (لا شرطة طويلة).
- لا شيفرة Rust تنفيذية جديدة هنا (أسطر `cfg` في الاختبارات فقط)، فلا طفرات.

## أوامر الفحص
أدوات القراءة وحدها (`grep` و`file` و`objdump` و`isoinfo` و`sha256sum`) تعمل خارج النطاق، و`make-iso.sh` لا يبني شيئاً.
- `C`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% sh -c 'cargo fmt --check && cargo clippy -j 2 --workspace --all-targets -- -D warnings'`
- `L`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% cargo test -j 2 --workspace`
- `S`: `scripts/windows/build.sh` ثم `scripts/windows/check-dlls.sh /home/sbarah/.cache/sbarhive-win-target/x86_64-pc-windows-gnu/release/sbarhive.exe` ثم `scripts/windows/check.sh` (كلها تدخل النطاق بنفسها)
- `T`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_TARGET_DIR=/home/sbarah/.cache/sbarhive-win-target cargo test --no-run -j 2 --target x86_64-pc-windows-gnu -p sbarhive-core -p sbarhive-engines -p sbarhive-cli`

## حالات الاختبار
| معيار | الاختبار | النوع |
| --- | --- | --- |
| AC1 | `S`: سطر `cgroup:` و`cat /sys/fs/cgroup<المسار>/memory.max` يعطي 6442450944، ووجود `sbarhive.exe` | بناء |
| AC2 | `check-dlls.sh --text` على الملفين المزروعين برمزي خروج 0 و1 ونصيهما، ثم على `sbarhive.exe` | تكامل |
| AC3 | `check.sh` يخرج بـ 0 | بناء |
| AC4 | `git diff` للملفات السبعة أسطر `cfg` مضافة فقط؛ وعدد `test result: ok` في `L` قبل وبعد | مراجعة، وحدة |
| AC5 | `T` وعدّ ملفات `.exe` | بناء |
| AC6 | `grep -c '^\[\[package\]\]' Cargo.lock` يعطي 671، و`git diff --quiet -- Cargo.lock` | مراجعة |
| AC8 | أمر `git diff -U0` المذكور، والمراجع يقرأ الوظيفتين | مراجعة |
| AC9 | `make-iso.sh` ثم `isoinfo -J -f` و`isoinfo -J -x` و`sha256sum` | تكامل |
| AC10 | `file scripts/windows/run-tests.cmd`، والمراجع يقرأ الملف | مراجعة |
| AC7، AC11 | المراجع يقرأ engines.md وREADME وCHANGELOG.md | مراجعة |

## تحقق حي مؤجَّل إلى S-034
- أول نجاح للوظيفتين في CI: `ci.yml` لا يعمل إلا عند الدفع إلى main أو طلب دمج، والدفع والدمج خارج هذه القصة ويحتاجان قرار محمد.
- `run-tests.cmd` في الآلة: الملف والسطر الأخير، وعدد الإخفاقات.
- إرفاق ISO بالآلة وقراءته من محرك الأقراص.

## خارج النطاق
- `sbarhived.exe` وفحص مكتباته بـ `check-dlls.sh`: S-027 (الخدمة تعتمد على ui).
- المثبّت والملف المضغوط المحمول: S-033.
- wine، وأداة الضيف `qemu-guest-agent`: يحتاجان موافقة محمد.

## سجل المراجعة
- 2026-10-22، المخطِّط: ADR يطلب فحص `objdump` على `sbarhived.exe` في أول قصة بناء، لكنه لا يُربط قبل S-027، فيُفحص هنا `sbarhive.exe` ويُضاف `sbarhived.exe` في S-027. وقائمة «ما يحتاجه المالك» تطلب الإذن بـ xorriso أو genisoimage، لكن genisoimage وisoinfo مثبتان على المضيف، فلا يلزم إذن لهذه القصة.
