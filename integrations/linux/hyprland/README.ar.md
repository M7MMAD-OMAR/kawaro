[English](README.md) | [العربية](README.ar.md)

<div dir="rtl">

# صبار هايف على Hyprland

يثبّت `scripts/install.sh` السكربت `~/.local/bin/sbarhive-open-selection` إن وجد `hyprctl`، ولا يعدّل ملف `hyprland.conf` أبداً، فأضف سطر الربط بنفسك.

يرسل السكربت Ctrl+C إلى النافذة النشطة فينسخ مدير الملفات الملفات المحددة، ثم يشغّل `sbarhive open --from-clipboard` الذي يقرؤها من الحافظة عبر `wl-paste`.

الربط المقترح من لوحة المفاتيح (آمن، ولا يتعارض مع الإعداد الافتراضي):

```
bind = SUPER SHIFT, H, exec, ~/.local/bin/sbarhive-open-selection
```

الربط بزر الفأرة الأيمن مع Super:

```
bind = SUPER, mouse:273, exec, ~/.local/bin/sbarhive-open-selection
```

انتبه: الإعداد الافتراضي في Hyprland يربط `SUPER + mouse:273` بتغيير حجم النافذة (`bindm = SUPER, mouse:273, resizewindow`). إن أردت ربط الفأرة فاحذف ذلك السطر أو اختر زراً آخر، مثل `mouse:274` (الزر الأوسط).

ملاحظات:
- استعمل الربط في مدير الملفات فقط، فالضغط على Ctrl+C في الطرفية يوقف البرنامج الذي يعمل فيها.
- يحتاج `wl-paste` من حزمة `wl-clipboard`.
- إن كان مدير الملفات بطيئاً في النسخ فزد المهلة: `SBARHIVE_COPY_DELAY=0.3`.
- Hyprland دون uwsm لا يشغّل `graphical-session.target` ولا ملفات autostart. تبدأ الخدمة عند أول طلب من `sbarhive open`، أو أضف: `exec-once = ~/.local/bin/sbarhived`.
- إن ثبّتّ بمسار PREFIX مختلف فضعه مكان `~/.local/bin`. المثبّت يطبع السطر بالمسار الصحيح.

## إلغاء حركة ظهور الخلية

على Hyprland تظهر الخلية طبقةً (layer-shell) باسم `sbarhive`، فيحرّكها Hyprland كما يحرّك أي طبقة. تُلغى الحركة بقاعدة طبقة، ولم نتحقق من القاعدة بعد على جلسة Hyprland حقيقية.

بصيغة Lua (Hyprland 0.55 وما بعده، الملف `hyprland.lua`):

```
hl.layer_rule({ match = { namespace = "^sbarhive$" }, no_anim = true })
```

بصيغة hyprlang (Hyprland 0.53 و0.54، الملف `hyprland.conf`)، ولم يُتحقق منها:

```
layerrule = no_anim on, match:namespace ^sbarhive$
```

## إمساك المفتاح للإفلات

لا تظهر خلية الإفلات إلا أثناء سحب ملفات لها إجراء واحد على الأقل. ابدأ السحب أولاً، ثم أمسك المفتاح: بعد ضغط مطوّل (400 ملي ثانية افتراضياً، والمفتاح `hold_ms` في `config.toml`) تظهر الخلية تحت الملف المسحوب، وتبقى ما دمت ممسكاً بالمفتاح، وتختفي حين تتركه. إن أمسكت المفتاح دون سحب، أو سحبت ملفاً ليس له إجراء، فلا يظهر شيء. والضغطة العابرة لا تُظهر شيئاً كذلك.

بعد انقضاء العتبة تنتظر الخدمة أول حدث من المؤلِّف لتعرف هل يجري سحب، وHyprland لا يرسله إلا مع أول حركة، فإن كانت يدك ساكنة ظهرت الخلية حين تحرّك الملف قليلاً. وإن بدأت السحب بعد انقضاء العتبة والمفتاح ممسوك فلن تظهر الخلية، فاترك المفتاح وأعد المحاولة بالترتيب الصحيح.

يلزم لذلك ربطان: ربط الضغط يشغّل `sbarhive drop --hold`، وربط الترك يشغّل `sbarhive drop --release`. المؤقّت في الخدمة لا في الربط، فيعود الأمران فوراً.

ربط الترك لا يشغّل الخدمة، والخدمة التي يشغّلها ربط الضغط قد يفوتها ترك سريع، لذلك شغّلها مع الجلسة: `exec-once = ~/.local/bin/sbarhived`.

بصيغة hyprlang (الملف `hyprland.conf`). هذه الأسطر مفترضة، ولم يجرّبها أحد بعد على جلسة Hyprland حقيقية:

```
bindn = SUPER, Z, exec, ~/.local/bin/sbarhive drop --hold --at "$(hyprctl cursorpos)"
bindri = SUPER, Z, exec, ~/.local/bin/sbarhive drop --release
```

بصيغة Lua (الملف `hyprland.lua`)، وهي مفترضة كذلك:

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

ملاحظات، ولم نتحقق من أي منها:
- ربط الترك لا يصل إن تركت المُعدِّل قبل المفتاح الأساسي (Super قبل Z مثلاً)، فيجب أن يتجاهل المُعدِّلات: العلم `i` مع `r` في hyprlang، و`ignore_mods` في Lua.
- ربط الضغط على Alt وحده، أو على أي مُعدِّل، يجب أن يكون غير مستهلِك (العلم `n` في hyprlang، و`non_consuming` في Lua) حتى تبقى تركيبات مثل Alt+Tab سليمة. ويجب أن يتجاهل المُعدِّلات كذلك (العلم `i` في hyprlang، و`ignore_mods` في Lua) وألا يشترط المُعدِّل نفسه، لأن Alt لا يكون ممسوكاً بعد في الضغطة الأولى، فربط مثل `ALT, ALT_L` لا يطابق إلا من الضغطة الثانية.
- يُقرأ الموضع `--at` عند الضغطة الأولى، فتظهر الخلية حيث كان المؤشر لحظة الضغط.
- `sbarhive drop` دون `--hold` لا مهلة له، وهو لربط ضغطة عادية، وتسري عليه القاعدة نفسها: لا تظهر الخلية إلا مع سحب ملفات لها إجراء.

</div>
