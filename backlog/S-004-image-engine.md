# S-004 بناء محرك الصور لكل عمليات جدول المواصفات
الحالة: مراجعة

## القصة
بصفتي مستخدماً يحدد صوراً في مدير الملفات، أريد تحويلها وتعديلها (صيغة، حجم، تدوير، قلب، رمادي، ضغط، إزالة البيانات الوصفية، دمج في PDF) محلياً،
حتى أحصل على نسخة جديدة بجانب الأصل دون رفع أي ملف إلى خادم.

المرجع: قسم "Image engine operations" في `docs/architecture.md`.

## معايير القبول
- AC1: `ImageEngine::id()` يعيد `image`، و`capabilities()` تعيد العمليات الثماني بالضبط (convert resize rotate flip grayscale compress strip to-pdf)، ولكل منها `MediaKind::Image` وحدها.
- AC2: `convert` يكتب صيغة امتداد `task.output` لكل من png jpg jpeg webp avif gif bmp tif tiff ico، ويبدأ الناتج بالبايتات السحرية للصيغة، وأبعاده تساوي أبعاد المصدر (عدا ico). امتداد غير معروف يعيد `BadParam` باسم `output`.
- AC3: `quality` خارج 1..100 يعيد `BadParam` باسم `quality`. JPEG بجودة 30 أصغر من JPEG بجودة 95 للمصدر نفسه. WebP بجودة أقل من 100 مضغوط بخسارة (مقطع `VP8 `)، وبجودة 100 بلا خسارة (مقطع `VP8L`).
- AC4: الصيغ بلا قناة شفافية (JPEG وBMP) تُسطَّح فيها الشفافية على أبيض: بكسل شفاف كلياً في المصدر يصبح أبيض (كل قناة 240 فأكثر في JPEG، 255 في BMP).
- AC5: ICO الناتج عن مصدر 300x200 يحوي 6 صور بأحجام 16 32 48 64 128 256 مربعة، وعن مصدر 64x48 يحوي 4 صور (16 32 48 64). المصدر غير المربع يُحشى بشفافية ولا يُمطّ.
- AC6: `resize` بـ `scale` 50 على 64x48 يعطي 32x24، وبـ `max_side` 32 يعطي 32x24، و`max_side` أكبر من المصدر لا يكبّره. غياب المعاملين، أو `scale` خارج 1..1000، أو `max_side` أقل من 1، يعيد `BadParam`.
- AC7: `rotate` بـ 90 و270 يبدّل العرض والارتفاع وينقل البكسل الصحيح، و180 يعكس الزوايا. قيمة أخرى أو غيابها يعيد `BadParam` باسم `degrees`.
- AC8: `flip` بـ `horizontal` و`vertical` يعكس البكسلات على المحور الصحيح. قيمة أخرى يعيد `BadParam` باسم `axis`.
- AC9: `grayscale` ينتج بكسلات قنواتها اللونية متساوية، بالأبعاد نفسها.
- AC10: اتجاه EXIF يُطبَّق قبل أي عملية: JPEG بعرض 64 وارتفاع 48 مع Orientation=6 يخرج من `convert` إلى PNG بأبعاد 48x64، وبكسله الأحمر في الزاوية العليا اليسرى ينتقل إلى الزاوية العليا اليمنى.
- AC11: `compress` لـ JPEG مُرمَّز بجودة 95 يعطي ملف JPEG أصغر من المصدر. `compress` لـ PNG يعطي PNG صالحاً بالأبعاد نفسها مطابق البكسلات، وحجمه أصغر من ترميز PNG بالضغط المتوازن (`CompressionType::Default`) الذي يستخدمه `convert`.
- AC12: `strip` لـ JPEG فيه مقطع EXIF يعطي JPEG بلا علامة `Exif` ولا مقطع APP1، بالأبعاد نفسها بعد تطبيق الاتجاه.
- AC13: `to-pdf` لثلاث صور يكتب PDF يبدأ بـ `%PDF-` وفيه 3 صفحات بالضبط، وحجم كل صفحة (MediaBox) يساوي أبعاد صورتها بالترتيب. JPEG بلا تدوير يُضمَّن كما هو (`/DCTDecode` وبايتات الملف الأصلي موجودة داخل الـ PDF).
- AC14: يُستدعى التقدم بـ 0.0 أولاً و1.0 أخيراً، والقيم لا تتناقص. في `to-pdf` لثلاث صور يوجد على الأقل 3 قيم وسيطة بين 0 و1.
- AC15: إلغاء قبل البدء يعيد `Cancelled` ولا يُنشئ `task.output`، لكل عملية.
- AC16: ملف إدخال غير موجود أو تالف يعيد `Input` ومساره هو مسار الإدخال. عدد إدخالات غير 1 لعملية فردية، أو صفر لـ `to-pdf`، يعيد `BadParam` باسم `inputs`. عملية غير معروفة تعيد `UnsupportedOp`. إدخال AVIF يعيد `Input` برسالة تذكر أن فك AVIF غير مدعوم.

## حالات الاختبار
| معيار | الاختبار | النوع |
| --- | --- | --- |
| AC1 | مقارنة قائمة العمليات والأنواع | وحدة |
| AC2 | تحويل PNG 64x48 بشفافية إلى كل امتداد، فحص البايتات السحرية وفك الناتج ومقارنة الأبعاد | تكامل |
| AC3 | quality 0 و101، حجم JPEG بجودة 30 مقابل 95، فحص مقطع VP8 / VP8L | تكامل |
| AC4 | بكسل (0,0) شفاف في المصدر، قراءته من JPEG وBMP الناتجين | تكامل |
| AC5 | فك ترويسة ICO وعدّ المدخلات وأحجامها، وفك أكبر صورة والتحقق من حشو الشفافية | تكامل |
| AC6 | أبعاد الناتج للحالات الثلاث وأخطاء المعاملات | تكامل |
| AC7 | موقع بكسل مميز بعد التدوير لكل زاوية | تكامل |
| AC8 | موقع بكسل مميز بعد القلب لكل محور | تكامل |
| AC9 | كل بكسل R=G=B | تكامل |
| AC10 | `tests/fixtures/images/orient6.jpg` إلى PNG، الأبعاد ولون الزاوية | تكامل |
| AC11 | مقارنة الأحجام | تكامل |
| AC12 | بحث عن `Exif` و`FF E1` في الناتج | تكامل |
| AC13 | عدّ `/Type /Page` (دون `/Pages`) وقراءة MediaBox، والبحث عن بايتات JPEG | تكامل |
| AC14 | تسجيل قيم التقدم | تكامل |
| AC15 | رمز ملغى مسبقاً لكل عملية | تكامل |
| AC16 | مسار غير موجود، ملف نصي باسم png، عدد إدخالات خاطئ، عملية مجهولة، ملف avif | تكامل |

## خارج النطاق
- فك AVIF (يحتاج dav1d من النظام)، وHEIC وJXL.
- الصور المتحركة: GIF وWebP المتحركان يُعامَلان بإطارهما الأول.
- الحفاظ على ملف ICC اللوني عند إعادة الترميز.
- خيار `page` غير `fit` في `to-pdf` (القيمة الوحيدة الآن، وأي قيمة أخرى `BadParam`).

## سجل المراجعة
- 2026-10-19 planner: كُتبت القصة بحالة معتمدة (موافقة مسبقة من المالك لهذا المسار).
- 2026-10-19 tester: الاختبارات في `crates/engines/tests/image_{ops,formats,pdf,fixtures}.rs` والمساعد المشترك في `tests/image_common/mod.rs`. الملفات الثابتة في `tests/fixtures/images/` (alpha.png وorient6.jpg وsample.gif وsample.bmp) يولّدها الاختبار المتجاهَل `regenerate`، ويتحقق `fixtures_are_current` من محتواها. التشغيل الأول على المحرك الفارغ (`cargo test -p sbarhive-engines --release --no-fail-fast`):

  ```
  Running tests/image_fixtures.rs   test result: ok. 1 passed; 0 failed; 1 ignored
  Running tests/image_formats.rs    test result: FAILED. 0 passed; 17 failed
  Running tests/image_ops.rs        test result: FAILED. 1 passed; 21 failed
  Running tests/image_pdf.rs        test result: FAILED. 0 passed; 6 failed
  ```

  الاختبار الوحيد الناجح هو `unknown_op` لأن المحرك الفارغ كان يعيد `UnsupportedOp` لكل عملية.
- 2026-10-19 tester: تعديل بعد قراءة شيفرة `image`: `PngEncoder::new` يستخدم `CompressionType::Fast` لا `Default`، فصار مرجع مقارنة PNG هو الضغط المتوازن (`Default`) صراحةً، وصار شرط AC11 لـ PNG "أصغر من الضغط المتوازن" بدل "لا يزيد". وعتبة AC4 لـ JPEG صارت 240 لأن أخذ عينات اللون في JPEG يمس الحواف.
- 2026-10-19 implementer: `crates/engines/src/image.rs` (التوزيع والمعاملات والفك والاتجاه)، و`image/encode.rs` (الصيغ والتسطيح وICO)، و`image/pdf.rs`. كل الاختبارات تنجح: 11 وحدة، و1 و20 و22 و6 تكامل (60 اختباراً بعد إضافة اختباري الطفرات أدناه).
- 2026-10-19 reviewer: `cargo clippy -p sbarhive-engines --all-targets -- -D warnings` يفشل بأخطاء في `crates/core` (queue.rs وi18n.rs وipc.rs) خارج ملكية هذا المسار. البوابة على هذا الـ crate وحده تنجح: `cargo clippy -p sbarhive-engines --all-targets --no-deps -- -D warnings`. يُرفع إصلاح core لمسار core.
- 2026-10-19 reviewer: `cargo mutants -f crates/engines/src/image.rs -f 'crates/engines/src/image/*.rs' --test-package sbarhive-engines --timeout 120 -j 4`: 130 طفرة، 114 مقتولة، 12 غير قابلة للبناء، 4 ناجية: (1) `==` إلى `!=` في اختيار الجودة الافتراضية في `run_single`، و(2 إلى 4) حساب الحشو الأفقي `(side - w) / 2` في `ico`، لأن اختبارات ICO كلها كانت بمصادر عريضة حشوها الأفقي صفر. أُضيف اختباران: `default_quality_is_90_and_75_for_compress` و`ico_pads_tall_sources_left_and_right`. إعادة التشغيل على الدالتين: 13 طفرة، 13 مقتولة. النتيجة: لا طفرات ناجية.
- 2026-10-19 reviewer: `cargo fmt --all --check` و`cargo test -p sbarhive-engines --release` ينجحان. الحالة: مراجعة.
