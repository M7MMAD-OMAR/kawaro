[English](README.md) | [العربية](README.ar.md)

الموقع: <https://kawaro.sbarah.com/>

<div dir="rtl">

# كوارو

حدّد ملفاتك في مدير الملفات، وافتح كوارو من قائمة النقر اليمين، ثم اختر من الخلية ما تريد. يُحفظ الناتج بجانب الملف الأصلي، ولا يُمس الأصل.

يعمل كل شيء على جهازك: لا رفع ولا خوادم ولا جمع بيانات.

<picture><source media="(prefers-color-scheme: dark)" srcset="docs/diagrams/hero.ar.dark.svg"><img src="docs/diagrams/hero.ar.light.svg" alt="خلية كوارو لصورة واحدة: الخلية المركزية تسمّي التحديد، والحلقة الأولى فيها صيغ الهدف، والثانية إجراءات الصورة" width="900"></picture>

رسم مأخوذ من هندسة التطبيق نفسها ورموز التصميم.

## ماذا يفعل

- **الصور:** التحويل بين PNG وJPEG وWebP وAVIF وGIF وBMP وTIFF وICO، وتغيير الحجم، والتدوير، والقلب، والتدرج الرمادي، والضغط، وحذف البيانات الوصفية، وجمع عدة صور في ملف PDF واحد.
- **الفيديو والصوت** (الإصدار 0.2، يحتاج FFmpeg): التحويل إلى MP4 وWebM وGIF، واستخراج الصوت، وتحويل الصوت بين MP3 وOGG وFLAC وWAV وM4A وOpus، وضغط الفيديو.
- **المستندات وPDF** (الإصدار 0.2، يحتاج LibreOffice وGhostscript): تحويل مستندات Office وODF إلى PDF، والتحويل بين DOCX وODT في الاتجاهين، وضغط ملفات PDF ودمجها وتدويرها، وتحويل صفحاتها إلى صور.

لا تعرض الخلية إلا ما يناسب الملفات المحددة: نوعها، وعددها، والأدوات المثبتة على جهازك.

## الاسم

«كوارو» (Kawaro) مأخوذ من «كوّارة»، وهي كلمة عربية قديمة لخلية النحل. التطبيق خلية من الإجراءات، فاختصرنا الكلمة إلى اسم يُقرأ بالعربية واللاتينية بالطريقة نفسها: كوارو. والشعار كوّارة تقليدية مقبّبة داخل خلية سداسية واحدة.

## كيف يعمل

<picture><source media="(prefers-color-scheme: dark)" srcset="docs/diagrams/flow.ar.dark.svg"><img src="docs/diagrams/flow.ar.light.svg" alt="سبع خطوات من تحديد الملفات إلى الناتج بجانب الأصل" width="860"></picture>

<picture><source media="(prefers-color-scheme: dark)" srcset="docs/diagrams/overview.ar.dark.svg"><img src="docs/diagrams/overview.ar.light.svg" alt="مدير الملفات يستدعي سطر الأوامر الذي يكلّم عبر مقبس محلي الخدمة التي تضم الخلية والطابور والصينية والسجل والمحركات" width="860"></picture>

- **خدمة واحدة تعرض الواجهة.** `kawarod` فيها الخلية وطابور المهام والصينية والمحركات، و`kawaro` عميل خفيف.
- **المحركات الثقيلة عمليات منفصلة** (FFmpeg وLibreOffice وGhostscript)، وتُقتل مع شجرتها كاملة عند الإلغاء.
- **صيغة جديدة تعني ملف TOML واحدا** في `actions/` ومحوّلا صغيرا ([مخطط](docs/diagrams/extend.ar.light.svg)).
- **تختار الخلية سطحها** عند كل عرض ([مخطط](docs/diagrams/surface.ar.light.svg))، و**التحديثات موقّعة** ([مخطط](docs/diagrams/updates.ar.light.svg)).
- المخططات تُولَّد بالأمر `python3 design/diagrams/build.py` من `design/tokens.json`. والعقود الكاملة في [`docs/architecture.md`](docs/architecture.md).

## التنزيل والتثبيت

نزّل آخر إصدار من [صفحة الإصدارات](https://github.com/M7MMAD-OMAR/kawaro/releases/latest) أو من [kawaro.sbarah.com](https://kawaro.sbarah.com/).

| النظام | الملف | الطريقة |
|---|---|---|
| ويندوز 10 و11 | `kawaro-<version>-windows-x64.msi` | انقر مرتين. يُثبَّت لحسابك دون صلاحيات المدير. |
| Fedora | `kawaro-<version>-1.x86_64.rpm` | `sudo dnf install ./kawaro-*.x86_64.rpm` |
| Ubuntu وDebian | `kawaro_<version>_amd64.deb` | `sudo apt install ./kawaro_*_amd64.deb` |

في كل إصدار ملف `SHA256SUMS` للتحقق من التنزيل. بعد التثبيت أعد فتح مدير الملفات وانقر بزر الفأرة الأيمن على أي ملف.

**تحذير ويندوز.** ملفات ويندوز موقّعة بشهادة اختبار ذاتية، فتحذّر SmartScreen عند أول تشغيل: اضغط «More info» ثم «Run anyway» ([التفاصيل](docs/windows-smartscreen.ar.md)). وعلى ويندوز 11 يظهر بند كوارو تحت «Show more options». التوقيع العلني قيد الترتيب عبر Microsoft Store وWinGet.

**التحديثات.** يتحقق كوارو من الإصدارات الجديدة ويحدّث نفسه: في الإعدادات، صفحة التحديثات تعرض الإصدار المثبّت و«التحقق من التحديثات» و«حدّث الآن». ومن سطر الأوامر: `kawaro update`.

**ملاحظات لينكس.** تضيف الحزمة كوارو إلى قائمة النقر اليمين في Dolphin وNautilus وNemo وتشغّل الخدمة مع كل جلسة. يحتاج Nautilus إلى `nautilus-python` (Fedora) أو `python3-nautilus` (Ubuntu)، ويحتاج الفتح من الحافظة إلى `wl-clipboard` على Wayland أو `xclip` على X11.

**من المصدر** (Rust 1.85 أو أحدث، لينكس):

```bash
git clone https://github.com/M7MMAD-OMAR/kawaro
cd kawaro
./scripts/install.sh      # لحسابك وحده دون root؛ للإزالة ./scripts/uninstall.sh
```

**المحركات.** FFmpeg وLibreOffice وGhostscript غير مضمّنة، فثبّت ما تحتاجه منها. وإن غاب محرك ذكرت رسالة الخطأ اسمه. وعلى ويندوز يبحث كوارو في `PATH` ثم في مجلدات التثبيت المعتادة.

## حالة الدعم

| النظام | الاختبار | ما يعمل |
|---|---|---|
| ويندوز 10 و11 | ثُبّت وشُغّل في آلة ويندوز 11 افتراضية | MSI، قائمة الزر الأيمن، أيقونة شريط المهام، خلية الإفلات |
| Hyprland 0.56 | جُرِّب | يعمل طبقةً عند المؤشر، مع الإفلات والضغط المطوّل |
| KDE Plasma 6 (KWin 6.7) | جُرِّب | يعمل طبقةً، مع التركيز والإفلات والاختصار |
| GNOME 50 | جُرِّب | يعمل نافذةً في المنتصف |
| Fedora 41 وما بعده (RPM)، وUbuntu 22.04 و24.04 وDebian 12 (DEB) | جُرِّب | تُثبَّت الحزم وتعمل |
| Sway وwlroots | لم يُجرَّب | يُتوقع أن يعمل طبقةً |
| X11 | غير مدعوم بعد | ضمن الخطة |
| ماك | غير مدعوم بعد | ضمن الخطة |

## ويندوز

انقر بزر الفأرة الأيمن على ملف في المستكشف واختر كوارو. وللتحويل أثناء السحب اسحب الملف بزر الفأرة الأيمن فتظهر الخلية عند الملف المسحوب. لا يُضغط أي مفتاح، فيبقى سلوك المستكشف مع السحب كما هو، والإفلات خارج الخلية ينسخ أو ينقل أو ينشئ اختصاراً كالعادة. والملف الأصلي لا يُمس. أوقف الخدمة من قائمة أيقونة شريط النظام («إنهاء»)، وأزل التثبيت من الإعدادات ثم التطبيقات. وأيقونة شريط النظام والاختصار العام وإشعار انتهاء المهمة ما زالت قيد الفحص على أجهزة حقيقية.

## سطر الأوامر

```bash
kawaro convert --to webp صورة.png        # تحويل مباشر دون الخدمة
kawaro actions *.jpg                     # ما تعرضه الخلية لهذه الملفات
kawaro open --from-clipboard             # فتح الخلية للملفات المنسوخة
```

على Hyprland يمكنك ربط Super مع النقر اليمين لتفتح الخلية للملفات المحددة مباشرة، والطريقة في [دليل Hyprland](integrations/linux/hyprland/README.ar.md).

ولتظهر الخلية وأنت تسحب ملفاً، اربط الأمر `kawaro drop` باختصار، والافتراضي Super+Z. خطوات GNOME وKDE وHyprland في [اختصار الإفلات](docs/drop-shortcut.ar.md).

## الإعدادات

افتح الإعدادات من أيقونة كوارو في شريط النظام. فيها صفحات للخيارات العامة (اللغة والمظهر) والتحويل (مكان الناتج والجودة) والإشعارات والتكامل والتحديثات، وصفحة «حول» فيها الإصدار والموقع. ويمكنك تعديل `config.toml` مباشرة: `~/.config/kawaro/config.toml` على لينكس، و`%APPDATA%\Kawaro\config.toml` على ويندوز. وإجراءاتك الخاصة في `actions/*.toml` بجانبه. وسجل خدمة ويندوز في `%LOCALAPPDATA%\Kawaro\logs\kawarod.log`.

المفتاح `hold_ms` مدة الضغط المطوّل بالملي ثانية قبل أن تظهر الخلية مع `kawaro drop --hold`: الافتراضي 400، والمقبول من 100 إلى 2000. يُقرأ عند كل ضغطة ولا تعرضه نافذة الإعدادات، فعدّله في الملف.

## المساهمة

اقرأ [دليل المساهمة](CONTRIBUTING.ar.md) و`docs/architecture.md` قبل أي تعديل. للإبلاغ عن ثغرة أمنية راجع `SECURITY.md`، وسجل الإصدارات في `CHANGELOG.md`. إضافة صيغة جديدة تعني ملف TOML في `actions/` ومحوّلاً صغيراً في `crates/engines/`.

## الدعم

كوارو مجاني ويُبنى بشكل مفتوح. إن أفادك، يمكنك دعم تطويره:

<a href="https://sbarah.com/support"><picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/support-sbarah-dark.svg"><img src="docs/assets/support-sbarah-light.svg" alt="ادعم صبارة" width="380"></picture></a>

## النشر والتوقيع

حزمة WinGet (`Sbarah.Kawaro`) مقدَّمة وبانتظار المراجعة. رفضت SignPath Foundation الطلب لأن المشروع يحتاج ظهوراً علنياً أكبر أولاً، فطريق اسم الناشر الموثوق هو Microsoft Store. التفاصيل في [حالة النشر](docs/distribution-status.md) و[سياسة توقيع الشيفرة](CODE_SIGNING_POLICY.ar.md).

## الرخصة

GPLv3 أو أحدث (GPL-3.0-or-later). راجع `LICENSE`.

</div>
