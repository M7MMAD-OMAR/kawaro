# S-012 ربط الخدمة الخلفية وسطر الأوامر
الحالة: مراجعة

## القصة
بصفتي مستخدماً على لينكس، أريد أن أفتح الخلية من مدير الملفات أو من سطر الأوامر، وأن تعمل التحويلات في الخلفية دون أن تحجب أي نافذة، حتى أجد الناتج بجانب ملفاتي وأعرف متى انتهى.

## معايير القبول
- AC1: `sbarhived` يستمع على `$XDG_RUNTIME_DIR/sbarhive.sock`، ونسخة ثانية تخرج برسالة «already running» ورمز 0 دون أن تمس المقبس.
- AC2: `sbarhive open` يشغّل الخدمة إن لم تكن تعمل خلال ثانيتين، ويمرّر رمز التفعيل من `--token` أو `XDG_ACTIVATION_TOKEN` أو `DESKTOP_STARTUP_ID`.
- AC3: `sbarhive open` لملفات بلا إجراءات مطابقة يخرج برمز غير الصفر ويرسل إشعاراً واحداً.
- AC4: `sbarhive run ID --wait` يطبع مسار كل ناتج، ولا يبقى أي ملف مؤقت مخفي بعد الانتهاء، والتشغيل الثاني لا يكتب فوق الأول.
- AC5: `sbarhive convert --to EXT` يعمل دون الخدمة ويفشل برسالة تذكر الصيغ المتاحة إن لم تكن EXT منها.
- AC6: `sbarhive open --from-clipboard` يقرأ `text/uri-list` ويفك الترميز المئوي ويتجاهل غير `file:`.
- AC7: إعداد `output = "subfolder"` يضع الناتج في مجلد فرعي يُنشأ عند الحاجة، و`quality` يعدّل معامل الجودة، و`notify = false` يمنع إشعار الانتهاء.
- AC8: `sbarhive quit` يوقف الخدمة ويحذف المقبس.
- AC9: النصوص الظاهرة للمستخدم (المركز، الإشعارات، قائمة الشريط) بالعربية والإنكليزية، وعدّ العربية صحيح (صورة واحدة، صورتان، 3 صور، 11 صورة).

## حالات الاختبار
| معيار | الاختبار | النوع |
| --- | --- | --- |
| AC1 | `tests/e2e/run.sh`: «daemon answers ping»، «single instance» | تكامل |
| AC2 | `tests/e2e/run.sh`: «open shows the hive» بعد تشغيل الخدمة؛ قراءة الشيفرة لمتغيرات الرمز | تكامل |
| AC3 | `tests/e2e/run.sh`: «open with no matching actions fails» | تكامل |
| AC4 | `tests/e2e/run.sh`: «run --wait prints output path»، «second run does not overwrite»، «no temp files left» | تكامل |
| AC5 | `tests/e2e/run.sh`: «convert works without daemon» | تكامل |
| AC6 | `crates/cli/src/clipboard.rs` اختبارات الوحدة | وحدة |
| AC7 | `crates/core/src/config.rs` و`crates/core/tests/queue.rs::output_dir_setting_writes_into_a_created_subfolder` | وحدة |
| AC8 | `tests/e2e/run.sh`: «quit stops the daemon and removes the socket» | تكامل |
| AC9 | `crates/daemon/src/text.rs` اختبارات الوحدة | وحدة |

## خارج النطاق
- نافذة الإعدادات (S-011).
- اختبار التركيز على GNOME وKDE الحقيقيين (يحتاج آلة افتراضية).

## سجل المراجعة
- 2026-10-19، المدير: كُتبت الشيفرة قبل القصة لأن العقد في `docs/architecture.md` كان مرجعها، ثم كُتبت هذه القصة لتربط كل سلوك باختبار قبل المراجعة المستقلة. هذا استثناء من ترتيب السلسلة، مسجّل هنا.
- 2026-10-20، المدير: `tests/e2e/run.sh` نجح بكل فحوصه الثلاثة عشر داخل KWin مخفي بملف شخصي معزول.
- 2026-10-20، المراجع المستقل (إصدار v0.1.0): **REWORK**. ما نجح: `tests/e2e/run.sh` على KWin مخفي 13 من 13، ووحدة cli 2 وdaemon 2. الأسباب: (1) طفرات ناجية، 36 في ثلاثة ملفات: `crates/daemon/src/text.rs` 25 (أذرع Video وAudio وDocument وPdf في `selection` في الأسطر 22 و25 و28 و29 و36 و37 و38، وكل من `no_actions:47` و`done:54` و`failed:61` و`cancelled:68` و`open_folder:75` و`cancel_all:82` و`quit:89` و`idle:96` و`app_name:105` يمكن إبداله بـ "" أو "xyzzy")؛ `crates/core/src/config.rs` 7 (`Config::path:80` مرتين، حارس `NotFound` في `load_from:89`، `Config::load:99` ثلاث مرات، `+` إلى `*` في `apply_quality:149`)؛ `crates/cli/src/clipboard.rs` 4 (`read_files:7` مرتين، حذف `!` في السطر 18، `&&` إلى `||` في `parse_uri_list:33`) ومهلتان في `percent_decode:50` و`:53`. (2) AC2 بلا اختبار: e2e يشغّل `sbarhived` يدوياً قبل `open`، فمسار التشغيل التلقائي خلال ثانيتين لا يُختبر، وتمرير الرمز من `--token` و`XDG_ACTIVATION_TOKEN` و`DESKTOP_STARTUP_ID` «بقراءة الشيفرة» لا باختبار. (3) AC3: لا شيء يتحقق من الإشعار الواحد، الفحص على رمز الخروج فقط. (4) AC7: `notify = false` بلا أي اختبار. (5) AC1: «لا يمس المقبس» غير مفحوص، و«open shows the hive» يفحص رمز الخروج لا ظهور النافذة. (6) ملاحظة: `ksni` (Unlicense) و`notify-rust` و`clap` غير مسجلة في `docs/engines.md` أو غيره، ورخصها متوافقة. (7) ملاحظة: `tests/e2e/run.sh` لا يحذف `HYPRLAND_INSTANCE_SIGNATURE` و`XDG_CURRENT_DESKTOP` كما يفعل `snapshots.sh`، فيُفعَّل `xdg-desktop-portal-hyprland` على الناقل الخاص.
