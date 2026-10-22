<div dir="rtl">

# صبار هايف

حدّد ملفاتك في مدير الملفات، وافتح صبار هايف من قائمة النقر اليمين، واختر من الخلية ما تريد. يُحفظ الناتج بجانب الملف الأصلي، ولا يُمس الأصل.

يعمل كل شيء على جهازك: لا رفع ولا خوادم ولا جمع بيانات.

![الخلية فوق سطح مكتب GNOME لصورة واحدة](docs/screenshots/hive-gnome.png)

## ماذا يفعل

- **الصور:** تحويل بين PNG وJPEG وWebP وAVIF وGIF وBMP وTIFF وICO، وتغيير الحجم، والتدوير، والقلب، والتدرج الرمادي، والضغط، وحذف البيانات الوصفية، وجمع عدة صور في ملف PDF واحد.
- **الفيديو والصوت** (الإصدار 0.2، يحتاج FFmpeg): تحويل إلى MP4 وWebM وGIF، واستخراج الصوت، وتحويل الصوت بين MP3 وOGG وFLAC وWAV وM4A وOpus، وضغط الفيديو.
- **المستندات وPDF** (الإصدار 0.2، يحتاج LibreOffice وGhostscript): تحويل مستندات Office وODF إلى PDF، والتحويل بين DOCX وODT، وضغط PDF ودمجه وتدويره وتحويل صفحاته إلى صور.

تعرض الخلية ما يناسب الملفات المحددة فقط: نوعها وعددها، والأدوات المثبتة على جهازك.

## حالة الدعم

بصراحة، ما الذي جُرِّب وكيف:

| النظام | الحالة | كيف تحققنا |
|---|---|---|
| Hyprland 0.56 | يعمل: طبقة فوق النوافذ، عند المؤشر، الإفلات والضغط المطوّل | اختبارات آلية في Hyprland خاص متداخل؛ والاستعمال اليومي عند المطوّر قيد التجربة |
| KDE Plasma 6 (KWin 6.7) | يعمل: طبقة، والتركيز في كل مستويات منع سرقته، والإفلات والاختصار | اختبارات آلية في KWin خاص بلا شاشة، بشاشة وبشاشتين |
| GNOME 50 | يعمل نافذةً في المنتصف | اختبارات آلية في gnome-shell خاص بلا شاشة؛ لم يُجرَّب التركيز على GNOME حقيقي بعد |
| Sway وwlroots | متوقع أن يعمل كطبقة | لم يُجرَّب |
| X11 | غير مدعوم بعد | لم يُجرَّب |
| التوزيعات | حزم RPM لـ Fedora 41 وما بعده، وDEB لـ Ubuntu 22.04 و24.04 وDebian 12 | تثبيت وتشغيل آلي في حاويات نظيفة لكل منها |
| ويندوز وماك | غير متوفر بعد | ضمن الخطة |

## التثبيت

يعمل الإصدار الحالي على لينكس (64 بت).

**الأسهل:** نزّل الحزمة المناسبة من [صفحة آخر إصدار](https://github.com/M7MMAD-OMAR/sbarhive/releases/latest) وافتحها بمثبّت الحزم، أو من الطرفية:

```bash
sudo dnf install ./sbarhive-*.x86_64.rpm     # Fedora
sudo apt install ./sbarhive_*_amd64.deb      # Ubuntu وDebian
```

تضيف الحزمة صبار هايف إلى قائمة النقر اليمين في Dolphin وNautilus وNemo، وتشغّل الخدمة مع كل جلسة. بعد التثبيت أعد فتح مدير الملفات. لـ Nautilus ثبّت `nautilus-python` (Fedora) أو `python3-nautilus` (Ubuntu). ولـ Thunar استعمل سكربت التثبيت أدناه.

**من المصدر:** تحتاج Rust 1.85 أو أحدث.

```bash
git clone https://github.com/M7MMAD-OMAR/sbarhive
cd sbarhive
./scripts/install.sh
```

يثبّت السكربت البرنامج لحسابك وحده دون صلاحيات المدير، ويضيف عنصر صبار هايف إلى قائمة النقر اليمين في مديري الملفات الموجودين عندك (Dolphin وNautilus وNemo وThunar)، ويشغّل الخدمة مع الجلسة. للإزالة: `./scripts/uninstall.sh`.

يحتاج Nautilus إلى حزمة `nautilus-python`، ويحتاج الفتح من الحافظة إلى `wl-clipboard` على Wayland أو `xclip` على X11.

## الاستعمال من سطر الأوامر

```bash
sbarhive convert --to webp صورة.png        # تحويل مباشر دون الخدمة
sbarhive actions *.jpg                     # ما تعرضه الخلية لهذه الملفات
sbarhive open --from-clipboard             # فتح الخلية للملفات المنسوخة
```

على Hyprland يمكن ربط Super مع النقر اليمين بفتح الخلية للملفات المحددة مباشرة. الطريقة في `integrations/linux/hyprland/README.md`.

لإظهار الخلية وأنت تسحب ملفاً، اربط الأمر `sbarhive drop` باختصار، والافتراضي Super+Z. الطريقة في GNOME وKDE وHyprland في `docs/drop-shortcut.md`.

## الإعدادات

افتح الإعدادات من أيقونة صبار هايف في شريط النظام، أو عدّل `~/.config/sbarhive/config.toml` مباشرة: اللغة، والمظهر، ومكان الناتج (بجانب الأصل أو في مجلد فرعي)، ومستوى الجودة. ويمكن إضافة إجراءات خاصة بك بملفات TOML في `~/.config/sbarhive/actions/`.

على ويندوز يكون ملف الإعدادات في `%APPDATA%\SbarHive\config.toml`، والإجراءات الخاصة في `%APPDATA%\SbarHive\actions\`. أما سجل الخدمة في `%LOCALAPPDATA%\SbarHive\logs\` فمخطَّط لقصة لاحقة (S-027) ولا يُكتب بعد.

وعلى ويندوز يشغّل `sbarhive open` الخدمة `sbarhived.exe` من مجلد البرنامج إن لم تكن تعمل، والمقصود أن تبقى تعمل بعد إغلاق الطرفية. نسخة ويندوز من الخدمة لم تُبنَ بعد (S-027)، ولم يُتحقق من هذا السلوك حياً (S-034).

ولا ينزّل صبار هايف المحركات الخارجية على ويندوز، بل يبحث عنها في `PATH` أولاً، ثم في مجلدات تثبيتها المعتادة: LibreOffice في `%ProgramFiles%\LibreOffice\program`، وGhostscript في أحدث `%ProgramFiles%\gs\gs*\bin`، وFFmpeg في المجلد `ffmpeg\bin` بجانب `sbarhive.exe`، ثم في سجل ويندوز لـ LibreOffice وGhostscript. وإن لم يجد محركاً ذكرت رسالة الخطأ اسمه ورابط تنزيله.

المفتاح `hold_ms` مدة الضغط المطوّل بالملي ثانية قبل أن تظهر الخلية مع `sbarhive drop --hold`: الافتراضي 400، والمقبول من 100 إلى 2000، وأي قيمة غير ذلك تعيد الافتراضي مع تحذير في السجل. يُقرأ عند كل ضغطة، فيسري تعديله دون إعادة تشغيل الخدمة. نافذة الإعدادات لا تعرضه، فعدّله في الملف.

## المساهمة

اقرأ `CONTRIBUTING.md` و`docs/architecture.md` قبل أي تعديل. للإبلاغ عن ثغرة أمنية راجع `SECURITY.md`، وسجل التغييرات في `CHANGELOG.md`. إضافة صيغة جديدة تعني ملف TOML في `actions/` ومحوّلا صغيرا في `crates/engines/`.

## الرخصة

GPLv3 أو أحدث. راجع `LICENSE`.

</div>

---

# SbarHive

Select files in your file manager, open SbarHive from the right-click menu,
and pick an action from the hive. The result is saved next to the original,
which is never modified.

Everything runs on your machine: no uploads, no servers, no telemetry.

![The hive over a GNOME desktop for one image](docs/screenshots/hive-gnome.png)

## What it does

- **Images:** convert between PNG, JPEG, WebP, AVIF, GIF, BMP, TIFF and ICO;
  resize, rotate, flip, grayscale, compress, strip metadata, and combine
  several images into one PDF.
- **Video and audio** (v0.2, needs FFmpeg): convert to MP4, WebM and GIF,
  extract the audio track, convert audio between MP3, OGG, FLAC, WAV, M4A and
  Opus, compress video.
- **Documents and PDF** (v0.2, needs LibreOffice and Ghostscript): Office and
  ODF documents to PDF, DOCX and ODT both ways, compress, merge and rotate
  PDFs, turn pages into images.

The hive only offers what fits the selection: file kind, file count, and the
tools installed on your system.

## Platform support

Plainly, what has been tested and how:

| Platform | Status | How it was checked |
|---|---|---|
| Hyprland 0.56 | Works: overlay layer, at the pointer, drop and long press | Automated tests in a private nested Hyprland; daily use on the developer's machine in progress |
| KDE Plasma 6 (KWin 6.7) | Works: layer, focus at every focus stealing prevention level, drop and shortcut | Automated tests in a private headless KWin, one and two outputs |
| GNOME 50 | Works as a centred window | Automated tests in a private headless gnome-shell; focus not yet tried on a real GNOME session |
| Sway and wlroots | Expected to work as a layer | Not tested |
| X11 | Not supported yet | Not tested |
| Distributions | RPM for Fedora 41 and later, DEB for Ubuntu 22.04 and 24.04 and Debian 12 | Automated install and run in a clean container for each |
| Windows and macOS | Not available yet | Planned |

## Install

Linux (64 bit) for now.

**Easiest:** download the package for your system from the
[latest release](https://github.com/M7MMAD-OMAR/sbarhive/releases/latest)
and open it with your software installer, or:

```bash
sudo dnf install ./sbarhive-*.x86_64.rpm     # Fedora
sudo apt install ./sbarhive_*_amd64.deb      # Ubuntu, Debian
```

The package adds SbarHive to the right-click menu of Dolphin, Nautilus and
Nemo and starts the service with every session; reopen your file manager
after installing. Nautilus needs `nautilus-python` (Fedora) or
`python3-nautilus` (Ubuntu). For Thunar use the script below.

**From source:** requires Rust 1.85 or newer.

```bash
git clone https://github.com/M7MMAD-OMAR/sbarhive
cd sbarhive
./scripts/install.sh
```

The script installs for your user only, without root. It adds a SbarHive
entry to the right-click menu of the file managers it finds (Dolphin,
Nautilus, Nemo, Thunar) and starts the service with your session. To remove
it: `./scripts/uninstall.sh`.

Nautilus needs `nautilus-python`. Opening from the clipboard needs
`wl-clipboard` on Wayland or `xclip` on X11.

## Command line

```bash
sbarhive convert --to webp photo.png    # convert here, no service needed
sbarhive actions *.jpg                  # what the hive offers for these files
sbarhive open --from-clipboard          # open the hive for copied files
```

On Hyprland you can bind Super plus right-click to open the hive for the
selected files directly; see `integrations/linux/hyprland/README.md`.

To show the hive while you drag a file, bind `sbarhive drop` to a
shortcut, Super+Z by default; GNOME, KDE and Hyprland steps are in
`docs/drop-shortcut.md`.

## Settings

Open Settings from the SbarHive tray icon, or edit `~/.config/sbarhive/config.toml`, which holds the language, theme, output place
(next to the original or in a subfolder) and quality preset. Your own actions
go in `~/.config/sbarhive/actions/*.toml`.

On Windows the settings file is `%APPDATA%\SbarHive\config.toml`, your own
actions go in `%APPDATA%\SbarHive\actions\*.toml`. A service log in
`%LOCALAPPDATA%\SbarHive\logs\` is planned for S-027 and not written yet.

On Windows `sbarhive open` starts the service, `sbarhived.exe`, from the
program folder when it is not running, and it is meant to keep running after
the terminal closes. The Windows service itself is not built yet (S-027), and
this behaviour is not verified live (S-034).

SbarHive does not download the external engines on Windows. It looks for
them in `PATH` first, then in their usual install folders: LibreOffice in
`%ProgramFiles%\LibreOffice\program`, Ghostscript in the newest
`%ProgramFiles%\gs\gs*\bin`, FFmpeg in the `ffmpeg\bin` folder next to
`sbarhive.exe`, then in the Windows registry for LibreOffice and
Ghostscript. When an engine is missing, the error names it and links to its
download page.

`hold_ms` is the long press, in milliseconds, before `sbarhive drop --hold`
shows the hive: 400 by default, accepted from 100 to 2000; any other value
falls back to the default with a warning in the log. It is read at every
press, so a change applies without restarting the daemon. The settings
window does not show it; edit the file.

## Contributing

Read `CONTRIBUTING.md` and `docs/architecture.md` first. Security reports:
see `SECURITY.md`. Release history: `CHANGELOG.md`. A new format is one TOML
file in `actions/` plus a small adapter in `crates/engines/`.

## License

GPL-3.0-or-later. See `LICENSE`.
