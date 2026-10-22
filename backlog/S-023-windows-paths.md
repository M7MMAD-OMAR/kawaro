# S-023 مسارات الإعدادات والإجراءات والسجل على ويندوز، وتسمية الناتج دون استبدال
الحالة: معتمدة للتنفيذ على فرع windows/0.6.0 بطلب محمد (2026-10-22)؛ ADR 0005 ما زال مقترحاً، ولا دمج في main

يبني على ADR 0005 القسم 4، وعلى S-022b (اعتماد windows-sys في core). للإصدار 0.6.0. لا يلمس `crates/ui`. يوم عمل واحد أو أقل، فلا تقسيم.

## القصة
بصفتي مستخدماً على ويندوز، أريد أن تُحفظ إعداداتي وإجراءاتي في `%APPDATA%\SbarHive` وسجلّ الخدمة في `%LOCALAPPDATA%\SbarHive\logs`، وألا يستبدل الناتج ملفاً موجوداً حتى على قرص USB، حتى أجد ملفاتي حيث يتوقعها ويندوز ولا أفقد ملفاً أبداً.

## الملفات
`crates/core/src/{lib.rs,paths.rs,paths/windows.rs,config.rs,output_impl.rs,output_impl/windows.rs}`، `crates/core/Cargo.toml` (ميزات windows-sys فقط)، `crates/daemon/src/state.rs`، `.cargo/mutants.toml`، `docs/architecture.md`، `README.md`، `CHANGELOG.md`.

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
