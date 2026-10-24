# التثبيت

## ويندوز 10 و11 (64 بت)

نزّل حزمة MSI. تُثبَّت لحسابك ولا تحتاج صلاحيات المدير. وهناك أيضا ملف ZIP محمول. البناء غير موقّع، فيظهر [تحذير SmartScreen](https://hive.sbarah.com/windows/).

## لينكس (64 بت)

نزّل DEB لأوبونتو وديبيان، أو RPM لفيدورا. يحتاج Nautilus إلى حزمة `nautilus-python`.

## التحقق

قارن كل ملف نزّلته بـ`SHA256SUMS`. الخطوات في [صفحة التنزيل](https://hive.sbarah.com/download/#verify).

## أدوات إضافية

الفيديو والصوت يحتاجان FFmpeg. المستندات وPDF تحتاج LibreOffice وGhostscript. الإجراء الذي تغيب أداته لا يظهر في الخلية. شغّل `sbarhive doctor` لترى الأدوات الموجودة عندك.

## من الشيفرة (لينكس)

```
git clone https://github.com/M7MMAD-OMAR/sbarhive
cd sbarhive
./scripts/install.sh
```

يحتاج Rust 1.85 أو أحدث. للإزالة: `./scripts/uninstall.sh`.
