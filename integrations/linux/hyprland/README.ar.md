[English](README.md) | [العربية](README.ar.md)

<div dir="rtl">

# كوارو على Hyprland

يثبّت `scripts/install.sh` السكربت `~/.local/bin/kawaro-open-selection` إن وجد `hyprctl`، ولا يعدّل ملف `hyprland.conf` أبداً، فأضف سطر الربط بنفسك.

يرسل السكربت Ctrl+C إلى النافذة النشطة فينسخ مدير الملفات الملفات المحددة، ثم يشغّل `kawaro open --from-clipboard` الذي يقرؤها من الحافظة عبر `wl-paste`.

يقرأ السكربت موضع المؤشر مرة واحدة بالأمر `hyprctl cursorpos` ثم يشغّل `kawaro open --from-clipboard --at X,Y`، فتظهر الخلية حيث المؤشر لا في وسط الشاشة. وإن فشل `hyprctl cursorpos` أو طبع ما ليس رقمين فيتوقف السكربت بسطر واحد على stderr ولا يفتح شيئاً.

الربط المقترح من لوحة المفاتيح (آمن، ولا يتعارض مع الإعداد الافتراضي):

```
bind = SUPER SHIFT, H, exec, ~/.local/bin/kawaro-open-selection
```

الربط بزر الفأرة الأيمن مع Super:

```
bind = SUPER, mouse:273, exec, ~/.local/bin/kawaro-open-selection
```

انتبه: الإعداد الافتراضي في Hyprland يربط `SUPER + mouse:273` بتغيير حجم النافذة (`bindm = SUPER, mouse:273, resizewindow`). إن أردت ربط الفأرة فاحذف ذلك السطر أو اختر زراً آخر، مثل `mouse:274` (الزر الأوسط).

ملاحظات:
- استعمل الربط في مدير الملفات فقط، فالضغط على Ctrl+C في الطرفية يوقف البرنامج الذي يعمل فيها.
- يحتاج `wl-paste` من حزمة `wl-clipboard`.
- إن كان مدير الملفات بطيئاً في النسخ فزد المهلة: `KAWARO_COPY_DELAY=0.3`.
- Hyprland دون uwsm لا يشغّل `graphical-session.target` ولا ملفات autostart. تبدأ الخدمة عند أول طلب من `kawaro open`، أو أضف: `exec-once = ~/.local/bin/kawarod`.
- إن ثبّتّ بمسار PREFIX مختلف فضعه مكان `~/.local/bin`. المثبّت يطبع السطر بالمسار الصحيح.

## إلغاء حركة ظهور الخلية

على Hyprland تظهر الخلية طبقةً (layer-shell) باسم `kawaro`، فيحرّكها Hyprland كما يحرّك أي طبقة. تُلغى الحركة بقاعدة طبقة، ولم نتحقق من القاعدة بعد على جلسة Hyprland حقيقية.

بصيغة Lua (Hyprland 0.55 وما بعده، الملف `hyprland.lua`):

```
hl.layer_rule({ match = { namespace = "^kawaro$" }, no_anim = true })
```

بصيغة hyprlang (Hyprland 0.53 و0.54، الملف `hyprland.conf`)، ولم يُتحقق منها:

```
layerrule = no_anim on, match:namespace ^kawaro$
```

## إمساك المفتاح للإفلات

لا تظهر خلية الإفلات إلا أثناء سحب ملفات لها إجراء واحد على الأقل. ابدأ السحب أولاً، ثم أمسك المفتاح: بعد ضغط مطوّل (400 ملي ثانية افتراضياً، والمفتاح `hold_ms` في `config.toml`) تظهر الخلية تحت الملف المسحوب، وتبقى ما دمت ممسكاً بالمفتاح، وتختفي حين تتركه. إن أمسكت المفتاح دون سحب، أو سحبت ملفاً ليس له إجراء، فلا يظهر شيء. والضغطة العابرة لا تُظهر شيئاً كذلك.

بعد انقضاء العتبة تنتظر الخدمة أول حدث من المؤلِّف لتعرف هل يجري سحب، وHyprland لا يرسله إلا مع أول حركة، فإن كانت يدك ساكنة ظهرت الخلية حين تحرّك الملف قليلاً. وإن بدأت السحب بعد انقضاء العتبة والمفتاح ممسوك فلن تظهر الخلية، فاترك المفتاح وأعد المحاولة بالترتيب الصحيح.

يلزم لذلك ربطان: ربط الضغط يشغّل `kawaro drop --hold`، وربط الترك يشغّل `kawaro drop --release`. المؤقّت في الخدمة لا في الربط، فيعود الأمران فوراً.

ربط الترك لا يشغّل الخدمة، والخدمة التي يشغّلها ربط الضغط قد يفوتها ترك سريع، لذلك شغّلها مع الجلسة: `exec-once = ~/.local/bin/kawarod`.

بصيغة hyprlang (الملف `hyprland.conf`). هذه الأسطر مفترضة، ولم يجرّبها أحد بعد على جلسة Hyprland حقيقية:

```
bindn = SUPER, Z, exec, ~/.local/bin/kawaro drop --hold --at "$(hyprctl cursorpos)"
bindrni = SUPER, Z, exec, ~/.local/bin/kawaro drop --release
```

بصيغة Lua (الملف `hyprland.lua`)، وهي مفترضة كذلك:

```
hl.bind("SUPER + Z", hl.dsp.exec_cmd("$HOME/.local/bin/kawaro drop --hold --at \"$(hyprctl cursorpos)\""),
    { non_consuming = true, description = "Kawaro: hold to drop" })
hl.bind("SUPER + Z", hl.dsp.exec_cmd("$HOME/.local/bin/kawaro drop --release"),
    { release = true, non_consuming = true, ignore_mods = true, description = "Kawaro: release to close" })
```

لإمساك Alt الأيسر بدل Super+Z (مفترض كذلك)، بصيغة hyprlang:

```
bindni = , ALT_L, exec, ~/.local/bin/kawaro drop --hold --at "$(hyprctl cursorpos)"
bindrni = , ALT_L, exec, ~/.local/bin/kawaro drop --release
```

وبصيغة Lua:

```
hl.bind("ALT_L", hl.dsp.exec_cmd("$HOME/.local/bin/kawaro drop --hold --at \"$(hyprctl cursorpos)\""),
    { non_consuming = true, ignore_mods = true, description = "Kawaro: hold Alt to drop" })
hl.bind("ALT_L", hl.dsp.exec_cmd("$HOME/.local/bin/kawaro drop --release"),
    { release = true, non_consuming = true, ignore_mods = true, description = "Kawaro: release Alt to close" })
```

### Alt مع مفتاح آخر (Alt+a وAlt+Tab)

لا يُطلق Hyprland ربط الترك إذا جاء مفتاح آخر بين الضغط والترك، فبعد Alt+a تنتظر الخدمة تركاً لن يأتي، وتُظهر الخلية بعد 400 ملي ثانية دون أن يكون Alt ممسوكاً. في إعداد Lua أضف خطّاف مفاتيح بجوار ربطَي Alt. الخطّاف يراقب المفاتيح فقط ولا يستهلك شيئاً، فتصل كل تركيبة إلى برنامجها بحكم التصميم. وقد قِيس هذا مع Alt+a فقط؛ ولم يُقَس Alt+Tab ولا Alt+F4 في بيئة الاختبار (إذ يأخذ المُركِّب الخارجي الخاص Alt+Tab). وما دام Alt مضغوطاً يشغّل أي مفتاح آخر الأمر `kawaro drop --other-key`، فتنتهي الضغطة المعلّقة (تسجّل الخدمة `hold: chord`) ولا يظهر شيء. و`sleep` القصير يبقيه خلف ربط الضغط الذي يشغّل `hyprctl` أولاً؛ وتتذكر الخدمة المفتاح الآخر 150 ملي ثانية، فالمفتاح الذي يسبق الضغطة رغم ذلك يلغيها. الرقم `64` هو `Alt_L` (الرمز 56 في evdev زائد 8). قِيس على Hyprland 0.57.0 في الجلسة الخاصة لـ `tests/hyprland/repro.sh --altdrag`: مع Alt+a لا يظهر سطح خلية ويُسجَّل `hold: chord`، وقبل الإصلاح كانت خمسة أسطح.

```
local alt_down = false
hl.on("input.keyboard.key", function(code, _, state)
  if code == 64 then -- Alt_L, the evdev code 56 plus 8
    alt_down = state == 1
  elseif alt_down and state == 1 then
    hl.exec_cmd("sleep 0.05; $HOME/.local/bin/kawaro drop --other-key")
  end
end)
```

لا مقابل لهذا في hyprlang: مفتاحه `catchall` يعمل داخل submap فقط، والـ submap يعطّل بقية ربطك ما دام Alt مضغوطاً. ومع hyprlang تعالج الحدَّ الزمني (`hold_max_ms`) هذه الحالة. والأمر `kawaro drop --other-key` لا يشغّل الخدمة، ولا ينهي إلا ضغطة لم تظهر خليتها بعد؛ والخلية الظاهرة تبقى حتى الترك.

كل ربط ترك أعلاه غير مستهلِك (العلم `n` مع `r` و`i` في hyprlang، و`non_consuming` في Lua)، ويجب أن يبقى كذلك. يبتلع Hyprland ضغطة المفتاح الذي له ربط ترك مستهلِك، ولأن ربط الترك يتجاهل المُعدِّلات فهو يطابق المفتاح وحده: ربط `bindri = SUPER, Z` المستهلِك يبتلع كل حرف Z يُكتب في أي برنامج، والربط على `ALT_L` يبتلع كل ضغطة Alt. وهذا متحقق منه في مصدر Hyprland 0.56.2 (يبتلع `KeybindManager.cpp` ضغطة ربط الترك ما لم يكن غير مستهلِك) وفي جلسة حقيقية توقف فيها حرف Z عن الكتابة. وترك المفتاح دون خلية ظاهرة لا يفعل شيئاً، فلا كلفة لهذا الاستدعاء مع Z العادي.

ملاحظات، ولم نتحقق من أي منها:
- ربط الترك لا يصل إن تركت المُعدِّل قبل المفتاح الأساسي (Super قبل Z مثلاً)، فيجب أن يتجاهل المُعدِّلات: العلم `i` مع `r` في hyprlang، و`ignore_mods` في Lua.
- ربط الضغط على Alt وحده، أو على أي مُعدِّل، يجب أن يكون غير مستهلِك (العلم `n` في hyprlang، و`non_consuming` في Lua) حتى تبقى تركيبات مثل Alt+Tab سليمة. ويجب أن يتجاهل المُعدِّلات كذلك (العلم `i` في hyprlang، و`ignore_mods` في Lua) وألا يشترط المُعدِّل نفسه، لأن Alt لا يكون ممسوكاً بعد في الضغطة الأولى، فربط مثل `ALT, ALT_L` لا يطابق إلا من الضغطة الثانية.
- يُقرأ الموضع `--at` عند الضغطة الأولى، فتظهر الخلية حيث كان المؤشر لحظة الضغط.
- `kawaro drop` دون `--hold` لا مهلة له، وهو لربط ضغطة عادية، وتسري عليه القاعدة نفسها: لا تظهر الخلية إلا مع سحب ملفات لها إجراء.

## اختصار البوابة

يبقى ربط `exec` السابق هو الطريقة الأولى. والبديل أن يوصل xdg-desktop-portal-hyprland اختصار Super+Z الذي تسجّله الخدمة، بسطر `global` واحد بدلاً من ربط الضغط وربط الترك. ويعمل بالإمساك كذلك (`hold_ms`). بصيغة hyprlang (الملف `hyprland.conf`)، ولم يُتحقق منها:

```
bind = SUPER, Z, global, io.github.m7mmad_omar.Kawaro:drop
```

وبصيغة Lua (الملف `hyprland.lua`):

```
hl.bind("SUPER + Z", hl.dsp.global("io.github.m7mmad_omar.Kawaro:drop"))
```

اختر إحدى الطريقتين فقط لـ Super+Z.

</div>
