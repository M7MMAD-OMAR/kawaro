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
