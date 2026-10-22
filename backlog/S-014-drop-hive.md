# S-014 إفلات ملف مسحوب على خلية بعد اختصار أثناء السحب
الحالة: محجوبة، تنتظر قرار المدير في تعارض `tests/drop/docs.sh` مع البند 2 (S-014d: بعد REWORK 1 من 2، لم تُرسل للمراجعة)

يعتمد على ADR 0003 (مقبول في 2026-10-20). النطاق: المرحلتان الأولى والثانية من «القرار المقترح» فيه، للإصدار 0.3.0.

## القصة
بصفتي مستخدماً على GNOME أو KDE بجلسة Wayland، أريد أن أبدأ سحب ملف في مدير الملفات، ثم أضغط اختصاراً أختاره وأنا ما زلت أسحب، فتظهر الخلية، وأفلت الملف على خلية فيُنفَّذ إجراؤها، حتى أحوّل ملفاً دون قائمة النقر اليمين.

## التقسيم (كل قصة فرعية يوم عمل أو أقل، تُنفَّذ بالترتيب)
| القصة | المحتوى | المعايير |
| --- | --- | --- |
| S-014a | الأجزاء الصافية: تحليل uri-list، اختيار الخلية من نقطة الإفلات، آلة حالات الوضع؛ ورسالة IPC و`sbarhive drop` | AC1 إلى AC4 |
| S-014b | مستقبِل الإفلات في `crates/ui`: اتصال ثانٍ على عرض winit، ربط بسطح كل ظهور، نسخ فقط، تشغيل الإجراء | AC5، AC6، AC7 |
| S-014c | وضع «انتظار الإفلات»: الخلايا عند `enter`، الإبراز، الإغلاق، النصوص | AC8، AC9، AC10 |
| S-014d | اختبار مخفي شامل في `tests/drop/` لـ GNOME وKWin، وتوثيق الربط اليدوي ومنه سطر Hyprland | AC11، AC12 |
| S-014e | تسجيل الاختصار عبر بوابة GlobalShortcuts بـ ashpd، وتسجيل الرخص | AC13، AC14 |
| S-014f | رمز التفعيل من البوابة في GNOME، وتغيير الاختصار من الإعدادات | AC15، AC16 |

## معايير القبول
- AC1: تحليل `text/uri-list` في الإفلات يطابق `crates/cli/src/clipboard.rs`: يقبل CRLF وLF، يتجاهل أسطر `#` والفارغة وكل ما ليس `file:`، يفك الترميز المئوي، ويعيد كل الملفات بترتيبها.
- AC2: لكل خلية في التخطيط بالإنكليزية وبالعربية المعكوسة، يعيد اختيار الخلية من نقطة الإفلات الفهرسَ نفسه الذي يعيده اختيار النقر (`crates/ui/src/geometry.rs`)، عند مركزها وعند 6 نقاط داخل حدودها بمسافة 1 px. الزوايا الشفافة والخلية المركزية لا تعيد إجراءً.
- AC3: آلة حالات الوضع (مخفية، تنتظر، فوق خلية، تنفّذ) تغلق الخلية عند `leave` بعد `enter`، وعند إلغاء السحب، وعند Esc، وبعد 10 ثوانٍ من الظهور دون `enter`. فقدان التركيز والنقر خارجها لا يغلقانها في حالتي الانتظار، ويبقى فقدان التركيز يغلقها في الوضع العادي.
- AC4: `sbarhive drop [--token T]` يرسل `Request::Drop { token }`، ويشغّل الخدمة إن لم تعمل خلال ثانيتين كما `open`، ويأخذ الرمز من `--token` ثم `XDG_ACTIVATION_TOKEN` ثم `DESKTOP_STARTUP_ID`.
- AC5: في كل ظهور بوضع الإفلات يربط المستقبِل `wl_data_device` بسطح الخلية الجديد: الإفلات ينجح في الظهور الأول والثالث على التوالي، ويظهر ذلك في `WAYLAND_DEBUG=1` للخدمة.
- AC6: المستقبِل يقبل `text/uri-list` فوق الخلايا فقط بـ `set_actions(Copy, Copy)`، حتى لو ضُغط Shift عند الإفلات. المصدر يبلغ `deleted=False`، وsha256 الأصل لم يتغير.
- AC7: الإفلات على خلية «WEBP» لملف PNG يكتب `<الاسم>.webp` بجانب الأصل خلال 5 ثوانٍ، والإفلات على زاوية شفافة أو على المركز لا يكتب أي ملف ويغلق الخلية.
- AC8: الخلايا تُبنى من الملفات المقروءة عند `enter` قبل الإفلات: معرّفات خلايا ملف PNG ثم ملف PDF تساوي ناتج `sbarhive actions` لكلٍّ منهما.
- AC9: الخلية تحت المؤشر مبرزة أثناء السحب، بألوان من `design/tokens.json` فقط، بلقطة يعتمدها محمد، ودون حركة إن طُلب تقليل الحركة.
- AC10: نص المركز في وضع الانتظار بالعربية والإنكليزية، وله تسمية وصفية.
- AC11: في GNOME Shell 50 (smart) وKWin 6.7 (Low)، مخفيين: سحب ملف من نافذة GTK 4، ثم الاختصار أثناء السحب يشغّل `sbarhive drop`، ثم الإفلات على خلية، ينتج الملف في 3 من 3 تشغيلات لكل مؤلِّف. Esc أثناء السحب يلغيه، والخلية تختفي خلال 500 ms، ولا ملف ناتج.
- AC12: من الاختصار إلى أول إطار مرسوم أقل من 150 ms بخدمة جاهزة (وسيط 20 ظهوراً). `docs/` يوثّق ربط `sbarhive drop` في إعدادات GNOME وKDE وسطر `bind` في Hyprland.
- AC13: الخدمة تسجّل عبر البوابة الاختصار `drop` بمعرّف التطبيق `io.github.m7mmad_omar.SbarHive` وربط مفضّل `<Super>z`، وضغطه في GNOME 50 وKDE 6 المخفيين يُظهر الخلية. إن عرض المنفذ نافذة موافقة تقبلها السكربتات بإدخال محقون.
- AC14: ashpd (MIT) وكل crate جديد في `crates/ui` (wayland-client، wayland-backend، raw-window-handle وغيرها) مسجّلة في `docs/engines.md` بالإصدار والرخصة، ولا تُضاف نسخة ثانية من zbus إلى `Cargo.lock`.
- AC15: في GNOME بسياسة strict، الإفلات عبر اختصار البوابة (مع `activation_token`) ينجح، وعبر `sbarhive drop` بلا رمز لا يصل الإفلات. هذا يثبت أن النجاح سببه الرمز.
- AC16: بعد تغيير الاختصار (KDE عبر KGlobalAccel على D-Bus، وGNOME عبر مخزن منفذها) يُظهر المفتاح الجديد الخلية ولا يُظهرها القديم.

## حالات الاختبار
| معيار | الاختبار | النوع |
| --- | --- | --- |
| AC1 | وحدة: CRLF، تعليق، `%20` و`%D8%B5`، `https:` متجاهَل، ملفان | وحدة |
| AC2 | وحدة على كل الخلايا في اللغتين، والزوايا والمركز تعيد `None` | وحدة |
| AC3 | وحدة: كل انتقال، والمهلة بساعة محقونة عند 9.9 و10.1 ثانية، وفقدان التركيز في الوضعين | وحدة |
| AC4 | `crates/cli/tests/cli.rs`: خدمة وهمية تستقبل `{"cmd":"drop","token":...}` من كل مصدر رمز؛ e2e «drop starts the daemon» | وحدة، تكامل |
| AC5 | `tests/drop/kwin.sh` و`gnome.sh`: 3 ظهورات، عدّ `wl_data_device_manager.get_data_device` على كل سطح جديد | تكامل |
| AC6 | السكربتان مع Shift عند الإفلات: `deleted=False` من `source.py` ومقارنة sha256 | تكامل |
| AC7 | السكربتان: وجود الناتج ونوعه بـ `file --mime-type`، وإفلات على زاوية ثم عدّ الملفات | تكامل |
| AC8 | السكربتان: سطر سجل الخدمة بمعرّفات الخلايا عند `enter` يقارَن بـ `sbarhive actions` | تكامل |
| AC9 | لقطة من المؤلِّف المخفي، و`scripts/design-lint.sh` | لقطة |
| AC10 | وحدة على `crates/daemon/src/text.rs` للغتين | وحدة |
| AC11 | `tests/drop/gnome.sh smart` و`tests/drop/kwin.sh level=1` ثلاث مرات، وحالة Esc | تكامل |
| AC12 | `crates/ui/tests/measure.sh` بوضع الإفلات؛ وجود الأقسام الثلاثة في التوثيق | أداء |
| AC13 | `tests/drop/*.sh portal` مع xdg-desktop-portal ومنفذه على الناقل الخاص و`XDG_CURRENT_DESKTOP` للمؤلِّف | تكامل |
| AC14 | `cargo tree -d -i zbus` بنسخة واحدة؛ مراجعة أسطر `docs/engines.md` | تكامل |
| AC15 | `tests/drop/gnome.sh strict portal` و`strict cli` | تكامل |
| AC16 | `tests/drop/*.sh rebind` | تكامل |

## خارج النطاق
- X11 (ترتيب «الخلية أولاً ثم السحب» مع `QueryPointer` و`AlwaysOnTop`): قصة مستقلة S-015، لأنها مسار مختلف لم يُشغَّل ولا يختبرها هذا الإطار.
- تشغيل Hyprland فعلياً (يُوثَّق سطر `bind` فقط)، وقيد أول `wl_data_device` فيه.
- KWin بمستوى Extreme: منفذ KDE لا يرسل رمزاً، فيبقى حداً موثقاً.
- Dolphin وNautilus الحقيقيان مصدراً للسحب، واختصارات GNOME المخصصة (gsd-media-keys)، ونظرة الأنشطة في GNOME.
- حذف المستقبِل الخاص بعد ترقية Slint إلى winit 0.31.

## سجل المراجعة
- 2026-10-20، المخطِّط: الجهد في ADR 0003 نحو 6 إلى 7 أيام، فقُسّمت القصة إلى ست قصص فرعية في ملف واحد بطلب المدير. الربط المفضّل `<Super><Control>F11` هو المتحقَّق منه في التجربة، ويحتاج موافقة محمد. ينتظر الموافقة.
- 2026-10-20، محمد (عبر المدير): القصة معتمدة. الاختصار الافتراضي `Super+Z`: من يسار لوحة المفاتيح ومفتاحان فقط. مفتاح تعديل وحده (Super أو Alt أو Shift) لا يصلح: Shift وCtrl وAlt أثناء السحب تغيّر نوع الإفلات في مدير الملفات (نقل، نسخ، ربط)، وSuper وحده يفتح نظرة الأنشطة في GNOME وقائمة التطبيقات في KDE فيقطع السحب، وبوابة الاختصارات لا تقبل مفتاح تعديل وحده. Super+Z غير مستعمل في GNOME وKDE افتراضياً ولا في إعداد Hyprland عند محمد. يغيّره المستخدم من إعدادات النظام.
- 2026-10-20، المختبِر (S-014a فقط، AC1 إلى AC4): كُتبت الاختبارات من المعايير وفشلت كلها قبل التنفيذ.
  - الملفات: `crates/core/tests/uri_list.rs` (AC1)، `crates/core/tests/ipc_drop.rs` (AC4، شكل الرسالة)، `crates/ui/src/drop_tests.rs` مع سطر `#[cfg(test)] mod drop_tests;` في `crates/ui/src/lib.rs` (AC2 وAC3)، وخمسة اختبارات `ac4_*` في `crates/cli/tests/cli.rs` بخدمة وهمية تسجّل JSON الخام، وفحص «drop starts the daemon» في `tests/e2e/run.sh`.
  - العقد الذي تثبّته الاختبارات (لا تُعدَّل):
    - `sbarhive_core::uri_list::parse(text: &str) -> Vec<PathBuf>`
    - `sbarhive_core::ipc::Request::Drop { token: Option<String> }` بالشكل `{"cmd":"drop","token":...}`، دون `files`، و`{"cmd":"drop"}` وحده يعني `token: None`.
    - `crate::dnd::pick(plan: &Plan, x: f64, y: f64) -> Option<usize>` في `crates/ui`.
    - `crate::dnd::State { Hidden, Normal, Waiting, Over(usize), Running(usize) }` بـ `Clone, Copy, Debug, PartialEq, Eq`.
    - `crate::dnd::Event { Show { drop: bool }, Enter, Motion(Option<usize>), Leave, Drop, Cancel, Escape, FocusLost, ClickOutside, Tick }`.
    - `crate::dnd::Machine`: `new()` (Hidden)، `state()`، `handle(&mut self, event: Event, now: Instant) -> State`، والساعة محقونة عبر `now`.
    - `sbarhive drop [--token T]`: الرمز من `--token` ثم `XDG_ACTIVATION_TOKEN` ثم `DESKTOP_STARTUP_ID`، والقيمة الفارغة تُتخطّى كما في `open`.
  - ملاحظة: سطر `mod drop_tests` يجعل كل اختبارات وحدة `sbarhive-ui` (ومنها `ui_tests` و`layout`) لا تُبنى حتى تُضاف الوحدة `dnd`. هذا متوقَّع.
  - مخرجات الفشل:
    ```
    $ cargo test -p sbarhive-core --test uri_list --test ipc_drop
    error[E0432]: unresolved import `sbarhive_core::uri_list`
     --> crates/core/tests/uri_list.rs:5:20
    error[E0599]: no variant named `Drop` found for enum `sbarhive_core::ipc::Request`
     --> crates/core/tests/ipc_drop.rs:7:22
    error: could not compile `sbarhive-core` (test "uri_list") due to 1 previous error
    error: could not compile `sbarhive-core` (test "ipc_drop") due to 3 previous errors

    $ cargo test -p sbarhive-ui --lib
    error[E0432]: unresolved import `crate::dnd`
      --> crates/ui/src/drop_tests.rs:16:12
    error: could not compile `sbarhive-ui` (lib test) due to 1 previous error

    $ cargo test -p sbarhive-cli --test cli
    thread 'ac4_drop_token_flag_wins_over_both_environment_variables' panicked at crates/cli/tests/cli.rs:425:5:
    drop failed: error: unrecognized subcommand 'drop'
    failures:
        ac4_drop_falls_back_to_desktop_startup_id
        ac4_drop_skips_an_empty_environment_token_like_open
        ac4_drop_takes_xdg_activation_token_before_desktop_startup_id
        ac4_drop_token_flag_wins_over_both_environment_variables
        ac4_drop_without_any_token_sends_none
    test result: FAILED. 14 passed; 5 failed

    $ tests/e2e/run.sh   (KWin مخفي خاص)
    FAIL drop starts the daemon
    ```
  - في التشغيل نفسه فشل أيضاً `idle memory after use within 40 MB`، وهو فحص قائم لا يخص هذه القصة، وبُني الإصدار مع تعديلات غير مُودَعة لعمل آخر في الشجرة (أمر `settings`).
- 2026-10-20، المدير: المنفِّذ أوقِف قبل كتابة مدخله بطلب محمد (اجتماع، بلا حمل على الجهاز)، بعد أن ثبّت S-014a في `8175440`. تحقق المدير: fmt وclippy نظيفان، و`cargo test --workspace` 353 ناجحاً بلا فشل. لم يُشغَّل بعد: اختبار e2e «drop starts the daemon» في KWin المخفي، و`cargo mutants` على الملفات المتغيرة. يُشغَّلان قبل مراجعة S-014a.
- 2026-10-20، المختبِر (S-014b فقط، AC5 إلى AC7): كُتبت الاختبارات من المعايير وجدول الحالات، ولم تُشغَّل بطلب محمد (اجتماع، بلا حمل على الجهاز، ولا مؤلِّف مخفي).
  - لا اختبارات Rust في S-014b: صفوف AC5 وAC6 وAC7 في الجدول كلها تكامل بالسكربتين، ولم يُخترع عقد وحدة لم يكتبه المخطِّط. لذلك لا مخرجات فشل Rust لهذه القصة.
  - الملفات: `tests/drop/kwin.sh` و`tests/drop/gnome.sh` و`tests/drop/lib.sh` (المنطق المشترك والفحوص)، و`tests/drop/source.py` (مصدر سحب GTK 4 يعرض النسخ والنقل، يقرأ الملف من ملف مسار عند كل سحب، ويحذف الأصل إن أُبلغ `deleted=True` كما يفعل مدير الملفات)، و`tests/drop/input_gnome.py` (مع `keydown` و`keyup` لإمساك Shift)، و`tests/drop/kwin-input/` (أداة `org_kde_kwin_fake_input` منسوخة من التجربة، خارج الـ workspace فلا تدخل `Cargo.lock`)، و`tests/drop/README.md`، وسطران في `.gitignore`.
  - ما يفعله كل سكربت: الخدمة بـ `WAYLAND_DEBUG=1` و`SBARHIVE_LANG=en` دون ظهور تسخين، ثم أربعة ظهورات بـ `sbarhive drop` أثناء السحب لنسخ من `tests/fixtures/images/alpha.png`: (1) على خلية WEBP، (2) على زاوية شفافة، (3) على WEBP مع Shift، (4) على المركز. موقع خلية WEBP من قاعدة التخطيط المشحونة: أول خلية في الحلقة الأولى أعلى يمين المركز، ونصف القطر والفجوة من `design/tokens.json`.
  - الفحوص: AC5 لكل ظهور `get_data_device` بعد `get_xdg_surface` السطح الجديد، والإفلات ينجح في الظهورين 1 و3. AC6 `deleted=False` وsha256 الأصل ثابت مع Shift، و`set_actions(1, 1)` مرسل ولا يعرض النقل أبداً، و`accept` بـ `text/uri-list` وحده، ورفض الإفلات خارج الخلايا (`drag-cancel` عند المصدر)، و`enter` على جهاز الإفلات في كل ظهور. AC7 `<الاسم>.webp` خلال 5 ثوانٍ و`file --mime-type` يعطي `image/webp`، والزاوية والمركز لا يكتبان ملفاً ويغلقان الخلية.
  - تحقق ثابت فقط: `bash -n` و`shellcheck -x` نظيفان، و`py_compile` للملفين، و`cargo check` لأداة `kwin-input` (`nice -n 19`، `-j 2`)، وتجربة `protocol_checks` على أثر بروتوكول مصطنع (ينجح بأثر صحيح، ويفشل بلا جهاز أو مع `set_actions(3, 2)`).
  - الفشل المتوقَّع قبل التنفيذ (تنبؤ، لم يُشغَّل): `crates/ui/Cargo.toml` بلا wayland-client ولا wayland-backend، وwinit 0.30 لا يربط `wl_data_device` (ADR 0003، النتيجة 1)، فلا `get_data_device` في الأثر ولا `set_actions` ولا `accept`، فيفشل AC5 في كل ظهور، ويفشل AC6 (`set_actions(Copy, Copy) sent` و`accepts "text/uri-list"`)، ولا يُكتب `.webp` فيفشل AC7 في الظهورين 1 و3. وكل ظهور يفشل قبل التنفيذ، ومنها الزاوية والمركز: لا `enter` على جهاز الإفلات ولا جهاز أصلاً. لذلك يمر المؤشر في الظهورين 2 و4 فوق خلية WEBP قبل الهدف، فيكون الرفض انتقالاً حقيقياً من قبول إلى رفض، ويُفحص إغلاق الخلية خلال ثانيتين بعد الإفلات مباشرة، فلا تحسبه مهلة العشر ثوانٍ (AC3).
  - لم يُشغَّل بعد: `tests/drop/kwin.sh` و`tests/drop/gnome.sh`. يُشغَّلان (مع بناء `tests/drop/kwin-input`) لإثبات الفشل قبل أن يبدأ المنفِّذ، ثم بعده. AC11 وAC13 (الاختصار، Esc، ثلاث تشغيلات، البوابة) تضيفها S-014d وS-014e إلى السكربتين نفسيهما.
  - تنبيه للمنفِّذ: AC7 يتطلب أن تكون خلية WEBP موجودة تحت المؤشر قبل الإفلات، أي أن تُعرف الملفات قبل `drop`، وهذا يتقاطع مع AC8 (S-014c).
- 2026-10-20، المدير (قرار): قراءة الملفات المسحوبة عند دخول السحب إلى الخلية تُنقل إلى S-014b، لأن AC7 لا يتحقق دونها (الخلية تحتاج الإجراءات قبل الإفلات). يبقى AC8 في S-014c للتحقق من سلوك الدخول مع الإبراز.
- 2026-10-20، المنفِّذ (S-014b، AC5 إلى AC7): المستقبِل مكتوب ومربوط بمسار الخدمة القائم، والحالة: جاهزة للمراجعة (رأس الملف يبقى «معتمدة» لأنه يغطي القصص الست).
  - `crates/ui/src/receiver.rs` (لينكس فقط): اتصال ثانٍ من `Backend::from_foreign_display` على عرض winit، بطابور وخيط خاصين (`sbarhive-drop`)، يبدأ عند أول ظهور بوضع الإفلات. بعد كل ظهور بوضع الإفلات يحرّر الجهاز القديم (`release`) ويطلب `get_data_device` جديداً، ويقرأ مؤشر السطح عند أول `RedrawRequested` للسطح الجديد، أي بعد `get_xdg_surface` له. يقبل `text/uri-list` فوق الخلايا فقط بـ `dnd::pick`، ويرسل `set_actions(Copy, Copy)` عند `enter` لعروض الإصدار 3. يقرأ الملفات عند `enter` (أنبوب وpoll بمهلة ثانية) ويحلّلها بـ `uri_list::parse`. لا يرسل `finish` إلا بعد قبول نوع واختيار المؤلِّف إجراءً، وإلا يهدم العرض دونه. عروض الحافظة (`selection`) تُهدم ولا تُقرأ.
  - `crates/ui/src/lib.rs`: `Hive::present_drop` و`Hive::on_drop_files`، وكل حدث يمر بـ `dnd::Machine`: `Over` يبرز الخلية، `Running` يستدعي `choose` فيمر بـ `on_choose` القائم، و`Hidden` بعد `leave` أو إفلات مرفوض أو مهلة 10 ثوانٍ يغلق ويستدعي `on_dismiss`. فقدان التركيز لا يغلق الخلية في الانتظار. الإغلاق يحرّر الجهاز.
  - `crates/ui/src/layout.rs`: `plan_rings(cells, rtl, min_rings)`، وخلية الإفلات بحلقتين دائماً، فلا يتغير حجم النافذة ولا مركزها حين تصل الخلايا عند `enter` (السكربت يحسب موقع WEBP من الهندسة قبل الدخول). اختبار وحدة جديد له في الملف نفسه.
  - الخدمة: `Request::Drop` يُظهر خلية الإفلات، و`on_drop_files` يطابق الإجراءات ويضع الملفات في `pending`. نص المركز فارغ قبل `enter` (AC10 في S-014c).
  - الرخص في `docs/engines.md`: wayland-client 0.31.15 (MIT)، wayland-backend 0.3.17 (MIT)، rustix 1.1.5 (Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT)، كلها موجودة في الشجرة عبر winit، و`Cargo.lock` أضاف روابط فقط بلا حزم جديدة. لم يُضف smithay-client-toolkit ولا raw-window-handle (winit يعيد تصدير rwh 0.6). Slint باقٍ على 1.18.1.
  - البوابات (`nice -n 19`، `-j 2`، `--test-threads=2`، دون `WAYLAND_DISPLAY` و`DISPLAY`): `cargo fmt --all --check` نظيف، `cargo clippy --workspace --all-targets -- -D warnings` نظيف، `cargo test --workspace` 354 ناجحاً بلا فشل، `scripts/design-lint.sh crates/ui` نظيف.
  - لم يُشغَّل بطلب محمد (حمل الجهاز): `tests/drop/kwin.sh` و`tests/drop/gnome.sh` (AC5 وAC6 وAC7 كلها تكامل)، و`tests/e2e/run.sh`، و`cargo mutants` (ومنه `plan_rings` الجديدة في ملف يخضع للطفرات)، وبناء الإصدار. أي أن المستقبِل لم يُختبر على مؤلِّف بعد.
  - مخاطر للمراجِع: (1) قراءة `text/uri-list` قبل الإفلات لم تُجرَّب في التجربة (قرأتها عند `drop`)، والمفترض أن مصدر GTK 4 يجيب أثناء السحب. (2) تحرير الجهاز وطلب جديد في كل ظهور لم يُختبر مع قيد Hyprland على أول `wl_data_device` (ADR 0003، النتيجة 3)، وHyprland هو سطح التطوير. (3) خطأ بروتوكول على الاتصال الثاني يُسقط اتصال winit كله، لذلك `set_actions` و`finish` محصوران بالإصدار 3 وبالشروط أعلاه.
  - إضافة بعد المراجعة الذاتية: `crates/ui/src/drop_hive_tests.rs` (لينكس، منصة Slint المخفية) يختبر المسار على الخيط الرئيسي بإرسال الملاحظات إلى `on_note` مباشرة بدل خيط المستقبِل: الحجم ثابت عند `enter`، الإبراز، الإفلات على خلية يشغّل `on_choose` وحده، الإفلات المرفوض و`leave` بعد `enter` يستدعيان `on_dismiss`، فقدان التركيز لا يغلق خلية الإفلات، والخلية العادية تتجاهل المستقبِل. وصار أخذ علامة الربط مشروطاً بوجود نافذة winit في `RedrawRequested`، حتى لا يضيع ربط الجهاز. `cargo test --workspace` الآن 355 ناجحاً.
  - انحراف عن التجربة: جولة التهيئة (`registry_queue_init`) تجري على الخيط الرئيسي داخل `RedrawRequested` في أول ظهور بوضع الإفلات، بينما أجرتها التجربة على خيطها. تضيف جولة واحدة إلى الظهور الأول، وهذا يهم AC12.
  - الحالة: طُلب مني ضبطها على «مراجعة»، لكن الرأس يغطي القصص الست، فتركته «معتمدة» وسجّلت هنا أن S-014b جاهزة للمراجعة. القرار للمدير.
- 2026-10-20، إصلاح البوابات (الفحوص المؤجلة لـ S-014a وS-014b، في مؤلِّفات مخفية خاصة فقط):
  - البناء: `cargo build --release --workspace` و`tests/drop/kwin-input` نجحا.
  - `tests/e2e/run.sh` (KWin مخفي): كل الفحوص ناجحة، ومنها «drop starts the daemon» و«idle memory after use within 40 MB» (38 MB).
  - `tests/drop/kwin.sh` (level=1): التشغيل الأول `RESULT FAIL (1)`، فشل AC5 للظهور الأول وحده. `tests/drop/gnome.sh smart`: التشغيل الأول `RESULT FAIL (13)`. السببان في السكربتين لا في المستقبِل:
    - في GNOME أعاد `hive_geo` نافذة الخلية قبل أن يرسمها Mutter (`0 0 0x0`)، فذهبت كل حركات المؤشر إلى زاوية الشاشة ولم يصل `enter`. صار يقبل النافذة الظاهرة فقط بعرض وارتفاع أكبر من صفر.
    - الظهور الأول يعيد استعمال سطح winit المنشأ عند بدء الخدمة (`get_xdg_surface` في السطر 300 من أثر KWin، و`get_data_device` في السطر 535 بعده)، فلا `get_xdg_surface` داخل مقطع الظهور. صار AC5 يبحث عن آخر `get_xdg_surface` في الأثر كله حتى نهاية الظهور، وبقي شرط أن يأتي الجهاز بعد السطح كما هو. هذا يوسّع نافذة البحث ولا يضعف الشرط.
  - بعد الإصلاح (`216c68b`): `kwin.sh` `RESULT PASS` و`gnome.sh smart` `RESULT PASS`، الأربعة ظهورات في كلٍّ منهما، والخلية في المنتصف (`444 235 393x362` في GNOME). أي أن قراءة الملفات عند `enter` وإعادة إنشاء الجهاز في كل ظهور تعملان في المؤلِّفين. `gnome.sh strict` لم يُشغَّل: هو AC15 في S-014f، والنتيجة 6 في ADR 0003 تثبت أن `sbarhive drop` بلا رمز يفشل فيه.
  - `cargo mutants --in-diff` على تغييرات S-014a وS-014b (`uri_list.rs`، `dnd.rs`، `layout.rs`، `lib.rs`، `ipc.rs`، `clipboard.rs`، `cli/src/main.rs`)، من نسخة `git archive HEAD` نظيفة حُذفت بعده: التشغيل الأول `66 mutants tested in 20m: 8 missed, 47 caught, 9 unviable, 2 timeouts`. الناجيات: ست في خطافات winit (`FocusWatcher::window_event` عند `RedrawRequested`، و`attach_receiver`)، واثنتان في حارسَي `before != after` في `Inner::drag`.
    - الحارسان حُذفا (`6364d23`): `choose` و`dismiss` لا يفعلان شيئاً بعد إغلاق الخلية، ولا يعيد فتحها إلا ظهور جديد، فالطفرتان متكافئتان.
    - `receiver.rs` وخطافا winit مستثناة في `.cargo/mutants.toml` مع السبب: تعمل على اتصال Wayland حي فقط، ويغطيها `tests/drop/*.sh`. جانب الخيط الرئيسي (`on_note` و`Inner::drag`) باقٍ تحت الطفرات. لم تُضف اختبارات.
    - التشغيل بعد الإصلاح: `55 mutants tested in 12m: 45 caught, 8 unviable, 2 timeouts`، صفر ناجية. المهلتان في `percent_decode` (`+=` إلى `-=` و`*=`) حلقة لا تنتهي، أي أنها لا تنجو.
  - بعد حذف الحارسين أُعيد بناء الإصدار وتشغيل `kwin.sh` و`gnome.sh smart` و`tests/e2e/run.sh`: كلها ناجحة. `cargo test -p sbarhive-ui` 72 ناجحاً، وclippy وfmt نظيفان.
  - لم يُشغَّل: Hyprland (خارج النطاق)، و`server.rs` في الخدمة بلا اختبارات وحدة.
- 2026-10-20، المدير: تجربة حية من محمد على Hyprland 0.56 بالضغطة Alt أثناء سحب صورة من Nautilus: ظهرت خلية الإفلات وأُبرزت الخلية تحت الملف. خطر «أول data device فقط» في Hyprland لم يظهر في هذه التجربة. لم يُتحقق بعد من تنفيذ الإجراء بعد الإفلات على Hyprland.
- 2026-10-21، المختبِر (S-014c فقط، AC8 وAC9 وAC10)، بعد انتهاء S-018 (القرار 7 فيها): كُتبت الاختبارات من المعايير وجدول الحالات، وشُغّل كل سكربت مرة في مؤلِّف مخفي خاص (`nice -n 19`، `-j 2`)، وفشلت قبل التنفيذ.
  - النطاق: صفوف AC8 وAC9 وAC10 فقط. الدخول والخروج والمهلة (AC3) تغطيها `drop_tests.rs` و`drop_hive_tests.rs` منذ S-014a وS-014b، فاختبار جديد لها ينجح قبل التنفيذ. Esc أثناء السحب هو AC11 (S-014d)، وعلى مسار الطبقة لوحة المفاتيح 0 في وضع الإفلات (S-018 AC13)، فلا يصل Esc إلى الخلية بل يلغي السحب عند المصدر. لم يُضف له اختبار هنا.
  - الملفات: `tests/drop/lib.sh` (خطافا `on_shown` و`on_over` في `cycle`، و`action_ids`، و`enter_cells_checks`)، `tests/drop/kwin.sh` (الظهور الخامس بملف PDF، ولقطات ScreenShot2، و`shot_checks`، وحذف مجلدي mktemp عند الخروج)، `tests/drop/gnome.sh` (الظهور الخامس، `SBARHIVE_DEBUG_FOCUS=1`، وحذف مجلدي mktemp)، `tests/drop/pixels.py` (قص وعدّ ألوان الرموز ومقارنة لقطتين، مكتبة Python القياسية فقط)، `tests/drop/source.py` (أيقونة سحب شفافة من بكسل واحد، فلا تغطي اللقطة الخلية)، `tests/fixtures/docs/one-page.pdf` (صفحة واحدة مكتوبة يدوياً، 500 بايت تقريباً)، `crates/daemon/src/text.rs` (اختبار `ac10_drop_waiting_text_in_both_languages`)، و`tests/drop/README.md`.
  - العقد الذي تثبّته الاختبارات (لا يُعدَّل):
    - `text::drop_waiting(lang: Lang) -> &'static str` في `crates/daemon/src/text.rs`: نص المركز في وضع الانتظار. غير فارغ وبلا مسافات طرفية في اللغتين، العربي بحروف عربية بلا حروف لاتينية، الإنكليزي ASCII بكلمات، مختلفان، ولا يساويان `no_actions` ولا `idle`، وبلا شرطة طويلة. الصياغة للمنفِّذ.
    - الخدمة تمرّر هذا النص إلى `Hive::present_drop(centre, token)` (اليوم تمرّر نصاً فارغاً). فحص الربط في `kwin.sh`: حبر `text-primary` في وسط المركز (72 × 48 حول مركز الخلية، 15 بكسلاً أو أكثر بتسامح 40) في لقطة قبل `enter`.
    - سطر في stderr الخدمة عند كل `enter` لسحب في خلية الإفلات، بعد معرفة خلايا الملفات وقبل أي إفلات، مع `SBARHIVE_DEBUG_FOCUS=1` على الأقل: `drop cells: ID ID ...` بمعرّفات الخلايا بترتيبها، بينها مسافة واحدة، ولا شيء بعد النقطتين إن لم يطابق إجراء. كل سطر كهذا في الظهور يساوي `sbarhive actions --json` لملف الظهور.
    - اللقطتان المرجعيتان `tests/drop/snapshots/drop-en-light-waiting.png` و`tests/drop/snapshots/drop-en-light-over.png` (صندوق الخلية من لقطة KWin بمقياس 1، فاتح، تقليل الحركة، بالإنكليزية) يعتمدهما محمد. المرشحتان تُكتبان دائماً في `tests/drop/out/`. لا ينشئ المختبِر المرجع ولا المنفِّذ دون اعتماد.
  - مخرجات الفشل:
    ```
    $ cargo test -p sbarhive-daemon ac10
    error[E0425]: cannot find function `drop_waiting` in this scope
       --> crates/daemon/src/text.rs:221:14
    error[E0425]: cannot find function `drop_waiting` in this scope
       --> crates/daemon/src/text.rs:221:38
    error: could not compile `sbarhive-daemon` (bin "sbarhived" test) due to 2 previous errors

    $ tests/drop/kwin.sh level=1
    FAIL AC8 show 1: the daemon reports the cells built at enter
    FAIL AC8 show 1: every enter's cells equal sbarhive actions for the dragged file
    (ومثلهما للظهورات 2 إلى 5)
    PASS AC8: the PNG and PDF action lists are non-empty and differ
    INFO AC10 show 1: 0 text-primary pixels in the centre before the drag enters
    FAIL AC10 show 1: the waiting hive's centre shows text (text-primary ink) before enter
    FAIL AC10: no approved reference tests/drop/snapshots/drop-en-light-waiting.png (candidate: tests/drop/out/drop-en-light-waiting.png, needs the owner's approval)
    INFO AC9 show 1: accent pixels under the drag 2697; across the centre accent 0, surface-2 2779
    FAIL AC9: no approved reference tests/drop/snapshots/drop-en-light-over.png (candidate: tests/drop/out/drop-en-light-over.png, needs the owner's approval)
    RESULT FAIL (13)

    $ tests/drop/gnome.sh smart
    FAIL AC8 show 1 .. show 5 (السطران نفساهما لكل ظهور)
    RESULT FAIL (10)
    ```
  - كل فحوص S-014b وS-018d (AC5 وAC6 وAC7 وAC13 وAC14) ناجحة في التشغيلين، ومنها الظهور الخامس الجديد، فأيقونة السحب الشفافة لم تغيّر شيئاً منها.
  - ما ينجح قبل التنفيذ ولماذا: فحصا بكسلات الإبراز في AC9 (الخلية تحت السحب `accent`، والمقابلة لها `surface-2` بلا `accent`) ينجحان، لأن S-014b أضافت الإبراز (`Note::Motion` إلى `highlight`) وجرّبه محمد حياً على Hyprland. لم يُلوَ الاختبار ليفشل: يبقيان حارسين، والجزء الجديد الفاشل من AC9 هو اللقطة المعتمدة. كذلك فحص «القائمتان مختلفتان» في AC8 يتحقق من البيئة لا من التنفيذ. واختبار وحدة لتسمية المركز الوصفية في وضع الانتظار (`find_by_accessible_label` بعد `present_drop(نص)`) نجح قبل التنفيذ، لأن الواجهة تسمّي المركز بنصه أصلاً، فحُذف. الفجوة في الخدمة لا في الواجهة، ويغطيها فحص الحبر في المركز.
  - حدود: تقليل الحركة لا يُختبر بالزمن (لقطة بعد 80 ms غير مستقرة). السكربت يشغّل الخدمة بـ `SBARHIVE_REDUCED_MOTION=1`، فاللقطة المعتمدة هي الحالة النهائية بلا حركة. لقطات AC9 وAC10 على KWin وحده: المساران يرسمان المكوّن نفسه (S-018 AC7)، وGNOME يفحص AC8 فقط. النص العربي في المركز يغطيه اختبار الوحدة لا لقطة.
  - ملاحظة: ما دامت `drop_waiting` غائبة لا تُبنى اختبارات وحدة `sbarhive-daemon` كلها. هذا متوقَّع.
  - أُصلح في أثناء العمل: `tests/overlay/shot.py` يلتقط عند استدعاء D-Bus لكنه لا يكتب الملف إلا بعد انتظار 5 ثوانٍ لخيط القراءة، وقد يعلق عند الخروج، فاصطدم في التشغيل الأول بمهلة العشر ثوانٍ. صار يُشغَّل في الخلفية بـ `timeout 10` ويُنتظر 1.5 ثانية والخلية ثابتة، وتُفحص البكسلات بعد الظهورات. أُعيد تشغيل `kwin.sh` لهذا السبب (التشغيلان الأولان كانا بسكربت معطوب).
  - إضافة بعد المراجعة الذاتية (`fe75f9f` ثم هذا الإيداع):
    - حارس توقيت لفحص AC10 في `kwin.sh`: لقطة الانتظار تُقبل فقط إن خلا صندوق خلية WEBP فيها من `surface-2` و`accent` (لا خلايا قبل `enter`)، وإلا `FAIL AC10 show 1: the waiting shot was taken before enter (no cells yet)`. السبب: لقطة متأخرة بعد `enter` تُظهر نص التحديد الذي تضعه S-014b بلون `text-primary` فتنجح زوراً. تحقق الحارس على اللقطتين المحفوظتين دون تشغيل جديد: لقطة الانتظار `surface-2 0` و`accent 0` (يقبلها، والحبر 0 فيفشل AC10 كما سبق)، ولقطة المرور كأنها لقطة انتظار متأخرة `accent 2697` (يرفضها).
    - خُفّفت قيود اختبار `drop_waiting`: الإنكليزي بحروف لاتينية ودون حروف عربية (لا يُشترط ASCII، فتُقبل علامات مثل «’»)، والعربي بحروف عربية (يُقبل «PDF» داخله). بقية القيود كما هي، وهي اختيار المختبِر لا نص المعيار.
    - لمحمد عند اعتماد اللقطة: في لقطة الانتظار المرشحة يظهر خافتاً نص «drag me» من نافذة `source.py` تحت الخلية المركزية، ولون المركز `#E0D4BC` لا `surface-3` الفاتح `#DFD3BB`، أي أن المركز في حالة الانتظار ليس معتماً تماماً. في لقطة المرور المركز `#DFD3BB` بالضبط. القاعدة: لا شفافية حيث لا ضبابية. لا فحص لهذا في الاختبارات، والقرار لمحمد عند الاعتماد.
    - `scripts/design-lint.sh` (صف AC9) بوابة مراجعة، نظيفة اليوم على `tests/drop` و`text.rs`، وليست اختباراً فاشلاً.
- 2026-10-21، المنفِّذ (S-014c، AC8 وAC10، وتحقيق في عتامة المركز):
  - `text::drop_waiting` في `crates/daemon/src/text.rs` بصياغة المدير: «أفلت الملف هنا» و"Drop files here". `server.rs` يمرّرها بلغة الخدمة إلى `ui::present_drop(centre, token)`، ومنها إلى `Hive::present_drop`.
  - سطر `drop cells: ID ...` في `crates/daemon/src/ui.rs`، في ردّ `on_drop_files` عند كل `enter` وقبل فحص القائمة الفارغة، مع `SBARHIVE_DEBUG_FOCUS` فقط. المعرّفات بترتيب الخلايا نفسه، ولا شيء بعد النقطتين إن لم يطابق إجراء (`drop_cells_line`، ومعه اختبار وحدة صغير). تحققت أن `sbarhive actions` يصفّي كما `State::matching`: السجل المضمّن مع مجلد إجراءات المستخدم نفسه، والمحركات المتاحة نفسها.
  - عتامة المركز في لقطة الانتظار (`#E0D4BC` بدل `#DFD3BB`، ونص «drag me» يظهر تحته): السبب في المؤلِّف لا في الخلية. لا شيء في `hive.slint` ولا `hex-cell.slint` يغيّر الشفافية، و`Bgra::blend` يعطي ألفا 255 لأي تعبئة معتمة فوق مخزن فارغ. الدليل: تجربة في KWin مخفي خاص (خارج السكربتات، في مجلد العمل المؤقت)، `sbarhive drop` ولقطتان: عند أول إطار وبعده بـ 1.5 ثانية. بإعدادات KWin الافتراضية مركز اللقطة الأولى `#9A9281` فوق الخلفية السوداء وحافته أصغر، والثانية `#DFD3BB` بالضبط. مع `[Plugins] scaleEnabled=false` في kwinrc اللقطتان `#DFD3BB` وحد `#D2C6AE`. في الحالتين أرسلت الخدمة مخزناً واحداً للخلية (مخزنان مع بكسل الملتقِط)، أي أن الخلية لم تُرسم إلا مرة.
    - التفسير: تأثير «scale» في KWin 6 (مفعّل افتراضياً، `effects/scale`) يحرّك كل نافذة عادية جديدة من شفافية 0 ومقياس 0.8 خلال 200 ms. اسم طبقتنا `sbarhive` لا يطابق أي نوع خاص في KWin فتُعدّ نافذة عادية. `on_shown` يلتقط عند أول سطر `hive surface: layer`، أي في آخر الحركة (ألفا نحو 0.97).
    - لم أغيّر الخلية: في الحالة المستقرة السطح معتم تماماً. تغيير اسم الطبقة إلى نوع يتجاهله التأثير يغيّر سلوك KWin في التركيز وقواعد Hyprland، وهو قرار خارج هذه القصة. وعند المستخدم يطفئ إعداد تقليل الحركة في KDE (`AnimationDurationFactor=0`) هذه الحركة.
    - الاختبار نفسه سليم لكن لقطة الانتظار المرشحة تُظهر آخر حركة المؤلِّف لا الحالة المستقرة. اقتراح للمختبِر لا أنفّذه: في `tests/drop/kwin.sh` يُضاف `[Plugins]` و`scaleEnabled=false` (أو `[KDE]` و`AnimationDurationFactor=0`) إلى kwinrc الخاص، كما يشغّل السكربت الخدمة بتقليل الحركة. القرار لمحمد قبل اعتماد لقطة الانتظار.
  - البوابات: `cargo fmt --all --check` نظيف، `cargo clippy --workspace --all-targets -- -D warnings` نظيف، `cargo test --workspace` كله ناجح، `scripts/design-lint.sh crates/ui` نظيف.
  - `tests/drop/kwin.sh level=1` (مرة واحدة): `RESULT FAIL (2)`، والفشلان هما المرجعان غير المعتمدين فقط (AC10 waiting وAC9 over). AC8 ناجح في الظهورات الخمسة (سطر واحد لكل ظهور يساوي `sbarhive actions`)، وحبر المركز في لقطة الانتظار 114 بكسلاً، وحارس التوقيت ناجح. `tests/drop/gnome.sh smart` (مرة واحدة): `RESULT PASS`.
  - المرشحتان للاعتماد بالعين: `tests/drop/out/drop-en-light-waiting.png` و`tests/drop/out/drop-en-light-over.png`. لم أنشئ `tests/drop/snapshots/`.
  - لم يُشغَّل: `cargo mutants` وe2e (بطلب المدير). النص العربي في المركز يغطيه اختبار الوحدة لا لقطة.
  - الحالة: الرأس يغطي القصص الست، فتركته «معتمدة» كما فعل منفّذ S-014b، وأسجّل هنا أن S-014c جاهزة للمراجعة.
- 2026-10-21، المدير: الشفافية في لقطة الانتظار كانت من حركة فتح KWin (تأثير scale، 200 ms)، لا من الخلية. أوقفها المدير في kwinrc الخاص بـ `tests/drop/kwin.sh` (`[Plugins] scaleEnabled=false`) لتكون اللقطة ثابتة. اللقطتان `drop-en-light-waiting.png` (المركز وحده «Drop files here»، معتم بلون surface-3 تماماً) و`drop-en-light-over.png` (WEBP مُبرزة وتسميتها في المركز) فُحصتا بالعين ومعتمدتان بالتفويض. التشغيل التالي: `RESULT PASS`. حركة KWin على سطح الطبقة لمستخدمي KDE تُراجع مع S-017.
- 2026-10-21، المختبِر (S-014d فقط، AC11 وAC12): كُتبت الاختبارات من المعايير وجدول الحالات، وشُغّل كل سكربت مرة في مؤلِّف مخفي خاص (`nice -n 19`، `-j 2`)، والمتغيرات المكتبية مُلغاة قبل `dbus-run-session`. أُعيد تشغيل `kwin.sh shortcut` مرة واحدة لأن السكربت كان معطوباً (انظر أدناه).
  - الملفات: `tests/drop/lib.sh` (`start_drag` و`shortcut_show` و`shortcut_drop` و`shortcut_esc` و`shortcut_shows`)، `tests/drop/kwin.sh` و`tests/drop/gnome.sh` (وضع `shortcut`، والوضع الافتراضي كما هو)، `tests/drop/budget.sh` (جديد)، `tests/drop/docs.sh` (جديد)، `crates/ui/tests/measure.sh` (`DROP=1`)، و`tests/drop/README.md`.
  - ربط الاختصار في الاختبار: السكربت لا يشغّل `sbarhive drop` بنفسه في وضع `shortcut`. Super+Z مربوط داخل المؤلِّف الخاص بغلاف يكتب سطر وقت ثم ينفّذ `sbarhive drop`. في KWin بطريقة «اختصار أمر مخصص» في KDE التي سيصفها التوثيق: ملف `.desktop` بـ `X-KDE-GlobalAccel-CommandShortcut=true` ومدخل `[services]` في `kglobalshortcutsrc`، يشغّله kglobalacceld داخل KWin (تحققت بتجربة قصيرة دون سحب: ضغطتان أعطتا علامتين). في GNOME بـ `grab_accelerator('<Super>z')` في الوضع NORMAL عبر `Shell.Eval` كما في التجربة (ADR 0003، النتيجة 4)، لأن اختصارات gsd-media-keys خارج النطاق. المفاتيح تمر بمسار إدخال المؤلِّف (fake_input في KWin، وRemoteDesktop في GNOME).
  - تفسيران للمدير:
    - «3 من 3 تشغيلات»: ثلاث سحبات متتالية بالاختصار في جلسة مؤلِّف واحدة لكل مؤلِّف، لا ثلاث جلسات، احتراماً لحمل الجهاز. إن أراد المدير ثلاث جلسات فيكفي تشغيل `kwin.sh level=1 shortcut` و`gnome.sh smart shortcut` ثلاث مرات.
    - Esc: على مسار الطبقة لوحة المفاتيح 0 في وضع الإفلات (S-018 AC13)، فلا يصل Esc إلى الخلية، بل يلغي المؤلِّف السحب عند المصدر. لذلك حالتان: (1) Esc والسحب فوق خلية WEBP بعد `enter`: المصدر يبلغ `drag-cancel`، والخلية تختفي خلال 500 ms من Esc بحسب طوابع أثر الخدمة (`zwlr_layer_surface_v1.destroy` في KWin و`xdg_toplevel.destroy` في GNOME، مقارنة بـ `date -u` عند Esc)، ولا ملف. (2) Esc قبل دخول السحب إلى الخلية: لا يصلها شيء، فتُغلق بمهلتها (تُفحص خلال 12 ثانية)، ولا ملف. وفي KWin لا يصل الخدمة `wl_keyboard.enter` ولا `key` في الحالتين. لم تُخفَّف حالة «فوق الخلية» إلى المهلة.
  - تعارض للمدير: سطر Alt عند محمد ليس ربط «نقرة ثم إفلات المفتاح» (`bindr`)، بل ربط ضغط غير مستهلك `ALT + ALT_L` بصيغة Lua يشغّل مساعداً (ينتظر 0.4 ثانية ويتحقق أن Alt والزر الأيسر ما زالا مضغوطين، ويحتاج مجموعة `input`)، والمساعد في مجلد محمد لا في المستودع. لذلك لا يثبّت `docs.sh` نوع الربط: يكفي سطر بالمفتاح `ALT_L` يشغّل sbarhive بصيغة hyprlang أو Lua، لكن كل برنامج يشغّله سطر موثَّق يجب أن يكون `sbarhive` أو ملفاً في المستودع، ولا مسارات `/home/`. السؤال: هل يُشحن المساعد؟ إن لم يُشحن فسطر `bindr = ALT, ALT_L, exec, sbarhive drop` يرضي الفحص. السؤال يمس فحص Alt وحده.
  - العقد الذي تثبّته الاختبارات (لا يُعدَّل):
    - التوثيق في `docs/` (أي ملف `.md` خارج `docs/adr/` و`docs/PLAN.md`)، بالعربية وبالإنكليزية، كل لغة في قسم (بين عنوانين) لغته غالبة في النثر: GNOME (الإعدادات، لوحة المفاتيح، اختصار مخصص / Settings، Keyboard، Custom Shortcuts، والأمر `sbarhive drop`)، KDE (إعدادات النظام، الاختصارات / System Settings، Shortcuts، والأمر `sbarhive drop`)، Hyprland (سطر Super+Z يشغّل `sbarhive drop`، وسطر `ALT_L` يشغّل sbarhive)، وذكر Super+Z. والسطران نفساهما في قسمي `integrations/linux/hyprland/README.md`. ولا شرطة طويلة.
    - `hive_bench show --drop` يُظهر الخلية كما يُظهرها `sbarhive drop` (`Hive::present_drop`)، ويطبع `mode=drop` في سطر كل ظهور. `measure.sh` يمرّره مع `DROP=1`.
  - مخرجات الفشل:
    ```
    $ tests/drop/docs.sh
    INFO sections in docs/: none;
    FAIL AC12: docs/ in Arabic: GNOME: Settings, Keyboard, custom shortcut running `sbarhive drop`
    FAIL AC12: docs/ in English: GNOME: Settings, Keyboard, custom shortcut running `sbarhive drop`
    FAIL AC12: docs/ in Arabic: KDE: System Settings, Shortcuts, command `sbarhive drop`
    FAIL AC12: docs/ in English: KDE: System Settings, Shortcuts, command `sbarhive drop`
    FAIL AC12: docs/ in Arabic: Hyprland: a Super+Z bind line running `sbarhive drop`
    FAIL AC12: docs/ in English: Hyprland: a Super+Z bind line running `sbarhive drop`
    FAIL AC12: docs/ in Arabic: Hyprland: a left Alt (ALT_L) bind line running sbarhive
    FAIL AC12: docs/ in English: Hyprland: a left Alt (ALT_L) bind line running sbarhive
    FAIL AC12: docs/ in Arabic: names the default shortcut Super+Z
    FAIL AC12: docs/ in English: names the default shortcut Super+Z
    FAIL AC12: integrations/linux/hyprland/README.md in Arabic: hypr-superz bind line
    FAIL AC12: integrations/linux/hyprland/README.md in English: hypr-superz bind line
    FAIL AC12: integrations/linux/hyprland/README.md in Arabic: hypr-alt bind line
    FAIL AC12: integrations/linux/hyprland/README.md in English: hypr-alt bind line
    FAIL AC12: the documented binds run programs that exist (a bind was found)
    FAIL AC12: every program a documented bind runs is sbarhive or a file the repository ships
    PASS AC12: no /home/ path in the docs
    PASS AC12: no em dash or en dash in the docs
    RESULT FAIL (16)

    $ tests/drop/budget.sh   (measure.sh 20 1، DROP=1، KWin مخفي)
    INFO drop: shows 21 (warm 20), mode=drop lines 0, hive surface layer 21 window 0, hive focus: true 20, median 2.7 ms
    PASS AC12: measure.sh ran all 21 daemon shows
    FAIL AC12: every show line says mode=drop (hive_bench show --drop)
    PASS AC12: every show was a layer surface (one "hive surface: layer" line per show, no window line)
    FAIL AC12: no show took the keyboard (no "hive focus: true": drop mode, S-018 AC13)
    PASS AC12: median warm first frame of drop-mode shows under 150 ms
    RESULT FAIL (2)
    ```
    السبب: `hive_bench show` يتجاهل `--drop` فيُظهر الخلية العادية التي تأخذ لوحة المفاتيح، ولا توثيق للربط في `docs/` ولا سطر `sbarhive drop` في توثيق Hyprland.
  - ما ينجح قبل التنفيذ ولماذا: AC11 كله. `tests/drop/kwin.sh level=1 shortcut` و`tests/drop/gnome.sh smart shortcut` كلاهما `RESULT PASS`: 3 من 3 سحبات كتبت WEBP، وفي كل منها شغّل الربط `sbarhive drop` مرة واحدة أثناء السحب، و`deleted=False`. Esc فوق الخلية: أُتلف سطح الخلية بعد 0 ms (KWin) و4 ms (GNOME) من Esc. Esc قبل الخلية: أُغلقت بالمهلة بعد 9894 ms (KWin) و9731 ms (GNOME). السبب أن السلوك كله بنته S-014a إلى S-014c، والربط في الاختبار خارجي (المؤلِّف). لم يُلوَ الاختبار ليفشل، ويبقى حارساً، والمنفِّذ لا يحتاج شيفرة له.
  - أُصلح في أثناء العمل (سبب إعادة `kwin.sh shortcut`): في التشغيل الأول `RESULT FAIL (3)` لعيبين في السكربت: حلقة `start_drag` غيّرت المتغير `i` في `shortcut_shows` («0 of 3» مع أن الفحوص الثلاثة نجحت)، وGTK 4 يرسل `drag-end deleted=False` بعد `drag-cancel`، فصار الفحص «`drag-cancel` ولا `drag-end deleted=True`» كما في S-014b.
  - حدود: زمن AC12 لا يشمل ما بين الضغطة وبدء عملية `sbarhive drop` (تشغيل المؤلِّف لها)؛ `measure.sh` يقيس من طلب IPC إلى أول إطار. `budget.sh` يبني `hive_bench` بـ `CARGO_BUILD_JOBS=2` ويحذف مجلد `measure.sh` المؤقت. لم يُشغَّل: الوضع الافتراضي لـ `kwin.sh` و`gnome.sh` (لم يتغير منطقه، `bash -n` و`shellcheck -x` نظيفان)، و`cargo mutants`، وe2e.
  - إضافة بعد المراجعة الذاتية (إيداع ثانٍ، دون تشغيل مؤلِّف جديد):
    - `docs.sh`: قسم Hyprland في `integrations/linux/hyprland/README.md` لا يُشترط أن يذكر كلمة Hyprland (الملف كله عنها)، فيكفي قسم فرعي بالسطرين. والعربية تُقارن بلا حركات ومع قراءة أ وإ وآ ألفاً. أُعيد تشغيله (بلا مؤلِّف): `RESULT FAIL (16)` كما سبق، وتحققت على مستند مصطنع خارج المستودع أنه ينجح بتوثيق صحيح.
    - حالة Esc فوق الخلية: صار الفرق محسوباً بكسر عشري ويُشترط أن يكون بين 0 و500 ms، فلا ينجح إتلاف قبل Esc. تحقق على الأثرين المحفوظين دون تشغيل جديد: أول إتلاف بعد Esc عند `22:16:06.531874` في KWin (Esc عند `22:16:06.531110`) و`22:17:27.916039` في GNOME (Esc عند `22:17:27.911701`)، أي بعد Esc لا قبله.
    - إضافات إلى العقد: كل موضوع (GNOME، KDE، Hyprland) بنصه وكتلة شيفرته تحت عنوان واحد في كل لغة، دون عنوان فرعي بينهما. ويبقى `tests/overlay/docs.sh` ناجحاً (AC20 فيه يقرأ ملف Hyprland نفسه: قاعدة layerrule سطراً لكل لغة، موسومة بأنها لم يُتحقق منها). وسطر في CHANGELOG.md.
    - للمراجِع: منطق الوضع الافتراضي في `kwin.sh` و`gnome.sh` نُقل داخل `if/else` ولم يُشغَّل بعد التعديل (فقط `bash -n` و`shellcheck -x`). يُشغَّلان في المراجعة.
- 2026-10-22، المنفِّذ (S-014d): بقرار المدير في سؤال المختبِر عن مساعد Alt: لا مساعد يُشحن، وربط Alt الموثَّق يستعمل `sbarhive drop --hold` و`--release` من S-019e، وموضع المؤشر في Hyprland بـ `--at "$(hyprctl cursorpos)"` (S-019، القرار 1).
  - الملفات: `docs/drop-shortcut.md` (جديد، بالعربية والإنكليزية: GNOME من الإعدادات، KDE من إعدادات النظام، Hyprland بسطر `bind = SUPER, Z` لظهور فوري وسطري `bindn`/`bindri` على `ALT_L` للإمساك، والافتراضي Super+Z)، `integrations/linux/hyprland/README.md` (سطرا `ALT_L` بصيغتي hyprlang وLua في القسمين، بعد أسطر Super+Z التي أضافتها S-019 دون تغييرها)، `README.md` (سطر يحيل إلى `docs/drop-shortcut.md` في القسمين)، وسطر في CHANGELOG.md. لا شيفرة Rust: `hive_bench show --drop` موجود منذ S-019 ويطبع `mode=drop`.
  - النتائج (كلها داخل `systemd-run` بـ MemoryMax=6G وMemoryHigh=5G وCPUQuota=200%، `nice -n 19`، `CARGO_BUILD_JOBS=2`، وKWin مخفي خاص واحد لـ budget.sh): `tests/drop/docs.sh` RESULT PASS؛ `tests/drop/budget.sh` RESULT PASS (21 ظهوراً كلها `mode=drop` وطبقة، `hive focus: true` صفر، الوسيط 51.3 ms والأقصى 52.1)؛ `tests/drop/hold_docs.sh` RESULT PASS؛ `tests/overlay/docs.sh` RESULT PASS؛ `scripts/design-lint.sh` نظيف.
  - ملاحظة: الوسيط 51.3 ms لا 2.7 كما في تشغيل المختبِر، لأن ظهور الإفلات صار ينتظر موضع المؤشر حتى 50 ms (S-019c) دون `--at`. يبقى تحت 150.
  - لم يُشغَّل: cargo fmt وclippy وtest وmutants، إذ لم تتغير شيفرة Rust. لم يُجرَّب شيء من التوثيق على جلسة حقيقية. موسوم في التوثيق بأنه غير متحقق: أسطر Hyprland كلها، والاختصار المخصص في GNOME أثناء السحب (ADR 0003 يذكر أن الاختبار كان بـ `grab_accelerator` لا gsd-media-keys)، وأثر Alt غير المستهلِك على نوع الإفلات. خطوات KDE هي طريقة «أمر أو سكربت» التي استعملها `kwin.sh shortcut` (kglobalacceld) فلم تُوسم.
  - تنبيه: `docs.sh` يفحص كل `.md` في `docs/` ومنها `docs/launch/intro-video-plan.md` غير المتتبَّع وليس من هذه المهمة. ينجح اليوم، لكن مسار `/home/` أو شرطة طويلة فيه لاحقاً تُفشل الفحص.
- 2026-10-22، المراجِع (S-014d، المراجعة 1): **REWORK** (1 من 2).
  - شُغّل كل شيء مرة واحدة داخل `systemd-run --user --scope` (MemoryMax=6G، MemoryHigh=5G، CPUQuota=200%، `nice -n 19`، `CARGO_BUILD_JOBS=2`)، ومؤلِّف مخفي واحد في كل مرة: `tests/drop/docs.sh` RESULT PASS؛ `tests/drop/hold_docs.sh` RESULT PASS؛ `tests/overlay/docs.sh` RESULT PASS؛ `scripts/design-lint.sh` نظيف؛ `tests/drop/budget.sh` RESULT PASS (21 ظهوراً كلها `mode=drop` وطبقة، `hive focus: true` صفر، الوسيط 51.2 ms والأقصى 52.4)؛ `tests/drop/kwin.sh level=1 shortcut` RESULT PASS (3 من 3 كتبت WEBP، Esc فوق الخلية: أُتلفت بعد 1.1 ms، Esc قبلها: أُغلقت بالمهلة بعد 9892.5 ms).
  - بوابات cargo: لا تنطبق. لا تغيّر إيداعات S-014d الخمسة (1498d5c، 69f8a18، d1bef4f، baf35ac، a53124f) أي ملف `.rs` ولا `Cargo.toml` ولا `Cargo.lock` (`git show --stat` على كل منها فارغ لهذه المسارات). لم يُشغَّل `cargo mutants` بطلب المدير، ولا شيفرة متغيرة له.
  - لم يُشغَّل في هذه الجولة: `tests/drop/gnome.sh` (أي وضع)، والوضع الافتراضي لـ `kwin.sh` و`gnome.sh` بعد نقل منطقه داخل `if/else` كما طلب المختبِر. تشغّله المراجعة التالية.
  - الأسباب (يصلحها المنفِّذ):
    1. (عالية) ربط ضغط Alt خاطئ: `bindn = ALT, ALT_L, ...` وبصيغة Lua `hl.bind("ALT + ALT_L", ...)`. في ضغطة Alt الأولى لا يكون Alt مُعدِّلاً ممسوكاً بعد، فلا يطابق الربط. دليل حي من Hyprland 0.56 عند محمد: إعداده في `~/.config/hypr/custom/keybinds.lua` (الأسطر 160 إلى 164) يقول إن `"ALT + ALT_L"` لم يطابق إلا من الضغطة الثانية، فربطه `"ALT_L"` مع `non_consuming = true` و`ignore_mods = true`. (وسجل المختبِر وصف ربط محمد خطأً بأنه `ALT + ALT_L`.) المطلوب: hyprlang `bindni = , ALT_L, exec, ...`، وLua `hl.bind("ALT_L", ..., { non_consuming = true, ignore_mods = true, ... })`، في أربعة مواضع: `docs/drop-shortcut.md` بالعربية والإنكليزية، و`integrations/linux/hyprland/README.md` بالعربية والإنكليزية. وتُصحَّح معه ملاحظة README «ربط الضغط على Alt وحده أو أي مُعدِّل يجب أن يكون غير مستهلِك (العلم `n`)»: يحتاج المُعدِّل وحده `i` أيضاً، باللغتين. وجملة `docs/drop-shortcut.md` «ربط الضغط غير مستهلِك (العلم `n`)» تذكر `i` كذلك.
    2. (متوسطة) `HOME ..` في أسطر Lua ليس متغيراً يعرّفه Hyprland: لا يوجد في `/usr/share/hypr/stubs/hl.meta.lua`، وعند محمد يعرّفه إعداده نفسه (`~/.config/hypr/hyprland/lib/init.lua`: `HOME = os.getenv("HOME")`). في `hyprland.lua` عادي يكون `nil` فيفشل الوصل ويسقط تحميل الإعداد. المطلوب: `os.getenv("HOME") .. "/.local/bin/sbarhive ..."` في كل أسطر Lua في `integrations/linux/hyprland/README.md`، الجديدة (Alt) وأسطر Super+Z من S-019e في القسم نفسه، باللغتين.
    3. (منخفضة) ملاحظة GNOME باللغتين تقول إن الاختبار المخفي سجّل المفتاح «بآلية بوابة الاختصارات نفسها / the way the GlobalShortcuts portal does». ما فعله الاختبار (هذا السجل، سطر المختبِر): `grab_accelerator('<Super>z')` في GNOME Shell عبر `Shell.Eval`. المطلوب وصف ما فعله الاختبار دون نسبته إلى البوابة.
    4. (منخفضة) GNOME الخطوة 3: زر حوار الاختصار المخصص اسمه «إضافة / Add»، لا «احفظ / save».
  - للتصعيد إلى المدير أو محمد (لا يصلحه المنفِّذ): AC11 ينص على أن الخلية تختفي خلال 500 ms بعد Esc أثناء السحب. حالة «Esc قبل دخول السحب إلى الخلية» تبقى فيها الخلية 9.9 ثوانٍ حتى مهلتها، لأن وضع الإفلات بلا لوحة مفاتيح (S-018 AC13) ولا يصل الخدمة شيء من سحب لم يدخل. فسّر المختبِر الحالة بالمهلة ورفعها للمدير، ولا قرار مسجلاً في هذا الملف ولا في `docs/PLAN.md`. يحتاج تعديل AC11 أو قبول الانحراف صراحة.
  - صحيح ومتحقق: `exec` في Hyprland يمر عبر `/bin/sh -c` (السلسلة `/bin/sh` في `/usr/bin/Hyprland`، ووجود `hl.dsp.exec_raw` منفصلاً عن `exec_cmd`)، فيعمل `"$(hyprctl cursorpos)"` و`~`، و`parse_at` في `crates/cli/src/main.rs` يقبل `X, Y` بمسافة كما يطبعها `hyprctl cursorpos`. خيارات Lua (`non_consuming`، `release`، `ignore_mods`) موجودة في `HL.BindOptions`. ربط الترك `bindri` و`"ALT_L"` مع `release` و`ignore_mods` سليم. مسار KDE («أمر أو سكربت» في الاختصارات) هو ما شغّله `kwin.sh shortcut` عبر kglobalacceld. مسار GNOME في الإعدادات صحيح لـ GNOME 50 وموسوم بأنه غير متحقق أثناء السحب. العربية سليمة، ولا شرطة طويلة في الملفات المتغيرة.
  - ملاحظة غير حاجبة: `docs.sh` يفحص `docs/launch/` غير المتتبَّع (كما نبّه المنفِّذ)؛ هشاشة في الاختبار لا عيب في المهمة.
  - ملاحظة إجرائية: شغّل المراجِع `hyprctl version` مرة واحدة على Hyprland الحي (قراءة فقط) لمعرفة الإصدار، قبل أن ينتبه للقيد؛ لم يُشغَّل غيره عليه.

### المدير: قرار AC11 بعد REWORK 1 لـ S-014d (2026-10-22)

- **Esc قبل دخول السحب إلى الخلية:** يُقبل الانحراف صراحة. وضع الإفلات لا يأخذ لوحة المفاتيح عمداً (S-018 AC13)، والسحب الذي لم يدخل لا يرسل شيئاً، فتُغلق الخلية بمهلتها (10 ثوانٍ). ومع الضغط المطوّل (S-019e) تُغلق برفع المفتاح، وهو المسار الموثّق. شرط 500 ms يبقى لـ Esc فوق الخلية بعد الدخول.
- البنود 1 إلى 4 للمنفِّذ: ربط Alt بصيغة `bindni = , ALT_L` و`"ALT_L"` مع `non_consuming` و`ignore_mods`؛ و`os.getenv("HOME")` بدل `HOME ..` في كل سطر Lua؛ وتصحيح وصف آلية GNOME وزر «Add».

- 2026-10-22، المنفِّذ (S-014d، إصلاح REWORK 1): **محجوبة، تنتظر قرار المدير.**
  - البنود 1 و3 و4 منفّذة في `docs/drop-shortcut.md` و`integrations/linux/hyprland/README.md` باللغتين: ربط الضغط `bindni = , ALT_L, exec, ...` وربط الترك `bindri = , ALT_L, exec, ...` (بلا `ALT` في الربطين)، وLua `hl.bind("ALT_L", ...)` مع `non_consuming = true` و`ignore_mods = true`. ملاحظة README عن `n` تذكر `i` الآن مع السبب (Alt لا يكون ممسوكاً في الضغطة الأولى، فربط `ALT, ALT_L` لا يطابق إلا من الثانية)، وكذلك جملة العلم `n` في `docs/drop-shortcut.md`. ملاحظة GNOME تقول إن الاختبار سجّل المفتاح بـ `grab_accelerator('<Super>z')` داخل GNOME Shell عبر `Shell.Eval`. الخطوة 3 في GNOME: «إضافة» و"Add".
  - البند 2 منفّذ حرفياً: أسطر Lua الثمانية في README (Super+Z وAlt، باللغتين) تستعمل `os.getenv("HOME") ..`. لا يبقى `HOME ..` في `docs/` ولا `integrations/` ولا `README.md`.
  - النتائج (كل أمر داخل `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200%` و`nice -n 19`، بلا مؤلِّف): `tests/drop/hold_docs.sh` RESULT PASS؛ `tests/overlay/docs.sh` RESULT PASS؛ `scripts/design-lint.sh` نظيف؛ `tests/drop/docs.sh` **RESULT FAIL (1)**، وكل فحوصه الأخرى ناجحة.
  - سبب الفشل: الفحص «every program a documented bind runs is sbarhive or a helper shipped in integrations/ or scripts/» في `tests/drop/docs.sh` (الأسطر 134 إلى 136). النمط `exec_cmd\((.{0,200}?)\)` يتوقف عند أول `)`، وهو قوس `os.getenv` نفسه، فتصبح المجموعة `os.getenv("HOME"` ويُقرأ البرنامج `os.getenv(` (السطر `INFO not shipped: os.getenv(`). حذف `HOME ..` في الاختبار لا يكفي: الإصلاح في الاختبار يحتاج تغيير الالتقاط نفسه. لم أعدّل الاختبار، ولم ألتف عليه (لا `"$HOME/..."` داخل السلسلة ولا متغير محلي باسم `sbarhive`).
  - خياران للمدير: (أ) يعدّل المختبِر المستخرِج ليفهم `os.getenv("HOME") ..`؛ (ب) يعدل المدير البند 2 إلى `hl.dsp.exec_cmd("$HOME/.local/bin/sbarhive ...")`، وهي صيغة يقبلها الاختبار اليوم، وتعمل لأن `exec_cmd` يمر عبر `/bin/sh -c` كما تحقق المراجِع.
  - لم تُرسل للمراجعة: بوابة حمراء تعني REWORK تلقائياً، وهذه المحاولة الثانية والأخيرة. لا شيفرة Rust متغيرة، فلم يُشغَّل cargo. لم يُلمس سطح المكتب الحي ولم يُشغَّل `hyprctl`.
