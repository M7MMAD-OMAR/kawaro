# استكشاف الأخطاء

- **إجراء غير ظاهر:** أداته غير مثبتة. شغّل `kawaro doctor`.
- **ويندوز يعرض رسالة Windows protected your PC عند التشغيل:** البناء غير موقّع. [تحقق من الملف](https://kawaro.sbarah.com/download/#verify) ثم اختر More info ثم Run anyway.
- **تفتح الخلية ولا تستجيب للوحة المفاتيح (GNOME وKDE):** لم تكتمل اختبارات التركيز على أجهزة حقيقية. بلّغنا مع اسم سطح المكتب وإصداره.
- **لا شيء يحدث عند النقر الأيمن:** شغّل `kawaro ping`. وإن لم تردّ الخدمة فشغّل `kawaro daemon` وأعد المحاولة.
- **التحديثات:** اضبط `update_check = false` لإيقاف الفحص اليومي ([الإعدادات](settings.md)).

بلّغ عن المشكلة على [GitHub](https://github.com/M7MMAD-OMAR/kawaro/issues).
