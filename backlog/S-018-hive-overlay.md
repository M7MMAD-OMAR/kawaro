# S-018 الخلية طبقةً فوق الشاشة، والملف نفسه في الخلية المركزية
الحالة: معتمدة

يعتمد على ADR 0004 (`docs/adr/0004-hive-overlay.md`، مقبول في 2026-10-20، الخيار أ). المرجع التنفيذي `spikes/overlay/` (`platform.rs` و`overlay.rs` و`thumb.rs` و`ui/preview.slint`)، ونموذج الاختبارات المخفية `spikes/overlay/kwin.sh` و`gnome.sh`. للإصدار 0.3.0.

## القصة
بصفتي مستخدماً على KDE أو Hyprland، أريد أن تظهر الخلية طبقةً في منتصف الشاشة بلا إطار ولا رصف، لا يغلقها مرور المؤشر، وأن أرى في مركزها الملف الذي اخترته، حتى تبدو الخلية جزءاً من سطح المكتب لا نافذة. وعلى GNOME أو مع قارئ الشاشة تبقى نافذة كما اليوم.

## التقسيم (كل قصة فرعية يوم عمل أو أقل، بالترتيب؛ S-018g مستقلة ويمكن أن توازي S-018a إلى S-018f)
| القصة | المحتوى | المعايير |
| --- | --- | --- |
| S-018a | الأجزاء الصافية: اختيار السطح، الموضع والمقياس، رموز المفاتيح، قيمة مخطط الألوان، شكل المؤشر | AC1 إلى AC3 |
| S-018b | المنصة الرقيقة ومحوّل الطبقة: الماسك والسطح الفرعي، الاختيار عند كل بناء، الرجوع في GNOME، اللقطات، و`tests/overlay/{kwin,gnome}.sh` | AC4 إلى AC7 |
| S-018c | الإدخال والتركيز: النقر، لوحة المفاتيح الحصرية، قواعد الإغلاق، الرمز، سطر التشخيص، قارئ الشاشة | AC8 إلى AC12 |
| S-018d | الإفلات على الاتصال الجديد: ماسك بلا إدخال، المستقبِل على اتصال الطبقة | AC13، AC14 |
| S-018e | مخطط الألوان من البوابة، شكل المؤشر، المقياس الكسري | AC15 إلى AC17 |
| S-018f | الميزانيات، تحرير المخازن، الرخص، التوثيق | AC18 إلى AC20 |
| S-018g | البحث الصافي: رابط GLib، الصورة المصغّرة، نوع MIME، خريطة الأيقونات | AC21 إلى AC23 |
| S-018h | المعاينة في المركز: الصورة، الكومة والعدد، التسمية، اللقطات، الميزانية | AC24 إلى AC27 |

## معايير القبول
- AC1: دالة صافية تختار `Layer` فقط إن أُعلن `zwlr_layer_shell_v1` و`wl_subcompositor` و`wp_viewporter` و`wp_single_pixel_buffer_v1` معاً، وقارئ الشاشة مطفأ، والجلسة Wayland. وإلا `Window`. `SBARHIVE_SURFACE=layer|window` يفرض الاختيار، و`layer` دون العوامّ يعطي `Window` مع سطر سجل. قائمتا عوامّ KWin 6.7 وGNOME 50 في `tests/fixtures/overlay/` (يولّدهما المختبِر من تشغيل مخفي، فملفات `spikes/overlay/out/*.txt` غير مُودَعة) تعطيان `Layer` و`Window`.
- AC2: دالة صافية للموضع والمقياس، لشاشة منطقية W × H وخلية w × h بالبكسل المنطقي الصحيح (اليوم 393 × 362 لـ `Layout::extent` 392 × 361.3 مقرّباً للأعلى، كما في S-017 AC13) ومقياس s/120: السطح الفرعي في `(round((W-w)/2), round((H-h)/2))`، والمخزن `ceil(w·s/120) × ceil(h·s/120)`، و`set_destination(w, h)`. صحيحة في (1280×800، 120)، (1280×800، 180)، (1920×1080، 240)، (3840×2160، 150).
- AC3: رموز evdev: 1 Esc، و28 و96 Enter، و57 مسافة، و15 Tab، و103 و108 و105 و106 الأسهم، وكل رمز آخر لا شيء. قيمة البوابة `color-scheme`: 1 داكن، 2 فاتح، 0 أو غياب أو خطأ فاتح. `SBARHIVE_COLOR_SCHEME` يسبق البوابة. شكل المؤشر يُطلب فقط إن أُعلن `wp_cursor_shape_manager_v1`.
- AC4: في KWin 6.7 المخفي (1280×800، مقياس 1) ينشئ كل ظهور في أثر `WAYLAND_DEBUG=1` للخدمة: `get_layer_surface` واحداً على الطبقة 3 باسم `sbarhive`، المثبتات 15، `exclusive_zone` -1، لوحة المفاتيح 1، و`get_subsurface` واحداً، وصفر `get_xdg_surface` للخلية. صندوق بكسلات الخلية غير الشفافة في لقطة ScreenShot2 في المنتصف بفرق 1 px أو أقل، بلا شريط عنوان.
- AC5: الاختيار يغطي كل بناء لمكوّن الخلية: خمسة ظهورات متتالية (بلا رمز، برمز، `present_drop`، بعد تغيير اللغة، بعد تغيير السمة) كلٌّ منها `get_layer_surface` وصفر `get_xdg_surface` للخلية.
- AC6: في GNOME Shell 50 المخفي، وفي KWin مع `SBARHIVE_SURFACE=window`: الخلية `get_xdg_surface` كما اليوم، وصفر `get_layer_surface`، ولا خطأ بروتوكول في السجل.
- AC7: `crates/ui/tests/snapshots.sh`: المراجع الأربعة `hive-*.png` مطابقة (0.1 % أو أقل) بـ `SBARHIVE_SURFACE=layer` وبـ `window`، و`take_snapshot` يعمل على محوّل الطبقة. `settings_snapshots.sh` مطابق دون تغيير.
- AC8: على مسار الطبقة: نقرة على خلية الحلقة الأولى يمين المركز تستدعي `on_choose` بمعرّفها. نقرة في (100، 700) على الماسك تغلق الخلية خلال 500 ms وتستدعي `on_dismiss` مرة واحدة، ونافذة GTK تحتها تستقبل 0 نقرات.
- AC9: لوحة المفاتيح حصرية: والمؤشر فوق نافذة GTK، السهم الأيمن ثم Enter يختاران المعرّف نفسه الذي يعطيه `ui_tests` للتسلسل نفسه. بعد تجوال المؤشر على 5 نقاط منها نافذة GTK، يغلق Esc الخلية (KWin، يتبع المؤشر، Low). فتح الإعدادات والخلية ظاهرة لا يغلقها. على مسار النافذة يبقى فقدان التركيز يغلقها (S-001 AC4، S-014 AC3).
- AC10: `sbarhive open --token T` يرسل `xdg_activation_v1.activate("T", <الماسك>)` في كل ظهور من 3 متتالية، والرمز لا يبقى معلّقاً بعدها (S-013 AC6). في KWin مع الرمز الحقيقي من `tests/focus/client.py` بالمستويين 1 و4 تأخذ الخلية التركيز (`hive focus: true`). بالمستوى 4 والمؤشر ساكن، دون رمز ومع رمز مختلَق، لا تأخذه (S-013 AC4). بالمستوى 1 دون رمز تأخذه خلال 500 ms من الظهور. المستويان 2 و3 دون رمز يُقاسان ويُسجَّلان في جدول (مُفترَضان في ADR)، واختلافهما عن المستوى 1 قرار لمحمد لا سبب REWORK.
- AC11: مع `SBARHIVE_DEBUG_FOCUS` يطبع مسار الطبقة `hive focus: true|false` عند `wl_keyboard.enter|leave` بالصيغة الحالية، ويطبع كل ظهور على المسارين `hive surface: layer|window X Y W H` بإحداثيات الشاشة المنطقية.
- AC12: قارئ الشاشة: خدمة `org.a11y.Bus` وهمية على الناقل الخاص بـ `ScreenReaderEnabled=true` تجعل الظهور التالي في KWin `get_xdg_surface` وصفر `get_layer_surface`. وبعد `PropertiesChanged` إلى `false` يعود الظهور التالي طبقةً.
- AC13: وضع الإفلات على مسار الطبقة: الماسك بمنطقة إدخال فارغة ولوحة مفاتيح 0. نقرة في (100، 700) تصل نافذة GTK (نقرة واحدة) والخلية باقية. `get_data_device` على اتصال الطبقة بعد `get_layer_surface` في كل ظهور، والجهاز السابق يُحرَّر قبله.
- AC14: كل فحوص `tests/drop/kwin.sh level=1` الموجودة عند بدء S-018d `RESULT PASS` على مسار الطبقة، و`tests/drop/gnome.sh smart` `RESULT PASS` على مسار النافذة.
- AC15: بوابة `Settings` وهمية على الناقل الخاص: القيمة 1 تجعل بكسل المركز `surface-3` الداكن من `design/tokens.json`، و2 الفاتح، و`SettingChanged` إلى 2 والخلية مخفية يجعل الظهور التالي فاتحاً.
- AC16: في KWin كل `wl_pointer.enter` على الماسك أو الخلية يتبعه `wp_cursor_shape_device_v1.set_shape(<serial>, 1)` في الأثر.
- AC17: بمقياس 1.5 (`kscreen-doctor`) يصل `preferred_scale` 180 قبل أول `configure`، ومخزن الخلية بحجم AC2، وعرض صندوقها في اللقطة يساويه بفرق 1 px.
- AC18: `crates/ui/tests/measure.sh 20 60` في KWin (مسار الطبقة، بلا رمز ومع `SHOW_TOKEN`): وسيط أول إطار أقل من 150 ms، وRSS بعد 60 ثانية خمول 40 MB أو أقل. كل فحوص `tests/e2e/run.sh` ناجحة، ومنها فحصا الذاكرة 40 MB أو أقل. عند كل إخفاء يُرسل `wl_buffer.destroy` و`wl_shm_pool.destroy` و`zwlr_layer_surface_v1.destroy` لمخازن ذلك الظهور.
- AC19: `i-slint-backend-winit` و`i-slint-core` بالإصدار `=1.18.1`، و`md-5`، و`png` مسجّلة في `docs/engines.md` بالإصدار والرخصة. `cargo tree -d` لا يضيف نسخة مكررة عمّا في `main`، وكل حزمة جديدة في `Cargo.lock` مسجّلة.
- AC20: `docs/architecture.md` يصف المسارين وقاعدة الاختيار. التوثيق الموجّه للمستخدم بالعربية والإنكليزية فيه قاعدة طبقة Hyprland لإلغاء حركة `sbarhive` موسومة «لم يُتحقق منها». `tests/overlay/README.md` يشغّل الكل بأمر واحد.
- AC21: رابط الملف يطابق `Gio.File.get_uri()` بايتاً ببايت لثمانية أسماء: عربي، مسافة، `;`، `#`، `?`، `[]`، `صورة 1+&~,(x).png`، `%`. والمسار `$XDG_CACHE_HOME/thumbnails/<الحجم>/MD5(الرابط).png`.
- AC22: تُقبل الصورة المصغّرة فقط إن ساوى `Thumb::URI` الرابطَ و`Thumb::MTime` زمن التعديل بالثواني، و`Thumb::Size` إن وُجد. الترتيب `large` ثم `x-large` ثم `xx-large` ثم `normal`. وجود الملف في `fail/` يعني أيقونة النوع. الاسم بـ `;` يجرّب صيغة Qt الحرفية أيضاً، والرابط الرمزي يجرّب هدفه أيضاً. بعد `touch` تُرفض. لا يُكتب أي ملف تحت `thumbnails/`.
- AC23: نوع MIME من `globs2` (`XDG_DATA_HOME` ثم `XDG_DATA_DIRS`، الوزن الأعلى ثم النمط الأطول، وعلامة `cs`). الخريطة: `image/*` image، `video/*` video، `audio/*` audio، `application/pdf` pdf، مستندات النصوص document، الجداول وCSV sheet، العروض slides، `text/*` text، وغيرها `file` (أيقونة جديدة بأسلوب المجموعة). رمز الصيغة الامتداد بحروف كبيرة، 4 أحرف أو أقل. لعدة ملفات: الفئة المشتركة وإلا `file`. لا يُفتح أي ملف تحت `/usr/share/icons`.
- AC24: دون خلية تحت المؤشر يعرض المركز الصورة أو الأيقونة في المربع المحاط بسداسيه بـ `image-fit: contain`، وعند المرور يعرض الاسم كما اليوم. `Layout::extent` وكل اختبارات `geometry.rs` و`layout.rs` دون تغيير. الصورة تُصغَّر قبل Slint إلى ضلع المربع الفيزيائي أو أقل، وتُحرَّر عند الإخفاء.
- AC25: لعدة ملفات: سداسيتان خلف المركز مزاحتان (معكوستان بالعربية) وشارة بالعدد، بألوان الرموز، ولا بكسل منها خارج حدود الخلية. التسمية الوصفية للمركز: اسم الملف لملف واحد، ولعدة ملفات نص `text::selection` الحالي في `crates/daemon/src/text.rs` بصيغ العدد العربية القائمة (2 «ملفان»، 3 «3 ملفات»، 12 «12 ملفاً»).
- AC26: أربع لقطات جديدة: `hive-en-dark-thumb.png` و`hive-ar-light-icon.png` (PDF) و`hive-en-light-stack.png` و`hive-ar-dark-stack.png` (3 ملفات)، يعتمدها محمد قبل الإيداع. المراجع الأربعة القديمة دون تغيير.
- AC27: مع ملف له صورة `xx-large` (1024 px): وسيط أول إطار في `measure.sh` أقل من 150 ms، وRSS الخدمة بعد 20 ظهوراً بها 40 MB أو أقل (فحص جديد في `tests/e2e/run.sh`).

## حالات الاختبار
| معيار | الاختبار | النوع |
| --- | --- | --- |
| AC1 إلى AC3 | وحدة بالحالات المذكورة في كل معيار | وحدة |
| AC4، AC5 | `tests/overlay/kwin.sh`: عدّ الطلبات في الأثر لكل ظهور، ولقطة ScreenShot2 وصندوقها | تكامل، لقطة |
| AC6 | `tests/overlay/gnome.sh`، و`tests/overlay/kwin.sh window` | تكامل |
| AC7 | `snapshots.sh` مرتين بقيمتي `SBARHIVE_SURFACE`، و`settings_snapshots.sh` | لقطة |
| AC8، AC9 | `tests/overlay/kwin.sh` مع نافذة GTK (`other.py`) وإدخال `org_kde_kwin_fake_input`: عدّ النقرات، سجل `on_choose` و`on_dismiss`، `sbarhive settings` والخلية ظاهرة | تكامل |
| AC10 | `tests/focus/kwin.sh token notoken bogus level=1..4` وعدّ `activate(` وأسطر `hive focus`؛ وEsc بعد الرمز في `tests/overlay/kwin.sh token` بالإدخال المحقون (`tests/focus/kwin.sh` لا يحقن إدخالاً) | تكامل |
| AC11 | `tests/overlay/kwin.sh` و`gnome.sh`: أسطر التشخيص تقارَن بالأثر واللقطة | تكامل |
| AC12 | `tests/overlay/kwin.sh a11y` بخدمة Python وهمية | تكامل |
| AC13، AC14 | `tests/overlay/kwin.sh drop`، و`tests/drop/kwin.sh level=1` و`gnome.sh smart` | تكامل |
| AC15 | `tests/overlay/kwin.sh scheme` ببوابة وهمية وبكسل المركز | تكامل |
| AC16، AC17 | `tests/overlay/kwin.sh`، و`tests/overlay/kwin.sh 1.5` | تكامل |
| AC18 | `measure.sh`، `tests/e2e/run.sh`، وعدّ طلبات الهدم في الأثر | أداء |
| AC19، AC20 | `cargo tree -d` قبل وبعد، ومراجعة `docs/` | تكامل |
| AC21 إلى AC23 | وحدة: قيم `get_uri()` محفوظة في الاختبار، ذاكرة صور مؤقتة بقطع `tEXt`، `globs2` في `tests/fixtures/mime/`؛ `strace -e openat` في `tests/overlay/kwin.sh` | وحدة، تكامل |
| AC24، AC25 | `ui_tests`: المعاينة والاسم عند المرور، أبعاد الصورة، الإزاحة بالعربية؛ وحدة على `text.rs` | وحدة |
| AC26 | `snapshots.sh` بالمراجع الثمانية | لقطة |
| AC27 | `measure.sh` مع ملف معاينة، والفحص الجديد في `tests/e2e/run.sh` | أداء |

اختبارات قائمة تبقى خضراء: `cargo test --workspace` كله (ومنه `geometry`، `layout`، `ui_tests`، `drop_tests`، `drop_hive_tests`، `uri_list`، `ipc_drop`، `cli.rs`)؛ `snapshots.sh` و`settings_snapshots.sh`؛ `measure.sh`؛ `tests/focus/gnome.sh` (smart وstrict) و`tests/focus/kwin.sh` (level=1 و4)؛ `tests/drop/kwin.sh` و`gnome.sh smart`؛ `tests/e2e/run.sh`؛ `tests/integrations/run_all.sh`. يعدّل المختبِر قبل التنفيذ سطور تعريف السطح فقط، والتأكيدات كما هي: `tests/focus/kwin.sh:42-47` (السطح الطبقي ليس `normalWindow`، فالتركيز من سطر `hive focus`) وإغلاق خلية التسخين بـ Esc صراحةً بعد السطر 70، و`tests/drop/lib.sh:157-163` (`get_layer_surface` سطحاً للظهور)، و`tests/drop/kwin.sh:51` (الموقع من سطر `hive surface`)، و`tests/e2e/run.sh` بعد السطر 64 (إغلاق صريح للخلية، لأن فقدان التركيز لم يعد يغلقها، فلا يبقى الماسك ومخازنه حتى فحص الذاكرة في السطر 105). كل تعديل يُسجَّل هنا بسطره.

## خارج النطاق
- Hyprland بالتشغيل (الموضع، بلا إطار، التركيز مع «يتبع المؤشر»، الإفلات، قاعدة إلغاء الحركة): يتحقق منه محمد على جهازه.
- Sway (غير مثبت)، والشاشات المتعددة (`output = None`، المؤلِّف يختار)، وجهاز ضعيف.
- KWin بالمستوى Extreme دون رمز: حدّ موثق، التركيز بعد نحو 300 ms من أول حركة للمؤشر.
- Nautilus وDolphin الحقيقيان مصدراً للرمز.
- X11 وويندوز، وتوليد الصور المصغّرة بأنفسنا، وaccesskit على محوّل الطبقة، وتكبير الخلية المركزية، والزجاج والضبابية (S-017).

## سجل المراجعة
- 2026-10-20، المخطِّط: ADR 0004 «مقبول» في ملفه، ويذكر المدير أن محمداً فوّض قبوله. الحالة «مسودة» لأن اعتماد القصة لا يُسجَّل إلا من محمد أو بتفويضه عبر المدير، كما في S-014 وS-017. الجهد نحو 7 أيام مقابل 5.5 إلى 7 في ADR (التحقق الحي ليس هنا). قرارات قبل الاعتماد:
  1. فقدان التركيز لا يغلق الخلية على مسار الطبقة، وهذا ينسخ S-001 AC4 وجزء «فقدان التركيز يغلقها في الوضع العادي» من S-014 AC3 على هذا المسار وحده. `dnd::Machine` لا يتغير، ومحوّل الطبقة لا يرسل `FocusLost`.
  2. قاعدة CLAUDE.md «لا تعتمد على wlr-layer-shell» تُستبدل بالقاعدة المصحّحة في ADR 0004، وسطر قرار 0.1.0 في `docs/PLAN.md` (layer-shell «غير مدعوم في GNOME وKDE») قديم. لم يُعدَّل أيٌّ منهما هنا.
  3. أيقونة `file.svg` جديدة عامة، واللقطات الأربع الجديدة (AC26)، تنتظر محمداً.
  4. الترتيب: S-018 قبل S-017b وS-017c، لأن الضبابية تُطلب على سطح الطبقة، والقصتان تعدّلان مركز `hive.slint`، وS-017 AC7 يعيد كتابة فحوص الإغلاق.
  5. `cargo mutants`: الأجزاء الصافية (AC1 إلى AC3، AC21 إلى AC23) تحت الطفرات، وإرسال Wayland في محوّل الطبقة يُستثنى في `.cargo/mutants.toml` مع السبب كما `receiver.rs`.
  6. الذاكرة: الخدمة 38 MB اليوم من 40، فـ AC18 وAC24 وAC27 هي الخطر الأول.
  7. التعارض مع S-014c إلى S-014f: كلها تبني على `receiver.rs` ومسار الإفلات في `lib.rs`، وS-018d ينقل المستقبِل إلى اتصال الطبقة. إما أن تسبق S-014c القصةَ S-018d، أو تنتظر S-014c إلى S-014f حتى تنتهي S-018.
- 2026-10-20، المدير (بتفويض محمد في قرارات الأداء والواجهة): القصة معتمدة، والقرارات:
  1. على مسار الطبقة لا يغلق فقدان التركيز الخلية (يحل محل S-001 AC4 وجزء S-014 AC3 على هذا المسار فقط). GNOME وX11 وقارئ الشاشة كما اليوم.
  2. قاعدة CLAUDE.md صُحّحت في `0040ad3`. سطر قرار 0.1.0 في PLAN.md يُصحَّح الآن.
  3. أيقونة `file.svg` واللقطات الأربع الجديدة يعتمدها المدير بالتفويض بعد فحصها بالعين، ويُسجَّل ذلك في السجل.
  4. الترتيب: S-018 قبل S-017b وS-017c.
  5. شيفرة إرسال Wayland في محوّل الطبقة تُستثنى في `.cargo/mutants.toml` مع السبب، والأجزاء الخالصة تبقى تحت فحص الطفرات.
  6. الذاكرة: تحرير مخازن الرسم عند الإخفاء شرط في S-018f، والحد 40 MB لا يُرفع.
  7. S-014c إلى S-014f تنتظر انتهاء S-018 (تبني على المستقبِل الذي ينقله S-018d).
- 2026-10-20، المختبِر (S-018a فقط، AC1 إلى AC3): كُتبت الاختبارات من المعايير وفشلت كلها قبل التنفيذ، لأن الوحدة غير موجودة.
  - الملفات: `crates/ui/tests/overlay_pure.rs` (هدف تكامل مستقل، 25 اختباراً بأسماء `ac1_*` و`ac2_*` و`ac3_*`، فلا يُعدَّل `lib.rs` ولا يُكسر بناء اختبارات `--lib` التي يعمل عليها مختبِر S-018g بالتوازي)، والعوامّ في `tests/fixtures/overlay/kwin-6.7.5-globals.txt` و`gnome-50.5-globals.txt`: نسخ حرفية من `spikes/overlay/out/kwin-s1-FocusFollowsMouse-fsp1-globals.txt` و`gnome-globals.txt` (`overlay-spike globals` في KWin 6.7.5 الافتراضي وGNOME Shell 50.5 المخفي، تشغيلا اليوم؛ قوائم KWin الأربع متطابقة). الاختبار يحذف العلامة `*` ويأخذ الكلمة الأولى من كل سطر.
  - العقد الذي تثبّته الاختبارات (لا يُعدَّل)، في وحدة عامة `sbarhive_ui::overlay` بلا أنواع Wayland ولا قراءة بيئة:
    - `enum Surface { Layer, Window }` بـ `Debug, PartialEq`.
    - `choose_surface(globals: &[&str], screen_reader: bool, wayland: bool, forced: Option<&str>) -> (Surface, Option<String>)`؛ `forced` قيمة `SBARHIVE_SURFACE` الخام، والعنصر الثاني سطر السجل.
    - `struct Placement { x: i32, y: i32, buffer: (u32, u32), destination: (u32, u32) }` و`place(screen: (u32, u32), cell: (u32, u32), scale120: u32) -> Placement`.
    - `key_for_evdev(code: u32) -> Option<slint::platform::Key>` (Enter هو `Key::Return`، والأسهم `UpArrow` و`DownArrow` و`LeftArrow` و`RightArrow`).
    - `scheme_is_dark(env: Option<&str>, portal: Option<u32>) -> bool`؛ `env` قيمة `SBARHIVE_COLOR_SCHEME` الخام (`light|dark` كما في `forced_dark` اليوم)، و`portal` قيمة `color-scheme`، والغياب والخطأ كلاهما `None` عند المستدعي (لا تفرّق الدالة بينهما).
    - `wants_cursor_shape(globals: &[&str]) -> bool`.
  - ملاحظات على النص: (1) AC1 يذكر `wp_single_pixel_buffer_v1`، وهو اسم البروتوكول؛ العامّ المُعلَن اسمه `wp_single_pixel_buffer_manager_v1`، والاختبارات تشترطه بهذا الاسم. (2) AC2: الخلية 393 × 362 تُمرَّر قيمةً ثابتة كما طلبها المدير في S-017 AC13، ولا يشتقها الاختبار من `Layout::extent`. القيم المتوقعة: الموضع (444، 219) و(444، 219) و(764، 359) و(1724، 899)، والمخزن 393 × 362 و590 × 543 و786 × 724 و492 × 453، والوجهة 393 × 362 دائماً. ثلاث حالات تقع على نصف بكسل في x، فالقسمة الصحيحة المقطوعة تفشل. (3) ما سكت عنه المعيار لم يُثبَّت: قيمة `SBARHIVE_SURFACE` غير معروفة، والفرض بـ `layer` خارج Wayland، ووجود سطر سجل في غير حالة «layer دون العوامّ».
  - التحقق من صحة الاختبارات: نُسخ الملف إلى مجلد مؤقت مع تنفيذ مرجعي صغير مكتوب للتحقق فقط (لم يُودَع) فنجحت الـ 25 كلها، فالفشل الحالي سببه غياب الوحدة وحده.
  - مخرجات الفشل (`nice -n 10 cargo test -j 4 -p sbarhive-ui --test overlay_pure`):
    ```
    error[E0432]: unresolved import `sbarhive_ui::overlay`
     --> crates/ui/tests/overlay_pure.rs:9:18
      |
    9 | use sbarhive_ui::overlay::{
      |                  ^^^^^^^ could not find `overlay` in `sbarhive_ui`

    error: could not compile `sbarhive-ui` (test "overlay_pure") due to 1 previous error
    ```
- 2026-10-20، المختبِر (S-018g فقط، AC21 إلى AC23): كُتبت الاختبارات من المعايير وفشلت كلها قبل التنفيذ، لأن الوحدة غير موجودة.
  - الملفات: `crates/ui/tests/thumb.rs` (هدف تكامل مستقل، 38 اختباراً بأسماء `ac21_*` و`ac22_*` و`ac23_*`، فلا يُعدَّل `lib.rs`)، و`tests/fixtures/mime/home/mime/globs2` (مكان `XDG_DATA_HOME`) و`tests/fixtures/mime/system/mime/globs2` (مدخل من `XDG_DATA_DIRS`). ذاكرة الصور المصغّرة تُبنى في كل اختبار تحت `CARGO_TARGET_TMPDIR/s018g/`: صور PNG بحجم 1 × 1 بقطع `tEXt` (`Thumb::URI` و`Thumb::MTime` و`Thumb::Size`). لا يُقرأ شيء من ذاكرة المالك ولا من مجلدات بياناته.
  - قيم `Gio.File.get_uri()` محفوظة في الاختبار لتسعة أسماء تحت `/home/u/`: الثمانية في AC21، وتاسع لبقية المحارف المسموحة (`x'!*=:@$-_.png`)، سُجّلت بـ PyGObject اليوم. وقيمتا MD5 للمسار محسوبتان بـ Python `hashlib`.
  - العقد الذي تثبّته الاختبارات (لا يُعدَّل)، في وحدة عامة `sbarhive_ui::thumb` بلا قراءة بيئة في الدوال المختبَرة:
    - `file_uri(path: &Path) -> String` (صيغة GLib)، و`kio_uri(path: &Path) -> String` (صيغة Qt، `;` حرفية).
    - `thumbnails_dir(xdg_cache_home: Option<&OsStr>, home: &Path) -> PathBuf`: القيمة الفارغة أو النسبية تعني `~/.cache`.
    - `enum Size { Normal, Large, XLarge, XxLarge }` بـ `Clone, Copy, Debug, PartialEq, Eq`، و`Size::dir(self) -> &'static str`، و`const SEARCH_ORDER: [Size; 4]`.
    - `thumbnail_path(thumbnails: &Path, size: Size, uri: &str) -> PathBuf`، و`find_thumbnail(thumbnails: &Path, file: &Path) -> Option<PathBuf>`.
    - `data_dirs(xdg_data_home: Option<&OsStr>, xdg_data_dirs: Option<&OsStr>, home: &Path) -> Vec<PathBuf>`.
    - `struct Globs` بـ `Globs::load(dirs: &[PathBuf]) -> Globs` (يقرأ `<dir>/mime/globs2`) و`mime_of(&self, file_name: &str) -> Option<String>`.
    - `enum Kind { Image, Video, Audio, Pdf, Document, Sheet, Slides, Text, File }` بـ `Clone, Copy, Debug, PartialEq, Eq` و`Kind::icon(self) -> &'static str`، و`kind_of(mime: &str) -> Kind`، و`kind_for(globs: &Globs, file_name: &str) -> Kind`، و`common_kind<I: IntoIterator<Item = Kind>>(kinds: I) -> Kind`، و`format_code(file_name: &str) -> Option<String>`.
  - ما اختارته الاختبارات حيث سكت المعيار: (1) الرابط الرمزي يجرّب رابطه هو أولاً ثم هدفه، وزمن التعديل والحجم من الهدف (`stat` يتبع الرابط). (2) صيغة Qt تُقبل فقط إن كان `Thumb::URI` فيها صيغة Qt نفسها. (3) غياب `Thumb::URI` أو `Thumb::MTime` رفض. (4) عند تساوي الوزن وطول النمط يفوز الملف الأسبق في الترتيب (`XDG_DATA_HOME`)، والملفات تُدمج ولا يحل أحدها محل الآخر. (5) بلا علامة `cs` المطابقة غير حساسة لحالة الحروف، والنمط دون `*` يطابق الاسم كله. (6) قائمة المستندات والجداول والعروض: ODF وMS Office القديم وOOXML، و`application/rtf` مستند كما في `crates/core/src/media.rs`. (7) `format_code`: الامتداد الأخير، 1 إلى 4 محارف، وإلا `None`. (8) سجل `fail/` وحده لا يعطي صورة، ولم يُثبَّت ما يحدث إن وُجد سجل فشل وصورة صالحة معاً.
  - `ac23_kind_icons_are_our_monoline_set` يشترط أن تكون `file` في `sbarhive_ui::icon_names()`، فيبقى فاشلاً حتى تُضاف `crates/ui/icons/file.svg` (قرار المدير 3).
  - خارج هذه الوحدة: «لا يُفتح أي ملف تحت `/usr/share/icons`» يُتحقق منه بـ `strace -e openat` في `tests/overlay/kwin.sh` (S-018b)؛ هنا يكفي أن العقد لا يقبل مجلد سمة أيقونات أصلاً.
  - التحقق من صحة الاختبارات: نُسخ الملف إلى crate مؤقت خارج المستودع مع تنفيذ مرجعي صغير للتحقق فقط (لم يُودَع) فنجحت الـ 38 كلها، ومع دوال `todo!()` فشلت كلها وخرج `clippy -D warnings` نظيفاً. فالفشل الحالي سببه غياب الوحدة وحده.
  - مخرجات الفشل (`nice -n 10 cargo test -j 4 -p sbarhive-ui --test thumb`):
    ```
    error[E0432]: unresolved import `sbarhive_ui::thumb`
      --> crates/ui/tests/thumb.rs:34:18
       |
    34 | use sbarhive_ui::thumb::{
       |                  ^^^^^ could not find `thumb` in `sbarhive_ui`

    error: could not compile `sbarhive-ui` (test "thumb") due to 1 previous error
    ```
- 2026-10-20، المدير: المنفِّذان لـ S-018a وS-018g توقفا مع انتهاء الجلسة قبل التثبيت، وأكمل المدير البوابات على شيفرتهما كما هي: fmt وclippy نظيفان، `cargo test --workspace` 419 ناجحاً، `overlay_pure` 25 من 25 و`thumb` 38 من 38، design-lint نظيف. أيقونة `file.svg` فُحصت بالعين بجانب document وpdf وimage: الإطار والخط والزوايا من العائلة نفسها، وتُعتمد بالتفويض. فحص الطفرات يلي هذا المدخل.
- 2026-10-20، المختبِر (S-018b فقط، AC4 إلى AC7): كُتبت الاختبارات من المعايير وشُغّل كل سكربت جديد مرة واحدة على بناء `1d42105` (`cargo build --release`، `nice`). فشلت فحوص AC4 وAC5 وAC7 الخاصة بالطبقة لأن السطح الطبقي غير موجود بعد، وبقيت فحوص «كما اليوم» ناجحة حارساتٍ للانحدار.
  - الملفات: `tests/overlay/kwin.sh` (الوضع الافتراضي طبقة، و`kwin.sh window` يشغّل الخدمة بـ `SBARHIVE_SURFACE=window`)، و`tests/overlay/gnome.sh`، و`tests/overlay/other.py` (نافذة GTK بملء الشاشة بلون واحد بلا إطار ولا نص، فلا تتغير صورتها مع التركيز)، و`tests/overlay/shot.py` (نسخة من `spikes/overlay/shot.py`)، و`tests/overlay/boxdiff.py` (صندوق البكسلات المتغيرة بين لقطتين، مكتبة Python القياسية وحدها)، وتعديل `crates/ui/tests/snapshots.sh`، و`tests/overlay/out/` في `.gitignore`. الإدخال من `tests/drop/kwin-input` المبني.
  - `kwin.sh`: KWin مخفي 1280×800، الخدمة بـ `WAYLAND_DEBUG=1` فوق `other.py`، وخمسة ظهورات: `open` بلا رمز، `open --token sbh-test-token`، `drop`، ثم `language = "ar"` في `config.toml` الخاص، ثم `theme = "dark"` (انتظار 3 ث لكلٍّ، الخدمة تراقب الملف كل 2 ث). يُقطع الأثر بعلامات أسطر لكل ظهور. لكل ظهور: `get_layer_surface` واحد بالضبط، وبالطبقة 3 والاسم `"sbarhive"`، و`set_anchor` كله 15، و`set_exclusive_zone` كله -1، و`set_keyboard_interactivity` كله 1 (عدا ظهور `drop`، لأن AC13 يجعلها 0)، و`get_subsurface` واحد بالضبط، وصفر `get_xdg_surface`. وفي الأثر كله صفر `get_xdg_surface` (ولا سطح نافذة مخفي عند البدء)، ولا `wl_display#1.error(` ولا «protocol error». لقطة ScreenShot2 قبل الظهور الأول وأثناءه: مركز صندوق الفرق في (640، 400) بفرق 1 px أو أقل، والصندوق بين 300 × 280 و394 × 363 (لا شريط عنوان، والماسك شفاف).
  - `gnome.sh`: GNOME Shell مخفي، ثلاثة ظهورات (بلا رمز، برمز، `drop`): نافذة `SbarHive` ظاهرة من `Shell.Eval`، و`get_xdg_surface` حتى نهاية الظهور، وصفر `get_layer_surface`، ولا خطأ بروتوكول.
  - `snapshots.sh` (AC7): كل مرجع يُقارن مرتين، بـ `SBARHIVE_SURFACE=layer` وبـ `window`، مع أثر `WAYLAND_DEBUG=1` لكل تشغيل: تشغيل الطبقة يشترط `get_layer_surface` واحداً أو أكثر وصفر `get_xdg_surface`، وتشغيل النافذة العكس. `SNAP_SURFACES` يحصر القائمة، و`--update` يكتب المراجع من تشغيل النافذة وحده. `settings_snapshots.sh` دون تغيير ولم يُشغَّل (حمل الجهاز).
  - العقد الذي تثبّته الاختبارات (لا يُعدَّل):
    1. على KWin بلا `SBARHIVE_SURFACE` يكون كل ظهور من الخدمة، بما فيه `drop` وما بعد تغيير اللغة والسمة في `config.toml`، سطحاً طبقياً بالطلبات أعلاه، ولا تنشئ الخدمة أي `xdg_surface` للخلية ولا عند البدء.
    2. **Esc (evdev 1) يغلق الخلية على مسار الطبقة من S-018b**، لا من S-018c: الاختبار يغلق كل ظهور بـ Esc عبر `org_kde_kwin_fake_input` قبل الظهور التالي، وإلا لم يكن ظهوراً جديداً بسطح جديد. ظهور `drop` يُنتظر بعده 10 ث (مهلة S-014a AC3) لأن لوحة المفاتيح قد تكون 0.
    3. `SBARHIVE_SURFACE=window` و`layer` يُقرآن في الخدمة وفي مثال `hive_snapshot`، و`take_snapshot` يعمل على محوّل الطبقة بأبعاد المرجع (393 × 362).
    4. صيغة الأثر: الطلبات `-> interface#id.request(` (wayland-rs) أو بـ `@` (libwayland)، والاختبارات تقبل الاثنين.
  - ما حُسم حيث سكت المعيار: عدد `set_anchor` و`set_exclusive_zone` و`set_keyboard_interactivity` واحد أو أكثر بشرط أن تكون كلها بالقيمة المطلوبة. ورمز مختلَق يكفي لظهور «برمز» في AC5 (العدّ وحده، والتفعيل الحقيقي في AC10).
  - الاختبارات القائمة: لم يُعدَّل `tests/focus/kwin.sh:42-47` ولا `tests/drop/lib.sh:157-163` ولا `tests/drop/kwin.sh:51`، لأنها تعتمد على سطري `hive focus` و`hive surface` (AC11، S-018c) وعلى المستقبِل على اتصال الطبقة (S-018d). ولم يُعدَّل `tests/e2e/run.sh` بعد السطر 64: الإغلاق الصريح يحتاج إدخالاً محقوناً، وتشغيل KWin فيه (السطر 15) بلا `KWIN_WAYLAND_NO_PERMISSION_CHECKS`، وهذا خارج السطور المسموحة. تنبيه للمنفِّذ والمراجع: من S-018b يصير KWin طبقةً افتراضياً، فقد تبقى الخلية ظاهرة من السطر 64 حتى فحص الذاكرة في السطر 105 (AC18، S-018f). وكذلك `tests/focus/kwin.sh` و`tests/drop/kwin.sh` قد يتغير سلوكهما على KWin قبل S-018c وS-018d.
  - لم يُشغَّل: `kwin.sh window` (AC6، تعليمات المدير: تشغيل واحد لكل سكربت جديد، وهو اليوم مطابق للوضع الافتراضي)، و`settings_snapshots.sh`. فحص `strace -e openat` لـ AC23 لم يُضف هنا (S-018g/h).
  - ملاحظة من الأثر: النافذة اليوم في KWin تطلب `get_subsurface` خمس مرات لكل ظهور تقريباً، فشرط «واحد بالضبط» لا يصح إلا لأن مسار الطبقة لا ينشئ نافذة winit للخلية.
  - مخرجات الفشل، `nice -n 10 tests/overlay/kwin.sh` (RESULT FAIL (33)، مقتطف):
    ```
    INFO whole trace: get_xdg_surface 5, get_layer_surface 0
    INFO show 1 (plain): get_layer_surface 0 (layer 3 "sbarhive" 0), set_anchor(15) 0, set_exclusive_zone(-1) 0, set_keyboard_interactivity(1) 0, get_subsurface 0, get_xdg_surface 0
    FAIL AC5 show 1 (plain): exactly one get_layer_surface
    FAIL AC4 show 1 (plain): the layer surface is on layer 3 (overlay) with namespace sbarhive
    FAIL AC4 show 1 (plain): anchors 15 (all edges)
    FAIL AC4 show 1 (plain): exclusive_zone -1
    FAIL AC4 show 1 (plain): keyboard_interactivity 1 (exclusive)
    FAIL AC4 show 1 (plain): exactly one get_subsurface (the hive in the catcher)
    PASS AC5 show 1 (plain): no get_xdg_surface
    INFO show 2 (token): get_layer_surface 0 (layer 3 "sbarhive" 0), set_anchor(15) 0, set_exclusive_zone(-1) 0, set_keyboard_interactivity(1) 0, get_subsurface 5, get_xdg_surface 1
    FAIL AC5 show 2 (token): exactly one get_layer_surface
    FAIL AC5 show 2 (token): no get_xdg_surface
    INFO show 3 (drop): get_layer_surface 0 (layer 3 "sbarhive" 0), set_anchor(15) 0, set_exclusive_zone(-1) 0, set_keyboard_interactivity(1) 0, get_subsurface 10, get_xdg_surface 2
    FAIL AC5 show 3 (drop): exactly one get_layer_surface
    FAIL AC5 show 3 (drop): no get_xdg_surface
    (الظهوران 4 و5 بالفشل نفسه)
    FAIL AC4/AC5: no get_xdg_surface in the whole trace (no hive toplevel, not even a hidden one)
    PASS AC4: no protocol error in the daemon log
    INFO hive pixel box: 447 223 384 353
    PASS AC4: screenshot box centred within 1 px
    PASS AC4: screenshot box no larger than the hive (no title bar, transparent catcher)
    RESULT FAIL (33)
    ```
    الظهوران 1 و5 بلا `get_xdg_surface` في قطعتيهما لأن الأول يعيد استخدام سطح winit المنشأ عند البدء، والخامس كذلك على ما يبدو؛ فحص الأثر كله يلتقطهما. فحوص اللقطة ناجحة اليوم لأن النافذة الحالية بلا إطار وفي المنتصف: حارسات انحدار.
  - `nice -n 19 tests/overlay/gnome.sh`: RESULT PASS (11 فحصاً)، متوقَّع لأن AC6 «كما اليوم»: حارس انحدار.
  - `CARGO_BUILD_JOBS=2 nice -n 19 crates/ui/tests/snapshots.sh`: `snapshots: FAILED`. المقارنة بالبكسل ناجحة في التشغيلات الثمانية (`diff 0 pixels`)، والفشل في فحص السطح للتشغيلات الأربعة بـ `layer`. سطور FAIL قُطعت من المخرجات المعروضة (`tail`)، وأعداد الأثر المحفوظة في مجلد المخرجات قبل حذفه: `layer-hive-{en,ar}-{dark,light}.trace` كلها `get_layer_surface 0` و`get_xdg_surface 1`، فالسطر المطبوع لكلٍّ منها: `FAIL AC7 layer hive-en-dark.png: drawn on get_layer_surface 0, get_xdg_surface 1 (want >= 1 and 0)`. تشغيلات `window` كلها `0` و`1`، ناجحة.
- 2026-10-20، المدير (قرار): الخيار (أ). بعد S-018b قد تفشل مؤقتاً `tests/focus/kwin.sh` و`tests/drop/kwin.sh` و`tests/e2e/run.sh` على KWin حتى S-018c وS-018d، وهذا مقبول ومسجّل. بطلب محمد (حمل الجهاز) تُشغَّل البوابات الثقيلة مرة واحدة في نهاية S-018 قبل المراجعة، وعندها يجب أن تنجح كل الاختبارات المذكورة في «تبقى ناجحة». نقل Esc على مسار الطبقة إلى S-018b مقبول.
- 2026-10-20، المنفِّذ (S-018b فقط، AC4 إلى AC7): S-018b جاهزة للمراجعة (الحالة: مراجعة للقصة الفرعية؛ رأس الملف يبقى «معتمدة» لأن S-018c إلى S-018h لم تبدأ، كما بعد S-018a وS-018g). لم يُعدَّل أي اختبار.
  - الملفات: `crates/ui/src/platform.rs` (منصة رقيقة تمرّر كل شيء إلى winit، إلا محوّل نافذة يُنشأ أثناء بناء الخلية والقرار `Layer`)، و`crates/ui/src/layer.rs` (محوّل الطبقة من `spikes/overlay/src/overlay.rs`: الماسك طبقةً على الطبقة 3 باسم `sbarhive`، المثبتات 15، المنطقة -1، لوحة المفاتيح 1 أو 0 في ظهور الإفلات مع منطقة إدخال فارغة، بكسل شفاف واحد يمدّه `wp_viewporter`، والخلية سطحاً فرعياً في الموضع والمخزن والوجهة من `overlay::place`، والمفاتيح من `overlay::key_for_evdev`)، وتعديلات `lib.rs`.
  - الاختيار: `init()` يتصل بالمؤلِّف ويقرأ العوامّ ويستدعي `overlay::choose_surface` (قارئ الشاشة `false` حتى S-018c) مع `SBARHIVE_SURFACE`. إن كان `Window` يبقى مسار `BackendSelector` كما هو حرفياً (GNOME، `window`، X11). وإلا تُثبَّت المنصة الرقيقة، و`build_window` يرفع علامة حول `HiveWindow::new()` وينزلها بعده نجح أو فشل، فيمر كل بناء (البدء، الرمز، تغيير اللغة والسمة في الخدمة) بالقرار من جديد. الخدمة ومثال `hive_snapshot` يتبعان المتغير دون شيفرة فيهما.
  - اتصال واحد وخيط قراءة واحد للعملية كلها، لا لكل محوّل، فلا يتسرب اتصال ولا خيط مع كل بناء جديد. الأسطح تُنشأ في `set_visible(true)` وتُهدم في `set_visible(false)` وفي `Drop`، فكل ظهور `get_layer_surface` واحد و`get_subsurface` واحد.
  - Esc يغلق على مسار الطبقة (قرار المدير): `wl_keyboard.enter` يرسل `WindowActiveChanged(true)` إلى نافذة Slint، والمفتاح يصل `FocusScope` في `hive.slint`. `leave` لا يمر بـ `focus_changed`، ففقدان التركيز لا يغلق (القرار 1). نقرة على الماسك و`closed` من المؤلِّف يرسلان `CloseRequested` فيمرّان بـ `dismiss` (التحقق في S-018c AC8). سطر `hive focus: true|false` مع `SBARHIVE_DEBUG_FOCUS` يُطبع عند الدخول والخروج. الرمز يأخذه المحوّل من `PENDING_TOKEN` عند إنشائه ويرسله `xdg_activation_v1.activate` مرة واحدة على الماسك (التحقق في S-018c AC10). خطاف `after_next_frame` يعمل بعد إيداع إطار الطبقة كما بعد `RedrawRequested`.
  - الاعتمادات: `i-slint-backend-winit` و`i-slint-core` بـ `=1.18.1` (`default-features = false`)، و`wayland-protocols` 0.32.13 و`wayland-protocols-wlr` 0.3.12، وميزة `fs` في `rustix` لـ `memfd_create`. `Cargo.lock` لم يُضف أي حزمة (أربعة أسطر اعتماد لـ `sbarhive-ui` فقط)، و`cargo tree -d` قبل وبعد متطابقان. الرخص في `docs/engines.md`. `layer.rs` و`platform.rs` و`frame_presented` و`token_hook` و`init` مستثناة في `.cargo/mutants.toml` مع السبب (القرار 5)؛ `cargo mutants --list --in-diff` (قائمة فقط، بلا تشغيل) لا يعرض بعدها إلا `Hive::show` و`build_window`، وتغطيهما `ui_tests`. Esc في الأثر المحفوظ: في الظهورات 1 و2 و4 و5 يتبع `wl_keyboard.key(..., 1, 1)` هدمُ `zwlr_layer_surface_v1` داخل الظهور نفسه، وظهور الإفلات يُغلق بالمؤقت بعد 10 ث.
  - تشغيل واحد لكلٍّ (`nice -n 19`، بناء `-j 2`): `tests/overlay/kwin.sh` RESULT PASS (كل فحوص الظهورات الخمسة، وصفر `get_xdg_surface` في الأثر كله، و5 `zwlr_layer_surface_v1.destroy`، وصندوق اللقطة `448 223 384 353`)؛ `tests/overlay/gnome.sh` RESULT PASS؛ `crates/ui/tests/snapshots.sh` `snapshots: ok`، المراجع الأربعة `diff 0 pixels` بـ `layer` و`window`. لم يُشغَّل `kwin.sh window` ولا `settings_snapshots.sh` ولا `measure.sh` ولا e2e ولا `cargo mutants` (حمل الجهاز).
  - البوابات: `cargo fmt --all --check` نظيف، `cargo clippy --workspace --all-targets -- -D warnings` نظيف، `cargo test --workspace` 419 ناجحاً و0 فاشلاً، `scripts/design-lint.sh crates/ui` نظيف.
  - مخاطر مفتوحة: (1) على مسار الطبقة مع `theme = "auto"` لا يصل مخطط ألوان النظام إلى Slint، فالخلية فاتحة حتى S-018e (البوابة). (2) لا شكل مؤشر على الماسك والخلية حتى S-018e. (3) مستقبِل الإفلات لا يُربط على مسار الطبقة (`ATTACH_PENDING` يبقى مرفوعاً بلا أثر) حتى S-018d، فـ `tests/drop/kwin.sh` يفشل على KWin كما قرّر المدير، ومعه `tests/focus/kwin.sh` و`tests/e2e/run.sh` (لم تُشغَّل). (4) مخازن `wl_shm` تُنشأ لكل إطار بـ memfd جديد وتُهدم عند `release` فقط؛ الهدم الصريح عند الإخفاء والميزانية في S-018f. (5) Hyprland وSway لم يُشغَّلا.
- 2026-10-20، المختبِر (S-018c فقط، AC8 إلى AC12): كُتبت الاختبارات من المعايير وشُغّل كل سكربت جديد أو معدَّل مرة واحدة على بناء `b55b071` (`cargo build --release -j 2`، `nice -n 19`). تفشل فحوص AC11 (سطر `hive surface` على المسارين) وAC12 (قارئ الشاشة) وAC10 (المستوى 4 بالرمز الحقيقي). وتنجح اليوم فحوص النقر ولوحة المفاتيح والرمز لأن S-018b سلّمت سلوكها مسبقاً (سجل المنفِّذ أعلاه: النقر على الماسك يمر بـ `CloseRequested`، و`leave` لا يغلق، و`PENDING_TOKEN` يُرسَل بـ `activate`)، فهي حارسات انحدار لا اختبارات خاطئة.
  - الملفات: `tests/overlay/kwin.sh` بثلاثة أوضاع جديدة لكلٍّ منها مؤلِّف خاص: `input` (AC8، AC9، و`kwinrc` بـ `FocusPolicy=FocusFollowsMouse` والمستوى 1)، و`token` (AC10: ثلاثة ظهورات `open --token sbh-token-<n>` ثم ظهور بلا رمز)، و`a11y` (AC12). وأُضيفت فحوص AC11 إلى الوضعين `layer` و`window`، وظهور سادس في `window` (نقرة على نافذة GTK). وكذلك `tests/overlay/a11y.py` (خدمة وهمية)، و`tests/overlay/focus.sh` (AC10 بالرمز الحقيقي، على نمط `tests/focus/kwin.sh`)، وفحوص AC11 في `tests/overlay/gnome.sh`، و`ac8_ac9_layer_input_reference` في `crates/ui/src/ui_tests.rs` (اختبار مستقل بخيطه، فلا تتغير أعداد `hive_behaviour`). الخدمة في كل الأوضاع بـ `SBARHIVE_DEBUG_FOCUS=1 WAYLAND_DEBUG=1`.
  - العقد الذي تثبّته الاختبارات (لا يُعدَّل):
    1. AC8: خلية الحلقة الأولى يمين المركز هي المحوري (1، 0)، الموضع 1 في الحلقة (`geometry::ring`)، أي `image.to-jpg` لملف PNG (ثاني سطر حلقته 1 في `sbarhive actions`). النقر في (640 + 78، 400) (الإزاحة من `hex.size.md` و`gap` في الرموز) يكتب `photo.jpg` من نوع `image/jpeg`، ولا شيء غيره، و0 نقرات عند GTK، وهدم السطح الطبقي. النقر في (100، 700): `wl_pointer.button(..., 272, 1)` يصل الماسك، و`zwlr_layer_surface_v1.destroy` خلال 500 ms **من الضغط** (طابعا الأثر)، وهدم واحد بالضبط، ولا ملف جديد، و0 نقرات عند GTK. «`on_dismiss` مرة واحدة» لا يُرى من خارج الخدمة: يثبته `ui_tests` (`CloseRequested` يستدعيه مرة، والثاني والخلية مخفية لا يضيف شيئاً)، والسكربت يثبت الإغلاق وغياب الناتج والنقر.
    2. AC9: السهم الأيمن ثم Enter من دون تمييز يختاران الموضع 0 (`ui_tests`: المعرّف `webp`)، أي `photo.webp` بنوع `image/webp`، والمؤشر في (100، 700) فوق GTK. التجوال على (100، 700) و(1200، 80) و(640، 400) و(718، 400) و(60، 60) بلا نقر: لا هدم ولا `wl_keyboard.leave` قبل Esc، ثم Esc يهدم السطح. `sbarhive settings` والخلية ظاهرة: `get_xdg_surface` للإعدادات وصفر هدم للطبقة خلال 3 ث. مسار النافذة (`kwin.sh window`): النقرة تصل GTK مرة واحدة، و`hive focus: false` و`xdg_toplevel.destroy`.
    3. AC10 (`kwin.sh token`): في كل ظهور طلب `xdg_activation_v1.activate` واحد بالضبط، برمز ذلك الظهور وعلى سطح الماسك (الـ `wl_surface` الممرَّر إلى `get_layer_surface` في القطعة نفسها)، وبعده في الأثر. الظهور الرابع بلا رمز: صفر `activate(`. Esc يغلق كل ظهور. اختير رمز مختلف لكل ظهور ليُكشف رمز قديم معلّق.
    4. AC10 (`tests/overlay/focus.sh`، مؤلِّف لكل حالة: `1:token 1:notoken 4:token 4:notoken 4:bogus 2:notoken 3:notoken`): خلية تسخين تُغلق بـ Esc ثم 3 ث، ثم `client.py` بـ `CLIENT_AUTO`، ولا يُحقن أي حدث مؤشر (المؤشر ساكن). بالرمز في 1 و4: `activate("<رمز client.py>", <الماسك>)` و`hive focus: true`. بالمستوى 4 بلا رمز ومع المختلَق: صفر `hive focus: true`، والمختلَق يُرسَل `activate` مع ذلك. بالمستوى 1 بلا رمز: `wl_keyboard.enter` على الماسك خلال 500 ms من `get_layer_surface`. المستويان 2 و3 في جدول `TABLE` بلا فحص.
    5. AC11: سطر واحد بالضبط لكل ظهور (ومنه ظهور الإفلات) بالصيغة `hive surface: layer|window X Y W H` حرفياً بأعداد صحيحة. على الطبقة في 1280×800 بمقياس 1: `444 219 393 362`، وX Y تساوي `wl_subsurface.set_position` في الأثر، وW H تساوي `wp_viewport.set_destination` فيه، وصندوق لقطة الظهور الأول داخل المستطيل. `hive focus: true` بعدد `wl_keyboard.enter` و`false` بعدد `leave` على سطح حي (KWin يرسل `leave(serial, nil)` بعد هدم الماسك، ولا يُعدّ)، و`true` واحد على الأقل في الظهورات غير الإفلات. على النافذة: لا يعرف عميل Wayland موضع نافذته، فيُشترط W H بحجم الخلية (393 × 362 بفرق 1 px في KWin، وإطار `Shell.Eval` بفرق 1 px في GNOME) وX Y داخل الشاشة فقط.
    6. AC12: `a11y.py` يملك `org.a11y.Bus` قبل تشغيل KWin، على `/org/a11y/bus`: `org.a11y.Status` بالخاصيتين `IsEnabled` و`ScreenReaderEnabled` (قراءة بـ `Properties.Get` و`GetAll`)، و`org.a11y.Bus.GetAddress` بعنوان وهمي. عند التبديل يرسل `org.freedesktop.DBus.Properties.PropertiesChanged` على المسار نفسه بالواجهة `org.a11y.Status`. يبدأ `true`: الظهور الأول `get_xdg_surface` حتى نهايته وصفر `get_layer_surface`. ثم `false` وانتظار 1.5 ث: الظهور التالي `get_layer_surface` واحد وصفر `get_xdg_surface`.
  - تعديل الاختبارات القائمة (التأكيدات كما هي): `tests/focus/kwin.sh:19` سطر جديد يشترط وجود `tests/drop/kwin-input` المبني، و`:30` (كان 29) أضيف `KWIN_WAYLAND_NO_PERMISSION_CHECKS=1` إلى تشغيل KWin، والسطور 73 إلى 82 جديدة بعد سطر `warm-up` (كان 71): Esc لخلية التسخين عبر `tests/drop/kwin-input` ثم 3 ث. هذا شرط إغلاق خلية التسخين بـ Esc، لأن الإدخال المحقون يحتاج المتغير. وسطر في `tests/focus/README.md` بذلك. السطور 42 إلى 47 لم تُعدَّل: تشغيل اليوم يُظهر سطح الطبقة في `windowList` باسم فارغ، وسطر `daemon focus lines` كافٍ. `tests/drop/lib.sh` و`tests/drop/kwin.sh:51` و`tests/e2e/run.sh` لم تُعدَّل (S-018d وS-018f).
  - تشغيل واحد لكلٍّ: `tests/overlay/kwin.sh` RESULT FAIL (25)، كل فحوص AC4 وAC5 ناجحة (43 PASS)، والفشل كله AC11. `kwin.sh window` RESULT FAIL (10)، كله AC11، وحارسا AC9 لمسار النافذة ناجحان. `kwin.sh input` RESULT FAIL (1)، والفشل كان خطأ في الاختبار نفسه (متغير مشترك لقائمة الملفات بين ظهورين)، فأُصلح وشُغّل مرة ثانية: RESULT PASS، ثم فُصل متغيرا الظهورين دون تشغيل ثالث. `kwin.sh token` RESULT PASS. `kwin.sh a11y` RESULT FAIL (2). `gnome.sh` RESULT FAIL (6)، كله AC11، وفحوص AC6 الـ 11 ناجحة. `focus.sh` RESULT FAIL (حالة واحدة). `tests/focus/kwin.sh` (المستوى 1، notoken وtoken): خلية التسخين أُغلقت (`before` فيه `focus-client` وحده)، و`hive focus: true` في الحالتين. `ac8_ac9_layer_input_reference` ناجح (مرجع). fmt وclippy لـ `sbarhive-ui` نظيفان.
  - بعد التشغيل عُدّل شرطان دون إعادة تشغيل (حمل الجهاز): عدّ `leave` على سطح حي فقط (الفشل الوحيد لهذا الفحص كان `leave(.., nil)`)، وحُذف فحص «الخدمة الوهمية سُئلت» من `a11y` لأن KWin نفسه يسأل الخاصية فكان ينجح دائماً، فصار سطر INFO.
  - مخرجات الفشل (مقتطف):
    ```
    == kwin layer
    INFO show 1 (plain): 0 surface line(s): none
    FAIL AC11 show 1 (plain): exactly one "hive surface: layer X Y W H" line
    FAIL AC11 show 1 (plain): the layer line is 444 219 393 362 (AC2 on 1280x800, scale 1)
    FAIL AC11 show 1 (plain): X Y equal the subsurface set_position in the trace
    FAIL AC11 show 1 (plain): W H equal a wp_viewport set_destination in the trace
    (والظهورات 2 إلى 5 بالفشل نفسه)
    FAIL AC11: the screenshot box lies inside the first show's "hive surface" rectangle
    RESULT FAIL (25)
    == kwin window
    FAIL AC11 show 1 (plain): exactly one "hive surface: window X Y W H" line
    FAIL AC11 show 1 (plain): the window line has the hive's size and lies on the 1280x800 screen
    PASS AC9 window path (guard): focus loss closes the hive (xdg_toplevel destroyed)
    RESULT FAIL (10)
    == kwin a11y
    INFO reader on: get_layer_surface 1, get_xdg_surface up to its end 0
    FAIL AC12 ScreenReaderEnabled=true: the next show is an xdg_surface (window)
    FAIL AC12 ScreenReaderEnabled=true: no get_layer_surface
    PASS AC12 PropertiesChanged to false: the next show is a layer again
    RESULT FAIL (2)
    == gnome
    INFO show 1 (plain): hive window 444 235 393 362
    FAIL AC11 show 1 (plain): exactly one "hive surface: window X Y W H" line
    RESULT FAIL (6)
    == kwin focus level 4 token
    INFO level 4 token: catcher wl_surface#24, token kwin-2, activate: -> xdg_activation_v1#12.activate("kwin-2", wl_surface#24), keyboard enter after none ms, hive focus lines:
    FAIL AC10 level 4 token: the hive takes the keyboard (hive focus: true)
    == AC10 table
      level=1 case=token focus=true enter_ms=4
      level=1 case=notoken focus=true enter_ms=4
      level=4 case=token focus=false enter_ms=none
      level=4 case=notoken focus=false enter_ms=none
      level=4 case=bogus focus=false enter_ms=none
      level=2 case=notoken focus=true enter_ms=3
      level=3 case=notoken focus=true enter_ms=5
    ```
  - ملاحظات للمنفِّذ والمراجع: (1) بالمستوى 4 يُرسَل الرمز الحقيقي على الماسك ولا يعطي KWin لوحة المفاتيح. والرمز صالح، فعلى مسار النافذة أخذ التركيز به في S-013. والأداة سليمة بشهادة أثر الحالة: خلية التسخين أخذت لوحة المفاتيح (`wl_keyboard.enter` ثم `hive focus: true`) وأغلقها Esc (`key(.., 1, 1)` ثم `zwlr_layer_surface_v1#16.destroy`) قبل العميل بـ 6 ث، و`client.py` كان مركَّزاً (`focus True`) حين أخذ الرمز `kwin-2`، و`activate("kwin-2", wl_surface#24)` بعد `get_layer_surface` بأقل من 1 ms، ثم لا `enter` خلال 6 ث. السبب لم يُبحث هنا: قد يكون أن KWin بالمستوى Extreme لا يفعّل سطحاً طبقياً بالرمز أصلاً، وهذا سؤال جدوى للمدير، لا يمنع الإيداع. (2) لا تعارض اليوم بين AC4 (لوحة مفاتيح 1) وAC10: بالمستوى 4 بلا رمز لا تأخذ الطبقة التركيز. (3) لا فحص فاشلاً اليوم لـ AC8 وAC9: `kwin.sh input` و`kwin.sh token` ناجحان كلهما لأن S-018b سلّمت السلوك. هذا يخالف قاعدة المختبِر («الاختبار الذي ينجح قبل التنفيذ خاطئ»)، ولم يُختلق شرط زائد ليفشل، فهو قرار للمدير: إما قبولهما حارسَين، أو اعتبار AC8 وAC9 منجزين في S-018b. (4) `kwin.sh input` بصيغته المودَعة لم يُشغَّل (فصل متغيري الملفات بعد التشغيل الثاني)، وكذلك شرط `leave` في `layer` وحذف فحص `a11y`. (5) `cargo test -p sbarhive-ui --lib` كله ناجح (74)، فالاختبار الجديد بخيطه لا يتداخل مع `hive_behaviour` ولا مع `PENDING_TOKEN`.
- 2026-10-20، المنفِّذ (S-018c فقط، AC10 إلى AC12؛ AC8 وAC9 منجزان في S-018b بقرار المدير وبقيا حارسَين): AC11 وAC12 منجزان، وAC10 منجز إلا حالة المستوى 4 بالرمز في `tests/overlay/focus.sh`، وسببها في الأداة لا في الشيفرة (الدليل أدناه)، فهي قرار للمدير. لم يُعدَّل أي اختبار. الحالة: مراجعة للقصة الفرعية بعد قرار المدير في المستوى 4؛ رأس الملف يبقى «معتمدة».
  - الملفات: `crates/ui/src/layer.rs` (الرمز يُحفظ عند الظهور ويُرسل `activate` على الماسك بعد أول إيداع فيه مخزن، ويُمسح في `unmap` فلا يبقى معلّقاً لظهور بلا رمز؛ وسطر `hive surface: layer X Y W H` مرة لكل ظهور مع أول إطار، من `overlay::place` نفسه الذي يعطي `set_position` و`set_destination`؛ و`decide` يأخذ حالة قارئ الشاشة)، و`crates/ui/src/a11y.rs` جديد (خيط يقرأ `org.a11y.Status.ScreenReaderEnabled` من `org.a11y.Bus` على `/org/a11y/bus` بعد الاشتراك في `PropertiesChanged`، ويحفظها في `AtomicBool`، فلا ينتظر أي ظهور الناقل)، و`crates/ui/src/platform.rs` (يسجّل نوع المحوّل لآخر بناء)، و`crates/ui/src/lib.rs` (`Hive::show` يبني مكوّناً جديداً إذا تغيّر قرار السطح عن نوع المكوّن الحالي، كما مع الرمز؛ وسطر `hive surface: window X Y W H` على مسار النافذة عند أول `RedrawRequested` يكون فيه حجم النافذة حجم الخلية، وX Y بافتراض التوسيط على الشاشة لأن عميل Wayland لا يعرف موضعه). السطران مشروطان بـ `SBARHIVE_DEBUG_FOCUS` كسطر `hive focus`.
  - `layer::start` يقرّر إمكان الطبقة دون قارئ الشاشة ويبدأ المراقبة، و`wanted()` عند كل بناء يأخذ القيمة الحية. لذلك يبقى الاتصال قائماً حين يبدأ القارئ مفعّلاً، ويعود الظهور التالي طبقةً حين يتوقف.
  - الاعتمادات: `zbus` 5.19.0 مباشرةً لـ `sbarhive-ui` (`default-features = false`، `async-io` و`blocking-api`)، MIT، موجود في الشجرة عبر accesskit وأيقونة الشريط. `Cargo.lock` لم يُضف أي حزمة (سطر اعتماد واحد). سُجّل في `docs/engines.md`. استُثني `a11y.rs` ودوال `report_surface` و`report_window` و`wants_layer` و`built_on_layer` في `.cargo/mutants.toml` مع السبب (تعمل على ناقل أو مؤلِّف حي فقط).
  - AC10 بالمستوى 4 والرمز الحقيقي، بحث محدود (أربعة تشغيلات لـ `focus.sh 4:token`): (1) التجربة في `spikes/overlay/` كانت ترسل `activate` على الماسك بعد إيداعه الفارغ مباشرةً، كالتطبيق حرفياً، فالترتيب ليس الفرق. ومع ذلك نُقل `activate` إلى ما بعد أول إيداع بمخزن (KWin 6.7.5 يحفظ الرمز على نافذة غير جاهزة للرسم ولا يقرّر)، والنتيجة نفسها. (2) شيفرة KWin 6.7.5 حرفياً (`src/activation.cpp:604`، `Workspace::mayActivate`): `if (!m_activationToken.isEmpty() && token == m_activationToken && input()->lastInteractionSerial() <= m_activationTokenSerial) return true; else if (focusStealingPreventionLevel == FocusStealingPreventionLevel::Extreme) return false;`. (3) أثر `client.py` بـ `WAYLAND_DEBUG=client`: `xdg_activation_token_v1.set_serial(0, wl_seat)` لأن GTK يمرّر رقم آخر ضغطة زر، ولا ضغطة في `CLIENT_AUTO`. وأثر الخدمة: Esc خلية التسخين المحقون `wl_keyboard.key(6, ..., 1, 1)`، فآخر تفاعل 6 > 0 ويرفض KWin الرمز قبل أي فحص للسطح. (4) تجربة ضابطة على البناء المودَع، من نسخة في مجلد مؤقت خارج `tests/` من `focus.sh` حُذف منها ظهور التسخين وEsc والانتظار فقط: `4:token` RESULT PASS، `activate("kwin-2", wl_surface#15)` ثم `wl_keyboard.enter` بعد 5 ms و`hive focus: true`. فالسبب مُثبَت بالقياس، والترتيب المؤجَّل يعمل بالمستوى 4 مع رمز صالح. وهذا يفسّر نجاح S-013 بالمستوى 4 على مسار النافذة (لا إدخال محقون قبل الرمز). في التجربة جاء الرمز بعد نقرة حقيقية، فرقمه أحدث. الخلاصة: KWin بالمستوى 4 يفعّل سطح الطبقة بالرمز إذا كان رقم الرمز أحدث من آخر تفاعل؛ والرفض في `focus.sh` من الأداة (Esc محقون بعد آخر نقرة وقبل رمز رقمه 0)، ولا يصلحه شيء في المحوّل. سؤال للمدير عن الأداة لا عن الجدوى: أن يأخذ `client.py` رمزه بعد نقرة محقونة فيه، أو أن يُغلق التسخين بغير إدخال محقون، أو أن يُقبل الفشل موثّقاً.
  - تشغيل واحد لكلٍّ (`nice -n 19`، بناء `-j 2`): `tests/overlay/kwin.sh` RESULT PASS (68 PASS، ومنها `hive surface: layer 444 219 393 362` في كل ظهور، وصندوق اللقطة `448 223 384 353` داخله)؛ `kwin.sh window` RESULT PASS (`hive surface: window 444 219 393 362`)؛ `kwin.sh a11y` RESULT PASS (القارئ مفعّل: `get_layer_surface 0` و`get_xdg_surface 1`، ثم بعد `false`: `1` و`0`)؛ `kwin.sh input` RESULT PASS؛ `kwin.sh token` RESULT PASS؛ `tests/overlay/gnome.sh` RESULT PASS؛ `tests/overlay/focus.sh` RESULT FAIL (حالة واحدة: `4:token`، والست الأخرى ناجحة). الجدول: `1:token true 4 ms`، `1:notoken true 4 ms`، `4:token false`، `4:notoken false`، `4:bogus false`، `2:notoken true 4 ms`، `3:notoken true 4 ms`. لم يُشغَّل `tests/focus/kwin.sh` ولا `tests/drop/*` ولا e2e ولا `cargo mutants` (حمل الجهاز).
  - البوابات: `cargo fmt --all --check` نظيف، `cargo clippy --workspace --all-targets -- -D warnings` نظيف، `cargo test --workspace` 420 ناجحاً و0 فاشلاً، `scripts/design-lint.sh crates/ui` نظيف.
  - مخاطر مفتوحة: (1) AC10 بالمستوى 4 بالرمز ينتظر قرار المدير، وكذلك `tests/focus/kwin.sh level=4 token` يتأثر بالسبب نفسه منذ أضيف Esc التسخين (لم يُشغَّل). (2) X Y في سطر النافذة افتراض التوسيط لا قياس. (3) على نظام بلا `org.a11y.Bus` قد يفعّل الطلب الأول `at-spi-bus-launcher` كما يفعل GTK؛ وإن لم يُجب الناقل يبقى القارئ `false`. (4) Hyprland وSway لم يُشغَّلا.
- 2026-10-20، المختبِر (S-018c، إصلاح الأداة في AC10 بالمستوى 4 بموافقة المدير): لم تتغير أي فحوص. السبب من سجل المنفِّذ: KWin بالمستوى 4 يقبل الرمز فقط إذا كان `lastInteractionSerial() <=` رقم الرمز (`src/activation.cpp:604`)، و`client.py` كان يأخذ رمزه برقم 0 بعد Esc التسخين المحقون.
  - الملفات: `tests/focus/client.py` (وضع `CLIENT_CLICK=1`: تُكبَّر النافذة، ويحجز `Gtk.GestureClick` الضغطة، لأن `Gtk.Entry` كان يحجزها فتُلغى الإيماءة بلا تحرير، والإطلاق عند التحرير كما يفعل مدير الملفات)، و`tests/overlay/focus.sh` و`tests/focus/kwin.sh` (في حالة `token` وحدها نقرة محقونة في (640، 400) بعد إغلاق التسخين، وأثر العميل في `*-client-wayland.log`؛ و`notoken` و`bogus` بلا تغيير، والمؤشر ساكن)، و`tests/focus/README.md`. وفي `kwin.sh` يبدأ `kwin-input` مرة واحدة قبل التسخين ويبقى مفتوحاً حتى النهاية.
  - التشغيل (`nice -n 19`، بناء `-j 2`): `tests/overlay/focus.sh` كاملاً مرة: نجحت `1:notoken` و`4:notoken` و`4:bogus` و`2:notoken` و`3:notoken`، وفشلت حالتا `token` لأن الأداة كانت معطوبة (الإيماءة أُلغيت فلا إطلاق). بعد حجز الضغطة أُعيدت حالتا `token` فقط: `1:token` RESULT PASS و`4:token` RESULT PASS (`activate("kwin-2", wl_surface#24)`، ثم `wl_keyboard.enter` بعد 5 ms، و`hive focus: true`). أثر العميل: الضغطة `button(21, ..., 1)` والتحرير `button(22, ..., 0)` ثم `set_serial(21, wl_seat)`، فالتحرير لا يُحسب تفاعلاً أحدث. `tests/focus/kwin.sh token level=4`: `daemon focus lines: hive focus: true`، الرمز `kwin-2` برقم 21. الجدول: `1:token true 5 ms`، `1:notoken true 4 ms`، `4:token true 5 ms`، `4:notoken false`، `4:bogus false`، `2:notoken true 3 ms`، `3:notoken true 3 ms`.
- 2026-10-20، المختبِر (S-018d فقط، AC13 وAC14): كُتبت الاختبارات من المعايير وشُغّل كل سكربت جديد أو معدَّل مرة واحدة على بناء `6bdf20a` (`cargo build --release -j 2`، `nice -n 19`). تفشل فحوص جهاز البيانات على اتصال الطبقة وكل فحوص الإفلات الفعلي في KWin، لأن المستقبِل لا يُربط على مسار الطبقة (سجل المنفِّذ في S-018b، الخطر 3). وتنجح اليوم فحوص الماسك في وضع الإفلات (لوحة المفاتيح 0، منطقة الإدخال الفارغة، النقرة تصل GTK والخلية باقية)، لأن S-018b سلّمتها مسبقاً (سجل المنفِّذ: «لوحة المفاتيح 1 أو 0 في ظهور الإفلات مع منطقة إدخال فارغة»)، فهي حارسات انحدار كما في AC8 وAC9، ولم يُختلق شرط زائد ليفشلها.
  - الملفات: `tests/drop/layer.sh` جديد (الدالة `layer_drop_show N FROM TO ENTER` يستعملها السكربتان)، ووضع `drop` جديد في `tests/overlay/kwin.sh` (السطور 26 إلى 32 في الرأس، والسطران 35 و44 لقائمة الأوضاع، والسطور 478 إلى 512)، وتعديل `tests/drop/kwin.sh` و`tests/drop/lib.sh` و`tests/drop/README.md`.
  - تعديل الاختبارات القائمة (التأكيدات كما هي):
    1. `tests/drop/lib.sh:160` (كان السطر 160 وحده، صار 160 إلى 162): سطح الظهور الجديد هو آخر `get_xdg_surface` **أو** `get_layer_surface` حتى نهاية الظهور، بالبحث نفسه في الأثر كله. يخدم السطرُ المسارين: على KWin صفر `get_xdg_surface` (S-018 AC4)، وعلى GNOME صفر `get_layer_surface` (AC6)، والشرط `dd > xs` كما هو. و`:165` (كان 163) نص سطر INFO فقط. لا تغيير آخر في `lib.sh`.
    2. `tests/drop/kwin.sh:47` إلى `:59` (كانت 46 إلى 63، ومنها السطر 51): `hive_geo` لا يمكن أن يأتي من سكربت KWin، لأن نافذة الطبقة في `windowList` هي الماسك بملء الشاشة واسمها فارغ. صار يقرأ سطر `hive surface: layer X Y W H` من سجل الخدمة (S-018 AC11)، والخلية ظاهرة ما دام عدد هذه الأسطر أكبر من عدد `zwlr_layer_surface_v1.destroy` في الأثر، فيبقى `wait_gone` (شرط AC7) صحيحاً. لذلك أُضيف `SBARHIVE_DEBUG_FOCUS=1` إلى تشغيل الخدمة في `:78` (كان 82)، لأن السطر لا يُطبع دونه. وفي `:2` و`:3` سطر الرأس، و`:91` إلى `:96` جديدة بعد `protocol_checks`: فحوص `layer.sh` لكل ظهور بـ `ENTER=1`.
  - العقد الذي تثبّته الاختبارات (لا يُعدَّل)، لكل ظهور بوضع الإفلات على KWin (`layer.sh`):
    1. `get_layer_surface` واحد بالضبط، وكل `set_keyboard_interactivity` في الظهور بالقيمة 0 (واحد على الأقل).
    2. منطقة إدخال فارغة على الماسك (الـ `wl_surface` الممرَّر إلى `get_layer_surface`): `wl_compositor.create_region(new id wl_region#R)` ثم `<الماسك>.set_input_region(wl_region#R)` بلا `wl_region#R.add` بينهما. `set_input_region(nil)` لا يُقبل (منطقة لا نهائية).
    3. `wl_data_device_manager.get_data_device` واحد بالضبط في الظهور، بعد `get_layer_surface` فيه.
    4. كل `wl_data_device` أُنشئ قبله في الأثر كله يُرسَل له `wl_data_device#N.release()` بعد إنشائه وقبل `get_data_device` الجديد. فالجهاز السابق يُحرَّر قبل الجديد، ولا يُستثنى الظهور الأول.
    5. AC14 (`tests/drop/kwin.sh` وحده، بسحب حقيقي): حدث `enter` على جهاز هذا الظهور نفسه يسمّي السطح الفرعي للخلية في هذا الظهور (أول `wl_surface` في `get_subsurface`). هذا دليل «اتصال الطبقة»: الأثر فيه أربعة اتصالات بمعرّفات متكررة، ولا يخبر المؤلِّفُ بسطح الخلية إلا جهازاً على اتصاله، فالمطابقة تجري داخل قطعة الظهور.
    6. `tests/overlay/kwin.sh drop`: ثلاثة ظهورات `sbarhive drop` بلا سحب، كلٌّ يُغلق بمهلة 10 ث (لا لوحة مفاتيح فلا Esc). في الأول نقرة في (100، 700) بعد 1.5 ث: نقرة واحدة عند GTK، وصفر `wl_pointer.enter` على الماسك وصفر `wl_pointer.button` في الخدمة، وصفر `wl_keyboard.enter`، وهدم طبقة واحد بالضبط بعد 9000 ms أو أكثر من `get_layer_surface` (المهلة لا النقرة). ولا خطأ بروتوكول.
    7. `tests/drop/kwin.sh level=1`: كل فحوص S-014b كما هي على مسار الطبقة، وموقع الخلية من سطر `hive surface` (444 219 393 362 في هذا التشغيل)، فيجب أن يُطبع السطر في كل ظهور إفلات كما في AC11.
  - ما حُسم حيث سكت المعيار: «لوحة مفاتيح 0» تعني أن كل طلب `set_keyboard_interactivity` في ظهور الإفلات بالقيمة 0؛ «المنطقة الفارغة» تعني منطقة بلا `add`؛ «الخلية باقية» تعني أن الهدم من المهلة (9 ث أو أكثر بعد الظهور)؛ «الجهاز السابق يُحرَّر قبله» يشمل كل جهاز سابق في الأثر.
  - تشغيل واحد لكلٍّ:
    - `nice -n 19 tests/overlay/kwin.sh drop`: RESULT FAIL (6).
      ```
      INFO S-018d show 1: catcher wl_surface#15, hive subsurface wl_surface#19, keyboard_interactivity 0 x1 other x0, input region wl_region#17 (adds 0), get_data_device x0 none at line none, unreleased before it: none
      PASS AC13 show 1: exactly one get_layer_surface (drop mode is a layer)
      PASS AC13 show 1: the catcher takes no keyboard (every set_keyboard_interactivity is 0)
      PASS AC13 show 1: the catcher has an empty input region (create_region, no add, set_input_region)
      FAIL AC13 show 1: exactly one get_data_device, after get_layer_surface
      FAIL AC13 show 1: every earlier wl_data_device is released before it
      (الظهوران 2 و3 بالفشل نفسه)
      INFO drop1: GTK clicks 1, layer destroys 1, shown for 10005 ms, pointer enter on the catcher 0, pointer buttons 0, keyboard enter 0
      PASS AC13 drop mode: the click at (100, 700) reaches the GTK window (one click)
      PASS AC13 drop mode: the catcher gets no pointer enter and the daemon no button
      PASS AC13 drop mode: the catcher gets no keyboard (no wl_keyboard.enter)
      PASS AC13 drop mode: the hive stays after the click (closed once, by the 10 s timeout, not by the click)
      PASS AC13: no protocol error in the daemon log
      RESULT FAIL (6)
      ```
    - `nice -n 19 tests/drop/kwin.sh level=1`: RESULT FAIL (28). `hive_geo` الجديد وجد الخلية في كل ظهور (`INFO show 1: hive at 444 219 393x362`)، فالأداة سليمة والفشل من غياب المستقبِل:
      ```
      FAIL AC7 show 1: photo one.webp written next to the original within 5 s
      FAIL AC7 show 1: output is image/webp
      FAIL AC7 show 2: drop on the corner closes the hive
      FAIL AC7 show 3: photo three.webp written next to the original within 5 s
      FAIL AC7 show 4: drop on the centre closes the hive
      INFO show 1: hive surface (get_xdg_surface or get_layer_surface) at line 407, get_data_device at line none, count 0
      FAIL AC5 show 1: wl_data_device requested for the new hive surface
      FAIL AC6 show 1: receiver got enter on its data device
      (الظهورات 2 إلى 4 بالفشل نفسه)
      FAIL AC6: set_actions(Copy, Copy) sent
      FAIL AC6: accepts "text/uri-list" and no other type
      FAIL AC13 show 1: exactly one get_data_device, after get_layer_surface
      FAIL AC13 show 1: every earlier wl_data_device is released before it
      FAIL AC14 show 1: the drag enters this show's data device on the hive subsurface (layer connection)
      (الظهورات 2 إلى 4 بالفشل نفسه، وفحوص لوحة المفاتيح 0 والمنطقة الفارغة ناجحة في الأربعة)
      RESULT FAIL (28)
      ```
    - `nice -n 19 tests/drop/gnome.sh smart` (لأن `lib.sh` المشترك تغيّر): RESULT PASS، حارس AC14 على مسار النافذة.
  - ملاحظات للمنفِّذ والمراجع: (1) السطح الفرعي للخلية `wl_surface#19` نفسه في كل الظهورات (يُعاد استعماله ويُنشأ له `wl_subsurface` جديد)، والاختبار لا يشترط سطحاً جديداً. (2) `tests/drop/kwin.sh` لا يدعم اليوم `SBARHIVE_SURFACE=window` على KWin (`hive_geo` من سطر الطبقة وحده)، لأن AC14 يطلب مسار الطبقة؛ مسار النافذة يحرسه `gnome.sh`. (3) السكربتان `tests/drop/*.sh` يتركان مجلدَي `mktemp` في `/tmp` كما قبل هذه القصة؛ حُذفت مجلدات تشغيلَيّ يدوياً. (4) فحص التحرير يطابق معرّفات `wl_data_device` في الأثر كله، وفيه أربعة اتصالات. اليوم جهاز المستقبِل هو الوحيد في كل أثر؛ إن ربط شيء آخر في العملية جهاز بيانات دائماً (حافظة على اتصال winit مثلاً) ظهر «غير محرَّر» وفشل الفحص لسبب لا يخص AC13، وسطر INFO يسمّي المعرّف وسطره فيُميَّز سريعاً. (5) بقاء الفحوص الناجحة اليوم حارساتٍ قرار للمدير، لا للمختبِر. (6) لم يُشغَّل `tests/overlay/kwin.sh` بوضعه الافتراضي ولا `focus.sh` ولا e2e ولا `cargo mutants` (حمل الجهاز)، ولم يُعدَّل أي فحص فيها.
- 2026-10-20، المنفِّذ (S-018d فقط، AC13 وAC14): S-018d جاهزة للمراجعة (الحالة: مراجعة للقصة الفرعية؛ رأس الملف يبقى «معتمدة» لأن S-018e وS-018f وS-018h لم تنته). لم يُعدَّل أي اختبار، وبقيت فحوص الماسك في وضع الإفلات حارساتٍ بقرار المدير.
  - الملفات: `crates/ui/src/receiver.rs` (مستقبِل واحد لكل `wl_display`: عرض winit على مسار النافذة، وعرض اتصال الطبقة على مسار الطبقة، كلٌّ باتصال `from_foreign_display` وطابوره وخيطه كما كان؛ `attach` يحرّر كل جهاز سابق في كل المستقبِلات ثم يطلب جهازاً واحداً على العرض المطلوب؛ والخطة في `Mutex` عامّ بدل الحقل والقيمة المبكرة)، و`crates/ui/src/layer.rs` (في `map()` لظهور الإفلات، بعد `get_layer_surface` والسطح الفرعي وإيداع الماسك، يُربط المستقبِل بمؤشر عرض اتصال الطبقة وبالسطح الفرعي للخلية؛ وخيط القراءة إذا فشل `prepare_read` لأن خيط المستقبِل قرأ أحداث الطبقة على العرض نفسه يطلب `dispatch` بدل الانتظار، وإلا بقيت الأحداث في الطابور بلا من يوزّعها)، و`crates/ui/src/lib.rs` (`attach_layer_receiver` يستهلك `ATTACH_PENDING` كما يفعل `RedrawRequested` على مسار النافذة، الذي بقي كما هو)، و`.cargo/mutants.toml` (استثناء `attach_layer_receiver` بسبب، فهي تعمل على اتصال طبقة حي فقط؛ `receiver.rs` و`layer.rs` مستثنيان أصلاً).
  - تشغيل واحد لكلٍّ على بناء `cargo build --release -j 2` بعد آخر تعديل (`nice -n 19`): `tests/overlay/kwin.sh drop` RESULT PASS (في الظهورات الثلاثة `get_data_device x1` بعد `get_layer_surface`، و`unreleased before it: none`)؛ `tests/drop/kwin.sh level=1` RESULT PASS (53 PASS، ومنها AC14 في الظهورات الأربعة: `enter` على جهاز الظهور يسمّي السطح الفرعي للخلية)؛ `tests/drop/gnome.sh smart` RESULT PASS؛ `tests/overlay/kwin.sh` (الافتراضي) RESULT PASS (68 PASS)؛ `tests/focus/kwin.sh` (المستوى 1، notoken وtoken؛ سكربت مشاهدة بلا سطر `RESULT`): خرج بـ 0 بلا أي FAIL أو error، و`hive focus: true` في الحالتين، وخلية التسخين أُغلقت؛ `tests/e2e/run.sh`: `e2e: all passed` (الذاكرة 28 MB عند البدء و37 MB بعد الاستعمال). لم يُشغَّل `cargo mutants` (حمل الجهاز). حُذفت مجلدات `mktemp` العشرة التي تركتها التشغيلات، ولم يبق مؤلِّف مخفي.
  - البوابات: `cargo fmt --all --check` نظيف، `cargo clippy --workspace --all-targets -- -D warnings` نظيف، `cargo test --workspace` 420 ناجحاً و0 فاشلاً، `scripts/design-lint.sh crates/ui` نظيف.
  - مخاطر مفتوحة: (1) أول ظهور إفلات على مسار الطبقة في العملية يبدأ المستقبِل بذهاب وإياب للسجل (`registry_queue_init`) على خيط Slint داخل `show()`، فيُحسب على ميزانية 150 ms لذلك الظهور وحده (لم يُقس؛ S-018f). (2) خيط ثالث يقرأ اتصال الطبقة، وربط ثانٍ لـ `wl_seat` عليه. (3) إن تغيّر المسار داخل العملية (قارئ الشاشة) يبدأ مستقبِل ثانٍ على عرض winit ويبقى الأول؛ `attach` و`detach` يحرّران جهازَي الاثنين، ولم يُختبر هذا الانتقال بسحب حقيقي. (4) تعديل خيط القراءة يمس كل ظهور على مسار الطبقة بعد بدء خيط المستقبِل؛ الوضع الافتراضي لـ `tests/overlay/kwin.sh` فيه ظهورات عادية بعد ظهور الإفلات ونجح، ولم تُشغَّل أوضاع `input` و`token` و`a11y` و`window` ولا `tests/overlay/gnome.sh` ولا `tests/overlay/focus.sh` (حمل الجهاز)، فهي للمراجع. (5) Hyprland وSway لم يُشغَّلا.
- 2026-10-20، المختبِر (S-018e فقط، AC15 إلى AC17): كُتبت الاختبارات من المعايير وشُغّل كل وضع جديد أو معدَّل مرة واحدة على بناء `f32d7da` (`cargo build --release -j 2`، `nice -n 19`). يفشل اليوم فحصان من AC15 (`SBARHIVE_COLOR_SCHEME` لا يسبق البوابة) وفحص AC16 (لا `set_shape` أبداً). وتنجح اليوم فحوص البوابة نفسها في AC15 وكل فحوص AC17، لأن السلوك موجود فعلاً في الأثر واللقطة (التفاصيل أدناه)، فهي حارسات انحدار، ولم يُختلق شرط زائد ليفشلها. قرار بقائها للمدير.
  - الملفات: `tests/overlay/portal.py` جديد (بوابة وهمية)، و`tests/overlay/pixel.py` جديد (يقرأ رقعة 3 × 3 من لقطة `shot.py` ويقارنها بـ `color.dark.surface-3` و`color.light.surface-3` من `design/tokens.json` بفرق 3 مستويات)، ووضعان جديدان في `tests/overlay/kwin.sh` (`scheme` في السطور 626 إلى 687، و`1.5` في السطور 689 إلى 745، وحلقة المرحلتين في 763 إلى 782)، وفحوص AC16 في الوضع الافتراضي (السطور 231 إلى 237 لحركة المؤشر، و336 إلى 387 للفحص). وفي الرأس: `SBARHIVE_COLOR_SCHEME` أضيف إلى `unset` (السطر 73)، والبوابة تبدأ قبل KWin، وKWin يأخذ `--width`/`--height` من متغيرين. لا لقطات مرجعية جديدة.
  - العقد الذي تثبّته الاختبارات (لا يُعدَّل):
    1. AC15 (`kwin.sh scheme`): `portal.py` يملك `org.freedesktop.portal.Desktop` قبل KWin و`other.py`، على `/org/freedesktop/portal/desktop`: `org.freedesktop.portal.Settings` بالإصدار 2، و`ReadAll(as)` و`Read(ss)` (متغيّر داخل متغيّر) و`ReadOne(ss)`، والإعداد الوحيد `org.freedesktop.appearance` `color-scheme` من النوع `u`، وغيره `org.freedesktop.portal.Error.NotFound`. عند التغيير يرسل `SettingChanged(s, s, v)`. سبعة ظهورات بـ `theme = "auto"` إلا حيث يُذكر، كلٌّ منها `get_layer_surface` واحد وصفر `get_xdg_surface`، ولقطة بعد 1.5 ث والمؤشر في (200، 200): البوابة 1 داكن؛ `SettingChanged` إلى 2 والخلية مخفية ثم فاتح؛ إلى 1 ثم داكن؛ `theme = "light"` والبوابة 1 فاتح؛ `theme = "dark"` والبوابة 2 داكن؛ ثم الخدمة تُعاد بـ `SBARHIVE_COLOR_SCHEME=dark` والبوابة 2 داكن؛ ثم بـ `light` والبوابة 1 فاتح.
    2. AC15، موضع العيّنة: رقعة 3 × 3 في (640، 400 - round(0.72 r)) = (640، 370)، و r نصف قطر الخلية `hex.size.md / sqrt(3)`، أي داخل الخلية المركزية وفوق المربع الذي تأخذه المعاينة في S-018h (AC24). في المراجع الأربعة `hive-*.png` هي `surface-3` كالمركز تماماً. المركز الحرفي (640، 400) يُطبع INFO فقط: في التشغيل كان `#F2ECE0` في الداكن و`#1F1A12` في الفاتح، أي محتوى مرسوم في المركز (الأيقونة)، فبكسل المركز الحرفي لا يصلح لـ «surface-3». هذا انحراف عن نص المعيار، سؤال للمدير.
    3. AC16 (`kwin.sh` الافتراضي): في كل ظهور غير الإفلات، بعد اللقطة، المؤشر إلى (201، 201) ثم (640، 400) ثم (200، 200) قبل Esc. كل `wl_pointer.enter(S, <الماسك أو السطح الفرعي للخلية في الظهور نفسه>, ...)` يتبعه في الأثر `wp_cursor_shape_device_v1#N.set_shape(S, 1)` قبل أي `wl_pointer.enter` تالٍ. المطابقة بالرقم التسلسلي لأن معرّفات الكائنات تتكرر بين اتصالات الأثر الأربعة. وفحص حارس: دخول واحد على الأقل على الماسك وواحد على الخلية في الأثر، فلا ينجح الفحص فارغاً.
    4. AC17 (`kwin.sh 1.5`): مرحلتان بمؤلِّفين خاصّين. المرجع: KWin بمقياس 1 (1280×800) وظهور واحد، وصندوق اللقطة يُحفظ في `out/kwin-1.5-ref.box`. الرئيسية: KWin بمخرج افتراضي 1920×1200 ومقياس 1.5 مزروع في `kwinoutputconfig.json` الخاص، و`kscreen-doctor -o` يقرأ `Scale: 1.5` و`Geometry: 0,0 1280x800`. في الظهور: `get_layer_surface` واحد وصفر `get_xdg_surface`؛ حدث `wp_fractional_scale_v1.preferred_scale(180)` قبل أول `zwlr_layer_surface_v1.configure`؛ كل `wl_shm_pool.create_buffer` في الظهور 590 × 543 (واحد على الأقل) وواحد منها على الأقل `attach` على السطح الفرعي للخلية؛ `set_position(444, 219)` و`set_destination(393, 362)`؛ عرض صندوق اللقطة الفيزيائي يساوي 1.5 × عرض صندوق المرجع بفرق 1 px؛ والصندوق متمركز في (960، 600) بفرق 1 px وداخل (666، 328.5، 589.5، 543).
  - ما حُسم حيث سكت المعيار أو خالفه القياس: (1) `kscreen-doctor output.Virtual-0.scale.1.5` (وبالمعرّف `output.1`) في KWin 6.7.5 المخفي يخرج بـ 0 بلا أي مخرجات، ولا يرسل أي طلب `kde_output_configuration` (أثر `WAYLAND_DEBUG`)، والمقياس يبقى 1. وخيار `--scale` في `kwin_wayland --virtual` يكبّر المخزن ويترك مقياس المخرج 1 (`--width 1920 --scale 1.5` أعطى 2880×1800 بمقياس 1). لذلك يُزرع المقياس في إعداد المخرجات الخاص ويُقرأ بـ `kscreen-doctor -o`. (2) «عرض صندوقها في اللقطة يساويه» حرفياً (عرض الصندوق = عرض المخزن 590) لا يتحقق حتى مع تنفيذ صحيح: بكسلات الخلية غير الشفافة أضيق من السطح (384 مقابل 393 بمقياس 1)، فالفحص صار 1.5 × صندوق المقياس 1، والحدّة يثبتها حجم المخزن في الأثر. سؤال للمدير. (3) أسبقية `SBARHIVE_COLOR_SCHEME` على `theme` في الإعداد لم تُثبَّت: كل فحص متغيّر يجري بـ `theme = "auto"`.
  - تشغيل واحد لكلٍّ:
    - `nice -n 19 tests/overlay/kwin.sh scheme`: RESULT FAIL (2)، و13 PASS.
      ```
      INFO show 1 (portal-1): get_layer_surface 1, get_xdg_surface 0, patch at (640, 370) dark #322C23, centre (640, 400) other #F2ECE0
      PASS AC15 show 1 (portal-1): the centre cell is color.dark.surface-3
      (الظهورات 2 إلى 5 ناجحة: فاتح، داكن، فاتح، داكن)
      INFO show 6 (env-dark-portal-2): get_layer_surface 1, get_xdg_surface 0, patch at (640, 370) light #DFD3BB, centre (640, 400) other #1F1A12
      PASS AC15 show 6 (env-dark-portal-2): a layer surface (not the window path)
      FAIL AC15 show 6 (env-dark-portal-2): the centre cell is color.dark.surface-3
      INFO show 7 (env-light-portal-1): get_layer_surface 1, get_xdg_surface 0, patch at (640, 370) dark #322C23, centre (640, 400) other #F2ECE0
      PASS AC15 show 7 (env-light-portal-1): a layer surface (not the window path)
      FAIL AC15 show 7 (env-light-portal-1): the centre cell is color.light.surface-3
      PASS AC15: no protocol error in the daemon log
      RESULT FAIL (2)
      ```
      البوابة تعمل اليوم على مسار الطبقة (16 استدعاء في سجل `portal.py`، والظهورات الخمسة الأولى تتبع القيمة و`SettingChanged` والإعداد)، خلافاً لخطر S-018b رقم 1. أما المتغيّر فتتجاهله الخدمة على مسار الطبقة: الظهوران 6 و7 يتبعان البوابة لا `SBARHIVE_COLOR_SCHEME`، وهذا يخالف AC3 «`SBARHIVE_COLOR_SCHEME` يسبق البوابة».
    - `nice -n 19 tests/overlay/kwin.sh` (الافتراضي): RESULT FAIL (1)، و69 PASS (الـ 68 السابقة كلها ناجحة مع حركة المؤشر الجديدة، وفحص الوصول في AC16).
      ```
      INFO AC16 show 1: catcher wl_surface#15, hive wl_surface#18, pointer enters on the catcher 2 on the hive 1, followed by set_shape(serial, 1) 0
      INFO AC16 show 3: catcher wl_surface#25, hive wl_surface#17, pointer enters on the catcher 0 on the hive 0, followed by set_shape(serial, 1) 0
      (الظهورات 2 و4 و5 مثل الأول)
      INFO AC16 enters without set_shape(serial, 1): show 1 catcher enter serial 9, show 1 hive enter serial 14, show 1 catcher enter serial 16, ... show 5 catcher enter serial 70
      PASS AC16: the pointer entered the catcher and the hive (the test reached both)
      FAIL AC16: every wl_pointer.enter on the catcher or the hive is followed by wp_cursor_shape_device_v1.set_shape(<serial>, 1)
      INFO hive pixel box: 448 223 384 353
      RESULT FAIL (1)
      ```
    - `nice -n 19 tests/overlay/kwin.sh 1.5`: RESULT PASS (8 PASS). `preferred_scale(180)` في السطر 15 من القطعة وأول `configure` في 16، ومخزنا الخلية `590x543`، وسطر `hive surface: layer 444 219 393 362`، والصندوق `672 335 576 530` مقابل المرجع `448 223 384 353` (576 = 1.5 × 384). فمحوّل الطبقة من S-018b يطبّق المقياس الكسري أصلاً (`overlay::place` بـ `scale120`). الفحوص الثمانية حارسات، أو يُعتبر AC17 منجزاً في S-018b، قرار للمدير كما في AC8 وAC9.
  - ملاحظات للمنفِّذ والمراجع: (1) الوضع الافتراضي صار أطول بنحو 0.9 ث لكل ظهور (حركة المؤشر). (2) `kwin.sh 1.5` يكتب `out/kwin-1.5.txt` من المرحلتين، و`RESULT` من الرئيسية وحدها. (3) لم تُشغَّل الأوضاع `window` و`input` و`token` و`a11y` و`drop` ولا `gnome.sh` ولا `focus.sh` ولا e2e ولا `cargo mutants` (حمل الجهاز)؛ لم يتغير فيها إلا `unset SBARHIVE_COLOR_SCHEME` ومتغيرا عرض KWin وارتفاعه (القيمة نفسها 1280×800). (4) مجلدا `mktemp` الموجودان في `/tmp` قبل التشغيل ليسا من هذه التشغيلات، وسكربت `kwin.sh` يحذف مجلداته.
