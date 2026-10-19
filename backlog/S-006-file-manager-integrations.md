# S-006 إضافة «صبار هايف» إلى قوائم مديري الملفات واختصار Hyprland
الحالة: مراجعة

## القصة
بصفتي مستخدماً على لينكس، أريد عنصراً واحداً باسم SbarHive في قائمة النقر اليمين في Dolphin وNautilus وNemo وThunar، واختصاراً في Hyprland،
حتى أفتح الخلية للملفات المحددة دون أن أكتب أي أمر.

## معايير القبول
- AC1 (Dolphin): الملف `integrations/linux/dolphin/sbarhive.desktop` فيه بالضبط: `Type=Service`، `X-KDE-ServiceTypes=KonqPopupMenu/Plugin`، `MimeType=all/allfiles;`، `X-KDE-Priority=TopLevel`، `Actions=sbarhive;`، وقسم إجراء واحد فيه `Name=SbarHive` و`Name[ar]=صبار هايف` و`Icon=sbarhive` و`Exec=sbarhive open -- %F`. وضع الملف في المستودع 0755.
- AC2 (Nautilus): الوحدة `integrations/linux/nautilus/sbarhive.py` تُستورد بـ gi الحقيقي (Nautilus 4.1، أو 4.0 عند غيابه) وبـ gi بديل، و`get_file_items` تعيد عنصراً واحداً لملفات `file://`، وقائمة فارغة إذا لم يكن بين الملفات ملف محلي.
- AC3 (Nautilus): تسمية العنصر `صبار هايف` عندما تبدأ اللغة (LANGUAGE ثم LC_ALL ثم LC_MESSAGES ثم LANG) بـ `ar`، و`SbarHive` في غير ذلك.
- AC4 (Nautilus): تفعيل العنصر يشغّل بالضبط `sbarhive open --token TOK -- a b` عند توفر رمز، و`sbarhive open -- a b` عند غيابه، عبر `subprocess.Popen` بجلسة جديدة ومداخل ومخارج `/dev/null`، ويعود خلال أقل من 100 ملي ثانية حتى لو بقي الأمر يعمل 5 ثوانٍ.
- AC5 (Nemo): `sbarhive.nemo_action` فيه `Selection=notnone` و`Extensions=any;` و`Exec=sbarhive open -- %F` و`Name[ar]=صبار هايف`.
- AC6 (Thunar): `merge_uca.py add` يضيف إجراءً واحداً بـ `unique-id` يساوي `sbarhive`، وتشغيله مرتين يترك إجراءً واحداً وملفاً مطابقاً بايتاً ببايت. `remove` مرتين يحذفه ولا يغيّر الإجراءات الأخرى ولا التعليقات. ملف `uca.xml` تالف لا يُعاد كتابته ويعيد الأمر رمز خروج غير صفري.
- AC7 (Hyprland): السكربت يستدعي `hyprctl dispatch sendshortcut CTRL,C,activewindow` ثم `sbarhive open --from-clipboard` بهذا الترتيب. الملف README بالعربية والإنكليزية يذكر سطر الربط بالفأرة وبديلاً من لوحة المفاتيح.
- AC8: لا يحتوي أي ملف في `integrations/linux/` على شرطة طويلة (U+2014) ولا متوسطة (U+2013)، و`shellcheck` نظيف على السكربتات.

## حالات الاختبار
| معيار | الاختبار | النوع |
| --- | --- | --- |
| AC1 | `tests/integrations/test_static.sh` يقرأ المفاتيح بمحلل ini بسيط ويتحقق من القيم والوضع 0755 | وحدة |
| AC2 | `tests/integrations/test_nautilus.py`: استيراد بـ gi الحقيقي إن وُجد، ثم بحزمة gi بديلة ترفض 4.1 وتقبل 4.0، مع ملفات وهمية `file` و`sftp` | وحدة |
| AC3 | نفس الملف: LANG=ar_SY.UTF-8 ثم LANG=en_US.UTF-8، مقارنة التسمية | وحدة |
| AC4 | نفس الملف: استبدال `Popen` بمسجّل، ورمز `TOK` ثم `None`، ومقارنة argv والمعاملات، وأمر حقيقي `sleep 5` مع قياس الزمن | وحدة |
| AC5 | `test_static.sh` يقرأ المفاتيح | وحدة |
| AC6 | `tests/integrations/test_merge_uca.sh` على ملف في مجلد مؤقت فيه إجراء آخر وتعليق، add مرتين وremove مرتين وملف تالف | تكامل |
| AC7 | `tests/integrations/test_hyprland.sh` مع `hyprctl` و`sbarhive` وهميين يسجلان الاستدعاءات في ملف | تكامل |
| AC8 | `test_static.sh`: grep للشرطات و`shellcheck` | وحدة |

## قرارات
- الملفات المصدرية تكتب `sbarhive` كما في المواصفة. المثبّت يستبدله بالمسار المطلق `${PREFIX}/bin/sbarhive` لأن مدير الملفات الذي تطلقه الجلسة قد لا يملك `~/.local/bin` في PATH.
- Dolphin: لا نمرر `--token`. KDE يضع `XDG_ACTIVATION_TOKEN` في بيئة الأمر الذي يطلقه، والـ CLI يقرؤه عند غياب `--token` (docs/architecture.md، قسم CLI).
- Dolphin: `all/allfiles` يغطي الملفات دون المجلدات، و`X-KDE-Protocols=file` يحصر العنصر في الملفات المحلية (المفتاح موجود في libKF6KIOWidgets). الثمن: لا يظهر العنصر للعناصر بمخطط غير `file`، مثل عرض `desktop:/` في Plasma. Dolphin على Hyprland لا يتأثر.
- Thunar: إذا لم يوجد `~/.config/Thunar/uca.xml` نبدأ من `/etc/xdg/Thunar/uca.xml`، لأن ملف المستخدم يحل محل ملف النظام كاملاً، ولو بدأنا من ملف فارغ لاختفت إجراءات Thunar الافتراضية.
- Hyprland: `SUPER, mouse:273` هو ربط تغيير حجم النافذة في الإعداد الافتراضي (`bindm`)، فالبديل من لوحة المفاتيح هو المقترح الأول في README.

## خارج النطاق
- كتابة الـ CLI نفسه (مسار آخر). نستدعيه فقط كما في المواصفة.
- اختبار التركيز الحي على GNOME وKDE (S-001 AC6).
- ويندوز.

## سجل المراجعة
- 2026-10-19: القصة معتمدة مسبقاً من محمد (غائب، فوّض القرارات).
- 2026-10-19 (الاختبارات أولاً، قبل أي تنفيذ): `bash tests/integrations/run_all.sh` فشل كما يجب: test_static 14 من 15 فحصاً فشلت، test_merge_uca 17 من 30، test_hyprland 4 من 4، وtest_nautilus توقف عند غياب الوحدة.
- 2026-10-19 (بعد التنفيذ): `bash tests/integrations/run_all.sh` أخضر: test_static 43 فحصاً، test_merge_uca 30، test_hyprland 4، test_nautilus بـ gi الحقيقي (Nautilus 4.1 على Fedora 44) وبـ gi البديل (مسار 4.0). shellcheck وdesktop-file-validate وappstreamcli نظيفة. الحالة: مراجعة.
- ملاحظة للمراجع: لم يُختبر ظهور العنصر حياً داخل Dolphin وNautilus وNemo وThunar، ولا التركيز برمز التفعيل؛ ذلك ضمن S-001 AC6.
