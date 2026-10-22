# S-022 نقل IPC خلف واجهة واحدة، ثم named pipe على ويندوز
الحالة: معتمدة للتنفيذ على فرع windows/0.6.0 بطلب محمد (2026-10-22)؛ ADR 0005 ما زال مقترحاً، ولا دمج في main؛ S-022a: الاختبارات مكتوبة وتفشل، بانتظار المنفِّذ

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
