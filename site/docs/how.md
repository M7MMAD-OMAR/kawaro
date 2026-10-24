# كيف يعمل

خدمة واحدة تعرض الواجهة. وعميل خفيف بسطر الأوامر يطلب منها فتح الخلية.

![من النقر الأيمن إلى الناتج](https://hive.sbarah.com/diagrams/flow.ar.light.svg)

## الأجزاء

![العمليات والحزم](https://hive.sbarah.com/diagrams/overview.ar.light.svg)

- الخدمة `sbarhived` فيها الخلية والطابور والصينية والمحركات.
- FFmpeg وLibreOffice وGhostscript تعمل كعمليات منفصلة، وتُقتل مع شجرتها كاملة عند الإلغاء.
- الملف الأصلي لا يتغير: يُكتب الناتج في ملف مؤقت ثم يُعاد تسميته.

## إضافة صيغة

![ملف TOML ومحوّل](https://hive.sbarah.com/diagrams/extend.ar.light.svg)

## أين تُرسم الخلية

![مسار الطبقة أو مسار النافذة](https://hive.sbarah.com/diagrams/surface.ar.light.svg)

## التحديثات

![تحديثات موقّعة](https://hive.sbarah.com/diagrams/updates.ar.light.svg)
