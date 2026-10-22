# S-025 المحركات الخارجية على ويندوز: شجرة العمليات والاكتشاف ورابط الملف
الحالة: S-025a: الاختبارات مكتوبة وتفشل، بانتظار المنفِّذ؛ S-025b لم تبدأ (فرع windows/0.6.0 بطلب محمد، 2026-10-22)؛ ADR 0005 ما زال مقترحاً، ولا دمج في main

يبني على ADR 0005 القسمين 3 و7. للإصدار 0.6.0. لا يلمس `crates/ui`. مستقلة عن S-022 إلى S-024 في الشيفرة.

## القصة
بصفتي مستخدماً على ويندوز ثبّت LibreOffice أو Ghostscript أو FFmpeg، أريد أن يجدها SbarHive في أماكنها المعتادة، وأن يقتل الإلغاء كل ما شغّلته، حتى يعمل التحويل دون إعداد ولا تبقى `soffice.bin` في الخلفية.

## التقسيم (بالترتيب، كل قصة فرعية يوم عمل أو أقل)
| القصة | المحتوى | المعايير | الملفات |
| --- | --- | --- | --- |
| S-025a | كائن المهمة بالتعليق ثم الإسناد ثم الاستئناف، و`is_executable` | AC1 إلى AC6 | `crates/engines/src/office.rs`، `crates/engines/src/office/{tree.rs,non_unix.rs}`، `crates/engines/Cargo.toml`، `Cargo.lock` (مصفوفة engines فقط)، `.cargo/mutants.toml`، `CHANGELOG.md` |
| S-025b | أسماء البرامج ومساراتها والسجل، و`file_url`، ورسالة المحرك الغائب | AC7 إلى AC12 | `crates/engines/src/{locate.rs,locate/windows.rs,office.rs,pdf.rs,ffmpeg.rs}`، `crates/engines/Cargo.toml` (ميزة السجل)، `.cargo/mutants.toml`، `docs/engines.md`، `README.md`، `CHANGELOG.md` |

## معايير القبول
S-025a:
- AC1: `tree::start_in_job(ops)` في ملف مشترك يُطفَّر، و`ops` سمة مُحقنة. التسلسل المسجّل بعمليات وهمية ناجحة: `create_job`، `kill_on_close`، `spawn_suspended`، `assign`، `threads_of(pid)`، `resume(tid)` لكل خيط. إن فشل `assign`: `terminate_process` ثم يعود الخطأ، ولا `resume` (عدّاد 0)، ولا يبقى مقبض مهمة مفتوح (عدّاد إغلاق 1).
- AC2: `tree::threads_of(pid, entries) -> Vec<u32>` صافية: `entries = [(10,1),(11,2),(10,3)]` تعطي `[1,3]` لـ 10 و`[]` لـ 12. قائمة فارغة لعملية معلّقة تعطي خطأً ولا تترك العملية (`terminate_process` مرة واحدة).
- AC3: `spawn_suspended` يضيف `CREATE_SUSPENDED` (0x4) و`CREATE_NO_WINDOW` (0x08000000) إلى أعلام الأمر، فلا تظهر نافذة طرفية لبرنامج طرفية تشغّله خدمة بلا طرفية. الثابتان في `tree.rs`، و`non_unix.rs` يتحقق منهما وقت الترجمة مقابل `windows_sys`.
- AC4: الإلغاء والمهلة والانتهاء الطبيعي على ويندوز تستدعي `TerminateJobObject` بدل `kill_group`، وإغلاق المقبض بـ `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. على لينكس يبقى `process_group(0)` و`killpg` كما اليوم، والاختبارات القائمة لـ `run_child` (الإلغاء يقتل المجموعة) تنجح دون تعديل في تأكيداتها.
- AC5: `tree::has_exe_extension(&str)` صافية، دون حساسية لحالة الأحرف: `a.exe` و`A.EXE` و`soffice.Com` صحيحة، و`a.bat` و`a.cmd` و`a` و`a.exe.txt` و`.exe` خاطئة. و`is_executable` على ويندوز تساوي `is_file() && has_exe_extension`.
- AC6: `cargo clippy -p sbarhive-engines --lib` للهدف ويندوز بلا تحذير. عدد `[[package]]` في `Cargo.lock` يبقى 671، و`X` يسرد `windows-sys v0.61.2` وحده جديداً. `non_unix.rs` باقٍ في `exclude_globs`.

S-025b:
- AC7: `locate::search(os, names, path_dirs, env, exe_dir, registry)` صافية بترتيب ADR: `PATH` أولاً، ثم المسارات المعروفة، ثم السجل. على ويندوز: LibreOffice بالاسمين `soffice.com` ثم `soffice.exe` في `%ProgramFiles%\LibreOffice\program` ثم مجلد `InstallPath` من `HKLM\SOFTWARE\LibreOffice\UNO\InstallPath`؛ وGhostscript بـ `gswin64c.exe` ثم `gswin32c.exe` في أحدث `%ProgramFiles%\gs\gs*\bin` ثم السجل؛ وFFmpeg بـ `ffmpeg.exe` و`ffprobe.exe` في `PATH` ثم `<exe_dir>\ffmpeg\bin`. على لينكس النتيجة كما اليوم (`soffice` ثم `libreoffice`، و`gs`، و`ffmpeg`).
- AC8: `locate::newest_gs(names)` صافية برقم الإصدار لا بالنص: `["gs9.56.1","gs10.04.0","gs10.10.0","gsx"]` تعطي `gs10.10.0`، و`["gs9.56.1","gs10.04.0"]` تعطي `gs10.04.0`، و`[]` تعطي `None`. والقاعدة نفسها لاختيار مفتاح الإصدار تحت `HKLM\SOFTWARE\GPL Ghostscript`.
- AC9: `locate/windows.rs` يقرأ السجل بـ `RegGetValueW`، والملف في `exclude_globs`. `cargo clippy -p sbarhive-engines --lib` للهدف بلا تحذير، وعدد `[[package]]` يبقى 671.
- AC10: `office::file_url_str(path: &str, windows: bool)` صافية، و`file_url` تستدعيها بـ `cfg!(windows)`. ويندوز: `C:\a b\x` تعطي `file:///C:/a%20b/x`، و`C:\ملف.docx` تعطي `file:///C:/%D9%85%D9%84%D9%81.docx`، و`\\?\C:\a` تعطي `file:///C:/a`. لينكس: `/a b/c` تعطي `file:///a%20b/c` كما اليوم، والاختبارات القائمة تنجح.
- AC11: رسالة `EngineError::Unavailable` حين لا يوجد البرنامج تذكر اسمه ورابط موقعه: `https://www.libreoffice.org/download/`، و`https://ghostscript.com/releases/`، و`https://ffmpeg.org/download.html`. رسائل «لا مُرمِّز» في ffmpeg (`missing`) كما هي، والاختبارات التي تفحص `libx264` تنجح.
- AC12: `docs/engines.md` يضيف عمود ويندوز (الأسماء والمسارات والسجل)، و`README.md` يذكر بالعربية والإنكليزية أين يبحث SbarHive عن المحركات على ويندوز وأنه لا ينزّلها.

لكل قصة فرعية:
- سطر واحد للقصة كلها في `CHANGELOG.md` تحت `[Unreleased]`، بالإنكليزية مع نص عربي بين «»: تضيفه S-025a، وS-025b تعدّل السطر نفسه.
- `git diff --stat main...HEAD -- crates/ui` فارغ.
- `C` بلا تحذير على لينكس: `start_in_job` و`threads_of` و`has_exe_extension` والثوابت في `tree.rs`، و`newest_gs` في `locate.rs`، لا يستدعيها إلا فرع ويندوز، فعليها `#[cfg_attr(not(windows), allow(dead_code))]`.
- `M` بلا طفرة ناجية.

## أوامر الفحص
أدوات القراءة وحدها (`grep`) تعمل خارج النطاق.
- `C`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% sh -c 'cargo fmt --check && cargo clippy -j 2 --workspace --all-targets -- -D warnings'`
- `L`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% cargo test -j 2 -p sbarhive-engines`
- `W`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_TARGET_DIR=/home/sbarah/.cache/sbarhive-win-target cargo clippy -j 2 --target x86_64-pc-windows-gnu -p sbarhive-engines --lib -- -D warnings`
- `M`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_BUILD_JOBS=2 cargo mutants --jobs 1 --in-diff <diff>` (الملفات المتغيرة وحدها؛ `non_unix.rs` و`locate/windows.rs` مستثنيان)
- `X`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_TARGET_DIR=/home/sbarah/.cache/sbarhive-win-target cargo tree --target x86_64-pc-windows-gnu -p sbarhive-engines -e normal --depth 1`

## حالات الاختبار
| معيار | الاختبار | النوع |
| --- | --- | --- |
| AC1، AC2 | وحدة في `tree.rs` بعمليات وهمية تسجّل التسلسل وعدّادات الإغلاق والإنهاء؛ تحت `M` | وحدة |
| AC3، AC6، AC9 | `W` بلا تحذير، والمراجع يقرأ `non_unix.rs` و`locate/windows.rs` | بناء، مراجعة |
| AC4 | اختبارات `run_child` القائمة في `office.rs` و`crates/engines/tests/office_engine.rs` على لينكس | تكامل |
| AC5 | وحدة بالأسماء التسعة | وحدة |
| AC7، AC8 | وحدة في `locate.rs` ببيئة وسجل وهميين، ومسارات مبنية بـ `Path::join`؛ تحت `M` | وحدة |
| AC10 | وحدة بالسلاسل الأربع | وحدة |
| AC11 | وحدة: محرك بلا برنامج يعيد رسالة فيها الاسم والرابط، لكل محرك من الثلاثة | وحدة |
| AC6 | `grep -c '^\[\[package\]\]' Cargo.lock` يعطي 671، و`X` | مراجعة |
| AC12 | المراجع يقرأ engines.md وREADME وCHANGELOG.md | مراجعة |

تبقى خضراء: `cargo test --workspace`، ومنها `office_engine.rs` و`pdf_ghostscript.rs` و`ffmpeg_engine.rs`.

## تحقق حي مؤجَّل إلى S-034
- إلغاء تحويل مستند يقتل `soffice.exe` و`soffice.bin` معاً، وموت الخدمة يقتلهما أيضاً.
- `soffice.com` ينتظر انتهاء التحويل (مُفترَض في ADR).
- مفتاحا السجل لـ LibreOffice وGhostscript موجودان بالأسماء المفترضة.
- بادئة `file:` في ffmpeg مع حرف القرص (`file:C:\...`).
- لا نافذة طرفية تومض عند تشغيل ffmpeg أو gswin64c.

## خارج النطاق
- تنزيل المحركات عند الحاجة: لا تنزيل في 0.6.0.
- ترجمة رسائل المحركات: تبقى بالإنكليزية كبقية أخطاء المحركات اليوم.
- مسارات UNC في `file_url`: المحركات تعمل في مجلد مؤقت محلي.

## سجل المراجعة
- 2026-10-22، المخطِّط: `CREATE_NO_WINDOW` إضافة لا يذكرها ADR؛ بدونها يفتح كل برنامج طرفية نافذة حين تشغّله خدمة بنظام `windows`. رسالة المحرك الغائب (ADR القسم 7) لم تُسند لقصة في جدول ADR، فوُضعت في S-025b لأنها في engines وحدها.
- 2026-10-22، المختبِر (S-025a):
  - العقد المثبت في رأس `crates/engines/src/office/tree.rs` (الملف جديد، فيه وحدة الاختبار وحدها، والتنفيذ يُكتب فوقها): الثابتان `CREATE_SUSPENDED` و`CREATE_NO_WINDOW` من النوع `u32`؛ السمة `JobOps` بنوعين مرافقين `Job` (مقبض مملوك يُغلق عند الإسقاط) و`Child`، وطرائقها `create_job` و`kill_on_close(&Job)` و`spawn_suspended() -> io::Result<(Child, u32)>` (العملية ومعرّفها) و`assign(&Job, &Child)` و`threads_of(pid) -> io::Result<Vec<u32>>` و`resume(tid)` و`terminate_process(&Child)` بلا قيمة معادة؛ `start_in_job<O: JobOps>(ops: &mut O) -> io::Result<(O::Job, O::Child)>`؛ `threads_of(pid: u32, entries: &[(u32, u32)]) -> Vec<u32>` والأزواج (معرّف العملية المالكة، معرّف الخيط)؛ `has_exe_extension(name: &str) -> bool`. وأُضيف `mod tree;` إلى `office.rs` بلا شرط منصة، ولم يتغير شيء آخر فيه.
  - الاختبارات في `crates/engines/src/office/tree.rs` (مكتبة `sbarhive-engines`): `ac3_flag_constants_have_the_win32_values`؛ `ac1_success_runs_the_sequence_and_keeps_the_job_open` (التسلسل كاملاً بـ `threads_of(10)` ثم `resume(1)` و`resume(3)`، ولا إنهاء، وعدّاد الإغلاق 0 ما دامت المهمة المعادة محفوظة ثم 1 بعد إسقاطها)؛ `ac1_assign_failure_terminates_returns_the_error_and_closes_the_job` (التسلسل ينتهي بـ `assign` ثم `terminate_process`، والخطأ المعاد هو خطأ `assign` برمزه 5، و`resume` صفر، والإنهاء 1، والإغلاق 1)؛ `ac2_threads_of_keeps_the_threads_of_one_pid_in_order`؛ `ac2_no_threads_is_an_error_and_terminates_the_suspended_process`؛ `ac5_has_exe_extension_for_the_nine_names`.
  - غموض وقرارات: (1) شكل السمة لم يحدده المعيار، فاختيرت الأنواع المرافقة حتى يصلح مقبض ويندوز و`std::process::Child`، ويُعدّ الإغلاق بإسقاط `Job` لا بطريقة `close` صريحة. (2) AC2 لا يذكر عدّاد الإغلاق للقائمة الفارغة؛ الاختبار يطلب 1 كما في AC1، لأن الخطأ لا يعيد المهمة فإما أن تُغلق أو تتسرّب. (3) AC5 يقول «الأسماء التسعة» والمعيار يسرد ثمانية (`a.exe` و`A.EXE` و`soffice.Com`، و`a.bat` و`a.cmd` و`a` و`a.exe.txt` و`.exe`)، فاختُبرت الثمانية ولم يُخترع تاسع. (4) فشل `create_job` أو `kill_on_close` أو `spawn_suspended` أو `threads_of` أو `resume` أو `terminate_process` لا يذكره المعيار، فلا اختبار له. (5) `is_executable` على ويندوز وثابتا AC3 مقابل `windows_sys` وجانب ويندوز من AC4 (`TerminateJobObject` و`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`) لا تُبنى على لينكس، فهي لـ `W` وللمراجع بقراءة `non_unix.rs`، ولا فحص نصي لها لأن الجدول يجعلها مراجعة.
  - للمراجع لا للاختبار: `W`، و`X` (اليوم لا يسرد `windows-sys` لـ engines، فهو أحمر قبل التنفيذ)، وعدد `[[package]]` (671 اليوم أصلاً، فاختبار له ينجح قبل التنفيذ)، و`non_unix.rs` في `exclude_globs`، وسطر CHANGELOG.md، و`git diff --stat main...HEAD -- crates/ui`، و`C` و`M`.
  - AC4 على لينكس: اختبارات `run_child` القائمة في وحدة اختبار `office.rs` و`crates/engines/tests/office_engine.rs` تبقى بلا تعديل. فحص `crates/*/tests/*` يغطي `office_engine.rs`؛ والوحدتان المضمّنتان يجب ألا تتغيرا بعد هذا الالتزام (فرق فارغ مطلوب، و`<T>` التزام المختبِر):
    `diff <(git show <T>:crates/engines/src/office/tree.rs | sed -n '/^#\[cfg(test)\]/,$p') <(sed -n '/^#\[cfg(test)\]/,$p' crates/engines/src/office/tree.rs)`
    `diff <(git show <T>:crates/engines/src/office.rs | sed -n '/^#\[cfg(test)\]/,$p') <(sed -n '/^#\[cfg(test)\]/,$p' crates/engines/src/office.rs)`
  - تنبيه: ثنائي اختبارات مكتبة engines لا يُبنى حتى يصل التنفيذ، فاختبارات الوحدة القائمة فيه (ومنها `run_child` في `office.rs`) حمراء حتى ذلك. اختبارات التكامل تُبنى: `office_engine` خضراء (12).
  - تحقق من منطق الاختبارات: نسخة مؤقتة في مجلد الجلسة (حُذفت ولا تُدمج) بتطبيق مختصر للعقد نجحت فيها الاختبارات الستة، و`cargo clippy --tests -D warnings` نظيف عليها. هنا `cargo fmt --check -p sbarhive-engines` نظيف.
  - مخرجات الفشل (كل أمر داخل `systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200%`):
```
$ cargo test -j 2 -p sbarhive-engines --lib
      1 error[E0405]: cannot find trait `JobOps` in this scope
      1 error[E0425]: cannot find function `has_exe_extension` in this scope
      3 error[E0425]: cannot find function `start_in_job` in this scope
      2 error[E0425]: cannot find function `threads_of` in this scope
      1 error[E0425]: cannot find value `CREATE_NO_WINDOW` in this scope
      1 error[E0425]: cannot find value `CREATE_SUSPENDED` in this scope
warning: unused import: `super::*`  --> crates/engines/src/office/tree.rs:31:9
error: could not compile `sbarhive-engines` (lib test) due to 9 previous errors; 1 warning emitted

$ cargo test -j 2 -p sbarhive-engines --test office_engine      # AC4، خط الأساس
test result: ok. 12 passed; 0 failed

$ env CARGO_TARGET_DIR=/home/sbarah/.cache/sbarhive-win-target cargo clippy -j 2 --target x86_64-pc-windows-gnu -p sbarhive-engines --lib -- -D warnings   # W، بوابة لا اختبار
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.50s

$ env CARGO_TARGET_DIR=/home/sbarah/.cache/sbarhive-win-target cargo tree --target x86_64-pc-windows-gnu -p sbarhive-engines -e normal --depth 1   # X
├── image v0.25.10  ├── lopdf v0.45.0  ├── miniz_oxide v0.8.9  ├── pdf-writer v0.15.0  ├── sbarhive-core v0.3.0
├── tempfile v3.27.0  ├── thiserror v2.0.21  ├── toml v0.8.23  ├── webp v0.3.1  └── zip v9.0.5   (لا windows-sys بعد)
```
