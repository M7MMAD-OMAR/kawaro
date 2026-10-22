# S-022 نقل IPC خلف واجهة واحدة، ثم named pipe على ويندوز
الحالة: معتمدة للتنفيذ على فرع windows/0.6.0 بطلب محمد (2026-10-22)؛ ADR 0005 ما زال مقترحاً، ولا دمج في main؛ S-022a: منتهية (PASS من المراجع 2026-10-22)؛ S-022b: مراجعة (نفّذها المنفِّذ 2026-10-22، تنتظر المراجع)

يبني على ADR 0005 القسم 2. للإصدار 0.6.0. لا يلمس `crates/ui`.

## القصة
بصفتي مستخدماً على ويندوز، أريد أن يتحدث `sbarhive.exe` و`sbarhived.exe` عبر أنبوب لا يفتحه غيري، حتى تعمل الخلية كما على لينكس دون أن يرسل مستخدم آخر أو جهاز آخر أوامر إلى خدمتي.

## التقسيم (بالترتيب، كل قصة فرعية يوم عمل أو أقل)
| القصة | المحتوى | المعايير | الملفات |
| --- | --- | --- | --- |
| S-022a | وحدة النقل بتطبيق Unix الحالي، ودالة اسم الأنبوب الصافية، وتحويل العميل والخدمة إليها | AC1 إلى AC7 | `crates/core/src/{lib.rs,transport.rs,transport/unix.rs,transport/windows.rs}` (بذرة)، `crates/cli/src/{client.rs,main.rs}`، `crates/daemon/src/{main.rs,server.rs,state.rs}`، `.cargo/mutants.toml`، `CHANGELOG.md` |
| S-022b | أنبوب ويندوز: DACL، ورفض البعيد، والنسخة الواحدة، والمهلة بـ `CancelIoEx`، ومالك الخادم | AC8 إلى AC14 | `crates/core/Cargo.toml`، `Cargo.lock` (مصفوفة اعتماديات core فقط)، `crates/core/src/transport.rs` و`transport/{windows.rs,deadline.rs}`، `.cargo/mutants.toml`، `SECURITY.md`، `docs/architecture.md`، `docs/engines.md`، `CHANGELOG.md` |

## معايير القبول
S-022a:
- AC1: `sbarhive_core::transport` يقدّم الواجهة كاملة (الأسماء مقترحة يثبّتها المختبِر): `endpoint() -> Endpoint`، و`Listener::bind(&Endpoint) -> io::Result<Bound>` حيث `Bound` إما `Listening(Listener)` أو `AlreadyRunning`، و`Listener::accept()`، و`Stream::connect(&Endpoint)`، و`Stream::set_read_timeout(Option<Duration>)`، و`Stream::try_clone()`، و`Stream::server_pid() -> Option<u32>` (تعيد `None` على Unix)، و`Stream` يطبّق `Read` و`Write`. `ipc::socket_path()` و`SOCKET_NAME` باقيان بتوقيعهما وقيمتهما.
- AC2: `transport::pipe_name(sid: &str, session: u32) -> String` دالة صافية: `pipe_name("S-1-5-21-1-2-3-1001", 1)` تساوي حرفياً `\\.\pipe\io.github.m7mmad_omar.SbarHive.S-1-5-21-1-2-3-1001.1`. SID مختلف أو جلسة مختلفة يعطي اسماً مختلفاً، وأطول SID ممكن (`S-1-5-21-4294967295-4294967295-4294967295-4294967295`) مع الجلسة `4294967295` يعطي 256 حرفاً أو أقل.
- AC3: على لينكس، `Listener` و`Stream` في مجلد مؤقت: سطر `{"cmd":"ping"}` من العميل يصل الخادم كما هو، ورده `{"status":"ok"}` يصل العميل. سطران يُكتبان من `Stream` ومن نسخته بـ `try_clone` يصلان بالترتيب نفسه.
- AC4: مهلة القراءة على لينكس: خادم يقبل ولا يكتب، والعميل بـ `set_read_timeout(Some(200 ms))`: `read_line` يعيد خطأً نوعه `ErrorKind::TimedOut` (لا `WouldBlock`) بعد 180 ms إلى 1000 ms. وبـ `None` يُقرأ رد كُتب بعد 300 ms.
- AC5: لا `std::os::unix::net` في `crates/cli/src` ولا `crates/daemon/src`: `grep -rn 'std::os::unix::net' crates/cli/src crates/daemon/src` يعيد 0 أسطر. `State::subscribers` من نوع `HashMap<JobId, Vec<transport::Stream>>`، و`sbarhive doctor` (`main.rs:384`) يطبع `endpoint()`.
- AC6: الخدمة: `bind` يعطي `AlreadyRunning` حين تجيب خدمة على `ping` خلال 500 ms، فتطبع `sbarhived: already running` وتخرج بـ 0، كما اليوم. ملف مقبس قديم لا يجيب يُحذف ثم يُربط.
- AC7: بذرة ويندوز `transport/windows.rs`: كل دالة فيها تعيد `ErrorKind::Unsupported`، والملف في `exclude_globs`. `cargo clippy -p sbarhive-core --lib` للهدف ويندوز بلا تحذير.

S-022b:
- AC8: `transport::dacl_sddl(sid) -> String` صافية: `dacl_sddl("S-1-5-21-1-2-3-1001")` تساوي `D:P(A;;GA;;;S-1-5-21-1-2-3-1001)`، ولا تحتوي `WD` ولا `AN` ولا `BU`. `windows.rs` يبني الواصف منها (`ConvertStringSecurityDescriptorToSecurityDescriptorW`) ويمرّره إلى كل `CreateNamedPipeW`.
- AC9: `transport::bind_result(first_instance_error: Option<u32>) -> BindOutcome` صافية: `None` تعطي استماعاً، و`Some(5)` (`ERROR_ACCESS_DENIED`) تعطي `AlreadyRunning`، وأي رمز آخر خطأً يحمل الرمز نفسه في `raw_os_error()`.
- AC10: `transport::deadline::read_with_deadline(timeout, read, cancel)` يشغّل خيط مراقب: قراءة وهمية تحجب حتى يُستدعى `cancel` تعيد `ErrorKind::TimedOut` بعد 180 ms إلى 1000 ms لمهلة 200 ms، و`cancel` يُستدعى مرة واحدة بالضبط. قراءة تنتهي خلال 50 ms تعيد بياناتها، و`cancel` يُستدعى 0 مرة، ولا يبقى خيط مراقب حي بعد العودة (عدّاد في الاختبار). على ويندوز `cancel` هو `CancelIoEx` على مقبض الأنبوب.
- AC11: `transport::connect_plan(results)` صافية لمحاولات الاتصال: `ERROR_FILE_NOT_FOUND` (2) تعطي `NotFound` فوراً دون انتظار، و`ERROR_PIPE_BUSY` (231) تعطي «انتظر بـ `WaitNamedPipeW` ثم أعد» حتى 2000 ms ثم `TimedOut`. ونتيجة التحقق من المالك: SID الخادم مختلف عن SID العميل تعطي `PermissionDenied` ولا يُرسل أي بايت (عدّاد كتابة في الاختبار 0).
- AC12: أعلام الإنشاء دالتان صافيتان في `transport.rs`: `open_mode(first: bool)` تعطي `0x00080003` للنسخة الأولى (`PIPE_ACCESS_DUPLEX` مع `FILE_FLAG_FIRST_PIPE_INSTANCE`) و`0x3` لما بعدها، فلا تفشل النسخ التالية بـ `ERROR_ACCESS_DENIED`. و`pipe_mode()` تعطي `0x8` (`PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT` قيمتها 0، مع `PIPE_REJECT_REMOTE_CLIENTS`). `windows.rs` يمرّرهما إلى `CreateNamedPipeW`، ويتحقق من كل ثابت وقت الترجمة مقابل `windows_sys` (`const _: () = assert!(...)`)، فيفشل `W` إن اختلفت قيمة. والتحقق من المالك بـ `GetNamedPipeServerProcessId` ثم رمز العملية، و`endpoint()` من `ProcessIdToSessionId` وSID من `GetTokenInformation(TokenUser)`. فحص الاستيراد في الملف التنفيذي للعميل يأتي في S-024 AC6.
- AC13: لا حزمة جديدة: عدد `[[package]]` في `Cargo.lock` يبقى 671، و`cargo tree --target x86_64-pc-windows-gnu -p sbarhive-core -e normal --depth 1` يسرد `windows-sys v0.61.2` وحده جديداً، و`cargo tree -p sbarhive-core -e normal --depth 1` على لينكس لا يسرده.
- AC14: `cargo check` و`cargo clippy -p sbarhive-core -p sbarhive-engines --lib` للهدف ويندوز بلا تحذير. `SECURITY.md` يصف الأنبوب (المستخدم وحده، لا عملاء عن بعد) بالإنكليزية والعربية، و`docs/architecture.md` يصف النسخة الواحدة على المنصتين، و`docs/engines.md` يسجّل windows-sys 0.61.2 (MIT OR Apache-2.0).

لكل قصة فرعية:
- سطر واحد للقصة كلها في `CHANGELOG.md` تحت `[Unreleased]`، بالإنكليزية مع نص عربي بين «»: تضيفه S-022a، وS-022b تعدّل السطر نفسه ولا تضيف غيره.
- `git diff --stat main...HEAD -- crates/ui` فارغ.
- `C` على لينكس بلا تحذير. الدوال الصافية التي لا يستدعيها إلا ملف ويندوز `pub` في core، أو عليها `#[cfg_attr(not(windows), allow(dead_code))]`.
- `M` بلا طفرة ناجية.

## أوامر الفحص
أدوات القراءة وحدها (`grep` و`file` و`objdump` و`isoinfo`) تعمل خارج النطاق.
- `C`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% sh -c 'cargo fmt --check && cargo clippy -j 2 --workspace --all-targets -- -D warnings'`
- `L`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% cargo test -j 2 -p sbarhive-core -p sbarhive-cli -p sbarhive-daemon`
- `W`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_TARGET_DIR=/home/sbarah/.cache/sbarhive-win-target cargo clippy -j 2 --target x86_64-pc-windows-gnu -p sbarhive-core --lib -- -D warnings` (ويُضاف `-p sbarhive-engines` في S-022b)
- `M`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_BUILD_JOBS=2 cargo mutants --jobs 1 --in-diff <diff>` (الملفات المتغيرة وحدها؛ ملفات ويندوز مستثناة بـ `exclude_globs`)
- `X`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_TARGET_DIR=/home/sbarah/.cache/sbarhive-win-target cargo tree --target x86_64-pc-windows-gnu -p sbarhive-core -e normal --depth 1`

## حالات الاختبار
| معيار | الاختبار | النوع |
| --- | --- | --- |
| AC1، AC3 | `crates/core/tests/transport.rs`: ربط واتصال ورحلة `ping` وترتيب السطرين عبر `try_clone`، و`server_pid()` تساوي `None`؛ `L` | تكامل |
| AC2، AC8، AC9، AC11، AC12 | وحدة بالقيم الحرفية والرموز 2 و5 و231 وقيم `open_mode` و`pipe_mode`، تحت `M`؛ و`W` يثبت مطابقة الثوابت | وحدة، بناء |
| AC4، AC10 | وحدة بالأزمنة المذكورة وعدّاد `cancel` وعدّاد الخيوط | وحدة |
| AC5 | الأمر `grep` المذكور يعيد 0 أسطر؛ `sbarhive doctor` في `crates/cli/tests/cli.rs` يطبع المسار في المجلد المؤقت | مراجعة، تكامل |
| AC6 | خدمتان على `XDG_RUNTIME_DIR` واحد: الثانية تخرج بـ 0 ونصها `already running`؛ ومقبس قديم يُستبدل | تكامل |
| AC7، AC14 | `W` بلا تحذير؛ المراجع يقرأ الوثائق الثلاث | بناء، مراجعة |
| AC13 | `grep -c '^\[\[package\]\]' Cargo.lock` يعطي 671، و`X`، و`cargo tree -p sbarhive-core -e normal --depth 1` داخل النطاق نفسه على لينكس | مراجعة |

تبقى خضراء: `cargo test --workspace`، و`crates/cli/tests/cli.rs` و`settings.rs` بخوادمهما الوهمية على `UnixListener` دون تعديل (دليل أن البروتوكول لم يتغير)، و`tests/e2e/run.sh`.

## تحقق حي مؤجَّل إلى S-034
- مستخدم ويندوز ثانٍ لا يفتح الأنبوب (`ERROR_ACCESS_DENIED`).
- اتصال من جهاز آخر عبر `\\<host>\pipe\...` مرفوض.
- `sbarhived.exe` ثانٍ يخرج بـ 0 فوراً، وعشر نسخ معاً تترك خادماً واحداً.
- عملية تسبق إلى الاسم بحساب آخر يرفضها العميل.
- مهلة 10 s في العميل و500 ms في فحص الخدمة تعملان بـ `CancelIoEx`.

## خارج النطاق
- تشغيل الخدمة وأعلامها و`AllowSetForegroundWindow`: S-024.
- فحص الخدمة كلها للهدف ويندوز: S-027 (تعتمد على ui).
- `crates/ui/examples/hive_bench.rs` يبقى على `UnixStream`: S-027.

## سجل المراجعة
- 2026-10-22، المخطِّط: الواجهة كلها تُثبَّت في S-022a (ومنها `server_pid` و`AlreadyRunning`) حتى لا تعيد S-022b وS-024 فتحها. قرارات ويندوز في دوال صافية تُختبر وتُطفَّر على لينكس، والاستدعاءات الرقيقة في `windows.rs` المستثنى.
- 2026-10-22، المختبِر (S-022a): كُتبت اختبارات AC1 إلى AC7. العقد الذي ثبّته المختبِر في رأس `crates/core/tests/transport.rs` (لا يُعدَّل): `transport::endpoint() -> Endpoint`، و`Endpoint: Clone + Debug + Display` يُعرض على Unix كما `ipc::socket_path().display()`، و`Endpoint::from_path(impl Into<PathBuf>)` على Unix حتى لا تغيّر الاختبارات `XDG_RUNTIME_DIR` (في edition 2024 `set_var` غير آمن ويتسابق مع الاختبارات المتوازية)، و`enum Bound { Listening(Listener), AlreadyRunning }`، و`Listener::bind(&Endpoint) -> io::Result<Bound>`، و`Listener::accept(&self) -> io::Result<Stream>`، و`Listener: Send`، و`Stream::connect(&Endpoint)`، و`set_read_timeout(&self, Option<Duration>) -> io::Result<()>`، و`try_clone(&self) -> io::Result<Stream>`، و`server_pid(&self) -> Option<u32>`، و`Stream: Read + Write + Send`، و`pipe_name(&str, u32) -> String` على كل المنصات.
  - الملفات والاختبارات:
    - `crates/core/tests/transport.rs`: `ac1_stream_is_read_write_send_and_listener_is_send`، `ac1_socket_name_and_socket_path_keep_their_value`، `ac1_endpoint_on_unix_is_the_socket_path`، `ac1_endpoint_from_path_displays_the_path`، `ac1_ac3_server_pid_is_none_on_unix`، `ac2_pipe_name_is_the_exact_literal`، `ac2_pipe_name_differs_by_sid_and_by_session`، `ac2_longest_sid_and_session_fit_in_256_chars`، `ac3_ping_line_reaches_the_server_and_the_reply_reaches_the_client`، `ac3_lines_from_a_stream_and_its_try_clone_arrive_in_order`، `ac4_read_timeout_is_timed_out_between_180_and_1000_ms`، `ac4_no_timeout_reads_a_reply_written_after_300_ms` (يضبط مهلة ثم يلغيها بـ `None`)، `ac6_bind_is_already_running_when_a_server_answers_ping`، `ac6_a_stale_socket_file_is_removed_and_bound`.
    - `crates/core/tests/transport_seed.rs`: `ac7_windows_seed_is_unsupported_and_excluded_from_mutants` (الملف موجود، فيه `ErrorKind::Unsupported` وبلا `todo!`، ومذكور في `exclude_globs`). بناء ويندوز نفسه يثبته `W` عند المراجع، فهو ينجح اليوم لأن الوحدة غير موجودة.
    - `crates/cli/tests/no_unix_net.rs`: `ac5_no_std_os_unix_net_in_cli_or_daemon_sources` (الأمر `grep` نفسه اختباراً).
    - `crates/cli/tests/cli.rs`: `ac5_doctor_prints_the_endpoint_in_the_runtime_dir`. لم يُضف أي استيراد لـ `transport` إلى هذا الملف، وخوادمه الوهمية باقية على `UnixListener` كما تطلب القصة.
    - `crates/daemon/tests/single_instance.rs`: `ac6_second_daemon_on_the_same_runtime_dir_says_already_running_and_exits_0`. الخدمة الأولى خادم وهمي على `transport::Listener`، والثانية `sbarhived` الحقيقي بلا متغيرات عرض، بمهلة 5 s ثم قتل.
  - ينجح قبل التنفيذ عن قصد: `ac5_doctor_prints_the_endpoint_in_the_runtime_dir`، لأن `doctor` يطبع مسار المقبس اليوم، وسلوك لينكس يجب ألا يتغير. هو حارس تراجع لا دليل تنفيذ. وكذلك سلوك AC6 موجود اليوم، وما يجعل اختباره يفشل هو الخادم الوهمي المبني على `transport`.
  - للمراجع لا للاختبار: نوع `State::subscribers` (`HashMap<JobId, Vec<transport::Stream>>`) لأن الخدمة crate تنفيذية وفحص النص هش، و`W` لـ AC7.
  - غموض: مقبس فيه مستمع يقبل ولا يجيب على `ping` خلال 500 ms: هل يُعدّ قديماً فيُحذف؟ AC6 لا يحدد، فلم يُختبر.
  - تحقق من منطق اختبار الخدمة: نسخة مؤقتة غير مثبتة بخادم وهمي على `UnixListener` نجحت على `sbarhived` الحالي (0.02 s)، فلا يحجبه عرض ولا أيقونة. و`cargo clippy -D warnings` نظيف على `sbarhive-cli --tests` و`transport_seed`؛ أما `transport.rs` و`single_instance.rs` فلا يُفحصان قبل وجود الواجهة، فأي تحذير فيهما من الاختبار لا من التنفيذ.
  - مخرجات الفشل (`L` داخل النطاق، `-j 2`):
```
$ cargo test -j 2 -p sbarhive-core --test transport
error[E0432]: unresolved import `sbarhive_core::transport`
  --> crates/core/tests/transport.rs:28:20
   |
28 | use sbarhive_core::transport::{self, Bound, Endpoint, Listener, Stream};
   |                    ^^^^^^^^^ could not find `transport` in `sbarhive_core`
error: could not compile `sbarhive-core` (test "transport") due to 1 previous error

$ cargo test -j 2 -p sbarhive-core --test transport_seed
---- ac7_windows_seed_is_unsupported_and_excluded_from_mutants stdout ----
panicked at crates/core/tests/transport_seed.rs:12:29:
.../crates/core/src/transport/windows.rs must exist: No such file or directory (os error 2)
test result: FAILED. 0 passed; 1 failed

$ cargo test -j 2 -p sbarhive-cli --test no_unix_net --test cli -- ac5
test ac5_doctor_prints_the_endpoint_in_the_runtime_dir ... ok
---- ac5_no_std_os_unix_net_in_cli_or_daemon_sources stdout ----
4 lines:
crates/cli/src/client.rs:5: use std::os::unix::net::UnixStream;
crates/daemon/src/main.rs:19: use std::os::unix::net::{UnixListener, UnixStream};
crates/daemon/src/server.rs:8: use std::os::unix::net::{UnixListener, UnixStream};
crates/daemon/src/state.rs:10: use std::os::unix::net::UnixStream;
test result: FAILED. 0 passed; 1 failed

$ cargo test -j 2 -p sbarhive-daemon --test single_instance
error[E0432]: unresolved import `sbarhive_core::transport`
 --> crates/daemon/tests/single_instance.rs:8:20
  |
8 | use sbarhive_core::transport::{Bound, Endpoint, Listener};
  |                    ^^^^^^^^^ could not find `transport` in `sbarhive_core`
error: could not compile `sbarhive-daemon` (test "single_instance") due to 1 previous error
```
- 2026-10-22، المنفِّذ (S-022a): الوحدة `crates/core/src/transport.rs` (`Bound` و`pipe_name` والتوجيه حسب المنصة)، و`transport/unix.rs` (المقبس بغلاف يحوّل `WouldBlock` إلى `TimedOut` في `Read`، و`bind` يرسل `ping` بمهلة 500 ms كما كان `already_running()` في الخدمة)، و`transport/windows.rs` بذرة تعيد `ErrorKind::Unsupported` و`endpoint()` فيها اسم فارغ حتى S-022b. العميل (`client.rs`) والخدمة (`main.rs` و`server.rs` و`state.rs`) و`doctor` تحوّلت إليها. لم يُعدَّل أي اختبار.
  - قرار المدير في الغموض: مستمع يقبل ولا يجيب على `ping` خلال 500 ms يبقى كما اليوم بالضبط: يُعدّ غير عامل، فيُحذف الملف ويُربط. وكذلك رد غير `ok` أو فشل الكتابة.
  - حذف الملف عند خروج الخدمة باقٍ على `ipc::socket_path()` كما كان.
  - `C`: `cargo fmt --check` نظيف، و`cargo clippy -j 2 --workspace --all-targets -- -D warnings` بلا تحذير (ومنه `transport.rs` و`single_instance.rs` في الاختبارات).
  - `W`: `cargo clippy --target x86_64-pc-windows-gnu -p sbarhive-core --lib -- -D warnings` بلا تحذير.
  - `L`: `cargo test -j 2 -p sbarhive-core -p sbarhive-cli -p sbarhive-daemon`: 210 ناجحاً، 0 فاشلاً (منها 14 في `transport.rs`، و1 في كل من `transport_seed.rs` و`no_unix_net.rs` و`single_instance.rs`، و32 في `cli.rs`).
  - AC5: `grep -rn 'std::os::unix::net' crates/cli/src crates/daemon/src` يعيد 0 أسطر. `git diff --stat main...HEAD -- crates/ui` فارغ.
  - `M` على الفرق من `20502bc`: 26 طفرة، 14 مكتشفة، 8 غير قابلة للبناء، 2 مهلة (`read` و`write` بـ `Ok(1)`، تكرار لا ينتهي فتكشفه المهلة)، و2 ناجيتان استُثنيتا في `exclude_re` مع السبب: `replace serve with ()` (حلقة القبول في الخدمة الحية، يغطيها `tests/e2e/run.sh` كسابقة `handle`)، و`flush -> Ok(())` في `unix.rs` (طفرة مكافئة: `flush` للمقبس في std لا يفعل شيئاً). بعد الاستثناء `cargo mutants --list` يسرد 24 طفرة، وليس بينها ناجية معروفة.
  - `scripts/design-lint.sh`: clean. سطر CHANGELOG تحت `[Unreleased]`.
  - تتمة: `tests/e2e/run.sh` على بناء release من هذا الفرع (KWin بلا شاشة، HOME مؤقت) نجح كله: «e2e: all passed» و«e2e S-019 AC7: all passed»، ومنه `open` و`drop` يشغّلان الخدمة، و«single instance»، و«second instance leaves the socket alone»، و«quit stops the daemon and removes the socket». و`cargo test -j 2 --workspace`: 581 ناجحاً، 0 فاشلاً.
  - عن `M`: مع المهلتين يخرج `cargo mutants` بالرمز 3 لا 0. المهلتان `read` و`write` بـ `Ok(1)`، تكشفهما الاختبارات بالدوران حتى المهلة، ولا يُسرَّع كشفهما دون تعديل الاختبارات.
- 2026-10-22، المراجع (S-022a): **PASS**. كل البوابات أعيد تشغيلها داخل النطاق (`MemoryMax=6G`، `-j 2`) من شجرة `windows/0.6.0`:
  - `C`: `cargo fmt --check` نظيف، و`cargo clippy -j 2 --workspace --all-targets -- -D warnings` انتهى بـ `Finished` بلا تحذير.
  - `W`: `cargo clippy --target x86_64-pc-windows-gnu -p sbarhive-core --lib -- -D warnings` بعد لمس `lib.rs` لإجبار الفحص: رمز الخروج 0، بلا تحذير.
  - `cargo test -j 2 --all`: 581 ناجحاً، 0 فاشلاً، 1 متجاهل. اختبارات AC1 إلى AC7 السبعة عشر كلها ناجحة، ولكل معيار اختبار واحد على الأقل (AC5 بنوع `State::subscribers` وAC7 بـ `W` تحقق منهما المراجع في الشيفرة).
  - `M`: `cargo mutants --jobs 1 --in-diff` على `git diff 20502bc..HEAD -- crates` بـ `CARGO_BUILD_JOBS=2`: 24 طفرة، 14 مكتشفة، 8 غير قابلة للبناء، 2 مهلة، **0 ناجية** (`missed.txt` فارغ). المهلتان `read` و`write` بـ `Ok(1)` في `unix.rs:103` و`unix.rs:109`: الاختبارات تدور حتى المهلة فتفشل، فليستا ناجيتين، ورمز الخروج 3 سببه المهلة وحدها.
  - `scripts/design-lint.sh`: clean. لا شرطة طويلة في فرق `main...HEAD`. سطر CHANGELOG واحد للقصة تحت `[Unreleased]` بالإنكليزية مع نص عربي. `git diff --stat main...HEAD -- crates/ui` فارغ. `Cargo.lock`: 671 `[[package]]` كما في `main`، ولا تغيير في أي `Cargo.toml`. `grep -rn 'std::os::unix::net' crates/cli/src crates/daemon/src`: 0 أسطر.
  - الاختبارات لم تُعدَّل بعد المختبِر: `git diff 20502bc..HEAD -- crates/*/tests` فارغ.
  - اللقطات: لا تغيير في الواجهة فلا لقطة جديدة. ميزانية الأداء: لا تمس مسار ظهور الخلية؛ الكلفة الوحيدة الجديدة `dup` واحد لكل طلب في `handle`.
  - الانحرافات مقبولة: (1) `replace serve with ()` في `exclude_re` على سابقة `handle`، ولم يُعِد المراجع تشغيل `tests/e2e/run.sh`، فالقبول على السابقة وعلى سجل المنفِّذ. (2) `flush -> Ok(())` مكافئة: `flush` لمقبس Unix في std يعيد `Ok(())` (مصدر std غير مثبت محلياً للاقتباس منه). (3) `endpoint()` و`server_pid()` و`Display` في بذرة ويندوز لا تستطيع إعادة `Unsupported` بتوقيعات AC1، فالاسم الفارغ و`None` مفروضان؛ وكل دالة تعيد `io::Result` تعيد `Unsupported`. (4) `handle()` يقرأ عبر `try_clone` لأن `Read` غير مطبّق على `&Stream`؛ السلوك على لينكس كما كان، وفشل النسخ يغلق الاتصال بلا رد.
  - ملاحظات صغيرة لما بعد: `crates/daemon/src/main.rs:84` ما زال يحذف `socket_path()` مباشرة لا عبر النقل (لـ S-022b أو S-027). و`flush` في `exclude_re` يطابق أي `<impl Write for Stream>` في المستودع، اليوم لا يوجد غير `unix.rs` و`windows.rs` المستثنى.
  - لا سطر CHANGELOG جديد من المراجع: السطر الواحد للقصة كلها أضافته S-022a، وS-022b تعدّله.
- 2026-10-22، المختبِر (S-022b): كُتبت اختبارات AC8 إلى AC14 في ملفات جديدة حتى لا يكسر استيراد غير محلول اختبارات S-022a الأربعة عشر في `transport.rs`. العقد الذي ثبّته المختبِر في رأس `crates/core/tests/transport_plan.rs` و`deadline.rs` (لا يُعدَّل)، وكله `pub` في `sbarhive_core::transport` على كل المنصات:
  - `dacl_sddl(&str) -> String`.
  - `enum BindOutcome { Listen, AlreadyRunning, Failed(io::Error) }` و`bind_result(Option<u32>) -> BindOutcome`: `None` استماع، و`Some(5)` `AlreadyRunning`، وغيره `Failed` برمزه في `raw_os_error()`.
  - `enum ConnectStep { Connected, Wait(Duration), Fail(io::Error) }` و`connect_plan(open_error: Option<u32>, elapsed: Duration) -> ConnectStep`، خطوة واحدة من حلقة الاتصال والزمن مُدخَل لا نوم حقيقي: `None` اتصال، و`Some(2)` `NotFound` في أي زمن دون انتظار، و`Some(231)` قبل 2000 ms `Wait(2000 ms - elapsed)` وهي مهلة `WaitNamedPipeW`، وعند 2000 ms أو بعدها `TimedOut` (الحد: 2000 نفسها مهلة منتهية)، وأي رمز آخر `Fail` برمزه.
  - `verify_owner<S>(server_sid: &str, client_sid: &str, stream: S) -> io::Result<S>`: يعيد المجرى إن تطابق SID، وإلا `PermissionDenied` ويُسقط المجرى دون كتابة. هذا شكل «نتيجة التحقق من المالك» في AC11، وعدّاد الكتابة 0 وعدّاد الإسقاط 1 في الاختبار.
  - `open_mode(bool) -> u32` و`pipe_mode() -> u32`.
  - `pub mod deadline` و`read_with_deadline<T, R, C>(timeout: Duration, read: R, cancel: C) -> io::Result<T>` حيث `R: FnOnce() -> io::Result<T>` على الخيط المستدعي و`C: FnOnce() + Send + 'static` يملكه المراقب. إن انقضت المهلة يُستدعى `cancel` مرة واحدة والنتيجة `TimedOut` أياً كان ما أعادته القراءة الملغاة؛ وإن عادت القراءة أولاً فنتيجتها كما هي ولا يُستدعى `cancel` ولو بعد حين؛ وعند العودة لا مراقب حي (عدّاد الخيوط: `Arc` داخل `cancel` يعود `strong_count` إلى 1).
  - الملفات والاختبارات:
    - `crates/core/tests/transport_plan.rs`: `ac8_dacl_sddl_is_the_exact_literal`، `ac8_dacl_sddl_grants_no_one_else`، `ac8_dacl_sddl_embeds_the_given_sid` (أطول SID، حتى لا تنجو قيمة ثابتة من `M`)، `ac9_bind_result_none_listens`، `ac9_bind_result_access_denied_is_already_running`، `ac9_bind_result_other_codes_fail_with_the_same_code` (2 و231 و1 و87 و`0xFFFFFFF0`)، `ac11_connect_plan_no_error_is_connected`، `ac11_connect_plan_file_not_found_is_not_found_at_once`، `ac11_connect_plan_pipe_busy_waits_for_the_rest_of_2000_ms` (0 و1 و750 و1999 ms)، `ac11_connect_plan_pipe_busy_times_out_at_2000_ms` (2000 و2001 و10000)، `ac11_connect_plan_other_codes_fail_with_the_same_code`، `ac11_another_users_server_is_permission_denied_and_gets_no_byte`، `ac11_own_server_hands_the_stream_back_for_writing` (يقتل طفرة «ارفض دائماً»)، `ac12_open_mode_first_instance_is_duplex_and_first_pipe_instance`، `ac12_open_mode_later_instances_are_duplex_only`، `ac12_pipe_mode_is_byte_wait_and_rejects_remote_clients`.
    - `crates/core/tests/deadline.rs`: `ac10_blocked_read_times_out_between_180_and_1000_ms_with_one_cancel` (القراءة الوهمية تحجب بـ `recv_timeout` 3 s حتى لا تعلّق طفرة لا تلغي `M`، وتعيد بعد الإلغاء خطأً ليس `TimedOut` فيثبت الاختبار التحويل)، `ac10_read_done_within_50_ms_returns_its_data_and_never_cancels` (تعود قبل 150 ms ولا يُستدعى `cancel` حتى 400 ms بعد العودة)، `ac10_a_read_error_before_the_deadline_is_returned_as_is`.
    - `crates/core/tests/transport_seed.rs` (عدّله المختبِر): حُذف فحص `ErrorKind::Unsupported` لأن S-022b تستبدل البذرة بالأنبوب الحقيقي ولا يحق للمنفِّذ تعديل الاختبار، وبقي فحص `exclude_globs` ومنع `todo!` باسم `ac7_windows_transport_is_excluded_from_mutants_and_has_no_todo`. وأُضيفت فحوص نص `windows.rs` التي تطلبها AC8 وAC10 وAC12: `ac8_windows_builds_the_descriptor_from_dacl_sddl`، `ac10_windows_cancels_the_read_with_cancel_io_ex`، `ac9_ac11_ac12_windows_uses_the_pure_decisions` (يستدعي `open_mode(` و`pipe_mode(` و`bind_result(` و`connect_plan(` و`verify_owner(` و`WaitNamedPipeW`)، `ac12_windows_asserts_every_flag_against_windows_sys_at_compile_time` (`const _: () = assert!` وأسماء الثوابت التسعة)، `ac12_windows_checks_the_owner_and_names_the_endpoint_from_the_token`، و`ac10_deadline_rs_is_mutated_on_linux` (الملف موجود وليس في `exclude_globs`).
    - `crates/core/tests/transport_deps.rs`: `ac13_cargo_lock_keeps_671_packages`، `ac13_core_depends_on_windows_sys_0_61_2_in_the_lock` (مدخل `sbarhive-core` في `Cargo.lock` يسرد `"windows-sys 0.61.2"`)، `ac13_core_declares_windows_sys_for_windows_only` (في `[target.'cfg(windows)'.dependencies]` من `crates/core/Cargo.toml` بإصدار 0.61، ولا في غيره)، `ac14_engines_md_records_windows_sys_and_its_license` (الاسم والإصدار و`MIT OR Apache-2.0` في حدود ثمانية أسطر).
  - ينجح قبل التنفيذ عن قصد: `ac13_cargo_lock_keeps_671_packages` (حارس تراجع، `windows-sys 0.61.2` موجود في القفل أصلاً)، و`ac7_windows_transport_is_excluded_from_mutants_and_has_no_todo` (حارس من S-022a).
  - للمراجع لا للاختبار: `X` و`cargo tree` على لينكس، و`W` بـ `-p sbarhive-engines`، ونص `SECURITY.md` بالعربية والإنكليزية و`docs/architecture.md`، وأن كل `CreateNamedPipeW` يأخذ الواصف (فحص النص يثبت الوجود لا كل استدعاء)، وتعديل سطر CHANGELOG نفسه.
  - تحقق من منطق الاختبارات: نسخة مؤقتة خارج المستودع (crate في مجلد الجلسة المؤقت، حُذفت ولا تُدمج) بتطبيق مختصر للعقد نجحت فيها اختبارات `transport_plan.rs` الستة عشر و`deadline.rs` الثلاثة، و`cargo clippy --tests -D warnings` نظيف عليها وعلى `transport_seed` و`transport_deps` هنا. `cargo fmt --check` نظيف، واختبارات S-022a في `transport.rs` باقية 14 ناجحة.
  - غموض: (1) AC11 يسمي `connect_plan(results)` بالجمع؛ ثُبّت خطوة واحدة بالزمن مُدخلاً حتى لا ينام الاختبار 2 s تحت `M`، والحلقة نفسها في `windows.rs`. (2) حالة «انتظر ثم أعد» حين يعيد `WaitNamedPipeW` نجاحاً مبكراً ثم يبقى الأنبوب مشغولاً: الزمن المنقضي يُقاس بالساعة في الحلقة، فلم يُختبر خارج الدالة. (3) رموز غير 2 و231 في الاتصال لم يحددها AC11؛ ثبّتها المختبِر كما AC9: الرمز نفسه في `raw_os_error()`.
  - مخرجات الفشل (`cargo test -j 2 -p sbarhive-core` داخل النطاق):
```
$ cargo test -j 2 -p sbarhive-core --test transport_plan
error[E0432]: unresolved imports `sbarhive_core::transport::BindOutcome`, `sbarhive_core::transport::ConnectStep`, `sbarhive_core::transport::bind_result`, `sbarhive_core::transport::connect_plan`, `sbarhive_core::transport::dacl_sddl`, `sbarhive_core::transport::open_mode`, `sbarhive_core::transport::pipe_mode`, `sbarhive_core::transport::verify_owner`
error: could not compile `sbarhive-core` (test "transport_plan") due to 1 previous error

$ cargo test -j 2 -p sbarhive-core --test deadline
error[E0432]: unresolved import `sbarhive_core::transport::deadline`
  --> crates/core/tests/deadline.rs:18:31
   |
18 | use sbarhive_core::transport::deadline::read_with_deadline;
   |                               ^^^^^^^^ could not find `deadline` in `transport`
error: could not compile `sbarhive-core` (test "deadline") due to 1 previous error

$ cargo test -j 2 -p sbarhive-core --test transport_seed
test ac7_windows_transport_is_excluded_from_mutants_and_has_no_todo ... ok
---- ac10_deadline_rs_is_mutated_on_linux ----
.../crates/core/src/transport/deadline.rs must exist
---- ac10_windows_cancels_the_read_with_cancel_io_ex ----
windows.rs lacks ["read_with_deadline(", "CancelIoEx"]: the read timeout must cancel the pipe read (AC10)
---- ac12_windows_asserts_every_flag_against_windows_sys_at_compile_time ----
compile time asserts are required (AC12)
---- ac12_windows_checks_the_owner_and_names_the_endpoint_from_the_token ----
windows.rs lacks ["GetNamedPipeServerProcessId", "ProcessIdToSessionId", "GetTokenInformation", "TokenUser", "pipe_name("]: owner check and endpoint come from the process token (AC12)
---- ac8_windows_builds_the_descriptor_from_dacl_sddl ----
windows.rs lacks ["dacl_sddl(", "ConvertStringSecurityDescriptorToSecurityDescriptorW", "CreateNamedPipeW"]: the pipe must be created with the DACL from dacl_sddl (AC8)
---- ac9_ac11_ac12_windows_uses_the_pure_decisions ----
windows.rs lacks ["open_mode(", "pipe_mode(", "bind_result(", "connect_plan(", "verify_owner(", "WaitNamedPipeW"]: windows.rs must call the tested pure functions (AC9, AC11, AC12)
test result: FAILED. 1 passed; 6 failed

$ cargo test -j 2 -p sbarhive-core --test transport_deps
test ac13_cargo_lock_keeps_671_packages ... ok
---- ac13_core_declares_windows_sys_for_windows_only ----
crates/core/Cargo.toml needs [target.'cfg(windows)'.dependencies]
---- ac14_engines_md_records_windows_sys_and_its_license ----
docs/engines.md must record windows-sys 0.61.2 (MIT OR Apache-2.0)
---- ac13_core_depends_on_windows_sys_0_61_2_in_the_lock ----
sbarhive-core must use windows-sys 0.61.2:
name = "sbarhive-core" ... dependencies = [ "serde", "serde_json", "tempfile", "thiserror 2.0.21", "toml 0.8.23" ]
test result: FAILED. 1 passed; 3 failed
```
- 2026-10-22، المنفِّذ (S-022b): لم يُعدَّل أي اختبار (`git diff 80345be -- crates/*/tests` فارغ). قبل المدير اختيارات المختبِر الستة كما هي.
  - `transport.rs`: `dacl_sddl` و`BindOutcome`/`bind_result` و`ConnectStep`/`connect_plan` و`verify_owner` و`open_mode`/`pipe_mode`، والقيم ثوابت حرفية خاصة (`OPEN_MODE_FIRST` و`OPEN_MODE_LATER` و`PIPE_MODE` ورموز الأخطاء الثلاثة) حتى لا تنتج `|` طفرات مكافئة؛ يتحقق `windows.rs` منها بـ `const _: () = assert!` مقابل `windows_sys`. الرمز 2 لا ذراع له في `connect_plan`: يمر رمزاً خاماً، وstd يقرؤه `NotFound` على ويندوز (وعلى لينكس لأن 2 هو `ENOENT`)، فلا ذراع تنجو طفرة حذفها؛ وثابته `ERROR_FILE_NOT_FOUND` عليه `cfg_attr(not(windows), allow(dead_code))` ويُستعمل في الفحص وقت الترجمة.
  - `transport/deadline.rs`: خيط مراقب بـ `recv_timeout` وقفل واحد يحسم من سبق (الإلغاء أو القراءة)، ثم `join` قبل العودة، فلا يُستدعى `cancel` بعد عودة القراءة ولا يبقى مراقب حي.
  - `transport/windows.rs`: موضع واحد لـ `CreateNamedPipeW` (`create_instance`) يأخذ الواصف المبني من `dacl_sddl` بـ `ConvertStringSecurityDescriptorToSecurityDescriptorW`، فكل نسخة تأخذه؛ والواصف يعيش مع `Listener` ويُحرَّر بـ `LocalFree`. `accept` ينشئ النسخة التالية قبل تسليم الاتصال، فيجد العميل الأنبوب مشغولاً لا مفقوداً، و`ERROR_PIPE_CONNECTED` نجاح. العميل يفتح بـ `OpenOptions` مع `security_qos_flags(SECURITY_IDENTIFICATION)` ويدور على `connect_plan` و`WaitNamedPipeW` (المهلة 1 ms على الأقل، لأن 0 تعني المهلة الافتراضية)، ثم `GetNamedPipeServerProcessId` و`OpenProcess` و`GetTokenInformation(TokenUser)` و`verify_owner`؛ أي فشل في معرفة المالك يعطي SID فارغاً فيُرفض (يفشل مغلقاً). المهلة مشتركة بين `try_clone` كما على Unix، والقراءة ذات المهلة `read_with_deadline` مع `CancelIoEx` على مقبض القراءة نفسه. لا `DisconnectNamedPipe` عند الإغلاق حتى لا يضيع رد لم يُقرأ. `endpoint()` من `ProcessIdToSessionId` وSID الرمز، و`server_pid()` يعيد pid الخادم على مجرى العميل.
  - `crates/core/Cargo.toml`: `windows-sys = { version = "0.61", features = [...] }` تحت `[target.'cfg(windows)'.dependencies]` وحده. `Cargo.lock`: سطر واحد في مدخل `sbarhive-core` (`"windows-sys 0.61.2"`)، و671 `[[package]]`.
  - `C`: `cargo fmt --check` نظيف، و`cargo clippy -j 2 --workspace --all-targets -- -D warnings` بلا تحذير.
  - `W`: `cargo clippy -j 2 --target x86_64-pc-windows-gnu -p sbarhive-core -p sbarhive-engines --lib -- -D warnings` بلا تحذير، و`cargo check` بالأعلام نفسها بلا تحذير.
  - `X`: يسرد serde وserde_json وthiserror وtoml و`windows-sys v0.61.2`؛ و`cargo tree -p sbarhive-core -e normal --depth 1` على لينكس لا يسرده.
  - `cargo test -j 2 --all`: 610 ناجحاً، 0 فاشلاً، 1 متجاهل (581 قبلها، و29 جديدة: 16 في `transport_plan.rs`، و3 في `deadline.rs`، و7 في `transport_seed.rs` بعد أن كانت 1، و4 في `transport_deps.rs`).
  - `M`: `cargo mutants --jobs 1 --in-diff` على `git diff 80345be -- crates` بـ `CARGO_BUILD_JOBS=2`: 21 طفرة، 15 مكتشفة، 6 غير قابلة للبناء، 0 مهلة، **0 ناجية**، رمز الخروج 0. لا إضافة إلى `exclude_re`؛ عُدّل تعليق `exclude_globs` لـ `windows.rs` وحده.
  - `scripts/design-lint.sh`: clean. لا شرطة طويلة في الفرق. `git diff --stat main...HEAD -- crates/ui` فارغ.
  - الوثائق: `SECURITY.md` (فقرة ويندوز بالإنكليزية وفقرة عربية)، و`docs/architecture.md` (النسخة الواحدة على المنصتين)، و`docs/engines.md` (سطر windows-sys 0.61.2، MIT OR Apache-2.0 من `Cargo.toml` الحزمة نفسها). سطر CHANGELOG نفسه عُدّل ولم يُضف غيره.
  - لم يُتحقق حياً ولا يمكن على لينكس: أن `CancelIoEx` من خيط آخر يلغي `ReadFile` متزامناً على المقبض (اعتماد على ADR 0005)، وكل بنود S-034.
  - لـ S-027: `crates/daemon/src/main.rs` ما زال يحذف `ipc::socket_path()` مباشرة عند الخروج؛ تُرك لأن معايير S-022b لا تطلبه والخدمة لا تُبنى لويندوز قبل S-027.
