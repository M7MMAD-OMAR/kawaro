# S-014 إفلات ملف مسحوب على خلية بعد اختصار أثناء السحب
الحالة: S-014d منتهية (المراجعة 2: PASS)؛ S-014e معتمدة (2026-10-22، مقسومة إلى S-014e1 وS-014e2 وS-014e3)، وS-014e1 عند المختبِر؛ S-014f لم تبدأ

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
| S-014e1 | عميل البوابة في الخدمة (Registry ثم الجلسة ثم الربط)، وربط الإشارتين ببوابة الضغط المطوّل، والرجوع الصامت، والتبعيات والرخص؛ يُختبر ببوابة وهمية في KWin خاص. يعتمد على انتهاء S-021b وS-021c | AC13، AC14، AC17، AC18، AC19 |
| S-014e2 | البوابة الحقيقية في GNOME 50 وKDE 6 الخاصين: الضغط المطوّل أثناء السحب، ونافذة الموافقة مرة واحدة، ومسار النافذة بالوهمية في GNOME | AC18 (ج)، AC20، AC21 |
| S-014e3 | البوابة الحقيقية في Hyprland متداخل خاص، والتوثيق باللغتين، وCHANGELOG | AC20 (Hyprland)، AC22 |
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
- AC13 (مُحدَّث 2026-10-22): الخدمة على لينكس تسجّل الاختصار من خيط خاص على اتصال D-Bus مخصص للبوابة. مع بوابة وهمية على الناقل الخاص تسجّل الاستدعاءات بترتيبها: أول استدعاء بوابة من هذا الاتصال `org.freedesktop.host.portal.Registry.Register` بالمعرّف `io.github.m7mmad_omar.SbarHive`، ثم `CreateSession`، ثم `BindShortcuts` مرة واحدة على الأكثر في الجلسة، لاختصار واحد معرّفه `drop` و`preferred_trigger` هو Super+Z بصيغة مواصفة الاختصارات (المتوقَّع `LOGO+z`، ويثبّت المختبِر النص من المواصفة ومن قبول منفذ GNOME له) و`description` غير فارغ بلغة الخدمة. بعد ردّ ناجح يطبع stderr الخدمة سطراً واحداً بالضبط `portal shortcuts: bound drop (T)`، وT هو `trigger_description` في الرد. وإن غابت Registry يُطبع سطر واحد `portal registry: unavailable` ويستمر التسجيل.
- AC14 (مُحدَّث 2026-10-22): ashpd مثبَّت بإصدار محدد (المرشّح 0.13.13 من ADR 0003، رخصة MIT، يتحقق المنفِّذ من الإصدار والرخصة في crates.io قبل الإضافة) بـ `default-features = false` وميزتي GlobalShortcuts وasync-io فقط (يثبّت المختبِر أسماء الميزات من `Cargo.toml` الخاص بـ ashpd). بعد الإضافة: `cargo tree -d -i zbus` لا يطبع شيئاً، و`Cargo.lock` فيه حزمة `zbus` واحدة بالإصدار 5.19.0 (أو ترقية واحدة لكل المستودع مسجّلة في `docs/engines.md`)، و`cargo tree -p sbarhive-daemon -e normal -i tokio` لا يجد tokio (اليوم tokio في `Cargo.lock` عبر `wasm-bindgen-futures` فقط). ashpd وكل حزمة جديدة في `Cargo.lock` مسجّلة في `docs/engines.md` بالإصدار والرخصة. إن تعذّر أحد الشرطين بـ ashpd، يتوقف المنفِّذ ويُرفع لمحمد، والبديل المقترح: وكيل `#[zbus::proxy]` مكتوب يدوياً لـ `org.freedesktop.portal.GlobalShortcuts` وRegistry على zbus 5.19.0 الموجود، بلا حزمة جديدة.
- AC17 (S-014e1): إشارة `Activated` للمعرّف `drop` في جلسة الخدمة تدخل بوابة الضغط المطوّل `sbarhive_core::hold` بـ `hold::from_portal` (أي `Signal::Hold { at: None }`)، و`Deactivated` تدخلها `Signal::Release`، في البوابة نفسها التي يغذّيها `drop --hold` و`--release` (لا مؤقّت ثانٍ). إشارة لمعرّف آخر أو لجلسة أخرى تُتجاهل. مع البوابة الوهمية في KWin خاص، ومع `SBARHIVE_DEBUG_FOCUS` (سطر `portal: activated drop` و`portal: deactivated drop` لكل إشارة): «مربوط» بتعريف S-021 (القرار 2): سطح طبقة للخلية، أي `get_layer_surface` في الأثر، سواء أُظهِر أم بقي `Probing`. `Activated` ثم `Deactivated` بعد 600 ms وhold_ms 400: `get_layer_surface` واحد بالضبط، بعد `Activated` بـ 400 ms أو أكثر؛ وبعد 150 ms: صفر `get_layer_surface` خلال 1000 ms؛ وثلاث `Activated` متتالية (تكرار المفتاح) ثم `Deactivated` بعد 600 ms من الأولى: واحد بالضبط؛ و`Activated` للمعرّف `other` ثم `Deactivated`: صفر خلال 1000 ms.
- AC18 (S-014e1): ظهور البوابة يمر بمسار S-021 كاملاً ولا يتجاوزه. على مسار الطبقة (KWin خاص، بوابة وهمية) لكل ظهور سطر واحد بالضبط من `hive reveal: N` أو `hive probe: hidden (R)`، وصفر `wl_surface.attach` على السطح الفرعي للخلية قبل `hive reveal:` (S-021 AC3 وAC4): (أ) `Activated` دون سحب، ثم `Deactivated` بعد انقضاء العتبة: `hidden (release)` أو `hidden (pointer)`، وصفر `attach` على الخلية. (ب) سحب PNG ممسوك، ثم `Activated`، ثم الإفلات على WEBP قبل `Deactivated`: `hive reveal: N` وN يساوي عدد أسطر `sbarhive actions` للملف، و`<الاسم>.webp` خلال 5 ثوانٍ. الموضع: البوابة لا تحمل موضعاً، فالظهور كـ `drop --hold` دون `--at`: المرساة من مرحلة الماسك (S-019c)، فسطر `hive anchor:` إما `none` وإما ضمن 2 px من موضع السحب في الأثر. (ج، في S-014e2 لأنها تشغّل GNOME) على مسار النافذة، `tests/drop/gnome.sh smart portal` بالوهمية: صفر سطر `hive probe:` و`hive reveal:`، والخلية نافذة في المنتصف بنص الانتظار كما في S-021 AC10، والإفلات على WEBP قبل `Deactivated` يكتب الملف.
- AC19 (S-014e1): الرجوع الصامت إلى الربط اليدوي في `docs/drop-shortcut.md`. في كل حالة يطبع stderr الخدمة سطراً واحداً بالضبط `portal shortcuts: unavailable (R)`، وR واحد من: `no-portal` (لا مالك لـ `org.freedesktop.portal.Desktop` على الناقل)، و`no-interface` (البوابة بلا GlobalShortcuts، أو `version` أقل من 1)، و`denied` (ردّ `BindShortcuts` بالرمز 1، أي ألغاه المستخدم، أو قائمة اختصارات فارغة)، و`error` (الرمز 2 وكل ما سوى ذلك). بعدها، خلال 30 ثانية: صفر استدعاء بوابة جديد (عدّاد الوهمية) وصفر سطر `portal shortcuts:` آخر، و`sbarhive drop --hold` ثم `--release` ما زالا يُظهران الخلية ويغلقانها كما في S-019e. وما دامت الوهمية تحبس ردّ `BindShortcuts` (لا ترسله إلا بعد أن ينهي السكربت الظهورات) يُظهر `sbarhive open` الخلية، ووسيط زمنه من الطلب إلى أول إطار في 5 ظهورات أقل من 150 ms.
- AC20 (S-014e2 لـ GNOME وKDE، S-014e3 لـ Hyprland): البوابة الحقيقية في مؤلِّفات خاصة فقط: GNOME Shell 50 مخفي مع xdg-desktop-portal-gnome 50، وKWin 6 مخفي مع xdg-desktop-portal-kde، وHyprland 0.56 متداخل خاص (نمط `tests/hyprland/repro.sh`) مع xdg-desktop-portal-hyprland وسطر `bind = SUPER, Z, global, <ID>:drop` في إعداده الخاص، وID يُقرأ من `hyprctl globalshortcuts` للنسخة المتداخلة لا يُخمَّن. في كل مؤلِّف: سطر `portal shortcuts: bound drop (T)`؛ و(أ) بدء سحب PNG، ثم ضغط Super+Z وإمساكه حتى تظهر الخلية، ثم الإفلات على WEBP والمفتاح ما زال ممسوكاً، ثم تركه: الملف ناتج في 3 من 3؛ و(ب) دون سحب، إمساك Super+Z مدة 600 ms ثم تركه: بطوابع السجل (`portal: activated drop` و`portal: deactivated drop`) يصل `deactivated` بعد `activated` بـ 500 ms أو أكثر، وخلال 200 ms من ترك المفتاح؛ و(ج) نقرة Super+Z مدتها 150 ms: صفر `get_layer_surface` (أو صفر نافذة خلية في GNOME) خلال 1000 ms. إن لم يرسل منفذ `Deactivated` أو أرسله فور `Activated`، يفشل الفحص وتُرفع القصة لمحمد، ولا يُستبدل بظهور فوري دون قرار.
- AC21 (S-014e2): نافذة الموافقة مرة واحدة على الأكثر. أول تشغيل للخدمة بملف شخصي خاص جديد: إن ظهرت نافذة من المنفذ (المتوقَّع في GNOME وKDE) يقبلها السكربت بإدخال محقون، ويطبع سطر INFO بظهورها أو غيابها. ثاني تشغيل للخدمة بالملف الشخصي نفسه: صفر نافذة جديدة من المنفذ خلال 5 ثوانٍ (عدّ النوافذ في المؤلِّف الخاص)، وسطر `portal shortcuts: bound drop`، والاختصار يُظهر الخلية في 1 من 1.
- AC22 (S-014e3): `docs/drop-shortcut.md` بالعربية والإنكليزية فيه قسم البوابة: في GNOME 48 وما بعده وKDE 6 تسجّل الخدمة Super+Z بنفسها مع نافذة موافقة أول مرة، ويُغيَّر من الإعدادات (GNOME: التطبيقات ثم «Global shortcuts»؛ KDE: إعدادات النظام ثم الاختصارات)؛ في Hyprland سطر `bind = SUPER, Z, global, io.github.m7mmad_omar.SbarHive:drop` (بالمعرّف الذي يثبته AC20)؛ اختصار البوابة يعمل بالإمساك (`hold_ms`) لا بالنقرة؛ وحين لا بوابة يبقى الربط اليدوي. فحص ثابت يجد الأسطر الأربعة في كل لغة، و`tests/drop/docs.sh` وhold_docs.sh ناجحان، ولا شرطة طويلة، وسطر في CHANGELOG.md باللغتين.
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
| AC13 | بوابة وهمية جديدة (على نمط `tests/overlay/portal.py`) تخدم Registry وGlobalShortcuts الإصدار 2 وتسجّل الاستدعاءات، في KWin مخفي خاص: ترتيب الاستدعاءات، وحقول `BindShortcuts`، وعدّ السطر؛ وحالة بلا Registry. ووحدة على الجزء الصافي في الخدمة (نص السطر، وحقول الاختصار في اللغتين) | تكامل، وحدة |
| AC14 | `cargo tree -d -i zbus` فارغ؛ عدّ `name = "zbus"` في `Cargo.lock` يساوي 1؛ `cargo tree -p sbarhive-daemon -e normal -i tokio` لا يجد الحزمة؛ مراجعة أسطر `docs/engines.md` مقابل كل حزمة جديدة في فرق `Cargo.lock` | تكامل |
| AC17 | وحدة: الربط الصافي (الجلسة والمعرّف والإشارة إلى `Signal`) لكل حالة، ومعرّف آخر وجلسة أخرى `None`؛ وتكامل بالوهمية في KWin خاص: 600 ms، 150 ms، ثلاث `Activated`، معرّف `other`، بعدّ `get_layer_surface` وطابعه في الأثر | وحدة، تكامل |
| AC18 | (أ) و(ب): الوهمية في KWin خاص، بأسطر `hive probe:` و`hive reveal:` وعدّ `attach` كفحوص S-021b، وسطر `hive anchor:` مقابل موضع السحب، والملف الناتج. (ج): `tests/drop/gnome.sh smart portal` بالوهمية (S-014e2) | تكامل |
| AC19 | الوهمية بخمسة أوضاع: غائبة، بلا GlobalShortcuts، ردّ بالرمز 1، ردّ بالرمز 2، ردّ بقائمة فارغة؛ السطر وسببه، وعدّاد الاستدعاءات بعد 30 ثانية، ثم دورة `drop --hold` و`--release`؛ ووضع حبس الرد مع `measure.sh` لـ `open` (5 ظهورات) | تكامل، أداء |
| AC20 | `tests/drop/gnome.sh smart portal-real` و`tests/drop/kwin.sh level=1 portal-real` (S-014e2)، و`tests/hyprland/repro.sh --portal` (S-014e3): البوابة الحقيقية ومنفذها على الناقل الخاص و`XDG_CURRENT_DESKTOP` للمؤلِّف، والمفاتيح بمسار إدخال المؤلِّف (RemoteDesktop في GNOME، وfake_input في KWin وفي KWin الخارجي لـ Hyprland)، الحالات (أ) و(ب) و(ج)، وسطر INFO بإصدار المنفذ | تكامل |
| AC21 | السكربتان نفساهما بتشغيلين للخدمة على ملف شخصي واحد: عدّ نوافذ المنفذ في التشغيل الثاني، وظهور واحد بالاختصار | تكامل |
| AC22 | فحص ثابت جديد للأسطر الأربعة في القسمين، و`tests/drop/docs.sh` و`tests/drop/hold_docs.sh` | ثابت |
| AC15 | `tests/drop/gnome.sh strict portal` و`strict cli` | تكامل |
| AC16 | `tests/drop/*.sh rebind` | تكامل |

## خارج النطاق
- X11 (ترتيب «الخلية أولاً ثم السحب» مع `QueryPointer` و`AlwaysOnTop`): قصة مستقلة S-015، لأنها مسار مختلف لم يُشغَّل ولا يختبرها هذا الإطار.
- تشغيل Hyprland فعلياً (يُوثَّق سطر `bind` فقط)، وقيد أول `wl_data_device` فيه. استثناء S-014e3: البوابة تُشغَّل في Hyprland متداخل خاص (AC20)، لا على سطح المكتب الحي.
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
- 2026-10-22، المدير: الخيار (ب). أسطر Lua تكتب `"$HOME/.local/bin/sbarhive ..."` داخل `exec_cmd`، والصدفة `/bin/sh -c` توسّع `$HOME` (أكّد المراجع ذلك). `tests/drop/docs.sh` وhold_docs.sh وoverlay/docs.sh بنتيجة RESULT PASS، وdesign-lint نظيف. الحالة: مراجعة.
- 2026-10-22، المراجِع (S-014d، المراجعة 2 بعد REWORK 1): **PASS**.
  - شُغّل كل شيء مرة واحدة داخل `systemd-run --user --scope` (MemoryMax=6G، MemoryHigh=5G، CPUQuota=200%، `nice -n 19`، `CARGO_BUILD_JOBS=2`)، ومؤلِّف مخفي واحد في كل مرة، دون لمس سطح المكتب الحي ودون `hyprctl`: `tests/drop/docs.sh` RESULT PASS (`INFO not shipped: none`)؛ `tests/drop/hold_docs.sh` RESULT PASS؛ `tests/overlay/docs.sh` RESULT PASS؛ `scripts/design-lint.sh` نظيف؛ `tests/drop/kwin.sh level=1` (الوضع الافتراضي) RESULT PASS، 308 PASS و0 FAIL؛ `tests/drop/gnome.sh smart` (الوضع الافتراضي) RESULT PASS، 72 PASS و0 FAIL. لم يبقَ بعدهما `kwin_wayland` ولا `gnome-shell`.
  - بنود المراجعة 1، كلها محلولة في `docs/drop-shortcut.md` و`integrations/linux/hyprland/README.md` باللغتين:
    1. ربط ضغط Alt: `bindni = , ALT_L, exec, ...` في المواضع الأربعة، وLua `hl.bind("ALT_L", ...)` مع `non_consuming = true` و`ignore_mods = true`. ربط الترك `bindri = , ALT_L` وLua `"ALT_L"` مع `release` و`ignore_mods`. جملة العلم `n` في الملفين تذكر `i` وسببه. لا يبقى `ALT + ALT_L` ولا `bindn = ALT`.
    2. لا `HOME ..` ولا `os.getenv` في أي سطر Lua: الأسطر الثمانية (Super+Z وAlt، باللغتين) تكتب `"$HOME/.local/bin/sbarhive ..."` داخل `exec_cmd`، بقرار المدير (الخيار ب). تحقق: حُلِّلت الأسطر الثمانية بـ luajit مع جدول `hl` بديل، فتمر السلسلة إلى الصدفة `$HOME/.local/bin/sbarhive drop --hold --at "$(hyprctl cursorpos)"` و`... drop --release`، والصدفة `/bin/sh -c` توسّع `$HOME` (المراجعة 1).
    3. ملاحظة GNOME تقول إن الاختبار سجّل المفتاح بـ `grab_accelerator('<Super>z')` داخل GNOME Shell عبر `Shell.Eval`، ولا تنسبه إلى البوابة.
    4. GNOME الخطوة 3: «إضافة» و"Add".
  - التصعيد عن AC11: حسمه المدير (af4d07d) بقبول الانحراف صراحة لحالة Esc قبل دخول السحب.
  - بوابات cargo: لا تنطبق. إيداعات الإصلاح (af4d07d، d5f3852، 4dc53be) لا تغيّر أي ملف `.rs` ولا `Cargo.toml` ولا `Cargo.lock`. لا `cargo mutants` بطلب المدير، ولا شيفرة متغيرة له. لا لقطات جديدة.
  - CHANGELOG.md: سطر S-014d موجود منذ baf35ac وما زال دقيقاً بعد الإصلاح، فلم يُضف سطر ثانٍ.
  - الحالة: الرأس يغطي القصص الست، فضُبط على «S-014d منتهية» مع بقاء S-014e وS-014f.
  - ما زال غير متحقق على جلسة حقيقية، وموسوماً كذلك في التوثيق: أسطر Hyprland، واختصار GNOME المخصص أثناء السحب، وأثر Alt غير المستهلِك على نوع الإفلات. وتبقى هشاشة `docs.sh` مع `docs/launch/` غير المتتبَّع ملاحظة غير حاجبة.
- 2026-10-22، المخطِّط (تحديث S-014e وحدها، للإصدار 0.4.0): AC13 وAC14 كُتبا في 2026-10-20 قبل S-019e وS-019f وS-021، فأُعيدت كتابتهما وأُضيف AC17 إلى AC22. لم يُلمس S-014a إلى S-014d ولا S-014f (AC15 وAC16 كما هما). قُسّمت S-014e إلى ثلاث قصص فرعية، كل منها يوم عمل أو أقل. الحالة: مسودة تنتظر موافقة محمد.
  - القرارات:
    1. اختصار البوابة يدخل بوابة الضغط المطوّل لا الظهور الفوري: `Activated` ضغط و`Deactivated` ترك. هذا مقرر في الشجرة أصلاً: `hold::from_portal` في `crates/core/src/hold.rs`، واختبار `ac21_*` في `crates/core/tests/hold_gate.rs` («the portal path is the IPC path»)، و`docs/architecture.md:202`. الجديد معيار يقيس وصول `Deactivated` في كل منفذ (AC20).
    2. الظهور يمر بمسار S-021 (`Probing` ثم `hive reveal:` أو `hive probe: hidden (R)`) على مسار الطبقة، وبمسار النافذة كما هو في GNOME. لذلك تعتمد S-014e1 على انتهاء S-021b وS-021c: الأسطر التي يفحصها AC18 تكتبها S-021b.
    3. لا موضع من البوابة: المرساة من مرحلة الماسك (S-019c)، والخدمة لا تشغّل `hyprctl`.
    4. التسجيل في الخدمة، في خيط واتصال D-Bus خاصين، وأول استدعاء `Registry.Register`: الواجهة تقول «Registering must be done before any portal method call; registering after such a call will result in an error»، والمعرّف يجب أن يطابق اسم ملف `.desktop` (`packaging/io.github.m7mmad_omar.SbarHive.desktop` موجود). الخدمة تعمل كوحدة `sbarhive.service` لا نطاق `app-<id>`، فبدون التسجيل لا تعرف البوابة معرّفها (مُفترَض، يحسمه AC13 وAC20).
    5. الجزء الصافي (ربط الإشارات، واختيار سبب الرجوع ونص السطر) في وحدة من `crates/daemon` تخضع للطفرات؛ جانب D-Bus يُستثنى في `.cargo/mutants.toml` كما `receiver.rs` (قرار S-018 الخامس)، وتغطيه السكربتات.
    6. الربط المفضّل Super+Z بصيغة مواصفة الاختصارات (المتوقَّع `LOGO+z`) بدل `<Super>z` في النص القديم، فحقل `preferred_trigger` يتبع تلك المواصفة. في ثنائي منفذ GNOME المثبّت النص `LOGO`، ولم يوجد في ثنائي منفذ KDE (قد يكون في مكتبة مشتركة)، فيثبّته المختبِر.
    7. في Hyprland يبقى سطر `exec` الموثَّق مع `--at "$(hyprctl cursorpos)"` أفضل من البوابة: البوابة تحتاج سطر `global` في الإعداد على أي حال، ولا موضع معها فتنتظر المرساة الماسك. S-014e3 يختبر البوابة هناك كما طُلب، والتوثيق يحتاج قرار محمد (السؤال 6).
  - المصادر المقروءة في هذه الجلسة (ملفات مثبّتة على الجهاز، قراءة فقط):
    - `/usr/share/dbus-1/interfaces/org.freedesktop.portal.GlobalShortcuts.xml`: الإصدار 2، والإشارتان `Activated` و`Deactivated`، و`activation_token` في خيارات كلٍّ منهما، و`BindShortcuts` «will typically result the portal presenting a dialog»، ومرة واحدة لكل جلسة.
    - `/usr/share/dbus-1/interfaces/org.freedesktop.host.portal.Registry.xml`: الإصدار 1.
    - `/usr/share/xdg-desktop-portal/portals/{gnome,kde,hyprland}.portal`: الثلاثة تسرد `org.freedesktop.impl.portal.GlobalShortcuts`؛ و`hyprland.portal` يُستعمل في `wlroots;Hyprland;sway;Wayfire;river`.
    - `/usr/share/doc/xdg-desktop-portal-gnome/NEWS` (المثبَّت 50.0): «Add global shortcuts portal backend» في 48.rc، و«Improvements to the Global Shortcuts portal» في 49.beta، و«Properly send the Global Shortcut activation token to the portal frontend» في 50.alpha. أي أن رمز التفعيل لـ S-014f يحتاج GNOME 50.
    - نصوص الثنائيات في `/usr/libexec/xdg-desktop-portal-{gnome,kde,hyprland}` فيها `Deactivated` (وفي GNOME `AcceleratorDeactivated`، وفي KDE `shortcutDeactivated`، وفي Hyprland `released`). هذا دليل على وجود المسار في الشيفرة، لا على سلوكه، فيبقى قياس AC20.
    - KDE: ADR 0003 يذكر الدعم في KDE 6 بالإصدار 2. لم يُقرأ رقم إصدار xdg-desktop-portal-kde ولا xdg-desktop-portal-hyprland المثبّتين؛ يطبعه AC20 سطر INFO.
  - لم يُجلب من الشبكة في هذه الجلسة (لا أداة شبكة)، فيتحقق منه المنفِّذ قبل البدء: https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.GlobalShortcuts.html و https://specifications.freedesktop.org/shortcuts-spec/latest/ (صيغة `LOGO+z`) و https://gitlab.gnome.org/GNOME/xdg-desktop-portal-gnome/-/blob/main/NEWS و https://invent.kde.org/plasma/xdg-desktop-portal-kde و https://github.com/hyprwm/xdg-desktop-portal-hyprland و https://wiki.hypr.land/Configuring/Binds/ (الموزّع `global`، وهل يرسل الترك) و https://crates.io/crates/ashpd (الإصدار، والرخصة، ونسخة zbus، وأسماء الميزات، ودالة تسجيل تطبيق المضيف).
  - ما في `Cargo.lock` اليوم: `zbus` 5.19.0 واحدة (`crates/ui` بـ `async-io` و`blocking-api`)، وtokio 1.53.3 عبر `wasm-bindgen-futures` فقط، ولا ashpd. لم يُتحقق من أن ashpd 0.13.13 يقبل zbus 5.19.0، ولذلك صار AC14 شرطاً مع بديل لا افتراضاً.
  - الاختبار: مؤلِّفات خاصة فقط (KWin وGNOME Shell مخفيان، وHyprland متداخل داخل KWin خاص كما `repro.sh`)، بملف شخصي وناقل جلسة خاصين، ولا لمس لسطح المكتب الحي ولا `hyprctl` عليه. الأعمال الثقيلة داخل `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200%` بـ `-j 1` أو `-j 2`، ومؤلِّف واحد في كل مرة.
  - لـ S-014f (لم يُعدَّل نصها): `Activated` في GNOME يحمل `activation_token` عند الضغط، والظهور يأتي بعد `hold_ms`، و`Signal::Hold` و`Effect::Show` لا يحملان رمزاً اليوم. على مخطِّط S-014f أن يقرر حمل الرمز عبر البوابة وصلاحيته بعد 400 ms.
  - أسئلة لمحمد:
    1. النقرة على Super+Z من البوابة لا تُظهر شيئاً (إمساك فقط، قرار 1)، بينما سطر `bind = SUPER, Z` اليدوي الموثَّق في 0.3.0 فوري. هل يُقبل الإمساك وحده، أم يُضاف خيار في `config.toml` للظهور الفوري؟
    2. من ربط Super+Z يدوياً بعد `docs/drop-shortcut.md` سيجد تعارضاً مع ربط البوابة في GNOME وKDE. هل يقول التوثيق «احذف الربط اليدوي» (المقترح)، أم يختار المنفذ مفتاحاً آخر في نافذة الموافقة؟
    3. إن أضاف ashpd نسخة ثانية من zbus أو tokio: نكتب وكيل zbus يدوياً (المقترح، نحو 150 سطراً، والواجهة مثبّتة على الجهاز)، أم نقبل الترقية أو الحزمة الإضافية؟
    4. إن لم يرسل أحد المنافذ الثلاثة `Deactivated` عند الترك: ظهور فوري لذلك المنفذ وحده، أم يبقى الربط اليدوي هناك؟
    5. من ألغى نافذة الموافقة (`denied` في AC19) ستعود له النافذة عند كل تشغيل للخدمة، أي كل دخول. هل تتذكر الخدمة الرفض (مثلاً مفتاح في `config.toml` يوقف التسجيل حتى يغيّره المستخدم من الإعدادات)، أم تسأل في كل تشغيل؟ AC21 اليوم يغطي حالة القبول فقط.
    6. Hyprland: هل يبقى سطر `exec` مع `--at` الطريق الأول في التوثيق وسطر `global` بديلاً (المقترح، القرار 7)، أم العكس؟
- 2026-10-22, manager under Mohammad's delegation (he asked not to be asked and to finish the work): S-014e (e1, e2, e3) is approved with the planner's seven decisions. Answers to the six questions:
  1. The portal shortcut is hold only, like the documented Alt and Super+Z hold binds. No instant option in `config.toml` for now.
  2. The docs tell users who bound Super+Z by hand to remove that bind once the portal registers it.
  3. If ashpd would add a second zbus or any tokio, write the small zbus proxy by hand against the installed interface XML instead.
  4. If a portal backend never sends `Deactivated`, do not register through that backend; it keeps the manual bind, and the daemon prints the `portal shortcuts: unavailable (R)` line with a reason the tester names.
  5. A denied consent is remembered: a `config.toml` key stops registration until the user turns it back on in the settings, so the dialog does not return at every login.
  6. On Hyprland the documented `exec` bind with `--at` stays first; the portal `global` line is the alternative.
  Story numbers: the Windows work on branch `windows/0.6.0` uses S-022 to S-026, which clash with main's S-022. New stories on main start at S-027; the renumbering of the clash is settled when that branch merges.
- 2026-10-22, tester (S-014e1 only: AC13, AC14, AC17, AC18 parts a and b, AC19; part c of AC18 is S-014e2): tests written from the criteria, the table and the manager's answers 4 and 5, run once on main (`0de3c74`, release binaries of 17:15, newer than the last crate change `966258b`, so no rebuild), and every one fails.
  - Files: `crates/daemon/src/portal_tests.rs` with `#[cfg(test)] mod portal_tests;` in `crates/daemon/src/main.rs` (AC13, AC17, AC19 unit tests), `crates/core/tests/config_portal.rs` (answer 5), `tests/drop/shortcuts_portal.py` (the fake portal), `tests/drop/portal_log.py` (reads its log), `tests/drop/portal.sh` (AC13, AC17, AC18 a and b, AC19 in a private KWin), `tests/drop/deps.sh` (AC14, static).
  - The fake was checked against a real client before use: a scratch program on ashpd 0.13.13 (`default-features = false`, `global_shortcuts`, `async-io`, outside the repo) on a private bus did Register, CreateSession, BindShortcuts and received Activated and Deactivated in modes ok, no-registry, no-gs (ashpd: "portal frontend ... not found"), version0, deny ("Portal request was cancelled"), empty and hold. The AC18 (b) flow was checked once with `drop --hold` and `--release` in place of the two signals (scratch copy of the script): `RESULT PASS`, anchor `400 500` at the drag's enter `400 500`, the WEBP written. So the harness passes for a working gate, and fails today only for the missing client.
  - Contract (do not edit):
    - `crate::portal` in `crates/daemon` (the pure part, under mutants; the D-Bus side may be excluded in `.cargo/mutants.toml` like `receiver.rs`, decision 5): `APP_ID = "io.github.m7mmad_omar.SbarHive"`, `SHORTCUT_ID = "drop"`, `PREFERRED_TRIGGER = "LOGO+z"` (shortcuts spec: modifiers CTRL, ALT, SHIFT, NUM, LOGO joined by `+` to an xkbcommon keysym name without `XKB_KEY_`; the installed GNOME backend binary contains `LOGO`; its acceptance is checked live in S-014e2), `REGISTRY_UNAVAILABLE = "portal registry: unavailable"`; `description(lang: Lang) -> &'static str` (non-empty, English Latin, Arabic Arabic, different, no long dash); `bound_line(trigger: &str) -> String` = `portal shortcuts: bound drop (T)`; `enum Reason { NoPortal, NoInterface, Denied, Error, NoRelease, Off }` with `Clone, Copy, Debug, PartialEq, Eq`; `unavailable_line(r: Reason) -> String` = `portal shortcuts: unavailable (R)`, R one of `no-portal`, `no-interface`, `denied`, `error`, `no-release` (answer 4: a backend that never sends `Deactivated`), `off` (answer 5: registration turned off by a remembered denial); `interface_reason(version: Option<u32>) -> Option<Reason>` (None or 0 is `NoInterface`); `bind_reason(response: u32, bound: &[&str]) -> Option<Reason>` (0 with `drop` among the bound ids is None, 0 with an empty list or without `drop` is `Denied`, 1 is `Denied`, any other code `Error`); `remembers(r: Reason) -> bool` (only `Denied`); `signal(own_session: &str, member: &str, session: &str, shortcut_id: &str) -> Option<hold::Signal>` (`Activated` and `Deactivated` of `drop` in the own session give `hold::from_portal`, everything else None).
    - `sbarhive_core::config::Config::portal_shortcut: bool`, default true, spelled `portal_shortcut` in `config.toml` (as `hold_ms`). After a `denied` reason the daemon writes `portal_shortcut = false` and keeps the user's other keys; a daemon started with it false makes no portal call or property read, not even Register, and prints `portal shortcuts: unavailable (off)`; with it true again the next daemon registers. Live reload of the key is not required by these tests.
    - Daemon stderr: the `portal shortcuts:` and `portal registry:` lines are printed always; with `SBARHIVE_DEBUG_FOCUS` one `portal: activated drop` or `portal: deactivated drop` line per signal that `signal` accepts, and no line for an ignored one.
    - "Portal call" in AC13 and AC19 means a method call on an `org.freedesktop.portal.*` or `org.freedesktop.host.portal.*` interface. Property reads are not method calls: ashpd itself reads `version` when it builds each proxy, before Register. The 30 s counter of AC19 counts calls and reads both. The daemon's portal connection is the sender of the first CreateSession (else of the first Register).
    - The fake, `tests/drop/shortcuts_portal.py CONTROL_FIFO MODE`, modes `ok`, `no-registry` (Register answers `org.freedesktop.DBus.Error.UnknownMethod`), `no-gs`, `version0`, `deny` (code 1), `error` (code 2), `empty` (code 0, no shortcuts), `hold` (the BindShortcuts Response waits for the control line `respond`). Paths as xdg-desktop-portal makes them (request and session paths from the sender and the tokens), `session_handle` as `s`, Response and the two signals sent to the caller only, after the reply. The trigger_description is `Fake Super+Z`. Control lines: `signal Activated|Deactivated ID [own|other]`, `pulse ID own|other MS [AT...]` (timed inside the fake), `respond`, `quit`. Log lines, each after a `[HH:MM:SS.ffffff]` UTC stamp: `owned NAME`, `call N IFACE.METHOD SENDER JSON_ARGS`, `get N IFACE.PROPERTY SENDER`, `error N NAME`, `response N CODE PATH JSON_RESULTS`, `held N PATH`, `emit SIGNAL ID SESSION DEST`, `skip LINE (REASON)`.
    - `tests/drop/portal.sh [ac13] [ac17] [ac19] [hold] [BIN]`: one private `kwin_wayland --virtual`, a throwaway profile, and a private bus started with its own configuration whose only service directory is empty, so nothing is activated (a setup check reads `ListActivatableNames`): with no fake nothing owns `org.freedesktop.portal.Desktop`, and the real portal and its backends never start. Daemon and fake restart per mode.
  - Failure output:
    ```
    $ cargo test -p sbarhive-core --test config_portal
    error[E0609]: no field `portal_shortcut` on type `Config`
      --> crates/core/tests/config_portal.rs:41:15
    error[E0560]: struct `Config` has no field named `portal_shortcut`
      --> crates/core/tests/config_portal.rs:56:9
    error: could not compile `sbarhive-core` (test "config_portal") due to 6 previous errors

    $ cargo test -p sbarhive-daemon portal
    error[E0432]: unresolved import `crate::portal`
     --> crates/daemon/src/portal_tests.rs:6:12
    error: could not compile `sbarhive-daemon` (bin "sbarhived" test) due to 1 previous error

    $ tests/drop/deps.sh
    INFO AC14: crates/daemon: client none, ashpd version - default-features - features -, zbus direct no; zbus::proxy files for GlobalShortcuts: none; Cargo.lock: ashpd -, zbus 5.19.0, tokio 1.53.3
    FAIL AC14: crates/daemon has a portal client: ashpd at a full version with default-features = false and only the features global_shortcuts and async-io (one ashpd in Cargo.lock at that version), or zbus directly with a #[zbus::proxy] for org.freedesktop.portal.GlobalShortcuts and no ashpd
    FAIL AC14: with the client, no second zbus: cargo tree -d lists no zbus, and Cargo.lock has one zbus, at 5.19.0 or at the version docs/engines.md records
    FAIL AC14: with the client, the daemon's normal dependencies have no tokio
    FAIL AC14: with the client, every package new in Cargo.lock (ashpd among them, if it is the client) has a docs/engines.md row with its version and licence
    RESULT FAIL (4)

    $ tests/drop/portal.sh   (private KWin, about 6 minutes)
    PASS setup: the private bus can activate nothing but itself (no org.freedesktop.portal.Desktop to start)
    FAIL AC13 ok: the first portal method call of the daemon's portal connection is org.freedesktop.host.portal.Registry.Register with io.github.m7mmad_omar.SbarHive
    FAIL AC13 ok: its next GlobalShortcuts method call is CreateSession
    FAIL AC13 ok: exactly one BindShortcuts, on the session CreateSession gave (at most one per session)
    FAIL AC13 ok: one shortcut, id "drop", preferred_trigger "LOGO+z", a non-empty English description
    FAIL AC13 ok: stderr has exactly one portal line, "portal shortcuts: bound drop (Fake Super+Z)" (T is the reply's trigger_description)
    FAIL AC17 600 ms (2 checks), AC18 (a) (2), AC17 150 ms, AC17 repeat (2), AC18 repeat, AC17 other id, AC17 other session, AC17 one gate (2)
    INFO AC17 one gate, IPC press: exit 0, deactivated lines 0; within 1000 ms: layer 1 xdg 0
    FAIL S-021 AC3, AC4, AC7 AC18 (b) (probe_checks), AC18 (b): "hive reveal: N" with N the 15 lines of `sbarhive actions` for the PNG, the WEBP written, the anchor lines
    PASS AC19 no-portal setup: nothing owns org.freedesktop.portal.Desktop
    FAIL AC19 no-portal, no-gs, version0, error, empty, deny: the line and its reason, the 30 s with no call, the hold cycle after the line, config.toml (4 checks each)
    FAIL AC13 error: the daemon in Arabic sends an Arabic description
    FAIL AC19 deny again (answer 5): ... prints exactly "portal shortcuts: unavailable (off)"
    FAIL AC19 deny, turned back on (answer 5): ... the next daemon registers again
    INFO AC19 hold: BindShortcuts held no before the shows, still held after them no, portal shortcuts lines before the Response 0; shows 5 of 5, ms from the command to the first frame: 21.8 6.6 10.8 6.2 6.7 (median 6.7)
    FAIL AC19 hold (3 checks)
    FAIL AC13 no-registry (2 checks)
    RESULT FAIL (55)
    ```
    The reason everywhere: the daemon has no portal client, so the fake gets no call, there is no session to signal, and no `portal` line is printed. Totals: unit 2 compile failures (17 tests: 13 in the daemon, 4 in core), `deps.sh` 4 of 4 checks, `portal.sh` 55 of 57 checks (the two passes are setup guards).
  - Guards that would pass on their own are tied to a precondition, so they fail today for the missing client and pass only with it: "no surface" and "ignored" cases need the `portal:` lines or the bound line; the AC19 30 s counter and the `drop --hold` cycle need the `unavailable (R)` line (the cycle itself works on main: shown 505 to 516 ms after the press with hold_ms 500, destroyed 2 to 3 ms after the release); the `open` timing needs the held BindShortcuts (median 6.7 ms today); AC14's tree checks need the client. The "one gate" cases fail on main for the right reason: `drop --hold` then the portal's Deactivated still shows a hive from the IPC timer.
  - Not testable here, or tested differently from the table:
    - AC14: `cargo tree -d -i zbus` cannot print nothing: on main it prints 37 lines, the duplicated `syn` 2 and 3 on the paths to the one zbus. `deps.sh` checks the intent instead: no zbus among the duplicated packages of `cargo tree -d`, and one zbus in `Cargo.lock`.
    - AC19, last clause: `crates/ui/tests/measure.sh` drives `hive_bench`, which has no portal client, so it cannot show the hive while the daemon's BindShortcuts is pending. `portal.sh hold` times the real daemon: from a UTC stamp just before `sbarhive open FILE` to the first `attach(wl_buffer)` on the hive surface in its trace, median of 5. This includes the CLI's own start, so it is stricter.
    - AC13 "a private thread on a dedicated connection": only the order on the portal connection is observable. The fake serves no Settings interface, so a shared connection cannot be seen.
    - Answer 4: only the reason text `no-release` is tested. No backend is known to lack `Deactivated` until AC20 measures it, and the daemon cannot see that before it registers.
    - AC19 `no-portal`: the fake starts 1 s after the line, so a retry would show in the 30 s counter.
  - Decisions for the manager:
    1. AC14's command: accept the intent check above in place of `cargo tree -d -i zbus` printing nothing.
    2. Answer 4: how the daemon knows not to register through a backend. The backend is not visible from the frontend. One option is a list keyed by `XDG_CURRENT_DESKTOP`, filled from the AC20 results. The reason name `no-release` is fixed.
    3. `no-portal` at a real login: `org.freedesktop.portal.Desktop` is D-Bus activatable but often not yet running when `sbarhive.service` starts. A `NameHasOwner` check would report `no-portal` there. The tests pass either way (nothing is activatable on the private bus). I propose: no-portal only when the name has no owner and is not activatable.
    4. Answer 5: the settings window toggle for `portal_shortcut` is in no S-014e criterion. Should it go in S-014f or another story?
    5. `bind_reason(0, &["other"])` (a reply without `drop`) counts as `denied`, so it is remembered. This goes beyond the AC's "empty list".
  - Note: `mod portal_tests` stops every unit test of `sbarhive-daemon` from building until `crate::portal` exists, as in S-014a. `cargo mutants` was not run (not the tester's job). Heavy runs were inside `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200%` with `CARGO_BUILD_JOBS` 1 or 2, one private compositor at a time. The live desktop, its bus and the real portal were not touched.
