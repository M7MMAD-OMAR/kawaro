# S-025 المحركات الخارجية على ويندوز: شجرة العمليات والاكتشاف ورابط الملف
الحالة: معتمدة للتنفيذ على فرع windows/0.6.0 بطلب محمد (2026-10-22)؛ ADR 0005 ما زال مقترحاً، ولا دمج في main

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
