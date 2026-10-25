# كيف يعمل

خدمة واحدة تعرض الواجهة. وعميل خفيف بسطر الأوامر يطلب منها فتح الخلية.

![من النقر الأيمن إلى الناتج](https://kawaro.sbarah.com/diagrams/flow.ar.light.svg)

## الأجزاء

![العمليات والحزم](https://kawaro.sbarah.com/diagrams/overview.ar.light.svg)

- الخدمة `kawarod` فيها الخلية والطابور والصينية والمحركات.
- FFmpeg وLibreOffice وGhostscript تعمل كعمليات منفصلة، وتُقتل مع شجرتها كاملة عند الإلغاء.
- الملف الأصلي لا يتغير: يُكتب الناتج في ملف مؤقت ثم يُعاد تسميته.

## إضافة صيغة

![ملف TOML ومحوّل](https://kawaro.sbarah.com/diagrams/extend.ar.light.svg)

## أين تُرسم الخلية

![مسار الطبقة أو مسار النافذة](https://kawaro.sbarah.com/diagrams/surface.ar.light.svg)

## التحديثات

![تحديثات موقّعة](https://kawaro.sbarah.com/diagrams/updates.ar.light.svg)
