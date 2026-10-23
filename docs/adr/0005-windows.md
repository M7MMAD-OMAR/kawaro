# ADR 0005: SbarHive على ويندوز (الإصدار 0.6.0)

الحالة: مقترح
التاريخ: 2026-10-22

## السياق

هدف 0.6.0 في `docs/PLAN.md`: مستخدم على ويندوز 10 أو 11 يحدد ملفاً في المستكشف، ويختار SbarHive من قائمة النقر اليمين، فتظهر الخلية عند المؤشر ويُحفظ الناتج بجانب الأصل، بالعربية والإنكليزية، دون أن يغادر أي ملف الجهاز. يحسم هذا الملف أدوات البناء، وIPC، وتشغيل الخدمة، والمسارات، والشريط والإشعارات والتركيز والإفلات والاختصار، وتكامل المستكشف، والمحركات، والمثبّت، والاختبار، وتقسيم القصص.

لم يُشغَّل أي شيء على ويندوز بعد. آلة `win11` (libvirt بجلسة المستخدم، EFI، 6 GB، شبكة user-mode، قنوات spice، بلا وكيل ضيف ولا لقطات) قد يكون ويندوز فيها غير مكتمل التثبيت، ولم تُشغَّل لهذا الملف. لذلك: (مُتحقَّق) تعني سطراً مقروءاً في الشيفرة مع الإصدار والمسار، أو أمراً شُغّل على جهاز التطوير، و(مُفترَض) تعني سلوكاً على ويندوز لم يُختبر، ويُتحقق منه في القائمة الحية.

**نقطة البداية (مُتحقَّق، 2026-10-22):** على Fedora مع `mingw64-gcc` 16.2.1 وrust-std لـ `x86_64-pc-windows-gnu`، دون rustup ودون MSVC، يفشل `cargo check --workspace --target x86_64-pc-windows-gnu` في موضعين فقط: `crates/cli/src/client.rs` (`std::os::unix::net`، و`exec`، و`process_group`) و`crates/ui/src/lib.rs:242-246`، لأن `winit::platform::startup_notify` مُعرَّف لـ X11 وWayland فقط (winit 0.30.13، `src/platform/mod.rs:13`). لم يصل الفحص إلى الخدمة لأنها تعتمد على ui. وفيها أيضاً `UnixListener` و`UnixStream` (`main.rs`، و`server.rs`، و`state.rs:32`)، وksni وهو Linux فقط، و`xdg-open` في `notify.rs`.

## النتائج والقرارات

### 1. أدوات البناء

- **القرار:** بناء متقاطع بـ `x86_64-pc-windows-gnu` من لينكس هو المسار الأساسي، محلياً وفي CI على ubuntu. وفي CI وظيفة ثانية على `windows-latest` تشغّل `cargo test` لـ core وengines وcli أصلياً بـ MSVC، فتُختبر الشيفرة على ويندوز حقيقي دون الآلة. الإصدار المنشور هو بناء gnu نفسه الذي يبنيه المطوّر.
- **السبب:** جهاز التطوير بلا MSVC ولا rustup، ونقطة البداية تثبت أن الاعتماديات كلها تُترجَم لـ gnu. المُصيِّر هو software (ADR 0001)، وعلى ويندوز يرسم softbuffer بـ GDI، فلا نحتاج skia ولا ربط C++ ثقيلاً. سؤال توفر ملفات skia المبنية مسبقاً لـ gnu لا يعنينا (مُفترَض أنها لـ MSVC فقط، ولم نتحقق).
- **المرفوض:** MSVC للإصدار، لأن كل بناء يحتاج مشغّل ويندوز ولا يُعاد على جهاز المطوّر. و`cargo-xwin`، لأنه ينزّل CRT وSDK من مايكروسوفت بقبول رخصتهما. و`x86_64-pc-windows-gnullvm`، لأن rust-std له غير مثبت، ولا ميزة ظاهرة له هنا.
- **الربط:** يُتحقق في أول قصة بناء (لا `cargo check` فقط) من أن الملفات التنفيذية لا تستورد إلا مكتبات النظام: `x86_64-w64-mingw32-objdump -p sbarhived.exe | grep 'DLL Name'`، أي لا `libgcc_s_seh-1.dll` ولا `libwinpthread-1.dll`. وإن ظهرتا يُضاف `-static` لرابط gnu. وlibwebp-sys تبنيها mingw gcc نفسها (مُفترَض أنها تُربط ساكنة).
- **الأنظمة:** `sbarhived.exe` بنظام `windows_subsystem = "windows"`، فلا تظهر نافذة طرفية عند تشغيله من المستكشف، وسجلّه في `%LOCALAPPDATA%\SbarHive\logs\sbarhived.log` بدل stderr. أما `sbarhive.exe` فيبقى تطبيق طرفية لسطر الأوامر. فحوص تعدّ أسطر `notify:` في stderr تقرأ ملف السجل على ويندوز.
- **بوابة إضافية لقصص ويندوز:** `cargo clippy --target x86_64-pc-windows-gnu --all-targets -- -D warnings` داخل نطاق systemd المحدود (`MemoryMax=6G`، `-j 2`).

### 2. IPC على named pipe

- **القرار:** الرسائل كما هي: سطر JSON لكل `Request` و`Response` من `core::ipc`، والأنبوب بنمط البايتات (`PIPE_TYPE_BYTE | PIPE_READMODE_BYTE`)، فيعمل `BufRead::read_line` كما على Unix. الحقل `token` يبقى `None` على ويندوز، فلا تغيير في المخطط.
- **الاسم:** `\\.\pipe\io.github.m7mmad_omar.SbarHive.<SID>.<session>`، من SID المستخدم ورقم الجلسة (`ProcessIdToSessionId`)، لأن مساحة أسماء الأنابيب مشتركة بين كل المستخدمين والجلسات. تبنيه دالة صافية في core تُختبر على لينكس، كما `socket_path()` اليوم.
- **الأمان:**
  - DACL صريح يمنح المستخدم الحالي وحده الوصول الكامل. الواصف الافتراضي يمنح Everyone والحساب المجهول القراءة (صفحة أمان الأنابيب)، فلا نقبله.
  - `PIPE_REJECT_REMOTE_CLIENTS` يرفض العملاء عبر الشبكة.
  - يتحقق العميل من أن مالك عملية الخادم (`GetNamedPipeServerProcessId` ثم رمز العملية) هو المستخدم نفسه، فلا تخدعه عملية سبقت إلى الاسم. هذا يقابل مجلد `$XDG_RUNTIME_DIR` بصلاحيات 0700 على لينكس.
- **نسخة واحدة:** أول `CreateNamedPipeW` بـ `FILE_FLAG_FIRST_PIPE_INSTANCE`. فشله بـ `ERROR_ACCESS_DENIED` يعني أن خدمة تعمل، فيحل محل فحص `already_running` بالـ ping.
- **المهلات:** الأنابيب بلا `set_read_timeout`. الاتصال بـ `WaitNamedPipeW` بمهلة، والقراءة ذات المهلة (500 ms في فحص الخدمة، و10 s في العميل) بخيط مراقب يستدعي `CancelIoEx` عند انقضائها. `try_clone` للمشتركين (`state.rs`) بـ `DuplicateHandle` عبر `File::try_clone`. المقبض متزامن فتتسلسل عملياته، وهذا لا يضر: الخادم بعد قراءة الطلب يكتب فقط، والعميل بعد الإرسال يقرأ فقط (مُفترَض).
- **الشكل في الشيفرة:** وحدة نقل في core (`Listener` و`Stream` بالواجهة نفسها على المنصتين)، Unix فوق `UnixListener` كما اليوم، وويندوز في ملف مستقل `transport/windows.rs` على windows-sys، يُضاف إلى `exclude_globs` في `.cargo/mutants.toml` مثل `non_unix.rs`. الخدمة وسطر الأوامر يستخدمانها بدل `std::os::unix` مباشرة.
- **المرفوض:** `interprocess` 2.4.6 (0BSD OR Apache-2.0). فيه `security_descriptor` و`server_process_id` و`FILE_FLAG_FIRST_PIPE_INSTANCE` (مُتحقَّق من الشيفرة، `src/os/windows/named_pipe/listener/create_instance.rs`)، لكنه يضيف ثلاث حزم جديدة (`doctest-file` و`recvmsg` و`widestring`)، وواصف الأمان نبنيه نحن في الحالتين. الشيفرة المباشرة نحو 200 سطر.

### 3. تشغيل الخدمة وإيقاف شجرة العمليات

- **من سطر الأوامر:** `ensure_daemon` يشغّل `sbarhived.exe` بـ `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_BREAKAWAY_FROM_JOB` عبر `CommandExt::creation_flags`. إن رُفض الخروج من مهمة الطرفية (`ERROR_ACCESS_DENIED`) يعاد التشغيل دون العلم الأخير، وإلا قتلت مهمةٌ بـ kill-on-close الخدمةَ مع سطر الأوامر. `exec_daemon` يصير تشغيلاً وانتظاراً ثم الخروج برمز الخدمة.
- **من المستكشف:** الأمر في السجل يستدعي `sbarhived.exe open "%1"` مباشرة، لا `sbarhive.exe`. إن كانت خدمة تعمل يصير عميلاً: يمنحها حق الواجهة (القسم 5) ويرسل `Open` ويخرج. وإلا صار هو الخدمة وأظهر الخلية بنفسه، فلا كلفة تشغيل عملية ثالثة.
- **محركات بعمليات فرعية (`office/non_unix.rs`):** كائن مهمة لكل مهمة تحويل، بـ `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. لتفادي سباق الأبناء قبل الإسناد: التشغيل بـ `CREATE_SUSPENDED`، ثم `AssignProcessToJobObject` بمقبض `Child`، ثم استئناف خيطه الوحيد (`CreateToolhelp32Snapshot` و`OpenThread` و`ResumeThread`)، لأن `Child` في std لا يعطي مقبض الخيط، و`raw_attribute` (`PROC_THREAD_ATTRIBUTE_JOB_LIST`) غير مستقر. الإلغاء والمهلة بـ `TerminateJobObject`، وإغلاق المقبض يقتل ما تبقى حتى لو ماتت الخدمة. هذا مهم لأن `soffice.exe` يشغّل `soffice.bin` ابناً (مُفترَض).

### 4. المسارات والكتابة الذرية

- الإعدادات والإجراءات: `%APPDATA%\SbarHive\config.toml` و`%APPDATA%\SbarHive\actions\`. السجل: `%LOCALAPPDATA%\SbarHive\logs\`. تُقرأ المتغيرات، ثم `SHGetKnownFolderPath` إن غابت. تنتقل `user_actions_dir` من `daemon/state.rs` إلى core بجانب `Config::path`، ويصير الاختيار دالة صافية تأخذ البيئة، فتُختبر على لينكس.
- المؤقت: `std::env::temp_dir()` يعطي `%TEMP%` (`GetTempPath2W`)، ولا تغيير في `tempfile`. والملف المؤقت للناتج يبقى بجانب الناتج (`output_impl::temp_path`)، فهو على القرص نفسه دائماً.
- **لا شيفرة إعادة تسمية جديدة.** `std::fs::rename` يستبدل الهدف إن وُجد، وعلى ويندوز يستخدم `MoveFileExW` مع رجوع إلى `SetFileInformationByHandle`، وعلى ويندوز 10 1607 وما بعده يطابق سلوك Unix حيث يدعم نظام الملفات `FileRenameInfoEx` (توثيق std)، فحفظ `config.toml` ذري كما هو. و`finalize` يبدأ بـ `hard_link` (`CreateHardLinkW`) الذي يفشل إن وُجد الهدف، ويعمل على NTFS. وعلى FAT وexFAT (أقراص USB) يرجع إلى الفحص ثم `rename`. تحسين صغير لويندوز وحده: يصير هذا الرجوع `MoveFileExW` دون `REPLACE_EXISTING`، فيفشل إن وُجد الهدف، ويزول السباق الموثق في `output_impl.rs:116`.

### 5. الشريط والإشعارات والتركيز والإفلات والاختصار

- **خيط الصدفة:** خيط واحد في الخدمة بحلقة رسائل Win32 ونافذة مخفية من المستوى الأعلى (لا `HWND_MESSAGE`، لأن نوافذ الرسائل لا تصلها رسالة `TaskbarCreated` المذاعة عند إعادة تشغيل المستكشف). يستضيف أيقونة الشريط والاختصار. COM وWinRT لا يُهيآن على الخيط الرئيسي قبل winit، لأن winit يذعر عند `RPC_E_CHANGED_MODE` من `OleInitialize` (مُتحقَّق من الشيفرة، winit 0.30.13، `platform_impl/windows/window.rs:1168-1175`).
- **الشريط:** `Shell_NotifyIconW` مباشرة على windows-sys: تلميح بالتقدم (128 حرفاً حداً)، وقائمة `TrackPopupMenu` بالبنود الثلاثة نفسها، مع `MFT_RIGHTORDER` للعربية. الأيقونة ملف `.ico` بجانب البرنامج (`LoadImageW` مع `LR_LOADFROMFILE`)، فلا حاجة لمترجم موارد الآن. ksni يصير اعتماداً لـ `cfg(target_os = "linux")` فقط، ووحدتا `tray` تتشاركان الواجهة نفسها (`start` و`set_active` و`set_lang`).
  - **المرفوض:** `tray-icon` 0.26.1 (MIT OR Apache-2.0)، لأنه يجرّ `muda ^0.21` وSlint يجرّ `muda` 0.19.3، فتدخل الشجرة نسختان، ولأنه يحتاج حلقة رسائل على خيطه على أي حال.
- **الإشعارات:** notify-rust 4.18.2 الموجود، وعلى ويندوز يستخدم `tauri-winrt-notification` 0.8.1 (MIT OR Apache-2.0، موجود في `Cargo.lock`). دون `app_id` يرسل باسم PowerShell (مُتحقَّق من الشيفرة، `notify-rust-4.18.2/src/windows.rs:76-79`). لذلك `.app_id("io.github.m7mmad_omar.SbarHive")`، ويسجّل المثبّت هذا المعرّف (AUMID) باختصار في قائمة ابدأ وبالمفتاح `HKCU\Software\Classes\AppUserModelId\<id>` مع `DisplayName` و`IconUri`. «فتح المجلد» يصير `explorer.exe /select,<file>` بدل `xdg-open`، و`wait_for_action` موجود على ويندوز (`windows.rs:111`).
- **الخلية نافذة winit على ويندوز** (القاعدة في CLAUDE.md وADR 0004): بلا إطار، شفافة، فوق الكل، دون زر في شريط المهام (`with_skip_taskbar`). الموضع عند `GetCursorPos`، ومنطقة العمل للشاشة تحت المؤشر (`MonitorFromPoint` و`GetMonitorInfoW`، أي `rcWork`) تُمرَّر إلى `layout::plan_at` نفسها، فيتكيّف الشكل مع الحواف كما على مسار الطبقة.
  - **قواعد التصميم:** لا ظل: `with_undecorated_shadow(false)`، وهو الافتراضي أصلاً (مُتحقَّق، winit 0.30.13، `platform_impl/windows/mod.rs:52`)، ويُكتب صراحة. لا تدوير لزوايا ويندوز 11: `DWMWA_WINDOW_CORNER_PREFERENCE = DWMWCP_DONOTROUND`. ولا ضبابية من المؤلِّف هنا، فتبقى الأسطح معتمة ولا تُستعمل رموز الزجاج، كما على GNOME. خلفيات DWM الشفافة (Acrylic) قرار لاحق.
  - **الخطر الأول: الشفافية.** softbuffer 0.4.8 يعرض بـ `Gdi::BitBlt` (مُتحقَّق، `vendor/softbuffer/src/backends/win32.rs:189`)، وwinit يجعل النافذة شفافة بـ `DwmEnableBlurBehindWindow` بمنطقة فارغة (`window.rs:1231-1242`). هل تبقى قناة ألفا عبر `BitBlt`؟ (مُفترَض، غير معروف). إن لم تبقَ ظهرت الزوايا سوداء كما حدث على Wayland. أول فحص في الآلة، والبديلان بالترتيب: تعديل في نسختنا من softbuffer يعرض بـ `UpdateLayeredWindow` لنافذة `WS_EX_LAYERED` بألفا مضروبة مسبقاً، وهي ما يكتبه مُصيِّر Slint، على نمط تعديل Wayland الموثق في `SBARHIVE-PATCH.md`؛ ثم منطقة نافذة من اتحاد السداسيات (`SetWindowRgn`) بحواف غير ناعمة، وهي تحصر النقر في الشكل مجاناً.
- **التركيز:** ويندوز يسمح بـ `SetForegroundWindow` لعملية شغّلتها العملية الأمامية، أو تلقّت آخر حدث إدخال، أو سمحت لها عملية أمامية بـ `AllowSetForegroundWindow`، ويسقط الإذن عند أول إدخال للمستخدم. `sbarhived.exe` العميل يشغّله المستكشف وهو العملية الأمامية، فيستدعي `AllowSetForegroundWindow(pid الخدمة)` (من `GetNamedPipeServerProcessId`) قبل إرسال `Open`. ثم تستدعي الخدمة `SetForegroundWindow` عند الظهور. هذا ما يقابل رمز xdg-activation، ولذلك يُستبدل `token_hook` في `lib.rs` على ويندوز ويُحصر الأصلي في X11 وWayland. `sbarhive.exe` من الطرفية يفعل الشيء نفسه. والإغلاق على مسار النافذة كما هو: Esc، أو الاختيار، أو فقدان التركيز.
- **الإفلات (ترتيب «الخلية أولاً ثم السحب»، ADR 0003):** winit 0.30.13 يسجّل `IDropTarget` بـ OLE، ويرد بـ `DROPEFFECT_COPY` فقط، فلا يحذف المستكشف الأصل، وهذا يحقق قاعدة النسخ فقط. ويرسل `HoveredFile` عند الدخول و`DroppedFile` عند الإفلات. لكنه يتجاهل موضع `_pt` ولا يرسل شيئاً في `DragOver` (مُتحقَّق، `platform_impl/windows/drop_handler.rs:79-150`)، وSlint 1.18.1 لا يوصل الحدثين. فلا يكفي وحده لاختيار الخلية.
  - **القرار:** نلتقط الحدثين في `FocusWatcher::window_event` الموجود، ونقرأ `GetCursorPos` بمؤقت كل 16 ms بين `HoveredFile` و`DroppedFile` أو `HoveredFileCancelled`، فيُبرز `Layout::pick` الخلية ويختارها، كما يفعل مسار X11 بـ `QueryPointer`.
  - **المرفوض الآن:** `IDropTarget` خاص بنا (`with_drag_and_drop(false)` ثم `RegisterDragDrop` بحزمة `windows` 0.62.2 الموجودة في الشجرة). يبقى بديلاً إن فشل الاستطلاع.
  - **قاعدة S-021** (لا تظهر خلية الإفلات إلا لسحب حقيقي): على ويندوز تظهر بالاختصار فقط وزر الفأرة الأيسر مضغوط (`GetAsyncKeyState(VK_LBUTTON)`). الملفات لا تُعرف إلا عند `HoveredFile`، كما على Wayland.
- **الاختصار العام:** `RegisterHotKey` على نافذة خيط الصدفة. يصل الضغط وحده، فالترك (`drop --hold` و`--release`، S-019e) باستطلاع `GetAsyncKeyState` بعد `WM_HOTKEY`، وهي طريقة `global-hotkey` 0.9.0 نفسها (مُتحقَّق، `src/platform_impl/windows/mod.rs:153-163`). نكتبها مباشرة ولا نضيف الحزمة (Apache-2.0 OR MIT)، لأنها تفتح نافذتها وخيطها وتجرّ `windows-sys` 0.59. على ويندوز تقرأ الخدمة موضع المؤشر بنفسها، فلا حاجة لـ `--at`. وهل يصل `WM_HOTKEY` أثناء سحب OLE من المستكشف؟ (مُفترَض، فحص في الآلة).

### 6. تكامل المستكشف

- **القرار لـ 0.6.0:** فعل قديم في السجل تحت `HKCU\Software\Classes\*\shell\SbarHive` (و`Directory\shell` لاحقاً إن احتجناه)، فيه `MUIVerb` و`Icon` و`MultiSelectModel = Player`، والأمر `"<dir>\sbarhived.exe" open "%1"`. لا صلاحيات مدير ولا توقيع. يكتبه المثبّت، ويكتبه `sbarhive integration install` ويحذفه `uninstall` لمن يستخدم الملف المضغوط. نص المفاتيح دالة صافية في core تُختبر على لينكس.
- **التحديد المتعدد:** الفعل القديم بأمر يُستدعى مرة لكل ملف (مُفترَض من التوثيق، يُتحقق في الآلة)، وحدّه 100 عنصر لـ Player و15 لـ Document. لذلك تدمج الخدمة طلبات `Open` المتتالية: الأول يُظهر الخلية فوراً، وما يصل خلال 300 ms من آخر طلب يُضاف إلى التحديد ويُعاد بناء الخلايا للنوع المشترك، فتبقى ميزانية 150 ms للظهور الأول. وعند تشغيل بارد يشغّل المستكشف عدة نسخ من `sbarhived.exe` معاً: تفوز واحدة بـ `FILE_FLAG_FIRST_PIPE_INSTANCE`، فتنشئ الأنبوب قبل `ui::build`، والبقية تصير عملاء تنتظر بـ `WaitNamedPipeW` ضمن المهلة نفسها، وتقع طلباتها في الدمج نفسه (مُفترَض). وإن ظهر وميض أو بطء في الآلة، فالبديل فعل `DropTarget`: خادم COM خارج العملية (`LocalServer32` تحت `HKCU\Software\Classes\CLSID`، بلا مدير ولا توقيع) يستلم كل العناصر في `IDataObject` واحد بلا حد. يُكتب في قصة مستقلة إن لزم.
- **ويندوز 11:** الأفعال القديمة تظهر تحت «عرض مزيد من الخيارات» (أو Shift+F10)، لا في القائمة الأولى. القائمة الحديثة تحتاج `IExplorerCommand` في DLL داخل المستكشف، وحزمة sparse تمنح هوية، وكلاهما يحتاج توقيعاً بشهادة. **خارج نطاق 0.6.0** حتى يوفر المالك شهادة، ومكانها `integrations/windows/{explorer-command,sparse-package}/` كما في CLAUDE.md. هذا يخالف حرفية هدف PLAN على ويندوز 11، فيحتاج قبول المالك.

### 7. المحركات

- محرك الصور كما هو.
- **الاكتشاف:** `PATH` أولاً، ثم المسارات المعروفة: `%ProgramFiles%\LibreOffice\program\`، و`%ProgramFiles%\gs\gs*\bin\` (الأحدث)، ومجلد `ffmpeg\bin` بجانب البرنامج إن وضعه المستخدم هناك. ثم السجل: `HKLM\SOFTWARE\LibreOffice\UNO\InstallPath`، و`HKLM\SOFTWARE\GPL Ghostscript\<ver>` (مُفترَض، يُتحقق في الآلة).
- **أسماء ويندوز:** `soffice.com` ثم `soffice.exe` (الأول غلاف طرفية ينتظر الانتهاء، مُفترَض)، و`gswin64c.exe` (موجود في `pdf.rs:272`)، و`ffmpeg.exe` و`ffprobe.exe`. `is_executable` في `non_unix.rs` يقبل `.exe` و`.com` فقط.
- **إصلاح لازم:** `office::file_url` يرمّز `:` و`\`، فيعطي لـ `C:\a b\x` رابطاً خاطئاً. على ويندوز يجب أن يعطي `file:///C:/a%20b/x`. دالة صافية تُختبر على لينكس بمسارات ويندوز نصية. وبادئة `file:` في ffmpeg مع حرف القرص (`file:C:\...`) بند في القائمة الحية.
- **لا تنزيل تلقائي في 0.6.0.** رسالة «المحرك غير موجود» تذكر اسم البرنامج ورابط موقعه الرسمي. التنزيل عند الحاجة يُقرَّر لاحقاً، لأنه يمس التحقق من التوقيع والتحديث.

### 8. المثبّت

- **القرار:** MSI لكل مستخدم (`InstallScope="perUser"`، في `%LOCALAPPDATA%\Programs\SbarHive`)، يُبنى بـ `wixl` من msitools 0.106 على لينكس، محلياً وفي CI على ubuntu. يكتب مفاتيح الفعل، ومفتاح AUMID، واختصار قائمة ابدأ بالمعرّف، وتشغيل الخدمة عند الدخول (`HKCU\...\Run`)، ويحذفها كلها عند الإزالة. ومعه ملف zip محمول وأمر `sbarhive integration install`.
- **السبب:** لا صلاحيات مدير، والإزالة نظيفة من «التطبيقات»، والبناء على المضيف نفسه دون ويندوز. msitools أداة بناء فقط ولا تُشحن، ورخصتها (LGPL-2.1-or-later وGPL-2.0-or-later وMS-RL) لا تمس البرنامج.
- **المرفوض:** WiX 4 وما بعده (.NET على ويندوز، أي CI فقط). Inno Setup (ويندوز فقط، أو wine). وNSIS (`mingw32-nsis` 3.11، Zlib) يبقى بديلاً إن عجز `wixl` عن التثبيت لكل مستخدم أو عن الاختصار بمعرّف، لأن `wixl` يدعم جزءاً من مخطط WiX 3 فقط (مُفترَض، أول ما يُتحقق منه).
- **غير موقّع في 0.6.0:** يظهر SmartScreen «حمى Windows جهاز الكمبيوتر» عند التشغيل من ملف منزَّل، والتجاوز بـ «مزيد من المعلومات» ثم «تشغيل على أي حال». يوثَّق في README بالعربية والإنكليزية مع بصمة SHA-256 لكل ملف. والتوقيع لاحقاً بـ `osslsigncode` 2.12 (GPL-3.0-or-later) على لينكس حين تتوفر شهادة.

### 9. الاختبار

- **على لينكس، في كل قصة:** المنطق الصافي (اسم الأنبوب، اختيار المسارات من البيئة، نص مفاتيح السجل، `file_url`، الدمج) في وحدات مشتركة، فيُختبر ويُطفَّر على لينكس. شيفرة ويندوز الرقيقة في ملفات مستقلة تُضاف إلى `exclude_globs`، وتُغطيها الاختبارات في ويندوز.
- **اختبارات الوحدات على ويندوز:** وظيفة `windows-latest` في CI تشغّل `cargo test` لـ core وengines وcli. وفي الآلة: `cargo test --no-run --target x86_64-pc-windows-gnu` يبني ملفات الاختبار، وسكربت `scripts/windows/run-tests.cmd` يشغّلها ويكتب النتائج في ملف. wine غير مثبت (wine-core 11.0 متاح في Fedora). يصلح لاختبارات core وengines دون واجهة إن وافق المالك على تثبيته، لكن ACL الأنابيب والتركيز لا يُحكم عليها إلا في ويندوز حقيقي.
- **النقل إلى الآلة دون وكيل:** صورة ISO بالملفات تُرفق بـ `virsh -c qemu:///session attach-disk win11 <iso> sdb --type cdrom --mode readonly`، وهو طريق مضمون. الشبكة user-mode (slirp أو passt، لم نتحقق) قد تسمح بـ HTTP إلى المضيف، فهي غير معتمدة حتى تُجرَّب. الأفضل بموافقة المالك: تثبيت virtio-win guest tools وفيها `qemu-guest-agent`، فيصير التشغيل وقراءة النتائج آلياً عبر `virsh qemu-agent-command` (`guest-exec` و`guest-file-read`)، ومعها spice-webdavd لمجلد مشترك من Boxes.
- **ما يبقى يدوياً في واجهة الآلة (القائمة الحية):** شفافية زوايا الخلية، وموضعها عند المؤشر وعلى الحواف وبمقياس 150%، والتركيز وEsc عند الفتح من المستكشف بملف ثم بعشرة ملفات، والإفلات من المستكشف على خلية والتأكد أن الأصل باقٍ، والاختصار أثناء السحب، والشريط وقائمته بالعربية، والإشعار وزر فتح المجلد، والمحركات الثلاثة بعد تثبيتها، والإلغاء يقتل `soffice.bin`، والمثبّت والإزالة، وتحذير SmartScreen، وNarrator على الخلية (accesskit_windows موجود في الشجرة عبر Slint). كل نافذة اختبار بحجم محدود ثابت، ويُتحقق من هندستها الفعلية، ولا يُغلق إلا ما فتحه الاختبار.
- **الاستعادة:** لقطة نظيفة للآلة قبل كل جولة إن وافق المالك على إنشائها، ومع EFI تكون اللقطة خارجية.

## الحزم

لا حزمة جديدة في `Cargo.lock`. الجديد اعتماد مباشر واحد تحت `cfg(windows)`:

| الحزمة | الإصدار | الرخصة | أين | ملاحظة |
| --- | --- | --- | --- | --- |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 | core، engines، cli، ui، daemon | موجودة في الشجرة عبر tempfile وrustix وwinit وغيرها |
| notify-rust (موجودة) ← tauri-winrt-notification | 0.8.1 | MIT OR Apache-2.0 | daemon | موجودة في الشجرة، تُفعَّل على ويندوز فقط |
| windows (احتياطي فقط) | 0.62.2 | MIT OR Apache-2.0 | ui | إن احتجنا `IDropTarget` خاصاً، موجودة في الشجرة |

المرفوض: `interprocess` 2.4.6 (0BSD OR Apache-2.0)، و`tray-icon` 0.26.1 (MIT OR Apache-2.0)، و`global-hotkey` 0.9.0 (Apache-2.0 OR MIT). أدوات البناء (لا تُشحن): mingw64-gcc 16.2.1 (مثبت)، وmsitools 0.106، وmingw32-nsis 3.11 احتياطياً، وosslsigncode 2.12 لاحقاً، وwine-core 11.0 اختيارياً. كلها في مستودع Fedora، وغير المثبت منها يحتاج موافقة المالك. تُسجَّل في `docs/engines.md` مع أول قصة تضيفها.

## القصص المقترحة

الترقيم من S-040، لأن S-021 مأخوذة («خلية الإفلات لا تظهر إلا لسحب حقيقي»)، وعبارة PLAN.md «تُرقَّم من S-021» خاطئة. القصص من S-040a إلى S-044 لا تلمس `crates/ui`، لأن جلسة أخرى تعدّل `crates/ui/src/layer.rs` الآن، ويُتحقق منها بـ `cargo check -p sbarhive-core -p sbarhive-engines -p sbarhive-cli --target x86_64-pc-windows-gnu`. الخدمة لا تُفحص لويندوز قبل S-045 لأنها تعتمد على ui، لكن تعديلاتها في S-040a وS-023 تُبنى وتُختبر على لينكس.

| القصة | النطاق | يلمس ui |
| --- | --- | --- |
| S-040a | واجهة النقل في core بتطبيق Unix الحالي، ودالة اسم الأنبوب الصافية، وتحويل `client.rs` والخدمة إليها؛ كلها تُختبر على لينكس | لا |
| S-040b | أنبوب ويندوز: DACL للمستخدم وحده، ورفض البعيد، والنسخة الواحدة، ومهلة القراءة بـ `CancelIoEx`، والتحقق من مالك الخادم | لا |
| S-041 | المسارات: `%APPDATA%` و`%LOCALAPPDATA%`، ونقل `user_actions_dir` إلى core، ورجوع `finalize` بـ `MoveFileExW` دون استبدال | لا |
| S-042 | تشغيل الخدمة من سطر الأوامر (الأعلام والرجوع عند رفض الخروج من المهمة)، وبديل `exec_daemon`، و`AllowSetForegroundWindow` قبل `Open` و`Drop` | لا |
| S-043a | كائن المهمة بالتعليق ثم الإسناد ثم الاستئناف في `non_unix.rs`، و`is_executable` | لا |
| S-043b | أسماء البرامج على ويندوز ومساراتها والسجل، وإصلاح `file_url` | لا |
| S-044 | البناء: سكربت بناء متقاطع داخل النطاق المحدود، وفحص `objdump`، وبوابة clippy للهدف، ووظيفتا CI، وسكربت ISO و`run-tests.cmd` | لا |
| S-045 | أول قصة ui: بوابة `cfg` لـ `token_hook` وما يخص Wayland وX11 في `lib.rs` فقط، وksni لـ لينكس، ووحدتا شريط وإشعار فارغتان لويندوز، و`windows_subsystem` وملف السجل، فيُبنى الـ workspace كله لويندوز | نعم (بعد انتهاء layer.rs) |
| S-046 | نافذة الخلية: الشفافية وبدائلها، والموضع عند المؤشر بمنطقة العمل، وبلا ظل ولا تدوير، والتركيز بـ `SetForegroundWindow`، ووضع العميل في `sbarhived open`، ودمج التحديد المتعدد | نعم |
| S-047 | الإفلات: `HoveredFile` و`DroppedFile` في `FocusWatcher`، واستطلاع `GetCursorPos`، و`Layout::pick`، وقاعدة S-021 | نعم |
| S-048 | خيط الصدفة: أيقونة الشريط وقائمتها بالعربية و`TaskbarCreated`، والاختصار بالضغط والترك | لا (daemon) |
| S-049 | الإشعارات: `app_id`، و`explorer.exe /select`، وعدّ الإشعارات من ملف السجل | لا (daemon) |
| S-050 | تكامل المستكشف: `integrations/windows/registry/`، و`sbarhive integration install` و`uninstall` | لا |
| S-051 | المثبّت: MSI لكل مستخدم بـ `wixl` (أو NSIS)، وAUMID، والاختصار، والتشغيل عند الدخول، والإزالة، ورفعه مع الإصدار | لا |
| S-052 | القائمة الحية في الآلة، وتحديث README وCHANGELOG، ووسم 0.6.0 | لا |

S-048 إلى S-051 لا تعتمد على ui، فيمكن أن تسبق S-046 إن طال انتظار layer.rs، لكن فحصها لويندوز يحتاج S-045. كل قصة يوم عمل أو أقل، وS-028 إن كبرت تنقسم إلى S-046a (النافذة والشفافية والموضع) وS-028b (التركيز ووضع العميل والدمج).

## ما يحتاجه المالك

- [ ] ويندوز 11 مكتمل التثبيت في `win11` بحساب محلي (حالة الآلة قد تكون «installation»)، ويُفضَّل ويندوز 10 22H2 أيضاً إن بقي مدعوماً.
- [ ] الإذن بالتثبيت داخل الآلة: virtio-win guest tools (فيها `qemu-guest-agent`) وspice-webdavd، وFFmpeg وLibreOffice وGhostscript لفحص المحركات.
- [ ] الإذن بإنشاء لقطة نظيفة للآلة.
- [ ] الإذن بتثبيت أدوات على المضيف: msitools، وxorriso أو genisoimage لصور ISO، واختيارياً wine-core وmingw32-nsis.
- [ ] شهادة توقيع (OV أو EV، أو خدمة توقيع سحابية) للمثبّت وللحزمة sparse. بدونها: القائمة القديمة وحدها، وتحذير SmartScreen.

## ما بقي مفتوحاً

- [ ] هل يقبل المالك أن تظهر SbarHive في ويندوز 11 تحت «عرض مزيد من الخيارات» في 0.6.0؟
- [ ] الدمج 300 ms أم فعل `DropTarget` للتحديد المتعدد؟ يُحسم بالقياس في الآلة.
- [ ] الاختصار الافتراضي على ويندوز (كثير من تركيبات Win محجوزة للنظام)، وهل يبقى «الضغط المطوّل» كما على Hyprland.
- [ ] الحد الأدنى: ويندوز 10 22H2 أم 11 فقط؟ وarm64 خارج النطاق؟
- [ ] شفافية softbuffer عبر `BitBlt`، ووصول `WM_HOTKEY` أثناء سحب OLE، وسلوك `soffice.com`، وبادئة `file:` في ffmpeg، وقدرة `wixl` على التثبيت لكل مستخدم: كلها (مُفترَض) حتى أول جولة في الآلة.

## المصادر

- أمان الأنابيب والواصف الافتراضي: https://learn.microsoft.com/en-us/windows/win32/ipc/named-pipe-security-and-access-rights
- `CreateNamedPipeW` (`PIPE_REJECT_REMOTE_CLIENTS`، `FILE_FLAG_FIRST_PIPE_INSTANCE`): https://learn.microsoft.com/windows/win32/api/namedpipeapi/nf-namedpipeapi-createnamedpipew
- `GetNamedPipeServerProcessId`: https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getnamedpipeserverprocessid
- `SetForegroundWindow` وشروطه: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setforegroundwindow
- `AllowSetForegroundWindow`: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-allowsetforegroundwindow
- أعلام إنشاء العمليات: https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags
- كائنات المهام: https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects
- `MoveFileExW`: https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw ، و`std::fs::rename` على ويندوز: https://doc.rust-lang.org/std/fs/fn.rename.html
- المجلدات المعروفة: https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid
- `Shell_NotifyIconW`: https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shell_notifyiconw
- `RegisterHotKey`: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerhotkey
- معرّفات AppUserModelID، وهي ما تحتاجه الإشعارات من تطبيق غير محزوم: https://learn.microsoft.com/en-us/windows/win32/shell/appids
- الأفعال الثابتة، و`MultiSelectModel` وحدوده، وطريقة `IDropTarget`: https://learn.microsoft.com/en-us/windows/win32/shell/context-menu-handlers
- قائمة ويندوز 11 بـ `IExplorerCommand`: https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/integrate-packaged-app-with-file-explorer ، والهوية بحزمة sparse: https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/grant-identity-to-nonpackaged-apps
- SmartScreen: https://learn.microsoft.com/en-us/windows/security/operating-system-security/virus-and-threat-protection/microsoft-defender-smartscreen/
- winit 0.30.13 (`drop_handler.rs`، `window.rs`): https://github.com/rust-windowing/winit/tree/v0.30.13/src/platform_impl/windows
- softbuffer 0.4.8 (`win32.rs`): https://github.com/rust-windowing/softbuffer/blob/v0.4.8/src/backends/win32.rs
- notify-rust 4.18.2: https://github.com/hoodie/notify-rust ، وtauri-winrt-notification: https://github.com/tauri-apps/winrt-notification
- msitools و`wixl`: https://gitlab.gnome.org/GNOME/msitools ، وNSIS: https://nsis.sourceforge.io ، وosslsigncode: https://github.com/mtrojnar/osslsigncode
