# S-021 خلية الإفلات لا تظهر إلا لسحب حقيقي لملفات لها إجراء
الحالة: معتمدة (2026-10-22)؛ S-021a في المراجعة، وS-021b وS-021c لم تبدأ

بلاغ محمد (2026-10-22، الإصدار 0.3.0 مثبت): دون تحديد ولا سحب، إمساك Alt وتحريك الفأرة يُظهر الخلية. المطلوب: لا تظهر خلية الإفلات إلا أثناء سحب ملف فعلي، ولملفات لها إجراء واحد على الأقل.

السبب: `~/.config/hypr/custom/keybinds.lua` يربط ضغط ALT_L (وSUPER+Z) بـ `sbarhive drop --hold --at "$(hyprctl cursorpos)"` والترك بـ `drop --release`. بعد `hold_ms` (400 ms) ترسم الخدمة الخلية (`server.rs` `Request::Drop`، ثم `ui::present_drop`، والبوابة `sbarhive_core::hold`، والحالة `Waiting` في `crates/ui/src/dnd.rs`) دون أي فحص لوجود سحب. الملفات لا تُعرف إلا عند `wl_data_device.enter` (`receiver.rs` `enter()` ثم `read_files` و`Note::Enter`). ولملف بلا إجراء تبقى الخلية مرسومة بصفر خلايا.

البحث:
- لا إشارة عامة في Wayland لوجود سحب: `wl_data_device.enter` يصل لسطح يملكه العميل فقط (https://doc.servo.org/wayland_client/protocol/wl_data_device/struct.WlDataDevice.html)، ولا حدث سحب في IPC الخاص بـ Hyprland (https://wiki.hypr.land/IPC/).
- أثناء السحب يمسك جهاز البيانات المؤشر، فتذهب أحداثه إليه لا إلى `wl_pointer` (https://phabricator.kde.org/D1046).
- `wl_data_offer.receive` مسموح قبل الإفلات (https://wayland.app/protocols/wayland#wl_data_offer:request:receive)، و`receiver.rs` يفعله اليوم.
- بديل مرفوض: `~/.config/hypr/custom/scripts/sbarhive-alt-hold.py` يقرأ BTN_LEFT من `/dev/input`. يحتاج مجموعة `input`، ولا يرى السحب بالنقر على لوحة اللمس، ولا يعرف إن كان للملف إجراء.

الدليل: Hyprland 0.56.2 خاص متداخل، 2026-10-22، مسار الطبقة، `WAYLAND_DEBUG=1`، بناء 0.3.0 المثبت (سكربت مؤقت من `repro.sh`):
- P1، `drop --hold` دون سحب والمؤشر ساكن داخل مستطيل الخلية: الخلية مرسومة بعد العتبة، ولا حدث عند الربط. مع أول حركة (بعد نحو 300 ms) يصل `wl_pointer.enter` على الماسك، ولا حدث من جهاز البيانات.
- P2، المؤشر خارج المستطيل ثم يدخله: `wl_pointer.enter` لحظة العبور.
- P3، سحب GTK لملف PNG (`tests/drop/source.py`) واليد ساكنة: لا حدث عند الربط. مع أول حركة يصل `wl_data_device.enter` وبين عروضه `text/uri-list`. ثم `set_actions` و`receive` و`accept` خلال 0.4 ms، و`drop cells:` بـ 15 إجراءً للصور، ثم `motion`. الدخول عند 480،270 (نصف حجم السطح، خاصية Hyprland المسجّلة في S-020).
- P4، سحب `notes.unknownext` (`sbarhive actions` لا يعيد شيئاً): `enter`، ثم `drop cells:` فارغ، والخلية باقية (الخلل).

الخلاصة: على Hyprland يحسم الأمرَ أولُ حدث على الماسك. `wl_pointer.enter` يعني لا سحب، و`wl_data_device.enter` بعرض `text/uri-list` يعني سحب ملفات.

## القصة
بصفتي مستخدماً على Hyprland أو KDE، أريد ألا تظهر خلية الإفلات حين أمسك المفتاح دون سحب أو أسحب ملفاً بلا إجراء، وأن تظهر فوراً حين أسحب ملفاً له إجراء، حتى لا تقاطعني الخلية في عملي العادي.

## قرارات المخطِّط (تنتظر الاعتماد)
1. «تحقّق ثم أظهِر» على مسار الطبقة وحده (Hyprland وKDE وwlroots). على مسار النافذة (GNOME، وقارئ الشاشة) الخلية نافذة في المنتصف لا تحت المؤشر، فالخلية الظاهرة هي الهدف الوحيد الذي يوجّه إليه المستخدم السحب، والنافذة الخفية لا يصلها سحب. يبقى مسار النافذة كما هو: مهلة 10 ثوانٍ (S-014 AC3)، ونص الانتظار (S-014 AC10).
2. تعريفان للمختبِر: **مربوط**: سطح الطبقة موجود (`hyprctl layers` يسرده، وهذا ما يفحصه `shown()` في `repro.sh:333`). **ظاهر**: أول `wl_surface.attach` على السطح الفرعي للخلية ثم `commit`. سطر `hive surface:` يبقى سطر الموضع كما اليوم ويُطبع قبل الإظهار، لأن `geo()` في `repro.sh` و`GEO` في `lib.sh` يحتاجانه لتوجيه السحب إلى المستطيل.
3. منطق القرار في شيفرة تخضع للطفرات: آلة الوضع في `dnd.rs`، والإخفاء عند القائمة الفارغة في `crates/daemon/src/ui.rs`. ‏`layer.rs` و`receiver.rs` مستثنيان (قرار S-018 الخامس)، فينقلان الأحداث فقط.
4. يتغير عقد `State` و`Event` الذي ثبّته مختبِر S-014a: حالة جديدة (المقترح `Probing`) وحدثان (المقترح `PointerEnter` و`Reject`). الأسماء النهائية يثبّتها المختبِر.
5. المهلة: تبقى `ENTER_TIMEOUT` (10 ثوانٍ) حدّاً للماسك الذي لا يصله حدث، فلا يتغير عدد من الفحوص القائمة (السؤال 3).
6. KWin يرسل `wl_pointer.enter` عند ربط الماسك تحت مؤشر ساكن (`tests/overlay/kwin.sh:775`)، وHyprland لا يرسله إلا مع الحركة (P1). فظهور دون سحب والمؤشر داخل منطقة الماسك يُخفى في KWin فوراً، وقد يكون ذلك قبل الموضع.

مصير المعايير القديمة:
- S-014 AC3: مهلة 10 ثوانٍ دون `enter` تصير على مسار الطبقة حدّاً لماسك خفي (`hidden (cap)`)، وتبقى كما هي على مسار النافذة.
- S-014 AC10: نص الانتظار لا يُرسم أبداً على مسار الطبقة، ويبقى كما هو على مسار النافذة (`text::drop_waiting` واختباره باقيان).
- S-020 AC4: يحل محله AC5 (ب). ‏S-019 AC8 («أو من `wl_pointer` دون سحب»، و«نقرة على الماسك لا تغلق») يحل محله AC1 وAC5 في وضع الإفلات.

## التقسيم (كل قصة فرعية يوم عمل أو أقل، بالترتيب)
| القصة | المحتوى | المعايير |
| --- | --- | --- |
| S-021a | آلة الوضع والحد الزمني، والإخفاء عند القائمة الفارغة في الخدمة، واختبارات الوحدة | AC1، AC2، AC9 |
| S-021b | الرسم المؤجَّل في `layer.rs`، وأسطر السجل، وفحوص `repro.sh` و`tests/drop/kwin.sh` و`tests/overlay/kwin.sh` | AC3 إلى AC7، AC10 |
| S-021c | الميزانية، والتوثيق، وCHANGELOG، والبوابات | AC8، AC11، AC12 |

## معايير القبول
- AC1: آلة الوضع (وحدة، ساعة محقونة)، في ظهور إفلات على مسار الطبقة:
  - الظهور يعطي `Probing`. `Enter` بخلايا (عددها 1 أو أكثر) يعطي `Waiting`. `Reject` يعطي `Hidden`، وكذلك `PointerEnter`، و`Release` (في ظهور `hold`)، و`Escape` و`Cancel` (وحدة فقط: لا يصلان الماسك قبل الدخول، فلوحة المفاتيح 0).
  - `Tick` عند 9.9 ثانية يبقى `Probing`، وعند 10.1 يعطي `Hidden`.
  - `FocusLost` و`ClickOutside` لا يغيّران `Probing`.
  - على مسار النافذة يعطي الظهور `Waiting` كما اليوم، و`PointerEnter` لا أثر له.
- AC2: كل انتقال من `Probing` إلى `Hidden` يستدعي `on_dismiss` مرة واحدة بالضبط (اختبار في `drop_hive_tests.rs`). `Enter` تعيد له `on_drop_files` صفر خلايا يعطي `Hidden` ولا يضع نص `no_actions` في المركز. سطر `drop cells:` يُطبع قبل فحص القائمة الفارغة كما اليوم (S-014 AC8).
- AC3: مع `SBARHIVE_DEBUG_FOCUS`، وعلى مسار الطبقة فقط، لكل ظهور إفلات سطر واحد بالضبط من اثنين:
  - `hive reveal: N`، وN يساوي عدد المعرّفات في سطر `drop cells:` الذي يسبقه.
  - `hive probe: hidden (R)`، وR واحد من `pointer` و`no-uri-list` و`no-actions` و`release` و`cap`.
  - ظهور أُخفي قبل الموضع (قبل انقضاء `ANCHOR_WAIT`) لا يطبع `hive anchor:` ولا `hive surface:`، بل سطر الإخفاء وحده.
- AC4: في أثر `WAYLAND_DEBUG=1` لظهور إفلات على مسار الطبقة: صفر `wl_surface.attach` على السطح الفرعي للخلية قبل سطر `hive reveal:`، وصفر في الظهور كله إن أُخفي. الماسك يأخذ بكسله الشفاف الواحد كما اليوم. مناطق S-020 AC1 دون تغيير، لكن مرجعها «قبل أول إطار» يصير «قبل أول `commit` للماسك بعد أول `configure`». ونافذة `MOVE_WINDOW` (300 ms) تُحسب من الموضع (نهاية `ANCHOR_WAIT`) لا من أول إطار.
- AC5: `repro.sh`، دون سحب:
  - (أ) `drop --at` عند المؤشر والمؤشر ساكن داخل المستطيل، ثم حركة 5 px: سطر `hidden (pointer)`، وصفر `attach` على الخلية، والطبقة تُهدم خلال 500 ms من `wl_pointer.enter` في الأثر، ونقرة بعد الحركة تصل `keys.py` (نقرة واحدة).
  - (ب) المؤشر في (40، 40) خارج المستطيل: نقرة هناك تصل `keys.py` (نقرة واحدة)، ثم الحركة إلى داخل المستطيل تعطي `hidden (pointer)`.
  - (ج) `drop --hold` ثم `drop --release` بعد ظهور الطبقة والمؤشر ساكن: `hidden (release)`، وصفر `attach` على الخلية.
- AC6: سحب ملف بلا إجراء (`notes.unknownext`) في `repro.sh` وفي `tests/drop/kwin.sh`: `drop cells:` فارغ، ثم `hidden (no-actions)`، وصفر `attach` على الخلية، وصفر `wl_data_offer.accept` بنوع غير `nil` في الظهور، ولا ملف جديد في المجلد، وسحب جديد يبدأ بعده. وسحب نص بلا `text/uri-list` في `repro.sh`: `hidden (no-uri-list)`، وصفر `wl_data_offer.receive`.
- AC7: سحب PNG في `repro.sh` (حالة `cell`) وفي الظهورات الخمسة لـ `tests/drop/kwin.sh level=1`:
  - `hive reveal: N` وN يساوي عدد أسطر `sbarhive actions` للملف.
  - أول `attach` على الخلية يأتي بعد أول `wl_data_device.enter` على الماسك.
  - الإفلات على WEBP يكتب الملف كما اليوم (فحوص S-020 AC3 وS-014 AC7 خضراء).
- AC8: الميزانية (تحتاج موافقة محمد، السؤال 1): من أول `wl_data_device.enter` على الماسك إلى أول `attach` على الخلية، بطوابع أثر الخدمة، 150 ms أو أقل في كل ظهور ظاهر في `repro.sh` و`tests/drop/kwin.sh level=1`. و`tests/drop/budget.sh` يقيس من انقضاء العتبة إلى أول `commit` للماسك (وسيط 20 ظهوراً)، أقل من 150 ms.
- AC9: دورة متتالية في `repro.sh`: ظهوران دون سحب (`hidden (pointer)` ثم `hidden (release)`)، ثم `drop --hold` مع سحب PNG يعطي `hive reveal:`. و`drop --release` بعد إخفاء يخرج بـ 0 خلال 200 ms ولا يضيف سطحاً. لا خطأ بروتوكول في سجل الخدمة.
- AC10: مسار النافذة دون تغيير: `tests/drop/gnome.sh smart` و`tests/overlay/gnome.sh` تطبعان `RESULT PASS` دون تعديل، وصفر سطر `hive probe:` أو `hive reveal:` فيهما.
- AC11: `tests/hyprland/repro.sh --json` يطبع `"failed": 0` (المجموع الجديد يثبّته المختبِر)، و`tests/drop/kwin.sh level=1` و`tests/drop/layer.sh` و`tests/overlay/kwin.sh` تطبع `RESULT PASS` بالتأكيدات المعدّلة أدناه، ومعها بوابات CLAUDE.md، وصفر طفرة ناجية في `dnd.rs` و`crates/daemon/src/ui.rs`، وسطر في CHANGELOG.md بالعربية والإنكليزية.
- AC12: `integrations/linux/hyprland/README.md` بالعربية والإنكليزية: الخلية تظهر فقط أثناء سحب ملفات لها إجراء؛ الترتيب: ابدأ السحب ثم أمسك المفتاح؛ دون سحب أو لملف بلا إجراء لا يظهر شيء. و`docs/architecture.md` يصف الماسك الخفي وأسباب الإخفاء. فحص ثابت في `tests/drop/hold_docs.sh` للجملتين في اللغتين، وفحوصه القائمة ناجحة، ولا شرطة طويلة.

## حالات الاختبار
| معيار | الاختبار | النوع |
| --- | --- | --- |
| AC1، AC2 | `drop_tests.rs` و`drop_hive_tests.rs`: كل انتقال، والحد عند 9.9 و10.1 ثانية، وعدّ `on_dismiss`، والمركز بلا `no_actions` | وحدة |
| AC3، AC4 | `repro.sh` و`tests/drop/kwin.sh`: عدّ الأسطر في كل ظهور، وعدّ `attach` على السطح الفرعي (من `get_subsurface`) قبل `hive reveal:` | تكامل |
| AC5، AC9 | `repro.sh`: الحالات (أ) و(ب) و(ج) ونقرات `keys.py`، ثم الدورة المتتالية | تكامل |
| AC6 | `repro.sh` و`kwin.sh`: ملف `notes.unknownext` من `source.py`، وسحب نص (يضيف المختبِر وضعاً لـ `source.py`) | تكامل |
| AC7 | حالة `cell` والظهورات الخمسة: سطر `hive reveal:` مقابل `sbarhive actions`، وترتيب `enter` ثم `attach`، والملف الناتج | تكامل |
| AC8 | الفرق بين الطابعين في الأثر لكل ظهور ظاهر؛ و`budget.sh` و`measure.sh DROP=1` بالتعريف الجديد | أداء |
| AC10، AC11 | السكربتات كلها مرة واحدة، والبوابات، و`cargo mutants --in-diff` على الملفين | تكامل، وحدة |
| AC12 | `tests/drop/hold_docs.sh` | ثابت |

تأكيدات تتغير (يعدّلها المختبِر وحده، ويُمنع المنفِّذ من لمسها):
- `tests/hyprland/repro.sh:444-445` (S-020 AC4، «الخلية باقية حتى المهلة»): يصير AC5 (ب)، والإخفاء عند الدخول لا بعد 9000 ms.
- `repro.sh:453-454` (S-019 القرار 12، مؤشر ساكن دون `--at`): لا إطار أول في ظهور دون سحب. يصير: منطقة الماسك مستطيل الخلية خلال 400 ms من أول `configure` للماسك، وصفر `attach` على الخلية. وتعليق الرأس 76 إلى 86.
- `tests/drop/layer.sh:24-30` و`70-86` (`still_region`) و`148-153` و`206-207` و`216-217`: المرجع «أول إطار» (أول `attach` للخلية) يصير أول `configure` للماسك، كما AC4. وهذه الفحوص تجري أيضاً من `tests/overlay/kwin.sh drop`.
- `tests/overlay/kwin.sh drop` (766 إلى 848): الظهوران 1 و3 دون سحب والمؤشر في المنتصف يُخفيان عند الربط (القرار 6)، فتسقط فحوص الموضع والمنطقة فيهما. يصيران: سحب PNG ممسوك كما في `tests/drop/kwin.sh`. والنقرة في (100، 700) لـ AC13 تنتقل إلى ظهور `drop --at 640,400` والمؤشر في (100، 700) خارج المستطيل.
- `repro.sh:548` (المفتاح يُترك): يبقى، فهو فحص «مربوط».
- `tests/overlay/kwin.sh:810-811` و`821` لـ drop2 (S-019 AC8 وAC5، نقرة دون حركة لا تغلق، وسطر المرساة قبل سطر الموضع): يصير `hidden (pointer)` وهدم واحد خلال 500 ms من `wl_pointer.enter`، وصفر `attach`، ولا سطر مرساة ولا موضع إن أُخفي قبل الموضع (AC3)؛ ووصول `wl_pointer.button` سطر INFO (السؤال 2).
- `tests/overlay/kwin.sh:846-847` (AC13، باقية حتى المهلة): يبقى ما دام الحد 10 ثوانٍ، في ظهور `--at` المذكور أعلاه، ويُضاف صفر `attach` على الخلية.
- `tests/overlay/kwin.sh:1000` و`1023` (S-019 outputs، `at-v1`: `drop --at` دون سحب ولقطة): المؤشر داخل المستطيل فيُخفى ولا يُرسم شيء. يصير الظهور بسحب PNG ممسوك عند النقطة، وفحوص 1025 إلى 1029 كما هي.
- `tests/drop/kwin.sh:270-286` (S-014 AC10، حبر نص الانتظار ولقطته): يصير: صندوق الخلية في لقطة ما قبل `enter` بصفر بكسل `text-primary` و`surface-2` و`surface-3`. اللقطة `drop-en-light-waiting.png` (السؤال 4).
- `tests/drop/kwin.sh:385-390` (S-019 AC8، حالة late): المرجع «أول إطار» يصير أول `configure` للماسك، فنافذة 300 ms تُحسب من الموضع لا من الرسم. و395 إلى 398 (still) تبقى أعدادها.
- `tests/drop/lib.sh:453` (AC11، Esc قبل الدخول يغلق بالمهلة): يبقى ما دام الحد 10 ثوانٍ، ويُضاف سطر `hidden (cap)`.
- `tests/drop/budget.sh` (S-014 AC12، S-019 AC20): `first_frame_ms` دون سحب لا يوجد بعد اليوم، فيصير AC8.
- `drop_tests.rs:142` و`312` و`373` (`Show { drop: true }` يعطي `Waiting`): تبقى لمسار النافذة إن جاء التمييز بحقل جديد، وتُضاف نظائرها لمسار الطبقة.

## خارج النطاق
- X11 (S-015)، واختصار البوابة (S-014e وS-014f)، والسكربت القديم `sbarhive-alt-hold.py`.
- بدء السحب بعد انقضاء العتبة: الماسك الخفي تحت المؤشر يأخذ الضغطة فيُخفى (`pointer`). كان معطوباً قبل هذه القصة أيضاً، والترتيب الموثّق هو السحب أولاً.
- تغيير مسار النافذة (القرار 1، السؤال 5).

## أسئلة لمحمد
1. قياس 150 ms: CLAUDE.md يقيس من انقضاء `hold_ms`. الإطار الأول الآن ينتظر دخول السحب، وعلى Hyprland يأتي الدخول مع أول حركة (P1 وP3)، فاليد الساكنة لا ترى شيئاً حتى تتحرك. المقترح: من الدخول إلى أول إطار ظاهر (AC8)، ومن العتبة إلى ربط الماسك. لا يُعتمد دون موافقتك.
2. النقرة الأولى: حين يدخل المؤشر المستطيل الخفي دون سحب يُخفى الماسك عند الدخول، وقد تضيع نقرة أتت معه دون حركة سابقة. هل يُقبل ذلك؟
3. الحد: المقترح 10 ثوانٍ (`ENTER_TIMEOUT` نفسه)، فلا تتغير فحوص كثيرة. البديل حد أقصر، 3 ثوانٍ مثلاً.
4. اللقطة المعتمدة `tests/drop/snapshots/drop-en-light-waiting.png` أُخذت على مسار الطبقة في KWin، وحالتها لن تُرسم هناك بعد اليوم. تُحذف، أم تُعاد على مسار النافذة؟
5. مسار النافذة: يبقى ظاهراً بنص الانتظار ونص «لا إجراءات» بعد دخول ملف بلا إجراء (المقترح)، أم يُخفى بصمت بعد الدخول أيضاً؟

## سجل المراجعة
- 2026-10-22، المخطِّط: الجهد أكثر من يوم عمل، فقُسّمت القصة إلى ثلاث قصص فرعية في ملف واحد، كما في S-014 وS-019. القرارات الستة والأسئلة الخمسة تنتظر الاعتماد. انحرافان عن الموجز يحتاجان قراراً: مسار النافذة دون بوابة قبل الدخول (القرار 1، السؤال 5)، والحد 10 ثوانٍ لا «ثوانٍ قليلة» (السؤال 3). الحالة «مسودة» حتى موافقة محمد. الأعمال الثقيلة داخل `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200%` بـ `-j 1` أو `-j 2`، ومؤلِّف خاص واحد في كل مرة (Hyprland متداخل خاص، لا سطح المكتب الحي)، والطفرات على الملفات المتغيرة فقط.
- 2026-10-22، محمد: القصة معتمدة بالإجابات المقترحة كلها. 1: الـ 150 ms تُقاس من دخول السحب إلى أول إطار ظاهر (AC8). 2: ضياع النقرة الأولى مقبول. 3: الحد 10 ثوانٍ. 4: تُعاد اللقطة `drop-en-light-waiting.png` على مسار النافذة. 5: مسار النافذة دون تغيير. القرارات الستة معتمدة.
- 2026-10-22، المختبِر (S-021a فقط، AC1 وAC2 وAC9): كُتبت الاختبارات من المعايير وجدول الحالات وإجابات محمد، وفشلت قبل التنفيذ. شُغّلت داخل `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200%` بـ `-j 2`، دون مؤلِّف.
  - الملفات: `crates/ui/src/drop_tests.rs` (11 اختباراً جديداً: عشرة `ac1_*` و`ac9_two_hides_without_a_drag_then_a_held_drag_reveals_and_runs`) و`crates/ui/src/drop_hive_tests.rs` (6 اختبارات `*_s021a_*` على منصة Slint المخفية).
  - العقد الذي تثبّته الاختبارات (لا تُعدَّل):
    - `crate::dnd::Event::Show { drop: bool, hold: bool, layer: bool }`: ‏`layer` ظهور على مسار الطبقة (ماسك خفي). ‏`drop: true, layer: true` يعطي `Probing` مع `hold` أو دونه، و`drop: false, layer: true` يعطي `Normal`، و`layer: false` كما اليوم.
    - `crate::dnd::State::Probing`: ظهور إفلات مربوط على مسار الطبقة لم يُظهَر بعد.
    - `crate::dnd::Event::PointerEnter`: ‏`wl_pointer.enter` على الماسك، أي لا سحب. يعطي `Hidden` من `Probing`، ولا أثر له في `Waiting` و`Over` و`Normal` (مسار النافذة).
    - `crate::dnd::Event::Reject`: السحب بلا `text/uri-list` أو ملفاته بلا إجراء. يعطي `Hidden` من `Probing`.
    - `Event::Enter` يبقى بلا حقل، ومعناه في `Probing` دخول بخلية واحدة أو أكثر، فيعطي `Waiting`، ثم تسري قواعد S-014 وS-019e كما هي (لا حد بعد الدخول، و`Motion` و`Drop` و`Leave`).
    - في `Probing`: ‏`Release` يعطي `Hidden` في ظهور `hold` فقط، ويبقى `Probing` دونه. ‏`Escape` و`Cancel` يعطيان `Hidden`. ‏`FocusLost` و`ClickOutside` لا يغيّران شيئاً. ‏`Tick` عند 9.9 ثانية من الظهور يبقى `Probing`، وعند 10.1 يعطي `Hidden` (الحد `ENTER_TIMEOUT` نفسه، ويُحسب من آخر ظهور).
    - `crate::receiver::Note::PointerEnter`: ينقله `layer.rs` إلى المصرف نفسه، و`on_note` يمرّره إلى الآلة `Event::PointerEnter`.
    - في الخلية (`Inner`): حين تكون الآلة `Probing` يستدعي `Note::Enter(files)` الدالة `on_drop_files` مرة واحدة. بخلية أو أكثر تُطبَّق الخلايا ونص المركز كما اليوم وتأخذ الآلة `Enter`. بصفر خلايا تأخذ الآلة `Reject`، ولا يُطبَّق نص المركز الذي أعادته الدالة (نص `no_actions` في الخدمة)، وتُخفى الخلية. في `Waiting` (مسار النافذة) يبقى السلوك كما اليوم: نص `no_actions` في المركز والخلية باقية (السؤال 5).
    - كل انتقال من `Probing` إلى `Hidden` (‏`PointerEnter` و`Release` و`Reject` و`Escape` و`Cancel` والحد) يستدعي `on_dismiss` مرة واحدة بالضبط، وأي حدث بعده لا يضيف استدعاءً. وفقدان التركيز (`focus_changed`) في `Probing` لا يغلقها.
  - المنفذ إلى الآلة في اختبارات الخلية: المنصة المخفية مسار نافذة دائماً، فتضع الاختبارات الآلة في `Probing` بنفسها بعد `present_drop*` بـ `machine.handle(Event::Show { .., layer: true }, t)`، كما يفعل فحص المهلة القائم في `drop_hive_behaviour`. ربط `on_layer` بـ `layer: true` داخل `show()` لا يصله اختبار مخفي، وتفحصه أسطر `hive reveal:` و`hive probe:` في S-021b (AC3 وAC4).
  - مكان قرار القائمة الفارغة: القرار 3 يضعه في `crates/daemon/src/ui.rs`. العقد المُلزِم هو سلوك الخلية أعلاه، فالمنفِّذ حرّ في مكان الحساب ما دام نص `no_actions` لا يُرسم على مسار الطبقة ويبقى على مسار النافذة. سطر `drop cells:` قبل فحص القائمة الفارغة يحرسه اختبار S-014 AC8 القائم (`drop_cells_line_lists_the_ids_in_order`)، ولم يُضف له اختبار.
  - تأكيدات قائمة تغيّرت: ‏`drop_tests.rs` الأسطر 141 و255 و306 و366 قبل التعديل (القصة تسميها 142 و312 و373) أخذت `layer: false`، وتأكيداتها كما هي، وأُضيفت نظائرها لمسار الطبقة. على المنفِّذ أن يضيف الحقل نفسه إلى حرفيات `Event::Show` في وحدة `mod tests` داخل `crates/ui/src/dnd.rs` (الأسطر 133 و178 و192) وفي `show()` في `crates/ui/src/lib.rs`؛ هذا ليس تعديلاً للاختبارات بل لازم من تغيير النوع.
  - ما ينجح قبل التنفيذ بعد وجود الأسماء: ثلاثة حراس لمسار النافذة لا يُلوَون: ‏`ac1_a_normal_show_on_the_layer_path_does_not_probe` و`ac1_the_window_path_still_waits_and_ignores_pointer_enter` و`ac1_s021a_the_window_path_is_unchanged`. تحقق المختبِر في نسخة مؤقتة (حُذفت): بأسماء بلا سلوك فشل الأربعة عشر الباقون كلهم عند تأكيد `Probing` الأول؛ وبتنفيذ تجريبي صغير نجحت اختبارات `sbarhive-ui --lib` كلها (140)، فالعقد قابل للتنفيذ.
  - AC9: الجزء الذي في الوحدة هنا (ظهوران يُخفيان دون سحب، `pointer` ثم `release`، ثم `drop --hold` بسحب PNG يُظهِر ويُشغِّل، وترك المفتاح بعد الإخفاء لا يضيف شيئاً، وكل ظهور يعدّ حدّه من ظهوره). الجزء التكاملي (خروج `drop --release` بـ 0 خلال 200 ms، ولا سطح جديد، ولا خطأ بروتوكول، وسطر `hive reveal:`) يحتاج أسطر S-021b ومؤلِّفاً خاصاً، فينتقل إلى فحوص `repro.sh` في S-021b. لم يُعدَّل `repro.sh` ولم يُشغَّل.
  - مخرجات الفشل:
    ```
    $ systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% cargo test -p sbarhive-ui --lib -j 2
    error[E0559]: variant `dnd::Event::Show` has no field named `layer`
       --> crates/ui/src/drop_hive_tests.rs:447:13
    error[E0599]: no variant, associated function, or constant named `Probing` found for enum `dnd::State` in the current scope
       --> crates/ui/src/drop_hive_tests.rs:451:31
    error[E0599]: no variant, associated function, or constant named `PointerEnter` found for enum `Note` in the current scope
       --> crates/ui/src/drop_hive_tests.rs:502:23
    error[E0599]: no variant, associated function, or constant named `Reject` found for enum `dnd::Event` in the current scope
       --> crates/ui/src/drop_hive_tests.rs:537:33
    error[E0599]: no variant, associated function, or constant named `PointerEnter` found for enum `dnd::Event` in the current scope
       --> crates/ui/src/drop_tests.rs:511:36
    error: could not compile `sbarhive-ui` (lib test) due to 35 previous errors
    ```
    الأخطاء الخمسة والثلاثون: 13 `Probing`، و8 الحقل `layer`، و6 `Note::PointerEnter`، و6 `Event::PointerEnter`، و2 `Reject`. كلها بنود من العقد، ولا خطأ غيرها.
  - ملاحظة: كما في S-014a، لا تُبنى اختبارات وحدة `sbarhive-ui` كلها حتى تُضاف هذه البنود.
- 2026-10-22، محمد: يُسمح للمنفِّذ بإضافة `layer: false` إلى حرفيات `Event::Show` في `mod tests` داخل `crates/ui/src/dnd.rs` (الأسطر 133 و178 و192)، تعديلاً آلياً فقط لا يغيّر أي تأكيد. ونصف AC9 الخاص بـ `repro.sh` (خروج `drop --release` بـ 0 خلال 200 ms، ولا سطح جديد، ولا خطأ بروتوكول، وسطر `hive reveal:`) ينتقل إلى فحوص `repro.sh` في S-021b.
- 2026-10-22، المنفِّذ (S-021a): نجحت الاختبارات دون تعديل أي تأكيد. الوحيد المسموح: `layer: false` في حرفيات `Event::Show` الثلاث في `mod tests` داخل `dnd.rs`.
  - `dnd.rs`: الحالة `Probing`، والحدثان `PointerEnter` و`Reject`، والحقل `layer` في `Show`. ‏`drop` مع `layer` يعطي `Probing`؛ ومنه `Enter` يعطي `Waiting`، و`PointerEnter` و`Reject` و`Escape` و`Cancel` و`Release` (في `hold`) والحد `ENTER_TIMEOUT` تعطي `Hidden`، والباقي لا يغيّرها.
  - `lib.rs`: ‏`show()` يمرّر `layer: inner.on_layer.get()`. ‏`drop_files` يقبل `Probing` ويعيد حدث الآلة: بصفر خلايا في `Probing` يعيد `Reject` دون تطبيق النص ولا الخلايا، وفي `Waiting` كما اليوم. ‏`on_note` يربط `Note::PointerEnter` بـ `Event::PointerEnter`. ‏`focus_lost` لا يغلق في `Probing`.
  - `receiver.rs`: البند `Note::PointerEnter` فقط. ‏`layer.rs`: ‏`wl_pointer.enter` على الماسك يضيف `Ev::PointerEnter`، والمحوّل في وضع الإفلات ينقله إلى `on_note` (المصرف نفسه) في الدورة التالية عبر `invoke_from_event_loop` كما يفعل المستقبِل.
  - قرار القائمة الفارغة في الخلية (`Inner::drop_files`) لا في `crates/daemon/src/ui.rs`، كما يسمح عقد المختبِر؛ سطر `drop cells:` يبقى قبله في الخدمة.
  - مؤقتاً حتى S-021b: على مسار الطبقة تُرسم الخلية فارغة عند الربط كما اليوم، وتُخفى عند `wl_pointer.enter` أو ملف بلا إجراء. فحوص `repro.sh` و`kwin.sh` القديمة التي تنتظر بقاء الخلية حتى المهلة دون سحب ستفشل حتى يعدّلها المختبِر في S-021b. لم يُشغَّل أي مؤلِّف.
  - البوابات داخل `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200%` بـ `-j 2`: ‏`cargo fmt --check` نظيف، و`cargo clippy --all-targets -- -D warnings` نظيف، و`cargo test --all` ‏563 ناجحاً و0 فاشلاً (‏`sbarhive-ui --lib` ‏140)، و`scripts/design-lint.sh` نظيف.
