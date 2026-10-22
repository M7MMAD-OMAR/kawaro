# S-022 ثلاث متابعات لخلية الإفلات بعد S-021
الحالة: مراجعة (2026-10-22)؛ نُفِّذت والاختبارات ناجحة، عند المراجِع

ثلاث ملاحظات من S-021 (الإصدار 0.3.1، 2026-10-22)، تفاصيلها في سجل مراجعة `backlog/S-021-drop-only-on-drag.md`:
1. مختبِر S-021b: `drop --release` والسحب فوق خلية مرسومة، ثم سحب جديد: دخوله لا يُقرأ (لا `drop cells:` ولا `accept`، ولا WEBP)، والخدمة تهدم عرض الظهور السابق عند العرض الجديد. إصلاح منفِّذ S-021b في `receiver.rs` (حفظ جهاز البيانات الذي دخله السحب) يصلحه على الأرجح، ولم يُختبر. ‏AC9 (3) يغادر الخلية قبل الترك فلا يصله.
2. منذ S-021b تُكشف خلية الإفلات على مسار الطبقة دون حركة الفتح: `show()` يشغّل `opener` عند الربط (`lib.rs:783-797`) والخلية خفية.
3. مراجِع S-021b (`lib.rs:714-720`): ظهور إفلات جديد فوق ظهور في `Probing` يمر بـ `inner.close()` دون سطر، فيكسر عقد «سطر واحد لكل ظهور» (S-021 AC3).

## القصة
بصفتي مستخدماً على Hyprland أو KDE، أريد أن يعمل السحب التالي بعد ترك المفتاح فوق الخلية، وأن تنفتح الخلية بحركتها حين تُكشف، حتى يبقى الإفلات موثوقاً ومتسقاً مع الظهور العادي.

## قرارات المخطِّط (للمدير)
1. قصة واحدة دون تقسيم: البند 1 فحوص فقط إن نجح على main، والبندان 2 و3 بضعة أسطر في `lib.rs`. إن فشل AC1 على main فالإصلاح هنا (`receiver.rs` أو `layer.rs`)، ولا تُقسَّم إلا إن تجاوز يوماً.
2. «إن أمكن في KWin» = إن أمكن قيادته: يُضاف إلى دورة السحب فوق WEBP في `hold_cycles` بـ `tests/drop/kwin.sh`. أثناء السحب يمسك جهاز البيانات المؤشر، فلا يعيقه القرار 6 في S-021.
3. إثبات الفشل أولاً: على main ينجح الفحص غالباً. يعكس المختبِر مؤقتاً قطعة حقل الجهاز من S-021b في `receiver.rs` محلياً، ويبني ويشغّل AC1، ويسجّل الفشل، ثم يعيدها (`git diff` نظيف، دون commit). لا worktree. إن نجح AC1 على البناء المعكوس أيضاً يبقى حارساً، ويسجّل المختبِر أن العلة لم تظهر حمراء، وتكمل القصة دون سؤال.
4. الحركة: لا يُشغَّل `opener` ما دامت الآلة `Probing`، ويُشغَّل عند الانتقال من `Probing` إلى `Waiting` في `Inner::drag` (`lib.rs`، تحت الطفرات)، لا في `layer.rs` المستثنى. أول إطار مكشوف هو أول إطار الحركة، فلا ينتظرها.
5. تقليل الحركة: الآلية القائمة كافية (`hive-open` ‏0 ms و`scale-from` = `scale-to` في `build.rs:318` و`323`).
6. اللقطات: لا يتغير أي مرجع. لقطتا `over` و`thumb` بعد الدخول بنحو 800 ms أو أكثر، والحركة 120 ms. أي فرق خلل لا اعتماد جديد.
7. السبب الجديد `replaced`، فلا سبب من الخمسة يصفه. يُطبع فقط إن كان الظهور المستبدَل في `Probing` على مسار الطبقة. استبدال ظهور مكشوف أو على مسار النافذة لا يطبع شيئاً (عقد S-021b).
8. الاستبدال لا يستدعي `on_dismiss`، كما `close()` اليوم، لأنه قد يمس حالة الطلب الجديد في الخدمة. استثناء مسمّى من S-021 AC2.
9. قرار الاستبدال في `lib.rs` أو `dnd.rs` باختبار وحدة، والطباعة في `layer.rs`. الأسماء يثبّتها المختبِر.
10. لف `tests/drop/README.md` ملف المختبِر، ويخص النثر خارج كتل الشيفرة.

## معايير القبول
- AC1: `repro.sh` (مسار الطبقة): سحب PNG أول، ‏`drop --hold`، دخول الخلية (`hive reveal: 15`) والبقاء فوق خلية WEBP؛ ‏`drop --release` والسحب فوقها؛ هدم الطبقة خلال 500 ms، ولا WEBP للملف الأول؛ ترك الزر في مكانه. ثم سحب PNG ثانٍ و`drop --hold` ودخول: سطر `drop cells:` واحد، و`hive reveal: 15`، و`wl_data_offer.accept` بنوع غير `nil`، وWEBP الملف الثاني مكتوب، وصفر خطأ بروتوكول.
- AC2: التسلسل نفسه في `tests/drop/kwin.sh level=1` (‏`hold_cycles`) بالنتائج نفسها.
- AC3: وحدة (نصف الكشف): ظهور إفلات تضعه الاختبارات في `Probing` ثم توقف `inner.opener` وتضع `opened` خطأ، فلا يجعله صحيحاً إلا الكشف. يبقى خطأ بعد `Tick` عند 9.9 ثانية؛ ‏`Enter` بخلية أو أكثر يجعله صحيحاً بعد دورة حلقة واحدة على الأكثر؛ ‏`Reject` و`PointerEnter` يُخفيان وهو خطأ. الظهور العادي وظهور مسار النافذة: صحيح بعد مؤقت الربط كما اليوم. مع `SBARHIVE_REDUCED_MOTION=1`: صحيح عند الكشف. نصف `show()` (لا `opener` في `Probing`) لا تصله المنصة المخفية، ويفحصه AC4.
- AC4: `repro.sh`، ظهور PNG مكشوف: السحب يدخل عند مركز الخلية (لا خلية مضاءة) ويسكن 400 ms. عدد `attach` على السطح الفرعي C في 200 ms بعد أوله: 4 أو أكثر بالحركة، و2 أو أقل في خدمة بـ `SBARHIVE_REDUCED_MOTION=1`. وعدد ظهور عادي بالحركة سطر INFO للمعايرة.
- AC5: ‏S-021 AC8 دون تغيير: من أول `wl_data_device.enter` إلى أول `attach` على الخلية 150 ms أو أقل في كل ظهور مكشوف في `repro.sh` و`tests/drop/kwin.sh level=1`، ومنها ظهورات AC1 وAC2 وAC4.
- AC6: لقطات `tests/drop/kwin.sh level=1` تختلف عن المراجع بـ 0.1 % أو أقل، دون مرجع جديد. ولقطتا `over` و`thumb`: فحص ناجح أو فاشل أن كلاً منهما بعد أول `attach` في ظهورها بـ 300 ms أو أكثر (لقطة `waiting` قبل الدخول فلا تشملها).
- AC7: `repro.sh`: المؤشر ساكن في (40، 40)، ‏`drop --at 500,300` مرتين بفاصل 1 s، ثم حركة إلى (500، 300): سطرا فحص بالضبط للظهورين، `hive probe: hidden (replaced)` ثم `hive probe: hidden (pointer)`، والأول بين سطري `hive surface:` للظهورين، وصفر `attach` على الخلية. وفي `tests/drop/kwin.sh`: ‏`drop --at 640,400` مرتين والمؤشر في (100، 700)، ثم حركة إلى (640، 400): `hidden (replaced)` ثم `hidden (pointer)` بالشروط نفسها.
- AC8: وحدة: استبدال ظهور في `Probing` يعطي السبب `replaced` مرة واحدة، و`on_dismiss` صفر مرة. استبدال `Waiting` أو `Over` أو `Normal`: لا سبب.
- AC9: `docs/architecture.md` يسرد الأسباب الستة ويقول إن حركة الفتح تبدأ عند الكشف؛ و`hold_docs.sh` يفحص `replaced`، وسطر S-022 بالعربية والإنكليزية تحت `## [Unreleased]` في CHANGELOG.md.
- AC10: `tests/drop/README.md`: صفر سطر نثر يتجاوز 80 عموداً خارج كتل الشيفرة (اليوم 4: الأسطر 82 و104 و179 و191)، والكتل كما هي.
- AC11: `repro.sh --json` و`--json --quasimode` ‏`"failed": 0`؛ ‏RESULT PASS في `tests/drop/kwin.sh level=1` و`level=1 shortcut` و`tests/overlay/kwin.sh` (الافتراضي و`drop` و`outputs`) و`window_path.sh` و`budget.sh 20 1` و`budget.sh 20 1 hold` و`hold_docs.sh`؛ وبوابات CLAUDE.md؛ وصفر طفرة ناجية (`--in-place`، ‏`CARGO_BUILD_JOBS=1`، ‏`--in-diff` على `lib.rs` و`dnd.rs` إن تغيّر)؛ وصفر شرطة طويلة.

## حالات الاختبار
| معيار | الاختبار | النوع |
| --- | --- | --- |
| AC1، AC2 | التسلسل في `repro.sh` و`hold_cycles`: الهدم، وغياب WEBP الأول، ثم `drop cells:` و`hive reveal: 15` و`accept` وWEBP الثاني؛ وتشغيل الفشل على البناء المعكوس (القرار 3) | تكامل |
| AC3، AC8 | `drop_hive_tests.rs` و`drop_tests.rs`: قراءة `opened` في كل انتقال بعد إيقاف `opener`، ومع `SBARHIVE_REDUCED_MOTION=1`؛ وعدّ `replaced` و`on_dismiss` | وحدة |
| AC4 | عدّ `attach` على C في الأثر، خدمتان (بالحركة ودونها) | تكامل |
| AC5 | `reveal_budget` القائم بالعدد الجديد للظهورات | أداء |
| AC6 | `snap_check` القائم، وفرق طابع اللقطة عن أول `attach` | لقطة |
| AC7 | زوج الظهورين وعدّ الأسطر وترتيبها وصفر `attach` | تكامل |
| AC9، AC10 | `hold_docs.sh`، وفحص أطوال أسطر README خارج كتل الشيفرة | ثابت |
| AC11 | السكربتات كلها مرة واحدة، والبوابات، والطفرات | تكامل، وحدة |

تأكيدات تتغير (يعدّلها المختبِر وحده):
- `tests/drop/layer.sh:305` و`307` (تعبير الأسباب) والتعليقان 277 و326: يضاف `replaced`.
- `tests/drop/hold_docs.sh:45` و`87` و`158`: ستة أسباب، وفحص سطر S-022.
- `tests/hyprland/repro.sh:579-581` (التعليق يصف العلة مفتوحة) وتعليقات الرأس عند الأسباب؛ و`tests/drop/README.md:220`.
- المجاميع: `repro.sh` ‏40 و47، و`tests/drop/kwin.sh level=1` ‏339، تصير ما يثبّته المختبِر.
- عدد الظهورات المكشوفة في S-021 AC8: ‏`reveal_budget ... 4` في `repro.sh:683-686`، والظهورات الثلاثة عشر في `tests/drop/kwin.sh`، يزيدان بظهورات AC1 وAC2 وAC4.
- S-021 AC2 (كل خروج من `Probing` يستدعي `on_dismiss` مرة): يستثنى الاستبدال (القرار 8). يتأكد المختبِر أن لا اختبار `*_s021a_*` يفحص الاستبدال.

وللمنفِّذ: `crates/ui/src/layer.rs:186-194` و`docs/architecture.md:242`.

## خارج النطاق
- `release()` لجهاز من الإصدار 1، وإعادة `URI_LIST_OFFERED` (ملاحظتا S-021b (ب) و(ج)).
- مقياس `hive_bench --drop` على مسار النافذة، وتركيز GNOME وKDE الحقيقيين، وX11.
- لف ملفات غير `tests/drop/README.md`.

## سجل المراجعة
- 2026-10-22، المخطِّط: كُتبت القصة من موجز المدير وسجل S-021. القرارات العشرة للمدير بتفويض محمد، ولا أسئلة مفتوحة. الأعمال الثقيلة داخل `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200%` بـ `CARGO_BUILD_JOBS` 1 أو 2، ومؤلِّف خاص واحد في كل مرة (Hyprland متداخل من `repro.sh` وKWin ‏`--virtual`)، لا سطح المكتب الحي. ‏`cargo mutants` دائماً بـ `--in-place`. الحالة «مسودة».
- 2026-10-22، المدير بتفويض محمد (طلب ألا يُسأل وأن يُكمل العمل): القصة معتمدة بقرارات المخطِّط العشرة كما هي. وفي القرار 3 يُرجع المختبِر تغيير `receiver.rs` مؤقتاً في شجرة العمل فقط، ولا يُلزمه أبداً، ويتحقق من `git diff` نظيف قبل الالتزام.
- 2026-10-22، المختبِر: كُتبت الاختبارات من المعايير وجدول الحالات وقائمة «تأكيدات تتغير» وعقد S-021، دون كتابة شيفرة إنتاج. تصحيح لما قرأه المختبِر: قرأ للتعرّف على المنافذ بنية `Inner` وأجسام `Hive::new` و`show()` و`Inner::drag` في `lib.rs`، و`layer::probed` في `layer.rs`، و`motion::detect`، ليعرف أسماء `opener` و`opened` و`reduced_motion`، وأن `show()` يغلق الظهور المعروض بـ `close()` قبل الجديد، وأن تقليل الحركة يُقرأ مرة في `Hive::new`. ومنه عُرفت أيضاً آلية ربط السبب بالحدث، ولم تُبنَ عليها فحوص غير ما في المعايير. شُغّل كل شيء على main ‏(485351b، `cargo build --release` جديد) داخل `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200%` بـ `CARGO_BUILD_JOBS=2`، وكل مؤلِّف خاص وحده (Hyprland 0.56.2 متداخل في KWin ‏`--virtual`، ثم KWin ‏`--virtual`)، بـ HOME مؤقت، لا سطح المكتب الحي.
  - الملفات: `crates/ui/src/drop_hive_tests.rs` (اختباران: `ac3_s022_the_opening_animation_starts_at_the_reveal` و`ac8_s022_a_new_show_over_a_probing_one_is_replaced_without_dismissal`)، و`tests/drop/layer.sh` (السبب `replaced` في التعبيرين والتعليقين، والدوال `attach_burst` و`again_checks` و`replace_checks`)، و`tests/hyprland/repro.sh` (AC1 وAC4 وAC7 وAC5، و`start_daemon [WL] [RM]`، و`-u SBARHIVE_REDUCED_MOTION` في البيئة الخاصة، والتعليقات)، و`tests/drop/kwin.sh` (AC2 في `hold_cycles` بعد hold-over، وAC6 ‏`shot_times`، وAC7 ‏`replace_pair`، و`reveal_budget` بعد `hold_cycles`)، و`tests/drop/hold_docs.sh` (AC9 وAC10)، و`tests/drop/README.md` (اللف وقسم S-022).
  - العقد الذي تثبّته الاختبارات (لا يُعدَّل):
    - السبب الجديد `replaced`: السطر `hive probe: hidden (replaced)`، والتعبير في `layer.sh` صار `^hive probe: hidden \((pointer|no-uri-list|no-actions|release|cap|replaced)\)$`. يُطبع للظهور المستبدَل أثناء صنع الظهور الجديد، فيقرؤه فحص واحد على مقطع يشمل الظهورين (`replace_checks`)، لا تقسيم لكل ظهور: بالضبط سطرا فحص في المقطع، `hidden (replaced)` ثم `hidden (pointer)`، وسطر الاستبدال بعد أول سطر `hive surface:` وقبل الثاني، واثنان `get_layer_surface`، وصفر `attach(wl_buffer` على كل سطح فرعي من `get_subsurface` في المقطع.
    - `Inner::replace(&self) -> Option<&'static str>` في `lib.rs`: ينهي الظهور المعروض لظهور جديد ويعيد سبب الظهور المستبدَل: `Some("replaced")` إن كان `Probing`، و`None` إن كان `Waiting` أو `Over` أو `Normal` أو مخفياً. لا يستدعي `on_dismiss` أبداً (القرار 8). يمرّ به `show()` حين يكون الظهور معروضاً، ويطبع `layer.rs` السطر لـ `Some`. لم يُثبَّت شيء في `dnd.rs`، فالقرار في `lib.rs` تحت الطفرات (القرار 9)، ولا اختبار جديد في `drop_tests.rs`.
    - الحركة (AC3): `opened` (‏`get_opened` و`set_opened`) هو الخاصية التي تتحرك إليها حركة الفتح، و`Inner::opener` المؤقت الذي يضعها بعد الربط كما اليوم. تضع الاختبارات الظهور في `Probing` ثم توقف `opener` وتضع `opened` خطأ. بعدها لا يجعله صحيحاً إلا الكشف (`Probing` إلى `Waiting` في `Inner::drag`): بالحركة خلال دورة حلقة واحدة (`mock_elapsed_time` بـ 1 ms)، ومع تقليل الحركة عند الكشف نفسه. و`Waiting` لاحق (السحب يغادر خلية) لا يفتحه ثانية. ويبقى خطأ بعد `Tick` عند 9.9 ثانية، وبعد `Reject` و`PointerEnter`. الظهور العادي وظهور مسار النافذة: صحيح بعد مؤقت الربط.
    - تقليل الحركة في اختبار الوحدة: يقرؤه `Hive::new` مرة من `SBARHIVE_REDUCED_MOTION`، فاختبار AC3 الواحد يبني الخليتين (`0` و`1`) ويضبط المتغير ثم يزيله فوراً (`unsafe` في إصدار 2024، والمتغير مشترك بين خيوط الاختبار).
    - AC4: «عدد `attach`» = طلبات `-> wl_surface#C.attach(wl_buffer` على C (أول `wl_surface` في أول `get_subsurface` في مقطع الظهور) بطابع أثر لا يزيد على 200 ms بعد أولها، وأولها معها (`attach_burst`). ‏4 أو أكثر في الخدمة الأساسية (بـ `SBARHIVE_REDUCED_MOTION=0` صريحاً)، و1 أو 2 في خدمة بـ `SBARHIVE_REDUCED_MOTION=1`، ثم تُعاد الخدمة بالحركة.
    - AC1 وAC2 (`again_checks`): في مقطع الظهور الثاني بالضبط سطر `drop cells:` واحد، و`hive reveal: N` وN عدد أسطر `sbarhive actions` (15)، و`wl_data_offer.accept(SERIAL, "...")` واحد على الأقل، وWEBP الملف الثاني `image/webp`، ومعها `probe_checks ... reveal`. والهدم خلال 500 ms من طابع UTC قبل `drop --release` إلى أول `zwlr_layer_surface_v1.destroy` بعده.
    - AC6 (`shot_times`): طابع UTC قبل بدء `shot.py` مباشرة (أبكر من الالتقاط، فالحد أشد) ناقص أول `attach` على C في الظهور 1 (لقطة `over`) والظهور 13 (لقطة `thumb`)، 300 ms أو أكثر.
    - AC9: في `docs/architecture.md`، في القسم نفسه الذي يصف الفحص، الأسباب الستة كلمات، وعبارة `open` أو `opening` ثم `animation`، و`reveal` قبلها أو بعدها في حدود 120 حرفاً (فواصل الأسطر مشمولة). وسطر يسمّي `S-022` بالعربية والإنكليزية تحت `## [Unreleased]`. وصار فحص S-021 ‏AC11 «سطر S-021 في CHANGELOG.md» دون قيد القسم، لأن إصدار 0.3.1 نقله من `[Unreleased]`، فكان يفشل على main قبل هذه القصة. جُرّب على نسخة مؤقتة (حُذفت) نصٌّ ينجح: إضافة «and `replaced` (a new show came first)» إلى بند Hide، و«The opening animation starts at the reveal, not at the bind.» إلى بند Reveal، وسطر S-022 تحت `[Unreleased]`.
    - AC10: صفر سطر يتجاوز 80 حرفاً خارج أسوار ``` في `tests/drop/README.md`، فحص في `hold_docs.sh`.
  - تأكيدات تغيّرت (كما في القائمة): `layer.sh` (التعبيران والتعليقان)، و`hold_docs.sh` (ستة أسباب، وسطر S-022، وفحص S-021 ‏AC11 دون قيد القسم)، و`repro.sh` (التعليق القديم عند AC9 (3) يحيل إلى AC1، وتعليق الرأس)، و`tests/drop/README.md` (قسم S-021)، وعدد الظهورات المكشوفة في `reveal_budget`: ‏`repro.sh` من 4 إلى 8 (‏AC9 (3) وظهورا AC1 وظهورا AC4 و`cell` و`timeout` و`esc`)، و`tests/drop/kwin.sh` من 13 إلى 15 (ومعها ظهور hold-over وظهور AC2)، وصار فحصه بعد `hold_cycles`. لا اختبار `*_s021a_*` يعرض ظهوراً فوق ظهور في `Probing`: كلها تعرض بعد إخفاء، أو فوق `Waiting` (‏`ac1_s021a_the_window_path_is_unchanged`).
  - لف `tests/drop/README.md` (القرار 10): أُعيد لفّ الفقرات الأربع التي فيها الأسطر 82 و104 و179 و191 على 80 عموداً، دون تغيير كلمة (الفرق بعد ضمّ المسافات فارغ)، والأسوار كما هي بايتاً بايتاً (`diff` لمحتوى الكتل مع HEAD فارغ).
  - المجاميع الجديدة لـ AC11: `repro.sh --json` ‏55 (كانت 40)، و`--json --quasimode` ‏62 (كانت 47)، و`tests/drop/kwin.sh level=1` ‏354 (كانت 339)، و`tests/overlay/kwin.sh drop` ‏49 دون تغيير، و`hold_docs.sh` ‏19 (كانت 16). وضع `window` في `repro.sh` دون تغيير (11، و18 مع `--quasimode`).
  - مخرجات الفشل (بناء main):
    ```
    $ cargo test -p sbarhive-ui --lib
    error[E0599]: no method named `replace` found for struct `std::rc::Rc<Inner>` in the current scope
       --> crates/ui/src/drop_hive_tests.rs:772:27   (796 بعد rustfmt)
    (6 أخطاء، كلها `Inner::replace`، ولا خطأ غيرها)
    $ (نسخة مؤقتة دون اختبار AC8، أُعيدت) cargo test -p sbarhive-ui --lib
    test drop_hive_tests::ac3_s022_the_opening_animation_starts_at_the_reveal ... FAILED
    panicked at crates/ui/src/drop_hive_tests.rs:733:5   assert!(opened(&hive), "the reveal starts the opening animation")
    test result: FAILED. 140 passed; 1 failed
    (وبتحييد ذلك التأكيد وحده في نسخة مؤقتة: نجحت الحراس كلها، وفشل التأكيد التالي
     assert!(opened(&calm), "reduced motion: opened at the reveal"))
    $ tests/hyprland/repro.sh --json
    {"total": 55, "failed": 3, "skipped": 0}
    FAIL S-022 AC7 pointer at (40, 40), drop --at 500,300 twice, then a move there: exactly two probe lines, "hive probe: hidden (replaced)" then "hive probe: hidden (pointer)"
    FAIL S-022 AC7 ...: the "replaced" line comes between the two shows' "hive surface:" lines, after two get_layer_surface in all
      (INFO: probe lines hive probe: hidden (pointer)|, hive surface lines at 48 and 110 (of 2), get_layer_surface x2, attach on them 0)
    FAIL S-022 AC4: a revealed drop show with motion draws its opening animation: 4 or more attach on the hive subsurface in the 200 ms from its first
      (INFO: drop reveal with motion 2, with SBARHIVE_REDUCED_MOTION=1 1; calibration, a normal show with motion 2)
    $ tests/hyprland/repro.sh --json --quasimode
    {"total": 62, "failed": 3, "skipped": 0}   (الثلاثة نفسها؛ AC4: 2 و1، والمعايرة 2)
    $ tests/drop/kwin.sh level=1
    RESULT FAIL (2)   (352 PASS)
    FAIL S-022 AC7 pointer at (100, 700), drop --at 640,400 twice, then a move there: exactly two probe lines, ...
    FAIL S-022 AC7 ...: the "replaced" line comes between the two shows' "hive surface:" lines, ...
      (INFO: probe lines hive probe: hidden (pointer)|, hive surface lines at 38 and 98 (of 2))
    $ tests/overlay/kwin.sh drop
    RESULT PASS   (49)
    $ tests/drop/hold_docs.sh
    FAIL S-021 AC12, S-022 AC9: docs/architecture.md, one section describes the probe: ... the six hide reasons (pointer, no-uri-list, no-actions, release, cap, replaced)
    FAIL S-022 AC9: docs/architecture.md, that section also says the opening animation starts at the reveal
    FAIL S-022 AC9: CHANGELOG.md has a line under [Unreleased] that names S-022 in English and in Arabic
    FAIL S-022 AC10: no prose line of tests/drop/README.md is longer than 80 columns (outside code fences)
      (INFO: prose lines over 80 columns: 82:102 104:81 179:108 191:92; قبل اللف)
    RESULT FAIL (4)   (بعد لف README: RESULT FAIL (3)، والثلاثة للمنفِّذ)
    ```
  - ما ينجح قبل التنفيذ ولماذا: AC1 وAC2 كاملين على main (إصلاح جهاز البيانات في S-021b): ‏`repro.sh` الهدم بعد 3.8 ms من `drop --release`، ولا WEBP للملف الأول، والسحب الثاني `drop cells:` واحد و`hive reveal: 15` و`accept` واحد وWEBP؛ و`kwin.sh` كذلك (`held again.webp`). ‏AC5: ‏`repro.sh` ثمانية من ثمانية (أسوأها 9.7 ms، و`--quasimode` ‏8.2)، و`kwin.sh` خمسة عشر من خمسة عشر (أسوأها 8.0 ms). ‏AC6: لقطة `over` بعد 1341.2 ms من أول `attach`، و`thumb` بعد 1747.2 ms، والفرق عن المرجعين 0.000000. ‏AC4 مع تقليل الحركة: 1. ‏AC7: صفر `attach` في الظهورين. وحراس AC3 (‏9.9 ثانية، و`Reject`، و`PointerEnter`، والظهور العادي، ومسار النافذة، ولا فتح ثانٍ) وحارس AC8 (لا `on_dismiss` عند الاستبدال عبر `present_drop_hold`). كلها حراس سلوك قائم يجب ألا يتغير.
  - القرار 3 (البناء المعكوس، شجرة العمل فقط، لم يُلزَم): أُرجع شرط الجهاز في `receiver.rs:475` إلى `if !s.on_us {` (الحقل `device` باقٍ فلا تتغير الترجمة)، و`cargo build --release`، ثم: `repro.sh --json` ‏`{"total": 55, "failed": 10}`، ومنها AC1: «second drag: hive probe: hidden (pointer)|، drop cells lines 0، hive reveal none، accept with a type 0، second.webp لم يُكتب»، فخمسة فحوص للسحب الثاني و`reveal_budget` (ثمانية) و`hidden (no-uri-list)` في سحب النص (S-021 AC6، سبب الإصلاح الأصلي). و`kwin.sh level=1` ‏RESULT FAIL (8) بـ 346 PASS، ومنها AC2 بالأعراض نفسها (`held again.webp` لم يُكتب) و`reveal_budget` (خمسة عشر). فالعلة تظهر حمراء في الفحصين، وAC1 وAC2 ليسا حارسين فقط. ثم `git checkout -- crates/ui/src/receiver.rs` وبناء جديد؛ `git diff crates/` لا يسرد إلا `crates/ui/src/drop_hive_tests.rs`.
  - ملاحظة للمنفِّذ والمدير (AC4، لا تغيّر في الفحص): سطر المعايرة يقول إن الظهور العادي بالحركة على Hyprland المتداخل يعطي `attach` اثنين فقط في 200 ms (عند 0 و2 ms)، أي أن حركة الفتح لا تُرسم إطارات في أثر مسار الطبقة حتى في الظهور العادي اليوم، بينما حركة المرور فوق الخلايا ترسم إطاراً كل 16 إلى 17 ms (ظهور AC9 (3): ‏18 `attach`). فإن بقي العدد 2 بعد نقل `opener` إلى الكشف، فالسبب أن الحركة تنقضي قبل أول إطار أو أن المحوّل لا يعيد الرسم لها، والحد 4 من القصة فلم يُخفَّض. قد يحجب هذا المنفِّذ: القرار 4 يضع الإصلاح في `lib.rs` ويستثني `layer.rs`، فإن لم يكفِ نقل `opener` فعلى المدير أن يقرر مسبقاً هل يُسمح بتغيير في `layer.rs` (إعادة الرسم أثناء الحركة) أو يُراجَع AC4.
  - لم يُشغَّل: `cargo clippy` و`cargo mutants` (اختبارات الوحدة لا تُبنى حتى يوجد `Inner::replace`)، و`tests/drop/kwin.sh level=1 shortcut` و`tests/overlay/kwin.sh` (الافتراضي و`outputs`) و`window_path.sh` و`budget.sh`، لأن فحوصها لم تتغير؛ للمراجِع ضمن AC11. ‏`cargo fmt --check -p sbarhive-ui` نظيف، و`bash -n` نظيف، و`shellcheck` لا يضيف تحذيراً على ما كان، ولا شرطة طويلة في الملفات المتغيرة.
- 2026-10-22، المدير بتفويض محمد، في خطر AC4: يبقى الحد 4 أو أكثر ولا يُعدَّل المعيار. ويُسمح للمنفِّذ بتغيير إعادة الرسم في `crates/ui/src/layer.rs` إن احتاجه حتى تُرسم إطارات الحركة الوسيطة على مسار الطبقة (ظهور عادي وظهور إفلات بعد الكشف)، مع بقاء قرار بدء الحركة نفسه في `lib.rs` (القرار 4). `layer.rs` مستثنى من الطفرات، ويحرسه فحص AC4 في الأثر.
- 2026-10-22، المنفِّذ: كُتبت الشيفرة حتى نجحت اختبارات c462b42 دون تعديل أي اختبار. كل عمل ثقيل داخل `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200%` بـ `CARGO_BUILD_JOBS=2` (والطفرات بـ 1)، ومؤلِّف خاص واحد في كل مرة.
  - `crates/ui/src/lib.rs`: ‏`Inner::replace(&self) -> Option<&'static str>` يغلق الظهور المعروض دون `on_dismiss` ويعيد `Some("replaced")` لظهور في `Probing` فقط، ويمرّ به `show()` ثم يعطي السبب لـ `layer::replaced`. ‏`Inner::open` يبدأ حركة الفتح: مع تقليل الحركة يضع `opened` فوراً، وإلا بعد الإطار الأول (مؤقت `opener` الصفري على مسار النافذة، و`layer::after_drawn` على مسار الطبقة). ‏`show()` يضع `opened` خطأ دائماً ولا يستدعي `open` ما دامت الآلة `Probing`، و`Inner::drag` يستدعيه عند الانتقال من `Probing` إلى `Waiting` وحده. فقرار البدء كله في `lib.rs` (القرار 4).
  - `crates/ui/src/layer.rs` (بقرار المدير في AC4): ‏`replaced` يطبع `hive probe: hidden (replaced)`، و`after_drawn` يحفظ بدء الحركة فيشغّله `render` بعد أول إطار مرسوم، ويسقطه `unmap` إن أُغلق الظهور قبله.
  - سبب AC4: كان `opener` الصفري يعمل قبل أول إطار للمحوّل (في الظهور العادي قبل configure، وعند الكشف قبل مؤقت إعادة الرسم)، فأول قيمة يقرؤها Slint للخاصية `transform-scale` هي النهائية ولا يُحرَّك شيء. ثبت بأسطر تتبّع مؤقتة (حُذفت): «opener fired» قبل أول render، و`has_active_animations` خطأ بعده. بعد الإصلاح يعيد متتبّع إعادة الرسم في Slint الرسم مع كل callback طوال الحركة، فلم يلزم تغيير آخر في الجدولة. أثر جانبي مقصود: الظهور العادي على مسار الطبقة صار يرسم حركته أيضاً (المعايرة من 2 إلى 12).
  - `docs/architecture.md`: السبب السادس `replaced` واستثناؤه من `on_dismiss`، وجملة «The opening animation starts at the reveal». ‏CHANGELOG.md: سطر S-022 بالعربية والإنكليزية تحت `## [Unreleased]`.
  - النتائج (بناء `cargo build --release` جديد بعد `cargo clean --release` لحزم المشروع): ‏`cargo fmt --check` نظيف؛ `cargo clippy --all-targets -- -D warnings` نظيف (ولا اعتراض على `unsafe` في `drop_hive_tests.rs`)؛ `cargo test --all` ‏565 ناجحة وصفر فاشل (`sbarhive-ui --lib` ‏142)؛ `scripts/design-lint.sh` نظيف؛ `repro.sh --json` ‏`{"total": 55, "failed": 0}` و`--json --quasimode` ‏`{"total": 62, "failed": 0}` (AC4: ‏11 بالحركة، و1 مع تقليل الحركة، والمعايرة 12)؛ `tests/drop/kwin.sh level=1` ‏RESULT PASS ‏354، و`level=1 shortcut` ‏44؛ `tests/overlay/kwin.sh` ‏104، و`drop` ‏49، و`outputs` ‏15؛ `window_path.sh` ‏4؛ `budget.sh 20 1` ‏6 (الوسيط 0.4 ms)، و`budget.sh 20 1 hold` ‏7؛ `hold_docs.sh` ‏19. اللقطات تختلف عن المراجع بـ 0.000000، ولقطتا `over` و`thumb` بعد أول `attach` بـ 1334.5 و1736.2 ms. ‏`cargo mutants --in-place --in-diff` على `lib.rs` بـ `CARGO_BUILD_JOBS=1`: ‏13 طفرة، 13 مقتولة.
