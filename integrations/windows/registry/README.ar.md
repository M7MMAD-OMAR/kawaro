[English](README.md) | [العربية](README.ar.md)

<div dir="rtl">

# قائمة الزر الأيمن في مستكشف ويندوز

الأمر `kawaro integration install` يضيف كوارو إلى قائمة الزر الأيمن للملفات في المستكشف، والأمر `kawaro integration uninstall` يزيله. لا يحتاج أيٌّ منهما صلاحيات المدير: لا تُكتب إلا مفاتيح تحت `HKCU` عبر `reg.exe`.

شغّل `install` من المجلد الذي فككت فيه ملف zip. يتحقق الأمر من وجود `kawarod.exe` و`kawaro.ico` بجانب `kawaro.exe`، ثم يسجّل هذا المجلد. أضف `--dry-run` لطباعة أوامر `reg.exe` دون تنفيذها.

## المفاتيح المكتوبة

| المفتاح | القيمة | البيانات |
| --- | --- | --- |
| `HKCU\Software\Classes\*\shell\Kawaro` | `MUIVerb` | `Kawaro` (`كوارو` عندما تكون اللغة عربية) |
| | `Icon` | `<المجلد>\kawaro.ico` |
| | `MultiSelectModel` | `Player` (استدعاء واحد لكل الملفات المحددة) |
| `HKCU\Software\Classes\*\shell\Kawaro\command` | الافتراضية | `"<المجلد>\kawarod.exe" open "%1"` |
| `HKCU\Software\Classes\AppUserModelId\io.github.m7mmad_omar.Kawaro` | `DisplayName` | `Kawaro` |
| | `IconUri` | `<المجلد>\kawaro.ico` |

يحذف `uninstall` الشجرتين `HKCU\Software\Classes\*\shell\Kawaro` و`HKCU\Software\Classes\AppUserModelId\io.github.m7mmad_omar.Kawaro` ولا يحذف غيرهما.

## ويندوز 11

يظهر البند تحت «إظهار خيارات إضافية» (أو اضغط Shift+F10 على الملفات المحددة). القائمة الأولى في ويندوز 11 تحتاج حزمة موقّعة وليست جزءاً من هذه النسخة التجريبية.

## الحالة

أوامر `reg.exe` مختبرة على لينكس. فحص القائمة على نظام ويندوز حي مسجل في S-052.

</div>
