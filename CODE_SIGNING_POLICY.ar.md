[English](CODE_SIGNING_POLICY.md) | [العربية](CODE_SIGNING_POLICY.ar.md)

# سياسة توقيع الشيفرة

Free code signing provided by [SignPath.io](https://about.signpath.io), certificate by [SignPath Foundation](https://signpath.org/).

(توقيع مجاني للشيفرة مقدَّم من SignPath.io، والشهادة من SignPath Foundation.)

إصدارات كوارو لويندوز (`kawaro.exe` و`kawarod.exe` والملف المضغوط المحمول
ومثبّت MSI) موقّعة بشهادة SignPath Foundation عبر
[SignPath.io](https://about.signpath.io). هذه الصفحة هي سياسة توقيع الشيفرة
لمشروع كوارو.

## ما الذي يُوقَّع

لا تُوقَّع إلا الملفات المبنية من شيفرة هذا المستودع نفسها. تبني مهمة ويندوز
في `.github/workflows/packages.yml` الملفَين `kawaro.exe` و`kawarod.exe` من
مساحة عمل Rust في هذا المستودع على أحد عدّادات GitHub Actions، ثم ترسلهما
إلى SignPath للتوقيع قبل أن تغلفهما `make-zip.sh` وتضمّنهما `make-msi.sh`،
وتوقّع حزمة MSI بالطريقة نفسها، وتتحقق من كل ملف موقّع قبل الرفع. لا تُوقَّع
أي ملفات طرف ثالث بشهادة هذا المشروع. كل طلب توقيع يحتاج موافقة يدوية قبل
توقيع الإصدار.

## الفريق وأدوار توقيع الشيفرة

يُعنى بكوارو شخص واحد، لذلك يحمل العضو نفسه الأدوار الثلاثة:

| الدور | العضو |
| --- | --- |
| المؤلفون (يرتبّون على المستودع دون مراجعة إضافية) | Muhmad Omar Haj Hmdo |
| المراجعون (يراجعون التغييرات القادمة ممن ليسوا مرتّبين) | Muhmad Omar Haj Hmdo |
| المفوّضون (يوافقون على كل طلب توقيع) | Muhmad Omar Haj Hmdo |

يُوافق المفوّض على كل طلب توقيع قبل أن يُوقَّع أي ملف. يستعمل جميع أعضاء
الفريق التحقق بخطوتين على GitHub وعلى SignPath.

## الخصوصية

This program will not transfer any information to other networked systems
unless specifically requested by the user or the person installing or
operating it.

(لن ينقل هذا البرنامج أي معلومات إلى أنظمة شبكات أخرى ما لم يطلب ذلك
المستخدم أو من يثبّت البرنامج أو يشغّله تحديداً.)

تُعالَج الملفات في كوارو على جهاز المستخدم وحده: لا رفع ولا تتبّع ولا حسابات.
بيان الخصوصية الكامل في [PRIVACY.ar.md](PRIVACY.ar.md) وعلى الموقع:
<https://kawaro.sbarah.com/privacy/>. الاتصال الشبكي الوحيد في التطبيق هو فحص
تحديثات اختياري يقرأ بياناً موقّعاً من إصدارات GitHub، ويمكن إيقافه في
`config.toml`. المكوّنات الخارجية المستعملة أو المضمّنة: مكتبة واجهة Slint
(MIT) مترجمة داخل الملفات التنفيذية ولا تتصل بالشبكة، والمحركات الخارجية
الاختيارية (FFmpeg وLibreOffice وGhostscript) تُؤخذ من نظام المستخدم أو
يُثبّتها المستخدم بطلبه، وتُستعمل كعمليات محلية فقط.

## الإصدارات والتحقق

الملفات المنشورة و`SHA256SUMS` على صفحة إصدارات GitHub
(<https://github.com/M7MMAD-OMAR/kawaro/releases>)، وموجز التحديث داخل
التطبيق موقّع منفصلاً بـ minisign (`docs/update-signing.md`). تحذير SmartScreen
وكيفية التحقق من التنزيل مشروحان في `docs/windows-smartscreen.ar.md`.
