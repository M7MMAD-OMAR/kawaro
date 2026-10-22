# S-024 تشغيل الخدمة من سطر الأوامر على ويندوز ومنحها حق الواجهة
الحالة: معتمدة للتنفيذ على فرع windows/0.6.0 بطلب محمد (2026-10-22)؛ ADR 0005 ما زال مقترحاً، ولا دمج في main

يبني على ADR 0005 القسمين 3 و5 (التركيز)، وعلى S-022b (`Stream::server_pid`). للإصدار 0.6.0. لا يلمس `crates/ui`. يوم عمل واحد أو أقل، فلا تقسيم. هذه أول قصة يُبنى فيها `sbarhive.exe` للهدف ويندوز.

## القصة
بصفتي مستخدماً على ويندوز، أريد أن يشغّل `sbarhive open` الخدمة إن لم تعمل دون أن تموت حين أغلق الطرفية، وأن تأخذ الخلية التركيز حين تظهر، حتى أستعمل SbarHive من الطرفية كما على لينكس.

## الملفات
`crates/cli/src/{client.rs,spawn.rs,spawn/windows.rs,main.rs}`، `crates/cli/Cargo.toml` (windows-sys تحت `cfg(windows)`)، `Cargo.lock` (مصفوفة اعتماديات cli فقط)، `.cargo/mutants.toml`، `README.md`، `CHANGELOG.md`.

## معايير القبول
- AC1: `spawn::start_with(attempt)` صافية، و`attempt(flags: u32) -> io::Result<()>` مُحقنة. المحاولة الأولى بالأعلام `0x01000208` (`DETACHED_PROCESS` 0x8، و`CREATE_NEW_PROCESS_GROUP` 0x200، و`CREATE_BREAKAWAY_FROM_JOB` 0x01000000). إن فشلت بـ `raw_os_error() == Some(5)` تُعاد مرة واحدة بـ `0x208`. تسلسل الأعلام المسجّل: نجاح أول `[0x01000208]`، ورفض ثم نجاح `[0x01000208, 0x208]`، ورفض مرتين `[0x01000208, 0x208]` والخطأ الثاني يعود، و`NotFound` أول `[0x01000208]` ويعود دون إعادة.
- AC2: الثوابت الثلاثة معرّفة في `spawn.rs` المشترك، و`spawn/windows.rs` يتحقق منها وقت الترجمة مقابل `windows_sys` (`const _: () = assert!(...)` لكل ثابت)، فيفشل البناء للهدف إن اختلفت قيمة. التشغيل على ويندوز بـ `CommandExt::creation_flags`، وعلى لينكس يبقى `process_group(0)` كما اليوم.
- AC3: `spawn::wait_exit(cmd) -> Result<i32, String>` مشتركة، وهي `exec_daemon` على ويندوز: تشغيل ثم انتظار ثم رمز خروج الخدمة. على لينكس `sh -c 'exit 3'` تعطي `Ok(3)`، وبرنامج غير موجود يعطي `Err` نصه يبدأ بـ `cannot run sbarhived:`. `exec_daemon` على لينكس يبقى `exec` كما اليوم.
- AC4: `spawn::daemon_binary_in(dir, suffix)` صافية: مجلد مؤقت فيه ملف `sbarhived.exe` مع `suffix = ".exe"` يعطي مساره، ودونه يعطي `PathBuf::from("sbarhived.exe")`. ومع `suffix = ""` النتيجتان كما اليوم (`sbarhived`). `daemon_binary()` تمرّر `std::env::consts::EXE_SUFFIX`.
- AC5: `spawn::foreground_grant(req, server_pid) -> Option<u32>` صافية: `Some(pid)` لـ `Open` و`Drop` (بـ `hold` صحيح وخطأ) حين `server_pid` يساوي `Some(pid)`، و`None` لكل طلب آخر (`Ping` و`DropRelease` و`Run` و`Actions` و`CancelAll` و`Settings` و`Reload` و`Quit`) و`None` حين `server_pid` يساوي `None`. العميل يستدعيها بعد الاتصال وقبل كتابة الطلب، وعلى ويندوز يمرّر الناتج إلى `AllowSetForegroundWindow` في `spawn/windows.rs`.
- AC6: `cargo check` و`cargo clippy -p sbarhive-cli --bins` للهدف ويندوز بلا تحذير، و`cargo build --release -p sbarhive-cli` للهدف ينتج `sbarhive.exe`. و`x86_64-w64-mingw32-objdump -p sbarhive.exe` يسرد بين الاستيرادات `AllowSetForegroundWindow` و`GetNamedPipeServerProcessId` و`CancelIoEx` و`WaitNamedPipeW`.
- AC7: لا `std::os::unix` خارج كتل `cfg(unix)` في `crates/cli/src`: `cargo check -p sbarhive-cli` للهدف ينجح، وهذا وحده يثبته.
- AC8: لا حزمة جديدة: عدد `[[package]]` في `Cargo.lock` يبقى 671، و`X` يسرد `windows-sys v0.61.2` وحده جديداً.
- AC9: `README.md` يذكر بالعربية والإنكليزية أن `sbarhive open` على ويندوز يشغّل `sbarhived.exe` من مجلد البرنامج. سطر واحد في `CHANGELOG.md` تحت `[Unreleased]` بالإنكليزية مع نص عربي بين «». `git diff --stat main...HEAD -- crates/ui` فارغ. `C` بلا تحذير على لينكس: `start_with` و`wait_exit` والثوابت الثلاثة لا يستدعيها في ملف cli التنفيذي إلا فرع ويندوز، فعليها `#[cfg_attr(not(windows), allow(dead_code))]`. `M` بلا طفرة ناجية.

## أوامر الفحص
أدوات القراءة وحدها (`grep` و`objdump`) تعمل خارج النطاق.
- `C`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% sh -c 'cargo fmt --check && cargo clippy -j 2 --workspace --all-targets -- -D warnings'`
- `L`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% cargo test -j 2 -p sbarhive-cli`
- `W`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_TARGET_DIR=/home/sbarah/.cache/sbarhive-win-target cargo clippy -j 2 --target x86_64-pc-windows-gnu -p sbarhive-core -p sbarhive-engines --lib -p sbarhive-cli --bins -- -D warnings`
- `B`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_TARGET_DIR=/home/sbarah/.cache/sbarhive-win-target cargo build -j 2 --release --target x86_64-pc-windows-gnu -p sbarhive-cli`، ثم `x86_64-w64-mingw32-objdump -p /home/sbarah/.cache/sbarhive-win-target/x86_64-pc-windows-gnu/release/sbarhive.exe`
- `M`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_BUILD_JOBS=2 cargo mutants --jobs 1 --in-diff <diff>` (الملفات المتغيرة وحدها، ومنها `main.rs`؛ `spawn/windows.rs` مستثنى)
- `X`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_TARGET_DIR=/home/sbarah/.cache/sbarhive-win-target cargo tree --target x86_64-pc-windows-gnu -p sbarhive-cli -e normal --depth 1`

## حالات الاختبار
| معيار | الاختبار | النوع |
| --- | --- | --- |
| AC1 | وحدة بأربع دوال `attempt` وهمية، تؤكد تسلسل الأعلام والخطأ العائد؛ تحت `M` | وحدة |
| AC2، AC6، AC7 | `W` و`B`، و`grep` على مخرج `objdump` للأسماء الأربعة | بناء |
| AC3 | وحدة على لينكس بـ `sh -c 'exit 3'` وبمسار غير موجود | وحدة |
| AC4 | وحدة في مجلد مؤقت باللاحقتين | وحدة |
| AC5 | وحدة بكل متغيرات `Request` العشرة مع `Some(42)` ومع `None` | وحدة |
| AC8 | `grep -c '^\[\[package\]\]' Cargo.lock` يعطي 671، و`X` | مراجعة |
| AC9 | المراجع يقرأ README وCHANGELOG.md | مراجعة |

تبقى خضراء: `cargo test --workspace`، و`crates/cli/tests/cli.rs` (تشغيل الخدمة على لينكس)، و`tests/e2e/run.sh`.

## تحقق حي مؤجَّل إلى S-034
- `sbarhive open` من طرفية Windows Terminal ثم إغلاقها: الخدمة باقية.
- طرفية داخل مهمة تمنع الخروج منها: الإعادة دون `CREATE_BREAKAWAY_FROM_JOB` تنجح.
- لا نافذة طرفية للخدمة (يعتمد على `windows_subsystem` في S-027).
- الخلية تأخذ التركيز بعد `AllowSetForegroundWindow` (يعتمد على S-028).

## خارج النطاق
- وضع العميل في `sbarhived.exe open "%1"` من المستكشف، و`SetForegroundWindow` في الخدمة: S-028.
- `Settings` لا يمنح حق الواجهة، كما يحدد ADR («قبل Open وDrop»)؛ يُراجع في S-034 إن ظهرت نافذة الإعدادات خلف غيرها.

## سجل المراجعة
- 2026-10-22، المخطِّط: الأعلام أرقام في ملف مشترك يُطفَّر، وصحتها مقابل windows-sys تُثبت وقت الترجمة للهدف، لأن windows-sys غير متاح على لينكس.
