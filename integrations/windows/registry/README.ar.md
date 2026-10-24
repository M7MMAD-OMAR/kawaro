[English](README.md) | [العربية](README.ar.md)

<div dir="rtl">

# قائمة الزر الأيمن في مستكشف ويندوز

الأمر `sbarhive integration install` يضيف صبار هايف إلى قائمة الزر الأيمن للملفات في المستكشف، والأمر `sbarhive integration uninstall` يزيله. لا يحتاج أيٌّ منهما صلاحيات المدير: لا تُكتب إلا مفاتيح تحت `HKCU` عبر `reg.exe`.

شغّل `install` من المجلد الذي فككت فيه ملف zip. يتحقق الأمر من وجود `sbarhived.exe` و`sbarhive.ico` بجانب `sbarhive.exe`، ثم يسجّل هذا المجلد. أضف `--dry-run` لطباعة أوامر `reg.exe` دون تنفيذها.

## المفاتيح المكتوبة

| المفتاح | القيمة | البيانات |
| --- | --- | --- |
| `HKCU\Software\Classes\*\shell\SbarHive` | `MUIVerb` | `SbarHive` (`صبار هايف` عندما تكون اللغة عربية) |
| | `Icon` | `<المجلد>\sbarhive.ico` |
| | `MultiSelectModel` | `Player` (استدعاء واحد لكل الملفات المحددة) |
| `HKCU\Software\Classes\*\shell\SbarHive\command` | الافتراضية | `"<المجلد>\sbarhived.exe" open "%1"` |
| `HKCU\Software\Classes\AppUserModelId\io.github.m7mmad_omar.SbarHive` | `DisplayName` | `SbarHive` |
| | `IconUri` | `<المجلد>\sbarhive.ico` |

يحذف `uninstall` الشجرتين `HKCU\Software\Classes\*\shell\SbarHive` و`HKCU\Software\Classes\AppUserModelId\io.github.m7mmad_omar.SbarHive` ولا يحذف غيرهما.

## ويندوز 11

يظهر البند تحت «إظهار خيارات إضافية» (أو اضغط Shift+F10 على الملفات المحددة). القائمة الأولى في ويندوز 11 تحتاج حزمة موقّعة وليست جزءاً من هذه النسخة التجريبية.

## الحالة

أوامر `reg.exe` مختبرة على لينكس. فحص القائمة على نظام ويندوز حي مسجل في S-052.

</div>
