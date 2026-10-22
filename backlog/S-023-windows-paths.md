# S-023 مسارات الإعدادات والإجراءات والسجل على ويندوز، وتسمية الناتج دون استبدال
الحالة: منتهية (PASS من المراجع 2026-10-22)، على فرع windows/0.6.0 بطلب محمد؛ ADR 0005 ما زال مقترحاً، ولا دمج في main

يبني على ADR 0005 القسم 4، وعلى S-022b (اعتماد windows-sys في core). للإصدار 0.6.0. لا يلمس `crates/ui`. يوم عمل واحد أو أقل، فلا تقسيم.

## القصة
بصفتي مستخدماً على ويندوز، أريد أن تُحفظ إعداداتي وإجراءاتي في `%APPDATA%\SbarHive` وسجلّ الخدمة في `%LOCALAPPDATA%\SbarHive\logs`، وألا يستبدل الناتج ملفاً موجوداً حتى على قرص USB، حتى أجد ملفاتي حيث يتوقعها ويندوز ولا أفقد ملفاً أبداً.

## الملفات
`crates/core/src/{lib.rs,paths.rs,paths/windows.rs,config.rs,output_impl.rs,output_impl/windows.rs}`، `crates/core/Cargo.toml` (ميزات windows-sys فقط)، `crates/daemon/src/state.rs`، `crates/daemon/src/watch.rs`، `crates/cli/src/main.rs`، `.cargo/mutants.toml`، `docs/architecture.md`، `README.md`، `CHANGELOG.md`.

## معايير القبول
- AC1: دالة صافية `paths::config_dir_with(os, env, known)` (الاسم مقترح): `os` من `Os::{Linux, Windows}`، و`env` دالة تقرأ متغيراً، و`known` دالة تعطي المجلد المعروف. على ويندوز مع `APPDATA=A` تعيد `Path::new(A).join("SbarHive")`، ولا تستدعي `known` (عدّاد 0). ودون `APPDATA` أو بقيمة فارغة تعيد `known()` مضافاً إليها `SbarHive`، و`None` إن أعادت `known` `None`. على لينكس تعيد ما تعيده اليوم: `XDG_CONFIG_HOME/sbarhive` ثم `HOME/.config/sbarhive` ثم `None`.
- AC2: `paths::log_dir_with(os, env, known)`: على ويندوز `Path::new(LOCALAPPDATA).join("SbarHive").join("logs")`، ورجوع `known` كما AC1. وعلى لينكس `None` (السجل يبقى stderr).
- AC3: `Config::path()` تساوي `config_dir().join("config.toml")`، و`Config::actions_dir()` الجديدة تساوي `config_dir().join("actions")`. توقيعا `Config::path` و`Config::load` و`Config::load_from` كما هي، لأن `crates/ui` يستدعيها. والاختبار القائم `path_and_load_follow_xdg_then_home` ينجح دون تعديل.
- AC4: `user_actions_dir` تُحذف من `crates/daemon/src/state.rs`، و`load_registry` يستدعي `Config::actions_dir()`. `grep -rn 'fn user_actions_dir' crates/` يعيد 0 أسطر، وإجراء TOML في `XDG_CONFIG_HOME/sbarhive/actions/` يظهر في `sbarhive actions` كما اليوم.
- AC5: `paths/windows.rs` يقرأ المجلدين بـ `SHGetKnownFolderPath` (`FOLDERID_RoamingAppData` و`FOLDERID_LocalAppData`) ويحرر الذاكرة بـ `CoTaskMemFree`. الملف في `exclude_globs`. `cargo clippy -p sbarhive-core --lib` للهدف ويندوز بلا تحذير.
- AC6: `finalize_with` تأخذ دالة وضع بلا استبدال مُحقنة بدل الفحص ثم `rename` المكتوب داخلها. مع دالة وهمية تعيد `AlreadyExists` لـ `a.webp` و`a (1).webp`: الناتج `a (2).webp`، ومحتوى الملفين الموجودين لم يتغير، والمؤقت محذوف. ومع دالة تعيد خطأً آخر (`PermissionDenied`): يعود الخطأ نفسه، والمؤقت باقٍ.
- AC7: على لينكس دالة الوضع هي سلوك اليوم (الفحص ثم `rename`)، فاختبارات `ac9_*` الأربعة في `output_impl.rs` و`crates/core/tests/output.rs` تنجح دون تعديل في تأكيداتها. وعلى ويندوز هي `MoveFileExW` دون `MOVEFILE_REPLACE_EXISTING` في `output_impl/windows.rs`، والرمزان 80 و183 يصيران `AlreadyExists`. دالة الربط بالرموز صافية في `output_impl.rs`: 80 و183 تعطيان `AlreadyExists`، و5 تعطي `PermissionDenied`. `output_impl/windows.rs` في `exclude_globs`.
- AC8: لا حزمة جديدة: عدد `[[package]]` في `Cargo.lock` يبقى 671، و`X` لا يسرد غير ما سرده بعد S-022b.
- AC9: `README.md` يذكر مكان `config.toml` والإجراءات والسجل على ويندوز بالعربية والإنكليزية، و`docs/architecture.md` يصف دالة الوضع بلا استبدال. سطر واحد في `CHANGELOG.md` تحت `[Unreleased]` بالإنكليزية مع نص عربي بين «». `git diff --stat main...HEAD -- crates/ui` فارغ. `C` بلا تحذير على لينكس: دالة الرموز ودوال المسارات التي لا يستدعيها إلا ملف ويندوز `pub` أو عليها `#[cfg_attr(not(windows), allow(dead_code))]`. `M` بلا طفرة ناجية.

## أوامر الفحص
أدوات القراءة وحدها (`grep`) تعمل خارج النطاق.
- `C`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% sh -c 'cargo fmt --check && cargo clippy -j 2 --workspace --all-targets -- -D warnings'`
- `L`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% cargo test -j 2 -p sbarhive-core -p sbarhive-daemon -p sbarhive-cli`
- `W`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_TARGET_DIR=/home/sbarah/.cache/sbarhive-win-target cargo clippy -j 2 --target x86_64-pc-windows-gnu -p sbarhive-core -p sbarhive-engines --lib -- -D warnings`
- `M`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_BUILD_JOBS=2 cargo mutants --jobs 1 --in-diff <diff>` (الملفات المتغيرة وحدها، ومنها `state.rs`؛ ملفات ويندوز مستثناة)
- `X`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_TARGET_DIR=/home/sbarah/.cache/sbarhive-win-target cargo tree --target x86_64-pc-windows-gnu -p sbarhive-core -e normal --depth 1`

## حالات الاختبار
| معيار | الاختبار | النوع |
| --- | --- | --- |
| AC1، AC2 | وحدة في `paths.rs` ببيئة وهمية لكل فرع: `APPDATA` موجودة، وفارغة، وغائبة مع `known` يعيد مجلداً ثم `None`، وفرع لينكس الثلاثي. القيم المتوقعة تُبنى بـ `Path::join` نفسها لا بنص فيه `\`، لأن لينكس يربط بـ `/`؛ تحت `M` | وحدة |
| AC3 | الاختبار القائم في `config.rs`، واختبار جديد لـ `actions_dir` | وحدة |
| AC4 | الأمر `grep`؛ و`crates/cli/tests/cli.rs` بإجراء مزروع في المجلد المؤقت | مراجعة، تكامل |
| AC5 | `W` بلا تحذير، والمراجع يقرأ `paths/windows.rs` | بناء، مراجعة |
| AC6، AC7 | وحدة في `output_impl.rs` بدالتي وضع وهميتين، ودالة الرموز؛ واختبارات `ac9_*` القائمة؛ تحت `M` | وحدة |
| AC8 | `grep -c '^\[\[package\]\]' Cargo.lock` يعطي 671، و`X` | مراجعة |
| AC9 | المراجع يقرأ README وarchitecture.md وCHANGELOG.md | مراجعة |

تبقى خضراء: `cargo test --workspace`، و`tests/e2e/run.sh`.

## تحقق حي مؤجَّل إلى S-034
- `config.toml` يُكتب في `%APPDATA%\SbarHive` من نافذة الإعدادات، والحفظ الذري بـ `std::fs::rename` يستبدل الملف القديم.
- حساب دون `APPDATA` في البيئة يجد المجلد بـ `SHGetKnownFolderPath`.
- تحويل على قرص exFAT لا يستبدل ملفاً موجوداً بالاسم نفسه.

## خارج النطاق
- كتابة السجل في الملف و`windows_subsystem`: S-027. هنا الدالة التي تعطي المجلد فقط.
- المؤقت: `std::env::temp_dir()` يعطي `%TEMP%`، والمؤقت يبقى بجانب الناتج؛ لا تغيير.
- مسارات UNC (`\\server\share`) للإعدادات.

## سجل المراجعة
- 2026-10-22، المخطِّط: الاختيار يأخذ `os` معاملاً حتى يُختبر فرع ويندوز على لينكس، والاستدعاء الحقيقي يمرّر `Os` من `cfg!(windows)`.
- 2026-10-22، المختبِر: العقد الذي ثبّته المختبِر في رأس `crates/core/tests/paths.rs` وفي تعليق وحدة الاختبار في `output_impl.rs` (لا يُعدَّلان):
  - `sbarhive_core::paths` وحدة `pub` على كل المنصات، فيها `enum Os { Linux, Windows }` و`config_dir_with(os, env, known)` و`log_dir_with(os, env, known)` و`config_dir()`، كلها `pub`. `env` بشكل `Fn(&str) -> Option<OsString>` (شكل `var_os`)، و`known` بشكل `Fn() -> Option<PathBuf>`، والاختبارات تمرّرهما بمرجع فيصلح `&dyn Fn` والمعاملات العامة. `Config::actions_dir() -> Option<PathBuf>`.
  - `finalize_with(temp, final_path, link: &LinkFn, place: &PlaceFn)` حيث `PlaceFn` هو `dyn Fn(&Path, &Path) -> std::io::Result<()>`: ينقل ويفشل بـ `AlreadyExists` إن كان الاسم مأخوذاً. `finalize_with` لا يفحص الوجود ولا يستدعي `rename` بنفسه حين يفشل الربط، ويبقى إرجاع خطأ الربط إن غاب المؤقت (`ac9_fallback_with_missing_temp_reports_the_link_error`). الدوال الوهمية لا تلتقط شيئاً، فيصلح اسم مستعار `'static` كـ `LinkFn`.
  - `place_no_replace(from, to)` مرئية في `output_impl.rs`: على لينكس الفحص ثم `rename`، وعلى ويندوز من `output_impl/windows.rs`. و`move_error_kind(code: u32) -> ErrorKind` صافية: 80 و183 `AlreadyExists`، و5 `PermissionDenied`، وغيرها ليس أياً منهما.
  - الملفات والاختبارات:
    - `crates/core/tests/paths.rs` (جديد): `ac1_windows_appdata_wins_and_known_is_not_called`، `ac1_windows_empty_appdata_falls_back_to_the_known_folder`، `ac1_windows_missing_appdata_falls_back_to_the_known_folder`، `ac1_windows_ignores_xdg_and_home`، `ac1_windows_without_appdata_and_without_known_folder_is_none`، `ac1_windows_appdata_is_not_read_as_localappdata`، `ac1_linux_xdg_config_home_first`، `ac1_linux_home_dot_config_second`، `ac1_linux_without_xdg_and_home_is_none_even_with_appdata` (فروع لينكس الثلاثة تتحقق أن `known` لا يُستدعى)، `ac2_windows_log_dir_is_localappdata_sbarhive_logs_without_known`، `ac2_windows_empty_or_missing_localappdata_falls_back_to_the_known_folder` (و`APPDATA` وحدها لا تنوب)، `ac2_windows_without_localappdata_and_without_known_folder_is_none`، `ac2_linux_has_no_log_dir`، `ac3_config_dir_follows_the_process_environment_on_linux`، `ac3_config_path_is_config_dir_joined_with_config_toml`، `ac3_actions_dir_is_config_dir_joined_with_actions`.
    - `crates/core/src/output_impl.rs` (وحدة الاختبار): `ac6_place_already_exists_moves_on_to_the_next_number` (الملفان موجودان فعلاً، وترتيب الأسماء المسؤولة `a.webp` ثم `a (1).webp` ثم `a (2).webp`)، `ac6_finalize_trusts_the_place_function_not_its_own_check` (لا ملف على القرص، فالفحص الباقي داخل `finalize_with` يعطي `a.webp`)، `ac6_place_permission_denied_is_returned_and_keeps_the_temp` (استدعاء واحد بلا إعادة على اسم آخر)، `ac6_a_successful_link_does_not_call_place`، `ac7_linux_place_refuses_a_taken_name_and_keeps_both_files`، `ac7_linux_place_refuses_a_dangling_link_name`، `ac7_linux_place_moves_to_a_free_name`، `ac7_move_error_codes_80_and_183_are_already_exists`، `ac7_move_error_code_5_is_permission_denied`، `ac7_move_error_other_codes_are_neither`. وعُدّلت استدعاءات `finalize_with` في `ac9_*` الثلاثة التي تستدعيها لتمرّر `&place_no_replace` معاملاً رابعاً، وتأكيداتها كما هي حرفياً (AC7).
    - `crates/core/tests/paths_windows.rs` (جديد، فحص نص ملفات ويندوز على طريقة `transport_seed.rs`): `ac5_paths_windows_reads_both_known_folders_and_frees_them`، `ac5_ac7_windows_files_are_excluded_from_mutants`، `ac5_ac7_paths_rs_and_output_impl_rs_stay_mutated`، `ac7_windows_places_with_move_file_ex_without_replacing` (يمنع اسم `MOVEFILE_REPLACE_EXISTING` في الملف كله، والتعليقات منه، ويطلب `MoveFileExW` و`move_error_kind(`).
    - `crates/daemon/tests/actions_dir.rs` (جديد): `ac4_no_crate_defines_user_actions_dir` (يمشي `crates/` كلها، والنص المبحوث مبني من قطعتين فلا يطابقه `grep` المراجع)، `ac4_load_registry_uses_config_actions_dir`.
  - ينجح قبل التنفيذ عن قصد: `actions_include_user_actions_from_the_config_dir` في `crates/cli/tests/cli.rs` (حارس تراجع لسلوك AC4 «كما اليوم»)، و`ac13_cargo_lock_keeps_671_packages` من S-022b (حارس AC8)، و`path_and_load_follow_xdg_then_home` و`ac9_*` في `crates/core/tests/output.rs`.
  - غموض وما اختاره المختبِر: (1) الجدول يطلب وحدة داخل `paths.rs`، والملف ملك المنفِّذ، فاختبارات AC1 وAC2 وAC3 في `crates/core/tests/paths.rs` عبر الواجهة العامة؛ واختبار `actions_dir` ليس في `config.rs` لأن الاختبار القائم هناك يغيّر البيئة فيتسابق معه، وثنائي منفصل لا يغيّر البيئة يقرأها كما هي. (2) **AC4 لا ينجح دون لمس ملفين خارج قائمة الملفات:** `crates/cli/src/main.rs:235` يعرّف `user_actions_dir` خاصة به، و`crates/daemon/src/watch.rs` يستوردها من `state.rs`؛ `grep` المعيار على `crates/` كلها يعدّ الأول، والثاني لا يُبنى بعد الحذف. لم يضيّق المختبِر الاختبار، فعلى المنفِّذ تعديلهما إلى `Config::actions_dir()`. (3) `XDG_CONFIG_HOME` الفارغة على لينكس غير مثبتة: «كما اليوم» يعني أنها تُستعمل كما هي، ولم يُكتب لها اختبار. (4) رموز ويندوز غير 80 و183 و5 لم يحددها AC7؛ ثبّت المختبِر أنها ليست `AlreadyExists` ولا `PermissionDenied` (0 و1 و2 و3 و79 و81 و87 و182 و184) حتى لا تنجو طفرة «كل رمز مأخوذ»، وترك نوعها للمنفِّذ. (5) أسماء `place_no_replace` و`move_error_kind` و`config_dir` اختارها المختبِر لأن المعيار لم يسمّها.
  - للمراجع لا للاختبار: `W` و`X`، وقراءة `paths/windows.rs` (تحرير الذاكرة في كل مسار)، وميزات `windows-sys` في `crates/core/Cargo.toml`، وREADME وarchitecture.md وCHANGELOG.md، و`git diff --stat main...HEAD -- crates/ui`، و`M`.
  - للمراجع، الاختبارات المضمّنة لا يراها فحص `crates/*/tests/*`: وحدة الاختبار في `output_impl.rs` والاختبار القائم في `config.rs` يجب ألا تتغير بعد هذا الالتزام. الأمران (فرق فارغ مطلوب):
    `diff <(git show f1e430e:crates/core/src/output_impl.rs | sed -n '/^#\[cfg(test)\]/,$p') <(sed -n '/^#\[cfg(test)\]/,$p' crates/core/src/output_impl.rs)`
    `diff <(git show f1e430e:crates/core/src/config.rs | sed -n '/fn path_and_load_follow_xdg_then_home/,/^    }$/p') <(sed -n '/fn path_and_load_follow_xdg_then_home/,/^    }$/p' crates/core/src/config.rs)`
  - تنبيه: ثنائي اختبارات `--lib` في core لا يُبنى حتى يصل التنفيذ، فكل اختبارات الوحدة القائمة في core (ومنها `transport.rs` من S-022a و`config.rs`) تظهر حمراء حتى ذلك؛ لا مفر منه لأن الدوال المختبرة خاصة.
  - تحقق من منطق الاختبارات: نسخة مؤقتة من المستودع في مجلد الجلسة (حُذفت ولا تُدمج) بتطبيق مختصر للعقد نجحت فيها 28 من اختبارات `--lib` في core، و`paths` الستة عشر، و`paths_windows` الأربعة، و`output` الأربعة عشر، و`actions_dir` الاثنان؛ و`cargo clippy -p sbarhive-core --tests -D warnings` نظيف عليها. `cargo fmt --check` نظيف هنا.
  - مخرجات الفشل (كل أمر داخل `systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200%`):
```
$ cargo test -j 2 -p sbarhive-core --lib
error[E0061]: this function takes 3 arguments but 4 arguments were supplied
error[E0425]: cannot find function `move_error_kind` in this scope
error[E0425]: cannot find function `place_no_replace` in this scope
error[E0425]: cannot find value `place_no_replace` in this scope
error: could not compile `sbarhive-core` (lib test) due to 17 previous errors

$ cargo test -j 2 -p sbarhive-core --test paths
error[E0432]: unresolved import `sbarhive_core::paths`
error[E0599]: no associated function or constant named `actions_dir` found for struct `Config` in the current scope
error: could not compile `sbarhive-core` (test "paths") due to 2 previous errors

$ cargo test -j 2 -p sbarhive-core --test paths_windows
test ac5_ac7_paths_rs_and_output_impl_rs_stay_mutated ... FAILED
test ac5_ac7_windows_files_are_excluded_from_mutants ... FAILED
test ac5_paths_windows_reads_both_known_folders_and_frees_them ... FAILED
test ac7_windows_places_with_move_file_ex_without_replacing ... FAILED
crates/core/src/paths.rs must exist
"crates/core/src/paths/windows.rs" must be in exclude_globs (AC5, AC7)
.../crates/core/src/output_impl/windows.rs must exist: No such file or directory (os error 2)
.../crates/core/src/paths/windows.rs must exist: No such file or directory (os error 2)
test result: FAILED. 0 passed; 4 failed

$ cargo test -j 2 -p sbarhive-daemon --test actions_dir
test ac4_load_registry_uses_config_actions_dir ... FAILED
test ac4_no_crate_defines_user_actions_dir ... FAILED
load_registry must call Config::actions_dir() (AC4):
these files still define the old helper: [".../crates/cli/src/main.rs", ".../crates/daemon/src/state.rs"] (AC4)
test result: FAILED. 0 passed; 2 failed

$ cargo test -j 2 -p sbarhive-cli --test cli actions_include_user_actions
test actions_include_user_actions_from_the_config_dir ... ok
test result: ok. 1 passed; 0 failed
```
- 2026-10-22، المدير: قرار على عائق المختبِر (2): تُضاف إلى قائمة الملفات `crates/cli/src/main.rs` (تُحذف دالته الخاصة للمجلد ويُستعمل `Config::actions_dir()`) و`crates/daemon/src/watch.rs` (يستورد `Config::actions_dir()` بدل الدالة المحذوفة). كلاهما خارج `crates/ui`.
- 2026-10-22، المنفِّذ:
  - `crates/core/src/paths.rs` (جديد): `Os` و`CURRENT` و`config_dir_with` و`log_dir_with` و`config_dir`، والمعاملان `&dyn Fn`. المتغير الفارغ على ويندوز يرجع إلى المجلد المعروف، و`XDG_CONFIG_HOME` الفارغة على لينكس تُستعمل كما هي (كما اليوم). لا دالة `log_dir()` على لينكس لأنها تعيد `None` دائماً ولا يختبرها شيء؛ هي في `paths/windows.rs` وحده ويُعاد تصديرها على ويندوز لـ S-027.
  - `crates/core/src/paths/windows.rs` (جديد): `SHGetKnownFolderPath` مع `FOLDERID_RoamingAppData` و`FOLDERID_LocalAppData`، و`CoTaskMemFree` على كل مسار، نجاحاً وفشلاً.
  - `crates/core/src/output_impl.rs`: `PlaceFn` و`finalize_with` بأربعة معاملات بلا فحص ولا `rename` داخلها، و`place_no_replace` (لينكس: الفحص ثم `rename`)، و`move_error_kind` (80 و183 `AlreadyExists`، و5 `PermissionDenied`، وغيرها `Other`؛ لا `from_raw_os_error` لأن 1 على لينكس EPERM). وحدة الاختبار لم تتغير (الفرقان من سجل المختبِر فارغان).
  - `crates/core/src/output_impl/windows.rs` (جديد): `MoveFileExW` بأعلام 0، ويحفظ رمز النظام داخل الخطأ.
  - `config.rs`: `path()` و`actions_dir()` من `paths::config_dir()`، والتوقيعات كما هي. `Cargo.toml`: الميزتان `Win32_UI_Shell` و`Win32_System_Com`. `.cargo/mutants.toml`: الملفان في `exclude_globs`.
  - انحراف: طفرة `replace run with ()` في `watch.rs` ناجية لأن `run` حلقة استطلاع لا تنتهي ولا يشغّلها اختبار وحدة، وS-023 غيّر فيها سطر الاستيراد واستدعاء مجلد الإجراءات داخل `run` فقط. أضفت النمط `replace run with \\(\\)$` إلى `exclude_re` مع تعليق، على سابقة `serve`؛ لا يطابق `run` في `cli/src/main.rs` لأنها تعيد `Res<()>` (تحققت بـ `--list`). `tests/e2e/run.sh` يغطي إعادة التحميل.
  - البوابات (كلها داخل `systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200%`):
    - `C`: `cargo fmt --check` و`cargo clippy -j 2 --workspace --all-targets -- -D warnings` نظيفان.
    - `L` و`cargo test -j 2 --all`: 642 ناجحاً، 0 فاشلاً، 1 متجاهلاً، في 43 ثنائياً.
    - `W`: clippy و`cargo check` لـ core وengines `--lib` للهدف `x86_64-pc-windows-gnu` بلا تحذير. daemon وcli بلا هدف `lib`.
    - `M` على فرق الملفات السبعة المتغيرة من 8a27c8e: 25 طفرة، 23 ممسوكة، 1 غير قابلة للبناء (`move_error_kind` بـ `Default::default()`، لأن `ErrorKind` بلا `Default`)، 1 ناجية (`watch.rs` أعلاه، مستثناة الآن، فالقائمة 24).
    - `X`: serde وserde_json وthiserror وtoml وwindows-sys كما بعد S-022b. `Cargo.lock` لم يتغير، و`[[package]]` 671.
    - `scripts/design-lint.sh` نظيف. `tests/e2e/run.sh`: كله ناجح. `git diff --stat main...HEAD -- crates/ui` فارغ.
    - ملاحظة: `--jobs 1` مرفوض مع `--in-place`، فشغّلت `--in-place` وهو تسلسلي بطبعه. وتداخل تشغيلان (بناء e2e والجولة الأخيرة) مع بناء من جلسة أخرى رغم الفحص بـ `pgrep`، ولم يفشل شيء.
- 2026-10-22، المراجع: **PASS**. كل البوابات أعيد تشغيلها داخل النطاق (`MemoryMax=6G`، `-j 2`) من شجرة `windows/0.6.0` على `6e010dd`:
  - `cargo fmt --check` نظيف. `cargo clippy -j 2 --workspace --all-targets -- -D warnings` بلا تحذير. `cargo test -j 2 --all`: 642 ناجحاً، 0 فاشلاً، 1 متجاهل، في 43 ثنائياً؛ اختبارات القصة الأربعة والثلاثون بالاسم ناجحة، ومعها `ac9_*` الأربعة في `output_impl.rs` والخمسة في `tests/output.rs` و`ac13_cargo_lock_keeps_671_packages`.
  - `W` (بعد `touch` لإجبار إعادة الفحص، `CARGO_TARGET_DIR` المنفصل): `cargo clippy` و`cargo check` للهدف `x86_64-pc-windows-gnu` لـ core وengines `--lib` بلا تحذير.
  - `X`: serde وserde_json وthiserror وtoml و`windows-sys v0.61.2` كما بعد S-022b. `Cargo.lock` بلا تغيير، و`[[package]]` 671.
  - `M`: `cargo mutants --jobs 1 --in-diff` على `git diff 8a27c8e..HEAD -- crates` بـ `CARGO_BUILD_JOBS=2`، في نسخة لا `--in-place`: 24 طفرة، 23 مكتشفة، 1 غير قابلة للبناء (`move_error_kind` بـ `Default::default()`)، 0 مهلة، **0 ناجية**. `load_registry` في `state.rs:42` مكتشفة.
  - `scripts/design-lint.sh` clean. لا شرطة طويلة في `git diff 15d7523..HEAD` (الضابط الموجب يعطي 1). `git diff --stat main...HEAD -- crates/ui` فارغ. `git diff 8a27c8e..HEAD -- 'crates/*/tests/*'` فارغ (الضابط على `15d7523..8a27c8e` يسرد الملفات الثلاثة)، وفرقا الاختبارات المضمّنة في `output_impl.rs` و`config.rs` من سجل المختبِر فارغان (الثاني يطابق 43 سطراً فعلاً). `grep -rn 'fn user_actions_dir' crates/` يعطي 0. `tests/e2e/run.sh`: 37 PASS، 0 FAIL، ومنه «edited settings apply without reload».
  - المعايير على الشيفرة: AC1 وAC2 في `paths.rs` صافيتان والمعاملات `&dyn Fn`، والفارغ على ويندوز يرجع إلى المجلد المعروف كما ينص AC1 حرفياً (ليس انحرافاً). AC3: التوقيعات كما هي. AC4: `state.rs` و`cli/src/main.rs` و`watch.rs` على `Config::actions_dir()` بقرار المدير. AC5: `paths/windows.rs` يحرر بـ `CoTaskMemFree` بعد كل استدعاء، نجاحاً وفشلاً ومؤشراً فارغاً (مسموح)، ويقرأ النص قبل التحرير، ويفحص `hr >= 0` و`!raw.is_null()`. AC6: `finalize_with` لا يفحص ولا يستدعي `rename`؛ `AlreadyExists` ينتقل للرقم التالي، وغيره يعود والمؤقت باقٍ، والنجاح ينقل المؤقت فلا يبقى؛ لا مسار يلمس غير المؤقت والأسماء المرشحة، فالأصل لا يُمس. AC7: `place_no_replace` على لينكس هو حرفياً الفحص ثم `rename` السابق، فسلوك لينكس لم يتغير، والسباق القائم موثّق؛ وعلى ويندوز `MoveFileExW` بأعلام 0 فلا استبدال أبداً. AC9: README بالعربية والإنكليزية، وarchitecture.md، وسطر CHANGELOG واحد من المنفِّذ، فلا سطر جديد من المراجع (سطر واحد للقصة).
  - الانحرافات مقبولة: `exclude_re` الجديد `replace run with \(\)$` يطابق في الشجرة كلها (`--list --no-config`) سطراً واحداً، `watch.rs:41`، و`run` حلقة لا تنتهي يغطيها e2e بإعادة تحميل `config.toml` دون `reload` (تفشل لو صارت `run` فارغة)، على سابقة `serve`. `log_dir()` على ويندوز وحده: AC2 يطلب `log_dir_with` وحدها وهي `pub`، فلا تحذير على لينكس.
  - ملاحظات غير حاجبة، لـ S-034:
    - منخفضة، `crates/core/src/output_impl/windows.rs:19-21`: `wide()` يمرر المسار خاماً إلى `MoveFileExW`، و`std::fs` على ويندوز يضيف بادئة `\\?\` للمسارات الطويلة (من معرفة تطبيق std؛ rust-src غير مثبت محلياً فلم يُتحقق). سؤال: مسار فوق 260 حرفاً على FAT قد ينجح بالربط ويفشل بالوضع `Other` مع بقاء المؤقت. لا خطر على الأصل.
    - منخفضة، `output_impl/windows.rs:30-35` مع `output_impl.rs:136-140`: إن كان الاسم المرشح مجلداً يعيد `MoveFileExW` غالباً 5، فيتوقف `finalize_with` بـ `PermissionDenied` بدل الانتقال للرقم التالي كما على لينكس. حالة نادرة على FAT وحده.
    - معلومة، `output_impl/windows.rs:34`: الخطأ المغلَّف بـ `io::Error::new(kind, os)` يعيد `raw_os_error() == None`؛ الرمز باقٍ في النص فقط.
    - معلومة، `.cargo/mutants.toml`: سطر تعليق `exclude_re` الجديد أطول من جيرانه.
