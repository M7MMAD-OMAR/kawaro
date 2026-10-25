# التثبيت

## ويندوز 10 و11 (64 بت)

نزّل حزمة MSI. تُثبَّت لحسابك ولا تحتاج صلاحيات المدير. وهناك أيضا ملف ZIP محمول. البناء غير موقّع، فيظهر [تحذير SmartScreen](https://kawaro.sbarah.com/windows/).

## لينكس (64 بت)

نزّل DEB لأوبونتو وديبيان، أو RPM لفيدورا. يحتاج Nautilus إلى حزمة `nautilus-python`.

## التحقق

قارن كل ملف نزّلته بـ`SHA256SUMS`. الخطوات في [صفحة التنزيل](https://kawaro.sbarah.com/download/#verify).

## أدوات إضافية

الفيديو والصوت يحتاجان FFmpeg. المستندات وPDF تحتاج LibreOffice وGhostscript. الإجراء الذي تغيب أداته لا يظهر في الخلية. شغّل `kawaro doctor` لترى الأدوات الموجودة عندك.

## من الشيفرة (لينكس)

```
git clone https://github.com/M7MMAD-OMAR/kawaro
cd kawaro
./scripts/install.sh
```

يحتاج Rust 1.85 أو أحدث. للإزالة: `./scripts/uninstall.sh`.
