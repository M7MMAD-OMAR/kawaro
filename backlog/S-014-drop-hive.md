# S-014 إفلات ملف مسحوب على خلية بعد اختصار أثناء السحب
الحالة: معتمدة

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
