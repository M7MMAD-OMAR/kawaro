# S-043 المحركات الخارجية على ويندوز: شجرة العمليات والاكتشاف ورابط الملف
الحالة: S-043a: منتهية (PASS من المراجع، 2026-10-22)؛ S-043b: منتهية (PASS من المراجع، 2026-10-22، فرع windows/0.6.0 بطلب محمد)؛ ADR 0005 ما زال مقترحاً، ولا دمج في main
Renumbered on 2026-10-23: this story was S-025 on windows/0.6.0 before the merge into main (main had already used S-022 and S-027). Commit messages and review-log hashes keep the old number.

يبني على ADR 0005 القسمين 3 و7. للإصدار 0.6.0. لا يلمس `crates/ui`. مستقلة عن S-040 إلى S-042 في الشيفرة.

## القصة
بصفتي مستخدماً على ويندوز ثبّت LibreOffice أو Ghostscript أو FFmpeg، أريد أن يجدها SbarHive في أماكنها المعتادة، وأن يقتل الإلغاء كل ما شغّلته، حتى يعمل التحويل دون إعداد ولا تبقى `soffice.bin` في الخلفية.

## التقسيم (بالترتيب، كل قصة فرعية يوم عمل أو أقل)
| القصة | المحتوى | المعايير | الملفات |
| --- | --- | --- | --- |
| S-043a | كائن المهمة بالتعليق ثم الإسناد ثم الاستئناف، و`is_executable` | AC1 إلى AC6 | `crates/engines/src/office.rs`، `crates/engines/src/office/{tree.rs,non_unix.rs}`، `crates/engines/Cargo.toml`، `Cargo.lock` (مصفوفة engines فقط)، `.cargo/mutants.toml`، `CHANGELOG.md` |
| S-043b | أسماء البرامج ومساراتها والسجل، و`file_url`، ورسالة المحرك الغائب | AC7 إلى AC12 | `crates/engines/src/{locate.rs,locate/windows.rs,office.rs,pdf.rs,ffmpeg.rs}`، `crates/engines/Cargo.toml` (ميزة السجل)، `.cargo/mutants.toml`، `docs/engines.md`، `README.md`، `CHANGELOG.md` |

## معايير القبول
S-043a:
- AC1: `tree::start_in_job(ops)` في ملف مشترك يُطفَّر، و`ops` سمة مُحقنة. التسلسل المسجّل بعمليات وهمية ناجحة: `create_job`، `kill_on_close`، `spawn_suspended`، `assign`، `threads_of(pid)`، `resume(tid)` لكل خيط. إن فشل `assign`: `terminate_process` ثم يعود الخطأ، ولا `resume` (عدّاد 0)، ولا يبقى مقبض مهمة مفتوح (عدّاد إغلاق 1).
- AC2: `tree::threads_of(pid, entries) -> Vec<u32>` صافية: `entries = [(10,1),(11,2),(10,3)]` تعطي `[1,3]` لـ 10 و`[]` لـ 12. قائمة فارغة لعملية معلّقة تعطي خطأً ولا تترك العملية (`terminate_process` مرة واحدة).
- AC3: `spawn_suspended` يضيف `CREATE_SUSPENDED` (0x4) و`CREATE_NO_WINDOW` (0x08000000) إلى أعلام الأمر، فلا تظهر نافذة طرفية لبرنامج طرفية تشغّله خدمة بلا طرفية. الثابتان في `tree.rs`، و`non_unix.rs` يتحقق منهما وقت الترجمة مقابل `windows_sys`.
- AC4: الإلغاء والمهلة والانتهاء الطبيعي على ويندوز تستدعي `TerminateJobObject` بدل `kill_group`، وإغلاق المقبض بـ `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. على لينكس يبقى `process_group(0)` و`killpg` كما اليوم، والاختبارات القائمة لـ `run_child` (الإلغاء يقتل المجموعة) تنجح دون تعديل في تأكيداتها.
- AC5: `tree::has_exe_extension(&str)` صافية، دون حساسية لحالة الأحرف: `a.exe` و`A.EXE` و`soffice.Com` صحيحة، و`a.bat` و`a.cmd` و`a` و`a.exe.txt` و`.exe` خاطئة. و`is_executable` على ويندوز تساوي `is_file() && has_exe_extension`.
- AC6: `cargo clippy -p sbarhive-engines --lib` للهدف ويندوز بلا تحذير. عدد `[[package]]` في `Cargo.lock` يبقى 671، و`X` يسرد `windows-sys v0.61.2` وحده جديداً. `non_unix.rs` باقٍ في `exclude_globs`.

S-043b:
- AC7: `locate::search(os, names, path_dirs, env, exe_dir, registry)` صافية بترتيب ADR: `PATH` أولاً، ثم المسارات المعروفة، ثم السجل. على ويندوز: LibreOffice بالاسمين `soffice.com` ثم `soffice.exe` في `%ProgramFiles%\LibreOffice\program` ثم مجلد `InstallPath` من `HKLM\SOFTWARE\LibreOffice\UNO\InstallPath`؛ وGhostscript بـ `gswin64c.exe` ثم `gswin32c.exe` في أحدث `%ProgramFiles%\gs\gs*\bin` ثم السجل؛ وFFmpeg بـ `ffmpeg.exe` و`ffprobe.exe` في `PATH` ثم `<exe_dir>\ffmpeg\bin`. على لينكس النتيجة كما اليوم (`soffice` ثم `libreoffice`، و`gs`، و`ffmpeg`).
- AC8: `locate::newest_gs(names)` صافية برقم الإصدار لا بالنص: `["gs9.56.1","gs10.04.0","gs10.10.0","gsx"]` تعطي `gs10.10.0`، و`["gs9.56.1","gs10.04.0"]` تعطي `gs10.04.0`، و`[]` تعطي `None`. والقاعدة نفسها لاختيار مفتاح الإصدار تحت `HKLM\SOFTWARE\GPL Ghostscript`.
- AC9: `locate/windows.rs` يقرأ السجل بـ `RegGetValueW`، والملف في `exclude_globs`. `cargo clippy -p sbarhive-engines --lib` للهدف بلا تحذير، وعدد `[[package]]` يبقى 671.
- AC10: `office::file_url_str(path: &str, windows: bool)` صافية، و`file_url` تستدعيها بـ `cfg!(windows)`. ويندوز: `C:\a b\x` تعطي `file:///C:/a%20b/x`، و`C:\ملف.docx` تعطي `file:///C:/%D9%85%D9%84%D9%81.docx`، و`\\?\C:\a` تعطي `file:///C:/a`. لينكس: `/a b/c` تعطي `file:///a%20b/c` كما اليوم، والاختبارات القائمة تنجح.
- AC11: رسالة `EngineError::Unavailable` حين لا يوجد البرنامج تذكر اسمه ورابط موقعه: `https://www.libreoffice.org/download/`، و`https://ghostscript.com/releases/`، و`https://ffmpeg.org/download.html`. رسائل «لا مُرمِّز» في ffmpeg (`missing`) كما هي، والاختبارات التي تفحص `libx264` تنجح.
- AC12: `docs/engines.md` يضيف عمود ويندوز (الأسماء والمسارات والسجل)، و`README.md` يذكر بالعربية والإنكليزية أين يبحث SbarHive عن المحركات على ويندوز وأنه لا ينزّلها.
- AC13 (أضافه المدير في 2026-10-22 من ملاحظة مراجع S-043a): على ويندوز يُشغَّل ffmpeg وffprobe (`crates/engines/src/ffmpeg.rs`، كل مواضع `Command`) بالعلم `CREATE_NO_WINDOW` (القيمة 0x08000000 من `office::tree`)، فلا تومض نافذة طرفية. على لينكس لا تغيير. الإثبات: فحص نصي أن كل `Command::new` في ffmpeg.rs يمر بدالة واحدة تضيف العلم تحت `cfg(windows)`، وبناء W للهدف ويندوز نظيف.

لكل قصة فرعية:
- سطر واحد للقصة كلها في `CHANGELOG.md` تحت `[Unreleased]`، بالإنكليزية مع نص عربي بين «»: تضيفه S-043a، وS-025b تعدّل السطر نفسه.
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
| AC5 | وحدة بالأسماء الثمانية | وحدة |
| AC7، AC8 | وحدة في `locate.rs` ببيئة وسجل وهميين، ومسارات مبنية بـ `Path::join`؛ تحت `M` | وحدة |
| AC10 | وحدة بالسلاسل الأربع | وحدة |
| AC11 | وحدة: محرك بلا برنامج يعيد رسالة فيها الاسم والرابط، لكل محرك من الثلاثة | وحدة |
| AC6 | `grep -c '^\[\[package\]\]' Cargo.lock` يعطي 671، و`X` | مراجعة |
| AC12 | المراجع يقرأ engines.md وREADME وCHANGELOG.md | مراجعة |
| AC13 | فحص نصي لـ ffmpeg.rs، وW | وحدة |

تبقى خضراء: `cargo test --workspace`، ومنها `office_engine.rs` و`pdf_ghostscript.rs` و`ffmpeg_engine.rs`.

## تحقق حي مؤجَّل إلى S-052
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
- 2026-10-22، المخطِّط: `CREATE_NO_WINDOW` إضافة لا يذكرها ADR؛ بدونها يفتح كل برنامج طرفية نافذة حين تشغّله خدمة بنظام `windows`. رسالة المحرك الغائب (ADR القسم 7) لم تُسند لقصة في جدول ADR، فوُضعت في S-043b لأنها في engines وحدها.
- 2026-10-22، المختبِر (S-043a):
  - العقد المثبت في رأس `crates/engines/src/office/tree.rs` (الملف جديد، فيه وحدة الاختبار وحدها، والتنفيذ يُكتب فوقها): الثابتان `CREATE_SUSPENDED` و`CREATE_NO_WINDOW` من النوع `u32`؛ السمة `JobOps` بنوعين مرافقين `Job` (مقبض مملوك يُغلق عند الإسقاط) و`Child`، وطرائقها `create_job` و`kill_on_close(&Job)` و`spawn_suspended() -> io::Result<(Child, u32)>` (العملية ومعرّفها) و`assign(&Job, &Child)` و`threads_of(pid) -> io::Result<Vec<u32>>` و`resume(tid)` و`terminate_process(&Child)` بلا قيمة معادة؛ `start_in_job<O: JobOps>(ops: &mut O) -> io::Result<(O::Job, O::Child)>`؛ `threads_of(pid: u32, entries: &[(u32, u32)]) -> Vec<u32>` والأزواج (معرّف العملية المالكة، معرّف الخيط)؛ `has_exe_extension(name: &str) -> bool`. وأُضيف `mod tree;` إلى `office.rs` بلا شرط منصة، ولم يتغير شيء آخر فيه.
  - الاختبارات في `crates/engines/src/office/tree.rs` (مكتبة `sbarhive-engines`): `ac3_flag_constants_have_the_win32_values`؛ `ac1_success_runs_the_sequence_and_keeps_the_job_open` (التسلسل كاملاً بـ `threads_of(10)` ثم `resume(1)` و`resume(3)`، ولا إنهاء، وعدّاد الإغلاق 0 ما دامت المهمة المعادة محفوظة ثم 1 بعد إسقاطها)؛ `ac1_assign_failure_terminates_returns_the_error_and_closes_the_job` (التسلسل ينتهي بـ `assign` ثم `terminate_process`، والخطأ المعاد هو خطأ `assign` برمزه 5، و`resume` صفر، والإنهاء 1، والإغلاق 1)؛ `ac2_threads_of_keeps_the_threads_of_one_pid_in_order`؛ `ac2_no_threads_is_an_error_and_terminates_the_suspended_process`؛ `ac5_has_exe_extension_for_the_nine_names`.
  - غموض وقرارات: (1) شكل السمة لم يحدده المعيار، فاختيرت الأنواع المرافقة حتى يصلح مقبض ويندوز و`std::process::Child`، ويُعدّ الإغلاق بإسقاط `Job` لا بطريقة `close` صريحة. (2) AC2 لا يذكر عدّاد الإغلاق للقائمة الفارغة؛ الاختبار يطلب 1 كما في AC1، لأن الخطأ لا يعيد المهمة فإما أن تُغلق أو تتسرّب. (3) AC5 يقول «الأسماء التسعة» والمعيار يسرد ثمانية (`a.exe` و`A.EXE` و`soffice.Com`، و`a.bat` و`a.cmd` و`a` و`a.exe.txt` و`.exe`)، فاختُبرت الثمانية ولم يُخترع تاسع. (4) فشل `create_job` أو `kill_on_close` أو `spawn_suspended` أو `threads_of` أو `resume` أو `terminate_process` لا يذكره المعيار، فلا اختبار له. (5) `is_executable` على ويندوز وثابتا AC3 مقابل `windows_sys` وجانب ويندوز من AC4 (`TerminateJobObject` و`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`) لا تُبنى على لينكس، فهي لـ `W` وللمراجع بقراءة `non_unix.rs`، ولا فحص نصي لها لأن الجدول يجعلها مراجعة.
  - للمراجع لا للاختبار: `W`، و`X` (اليوم لا يسرد `windows-sys` لـ engines، فهو أحمر قبل التنفيذ)، وعدد `[[package]]` (671 اليوم أصلاً، فاختبار له ينجح قبل التنفيذ)، و`non_unix.rs` في `exclude_globs`، وسطر CHANGELOG.md، و`git diff --stat main...HEAD -- crates/ui`، و`C` و`M`.
  - AC4 على لينكس: اختبارات `run_child` القائمة في وحدة اختبار `office.rs` و`crates/engines/tests/office_engine.rs` تبقى بلا تعديل. فحص `crates/*/tests/*` يغطي `office_engine.rs`؛ والوحدتان المضمّنتان يجب ألا تتغيرا بعد هذا الالتزام (فرق فارغ مطلوب، و`c3619de` التزام المختبِر):
    `diff <(git show c3619de:crates/engines/src/office/tree.rs | sed -n '/^#\[cfg(test)\]/,$p') <(sed -n '/^#\[cfg(test)\]/,$p' crates/engines/src/office/tree.rs)`
    `diff <(git show c3619de:crates/engines/src/office.rs | sed -n '/^#\[cfg(test)\]/,$p') <(sed -n '/^#\[cfg(test)\]/,$p' crates/engines/src/office.rs)`
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
- 2026-10-22، المنفِّذ (S-043a):
  - التنفيذ: في `tree.rs` فوق وحدة الاختبار: الثابتان، والسمة `JobOps`، و`start_in_job` بمسار تنظيف واحد (مساعد `assign_and_resume` يُسند ثم يقرأ الخيوط ويعيد خطأً لقائمة فارغة ثم يستأنف، وعند أي خطأ بعد التشغيل: `terminate_process` ثم إغلاق المهمة ثم الخطأ)، و`threads_of`، و`has_exe_extension` (`.exe` أو `.com` بأي حالة وبعد جذع غير فارغ). كلها بـ `#[cfg_attr(not(windows), allow(dead_code))]`. في `non_unix.rs`: `WinOps` بـ `CreateJobObjectW` و`SetInformationJobObject` (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`) و`creation_flags(CREATE_SUSPENDED | CREATE_NO_WINDOW)` و`AssignProcessToJobObject` ولقطة خيوط Toolhelp تمر عبر `tree::threads_of` ثم `OpenThread` و`ResumeThread`، و`TerminateProcess` عند الفشل؛ و`spawn_in_job` و`terminate_job` (`TerminateJobObject`)، ومقبض مملوك يُغلق بالإسقاط؛ و`is_executable` تساوي `is_file() && has_exe_extension`؛ وتحقق وقت الترجمة من الثابتين مقابل `windows_sys`؛ وحُذف `kill_group` غير اللينكسي. في `run_child` سطرا تشغيل وإنهاء بـ `#[cfg(not(unix))]` فقط، وأسطر لينكس كما هي. `windows-sys` 0.61 تحت `[target.'cfg(windows)'.dependencies]` لـ engines بميزات Foundation وSecurity وDiagnostics_ToolHelp وJobObjects وThreading.
  - تصحيح نص القصة: AC5 ثمانية أسماء لا تسعة (قبِل المدير اختيار المختبِر)، فصُحّح صف جدول حالات الاختبار إلى «الثمانية»؛ اسم الاختبار `ac5_has_exe_extension_for_the_nine_names` باقٍ دون تعديل.
  - ملاحظة لـ S-043b: على ويندوز لا يجد `find_program` الاسمين `soffice` و`libreoffice` بلا امتداد حتى تضيف S-043b أسماء `.exe` و`.com`، وهذا متوقع.
  - البوابات (كل أمر داخل `systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200%` و`-j 2`): `C` نظيف (fmt وclippy للمساحة كلها بكل الأهداف)؛ `cargo test -j 2 --all` نجح 670 وفشل 0 (`sbarhive-engines` lib 51، و`office_engine` 12، و`pdf_ghostscript` 6، و`ffmpeg_engine` 23)؛ `W` نظيف، و`cargo check --target x86_64-pc-windows-gnu -p sbarhive-engines --lib` نظيف؛ `X` يسرد `windows-sys v0.61.2` وحده جديداً؛ `grep -c '^\[\[package\]\]' Cargo.lock` يعطي 671، وفرق `Cargo.lock` سطر واحد في مصفوفة `sbarhive-engines`؛ `scripts/design-lint.sh` نظيف؛ فرقا وحدتي الاختبار المضمّنتين فارغان؛ `git diff --stat main...HEAD -- crates/ui` فارغ.
  - `M` على نسخة منسوخة في مجلد الجلسة، بفرق `git diff c3619de -- crates/engines/src`، و`CARGO_BUILD_JOBS=2 cargo mutants --jobs 1 -p sbarhive-engines`: 12 طفرة، 10 ممسوكة، 2 غير قابلة للبناء، ولا ناجية.
- 2026-10-22، المراجع (S-043a): **PASS**.
  - البوابات (كل أمر داخل `systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200%` و`-j 2`): `cargo fmt --check` نظيف؛ `cargo clippy --workspace --all-targets -- -D warnings` نظيف؛ `cargo test --all` نجح 670 وفشل 0 (`sbarhive-engines` lib 51، و`office_engine` 12، و`pdf_ghostscript` 6، و`ffmpeg_engine` 23)؛ `W` نظيف و`cargo check --target x86_64-pc-windows-gnu -p sbarhive-engines --lib` نظيف؛ `X` يسرد `windows-sys v0.61.2` وحده جديداً؛ `[[package]]` في `Cargo.lock` 671؛ `scripts/design-lint.sh` نظيف؛ لا شرطة طويلة في الفرق؛ `git diff --stat main...HEAD -- crates/ui` فارغ؛ `git diff f800d92..HEAD -- 'crates/*/tests/*'` فارغ، وفرقا الوحدتين المضمّنتين فارغان.
  - `M` على نسخة منسوخة في مجلد الجلسة، `cargo mutants --in-place -p sbarhive-engines --in-diff` بفرق `f800d92..HEAD` و`CARGO_BUILD_JOBS=2`: 12 طفرة، 10 ممسوكة، 2 غير قابلة للبناء (`Default` لمقبض ونوع مرافق)، 0 ناجية، 0 مهلة.
  - ربط المعايير: AC1 وAC2 وAC3 (الثابتان) وAC5 باختبارات `tree.rs` الستة؛ AC4 على لينكس باختبارات `run_child` و`office_engine` دون تعديل، و`run_child` اللينكسي لم يتغير إلا بإضافة `#[cfg(unix)]`؛ AC3 وAC4 لويندوز وAC6 بـ `W` و`X` والقراءة.
  - تحقق من `non_unix.rs`: المهمة تُنشأ ويُضبط `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` قبل التشغيل؛ `CREATE_SUSPENDED | CREATE_NO_WINDOW` على الأمر، والثابتان مُتحقق منهما وقت الترجمة؛ مسار تنظيف واحد بعد التشغيل (`TerminateProcess` ثم إغلاق المهمة) لكل فشل، وإغلاق المهمة بالإسقاط عند فشل ما قبل التشغيل؛ مقبض المهمة غير قابل للوراثة (أمان افتراضي null)، فلا يمنع ابنٌ ورثه عملَ kill-on-close؛ مقبض لقطة Toolhelp يُلف بعد فحص `INVALID_HANDLE_VALUE` ويُغلق بالإسقاط؛ الاستئناف لخيوط معرّف الابن وحده عبر `tree::threads_of`، ولا إعادة استخدام للمعرّف لأن مقبض `Child` مفتوح؛ `OpenThread` بـ `THREAD_SUSPEND_RESUME`، وفشل `ResumeThread` يُفحص بـ `u32::MAX`؛ الإلغاء والمهلة والانتهاء تستدعي `TerminateJobObject`، والخروج المبكر بـ `try_wait()?` يتجاوزه لكن إسقاط المهمة يقتل الشجرة. كتل `unsafe` سليمة.
  - لا ينطبق: لقطات الشاشة والأداء ونصوص الواجهة (لا تغيير في `crates/ui` ولا نصوص). سطر CHANGELOG.md أضافه المنفِّذ كما تطلب القصة، فلا سطر ثانٍ.
  - ملاحظات غير مانعة:
    - متوسطة، خارج AC1 إلى AC6: `crates/engines/src/ffmpeg.rs:148` (و`:65` و`:77` و`:92`) يشغّل ffmpeg وffprobe بلا `CREATE_NO_WINDOW` ولا مهمة، والقائمة الحية في S-052 تنتظر «لا نافذة طرفية تومض عند تشغيل ffmpeg». لا قصة تغطيه اليوم؛ يقترح المراجع إسناده إلى S-043b أو قصة جديدة بقرار المخطِّط. سطر CHANGELOG.md يذكر LibreOffice وGhostscript وحدهما فهو دقيق.
    - منخفضة، انحراف مقبول: `crates/engines/src/office.rs:301` يشرط `mod non_unix` بـ `not(unix)` و`windows-sys` تحت `cfg(windows)`، فهدف ليس unix ولا windows لا يُبنى. لا هدف مدعوم يتأثر؛ يُقترح `cfg(windows)` لاحقاً.
    - مقبول: `find_program` بالأسماء المجردة لا يجد `soffice` على ويندوز حتى S-043b، ولم يكن يجده قبل هذا التغيير أيضاً، فلا تراجع.
- 2026-10-22, tester (S-043b):
  - Tests commit `863709c`. Contract pinned at the top of `crates/engines/src/locate.rs` (new file, test module only; the implementation goes above it), and `mod locate;` added to `crates/engines/src/lib.rs` (not in the story's file list, needed to compile the module):
    - `enum Os { Unix, Windows }` and `enum Program { Office, Ghostscript, Ffmpeg, Ffprobe }` (`Clone, Copy, Debug, PartialEq`); `fn names(os, program) -> &'static [&'static str]`.
    - `trait Registry { fn value(&self, key: &str, name: &str) -> Option<String>; fn subkeys(&self, key: &str) -> Vec<String>; }`, keys under HKLM without the hive (`SOFTWARE\LibreOffice\UNO\InstallPath`), `""` is the default value. The fake compares keys and names without case and accepts an optional `HKLM\` prefix.
    - `fn search(os: Os, program: Program, path_dirs: &[PathBuf], env: &dyn Fn(&str) -> Option<OsString>, exe_dir: Option<&Path>, registry: &dyn Registry) -> Option<PathBuf>`.
    - `fn newest_gs<'a>(names: &[&'a str]) -> Option<&'a str>`.
    - `office::file_url_str(path: &str, windows: bool) -> String`.
  - Tests:
    - `crates/engines/src/locate.rs`: `ac7_windows_names_per_program`, `ac7_unix_names_as_today`, `ac7_windows_office_path_wins_over_program_files_and_registry`, `ac7_windows_office_com_before_exe_in_one_dir`, `ac7_windows_office_program_files_wins_over_registry`, `ac7_windows_office_registry_install_path_last`, `ac7_windows_office_none_when_missing_everywhere`, `ac7_windows_ghostscript_path_wins_and_64_before_32`, `ac7_ac8_windows_ghostscript_newest_program_files_dir_by_number`, `ac7_windows_ghostscript_32_bit_name_in_program_files`, `ac7_ac8_windows_ghostscript_registry_newest_version_key_by_number`, `ac7_windows_ghostscript_none_when_missing_everywhere`, `ac7_windows_ffmpeg_path_wins_over_exe_dir`, `ac7_windows_ffmpeg_and_ffprobe_in_exe_dir_ffmpeg_bin`, `ac7_unix_searches_path_only`, `ac7_unix_ignores_program_files_exe_dir_and_registry`, `ac8_newest_gs_compares_versions_by_number`.
    - `crates/engines/src/office.rs` test module: `ac10_file_url_str_windows_drive_letter_and_space`, `ac10_file_url_str_windows_arabic_name`, `ac10_file_url_str_windows_drops_verbatim_prefix`, `ac10_file_url_str_unix_unchanged` (also `file_url` equals `file_url_str(.., false)` off Windows), `ac11_office_without_program_names_libreoffice_and_link`.
    - `crates/engines/src/pdf.rs` test module: `ac11_ghostscript_without_program_names_it_and_link` (`compress` and `to-images`).
    - `crates/engines/tests/windows_engines.rs`: `ac11_office_missing_program_names_libreoffice_and_link`, `ac11_ghostscript_missing_program_names_it_and_link`, `ac11_ffmpeg_missing_programs_names_ffmpeg_and_link`, `ac13_every_ffmpeg_command_goes_through_one_no_window_function`.
  - Ambiguities and choices:
    1. AC7's `names` slot is a `Program` enum, with ffprobe as its own program, because the known dirs and the registry depend on the program, not only on its names. `names(os, program)` gives the lists.
    2. Order inside `PATH` stays as `find_program` today (names first, then dirs), which is what "Linux as today" requires. Across tiers PATH, then known dirs, then registry. Within one dir `.com` before `.exe`. Name against dir ties across different dirs on Windows are not pinned.
    3. The registry trait needs subkey enumeration for the Ghostscript version keys; AC9 names only `RegGetValueW`, so `locate/windows.rs` also needs `RegEnumKeyExW` (or similar). The Ghostscript value read is `GS_DLL` (full path of the DLL in `bin`, its parent is the folder), and LibreOffice's is the default value of `...\UNO\InstallPath`. Both are my choice: ADR 0005 marks the keys as assumed, to be verified in the VM (S-052).
    4. The registry version rule is tested through `search` with bare version subkeys (`9.56.1` larger as text, `10.10.0` newest), not by calling `newest_gs` with names without `gs`.
    5. AC11 "when the program is missing" is tested both ways: no program at all (`program: None`, inline, built with struct update syntax, so only the field name `program` is pinned) and a configured path that does not exist (public `with_program` / `with_binaries`). Name checks: `LibreOffice`, `Ghostscript`, `ffmpeg` (any case).
    6. AC13: exactly one `Command::new(` in the non-test part of `ffmpeg.rs`, and the enclosing fn (with its attributes) holds `cfg(windows)`, `creation_flags` and `CREATE_NO_WINDOW`; `ffmpeg.rs` has no `const CREATE_NO_WINDOW` of its own. Call sites are not counted. The constant in `office/tree.rs` must become reachable from ffmpeg.rs (today `mod tree;` is private).
    7. The executable helper in `locate.rs` sets the exec bit under `#[cfg(unix)]` only, so the module also builds for the Windows target.
  - For the reviewer, not tests: `W` (and `locate/windows.rs` in `exclude_globs`), the `[[package]]` count, AC12 docs, CHANGELOG.md, `C`, `M`, and `git diff --stat main...HEAD -- crates/ui`.
  - Tests unchanged after `863709c` (all must print nothing). The S-043a command for `office.rs` (keyed on `c3619de`) is superseded by the one below, since this commit adds tests to that module:
    `git diff 863709c -- 'crates/*/tests/*'`
    `diff <(git show 863709c:crates/engines/src/locate.rs | sed -n '/^#\[cfg(test)\]/,$p') <(sed -n '/^#\[cfg(test)\]/,$p' crates/engines/src/locate.rs)`
    `diff <(git show 863709c:crates/engines/src/office.rs | sed -n '/^#\[cfg(test)\]/,$p') <(sed -n '/^#\[cfg(test)\]/,$p' crates/engines/src/office.rs)`
    `diff <(git show 863709c:crates/engines/src/pdf.rs | sed -n '/^#\[cfg(test)\]/,$p') <(sed -n '/^#\[cfg(test)\]/,$p' crates/engines/src/pdf.rs)`
    `diff <(git show 863709c:crates/engines/src/office/tree.rs | sed -n '/^#\[cfg(test)\]/,$p') <(sed -n '/^#\[cfg(test)\]/,$p' crates/engines/src/office/tree.rs)`
    `diff <(git show 863709c:crates/engines/src/ffmpeg.rs | sed -n '/^#\[cfg(test)\]/,$p') <(sed -n '/^#\[cfg(test)\]/,$p' crates/engines/src/ffmpeg.rs)`
  - Note: the engines lib test binary does not build until the implementation lands, so every existing unit test in it is red until then (as in S-043a). The integration tests build.
  - Logic check: a throwaway copy in the session scratch dir (deleted, not merged) with a short implementation of the contract passed all 74 lib tests and the 4 in `windows_engines.rs`, with clippy `--all-targets -D warnings` clean on Linux; on the Windows target the new test code compiled (the only errors were the pre-existing unix-only tests in `office.rs` and the stub's own private-constant use). It caught one contract wording error (PATH order), fixed in the header before the commit. Here `cargo fmt -p sbarhive-engines --check` is clean.
  - Failing output (each command inside `systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200%`):
```
$ cargo test -j 2 -p sbarhive-engines --lib
      2 error[E0405]: cannot find trait `Registry` in this scope
      7 error[E0425]: cannot find function `file_url_str` in this scope
      8 error[E0425]: cannot find function `names` in this scope
      3 error[E0425]: cannot find function `newest_gs` in this scope
      5 error[E0425]: cannot find function `search` in this scope
      2 error[E0425]: cannot find type `Program` in this scope
     13 error[E0433]: cannot find type `Os` in this scope
     33 error[E0433]: cannot find type `Program` in this scope
warning: unused import: `super::*`
error: could not compile `sbarhive-engines` (lib test) due to 73 previous errors; 1 warning emitted

$ cargo test -j 2 -p sbarhive-engines --test windows_engines
---- ac11_ffmpeg_missing_programs_names_ffmpeg_and_link stdout ----
panicked at crates/engines/tests/windows_engines.rs:71:5:
`/nonexistent/ffmpeg` or `/nonexistent/ffprobe` not found
---- ac11_ghostscript_missing_program_names_it_and_link stdout ----
panicked at crates/engines/tests/windows_engines.rs:55:5:
/nonexistent/gs not found
---- ac13_every_ffmpeg_command_goes_through_one_no_window_function stdout ----
panicked at crates/engines/tests/windows_engines.rs:109:5:
assertion `left == right` failed: every ffmpeg and ffprobe command must be built in one function; `Command::new(` at lines [65, 77, 92, 148]
  left: 4
 right: 1
---- ac11_office_missing_program_names_libreoffice_and_link stdout ----
panicked at crates/engines/tests/windows_engines.rs:40:5:
/nonexistent/soffice not found
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out

$ cargo test -j 2 -p sbarhive-engines --test office_engine --test pdf_ghostscript --test ffmpeg_engine   # stay green
test result: ok. 23 passed; 0 failed   (ffmpeg_engine)
test result: ok. 12 passed; 0 failed   (office_engine)
test result: ok. 6 passed; 0 failed    (pdf_ghostscript)
```
- 2026-10-22, implementer (S-043b):
  - Manager decisions applied: the tester's choices are accepted (Program enum with Ffprobe; PATH, then known dirs, then registry; `.com` before `.exe`; `GS_DLL` and the LibreOffice UNO `InstallPath` default value as assumptions to check in S-052; `RegEnumKeyExW` with the `Win32_System_Registry` feature). The S-043b file list is extended by the manager with `crates/engines/src/lib.rs` and the non-test part of `crates/engines/src/office/tree.rs`: `CREATE_NO_WINDOW` is now `pub(crate)` and `mod tree` is `pub(crate) mod tree` in `office.rs`, so `ffmpeg.rs` reads `crate::office::tree::CREATE_NO_WINDOW`. `lib.rs` is unchanged by me (the tester's `mod locate;` is enough).
  - `crates/engines/src/locate.rs` (above the tester's header and test module): `Os`, `Program`, `names`, `Registry`, `search`, `newest_gs` (one `newest(names, prefix)` helper with a `Vec<u32>` version and `max_by_key`, shared with the registry version subkeys), and `find(program)`, the live glue (`PATH`, `std::env::var_os`, `current_exe` folder, registry). `const OS` comes from `cfg!(windows)`, so the Windows tiers build and run on Linux and `newest_gs` needs no `allow(dead_code)` (clippy `--all-targets` is clean without it; the story asked for the attribute only if it was dead). Off Windows the registry is an empty `SystemRegistry` that `search` never asks.
  - `crates/engines/src/locate/windows.rs`: `SystemRegistry` over `RegGetValueW` (`RRF_RT_REG_SZ`, size query then read), `RegOpenKeyExW`, `RegEnumKeyExW` and `RegCloseKey`; added to `exclude_globs`.
  - Wiring: `OfficeEngine::default` and `GhostscriptEngine::default` call `locate::find` (still once per process); `FfmpegEngine::default` calls it for both programs and keeps the bare name when one is not found, so on Linux the stored name and the message stay as before. `find_program` stays in `office.rs` (its tests use it) and is step 1 of `search`.
  - AC10: `file_url_str(path, windows)`; `file_url` calls it with `cfg!(windows)`. Windows mode drops `\\?\`, turns `\` into `/`, prefixes `/` and keeps `:`; Unix mode is byte for byte the old encoder.
  - AC11: `office::missing(name, url, tried)` gives `"<name> not found[ at <path>]; install it from <url>"` for LibreOffice and Ghostscript, both for no program and for a spawn `NotFound`. ffmpeg keeps its path message and adds `; install FFmpeg from https://ffmpeg.org/download.html`. The encoder messages are unchanged.
  - AC13: one `fn command(bin)` in `ffmpeg.rs` holds the only `Command::new(`, with `#[cfg(windows)] cmd.creation_flags(crate::office::tree::CREATE_NO_WINDOW)` and `#[cfg_attr(not(windows), allow(unused_mut))]` on the `let mut`. The four call sites use it.
  - `.cargo/mutants.toml`: `locate/windows.rs` in `exclude_globs`; `exclude_re` adds `replace find -> Option<PathBuf> with None` (live glue; the Office and Ghostscript tests skip when the program is absent and ffmpeg falls back to the bare name the OS resolves the same way) and the two bodies of the empty off-Windows `SystemRegistry` (never asked on Unix). `find`'s other mutant (`Some(Default::default())`) stays and is caught.
  - Docs: `docs/engines.md` new section "Finding the programs (S-043b)" with Linux and Windows columns (names, install folder, registry, download page), the office "Program" bullet and the windows-sys row; `README.md` one paragraph in Arabic and one in English; the S-043 line in `CHANGELOG.md` edited (now "Windows engines (S-043)").
  - Gates (each cargo command inside `systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200%`, `-j 2`):
    - `cargo fmt --check`: clean. `cargo clippy -j 2 --workspace --all-targets -- -D warnings`: clean.
    - Windows target (`CARGO_TARGET_DIR=/home/sbarah/.cache/sbarhive-win-target`): `cargo clippy --target x86_64-pc-windows-gnu -p sbarhive-engines --lib -- -D warnings` clean; same for `-p sbarhive-cli` clean; `cargo check --target x86_64-pc-windows-gnu -p sbarhive-engines --lib -p sbarhive-cli` clean. `X` lists `windows-sys v0.61.2` as before; `[[package]]` count 671; `Cargo.lock` unchanged.
    - `cargo test -j 2 --all`: 697 passed, 0 failed, 1 ignored (engines lib 74, `windows_engines` 4).
    - `cargo mutants --jobs 1 --in-diff <git diff 8160592 -- crates/engines/src> -p sbarhive-engines` in a copied tree, `CARGO_BUILD_JOBS=2`: 43 mutants, 41 caught, 2 unviable (`command -> Command with Default::default()`, `missing -> EngineError with Default::default()`), 0 missed, 0 timeouts.
    - `scripts/design-lint.sh`: clean. No em or en dash in the diff.
    - The six test-unchanged commands above print nothing. `crates/ui` untouched.
  - Follow-up: `docs/engines.md` "Finding the programs" now says the lookup runs once per process for LibreOffice and Ghostscript and once per engine for ffmpeg (no `OnceLock` added). A grep found no old message text (`LibreOffice (soffice)`, `Ghostscript (gs)`) left in code or docs, and `fn find(` returning `Option<PathBuf>` exists only in `locate.rs`, so the new `exclude_re` entry hides no other mutant.
- 2026-10-22, reviewer (S-043b): **PASS**.
  - Gates (each cargo command inside `systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200%`, `-j 2`): `cargo fmt --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo test --all` 697 passed, 0 failed, 1 ignored (the ignored one is the fixture `regenerate` test, not S-043b; engines lib 74, `windows_engines` 4); Windows target (`CARGO_TARGET_DIR=/home/sbarah/.cache/sbarhive-win-target`) clippy `-D warnings` clean for `-p sbarhive-engines --lib` and `-p sbarhive-cli`, `cargo check` clean for both; `X` lists `windows-sys v0.61.2` as before; `[[package]]` in `Cargo.lock` 671, `Cargo.lock` unchanged since `8160592`; `scripts/design-lint.sh` clean; no em or en dash in `git diff 4be2ce1..HEAD`; `git diff --stat main...HEAD -- crates/ui` empty; the six tests-unchanged commands print nothing.
  - `M` in a copied tree (`git archive HEAD`), `git diff 8160592..HEAD -- crates/engines/src`, `CARGO_BUILD_JOBS=2 cargo mutants --in-place -p sbarhive-engines --in-diff`: 43 mutants, 41 caught, 2 unviable (`command` and `missing` with `Default::default()`), 0 missed, 0 timeouts.
  - `exclude_re` checked by listing, not grep: `cargo mutants --list --no-config --workspace` shows the three new patterns match only `locate.rs:179` (`find -> None`), the off-Windows `SystemRegistry` stub bodies in `locate.rs:200,203`, and `locate/windows.rs` (already in `exclude_globs`). The `find -> None` exclusion is justified: `find` only passes the live `PATH`, environment, `current_exe` and registry to `search`, which carries the order and is fully mutated; the engine tests skip when the program is absent, so the mutant is not observable without environment-dependent tests.
  - AC mapping: AC7 and AC8 by the 17 `locate::tests`; AC10 by the four `office::tests::ac10_*` (plus the old `file_url` tests unchanged); AC11 by `office::tests::ac11_*`, `pdf::tests::ac11_*` and three tests in `windows_engines.rs`, and the `libx264` encoder tests stay green; AC13 by `ac13_every_ffmpeg_command_goes_through_one_no_window_function` and `W`; AC9 and AC12 by reading and `W`.
  - Code checks: Linux lookup is `find_program(names, path_dirs)` with the same names as before and stops after `PATH` on Unix; `path_dirs` uses `std::env::split_paths`, so the Windows `PATH` tier splits on `;` with drive letters intact. Registry reader: `value` opens no key, sizes the buffer with `div_ceil(2)` from the queried byte count and passes `buf.len() * 2`, uses `RRF_RT_REG_SZ` (which also guarantees the nul, and `until_nul` cuts at it), and a value that grows between the two calls gives `None`; `subkeys` returns early only when `RegOpenKeyExW` failed (no handle), otherwise the single `RegCloseKey` after the loop runs on every path, and 256 characters covers the 255 character key name limit. `file_url_str` matches the four AC10 strings; it is only applied to the private temporary profile folder (`office.rs:115`), so UNC is not reachable. One `fn command` in `ffmpeg.rs` holds the only `Command::new` with `creation_flags(CREATE_NO_WINDOW)` under `cfg(windows)`. `sbarhive_engines::all()` runs once at daemon or CLI start, so the per engine ffmpeg lookup is off the hive path; no performance budget touched.
  - Not applicable: screenshots and UI strings (no `crates/ui` change). The CHANGELOG.md line is the single S-043 line edited as the story asks, so no second line.
  - Non-blocking notes:
    - Low, follow-up for S-052: a 64-bit build reads the 64-bit registry view and `%ProgramFiles%`, so a 32-bit LibreOffice or Ghostscript (under `WOW6432Node` and `Program Files (x86)`) is found only through `PATH`; in practice `gswin32c.exe` comes only from `PATH`. AC7 as written is met. Suggest checking `KEY_WOW64_32KEY` and `ProgramFiles(x86)` in the VM.
    - Low, for S-052 or the CLI story: `crates/cli/src/main.rs:348` (`on_path`) and the `doctor` tool list at `:362` check bare `ffmpeg`, `soffice` and `gs` in `PATH`, so on Windows `sbarhive doctor` reports them missing even when `locate::find` finds them. The engines section of `doctor` uses `available()` and is correct.
    - Low: the drive colon kept unencoded is the RFC 8089 form (`file:///C:/...`), not a limit; the S-052 live LibreOffice launch confirms it. UNC needs no follow-up while `file_url` is used only for the temporary profile.
    - Cosmetic: on Linux a found ffmpeg is now stored as an absolute path, so the "not found" message (only reached when `-version` fails) shows the full path instead of the bare name.
