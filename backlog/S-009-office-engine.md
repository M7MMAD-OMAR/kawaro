# S-009 تحويل مستندات Office وODF عبر LibreOffice بلا واجهة
الحالة: منتهية

## القصة
بصفتي مستخدماً يحدد مستندات Word أو جداول أو عروضاً تقديمية في مدير الملفات، أريد تحويلها إلى PDF أو إلى DOCX وODT وXLSX وODS وPPTX وODP من الخلية،
حتى أحصل على نسخة بالصيغة التي أحتاجها دون فتح LibreOffice ودون أن يتأثر LibreOffice المفتوح عندي.

## معايير القبول
- AC1: المحرك `office` يقدم أربع عمليات للمستندات: `to-pdf` (doc docx odt rtf txt md xls xlsx ods csv ppt pptx odp)، و`convert-doc` إلى docx أو odt، و`convert-sheet` إلى xlsx أو ods، و`convert-slides` إلى pptx أو odp، والصيغة الهدف من امتداد `task.output`.
- AC2: ملف RTF من صفحتين يتحول إلى DOCX صالح (فيه `word/document.xml`)، ثم يتحول DOCX إلى PDF يبدأ بـ `%PDF-` وعدد صفحاته 2.
- AC3: TXT إلى ODT صالح (mimetype النص المفتوح)، وMarkdown إلى PDF من صفحة واحدة، وCSV إلى XLSX ثم ODS ثم PDF، وODP من شريحتين إلى PPTX ثم PDF من صفحتين.
- AC4: كل تشغيل يستعمل ملف مستخدم خاصاً في مجلد مؤقت (`-env:UserInstallation`)، ولا يبقى في المجلد المؤقت أي ملف بعد النجاح أو الفشل أو الإلغاء أو انتهاء المهلة، ولا يظهر بجانب الملف الأصلي أي ملف قفل، ولا يتغير الأصل.
- AC5: انتهاء المهلة (`timeout_s`، الافتراضي 180) يعيد خطأ فيه «timed out» بعد المهلة وقبل 5 ثوانٍ منها، ويقتل مجموعة العمليات كلها، بما فيها العمليات الحفيدة، خلال 3 ثوانٍ.
- AC6: الإلغاء يعيد `Cancelled` خلال 3 ثوانٍ ويقتل مجموعة العمليات كلها، ومع LibreOffice الحقيقي لا تبقى أي عملية تذكر مسار المجلد المؤقت بعد 3 ثوانٍ. والإلغاء قبل البدء لا يشغّل أي عملية.
- AC7: التقدم 0.0 عند البدء و1.0 عند الانتهاء.
- AC8: مدخل من عائلة خاطئة خطأ إدخال، وامتداد ناتج لا يناسب العملية أو `timeout_s` غير موجب أو أكثر من ملف خطأ معامل، والعملية المجهولة `UnsupportedOp`، وكل ذلك قبل إنشاء أي مجلد عمل. البرنامج المفقود خطأ `Unavailable`، وخروج LibreOffice بلا ملف ناتج خطأ `Failed`.
- AC9: `actions/documents.toml` يحوي «تحويل إلى PDF» في الحلقة الأولى لكل المستندات، وتحويلات DOCX وODT وXLSX وODS وPPTX وODP، وكل تحويل يُخفى لصيغته نفسها وللعائلات الأخرى عبر `skip_ext`، وكل عملية فيه موجودة في `capabilities()`.

## حالات الاختبار
| معيار | الاختبار | النوع |
| --- | --- | --- |
| AC1 | `office_capabilities_list_the_four_ops_for_documents`، `office::tests::office_plan_maps_targets_to_filters` | وحدة |
| AC2، AC4، AC7 | `office_rtf_to_docx_then_pdf_keeps_two_pages_and_removes_profile` | تكامل |
| AC3 | `office_text_to_odt_and_markdown_to_pdf`، `office_csv_to_xlsx_and_ods_then_pdf`، `office_odp_to_pptx_then_pdf_keeps_two_slides` | تكامل |
| AC5 | `office_timeout_kills_the_process_group` (برنامج وهمي يشغّل `sleep` حفيداً) | تكامل |
| AC6 | `office_cancel_kills_the_process_group`، `office_cancel_stops_real_libreoffice_and_cleans_up`، `office_already_cancelled_starts_nothing` | تكامل |
| AC8 | `office_rejects_wrong_family_bad_target_and_unknown_op`، `office_reports_failure_when_no_output_is_produced`، `office_missing_program_is_unavailable_error`، ووحدات `timeout_param` و`find_program` و`run_child` | وحدة وتكامل |
| AC9 | `office_actions_match_engine_capabilities`، `office_actions_cover_pdf_and_each_conversion` | وحدة |

اختبارات LibreOffice الحقيقية تطبع إشعاراً وتعود مبكراً إن لم يكن مثبتاً.

## خارج النطاق
- doc وxls وppt صيغاً ناتجة (القديمة تُقرأ فقط).
- تقدم حقيقي أثناء التحويل (LibreOffice لا يبلغ عنه).
- ويندوز: قتل المجموعة يقتصر هناك على العملية المباشرة.

## سجل المراجعة
- 2026-10-19، tester: كُتبت الاختبارات على هيكل يعيد `UnsupportedOp`. التشغيل `cargo test -p kawaro-engines --no-fail-fast -- office pdf` فشل: 12 من 12 في `office_engine` و1 من 2 في `office_actions` (الاختبار الناجح يفحص ملف TOML وحده).
- 2026-10-19، implementer: فشلت ثلاثة اختبارات بعد التنفيذ لأن crate zip بلا ميزات لا يقرأ الأرشيفات المضغوطة بـ deflate التي يكتبها LibreOffice. أُضيفت ميزة `deflate-flate2` في dev-dependencies فقط، دون تعديل الاختبارات. النتيجة: 12 من 12 و2 من 2 و7 وحدات.
- 2026-10-19، reviewer: `cargo fmt --all --check` نظيف. `cargo clippy -p kawaro-engines --all-targets -- -D warnings` يفشل على شيفرة مسارات أخرى فقط: `dead_code` في `crates/core/src/queue.rs` (placeholder) و`default_constructed_unit_structs` في سطر `image::ImageEngine::default()` في `lib.rs`؛ مع `-A dead_code -A clippy::default_constructed_unit_structs` نظيف. نتيجة cargo mutants أدناه.


### Release review 2026-10-20

- 2026-10-20، مراجع الإصدار (HEAD 71576ee، سياق جديد): **REWORK**. مدخل المراجع أعلاه يعد بنتيجة cargo mutants «أدناه» ولا نتيجة، و`office.rs` غير مشمول في `scripts/release-gates.sh:41-43`. شغّلتُ `cargo mutants -f crates/engines/src/office.rs -f crates/engines/src/pdf.rs` على نسخة نظيفة: ثلاث طفرات ناجية في هذا الملف:
  1. `crates/engines/src/office.rs:299` (`is_executable` إلى true وإلى false) و`crates/engines/src/office.rs:408` (`kill_group` إلى `()`). الدالتان تحت `#[cfg(not(unix))]` فلا تُبنيان على لينكس ولا يمكن لأي اختبار هنا قتلهما. الإصلاح (commit واحد): استثناؤهما من cargo mutants (`#[mutants::skip]` أو `exclude_re` في `.cargo/mutants.toml`)، وإضافة `office.rs` و`pdf.rs` إلى قائمة الطفرات في بوابات الإصدار.
  - ما نجح: office_engine 12 وoffice_actions 2 ناجحة، وe2e يحوّل نصاً إلى PDF.
- 2026-10-22، تحديث الحالة: الطفرات الناجية عولجت في `b95902e`: نُقلت `is_executable` و`kill_group` لغير يونكس إلى `office/non_unix.rs` واستُثني من cargo mutants، وأُضيف `office.rs` و`pdf.rs` إلى طفرات `scripts/release-gates.sh`. صدرت القصة في v0.2.0 والإصلاح ضمنه. لا مدخل مراجعة مستقلة بعد الإصلاح في هذا السجل؛ الحالة تتبع الإصدار.
