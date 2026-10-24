# استكشاف الأخطاء

- **إجراء غير ظاهر:** أداته غير مثبتة. شغّل `sbarhive doctor`.
- **ويندوز يحذّر «Windows protected your PC»:** البناء غير موقّع. [تحقق من الملف](https://m7mmad-omar.github.io/sbarhive/download/#verify) ثم اختر More info ثم Run anyway.
- **تفتح الخلية بلا تركيز على لوحة المفاتيح (GNOME وKDE):** لم تكتمل اختبارات التركيز على أجهزة حقيقية. بلّغنا مع اسم سطح المكتب وإصداره.
- **لا شيء يحدث عند النقر الأيمن:** شغّل `sbarhive ping`. وإن لم تجب الخدمة فشغّل `sbarhive daemon` وأعد المحاولة.
- **التحديثات:** اضبط `update_check = false` لإيقاف الفحص اليومي ([الإعدادات](settings.md)).

بلّغ عن المشكلة على [GitHub](https://github.com/M7MMAD-OMAR/sbarhive/issues).
