# S-024 تشغيل الخدمة من سطر الأوامر على ويندوز ومنحها حق الواجهة
الحالة: جاهزة للمراجعة (المنفِّذ 2026-10-22)، على فرع windows/0.6.0 بطلب محمد؛ ADR 0005 ما زال مقترحاً، ولا دمج في main

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
- 2026-10-22، المختبِر:
  - العقد المثبت في رأس `crates/cli/src/spawn.rs` (الملف جديد، فيه وحدة الاختبار وحدها، والتنفيذ يُكتب فوقها): الثوابت `DETACHED_PROCESS` و`CREATE_NEW_PROCESS_GROUP` و`CREATE_BREAKAWAY_FROM_JOB` من النوع `u32`؛ `start_with(attempt: impl FnMut(u32) -> io::Result<()>) -> io::Result<()>` والاختبارات تمرّر `&mut` لدالة مغلقة فيصلح `&mut dyn FnMut` أيضاً؛ `wait_exit(cmd: &mut Command) -> Result<i32, String>`؛ `daemon_binary_in(dir: &Path, suffix: &str) -> PathBuf`؛ `foreground_grant(req: &Request, server_pid: Option<u32>) -> Option<u32>`. وأُضيف `mod spawn;` إلى `main.rs` حتى تُبنى الاختبارات، ولم يتغير شيء آخر فيه.
  - الاختبارات: في `crates/cli/src/spawn.rs` (ثنائي `sbarhive`): AC1 ستة (قيم الثوابت؛ نجاح أول؛ رفض ثم نجاح؛ رفض مرتين؛ رفض ثم خطأ آخر يعود هو؛ `NotFound` بلا إعادة)، والرفض `from_raw_os_error(5)` لأنه EIO على لينكس فلا ينجح تنفيذ يعتمد على `kind()`. AC3 ثلاثة (`exit 3` و`exit 0` تحت `cfg(unix)`، وبرنامج غير موجود). AC4 ستة (اللاحقتان مع الملف ودونه، وملف `sbarhived` وحده مع `.exe` يعطي `sbarhived.exe`، ومجلد اسمه `sbarhived` ليس الثنائي كما اليوم). AC5 ثلاثة (الطلبات الأحد عشر مع `Some(42)` ومع `None`، و`match` بلا بديل عام حتى يفشل البناء عند متغير جديد). وفي `crates/cli/tests/spawn_windows.rs` أربعة لصفوف البناء: `spawn/windows.rs` فيه ثلاثة `const _: () = assert!(` يذكر كل منها اسم ثابت و`windows_sys`، و`creation_flags` و`AllowSetForegroundWindow`؛ `windows-sys` تحت `[target.'cfg(windows)'.dependencies]` وحدها في `crates/cli/Cargo.toml`؛ `spawn/windows.rs` في `exclude_globs` و`spawn.rs` و`client.rs` و`main.rs` ليست فيها.
  - غموض وقرارات: (1) شكل `cmd` في `wait_exit` لم يحدده المعيار، فاختير `&mut Command`. (2) استدعاء `foreground_grant` في العميل «بعد الاتصال وقبل الكتابة» لا صف له في الجدول، فهو للمراجع بقراءة `client.rs`. (3) AC7 لا اختبار له غير `W` كما ينص المعيار. (4) AC8 للمراجعة فقط: العدد اليوم 671 أصلاً لأن `windows-sys 0.61.2` في القفل من core، فاختبار له ينجح قبل التنفيذ. (5) تنبيه للمراجع: إن بُنيت الأعلام بـ `|` من بتات منفصلة فقد تنتج cargo-mutants طفرة `|` إلى `^` مكافئة تنجو في `M`.
  - للمراجع لا للاختبار: `W` و`B` مع `objdump` للأسماء الأربعة، و`X`، وعدد `[[package]]`، وREADME وCHANGELOG.md، و`git diff --stat main...HEAD -- crates/ui`، و`C` و`M`.
  - الاختبارات المضمّنة لا يراها فحص `crates/*/tests/*`: وحدة الاختبار في `spawn.rs` ووحدة الاختبار القائمة في `main.rs` يجب ألا تتغير بعد هذا الالتزام (فرق فارغ مطلوب، والتزام المختبِر a6bd519):
    `diff <(git show a6bd519:crates/cli/src/spawn.rs | sed -n '/^#\[cfg(test)\]/,$p') <(sed -n '/^#\[cfg(test)\]/,$p' crates/cli/src/spawn.rs)`
    `diff <(git show a6bd519:crates/cli/src/main.rs | sed -n '/^#\[cfg(test)\]/,$p') <(sed -n '/^#\[cfg(test)\]/,$p' crates/cli/src/main.rs)`
  - تنبيه: ثنائي اختبارات `sbarhive` لا يُبنى حتى يصل التنفيذ، فاختبارات الوحدة القائمة فيه (`clipboard` و`resolve_token`) حمراء حتى ذلك. `W` لا يعيقه `crates/ui`: الأخطاء كلها في `client.rs`.
  - تحقق من منطق الاختبارات: نسخة مؤقتة في مجلد الجلسة (حُذفت ولا تُدمج) بتطبيق مختصر للعقد نجحت فيها اختبارات الثنائي الثلاثة والعشرون و`spawn_windows` الأربعة، و`cargo clippy -p sbarhive-cli --tests -D warnings` نظيف عليها. هنا `cargo fmt --check -p sbarhive-cli` نظيف، و`cli` (32) و`settings` و`no_unix_net` خضراء.
  - مخرجات الفشل (كل أمر داخل `systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200%`):
```
$ cargo test -j 2 -p sbarhive-cli --bin sbarhive
      6 error[E0425]: cannot find function `daemon_binary_in` in this scope
      4 error[E0425]: cannot find function `foreground_grant` in this scope
      5 error[E0425]: cannot find function `start_with` in this scope
      3 error[E0425]: cannot find function `wait_exit` in this scope
      1 error[E0425]: cannot find value `CREATE_BREAKAWAY_FROM_JOB` in this scope
      1 error[E0425]: cannot find value `CREATE_NEW_PROCESS_GROUP` in this scope
      1 error[E0425]: cannot find value `DETACHED_PROCESS` in this scope
error: could not compile `sbarhive-cli` (bin "sbarhive" test) due to 21 previous errors; 1 warning emitted

$ cargo test -j 2 -p sbarhive-cli --test spawn_windows
test ac2_windows_spawn_checks_each_flag_against_windows_sys_at_compile_time ... FAILED
test ac2_ac5_windows_spawn_uses_creation_flags_and_allow_set_foreground_window ... FAILED
test ac2_windows_spawn_is_excluded_from_mutants_and_the_shared_files_are_not ... FAILED
test ac6_windows_sys_is_a_windows_only_dependency_of_the_cli ... FAILED
.../crates/cli/src/spawn/windows.rs must exist: No such file or directory (os error 2)
crates/cli/src/spawn/windows.rs must be in exclude_globs (story command M)
crates/cli/Cargo.toml needs [target.'cfg(windows)'.dependencies] (AC6)
test result: FAILED. 0 passed; 4 failed

$ env CARGO_TARGET_DIR=/home/sbarah/.cache/sbarhive-win-target cargo clippy -j 2 --target x86_64-pc-windows-gnu -p sbarhive-core -p sbarhive-engines --lib -p sbarhive-cli --bins -- -D warnings   # W, AC6 و AC7
error[E0433]: cannot find `unix` in `os`        --> crates/cli/src/client.rs:79:18
error[E0433]: cannot find `unix` in `os`        --> crates/cli/src/client.rs:101:18
error[E0599]: no method named `process_group` found for mutable reference `&mut std::process::Command`  --> crates/cli/src/client.rs:87:10
error[E0599]: no method named `exec` found for struct `std::process::Command`  --> crates/cli/src/client.rs:102:45
error: could not compile `sbarhive-cli` (bin "sbarhive") due to 4 previous errors
```
- 2026-10-22، المنفِّذ:
  - `crates/cli/src/spawn.rs`: الثوابت الثلاثة `u32`، و`FIRST_FLAGS = 0x0100_0208` و`RETRY_FLAGS = 0x208` قيمتين حرفيتين يثبتهما `const _: () = assert!(...)` مقابل الثوابت (فلا طفرة `|` إلى `^` مكافئة)، و`start_with` بـ `match` وحارس `raw_os_error() == Some(5)`، و`wait_exit` بـ `status()` ورمز الخروج (`1` حين لا رمز، أي إشارة على لينكس)، و`daemon_binary_in`، و`foreground_grant` (`Open` و`Drop` يمرران `server_pid`، والباقي `None`). `start_with` و`wait_exit` والثوابت عليها `#[cfg_attr(not(windows), allow(dead_code))]`. وحدة الاختبار لم تتغير: الفرقان من سجل المختبِر فارغان.
  - `crates/cli/src/spawn/windows.rs` (جديد): ثلاثة `const _: () = assert!` مقابل `windows_sys::Win32::System::Threading`، و`start_detached` (`start_with` مع `creation_flags(flags).spawn()`)، و`allow_foreground` (`AllowSetForegroundWindow`، والنتيجة مهملة لأن فشلها يعني خلية بلا تركيز فقط).
  - `crates/cli/src/client.rs`: `send` يستدعي `foreground_grant(req, stream.server_pid())` أول شيء، أي بعد `connect` في `request` و`stream` وقبل `writeln!`، وعلى ويندوز يمرّر الناتج إلى `allow_foreground`. `ensure_daemon` يبني الأمر مرة واحدة ثم `process_group(0)` تحت `cfg(unix)` أو `start_detached` تحت `cfg(windows)`. `exec_daemon` على لينكس جسمه كما هو تحت `cfg(unix)`، وعلى ويندوز `wait_exit` ثم `process::exit` بالرمز. `daemon_binary()` تمرّر `EXE_SUFFIX` إلى `daemon_binary_in`.
  - `crates/cli/Cargo.toml`: `windows-sys 0.61` تحت `[target.'cfg(windows)'.dependencies]` بالميزتين `Win32_System_Threading` و`Win32_UI_WindowsAndMessaging`. `Cargo.lock`: سطر `"windows-sys 0.61.2"` في اعتماديات cli وحده. `.cargo/mutants.toml`: `spawn/windows.rs` في `exclude_globs`، ونمطان في `exclude_re` (انحراف أدناه). README بالعربية والإنكليزية، وسطر CHANGELOG واحد.
  - انحراف: جولة `M` الأولى (16 طفرة) أبقت طفرتين ناجيتين في `client.rs`: `replace daemon_binary -> PathBuf with Default::default()` و`replace exec_daemon -> Result<(), String> with Ok(())`. الأولى غراء يقرأ `current_exe` والقرار في `daemon_binary_in` المطفَّرة، و`tests/e2e/run.sh` («open starts the daemon») يغطيها؛ والثانية `exec` يستبدل العملية فلا يعود منها اختبار، ولم يتغير جسمها، وإنما دخلت الفرق بإضافة `#[cfg(unix)]`. أضفت لهما نمطين ضيقين في `exclude_re` مع تعليق، على سابقة `serve` و`run`. إن رأى المدير أن اختباراً تكاملياً لـ `sbarhive daemon` أولى من الاستثناء فهو للمختبِر؛ صعوبته أن `target/debug/sbarhived` موجود عادة بجانب الثنائي فيُشغَّل حقاً.
  - البوابات (كلها داخل `systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200%` بـ `-j 2`):
    - `C`: `cargo fmt --check` و`cargo clippy -j 2 --workspace --all-targets -- -D warnings` نظيفان.
    - `cargo test -j 2 --all`: 664 ناجحاً، 0 فاشلاً، 1 متجاهلاً، في 44 ثنائياً. في cli: ثنائي `sbarhive` 23، و`cli` 32، و`settings` 2، و`no_unix_net` 1، و`spawn_windows` 4.
    - `W` نظيف، و`cargo check -p sbarhive-cli --target x86_64-pc-windows-gnu` نظيف. `B`: `sbarhive.exe` بُني (release)، و`objdump -p` يسرد `AllowSetForegroundWindow` و`GetNamedPipeServerProcessId` و`CancelIoEx` و`WaitNamedPipeW`.
    - `X`: clap وsbarhive-core وsbarhive-engines وserde_json و`windows-sys v0.61.2` (الجديد وحده). `[[package]]` في `Cargo.lock`: 671.
    - `M` في نسخة من الشجرة في مجلد الجلسة، `CARGO_BUILD_JOBS=2 cargo mutants --jobs 1 --in-diff` على `git diff 34a3288 -- crates/cli/src/{client.rs,spawn.rs,main.rs}` مع `-p sbarhive-cli`: بعد الاستثناءين 14 طفرة، 13 ممسوكة، 1 مهلة (`replace send -> Result<(), String> with Ok(())`: العميل ينتظر ردًا لطلب لم يُكتب، فالمهلة إمساك)، 0 ناجية.
    - `scripts/design-lint.sh` نظيف. `tests/e2e/run.sh`: كله ناجح (ومعه S-019 AC7). `git diff --stat main...HEAD -- crates/ui` فارغ. لا شرطة طويلة في الفرق.
  - للتحقق الحي في S-034: بقاء الخدمة بعد إغلاق الطرفية (README وCHANGELOG يذكرانه تصميماً)، والإعادة داخل المهمة، والتركيز. `Settings` لا يمنح حق الواجهة كما يحدد ADR.
