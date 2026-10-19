# S-007 تثبيت محلي للمستخدم وإزالته بسكربت واحد
الحالة: مراجعة

## القصة
بصفتي مستخدماً على لينكس، أريد تثبيت SbarHive في مجلدي الشخصي بأمر واحد دون sudo، وإزالته بأمر واحد،
حتى أجربه دون أن ألمس النظام، وأتخلص منه دون بقايا.

## معايير القبول
- AC1: `scripts/install.sh --no-build` مع ثنائيات جاهزة يضع `sbarhive` و`sbarhived` في `${PREFIX:-$HOME/.local}/bin` بوضع 0755، والملفات التالية بوضع 0644: ملف سطح المكتب `io.github.m7mmad_omar.SbarHive.desktop` في `$XDG_DATA_HOME/applications`، وملف metainfo في `$XDG_DATA_HOME/metainfo`.
- AC2: الأيقونتان تنسخان إلى `hicolor/scalable/apps/sbarhive.svg` و`hicolor/symbolic/apps/sbarhive-symbolic.svg` إن وُجدتا في `design/brand/`، وغيابهما لا يوقف التثبيت (رمز خروج 0 ورسالة تخطٍّ).
- AC3: يُثبَّت تكامل مدير الملفات الموجود فقط. مع الأربعة: Dolphin في `$XDG_DATA_HOME/kio/servicemenus/sbarhive.desktop` (0755)، Nautilus في `$XDG_DATA_HOME/nautilus-python/extensions/sbarhive.py`، Nemo في `$XDG_DATA_HOME/nemo/actions/sbarhive.nemo_action`، Thunar إجراء واحد في `$XDG_CONFIG_HOME/Thunar/uca.xml`. دون أي منها: لا يُنشأ أي من هذه الملفات.
- AC4: مع `hyprctl` يُثبَّت `${PREFIX}/bin/sbarhive-open-selection` (0755) ويُطبع سطر الربط، ولا يُعدَّل `hyprland.conf`.
- AC5: مع جلسة systemd للمستخدم: `sbarhive.service` في `$XDG_CONFIG_HOME/systemd/user/` ثم `systemctl --user daemon-reload` و`enable` (و`--now` عندما يكون `graphical-session.target` نشطاً). دونها: `$XDG_CONFIG_HOME/autostart/sbarhive.desktop` بدل الوحدة.
- AC6: كل `Exec` و`ExecStart` وأمر Thunar في الملفات المثبتة يشير إلى المسار المطلق للثنائي داخل `${PREFIX}/bin`.
- AC7: تشغيل التثبيت مرتين يعطي نفس شجرة الملفات (المسارات والأوضاع والمحتوى) مثل تشغيله مرة.
- AC8: `scripts/uninstall.sh` يعيد شجرة HOME التجريبية إلى حالتها قبل التثبيت تماماً (ملفات ومجلدات وأوضاع ومحتوى)، عدا ترتيب ملف `uca.xml` الذي يبقى فيه الإجراء الآخر ويختفي إجراؤنا. مجلد `$XDG_CONFIG_HOME/sbarhive` يبقى، ويُحذف فقط مع `--purge`.
- AC9: يطبع التثبيت ملخصاً بالعربية والإنكليزية.
- AC10: ملفات سطح المكتب للتطبيق والتشغيل التلقائي تنجح في `desktop-file-validate`، وملف metainfo ينجح في `appstreamcli validate --no-net`، و`shellcheck` نظيف على `install.sh` و`uninstall.sh` وسكربتات الاختبار. لا شرطة طويلة ولا متوسطة في `packaging/` و`scripts/install.sh` و`scripts/uninstall.sh` و`tests/integrations/`.
- AC11: الاختبارات لا تلمس HOME الحقيقي: HOME وكل مجلدات XDG في مجلد مؤقت، و`systemctl` و`hyprctl` و`sbarhive` وهمية في PATH.

## حالات الاختبار
| معيار | الاختبار | النوع |
| --- | --- | --- |
| AC1, AC3, AC4, AC5, AC6 | `tests/integrations/test_install.sh`، سيناريو «الكل موجود»: ثنائيات وهمية عبر `SBARHIVE_BIN_DIR`، ثم التحقق من كل مسار ووضعه ومن سجل `systemctl` الوهمي | تكامل |
| AC2 | نفس الملف: مرة مع `SBARHIVE_BRAND_DIR` فيه أيقونتان، ومرة بمجلد غير موجود | تكامل |
| AC3, AC5 | سيناريو «لا شيء»: `SBARHIVE_FILE_MANAGERS=none` و`SBARHIVE_SYSTEMD=0` ودون `hyprctl`، ثم التحقق من غياب الملفات ووجود ملف autostart | تكامل |
| AC7 | لقطة `find` بالمسار والوضع وsha256 بعد تثبيت واحد وبعد تثبيتين | تكامل |
| AC8 | لقطة قبل التثبيت ثم بعد الإزالة، ومقارنة، ثم `--purge` | تكامل |
| AC9 | grep في المخرجات عن نص عربي ونص إنكليزي ثابتين | تكامل |
| AC10 | `tests/integrations/test_static.sh` | وحدة |
| AC11 | كل الاختبارات تضبط HOME وXDG_* على مجلد `mktemp -d` وتتحقق من أن المسارات المثبتة تبدأ به | تكامل |

## قرارات
- المثبّت يكتب قائمة بكل ملف ومجلد أنشأه في `$XDG_STATE_HOME/sbarhive/install-manifest`، والإزالة تحذف ما في القائمة فقط. دون القائمة تحذف الإزالة المسارات المعروفة.
- متغيرات للاختبار والحزم: `SBARHIVE_BIN_DIR` (مكان الثنائيات المبنية)، `SBARHIVE_BRAND_DIR`، `SBARHIVE_FILE_MANAGERS` (قائمة بفواصل أو `none`)، `SBARHIVE_HYPRLAND` و`SBARHIVE_SYSTEMD` (1 أو 0)، `SBARHIVE_THUNAR_SYSTEM_UCA`.
- لا يشغّل المثبّت `kbuildsycoca6` ولا `nautilus -q` ولا `thunar -q`. يطبعها تلميحاً، لأن إغلاق مدير ملفات مفتوح قرار المستخدم.
- Hyprland دون uwsm لا يفعّل `graphical-session.target` ولا يقرأ ملفات autostart. لذلك يُفعَّل `enable` دون `--now` عندما يكون الهدف غير نشط، ويطبع المثبّت تلميح `exec-once`. والـ CLI يشغّل الخدمة عند الطلب إن لم تكن تعمل (docs/architecture.md)، فالعنصر في القائمة يعمل في كل الأحوال.
- مسار الوحدة `ExecStart` يُستبدل بالمسار المطلق، لأن `%h/.local/bin` خاطئ عندما يختلف PREFIX.

## خارج النطاق
- حزم RPM وDEB وFlatpak.
- تثبيت على مستوى النظام.

## سجل المراجعة
- 2026-10-19: القصة معتمدة مسبقاً من محمد (غائب، فوّض القرارات).
- 2026-10-19 (الاختبارات أولاً، قبل أي تنفيذ): test_install فشل في 48 من 78 فحصاً لغياب `scripts/install.sh` و`scripts/uninstall.sh` وملفات `packaging/`.
- 2026-10-19 (بعد التنفيذ): test_install أخضر، 89 فحصاً في خمسة سيناريوهات (الكل موجود، لا شيء، الكشف التلقائي دون متغيرات، PREFIX مخصص مع Thunar من ملف النظام وهدف غير نشط، ثنائيات مفقودة). HOME الحقيقي لم يُمس: لا ملفات sbarhive فيه بعد التشغيل. الحالة: مراجعة.
- ملاحظة للمراجع: البناء الحقيقي `cargo build --release --locked` لم يُشغَّل في الاختبارات (`--no-build` بثنائيات وهمية)، لأن الـ CLI والخدمة يبنيهما مسار آخر.
