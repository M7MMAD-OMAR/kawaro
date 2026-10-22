# Drop shortcut | اختصار الإفلات

## العربية

اختصار الإفلات يُظهر الخلية وأنت تسحب ملفاً، فتفلته على خلية الإجراء الذي تريده. يشغّل الاختصار الأمر `sbarhive drop`، وتعمل الخلية بعدها دون لوحة المفاتيح، فيبقى السحب في يدك. الاختصار الافتراضي Super+Z: مفتاحان من يسار لوحة المفاتيح، وغير مستعمل في GNOME وKDE. لا يصلح مفتاح تعديل وحده (Shift أو Ctrl أو Alt) في GNOME وKDE، لأنه يغيّر نوع الإفلات في مدير الملفات أثناء السحب، وSuper وحده يفتح نظرة الأنشطة أو قائمة التطبيقات فيقطع السحب.

إن لم يكن المجلد `~/.local/bin` في مسار البحث (PATH) عند مديرك، فاكتب المسار الكامل للأمر `sbarhive` كما يطبعه المثبّت، بلا `~`.

### GNOME

في GNOME يُربط الاختصار يدوياً من الإعدادات:

1. افتح الإعدادات، ثم لوحة المفاتيح، ثم «عرض الاختصارات وتخصيصها»، ثم «اختصارات مخصصة».
2. أضف اختصاراً جديداً: الاسم «صبار هايف: إفلات»، والأمر:

   ```
   sbarhive drop
   ```

3. اضغط Super+Z لتسجيله، ثم اختر «إضافة».

تظهر الخلية في GNOME نافذةً في منتصف الشاشة، لا عند المؤشر. لم يُتحقق بعد من أن الاختصار المخصص في GNOME يعمل أثناء السحب: الاختبار المخفي سجّل المفتاح بالدالة `grab_accelerator('<Super>z')` داخل GNOME Shell عبر `Shell.Eval`، لا من الإعدادات.

### KDE

في KDE Plasma يُربط الاختصار من إعدادات النظام:

1. افتح إعدادات النظام، ثم لوحة المفاتيح، ثم الاختصارات.
2. اختر «إضافة جديد»، ثم «أمر أو سكربت»، واكتب الأمر:

   ```
   sbarhive drop
   ```

3. اضبط الاختصار على Super+Z (يسمّيه KDE مفتاح Meta)، ثم طبّق.

تظهر الخلية في KDE طبقةً عند الملف المسحوب.

### Hyprland

في Hyprland يُضاف سطر ربط إلى ملف الإعداد بنفسك. الأسطر أدناه مفترضة، لم تُجرَّب بعد على Hyprland حقيقي. يقرأ `--at` موضع المؤشر من `hyprctl cursorpos` لحظة الضغطة. ولا تظهر الخلية إلا أثناء سحب ملفات لها إجراء واحد على الأقل، فإن كان السحب جارياً ظهرت عند الملف المسحوب، وإلا فلا يظهر شيء.

ربط Super+Z يُظهر الخلية بالضغطة إن كنت تسحب ملفات لها إجراء، بصيغة hyprlang (الملف `hyprland.conf`):

```
bind = SUPER, Z, exec, ~/.local/bin/sbarhive drop --at "$(hyprctl cursorpos)"
```

أو إمساك Alt الأيسر: تظهر الخلية بعد ضغط مطوّل (المفتاح `hold_ms`، 400 ملي ثانية افتراضياً) وتختفي حين تترك المفتاح. ربط الضغط غير مستهلِك (العلم `n`) حتى تبقى تركيبات مثل Alt+Tab سليمة، ويتجاهل المُعدِّلات (العلم `i`) لأن Alt لا يكون ممسوكاً بعد في ضغطته الأولى، فربطٌ يشترط Alt لا يطابق إلا من الضغطة الثانية. وربط الترك يتجاهل المُعدِّلات كذلك (العلمان `r` و`i`). ولأن الضغط غير مستهلِك يصل Alt إلى مدير الملفات أيضاً، وقد يغيّر نوع الإفلات فيه، ولم يُتحقق من ذلك:

```
bindni = , ALT_L, exec, ~/.local/bin/sbarhive drop --hold --at "$(hyprctl cursorpos)"
bindri = , ALT_L, exec, ~/.local/bin/sbarhive drop --release
```

يحتاج ربط الترك خدمةً تعمل مسبقاً، فشغّلها مع الجلسة: `exec-once = ~/.local/bin/sbarhived`. صيغة Lua وإمساك Super+Z بدل Alt في `integrations/linux/hyprland/README.md`.

## English

The drop shortcut shows the hive while you drag a file, so you can drop it on the cell of the action you want. The shortcut runs `sbarhive drop`; the hive it shows takes no keyboard, so the drag stays in your hand. The default shortcut is Super+Z: two keys on the left of the keyboard, unused in GNOME and KDE. A modifier alone (Shift, Ctrl or Alt) does not work on GNOME and KDE, because during a drag it changes the drop kind in the file manager, and Super alone opens the Activities overview or the application launcher, which ends the drag.

If `~/.local/bin` is not on your PATH, use the full path of `sbarhive` as the installer prints it, without `~`.

### GNOME

On GNOME, bind the shortcut by hand in Settings:

1. Open Settings, then Keyboard, then View and Customize Shortcuts, then Custom Shortcuts.
2. Add a new shortcut: name it "SbarHive: drop", with the command:

   ```
   sbarhive drop
   ```

3. Press Super+Z to set it, then choose Add.

On GNOME the hive is a window in the middle of the screen, not at the pointer. That a GNOME custom shortcut fires during a drag is not verified yet: the headless test grabbed the key with `grab_accelerator('<Super>z')` inside GNOME Shell through `Shell.Eval`, not through Settings.

### KDE

On KDE Plasma, bind the shortcut in System Settings:

1. Open System Settings, then Keyboard, then Shortcuts.
2. Choose Add New, then Command or Script, and enter the command:

   ```
   sbarhive drop
   ```

3. Set the shortcut to Super+Z (KDE calls it Meta+Z), then apply.

On KDE the hive is a layer at the dragged file.

### Hyprland

On Hyprland, add a bind line to your config yourself. The lines below are assumed, not verified yet on a real Hyprland session. `--at` reads the pointer from `hyprctl cursorpos` at the press. The hive shows only during a drag of files that have an action: with such a drag it shows at the dragged file, otherwise nothing shows.

Super+Z shows the hive on the press while you drag files that have an action, hyprlang form (`hyprland.conf`):

```
bind = SUPER, Z, exec, ~/.local/bin/sbarhive drop --at "$(hyprctl cursorpos)"
```

Or hold left Alt: the hive shows after a long press (`hold_ms`, 400 ms by default) and closes when you let the key go. The press bind is non consuming (the flag `n`) so combinations such as Alt+Tab keep working, and ignores modifiers (the flag `i`) because Alt is not held yet on its own first press, so a bind that requires Alt matches only from the second press. The release bind ignores modifiers too (the flags `r` and `i`). Being non consuming, Alt still reaches the file manager and may change the drop kind there; not verified:

```
bindni = , ALT_L, exec, ~/.local/bin/sbarhive drop --hold --at "$(hyprctl cursorpos)"
bindri = , ALT_L, exec, ~/.local/bin/sbarhive drop --release
```

The release bind needs a running daemon, so start it with the session: `exec-once = ~/.local/bin/sbarhived`. The Lua form, and holding Super+Z instead of Alt, are in `integrations/linux/hyprland/README.md`.
