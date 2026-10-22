# S-010 ضغط ملفات PDF ودمجها وتقسيمها وتدويرها وتحويل صفحاتها إلى صور
الحالة: منتهية

## القصة
بصفتي مستخدماً يحدد ملف PDF أو أكثر في مدير الملفات، أريد ضغطها أو دمجها أو تقسيمها أو تدويرها أو تحويل صفحاتها إلى صور من الخلية،
حتى أنجز أعمال PDF اليومية محلياً دون برنامج آخر ودون رفع الملفات إلى أي موقع.

## القرارات
- `available()` لكل محرك لا لكل عملية، فانقسم العمل إلى محركين: `pdf` (Rust خالص على lopdf، متاح دائماً: merge وrotate وsplit) و`ghostscript` (يحتاج `gs`: compress وto-images).
- عقد المحرك يكتب ملفاً واحداً لكل مهمة، فـ split وto-images يكتبان ملف ZIP واحداً (`page-001.pdf` أو `page-001.png`...). الناتج مجلداً يحتاج نوع ناتج جديداً في core وقواعد تسمية وتنظيف للمجلد، وهذا تغيير في العقد يحتاج ADR؛ الـ ZIP يحافظ على النقل الذري. التفصيل في `docs/engines.md`.
- compress يبقي الأصل حين لا يصغر الناتج: يُنسخ المدخل إلى `task.output` وتنجح المهمة.

## معايير القبول
- AC1: دمج ملف من صفحتين وملف من ثلاث يعطي PDF من 5 صفحات بترتيب التحديد، وكل صفحة تحتفظ بـ MediaBox وResources الموروثة من شجرة صفحاتها الأصلية. أقل من ملف واحد خطأ معامل، والملف غير الصالح خطأ إدخال يسمي ذلك الملف.
- AC2: التدوير يضع `/Rotate 90` على كل صفحة مباشرة في ملف بلا تدوير سابق، ويضيف إلى التدوير الموروث (270 يصير 0)، و`degrees` يقبل 90 و180 و270 فقط.
- AC3: تقسيم ملف من 3 صفحات يعطي ZIP فيه `page-001.pdf` و`page-002.pdf` و`page-003.pdf`، كل منها صفحة واحدة بمحتواها الصحيح وMediaBox الموروثة، ولا يحمل محتوى الصفحات الأخرى. والتقدم يُبلغ لكل صفحة ولا يتراجع.
- AC4: ضغط PDF فيه صورة غير مضغوطة (600 x 600) بالإعدادات ebook وscreen وprinter يعطي PDF صالحاً من صفحة واحدة بأقل من نصف الحجم.
- AC5: الناتج المضغوط لا يكون أكبر من المدخل أبداً؛ وحين لا يصغر يكون نسخة مطابقة للأصل.
- AC6: تحويل 3 صفحات بحجم 72 x 144 نقطة إلى صور يعطي ZIP فيه 3 ملفات PNG (`page-001.png`...) بحجم 150 x 300 بكسل عند 150 dpi الافتراضية، و72 x 144 عند `dpi = 72`. dpi خارج 36..600 خطأ معامل.
- AC7: الإلغاء أثناء تحويل الصور يعيد `Cancelled` خلال 3 ثوانٍ، ولا تبقى عملية gs، ولا يبقى شيء في المجلد المؤقت. الإلغاء قبل البدء يعيد `Cancelled` لكل العمليات.
- AC8: `preset` خارج screen وebook وprinter خطأ معامل، وخروج gs بخطأ `Failed`، وgs المفقود `Unavailable`، والمدخل المفقود خطأ إدخال، والعملية المجهولة `UnsupportedOp`.
- AC9: `actions/pdf.toml` فيه في الحلقة الأولى: ضغط (`-compressed`)، ودمج (combined، `min_files = 2`، ext pdf، `-merged`)، وصور (ext zip، `-pages`)، وتدوير (`-rotated`)؛ وفي الحلقة الثانية: تقسيم (ext zip، `-split`)، برموز SHRINK وMERGE وIMAGES وROTATE وSPLIT وبالعربية ضغط ودمج وصور وتدوير وتقسيم، وكل عملية موجودة في `capabilities()` لمحركها.

## حالات الاختبار
| معيار | الاختبار | النوع |
| --- | --- | --- |
| AC1 | `pdf_merge_two_and_three_pages_gives_five_in_selection_order`، `pdf_merge_same_file_twice_and_rejects_bad_input` | تكامل |
| AC2 | `pdf_rotate_sets_rotate_90_on_every_page`، `pdf_rotate_adds_to_inherited_rotation_and_honours_degrees`، `pdf::tests::pdf_rotation_math_wraps` | تكامل ووحدة |
| AC3 | `pdf_split_zip_holds_one_single_page_pdf_per_page`، `pdf::tests::pdf_page_names_pad_to_the_page_count` | تكامل ووحدة |
| AC4 | `pdf_compress_shrinks_an_image_heavy_pdf` | تكامل |
| AC5 | `pdf_compress_never_returns_a_larger_file`، `pdf::tests::pdf_keep_original_unless_strictly_smaller` | تكامل ووحدة |
| AC6 | `pdf_to_images_zip_holds_one_png_per_page_sized_by_dpi`، `pdf::tests::pdf_params_default_and_validate` | تكامل ووحدة |
| AC7 | `pdf_ghostscript_cancel_kills_gs_and_cleans_up`، `pdf_engine_cancel_unknown_op_and_capabilities` | تكامل |
| AC8 | `pdf_ghostscript_rejects_bad_params_and_unknown_op`، `pdf_ghostscript_identity_and_capabilities`، `pdf::tests::pdf_gs_lines_and_paths` | تكامل ووحدة |
| AC9 | `pdf_actions_match_engine_capabilities`، `pdf_actions_have_the_planned_rings_outputs_and_codes` | وحدة |

الملفات تُولَّد داخل الاختبارات بـ lopdf، فلا ملفات في `tests/fixtures/docs/`. اختبارات Ghostscript تطبع إشعاراً وتعود مبكراً إن لم يكن مثبتاً.

## خارج النطاق
- ملفات PDF المحمية بكلمة مرور (تُرفض بخطأ إدخال).
- نقل الفهارس (bookmarks) والنماذج (AcroForm) والبنية الموسومة عند الدمج.
- اختيار صفحات بعينها للتقسيم أو التدوير.

## سجل المراجعة
- 2026-10-19، tester: كُتبت الاختبارات على هيكلين يعيدان `UnsupportedOp`. التشغيل `cargo test -p sbarhive-engines --no-fail-fast -- office pdf` فشل: 6 من 6 في `pdf_engine` و6 من 6 في `pdf_ghostscript` و1 من 2 في `pdf_actions` (الناجح يفحص ملف TOML وحده).
- 2026-10-19، implementer: كل الاختبارات نجحت (6 و6 و2 و6 وحدات). لاحظنا أن gs يستغرق 15 إلى 40 ثانية لكل تشغيل على هذا الجهاز المشغول حين يبحث عن خط Helvetica غير مضمّن، فصارت ملفات اختبار gs ترسم مربعات بدل النص؛ ما تتحقق منه الاختبارات لم يتغير، وزمن ملف الاختبار نزل من 90 ثانية إلى أقل من ثانيتين.
- 2026-10-19، reviewer: fmt نظيف؛ clippy نظيف مع السماح بخطأين من شيفرة مسارات أخرى (التفصيل في S-009). نتيجة cargo mutants أدناه.


### Release review 2026-10-20

- 2026-10-20، مراجع الإصدار (HEAD 71576ee، سياق جديد): **REWORK**. مدخل المراجع أعلاه يعد بنتيجة cargo mutants ولا نتيجة. التشغيل على نسخة نظيفة: 14 طفرة ناجية في `crates/engines/src/pdf.rs`، ويمكن إصلاحها في ثلاثة commits:
  1. ترقيم الكائنات في `merge`: `pdf.rs:133` (`&&` إلى `||`)، `pdf.rs:134` (`+` إلى `-` وإلى `*`)، `pdf.rs:159` (`-` إلى `+` وإلى `/`). لا اختبار يفحص أن `max_id` في الناتج يساوي أعلى رقم كائن، ولا أن فهرس المدخل وشجرة صفحاته لا يُنسخان.
  2. قيم التقدم: `pdf.rs:144` (خمس طفرات في `(n + 1) as f32 / total * 0.9`)، `pdf.rs:255` (`+` إلى `*` في `split`)، `pdf.rs:328` (الحارس `total > 0` إلى true، و`>` إلى `>=`). لا اختبار يفحص قيم التقدم الوسيطة ولا غيابها قبل أن يطبع gs عدد الصفحات.
  3. `pdf.rs:212`: `zip_options` يمكن إبداله بـ `Default::default()`. لا اختبار يفحص أن مدخلات ZIP مخزّنة بلا ضغط (Stored).
  - ما نجح: pdf_engine 6 وpdf_ghostscript 6 وpdf_actions 2 ناجحة، وe2e يدمج ملفي PDF.
- 2026-10-22، تحديث الحالة: الطفرات الناجية الأربع عشرة عولجت في `b95902e`: ترقيم الكائنات في `merge` من أعلى رقم باقٍ بعد `prune_objects`، وقراءة تقدم Ghostscript في `GsProgress` مع اختباراته، واختبارات لترقيم الدمج وتقدمه وتقدم التقسيم ومدخلات ZIP المخزّنة بلا ضغط. صدرت القصة في v0.2.0 والإصلاح ضمنه. لا مدخل مراجعة مستقلة بعد الإصلاح في هذا السجل؛ الحالة تتبع الإصدار.
