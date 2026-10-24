[English](settings.md) | [العربية](settings.ar.md)

# الإعدادات

الملف هو `config.toml`: في `~/.config/sbarhive/` على لينكس، وفي `%APPDATA%\SbarHive\` على ويندوز. كل مفتاح اختياري، والمفاتيح غير المعروفة تُهمل، وإذا تلف الملف استُعملت القيم الافتراضية.

| المفتاح | القيم | الافتراضي |
| --- | --- | --- |
| `language` | `auto` و`ar` و`en` | `auto` |
| `theme` | `auto` و`light` و`dark` | `auto` |
| `output` | `beside` و`subfolder` | `beside` |
| `output_folder` | اسم المجلد حين تكون القيمة `subfolder` | `SbarHive` |
| `quality` | `light` و`balanced` و`max` | `balanced` |
| `open_folder_when_done` | `true` و`false` | `false` |
| `notify` | `true` و`false` | `true` |
| `hive_surface` | `auto` و`layer` و`window` | `auto` |
| `hold_ms` | من 100 إلى 2000 (بالميلي ثانية) | `400` |
| `portal_shortcut` | `true` و`false` | `true` |
| `shortcut` | اختصار ويندوز | `Ctrl+Alt+H` |
| `update_check` | `true` و`false` | `true` |
| `update_channel` | `stable` و`beta` | `stable` (و`beta` في نسخة بيتا) |
| `update_mode` | `ask` و`auto` | `ask` |
