# SbarHive on Hyprland | صبار هايف على Hyprland

## العربية

يثبّت `scripts/install.sh` السكربت `~/.local/bin/sbarhive-open-selection` عندما يجد `hyprctl`، ولا يعدّل ملف `hyprland.conf`. أضف سطر ربط بنفسك.

يرسل السكربت Ctrl+C إلى النافذة النشطة، فينسخ مدير الملفات الملفات المحددة، ثم يشغّل `sbarhive open --from-clipboard` الذي يقرأها من الحافظة عبر `wl-paste`.

الربط المقترح من لوحة المفاتيح (آمن، لا يتعارض مع الإعداد الافتراضي):

```
bind = SUPER SHIFT, H, exec, ~/.local/bin/sbarhive-open-selection
```

الربط بزر الفأرة الأيمن مع Super:

```
bind = SUPER, mouse:273, exec, ~/.local/bin/sbarhive-open-selection
```

انتبه: الإعداد الافتراضي في Hyprland يربط `SUPER + mouse:273` بتغيير حجم النافذة (`bindm = SUPER, mouse:273, resizewindow`). إن أردت ربط الفأرة فاحذف ذلك السطر أو اختر زراً آخر، مثل `mouse:274` (الزر الأوسط).

ملاحظات:
- استخدم الربط في مدير الملفات فقط. في الطرفية يقطع Ctrl+C البرنامج الذي يعمل.
- يحتاج `wl-paste` من حزمة `wl-clipboard`.
- إن كان مدير الملفات بطيئاً في النسخ، زد المهلة: `SBARHIVE_COPY_DELAY=0.3`.
- Hyprland دون uwsm لا يشغّل `graphical-session.target` ولا ملفات autostart. الخدمة تبدأ عند أول طلب من `sbarhive open`، أو أضف: `exec-once = ~/.local/bin/sbarhived`.
- إن ثبّتّ بمسار PREFIX مختلف فاستبدل `~/.local/bin` به. المثبّت يطبع السطر بالمسار الصحيح.

### إلغاء حركة ظهور الخلية

على Hyprland تظهر الخلية طبقةً (layer-shell) باسم `sbarhive`، فيطبّق عليها Hyprland حركة الطبقات. لإلغائها أضف قاعدة طبقة. القاعدة لم يُتحقق منها بعد على Hyprland حقيقي.

بصيغة Lua (Hyprland 0.55 وما بعده، الملف `hyprland.lua`):

```
hl.layer_rule({ match = { namespace = "^sbarhive$" }, no_anim = true })
```

بصيغة hyprlang (Hyprland 0.53 و0.54، الملف `hyprland.conf`)، لم يُتحقق منها:

```
layerrule = no_anim on, match:namespace ^sbarhive$
```

## English

`scripts/install.sh` installs `~/.local/bin/sbarhive-open-selection` when it finds `hyprctl`. It never edits `hyprland.conf`; add a bind yourself.

The helper sends Ctrl+C to the active window, so the file manager copies the selected files, then runs `sbarhive open --from-clipboard`, which reads them from the clipboard with `wl-paste`.

Suggested keyboard bind (safe, no clash with the defaults):

```
bind = SUPER SHIFT, H, exec, ~/.local/bin/sbarhive-open-selection
```

Mouse bind, Super plus right button:

```
bind = SUPER, mouse:273, exec, ~/.local/bin/sbarhive-open-selection
```

Careful: the default Hyprland config binds `SUPER + mouse:273` to window resizing (`bindm = SUPER, mouse:273, resizewindow`). To use the mouse bind, remove that line or pick another button such as `mouse:274` (middle).

Notes:
- Use it in file managers only. In a terminal Ctrl+C interrupts the running program.
- Needs `wl-paste` from `wl-clipboard`.
- If your file manager is slow to copy, raise the delay: `SBARHIVE_COPY_DELAY=0.3`.
- Hyprland without uwsm starts neither `graphical-session.target` nor autostart files. The daemon starts on the first `sbarhive open`, or add `exec-once = ~/.local/bin/sbarhived`.
- With a different PREFIX, replace `~/.local/bin`. The installer prints the line with the right path.

### Turning off the hive's animation

On Hyprland the hive is a layer-shell surface with the namespace `sbarhive`, so Hyprland animates it like any layer. A layer rule turns that off. The rule is not verified yet on a real Hyprland session.

Lua form (Hyprland 0.55 and later, `hyprland.lua`):

```
hl.layer_rule({ match = { namespace = "^sbarhive$" }, no_anim = true })
```

hyprlang form (Hyprland 0.53 and 0.54, `hyprland.conf`), not verified:

```
layerrule = no_anim on, match:namespace ^sbarhive$
```
