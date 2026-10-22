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

### إمساك المفتاح للإفلات

لا تظهر خلية الإفلات إلا أثناء سحب ملفات لها إجراء واحد على الأقل. ابدأ السحب أولاً، ثم أمسك المفتاح: بعد ضغط مطوّل (400 ملي ثانية افتراضياً، المفتاح `hold_ms` في `config.toml`) تظهر الخلية تحت الملف المسحوب، وتبقى ما دمت ممسكاً بالمفتاح، وتختفي حين تتركه. إن أمسكت المفتاح دون سحب، أو سحبت ملفاً ليس له إجراء، فلا يظهر شيء. والضغطة العابرة لا تُظهر شيئاً كذلك.

بعد انقضاء العتبة تنتظر الخدمة أول حدث من المؤلِّف لتعرف أيجري سحب أم لا، وHyprland لا يرسله إلا مع أول حركة، فإن كانت يدك ساكنة تظهر الخلية حين تحرّك الملف قليلاً. وإن بدأت السحب بعد انقضاء العتبة والمفتاح ممسوك فلن تظهر الخلية، فاترك المفتاح وأعد بالترتيب الصحيح.

يحتاج ذلك ربطين: ربط الضغط يشغّل `sbarhive drop --hold`، وربط الترك يشغّل `sbarhive drop --release`. المؤقّت في الخدمة لا في السكربت، فيعود الأمران فوراً.

الخدمة لا تبدأ مع ربط الترك، وإن بدأت مع ربط الضغط فقد يضيع ترك سريع. لذلك شغّلها مع الجلسة: `exec-once = ~/.local/bin/sbarhived`.

بصيغة hyprlang (الملف `hyprland.conf`). هذه الأسطر مفترضة، لم يجرّبها أحد بعد على Hyprland حقيقي:

```
bindn = SUPER, Z, exec, ~/.local/bin/sbarhive drop --hold --at "$(hyprctl cursorpos)"
bindri = SUPER, Z, exec, ~/.local/bin/sbarhive drop --release
```

بصيغة Lua (الملف `hyprland.lua`)، مفترضة كذلك:

```
hl.bind("SUPER + Z", hl.dsp.exec_cmd("$HOME/.local/bin/sbarhive drop --hold --at \"$(hyprctl cursorpos)\""),
    { non_consuming = true, description = "SbarHive: hold to drop" })
hl.bind("SUPER + Z", hl.dsp.exec_cmd("$HOME/.local/bin/sbarhive drop --release"),
    { release = true, ignore_mods = true, description = "SbarHive: release to close" })
```

لإمساك Alt الأيسر بدل Super+Z (مفترض كذلك)، بصيغة hyprlang:

```
bindni = , ALT_L, exec, ~/.local/bin/sbarhive drop --hold --at "$(hyprctl cursorpos)"
bindri = , ALT_L, exec, ~/.local/bin/sbarhive drop --release
```

وبصيغة Lua:

```
hl.bind("ALT_L", hl.dsp.exec_cmd("$HOME/.local/bin/sbarhive drop --hold --at \"$(hyprctl cursorpos)\""),
    { non_consuming = true, ignore_mods = true, description = "SbarHive: hold Alt to drop" })
hl.bind("ALT_L", hl.dsp.exec_cmd("$HOME/.local/bin/sbarhive drop --release"),
    { release = true, ignore_mods = true, description = "SbarHive: release Alt to close" })
```

ملاحظات، كلها غير متحقق منها:
- ربط الترك لا يصل إن تركت المُعدِّل قبل المفتاح الأساسي (تترك Super قبل Z مثلاً)، لذلك يحتاج تجاهل المُعدِّلات: العلم `i` مع `r` في hyprlang، و`ignore_mods` في Lua.
- ربط الضغط على Alt وحده أو أي مُعدِّل يجب أن يكون غير مستهلِك (العلم `n` في hyprlang، و`non_consuming` في Lua)، حتى تبقى تركيبات مثل Alt+Tab سليمة. ويجب أن يتجاهل المُعدِّلات أيضاً (العلم `i` في hyprlang، و`ignore_mods` في Lua) وألا يشترط المُعدِّل نفسه: في الضغطة الأولى لا يكون Alt ممسوكاً بعد، فربطٌ مثل `ALT, ALT_L` لا يطابق إلا من الضغطة الثانية.
- الموضع `--at` يُقرأ عند الضغطة الأولى، فتظهر الخلية حيث كان المؤشر لحظة الضغط.
- `sbarhive drop` دون `--hold` يبقى دون مهلة، لربط ضغطة عادية، وتسري عليه القاعدة نفسها: لا تظهر الخلية إلا مع سحب ملفات لها إجراء.

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

### Hold a key to drop

The drop hive shows only during a drag of files that have at least one action. Start the drag first, then hold the key: after a long press (400 ms by default, `hold_ms` in `config.toml`) the hive shows under the dragged file, stays while you hold the key and closes when you let it go. Without a drag, or for a file with no action, nothing shows. A short tap shows nothing either.

Once the threshold ends, the daemon waits for the compositor's first event to learn whether a drag is under way, and Hyprland sends it only with the first move, so with a still hand the hive shows as soon as you move the file a little. If you start the drag only after the threshold has ended, the hive does not show; let go of the key and try again in the right order.

That takes two binds: the press runs `sbarhive drop --hold` and the release runs `sbarhive drop --release`. The timer is in the daemon, not in the bind, so both commands return at once.

The release bind never starts the daemon, and a daemon started by the press may miss a quick release, so start it with the session: `exec-once = ~/.local/bin/sbarhived`.

hyprlang form (`hyprland.conf`). These lines are assumed, not verified yet on a real Hyprland session:

```
bindn = SUPER, Z, exec, ~/.local/bin/sbarhive drop --hold --at "$(hyprctl cursorpos)"
bindri = SUPER, Z, exec, ~/.local/bin/sbarhive drop --release
```

Lua form (`hyprland.lua`), assumed as well:

```
hl.bind("SUPER + Z", hl.dsp.exec_cmd("$HOME/.local/bin/sbarhive drop --hold --at \"$(hyprctl cursorpos)\""),
    { non_consuming = true, description = "SbarHive: hold to drop" })
hl.bind("SUPER + Z", hl.dsp.exec_cmd("$HOME/.local/bin/sbarhive drop --release"),
    { release = true, ignore_mods = true, description = "SbarHive: release to close" })
```

To hold left Alt instead of Super+Z (assumed as well), hyprlang form:

```
bindni = , ALT_L, exec, ~/.local/bin/sbarhive drop --hold --at "$(hyprctl cursorpos)"
bindri = , ALT_L, exec, ~/.local/bin/sbarhive drop --release
```

Lua form:

```
hl.bind("ALT_L", hl.dsp.exec_cmd("$HOME/.local/bin/sbarhive drop --hold --at \"$(hyprctl cursorpos)\""),
    { non_consuming = true, ignore_mods = true, description = "SbarHive: hold Alt to drop" })
hl.bind("ALT_L", hl.dsp.exec_cmd("$HOME/.local/bin/sbarhive drop --release"),
    { release = true, ignore_mods = true, description = "SbarHive: release Alt to close" })
```

Notes, none of them verified:
- The release bind does not fire when the modifier is let go before the main key (Super before Z, say), so it needs to ignore modifiers: the flag `i` with `r` in hyprlang, `ignore_mods` in Lua.
- A press bind on Alt alone, or any modifier, must be non consuming (the flag `n` in hyprlang, `non_consuming` in Lua), so combinations such as Alt+Tab keep working. It must also ignore modifiers (the flag `i` in hyprlang, `ignore_mods` in Lua) and not require the modifier itself: on the first press Alt is not held yet, so a bind such as `ALT, ALT_L` matches only from the second press.
- `--at` is read at the first press, so the hive shows where the pointer was when the key went down.
- `sbarhive drop` without `--hold` has no threshold, for a plain key press bind, and follows the same rule: the hive shows only with a drag of files that have an action.
