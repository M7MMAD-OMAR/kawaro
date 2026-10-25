Kawaro for Windows (beta)
===========================

Kawaro converts files on your own computer. No file leaves your machine.
This beta is not signed, so Windows may warn you the first time.

Install in three steps
----------------------
1. Unzip the folder anywhere you like (for example %LOCALAPPDATA%\Programs\Kawaro).
2. Open a terminal in that folder and run:
       kawaro integration install
   This adds Kawaro to the right click menu of files. No administrator
   rights are needed. To remove it again: kawaro integration uninstall
3. Right click a file. On Windows 11 choose "Show more options" first, then
   Kawaro.

If you installed the .msi instead, steps 1 and 2 are done for you and
Kawaro starts when you sign in.

SmartScreen
-----------
Windows SmartScreen may say "Windows protected your PC". Click "More info",
then "Run anyway". This appears because the beta has no code signing
certificate. Check the download first: the SHA-256 value of the file must
match the one on the release page (SHA256SUMS inside the zip lists every
file).

Your own actions go in %APPDATA%\Kawaro\actions\*.toml. The folder
actions\ in this package holds the built in action files for reference.

------------------------------------------------------------------------

كوارو لنظام ويندوز (نسخة تجريبية)
=====================================

يحوّل كوارو الملفات على حاسوبك نفسه، ولا يغادر أي ملف جهازك.
هذه النسخة التجريبية غير موقّعة، فقد يحذّرك ويندوز في المرة الأولى.

التثبيت في ثلاث خطوات
---------------------
1. فُكّ ضغط المجلد (unzip) في أي مكان تريده.
2. افتح الطرفية داخل المجلد ونفّذ:
       kawaro integration install
   يضيف هذا كوارو إلى قائمة الزر الأيمن للملفات دون صلاحيات المدير.
   وللإزالة: kawaro integration uninstall
3. انقر بزر الفأرة الأيمن على ملف. في ويندوز 11 اختر أولاً
   "Show more options" (إظهار خيارات إضافية)، ثم كوارو.

إن ثبّتّ ملف .msi بدلاً من ذلك فالخطوتان 1 و2 تتمّان عنك،
ويبدأ كوارو عند تسجيل الدخول.

SmartScreen
-----------
قد تعرض SmartScreen عبارة "Windows protected your PC". اضغط "More info"
ثم "Run anyway". يظهر هذا لأن النسخة التجريبية بلا شهادة توقيع.
تحقق من التنزيل أولاً: يجب أن تطابق قيمة SHA-256 للملف القيمة المنشورة
في صفحة الإصدار (ويسرد SHA256SUMS داخل الملف المضغوط كل ملف).

إجراءاتك الخاصة تضعها في %APPDATA%\Kawaro\actions\*.toml. أما المجلد
actions\ في هذه الحزمة فيحمل ملفات الإجراءات المضمّنة للاطلاع.
