# ADR 0002: مكونات نافذة الإعدادات

الحالة: مقبول
التاريخ: 2026-10-20

## السياق

يطلب CLAUDE.md (قسم «الواجهة») أن تُبنى نوافذ الإعدادات من مكتبة Material 3 الرسمية من Slint، بألوان من `design/tokens.json` تُمرَّر مرة واحدة إلى `MaterialPalette.schemes`، وبلا أي ظل. البديل الأول Sleek-ui، والأخير مكونات Slint القياسية بنمط fluent. إصدار Slint مثبت على 1.18.1.

## ما جرّبناه

1. **المستودع المستقل `slint-ui/material-components`:** مؤرشف منذ سبتمبر 2025، بلا إصدارات ولا وسوم، ويقول ملف README فيه إن التطوير انتقل إلى مستودع Slint نفسه. لم نعتمده لأنه لا يطابق إصداراً بعينه.
2. **المكتبة داخل مستودع Slint:** موجودة في `ui-libraries/material/src` عند الوسم `v1.18.1` نفسه، أي أنها مطابقة للإصدار المثبت. نزّلناها من أرشيف الوسم (`codeload.github.com/slint-ui/slint/tar.gz/refs/tags/v1.18.1`)، ورخصتها MIT. بُنيت نافذة الإعدادات عليها دون أي خطأ أو تحذير من المترجم.

لم نحتج إلى تجربة Sleek-ui ولا fluent.

## القرار

- نعتمد مكتبة Material 3 من مستودع Slint عند الوسم v1.18.1، منسوخة في `crates/ui/material/` مع رخصتها وملف README يذكر المصدر والبصمة وكل تعديل.
- تُستورد باسم `@material` (مسار مكتبة في `crates/ui/build.rs`)، وتُترجم نافذة الإعدادات في ملف Rust مستقل (`settings_ui.rs`) تضمّه `src/settings.rs` في وحدة خاصة، لأن `include_modules!()` يأخذ آخر ملف مترجم فقط وهو الخلية.
- المكونات المستعملة: `MaterialWindow`، `SegmentedButton` (اللغة والمظهر ومكان النتائج والجودة)، `TextField` (اسم المجلد)، `Switch`، `FilledButton` (حفظ)، `TextButton` (إلغاء). لم نبنِ أي مكوّن موجود في المكتبة، والمكوّنان الصغيران في `settings.slint` (`Choice` و`SwitchRow`) تجميع لمكونات المكتبة مع عنوان أو تسمية.

## التعديلات على المكتبة

1. **الظلال:** `elevation.slint` صار مستطيلاً مدوّراً بلا ظل، ويحتفظ بخصائصه حتى تبقى بقية المكتبة كما هي. لا نضبط `level` في ملفاتنا.
2. **حقل النص:** كان خطاً سفلياً وحده مع زوايا علوية مدوّرة، وهذا حد من جهة واحدة. صار حداً كاملاً مدوّراً يزداد سمكه عند التركيز، وأضفنا خاصية `horizontal_alignment` لمحاذاة النص والتسمية إلى اليمين في العربية.
3. **الأزرار المجزأة:** حذفنا الخط الفاصل الجانبي بين الأجزاء (خط تمييز جانبي)، فالجزء المختار يتميز بسطحه وعلامة الصح. ودوّرنا زوايا الجزء المختار الطرفي بنفسه، لأن المُصيِّر البرمجي لا يقص الأبناء على حدود الأب المدوّرة، فكان اللون يخرج من الحافة.

## الألوان

يقرأ `build.rs` حقول `MaterialScheme` من الملف المنسوخ، ويفشل البناء إن لم يطابقها جدول `MATERIAL_ROLES` حقلاً بحقل، فلا يبقى دور بلا لون. ثم يكتب `material-tokens.slint`، وتسنده النافذة مرة واحدة إلى `MaterialPalette.schemes` في `init`.

| الرمز | أدوار Material |
| --- | --- |
| `accent` | primary، surfaceTint، tertiary، primaryFixed وأخواتها |
| `accent-ink` | onPrimary، onTertiary، onPrimaryFixed |
| `surface-0` | background، surface، surfaceBright، surfaceContainerLowest، primaryContainer (مقبض المفتاح) |
| `surface-1` | surfaceContainer، surfaceContainerLow، surfaceDim |
| `surface-2` | surfaceContainerHigh، surfaceVariant، tertiaryContainer، errorContainer |
| `surface-3` | surfaceContainerHighest، secondaryContainer (الجزء المختار) |
| `text-primary` | onSurface، onBackground، on*Container، shadow، scrim |
| `text-secondary` | outline، secondary، onSurfaceVariant |
| `border` | outlineVariant |
| `danger` | error، onErrorContainer |
| الرمز نفسه من المظهر الآخر | inverseSurface، inverseOnSurface، inversePrimary |

اخترنا `text-secondary` لدور outline لا `border`، لأن outline يرسم حدود المفتاح المطفأ والأزرار المجزأة وحقل النص، ورمز `border` أضعف من أن يُرى عليها.

المظهر: تتبع النافذة `Palette.color-scheme` القياسي، لأن `MaterialPalette` يختار منه. `SBARHIVE_COLOR_SCHEME` يفرضه أولاً (كما في الخلية)، ثم قيمة المظهر في الإعدادات (فاتح أو داكن)، والتلقائي يتبع النظام. لذلك تُبنى النافذة من جديد في كل `open`، فلا يبقى مظهر مفروض بعد العودة إلى التلقائي. تغيير المظهر داخل النافذة لا يغيّر شكلها قبل الحفظ.

## فحص التصميم

المكتبة المنسوخة فيها ألوان وأطوال مكتوبة يدوياً، فيتجاوز `scripts/design-lint.sh` المسار `crates/ui/material/` في فحصي اللون والطول فقط، بنمط مسار كامل لا بكلمة `material` وحدها حتى لا يخفي ملفاتنا. فحص الظلال والتدرجات والشرطات يمر على المكتبة أيضاً، وهو الدليل على أن الظلال أُزيلت. أضفنا فحص الشرطة المتوسطة بجانب الطويلة.

## ما بقي ومعروف

- أوزان الخط في `MaterialTypography` (300 و600) لا تطابق رموزنا (400 و500 و600)، والخط المضمّن يختار أقرب وزن. تغييرها يحتاج تعديلاً رابعاً على المكتبة، وأجّلناه.
- علامة الصح في الجزء المختار تبقى يسار النص في العربية، لأن ترتيبها داخل زر المكتبة. الترتيب بين الأجزاء معكوس كاملاً.
- النافذة والخلية في عملية واحدة وحلقة أحداث واحدة. معالج التركيز في `lib.rs` يستقبل أحداث التركيز لكل النوافذ؛ على Wayland يصل خروج التركيز من النافذة القديمة قبل دخوله الجديدة، و`present` يمسح حالة التركيز، فلا نتوقع أن تُغلق الخلية بسبب نافذة الإعدادات. لم يُختبر هذا على مؤلِّف حقيقي.
- اللقطات من KWin مخفي خاص؛ لم تُفتح النافذة على GNOME أو KDE حقيقيين.
