# S-021 خلية الإفلات لا تظهر إلا لسحب حقيقي لملفات لها إجراء
الحالة: معتمدة (2026-10-22)؛ S-021a منتهية (المراجعة 1: PASS)، وS-021b منتهية (المراجعة 1: PASS)، وS-021c في المراجعة (المراجعة 1: REWORK، توثيق فقط؛ إعادة العمل 1 تنتظر المراجعة 2)

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
- 2026-10-22، المراجِع (S-021a، المراجعة 1): **PASS**.
  - النطاق: `0f57386..HEAD` (‏3cdba9e و8ab5d2f، و79b6435 سجل القصة فقط). كل شيء داخل `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200%`، دون أي مؤلِّف.
  - البوابات: `cargo fmt --check` خرج بـ 0؛ `cargo clippy --all-targets -- -D warnings` (‏`-j 2`) خرج بـ 0 بلا تحذير؛ `cargo test --all` (‏`-j 2`) ‏563 ناجحاً و0 فاشلاً و1 متجاهَلاً (‏`sbarhive-ui --lib` ‏140)؛ `scripts/design-lint.sh` نظيف.
  - الطفرات (‏cargo-mutants 27.1.0، `--in-place` كما في القصص السابقة، تسلسلياً بـ `CARGO_BUILD_JOBS=1` لأن `--in-place` لا يقبل `-j`): `--file crates/ui/src/dnd.rs`: ‏22 طفرة، 20 مقتولة، 0 ناجية، 0 مهلة، 2 غير قابلة للبناء (`Machine::state` و`Machine::handle` بـ `Default::default()`، فلا `Default` لـ `State`). و`--in-diff` على فرق `lib.rs`: ‏8 طفرات (`on_note` و`Inner::drop_files` و`Inner::focus_lost` و`Hive::show`)، 7 مقتولة، 0 ناجية، 0 مهلة، 1 غير قابلة للبناء (`drop_files` بـ `Default::default()`). ‏`layer.rs` و`receiver.rs` مستثنيان في `.cargo/mutants.toml` (قرار S-018 الخامس، والقرار 3 هنا). البدائل `Probing |` في `matches!` داخل `drop_files` و`focus_lost` وأذرع `Probing` في `Machine::handle` لا تطفّرها الأداة، وتغطيها: `ac2_s021a_enter_without_cells_hides_once_without_the_no_actions_text` (يُسأل `on_drop_files` مرة في `Probing`)، و`ac1_s021a_focus_loss_and_click_outside_keep_a_probing_show`، واختبارات `ac1_*` في `drop_tests.rs` لكل ذراع. شجرة العمل نظيفة بعد التشغيل.
  - سلامة الاختبارات: `git diff 0f57386..HEAD` لا يمس `drop_tests.rs` ولا `drop_hive_tests.rs` ولا أي ملف تحت `tests/`. التغيير الوحيد في شيفرة اختبار: `layer: false` في حرفيات `Event::Show` الثلاث في `mod tests` داخل `dnd.rs`، دون تغيير أي تأكيد، كما سمح محمد.
  - المعايير: AC1 تغطيه اختبارات `ac1_*` العشرة في `drop_tests.rs` و`ac1_ac2_s021a_*` و`ac1_s021a_*` في `drop_hive_tests.rs`. AC2 تغطيه `ac2_s021a_*` (عدّ `on_dismiss` لكل انتقال، والمركز بلا `no_actions`)، وترتيب سطر `drop cells:` يحرسه `drop_cells_line_lists_the_ids_in_order` (`crates/daemon/src/ui.rs:330`، نجح في التشغيل). نصف AC9 الخاص بالوحدة يغطيه `ac9_two_hides_without_a_drag_then_a_held_drag_reveals_and_runs` و`ac9_s021a_two_hides_without_a_drag_then_a_held_png_drag_reveals`. بنود العقد كلها متحققة في الشيفرة: `Show { drop, hold, layer }` و`Probing` و`PointerEnter` و`Reject` و`Note::PointerEnter`، و`show()` يمرّر `inner.on_layer.get()` بعد إعادة البناء إن حصلت، و`focus_lost` لا يغلق في `Probing`. مسار النافذة دون تغيير: `layer: false` يعطي `Waiting` كما كان، و`drop_files` في `Waiting` يطبّق النص والخلايا كما كان، و`PointerEnter` لا أثر له خارج `Probing`.
  - النطاق: لا رسم مؤجَّل، ولا سطر `hive reveal:` ولا `hive probe:` في الشيفرة، ولا تعديل في `tests/` (ومنها `repro.sh`) ولا `docs/` ولا `video/` ولا `crates/daemon/`. ‏`layer.rs` ينقل `wl_pointer.enter` على الماسك في وضع الإفلات فقط، و`receiver.rs` بند واحد.
  - لا تنطبق على S-021a: اللقطات والميزانية ونصوص الواجهة (لا نص جديد ولا عنصر تفاعلي جديد)، وهي لـ S-021b وS-021c. لم يُضف سطر CHANGELOG.md لأنه من AC11 في S-021c.
  - لا شرطة طويلة في أي ملف متغير. ملاحظة غير حاجبة: سطر التوثيق `crates/ui/src/lib.rs:681` (‏`key_released`) صار 86 عموداً بعد إضافة «probing,» دون إعادة لف، وما حوله يلتف عند 80؛ في المستودع 35 سطر تعليق من 1548 تتجاوز 80، فهو داخل الأسلوب القائم. يمكن لفّه في S-021b.
  - ملاحظة إجرائية: أول تشغيل للطفرات بوضع النسخ بدأ ينسخ `target/` (‏76G) إلى `/tmp` (‏tmpfs) فأُوقف عند 6.3G وحُذفت النسخة، ثم أُعيد التشغيل بـ `--in-place`.
- 2026-10-22، المختبِر (S-021b: AC3 إلى AC7 وAC10، ونصف AC9 الخاص بـ `repro.sh`): كُتبت الفحوص من المعايير وجدول الحالات وقائمة «تأكيدات تتغير»، دون قراءة شيفرة الميزة. شُغّلت على بناء main ‏(aa84616، `cargo build --release` جديد) داخل `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200%`، كل مؤلِّف خاص وحده: Hyprland 0.56.2 متداخل، وKWin ‏`--virtual`، وgnome-shell ‏`--headless`، وكلها بـ HOME مؤقت.
  - الملفات: `tests/drop/layer.sh` (‏`catcher_ready` و`probe_show` و`probe_checks`، والمرجع الجديد في `still_region` و`layer_drop_show`)، و`tests/hyprland/repro.sh`، و`tests/drop/kwin.sh`، و`tests/drop/lib.sh` (‏`shortcut_esc` قبل الدخول)، و`tests/overlay/kwin.sh` (أوضاع `drop` و`outputs` و`layer`)، و`tests/drop/source.py` (وضع `text:`)، و`tests/drop/window_path.sh` (جديد، AC10)، وحذف `tests/drop/snapshots/drop-en-light-waiting.png` (السؤال 4).
  - العقد الذي تثبّته الفحوص (لا تُعدَّل):
    - مع `SBARHIVE_DEBUG_FOCUS` تطبع الخدمة على stderr، في المجرى نفسه الذي يحمل أثر `WAYLAND_DEBUG`، سطراً واحداً بالضبط لكل ظهور إفلات على مسار الطبقة: `hive reveal: N` (التعبير `^hive reveal: [0-9]+$`) أو `hive probe: hidden (R)` (التعبير `^hive probe: hidden \((pointer|no-uri-list|no-actions|release|cap)\)$`). لا سطر غيرهما يبدأ بـ `hive reveal:` أو `hive probe:`. إغلاق ظهور ظاهر (إفلات، Esc، مغادرة، ترك المفتاح) لا يطبع شيئاً. الضغطة القصيرة التي لا تربط سطحاً لا تطبع شيئاً. مسار النافذة لا يطبع أياً منهما.
    - السطران بلا طابع زمني، فـ«قبل» تعني ترتيب الأسطر في ذلك المجرى. ‏N يساوي عدد المعرّفات في آخر سطر `drop cells:` قبله. سطر `drop cells:` الفارغ (‏`drop cells:` بلا شيء بعد النقطتين) يسبق `hidden (no-actions)`.
    - «ظاهر» = أول `-> wl_surface#C.attach(wl_buffer` على السطح الفرعي للخلية، وC هو السطح الأول في `get_subsurface(new id wl_subsurface#S, wl_surface#C, ...)`. سطر `hive reveal:` يأتي قبل أول `attach` كهذا، ويليه واحد على الأقل، وأولها بعد أول `wl_data_device.enter` يسمّي الماسك. في الظهور المخفي صفر `attach` على C.
    - الرسم المؤجَّل يؤجّل مخازن الخلية وحدها: الماسك وبكسله الشفاف (‏`attach` على الماسك مرة على الأقل)، و`get_subsurface`، و`set_position`، و`set_destination`، ومنطقتا الإدخال تُصنع عند الربط كما اليوم. لذلك يبقى فحص S-020 AC1 (منطقة فارغة للسطح الفرعي) وفحوص AC11 في `tests/overlay/kwin.sh` (الموضع والمقصد) على ظهور لا يُظهَر أبداً، ويُطلب فيه صفر مخزن `wl_shm` غير بكسل الماسك.
    - ظهور أُخفي قبل الموضع لا يطبع `hive anchor:` ولا `hive surface:`. والقاعدة المفحوصة: إن جاء أول `wl_pointer.enter` على الماسك قبل أي سطر `hive anchor:` فلا سطر مرساة ولا موضع في الظهور كله؛ وفي كل ظهور مخفي لا سطر مرساة ولا موضع بعد سطر الإخفاء.
    - «جاهزية الماسك» (`catcher_ready`) = أول `commit` على الماسك بعد أول حدث `configure` لـ `zwlr_layer_surface_v1` الخاص به (الـ `commit` قبله بلا مخزن). هو مرجع الـ 400 ms لمنطقة الماسك في القرار 12 (‏`still_region` ووضع `0` في `layer_drop_show`)، ومرجع نافذة الـ 300 ms في حالة late في `tests/drop/kwin.sh`، بدل أول `attach` للخلية.
    - «الطبقة تُهدم» = `-> zwlr_layer_surface_v1#N.destroy(`. ‏«خلال 500 ms من `wl_pointer.enter`» بطوابع الأثر.
    - `source.py`: سطر `text:WORDS` في ملف NEXT يسحب النص WORDS (عروض `text/plain` بلا `text/uri-list`).
  - تأكيدات قائمة تغيّرت (كما في القائمة أعلاه): `repro.sh` (S-020 AC4 صار AC5 (ب)، وفحص القرار 12 على الظهور نفسه بالمرجع الجديد ودون `set_position`، وتعليق الرأس)، و`layer.sh` (المرجع)، و`tests/overlay/kwin.sh drop` (‏drop1 وdrop3 سحب PNG، وdrop2 ‏`hidden (pointer)`، والنقرة في (100، 700) في drop4 ‏`--at 640,400`)، و`outputs at-v1` (سحب PNG ممسوك وحركة 1 px، ولقطة «قبل» مع السحب)، و`tests/drop/kwin.sh` (لقطة ما قبل `enter` بصفر بكسل من الرموز الثلاثة مع حارس من لقطة المصدر وحده، والمرجع في late)، و`lib.sh` (‏`hidden (cap)` على مسار الطبقة فقط، ‏`ESC_NO_KEYBOARD=1`، فلا يتغير `gnome.sh`).
  - تغييرات إعداد خارج القائمة، تحتاج إقرار محمد (القرار 6 يكسرها بعد التنفيذ الصحيح): (1) ‏`tests/drop/kwin.sh hold_cycles` حالة held: المؤشر في (640، 400) فيُخفي KWin الماسك عند الربط قبل الترك، فصارت `drop --hold --at 640,400` والمؤشر في (100، 700)، والتأكيدات كما هي، مع `hidden (release)`. (2) ‏`tests/overlay/kwin.sh` الوضع الافتراضي، ظهور drop: صار `drop --at 640,400` والمؤشر في (100، 700)، وفحص AC18 «رسم في مخازن wl_shm» صار لهذا الظهور «صفر مخزن wl_shm وبكسل الماسك وحده». الإعداد نفسه الذي اعتمدته القصة لسطري 846 و847.
  - اللقطة (السؤال 4): حُذف المرجع القديم (أُخذ على مسار الطبقة، ويختلف عن مرشّح مسار النافذة في 0.59 % من بكسلاته). `tests/drop/kwin.sh` يعيد تشغيل الخدمة بـ `SBARHIVE_SURFACE=window` بعد `hold_cycles`، ويلتقط الخلية المنتظرة تحت سحب PNG ساكن، ويقصّها بمستطيل سطر `hive surface: window` (‏393 x 362)، ويفحص نص الانتظار (حبر `text-primary` في المركز) وغياب الخلايا وغياب أسطر S-021، ثم يقارنها بالمرجع. المرشّح في `tests/drop/out/drop-en-light-waiting.png` بعد التشغيل، وفحص المرجع يفشل حتى يعتمده محمد. لم يعتمده المختبِر.
  - ما ينجح قبل التنفيذ ولماذا (`repro.sh` ‏18 من 39): حراس سلوك قائم أو سلوك S-021a لا يُلوَون: نقرات `keys.py` والإخفاء خلال 500 ms (‏AC5 (أ) و(ب) و(ج) وAC9 (1)، الآلة من S-021a)، ومنطقة القرار 12 بالمرجع الجديد (‏350.8 ms، والحد 400)، وغياب `receive` في سحب النص، وكتابة WEBP، والنقرة في (100، 700)، وفحوص مسار النافذة، وAC10 كله. كل فحص S-021 لأسطر السجل أو المخازن يفشل.
  - ملاحظة لمحمد (خلل خارج المعايير، ظهر في تشغيل أول): ترك المفتاح (`drop --release`) والسحب فوق خلية ظاهرة، ثم سحب جديد: دخول الظهور التالي لا يُقرأ (لا `drop cells:` ولا `accept`، والخدمة تهدم عرض الظهور السابق `wl_data_offer#4278190080` عند `enter` العرض الجديد)، فلا يُكتب WEBP. أثره في `hypr-layer-daemon.log` من أول تشغيل. تجنّبه AC9 (3) بمغادرة الخلية قبل الترك، فهو ليس من AC9.
  - مخرجات الفشل (بناء main):
    ```
    $ tests/hyprland/repro.sh --json
    {"total": 39, "failed": 21, "skipped": 0}
    $ tests/hyprland/repro.sh --json --quasimode
    {"total": 46, "failed": 21, "skipped": 0}
    FAIL S-021 AC3 AC5 (b) pointer into the rectangle: exactly one probe line, "hive probe: hidden (pointer)" ...
    FAIL S-021 AC4 AC5 (b) pointer into the rectangle: no attach on the hive subsurface in the whole show, and the catcher took its pixel
    FAIL S-021 AC3 AC9 (3) PNG drag, drop --hold: exactly one probe line, "hive reveal: N" ...
    FAIL S-021 AC7 AC7 cell drag: the hive's first attach comes after the first wl_data_device.enter on the catcher
    FAIL S-021 AC7 cell drag: "hive reveal: N" with N the number of `sbarhive actions` lines for the PNG (15)
    FAIL S-021 AC6 notes.unknownext: an empty "drop cells:" line before the hide, ...
    FAIL S-021 AC3 AC6 text drag: exactly one probe line, "hive probe: hidden (no-uri-list)" ...
    $ tests/drop/kwin.sh level=1
    RESULT FAIL (28)   (310 PASS)
    FAIL S-021 AC4 show 1: before the drag enters the hive is not drawn (no text-primary, surface-2 or surface-3 pixel in its box)
      (INFO: waiting shot text-primary 46, surface-2 24, surface-3 3896; source shot before show 1: 0, 0, 0)
    FAIL S-021 AC3/AC4/AC7 show 1 to 5 (4 each), AC3 shows 6 to 13 (1), AC6 notes.unknownext (3), S-019e held released (2)
    FAIL S-014 AC10 window path: no approved reference .../drop-en-light-waiting.png (... needs the owner's approval)
    $ tests/overlay/kwin.sh drop
    RESULT FAIL (11)   (38 PASS)
    FAIL S-021 AC3 drop2: hidden before the position, so no "hive anchor" and no "hive surface" line
      (INFO: first wl_pointer.enter at line 27, anchor lines before it 0, then "hive anchor: 640 400")
    $ tests/overlay/kwin.sh outputs
    RESULT FAIL (3)   (12 PASS: the at-v1 box, anchor and placement guards)
    $ tests/overlay/kwin.sh
    FAIL S-021 AC4 show 3 (drop): never revealed, so no wl_shm buffer, only the catcher's single pixel buffer
    RESULT FAIL (1)
    $ tests/drop/kwin.sh level=1 shortcut
    INFO AC11 esc before: ... hive surface destroyed 9892.1 ms after Esc
    FAIL S-021 AC11 esc before (layer path): the timeout prints exactly one probe line, "hive probe: hidden (cap)"
    RESULT FAIL (1)
    $ tests/drop/window_path.sh
    RESULT PASS   (gnome.sh smart وoverlay/gnome.sh: RESULT PASS، وصفر سطر probe أو reveal)
    ```
  - في `tests/drop/kwin.sh shortcut` هُدم ماسك «Esc قبل الدخول» بعد 9892 ms من Esc، أي بالحد لا بدخول المؤشر، فالسبب المنتظر `cap` كما توقعت القصة. فحوص S-021 في `repro.sh` تجري في وضع الطبقة فقط (القرار 1)، فوضع `window` فيه 11 فحصاً (18 مع `--quasimode`). المرشّح بعد التشغيل الأخير 393 x 362 بمستطيل النافذة.
  - هامش حد الـ 400 ms في القرار 12 بالمرجع الجديد: Hyprland ‏350.8 ms، وKWin (ظهورات still السبعة) 350.1 إلى 356.6 ms، أي نحو 45 ms. المرجع الجديد أبكر بنحو 50 ms (‏ANCHOR_WAIT)، والحد 400 من القصة فلم يُغيَّر.
  - ليس من S-021b: AC8 (الميزانية) وتوثيق `tests/drop/README.md` الذي ما زال يذكر مرجع الانتظار على مسار الطبقة، لـ S-021c.
- 2026-10-22، المدير بتفويض محمد (طلب ألا يُسأل وأن يُكمل العمل): تُعتمد قرارات المختبِر في S-021b. 1: تغييرا الإعداد خارج القائمة (`tests/drop/kwin.sh` حالة held، وظهور الإفلات في الوضع الافتراضي في `tests/overlay/kwin.sh`) بإعداد `--at 640,400` والمؤشر في (100، 700). 2: التأجيل يخص مخازن الخلية وحدها، والماسك وبكسله ومنطقتا الإدخال تُصنع عند الربط. 3: علّة `drop --release` فوق خلية مرسومة ثم سحب جديد لا يُقرأ تصير قصة مستقلة S-022 بعد S-021c. 4: اللقطة المرشّحة لمسار النافذة تُعتمد بعد التنفيذ بعد فحصها بالعين. 6: يبقى حد 400 ms.
- 2026-10-22، المنفِّذ (S-021b): نجحت الفحوص دون تعديل أي اختبار أو سكربت اختبار. شُغّل كل شيء داخل `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200%` بـ `CARGO_BUILD_JOBS=2`، وكل مؤلِّف خاص وحده.
  - `layer.rs`: ظهور الإفلات يبدأ غير مكشوف (`revealed`). ‏`render()` قبل الكشف يرسل `set_destination` للخلية ويلتزم الماسك ويطبع سطر `hive surface:` مرة واحدة، دون مخزن للخلية ولا تفعيل ولا `frame_presented`. الماسك وبكسله و`get_subsurface` و`set_position` ومنطقتا الإدخال كما كانت عند الربط. الدالة `probed(from, to, event, cells)` تطبع السطر الوحيد حين تخرج الآلة من `Probing`: ‏`hive reveal: N` ثم الكشف و`request_redraw()`، أو `hive probe: hidden (R)` قبل هدم الماسك (`PointerEnter` ثم `pointer`، و`Reject` ثم `no-actions` أو `no-uri-list`، و`Release` ثم `release`، و`Tick` ثم `cap`). ‏Esc وCancel لا يصلان الماسك قبل الدخول فلا سطر لهما.
  - AC3، الإخفاء قبل الموضع: `wl_pointer.enter` على الماسك يُدفع قبل `Position`، وفي ظهور إفلات غير مكشوف يضبط `no_drag` ويوقف المؤقت، فلا يضع `decide()` الخلية بعده، ولا يُطبع `hive anchor:` ولا `hive surface:`. تعليقات `MOVE_WINDOW` تقول الآن «من الموضع» (المؤقت كان يبدأ عنده أصلاً).
  - `lib.rs`: ‏`drag()` يقرأ الحالة قبل الحدث ويستدعي `layer::probed` قبل أن يتصرف بالنتيجة، و`drop_files` يحفظ عدد الخلايا التي أعادتها `on_drop_files` (‏N، لا خلايا الخطة المحدودة بـ 18). ولُفّ تعليق `key_released` عند 80 عموداً.
  - `receiver.rs` (نقل فقط): `offered_uri_list()` تقول إن كان آخر دخول يعرض `text/uri-list`، فيميّز `no-uri-list` من `no-actions`. وإصلاح لازم لـ AC6: بعد إخفاء `no-actions` تحت السحب يُحرَّر جهاز البيانات دون `leave`، فبقي `on_us` صحيحاً وأُسقط دخول سحب النص التالي (سطر `hidden (pointer)` بدل `no-uri-list`، أول تشغيل لـ `repro.sh`: 1 فاشل من 39). صار المستقبِل يحفظ الجهاز الذي دخل عليه السحب ويبدأ من جديد عند عرض على جهاز آخر. هذا على الأرجح سبب علّة S-022 أيضاً (الآلية نفسها)، ولم تُكتب لها فحوص، فتبقى S-022 للتحقق.
  - القرار 3: قرار الإخفاء أو الكشف تتخذه آلة `dnd.rs` وحدها. ‏`layer.rs` يسمّي الحدث الذي سبّبه، ويرسم، ويكفّ عن وضع ظهور ستُخفيه الآلة.
  - لم يُمسّ `dnd.rs` ولا `crates/daemon/src/ui.rs`. مسار النافذة: المستقبِل مشترك، فإصلاحه يمسّ شيفرة مسار النافذة، ودليل ثبات سلوكه `tests/drop/window_path.sh` (RESULT PASS، وصفر سطر probe أو reveal).
  - النتائج (بناء `cargo build --release` جديد): `cargo fmt --check` نظيف؛ `cargo clippy --all-targets -- -D warnings` نظيف؛ `cargo test --all` ‏563 ناجحاً و0 فاشلاً و1 متجاهَلاً؛ `scripts/design-lint.sh` نظيف. ‏`tests/hyprland/repro.sh --json` ‏`{"total": 39, "failed": 0, "skipped": 0}`، و`--quasimode` ‏`{"total": 46, "failed": 0, "skipped": 0}`، وصفر خطأ بروتوكول في سجلي الخدمة. ‏`tests/drop/kwin.sh level=1` ‏RESULT FAIL (1) بـ 337 PASS، والفاشل الوحيد مرجع مسار النافذة `drop-en-light-waiting.png` الذي ينتظر اعتماد محمد، والمرشّح في `tests/drop/out/drop-en-light-waiting.png`. ‏`tests/drop/kwin.sh level=1 shortcut` ‏RESULT PASS (44). ‏`tests/overlay/kwin.sh drop` ‏RESULT PASS (49)، و`outputs` ‏RESULT PASS (15)، والوضع الافتراضي RESULT PASS (104). ‏`tests/drop/window_path.sh` ‏RESULT PASS (4). ‏`tests/drop/layer.sh` لا يعمل وحده (يُستورد من السكربتين)، وفحوصه جرت ضمنهما.
  - ليس من S-021b: AC8 والتوثيق وCHANGELOG والطفرات (S-021c). بند مفتوح لمحمد: الخلية تظهر عند الكشف دون حركة الفتح، لأن الحركة تجري عند الربط والخلية خفية. لم يُغيَّر حتى لا تتأثر فحوص البكسل في `kwin.sh`.
- 2026-10-22، المدير بتفويض محمد: اللقطة `tests/drop/snapshots/drop-en-light-waiting.png` لمسار النافذة معتمدة بعد فحصها بالعين (393 × 362، الخلية المركزية بنص «Drop files here»، ألوان الرموز، بلا ظل). وإصلاح `receiver.rs` خارج الموجز (جهاز البيانات الذي دخله السحب) مقبول، وتبقى S-022 لاختبار علّة `drop --release`. وغياب حركة الظهور عند الكشف يُسجَّل في S-022 لينظر فيه.
- 2026-10-22، المراجِع (S-021b، المراجعة 1): **PASS**.
  - النطاق: `bc697f6..HEAD` (‏2b37d6d التنفيذ، و6e16cb7 وa6d8be1 سجل القصة واللقطة المعتمدة فقط). كل شيء داخل `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200%`، وكل مؤلِّف خاص وحده، لا سطح المكتب الحي.
  - سلامة الاختبارات: `git diff --name-only bc697f6..HEAD` لا يسرد إلا `layer.rs` و`lib.rs` و`receiver.rs` وملف القصة و`tests/drop/snapshots/drop-en-light-waiting.png` (المستثناة، اعتمدها المدير بتفويض محمد في a6d8be1، لا محمد مباشرة). لا سكربت ولا اختبار وحدة تغيّر.
  - البوابات: `cargo fmt --check` خرج بـ 0؛ `cargo clippy --all-targets -- -D warnings` (‏`-j 2`) خرج بـ 0؛ `cargo test --all` (‏`-j 2`) ‏563 ناجحاً و0 فاشلاً و1 متجاهَلاً؛ `scripts/design-lint.sh` نظيف.
  - الطفرات (‏cargo-mutants 27.1.0، `--in-place`، تسلسلياً بـ `CARGO_BUILD_JOBS=1`، `--in-diff` على فرق `lib.rs` وحده): طفرتان، 1 مقتولة (`Inner::drag` بـ `()`)، 0 ناجية، 0 مهلة، 1 غير قابلة للبناء (`Inner::drop_files` بـ `Default::default()`). العدد صغير لأن تغيير `lib.rs` أربعة أسطر (الحقل `found` وقراءة الحالة قبل الحدث واستدعاء `layer::probed`)، وربط الحدث بالسبب داخل `layer.rs` المستثنى في `.cargo/mutants.toml`. شجرة العمل نظيفة بعد التشغيل.
  - السكربتات بعد `cargo build --release` جديد: `tests/hyprland/repro.sh --json` ‏`{"total": 39, "failed": 0, "skipped": 0}`؛ و`--quasimode` ‏`{"total": 46, "failed": 0, "skipped": 0}`؛ `tests/drop/kwin.sh level=1` ‏RESULT PASS (338 PASS، 0 FAIL، واللقطة تختلف عن المرجع في 0.000000 من بكسلاتها)؛ `tests/drop/kwin.sh level=1 shortcut` ‏RESULT PASS (44)؛ `tests/overlay/kwin.sh drop` ‏RESULT PASS (49)؛ `outputs` ‏RESULT PASS (15)؛ الوضع الافتراضي RESULT PASS (104)؛ `tests/drop/window_path.sh` ‏RESULT PASS (4). وفحوص «لا خطأ بروتوكول في سجل الخدمة» كلها ناجحة، ومنها S-021 AC9 بعد الدورة.
  - المعايير: AC3 وAC4 وAC7 في `repro.sh` و`tests/drop/kwin.sh` و`layer.sh` و`tests/overlay/kwin.sh`؛ AC5 وAC9 في `repro.sh`؛ AC6 في `repro.sh` و`tests/drop/kwin.sh`؛ AC10 في `window_path.sh` و`tests/drop/kwin.sh`. كل معيار من النطاق له فحص واحد على الأقل، وكلها نجحت.
  - العقد في الشيفرة لا بالمصادفة: (1) سطر واحد لكل ظهور: `layer::probed` (`layer.rs:164`) يطبع فقط حين تخرج الآلة من `Probing` في `Inner::drag` (`lib.rs:509-513`)، وحالة `Probing` لا تكون إلا على مسار الطبقة. (2) صفر مخزن قبل الكشف: `render()` (`layer.rs:1108-1118`) دون `revealed` يرسل `set_destination` ويلتزم الماسك فقط، و`revealed` لا يصير صحيحاً إلا في `probed` عند `Waiting` أو خارج وضع الإفلات (`layer.rs:908`). (3) الماسك ومنطقتاه و`get_subsurface` عند الربط كما كانت. (4) سطر `hive surface:` لا يُطبع لظهور أُخفي قبل الموضع: `render()` محروس بـ `decided`، و`decide()` يعود عند `no_drag` (`layer.rs:641`)، و`wl_pointer.enter` يُدفع الآن قبل `Position` (`layer.rs:1478-1479`) ويوقف مؤقت `ANCHOR_WAIT`. (5) القرار 3: `no_drag` يمنع الوضع فقط، ولا يُخفي ولا يكشف ولا يستدعي `on_dismiss`؛ الإخفاء والكشف من نتيجة آلة `dnd.rs` وحدها. (6) مسار النافذة: الآلة لا تدخل `Probing` فيه فلا سطر، و`window_path.sh` يثبت صفر سطر probe أو reveal.
  - إصلاح `receiver.rs` (جهاز البيانات الذي دخله السحب): صحيح وصغير، حقل واحد يُضبط عند `enter` ويُمحى في `clear()` ويُقارن عند `data_offer` (`receiver.rs:475`). على جهاز جديد يمحو عرض السحب القديم الذي لن يأتيه `leave`، ثم يأخذ العرض الجديد؛ وعلى الجهاز نفسه السلوك كما كان، فمسار النافذة (جهاز واحد لكل ظهور) لا يتغير، ويؤكده `window_path.sh` و`tests/drop/kwin.sh`.
  - ملاحظات غير حاجبة: (أ) `lib.rs:714-717`: ظهور إفلات جديد فوق ظهور في `Probing` يمر بـ `inner.close()` فتُعاد الآلة دون سطر، فالظهور المستبدَل لا يطبع شيئاً. لا يصله مسار الضغط المطوّل (الترك ينهي الفحص أولاً) ولا أي فحص، بل `drop --at` ثانٍ خلال الفحص فقط؛ يمكن أن تنظر فيه S-022. (ب) `URI_LIST_OFFERED` (`receiver.rs:94`) لا يُعاد لكل ظهور، وهو آمن لأن `Reject` يأتي دائماً بعد ملاحظة `Enter` التي تضبطه (`receiver.rs:318`). (ج) `release()` (`receiver.rs:174`) لا يحرّر جهازاً من الإصدار 1، فقد يصل منه `leave` يمحو حالة الجهاز الجديد؛ سلوك قائم قبل هذه القصة، وKWin وHyprland يقدّمان الإصدار 3.
  - اللقطة: المرجع والمرشّح الجديد `tests/drop/out/drop-en-light-waiting.png` متطابقان بالعين (393 × 362، الخلية المركزية «Drop files here»، بلا ظل ولا تدرج)، وبالفحص 0 % فرق.
  - لا نص واجهة جديد ولا عنصر تفاعلي جديد (أسطر السجل للتشخيص فقط). الأسلوب يطابق ما حوله، ولا شرطة طويلة في الملفات المتغيرة. لم يُضف سطر CHANGELOG.md لأنه من AC11 في S-021c، والميزانية AC8 والتوثيق كذلك. غياب حركة الفتح عند الكشف مسجّل لـ S-022 وليس سبباً للإعادة.
- 2026-10-22، المختبِر (S-021c: AC8 وAC11 وAC12): كُتبت الفحوص من المعايير وجدول الحالات وموافقة محمد على السؤال 1، دون قراءة شيفرة الميزة. شُغّلت على بناء main ‏(62a013a، `cargo build --release` جديد) داخل `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200%` بـ `CARGO_BUILD_JOBS=2`، وكل مؤلِّف خاص وحده (KWin ‏`--virtual` ثم Hyprland 0.56.2 متداخل، بـ HOME مؤقت)، ولا سطح المكتب الحي.
  - الملفات: `tests/drop/layer.sh` (‏`reveal_ms` و`reveal_budget`)، و`tests/drop/kwin.sh` و`tests/hyprland/repro.sh` (فحص AC8 لكل ظهور ظاهر)، و`tests/drop/budget.sh` (الوضعان)، و`crates/ui/tests/measure.sh` (أسطر الملخص الجديدة فقط)، و`tests/drop/hold_docs.sh` (‏AC11 وAC12)، و`tests/drop/README.md` (البند الإضافي: المرجع الجديد على مسار الطبقة).
  - العقد الذي تثبّته الفحوص (لا يُعدَّل):
    - AC8، الشق الأول: «ظاهر» كما في S-021b: أول `-> wl_surface#C.attach(wl_buffer` على السطح الفرعي للخلية (C من `get_subsurface`). الزمن بطوابع أثر الخدمة من أول `wl_data_device.enter` يسمّي الماسك إلى ذلك `attach`، بين 0 و150 ms شاملاً، في كل ظهور ظاهر: أربعة في `repro.sh` على مسار الطبقة (‏AC9 (3) و`cell` و`timeout` و`esc`، ويجري في `--json` و`--json --quasimode`)، وثلاثة عشر في `tests/drop/kwin.sh level=1` (الظهورات 1 إلى 13). ظهور بلا `enter` أو بلا `attach`، أو عدد ظهورات أقل، يُفشل الفحص.
    - AC8، الشق الثاني (`budget.sh`): ظهور المقياس دون سحب لا يُرسم بعد اليوم، فالمرجع «جاهزية الماسك» كما في S-021b (`catcher_ready`): أول `commit` على الماسك بعد أول `configure` لسطح طبقته. اختير هذا لا أول `commit` مطلقاً لأن الـ `commit` الأول قبل `configure` بلا مخزن، ولأنه الأبكر فلا يوفي بالمعنى، والمتأخر يوفي بالقراءتين. ‏`hive_bench` (شيفرة المنفِّذ في `crates/ui/examples/`) يطبع بدل `first_frame_ms`:
      - `show --drop`: ‏`mode=drop catcher_ms=A`، وA من وصول سطر الطلب إلى جاهزية الماسك.
      - `show --drop --hold`: ‏`mode=hold hold_ms=400 catcher_ms=A press_to_catcher_ms=B`، وA من انقضاء العتبة (عرض البوابة) إلى جاهزية الماسك، وB من الطلب، وB ناقص A بين 400 و450.
      - في الوضعين: سطر `hive surface: layer` واحد لكل ظهور، ولا `hive focus: true`، ولا سطر `hive reveal:` (لا سحب يدخل)، ووسيط `catcher_ms` الدافئ لعشرين ظهوراً أقل من 150 ms. ‏`measure.sh` يطبع `warm catcher_ms median` و`max` و`warm press_to_catcher_ms median`.
    - AC12 (‏`hold_docs.sh`، الأقسام والتطبيع كما في S-019: بلا تشكيل، وأ إ آ تُقرأ ا). في `integrations/linux/hyprland/README.md`، في قسم عربي واحد وقسم إنكليزي واحد، ثلاث جمل (الجملة تنتهي عند . ! ? ؟ أو نهاية السطر، دون الشيفرة المضمّنة):
      - only: الإنكليزية `only` و`drag...` و`action`؛ العربية `فقط` أو `الا` و`سحب` و`اجراء`.
      - first: الإنكليزية `start` أو `begin` و`drag...` و`then` أو `first` أو `before` و`hold` أو `press`؛ العربية `ابدا` و`سحب` و`ثم` أو `اولا` أو `قبل` و`مسك` أو `ضغط`.
      - nothing: الإنكليزية `without` أو `no` قبل `drag`، و`no action(s)` أو `without an action` أو `with no action`، و`nothing` أو `never` أو `does not show|appear`؛ العربية `دون سحب` أو `بلا سحب` أو `من غير سحب`، و`بلا اجراء` أو `دون اجراء` أو `لا اجراء` أو `ليس له اجراء`، و`لا يظهر` أو `لا تظهر` أو `لا شيء`.
      - وفي `docs/architecture.md` قسم واحد يسمّي `Probing` و`hive reveal` و`hive probe`، وجملة فيها `hidden` و`catcher`، والأسباب الخمسة كلمات: `pointer` و`no-uri-list` و`no-actions` و`release` و`cap`.
      - مثال ينجح (جُرّب على نسخة مؤقتة حُذفت): «تظهر الخلية فقط أثناء سحب ملفات لها إجراء واحد على الأقل. ابدأ السحب أولاً، ثم أمسك المفتاح. دون سحب، أو لملف بلا إجراء، لا يظهر شيء.» و«The hive shows only during a drag of files with at least one action. Start the drag first, then hold the key. Without a drag, or for a file with no action, nothing shows.» وحذف «ثم أمسك المفتاح» من العربية وحده يُفشل فحص القسم العربي.
    - AC11: في `CHANGELOG.md` تحت `## [Unreleased]` (حتى عنوان `## [` التالي) سطر يسمّي `S-021` بنص عربي وإنكليزي. وفحص الشرطة الطويلة يشمل الآن `tests/drop/README.md`.
  - `tests/drop/README.md` عدّله المختبِر (ملف اختبار، والمنفِّذ ممنوع من `tests/`): المرجع «أول إطار» صار جاهزية الماسك في فقرة مسار الطبقة والقرار 12، ولقطة الانتظار صارت على مسار النافذة، وأوضاع `tests/overlay/kwin.sh drop` كما صارت في S-021b، و`hidden (cap)` لـ Esc قبل الدخول، وحقول `budget.sh` الجديدة، وقسم S-021. لا فحص له.
  - ما ينجح قبل التنفيذ ولماذا: الشق الأول من AC8 كله، لأن S-021b نفّذ الكشف عند الدخول: `repro.sh` ‏2.7 و2.6 و4.0 و3.9 ms (‏`--quasimode`: 4.0 و2.7 و2.5 و3.5)، و`kwin.sh` ‏16.1 و2.3 و2.5 و3.7 و2.4 و2.5 و2.9 و3.8 و2.7 و2.3 و3.2 و3.2 و2.5 ms. ليس فارغاً: على سجل KWin سابق، حذف ظهور واحد من القائمة (12 من 13) أفشله، والظهورات المخفية تعطي `none` فتُفشله. وفي `budget.sh` تنجح حراس قائمة (عدد الظهورات، وسطح طبقة لكل ظهور، ولا لوحة مفاتيح) وفحص «لا `hive reveal:`». وفي `hold_docs.sh` فحوص S-019 الأحد عشر وفحص الشرطة الطويلة.
  - مخرجات الفشل (بناء main):
    ```
    $ tests/drop/budget.sh 20 1
    INFO first show after daemon start: Error: Os { code: 11, kind: WouldBlock, message: "Resource temporarily unavailable" }
    INFO drop: shows 21 (warm 20), mode=drop lines with catcher_ms 0, hive surface layer 21 window 0, hive focus: true 0, hive reveal 0, median none ms
    FAIL AC12, S-021 AC8: every show line says mode=drop with catcher_ms (hive_bench show --drop)
    FAIL AC12, S-021 AC8: median warm catcher_ms of drop-mode shows, from the request to the catcher's ready commit, under 150 ms
    RESULT FAIL (2)   (4 PASS)
    $ tests/drop/budget.sh 20 1 hold
    INFO hold: shows 21 (warm 20), mode=hold hold_ms=400 lines with catcher_ms and press_to_catcher_ms 0, threshold span lines 0, hive surface layer 21 window 0, hive focus: true 0, hive reveal 0, median from the threshold none ms, median from the press none ms
    FAIL S-019 AC20, S-021 AC8: every show line says mode=hold and hold_ms=400 with catcher_ms and press_to_catcher_ms (hive_bench show --drop --hold)
    FAIL S-019 AC20, S-021 AC8: in every show the time from the press is the threshold plus the time from its end (press_to_catcher_ms - catcher_ms 400 to 450)
    FAIL S-021 AC8: median warm catcher_ms of hold shows, from the end of the threshold to the catcher's ready commit, under 150 ms
    RESULT FAIL (3)   (4 PASS)
    $ tests/drop/hold_docs.sh
    FAIL S-021 AC12: Hyprland README, one Arabic section says the hive shows only during a drag of files with an action, to start the drag first and then hold the key, and that with no drag or for a file with no action nothing shows
    FAIL S-021 AC12: Hyprland README, one English section says ... (as above)
    FAIL S-021 AC12: docs/architecture.md, one section describes the probe: the Probing state, the hidden catcher, the "hive reveal" and "hive probe" lines and the five hide reasons (pointer, no-uri-list, no-actions, release, cap)
    FAIL S-021 AC11: CHANGELOG.md has a line under [Unreleased] that names S-021 in English and in Arabic
    RESULT FAIL (4)   (12 PASS)
    $ tests/drop/kwin.sh level=1
    RESULT PASS   (339 PASS، منها S-021 AC8 shows 1 to 13)
    $ tests/hyprland/repro.sh --json
    {"total": 40, "failed": 0, "skipped": 0}
    $ tests/hyprland/repro.sh --json --quasimode
    {"total": 47, "failed": 0, "skipped": 0}
    ```
  - سبب فشل `budget.sh`: ‏`hive_bench` ينتظر أول إطار (`after_next_frame`)، وظهور الإفلات على مسار الطبقة دون سحب لا يرسم إطاراً منذ S-021b، فتنقضي مهلة القراءة (10 ثوانٍ) في كل ظهور ولا يُطبع `catcher_ms`. هذا المرجع الذي أسقطه AC8، والإصلاح في المقياس لا في السكربت.
  - المجاميع الجديدة لـ AC11: `repro.sh --json` ‏40 (كانت 39)، و`--quasimode` ‏47 (كانت 46)، و`tests/drop/kwin.sh level=1` ‏339 PASS (كانت 338). لم تتغير `tests/drop/layer.sh` في الفحوص القائمة، ولا `tests/overlay/kwin.sh` (AC8 لا يسمّيه).
  - ما لم يُختبر: (1) الطفرات وبوابات CLAUDE.md الكاملة من AC11 للمراجِع، ولم يُشغَّل `cargo mutants` هنا. (2) قيمة `catcher_ms` التي يطبعها المقياس لا تُقارن بطوابع الأثر (‏`budget.sh` بلا `WAYLAND_DEBUG` حتى لا يبطئ الظهور)؛ صحة مرجعها مسؤولية المنفِّذ، ويقرؤها المراجِع في `hive_bench.rs`. (3) لم يُشغَّل `tests/overlay/kwin.sh` ولا `tests/drop/window_path.sh` ولا `tests/drop/kwin.sh shortcut` لأن S-021c لم يغيّر فحوصها؛ يشغّلها المراجِع ضمن AC11.
- 2026-10-22، المنفِّذ (S-021c: AC8 وAC11 وAC12): نجحت الفحوص دون تعديل أي اختبار أو سكربت اختبار. قبِل المدير بتفويض محمد الجملة الثالثة («nothing») كما صاغها المختبِر. شُغّل كل شيء داخل `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200%` بـ `CARGO_BUILD_JOBS=2`، وكل مؤلِّف خاص وحده، لا سطح المكتب الحي.
  - `layer.rs`: خطّاف قياس `after_catcher_ready(f)` (مخفي من التوثيق، كـ `after_next_frame`) يعمل مرة واحدة بعد أول `configure` لسطح طبقة الظهور، بعد أن يلتزم الفرعان الماسكَ ويدفعا الطلب (`wait_for_anchor` أو `place` ثم `render`)، أي عند «جاهزية الماسك» بتعريف `catcher_ready` في `tests/drop/layer.sh`. ‏`lib.rs`: إعادة تصدير فقط (`pub use layer::after_catcher_ready`)، فلا شيء جديد للطفرات فيه، و`layer.rs` مستثنى في `.cargo/mutants.toml`.
  - `hive_bench.rs`: ‏`show --drop` يطبع `mode=drop catcher_ms=A` من سطر الطلب، و`show --drop --hold` يطبع `mode=hold hold_ms=400 catcher_ms=A press_to_catcher_ms=B`، وA من انقضاء العتبة وB من الطلب. الظهور العادي باقٍ على `first_frame_ms`.
  - المهلة `SETTLE` (250 ms): أول تشغيل دونها أعطى `hive surface layer 0`، لأن جاهزية الماسك تسبق انتظار المرساة (50 ms) فيخفي `measure.sh` الظهور قبل وضعه وقبل سطر `hive surface:`. الزمن يُؤخذ عند الالتزام الجاهز، والمهلة تؤخر كتابة الرد فقط. معقولية القيمة: الوسيط القديم 51.3 ms كان `ANCHOR_WAIT` ونحو 1 ms، فـ `catcher_ms` قرابة 0.6 ms يوافقه. على مسار النافذة في لينكس لا ماسك فلا يعمل الخطّاف؛ المقياس لمسار الطبقة وحده، فسُجّل ذلك ولم يُعالَج.
  - التوثيق: `integrations/linux/hyprland/README.md` (قسم إمساك المفتاح بالعربية والإنكليزية: الجمل الثلاث، وأن Hyprland يرسل الحدث الأول مع أول حركة، وأن السحب بعد انقضاء المهلة لا يُظهر الخلية، وملاحظة `sbarhive drop` دون `--hold`)، و`docs/architecture.md` (قسم «Probing a drop show (S-021)»، وتصحيح مرحلة الماسك وبند الميزانية ونص الانتظار على مسار النافذة)، وسطر S-021 تحت `### Fixed` في `## [Unreleased]` في CHANGELOG.md. ‏`docs/drop-shortcut.md` و`README.md` يقولان «وأنت تسحب ملفاً»، فلا تعارض.
  - النتائج (بناء `cargo build --release` جديد): `cargo fmt --check` نظيف؛ `cargo clippy --all-targets -- -D warnings` نظيف؛ `cargo test --all` ‏563 ناجحاً و0 فاشلاً و1 متجاهَلاً؛ `scripts/design-lint.sh` نظيف؛ `tests/drop/hold_docs.sh` ‏RESULT PASS. ‏`tests/drop/budget.sh 20 1` ‏RESULT PASS (وسيط `catcher_ms` الدافئ 0.6 ms، والأقصى 0.8، و21 سطر `hive surface: layer`). ‏`tests/drop/budget.sh 20 1 hold` ‏RESULT PASS (وسيط `catcher_ms` من العتبة 0.4 ms، والأقصى 0.6، ووسيط `press_to_catcher_ms` ‏401.1 ms، و21 من 21 في نطاق 400 إلى 450). ‏`tests/hyprland/repro.sh --json` ‏`{"total": 40, "failed": 0, "skipped": 0}`. ‏`tests/drop/kwin.sh level=1` ‏RESULT PASS (339 PASS، 0 FAIL)، وزمن AC8 من الدخول إلى أول `attach` في الظهورات 13: أسوأها 6.1 ms.
  - لم يُشغَّل: `cargo mutants` (للمراجِع، ولا شيفرة جديدة قابلة للطفرة خارج `layer.rs` المستثنى)، و`repro.sh --quasimode` و`tests/overlay/kwin.sh` و`tests/drop/window_path.sh` و`tests/drop/kwin.sh shortcut`.
- 2026-10-22، المراجِع (S-021c، المراجعة 1، وهي مراجعة إغلاق S-021 كلها): **REWORK**. الشيفرة والفحوص سليمة كلها، والسبب توثيق باقٍ على المرجع القديم «أول إطار» يناقض السلوك الحالي والسكربت.
  - النطاق: `ebe60ee..HEAD` (‏43c2100)، والطفرات على فرق S-021 كله في `crates/` (‏`0f57386..HEAD`). كل شيء داخل `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200%`، وكل مؤلِّف خاص وحده، لا سطح المكتب الحي.
  - سلامة الاختبارات: `git diff --name-only ebe60ee..HEAD` يسرد `CHANGELOG.md` وملف القصة و`hive_bench.rs` و`layer.rs` و`lib.rs` و`docs/architecture.md` و`integrations/linux/hyprland/README.md` فقط. لا اختبار ولا سكربت تحت `tests/` ولا `measure.sh` تغيّر.
  - البوابات: `cargo fmt --check` خرج بـ 0؛ `cargo clippy --all-targets -- -D warnings` (‏`-j 2`) خرج بـ 0؛ `cargo test --all` (‏`-j 2`) ‏563 ناجحاً و0 فاشلاً و1 متجاهَلاً؛ `scripts/design-lint.sh` نظيف.
  - الطفرات (‏cargo-mutants 27.1.0، `--in-place`، تسلسلياً بـ `CARGO_BUILD_JOBS=1`): `--in-diff` على فرق `crates/` من 0f57386: ‏16 طفرة (‏`lib.rs` تسع و`dnd.rs` سبع)، 14 مقتولة، 0 ناجية، 0 مهلة، 2 غير قابلة للبناء (`Inner::drop_files` و`Machine::handle` بـ `Default::default()`). و`--file crates/ui/src/dnd.rs` كاملاً: ‏22 طفرة، 20 مقتولة، 0 ناجية، 0 مهلة، 2 غير قابلة للبناء (`Machine::state` و`Machine::handle`). ‏`crates/daemon/src/ui.rs` لم يتغير في S-021 فلا طفرة فيه، و`layer.rs` و`receiver.rs` مستثنيان في `.cargo/mutants.toml`، و`hive_bench.rs` مثال لا تطفّره الأداة. شجرة العمل نظيفة بعد التشغيل.
  - السكربتات بعد `cargo build --release` جديد: `tests/hyprland/repro.sh --json` ‏`{"total": 40, "failed": 0, "skipped": 0}`؛ و`--quasimode` ‏`{"total": 47, "failed": 0, "skipped": 0}`؛ `tests/drop/kwin.sh level=1` ‏RESULT PASS (339)؛ و`level=1 shortcut` ‏RESULT PASS (44)؛ `tests/overlay/kwin.sh` ‏RESULT PASS (104)، و`drop` ‏(49)، و`outputs` ‏(15)؛ `tests/drop/window_path.sh` ‏RESULT PASS (4)؛ `tests/drop/budget.sh 20 1` ‏RESULT PASS (6، وسيط `catcher_ms` الدافئ 0.5 ms والأقصى 0.8)؛ و`budget.sh 20 1 hold` ‏RESULT PASS (7، الوسيط من العتبة 0.5 ms والأقصى 1.0، ووسيط `press_to_catcher_ms` ‏401.0)؛ `tests/drop/hold_docs.sh` ‏RESULT PASS (16).
  - AC8 بطوابع الأثر: من أول `wl_data_device.enter` على الماسك إلى أول `attach` على الخلية، `repro.sh` ‏4.0 و2.4 و2.8 و2.6 ms (‏`--quasimode`: 3.9 و2.7 و2.6 و2.9)، و`tests/drop/kwin.sh` في الظهورات الثلاثة عشر أسوأها 6.0 ms. الحد 150.
  - المهلة `SETTLE` (‏`crates/ui/examples/hive_bench.rs:97-109`): مقبولة، لا تنفخ الزمن ولا تنقصه. الخطّاف يعمل في `Ev::Configured` بعد أول `configure` مباشرة (‏`crates/ui/src/layer.rs:779-788`)، والمقياس لا يمرّر `--at` فيمرّ دائماً بفرع `wait_for_anchor` الذي يلتزم الماسك ببكسله ثم يدفع الطلب (‏`layer.rs:897-907`)، أي عند «جاهزية الماسك» بتعريف `catcher_ready` في `tests/drop/layer.sh`. ‏`ms` يُحسب داخل الإغلاق قبل `reply_later` (‏`hive_bench.rs:141-149` و`205-207`)، فالمهلة تؤخر كتابة الرد وحدها، والدليل في التشغيل: `catcher_ms=0.7` و`client_roundtrip_ms=250.6`. والقيمة معقولة: الوسيط القديم 51.3 ms كان `ANCHOR_WAIT` ‏(50) ونحو 1 ms. ملاحظة غير حاجبة سجّلها المنفِّذ: الخطّاف واحد عام ولا يعمل إلا على مسار الطبقة، فمقياس `--drop` على مسار النافذة في لينكس ينتظر حتى مهلة القراءة.
  - اللقطات: لا لقطة جديدة في S-021c. مرشّح `tests/drop/out/drop-en-light-waiting.png` يطابق المرجع بالعين (الخلية المركزية «Drop files here»، بلا ظل ولا تدرج) وبالفحص 0.000000، وكذلك `drop-en-light-over.png`.
  - النصوص: README الخاص بـ Hyprland يقرأ بعربية طبيعية لا مترجمة، والقسمان متطابقان مع السلوك (الجمل الثلاث، وحاجة Hyprland إلى أول حركة، والسحب بعد العتبة لا يُظهر شيئاً، و`drop` دون `--hold`). سطر CHANGELOG.md تحت `## [Unreleased]` بالعربية والإنكليزية. لا عنصر تفاعلي جديد. لا شرطة طويلة في أي ملف تغيّر في S-021 كلها.
  - أسباب الإعادة (تناقض باقٍ في التوثيق، والموجز يطلب ألا يبقى تناقض):
    1. `tests/drop/README.md:185` (للمختبِر، فالمنفِّذ ممنوع من `tests/`): «show 12, where the drag moves about 100 ms after the first frame, inside the 300 ms window of manager decision 12». الظهور 12 سحب ممسوك لا تُرسم خليته إلا بعد الدخول، أي بعد الحركة، والسكربت يقيس النافذة من جاهزية الماسك (`tests/drop/kwin.sh:393-406`). تُصحَّح الجملة إلى «بعد جاهزية الماسك» كما في فقرة القرار 12 في الملف نفسه (الأسطر 60 إلى 67). ومعها تعليقا السكربت نفسه، دون أي تأكيد: `tests/drop/kwin.sh:70` (‏«about 100 ms after the first frame») و`kwin.sh:366-367` (‏«the first frame comes some 60 to 80 ms after it»)، وكلاهما يناقض التعليق في `kwin.sh:394-396`.
    2. `docs/architecture.md:147-154` (للمنفِّذ): بند «Anchor and wait» ما زال يعدّ `wl_pointer.enter` على الماسك مرساةً «before the first frame»، ويقول «After the first frame the centre does not follow the pointer»، دون تقييد. في ظهور الإفلات على مسار الطبقة `wl_pointer.enter` يُخفي الظهور ولا إطار قبل الكشف، والمنفِّذ نفسه غيّر المرجع في بند «Catcher phase» تحته إلى «the placement». يكفي تقييد البند بالظهور العادي، أو جعل المرجع الموضع (نهاية `ANCHOR_WAIT`) لا أول إطار، كما في AC4.
  - ملاحظات غير حاجبة: (أ) `integrations/linux/hyprland/README.md:50`: «انقضاء المهلة» مرتين بمعنى عتبة الضغط المطوّل، والسطر 26 يستعمل «المهلة» لـ `SBARHIVE_COPY_DELAY`، والقصة تسمّي الحد 10 ثوانٍ مهلة أيضاً؛ «انقضاء العتبة» أوضح. (ب) `docs/architecture.md:165-171` لم يُعَد لفّه بعد التعديل، و`docs/architecture.md:215` يتجاوز 80 عموداً. (ج) `crates/ui/tests/measure.sh:19-22` ما زال يصف `first_frame_ms` لـ `HOLD=1` قبل ملاحظة S-021، ويطبع `warm press_to_frame_ms median none` (ملف المختبِر). (د) `docs/drop-shortcut.md:96` و`98` («so the hive shows there at once» و«an immediate show») ليسا من الملفات الأربعة؛ يحسن ذكر أن الخلية تظهر مع سحب ملفات لها إجراء.
  - بعد الإصلاح يكفي في المراجعة 2: `tests/drop/hold_docs.sh`، وقراءة الملفات المعدّلة، والتأكد أن الفرق لا يمس تأكيداً ولا شيفرة. لم يُضف سطر CHANGELOG.md جديد (سطر S-021 موجود).
- 2026-10-22، المختبِر (S-021c، إصلاح المراجعة 1، تعليقات وتوثيق فقط): لم يتغير أي تأكيد ولا منطق اختبار.
  - `tests/drop/README.md` (فقرة S-019e): الظهور 12 يتحرك بعد LATE_MS (‏170 ms افتراضياً) من عودة `sbarhive drop`، والنافذة (300 ms) تُعدّ منذ S-021 AC4 من جاهزية الماسك (`catcher_ready` في `layer.sh`) لا من أول إطار.
  - `tests/drop/kwin.sh`: تعليق النوع `late` في رأس الملف (السطر 70 وجملة «The first enter on the catcher must fall after the first frame») وتعليق الفرع `late` (السطران 366 و367) صارا يقيسان من جاهزية الماسك، موافقَين للتعليق عند الفحص. آخر تشغيل في `tests/drop/out` سجّل الدخول الأول بعد 170.6 ms من جاهزية الماسك، فحُذف رقم «60 إلى 80 ms».
  - `crates/ui/tests/measure.sh`: وصف `HOLD=1` صار `catcher_ms` و`press_to_catcher_ms`، وحُذف سطر الملخص `warm press_to_frame_ms median` الذي يطبع `none` بعد التأكد أن لا سكربت يقرؤه (`grep -rn press_to_frame tests crates scripts`: تعليقات قديمة في `tests/drop/budget.sh:22-27` ومخرجات في `tests/drop/out/` المتجاهَل فقط، و`budget.sh` يقرأ `press_to_catcher_ms`).
  - التشغيل: `bash -n` على `kwin.sh` و`measure.sh` نظيف، و`tests/drop/hold_docs.sh` ‏RESULT PASS، ولا شرطة طويلة في الملفات المعدّلة.
- 2026-10-22، المنفِّذ (S-021c، إعادة العمل 1، توثيق فقط): أُصلحت أجزاء التوثيق التي للمنفِّذ من أسباب المراجعة 1، دون أي شيفرة أو اختبار. ملفات المختبِر (`tests/drop/README.md` و`tests/drop/kwin.sh` و`crates/ui/tests/measure.sh`) لم تُمسّ.
  - `docs/architecture.md`: بند «Anchor and wait» صار مقيَّداً بالظهور العادي، ومرجعه الموضع (نهاية `ANCHOR_WAIT`) لا أول إطار، ويحيل ظهور الإفلات دون `--at` إلى بند مرحلة الماسك. وأُعيد لفّ بند «Catcher phase» وفقرة المعاينة على 80 عموداً.
  - `integrations/linux/hyprland/README.md`: «انقضاء المهلة» صارت «انقضاء العتبة» في الموضعين، لأن «المهلة» مستعملة لـ `SBARHIVE_COPY_DELAY` وللحد 10 ثوانٍ.
  - `docs/drop-shortcut.md`: قسما Hyprland بالعربية والإنكليزية يقولان إن الخلية لا تظهر إلا أثناء سحب ملفات لها إجراء، بدل «فوراً» و«at once» و«an immediate show».
  - النتائج: `tests/drop/hold_docs.sh` ‏RESULT PASS؛ `scripts/design-lint.sh` نظيف؛ لا شرطة طويلة في الملفات المعدّلة. لم يُشغَّل غيرهما بطلب المدير، فالتغيير توثيق فقط.
