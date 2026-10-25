# S-020 الإفلات على خلية الطبقة يعمل على Hyprland
الحالة: منتهية

الدليل (`tests/hyprland/repro.sh`، Hyprland 0.56.2 خاص متداخل): في وضع الإفلات الخلية سطح فرعي للماسك، والماسك بمنطقة إدخال فارغة، والمستقبِل يستهدف السطح الفرعي (`crates/ui/src/layer.rs` 479 إلى 524، `receiver.rs` 409 إلى 419). Hyprland لا ينشئ view لسطح فرعي من سطح طبقة (`Subsurface.cpp:14-60`، `WLSurface.cpp:195-199`)، فلا يرسل `wl_data_device.motion` (`DataDevice.cpp:614-628`)، ويأتي `enter` دائماً عند نصف حجم السطح (`DataDevice.cpp:712`). النتيجة: لا تضيء خلية، ويُرفض كل إفلات، وتبقى الخلية حتى مهلة 10 ثوانٍ. `repro.sh --json`: 11 فحصاً، يفشل منها اثنان. مسار النافذة وKWin سليمان. للإصدار 0.3.0، ويسبق S-019.

## القصة
بصفتي مستخدماً على Hyprland، أريد أن تضيء الخلية تحت الملف المسحوب وأن يُنفَّذ إجراؤها عند الإفلات، حتى يعمل الإفلات على الخلية كما يعمل على KDE.

## معايير القبول
- AC1: في وضع الإفلات على مسار الطبقة، في أثر `WAYLAND_DEBUG=1` لكل ظهور:
  - قبل أول `commit` للماسك `set_input_region` بمنطقة فارغة كما اليوم، فلا يعمل الماسك بالمنطقة الافتراضية لحظةً.
  - بعد أول `configure` وقبل أول إطار للخلية، آخر `set_input_region` على الماسك منطقة فيها `add` واحد بالضبط يساوي `X Y W H` من سطر `hive surface: layer X Y W H` للظهور نفسه.
  - السطح الفرعي للخلية `set_input_region` بمنطقة فارغة (بلا `add`).
- AC2: المستقبِل على الماسك: أول `wl_data_device.enter` في الظهور يسمّي سطح الماسك (المُمرَّر إلى `get_layer_surface`)، وإحداثياته داخل المستطيل `[X, X+W] × [Y, Y+H]`. ويُطرح `(X, Y)` قبل `dnd::pick`.
- AC3: `tests/hyprland/repro.sh --json` (الوضع الافتراضي layer، بلا `--quasimode`) يطبع `"failed": 0`، ومنه بالاسم:
  - `drop mid-drag: the hive gets drag motion events` (عدد `motion` أكبر من 0).
  - `drop mid-drag: released over the WEBP cell writes the file`.
  - الفحوص التسعة الباقية كما هي، وسطر `drop cells:` واحد أو أكثر في حالة `cell` (للاطمئنان، فهو يظهر اليوم أيضاً).
- AC4: فحص جديد في `repro.sh`: في ظهور `drop` دون سحب، نقرة في (40، 40) خارج المستطيل تصل `keys.py` (نقرة واحدة)، والخلية باقية حتى المهلة. فيصير المجموع 12 والفاشل 0.
- AC5: الوضع العادي دون تغيير: صفر `set_input_region` في ظهور `open` في `tests/overlay/kwin.sh`، وكل فحوصه الحالية ناجحة.
- AC6: `tests/drop/kwin.sh level=1` و`tests/drop/layer.sh` (بالتأكيدات المعدّلة أدناه) و`tests/overlay/kwin.sh` و`tests/drop/gnome.sh smart` (المستقبِل مشترك مع مسار النافذة) تطبع `RESULT PASS`، ومعها بوابات CLAUDE.md وسطر في CHANGELOG.md بالعربية والإنكليزية.

## حالات الاختبار
| معيار | الاختبار | النوع |
| --- | --- | --- |
| AC1، AC2 | `tests/drop/layer.sh` معدّلاً، من `tests/drop/kwin.sh` و`tests/overlay/kwin.sh drop` | تكامل |
| AC3، AC4 | `tests/hyprland/repro.sh --json`، وفحص النقرة الجديد | تكامل |
| AC5، AC6 | `tests/overlay/kwin.sh` كله، و`tests/drop/kwin.sh level=1`، و`gnome.sh smart`، والبوابات | تكامل، وحدة |

تأكيدات تتغير بالقرار 2 (يعدّلها المختبِر وحده، ويُمنع المنفِّذ من لمسها):
- `tests/drop/layer.sh:70-71`، «منطقة الماسك فارغة»، يصير AC1.
- `tests/drop/layer.sh:76-77`، «الدخول على السطح الفرعي»، يصير AC2، ويُضاف فحص المنطقة الفارغة للسطح الفرعي.
- `tests/overlay/kwin.sh:643`، «لا `wl_pointer.enter` على الماسك»، يصير: لا دخول على الماسك بإحداثيات خارج المستطيل، وصفر `wl_pointer.button` للنقرة في (100، 700).

## خارج النطاق
- المرساة والموضع عند المؤشر (S-019d يبني على هذه المنطقة).
- الطفرات: الطرح في `receiver.rs` والمنطقة في `layer.rs` مستثناة كقرار S-018 الخامس، ويحرسهما الإفلات الفعلي على WEBP في KWin وHyprland.
- قاعدة `stay_focused` في إعداد Hyprland الحي في جلسة حقيقية تجعل النقرة خارج خلية مسار النافذة لا تغلقها. `integrations/linux/hyprland/README.md` لا يذكرها ولا يوصي بها (فحص 2026-10-21). لكن `tests/hyprland/repro.sh:114` يضعها نسخةً من قواعد المشرف، فلا يصلح `repro.sh window` بوابةً هنا.

## سجل المراجعة
- 2026-10-21، المخطِّط: الجهد يوم عمل أو أقل، فلا تقسيم. الحالة «مسودة» لأن الاعتماد يُسجَّل في السطر الأول من المدير أو المشرف، كما في S-018. AC4 إضافة من المخطِّط تنتظر المدير. ملاحظة للمدير: بعد النقل تصل النقرة داخل المستطيل (دون سحب) الماسكَ، فيقرؤها `layer.rs` نقرةً خارجية (420 إلى 423) وتُغلق الخلية. اليوم تُغلق أيضاً (`hive.slint` 178 إلى 184: الخلية `choose` بلا ملفات، وغيرها `dismiss`)، فالنتيجة الظاهرة واحدة ولا معيار لها هنا. وإبقاؤها مفتوحة شأن S-019 AC8.
- 2026-10-21، قرارات المدير (بتفويض من المشرف، كما ذكر المدير):
  1. في وضع الإفلات يستقبل الماسك السحب: منطقته مستطيل الخلية من `overlay::place` بإحداثيات الماسك، ومنطقة السطح الفرعي فارغة، والمستقبِل على الماسك، ويُطرح موضع الخلية قبل `dnd::pick`. خارج المستطيل يعبر السحب والنقر إلى التطبيقات (S-018 AC13). الوضع العادي دون تغيير.
  2. يحل محل تأكيدات S-018d في `tests/drop/layer.sh` (الماسك الفارغ والدخول على السطح الفرعي)، والمعايير المعدّلة أعلاه.
  3. البوابات: `repro.sh --json` بـ `failed 0`، و`tests/drop/kwin.sh` و`layer.sh` و`tests/overlay/kwin.sh` خضراء، وبوابات CLAUDE.md. البوابات الثقيلة مرة واحدة في النهاية (`nice -n 19`، `-j 2`، مؤلِّف واحد في كل مرة).
  4. S-019 يعتمد على S-020، وخطوة الإمساك في S-019d تبني على هذه المنطقة.
  5. سطر في جدول 0.3.0 في `docs/PLAN.md` بجانب S-019.
  6. `stay_focused` خارج النطاق، مسجّل أعلاه.
- 2026-10-21، المدير: اعتماد القصة. النقاط الخمس كما اقترحها المخطِّط: ترتيب منطقة الإدخال بعد configure وقبل أول إطار، وAC4 معتمد (12 فحصاً)، والنقرة على الخلية بلا سحب تُترك لـ S-019، وgnome.sh smart بوابة إضافية، والدليل فحصا repro.sh لا سطر drop cells.
- 2026-10-21، المختبِر: الاختبارات من المعايير وحدها، على الشيفرة الحالية بعد `cargo build --release` (`nice -n 19`، `CARGO_BUILD_JOBS=2`، مؤلِّف واحد في كل مرة، كل سكربت ثقيل مرة واحدة).
  - `tests/drop/layer.sh`: السطران 70-71 صارا ثلاثة فحوص AC1 (منطقة فارغة قبل أول `commit` للماسك؛ بعد أول `configure` وقبل أول `commit` للماسك بعده، وهو الذي يعرض الخلية المتزامنة، آخر منطقة للماسك `add` واحد يساوي `X Y W H` من سطر `hive surface: layer`؛ والسطح الفرعي `set_input_region(wl_region)` بلا `add`، و`nil` مرفوض لأنه منطقة غير محدودة). السطران 76-77 صارا فحصي AC2 (أول `enter` يسمّي الماسك، وإحداثياته داخل المستطيل بمقارنة awk للأعداد العشرية). طرح `(X, Y)` قبل `dnd::pick` لا يظهر في الأثر، ويحرسه الإفلات الفعلي على WEBP.
  - `tests/overlay/kwin.sh`: فحص السطر 643 صار: لا `wl_pointer.enter` على الماسك بإحداثيات خارج المستطيل، وصفر `wl_pointer.button`. وفحص AC5 جديد في وضع layer: صفر `set_input_region` في كل ظهور غير `drop`.
  - `tests/hyprland/repro.sh`: فحص AC4 في ظهور `drop` دون سحب (نقرة واحدة في `keys.py`، والخلية ظاهرة بعد النقرة، وتختفي بعد 9 ث أو أكثر من الظهور)، خارج شرط `ping` فيبقى المجموع 12 دائماً. ومعه إصلاحات مراجعة المنسِّق: زمن الظهور يُقاس على خدمة بلا `WAYLAND_DEBUG` (تُعاد بعد الظهور الأول) وبانتظار `tail -f | grep -m1` بدل الاستطلاع؛ حُذفت قاعدة `stay_focused` مع تعليق بالسبب؛ `drag_past_threshold` و`move_to_webp` من `tests/drop/lib.sh` عبر `ptr_move` و`ptr_down` و`ptr_up`؛ خريطة المؤشر تُحسب مرة واحدة و`move` حساب صحيح؛ قراءة CPU لعمليتين بنوم واحد؛ `"skipped": 0` مباشرة؛ ودالة `start_keys`. أُبقي `keys.py` ولم يُدمج في `tests/overlay/other.py`: AC4 المعتمد يسمّيه، والدمج يغيّر معرّف التطبيق (`org.kawaro.HyprKeys` الذي ينتظره `repro.sh`) وبادئة السجل وقابلية التركيز في `other.py`.
  - الفشل قبل التنفيذ:
    - `repro.sh --json`: `{"total": 12, "failed": 2, "skipped": 0}`:
      ```
      INFO drag cell: under way yes; drop hive at 284 89 393 362 in 3 ms; enter at 196.50000000,181.00000000 (surface-local); cursor over WEBP at 519, 201; motion events 0; kawarod 1% CPU; file written no; hive gone yes; source: drag-end deleted=False
      FAIL drop mid-drag: the hive gets drag motion events
      FAIL drop mid-drag: released over the WEBP cell writes the file
      ```
      السبب: الدخول على السطح الفرعي للخلية، وHyprland لا يرسل له `motion`، فيُرفض الإفلات.
    - `tests/drop/kwin.sh level=1`: `RESULT FAIL (20)`، أربعة لكل ظهور من الخمسة، مثل الظهور 1:
      ```
      FAIL S-020 AC1 show 1: after the first configure and before the first hive frame the catcher's last input region is one add equal to the hive rectangle
      FAIL S-020 AC1 show 1: the hive subsurface has an empty input region (wl_region, no add)
      INFO S-020 show 1: first enter on wl_data_device#28: surface wl_surface#21 at 4.00000000 4.00000000 (catcher wl_surface#17, hive 444 219 393 362)
      FAIL S-020 AC2 show 1: the first enter on this show's data device names the catcher
      FAIL S-020 AC2 show 1: the first enter is inside the hive rectangle [X, X+W] x [Y, Y+H]
      ```
      السبب: الماسك لا يأخذ منطقة الخلية بعد `configure`، والسطح الفرعي بلا `set_input_region`، والدخول على السطح الفرعي بإحداثياته المحلية (4، 4).
    - `tests/overlay/kwin.sh drop`: `RESULT FAIL (6)`، فحصا AC1 نفسهما في كل ظهور من الثلاثة.
  - حُرّاس تنجح اليوم عمداً، لأنها تحمي ما يجب ألا يتغير: AC1 «منطقة فارغة قبل أول commit»، وAC4 (`keys.py clicks 1`، والخلية اختفت بعد 10052 ms)، وفحص السطر 643 المعدّل (صفر دخول وصفر زر اليوم)، وAC5 (`tests/overlay/kwin.sh` في وضع layer: `RESULT PASS`). `gnome.sh smart` لم يُعدَّل ولم يُشغَّل، فهو بوابة للمنفِّذ.
- 2026-10-21، المنفِّذ: لم يُعدَّل أي اختبار.
  - `crates/ui/src/layer.rs`: في وضع الإفلات يأخذ السطح الفرعي للخلية منطقة إدخال فارغة قبل `get_subsurface`؛ ويبقى للماسك منطقته الفارغة قبل أول `commit`؛ و`place()` (بعد كل `configure` وتغيّر حجم أو مقياس، وقبل `commit` الإطار) يضع للماسك منطقة فيها `add(X, Y, W, H)` واحد من `overlay::place`، ويمرّر `(X, Y)` إلى المستقبِل. المستقبِل يُربط بسطح الماسك بدل السطح الفرعي. الوضع العادي دون تغيير. الحقل `drop_mode` في `Surfaces` يثبّت الوضع للظهور كله.
  - `crates/ui/src/receiver.rs`: `set_offset(x, y)` وثابت `OFFSET` يُصفَّر في كل `attach` (مسار النافذة يبقى (0، 0))، ويُطرح قبل `dnd::pick`.
  - لا منطق جديد في `overlay.rs`: المستطيل هو `Placement` نفسه (`x`، `y`، `destination`) الذي يطبعه سطر `hive surface`، فلا حساب يُختبر وحدةً. الطرح والمنطقة في ملفين مستثنيين من الطفرات (قرار S-018 الخامس)، ويحرسهما الإفلات الفعلي على WEBP في Hyprland وKWin.
  - البوابات (مرة واحدة، `nice -n 19`، `CARGO_BUILD_JOBS=2`، مؤلِّف واحد في كل مرة): `cargo fmt --check` نظيف؛ `cargo clippy --all-targets -- -D warnings` نظيف؛ `cargo test -p kawaro-ui` 94 و25 و38 ناجحة، 0 فاشل؛ `cargo build --release` ناجح؛ `tests/hyprland/repro.sh --json` `{"total": 12, "failed": 0, "skipped": 0}` (motion events 2، file written yes)؛ `tests/drop/kwin.sh level=1` `RESULT PASS` (98 PASS، ومعه `tests/drop/layer.sh`)؛ `tests/overlay/kwin.sh drop` `RESULT PASS` (26)؛ `tests/overlay/kwin.sh` `RESULT PASS` (99)؛ `tests/drop/gnome.sh smart` `RESULT PASS` (46)؛ `scripts/design-lint.sh` نظيف؛ `cargo mutants --in-diff -j 2`: لا طفرات (الفرق كله في ملفات مستثناة).
- 2026-10-21، المراجِع (s020-reviewer، بسياق جديد): **PASS**. أُعيد تشغيل كل بوابة مرة واحدة على HEAD 7b7f295 بعد `cargo build --release` (`nice -n 19`، `-j 2`، مؤلِّف واحد في كل مرة، دون لمس سطح المكتب الحي).
  - `cargo fmt --check` نظيف؛ `cargo clippy --all-targets -- -D warnings` بلا تحذير؛ `cargo test -p kawaro-ui` 94 و25 و38 ناجحة، 0 فاشل؛ `scripts/design-lint.sh` نظيف؛ `cargo mutants --in-diff` على فرق S-020: لا طفرات (الشيفرة المتغيرة في `layer.rs` و`receiver.rs` المستثنيين بقرار S-018 الخامس، و`lib.rs` تعليق فقط).
  - AC3 وAC4: `tests/hyprland/repro.sh --json` `{"total": 12, "failed": 0, "skipped": 0}`؛ الدخول على الماسك عند 480،270 داخل الخلية 284 89 393 362، و`motion events 2`، و`file written yes`؛ نقرة (40، 40) تصل `keys.py` مرة واحدة والخلية تختفي بعد 10087 ms؛ زمن الظهور 7 ms؛ وأسطر `drop cells:` موجودة في سجل الخدمة.
  - AC1 وAC2 وAC6: `tests/drop/kwin.sh level=1` `RESULT PASS` (97 سطر PASS، صفر FAIL؛ المنفِّذ ذكر 98)، ومنها فحوص S-020 الستة لكل ظهور من الخمسة، والدخول الأول على الماسك عند 448،223 داخل 444 219 393 362، وكتابة WEBP في الظهورين 1 و3، وهي التي تثبت طرح `(X, Y)` قبل `dnd::pick`. `tests/overlay/kwin.sh drop` `RESULT PASS` (26)، وصفر دخول وصفر زر على الماسك في `drop1`.
  - لم يُعَد تشغيل `tests/overlay/kwin.sh` كله (AC5) ولا `gnome.sh smart`: الوضع العادي محروس بـ `drop_mode` في كل سطر جديد، و`set_offset` يستدعيه `layer.rs::place()` وحده، و`attach` يصفّر الإزاحة في كل ظهور، فمسار النافذة (0، 0). نتيجتاهما من تشغيل المنفِّذ.
  - قراءة الفرق: منطقة الماسك فارغة حتى أول `configure`، ثم تأتي مع موضع السطح الفرعي في `commit` الإطار نفسه؛ `attach` يسبق أول `place()` فلا إزاحة قديمة بين الظهورات؛ المنطقة والإزاحة والموضع كلها بإحداثيات منطقية، كما إحداثيات `wl_data_device`؛ كل `wl_region` يُتلف بعد `set_input_region`. سطر CHANGELOG.md موجود من المنفِّذ (عربي وإنكليزي)، فلم يُضف سطر ثانٍ. لا لقطات جديدة.
  - ملاحظات صغيرة لا تحجب: (1) `set_offset` يسري فوراً بينما المنطقة والموضع يسريان مع `commit` التالي، وكل `place()` يتبعه رسم، فالنافذة الزمنية إطار واحد، ولا يتغير حجم الخلية أثناء الإفلات؛ (2) لا تشغيل للإفلات بمقياس كسري، والشيفرة متسقة منطقياً؛ (3) النقرة داخل الخلية دون سحب تصل الماسك فتغلق، كما سجّل المخطِّط، وتُترك لـ S-019 AC8.
