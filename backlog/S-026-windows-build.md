# S-026 البناء المتقاطع لويندوز وبوابته وCI وصورة الاختبار
الحالة: S-026a: الاختبارات مكتوبة وحمراء (`4af4c05`)، تنتظر المنفِّذ؛ S-026b لم تبدأ (فرع windows/0.6.0 بطلب محمد، 2026-10-22)؛ ADR 0005 ما زال مقترحاً، ولا دمج في main

يبني على ADR 0005 القسمين 1 و9، وعلى S-022 إلى S-025 (قبلها لا يُبنى `sbarhive.exe` للهدف). للإصدار 0.6.0. لا يلمس `crates/ui`.

## القصة
بصفتي مطوّراً على Fedora دون ويندوز، أريد أمراً واحداً يبني `sbarhive.exe` وملفات الاختبار لويندوز داخل نطاق ذاكرة محدود ويرفض أي مكتبة mingw مشحونة، وأن يفحص CI الشيفرة على ويندوز حقيقي، حتى لا يصل الآلة إلا بناء نظيف.

## التقسيم (بالترتيب، كل قصة فرعية يوم عمل أو أقل)
ADR يجعلها قصة واحدة، وهي أكبر من يوم، فقُسمت:
| القصة | المحتوى | المعايير | الملفات |
| --- | --- | --- | --- |
| S-026a | سكربت البناء، وفحص `objdump`، وبوابة clippy للهدف بكل الأهداف، وحصر اختبارات Unix | AC1 إلى AC7 | `scripts/windows/{build.sh,check-dlls.sh,check.sh,README.md}`، `tests/windows/fixtures/{objdump-ok.txt,objdump-mingw.txt}`، `crates/cli/tests/{cli.rs,settings.rs}`، `crates/core/tests/{output.rs,contracts.rs}`، `crates/engines/src/office.rs` (وحدة الاختبار)، `crates/engines/tests/{office_engine.rs,pdf_ghostscript.rs}`، وربما `.cargo/config.toml` (AC2)، `docs/engines.md`، `CHANGELOG.md` |
| S-026b | وظيفتا CI، وسكربت ISO، و`run-tests.cmd` | AC8 إلى AC11 | `.github/workflows/ci.yml`، `scripts/windows/{make-iso.sh,run-tests.cmd,README.md}`، `CHANGELOG.md` |

## معايير القبول
S-026a:
- AC1: `scripts/windows/build.sh` خارج نطاق يعيد تشغيل نفسه داخل `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200%`، وداخل CI (`CI=true`) يعمل مباشرة. يطبع أول سطر `cgroup: <المسار>`، وقيمة `memory.max` لذلك المسار تساوي `6442450944`. يبني بـ `-j 2` و`CARGO_TARGET_DIR=/home/sbarah/.cache/sbarhive-win-target` افتراضياً، وينتج `x86_64-pc-windows-gnu/release/sbarhive.exe`، ويخرج بـ 0.
- AC2: `scripts/windows/check-dlls.sh <exe>` يقرأ أسطر `DLL Name` من `x86_64-w64-mingw32-objdump -p`، ويقبل `--text <file>` للاختبار. `objdump-ok.txt` (مكتبات النظام وحدها) يعطي 0 وسطر `dll check: ok <name> <N> imports`. و`objdump-mingw.txt` (فيه `libwinpthread-1.dll` و`libgcc_s_seh-1.dll`) يعطي 1 ويسرد الاسمين. أي اسم يطابق `^lib.*\.dll$` دون حساسية للحالة مرفوض. على `sbarhive.exe` الحقيقي يعطي 0، وإن لم يعطه يُضاف `.cargo/config.toml` بـ `-C target-feature=+crt-static` للهدف وحده، ويُسجَّل ذلك هنا.
- AC3: `scripts/windows/check.sh` داخل النطاق نفسه يشغّل `cargo clippy -j 2 --target x86_64-pc-windows-gnu -p sbarhive-core -p sbarhive-engines -p sbarhive-cli --all-targets -- -D warnings` ويخرج بـ 0.
- AC4: اختبارات Unix وحدها تُحصر بـ `cfg(unix)` تهيئةً لا تغييراً في التأكيدات: على مستوى الملف `crates/cli/tests/cli.rs` و`settings.rs` (خوادم `UnixListener` و`PermissionsExt`)، وعلى مستوى الدالة اختبار الرابط المعلّق في `core/tests/output.rs:108`، و`socket_path_lives_in_the_runtime_dir` في `core/tests/contracts.rs:228`، واختبار `PermissionsExt` في `engines/src/office.rs`، واختبارات البرامج الوهمية بـ `sh` في `office_engine.rs` و`pdf_ghostscript.rs`. كل سطر مضاف في `git diff` لهذه الملفات سطر `cfg`، ولا سطر محذوف. وعدد الاختبارات الناجحة في `cargo test --workspace` على لينكس قبل التعديل وبعده متساوٍ.
- AC5: `cargo test --no-run -j 2 --target x86_64-pc-windows-gnu -p sbarhive-core -p sbarhive-engines -p sbarhive-cli` داخل النطاق ينجح، وعدد ملفات الاختبار `.exe` التي يسردها يساوي عدد أهداف الاختبار في الحزم الثلاث (يُسجَّل العدد هنا).
- AC6: لا حزمة جديدة: عدد `[[package]]` في `Cargo.lock` يبقى 671، ولا تغيير في `Cargo.lock` في هذه القصة (`git diff --quiet -- Cargo.lock`).
- AC7: `docs/engines.md` يسجّل أدوات البناء التي لا تُشحن: mingw64-gcc 16.2.1 وgenisoimage (مثبتان)، ورخصتيهما. `scripts/windows/README.md` يشرح الأوامر بالعربية والإنكليزية.

S-026b:
- AC8: `ci.yml` يضيف وظيفتين ولا يغيّر الوظيفة `check`: `git diff -U0 main...HEAD -- .github/workflows/ci.yml` فيه أسطر `+` فقط. `windows-cross` على `ubuntu-24.04` تثبّت `mingw-w64` وهدف `x86_64-pc-windows-gnu` ثم تشغّل `check.sh` و`build.sh` و`check-dlls.sh`. `windows-native` على `windows-latest` تثبّت FFmpeg (`choco install ffmpeg -y`)، لأن `ffmpeg_engine.rs:123` يؤكد وجوده ولا يتخطاه، ثم تشغّل `cargo test -p sbarhive-core -p sbarhive-engines -p sbarhive-cli`. اختبارات LibreOffice وGhostscript تتخطى نفسها حين يغيب البرنامج (`office_engine.rs:37`، `pdf_ghostscript.rs:91`)، فلا تثبيت لهما.
- AC9: `scripts/windows/make-iso.sh` يبني بـ `genisoimage -J -R` صورة `/home/sbarah/.cache/sbarhive-win-target/sbarhive-win-tests.iso` فيها `bin/sbarhive.exe`، وملفات الاختبار في `tests/`، و`run-tests.cmd`، و`SHA256SUMS`. `isoinfo -J -f -i <iso>` يسرد كل هذه، وعدد أسطر `SHA256SUMS` يساوي عدد الملفات التنفيذية، و`sha256sum` لـ `run-tests.cmd` المستخرج بـ `isoinfo -J -x` يساوي الأصل.
- AC10: `run-tests.cmd` بنهايات CRLF (`file` يقول `with CRLF line terminators`). يشغّل كل `tests\*.exe` بـ `--test-threads=1`، ويكتب في `%USERPROFILE%\sbarhive-tests\results.txt` سطراً `<exe> <errorlevel>` لكل ملف، وآخر سطر `total N failed F`، والمخرج الكامل في `log.txt` بجانبه. لا يكتب على القرص المضغوط.
- AC11: `scripts/windows/README.md` يضيف بالعربية والإنكليزية أمر الإرفاق `virsh -c qemu:///session attach-disk win11 <iso> sdb --type cdrom --mode readonly` دون تشغيله في هذه القصة.

لكل قصة فرعية:
- سطر واحد للقصة كلها في `CHANGELOG.md` تحت `[Unreleased]`، بالإنكليزية مع نص عربي بين «»: تضيفه S-026a، وS-026b تعدّل السطر نفسه.
- `git diff --stat main...HEAD -- crates/ui` فارغ، و`C` بلا تحذير على لينكس.
- `scripts/design-lint.sh` نظيف على الملفات الجديدة (لا شرطة طويلة).
- لا شيفرة Rust تنفيذية جديدة هنا (أسطر `cfg` في الاختبارات فقط)، فلا طفرات.

## أوامر الفحص
أدوات القراءة وحدها (`grep` و`file` و`objdump` و`isoinfo` و`sha256sum`) تعمل خارج النطاق، و`make-iso.sh` لا يبني شيئاً.
- `C`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% sh -c 'cargo fmt --check && cargo clippy -j 2 --workspace --all-targets -- -D warnings'`
- `L`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% cargo test -j 2 --workspace`
- `S`: `scripts/windows/build.sh` ثم `scripts/windows/check-dlls.sh /home/sbarah/.cache/sbarhive-win-target/x86_64-pc-windows-gnu/release/sbarhive.exe` ثم `scripts/windows/check.sh` (كلها تدخل النطاق بنفسها)
- `T`: `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% env CARGO_TARGET_DIR=/home/sbarah/.cache/sbarhive-win-target cargo test --no-run -j 2 --target x86_64-pc-windows-gnu -p sbarhive-core -p sbarhive-engines -p sbarhive-cli`

## حالات الاختبار
| معيار | الاختبار | النوع |
| --- | --- | --- |
| AC1 | `S`: سطر `cgroup:` و`cat /sys/fs/cgroup<المسار>/memory.max` يعطي 6442450944، ووجود `sbarhive.exe` | بناء |
| AC2 | `check-dlls.sh --text` على الملفين المزروعين برمزي خروج 0 و1 ونصيهما، ثم على `sbarhive.exe` | تكامل |
| AC3 | `check.sh` يخرج بـ 0 | بناء |
| AC4 | `git diff` للملفات السبعة أسطر `cfg` مضافة فقط؛ وعدد `test result: ok` في `L` قبل وبعد | مراجعة، وحدة |
| AC5 | `T` وعدّ ملفات `.exe` | بناء |
| AC6 | `grep -c '^\[\[package\]\]' Cargo.lock` يعطي 671، و`git diff --quiet -- Cargo.lock` | مراجعة |
| AC8 | أمر `git diff -U0` المذكور، والمراجع يقرأ الوظيفتين | مراجعة |
| AC9 | `make-iso.sh` ثم `isoinfo -J -f` و`isoinfo -J -x` و`sha256sum` | تكامل |
| AC10 | `file scripts/windows/run-tests.cmd`، والمراجع يقرأ الملف | مراجعة |
| AC7، AC11 | المراجع يقرأ engines.md وREADME وCHANGELOG.md | مراجعة |

## تحقق حي مؤجَّل إلى S-034
- أول نجاح للوظيفتين في CI: `ci.yml` لا يعمل إلا عند الدفع إلى main أو طلب دمج، والدفع والدمج خارج هذه القصة ويحتاجان قرار محمد.
- `run-tests.cmd` في الآلة: الملف والسطر الأخير، وعدد الإخفاقات.
- إرفاق ISO بالآلة وقراءته من محرك الأقراص.

## خارج النطاق
- `sbarhived.exe` وفحص مكتباته بـ `check-dlls.sh`: S-027 (الخدمة تعتمد على ui).
- المثبّت والملف المضغوط المحمول: S-033.
- wine، وأداة الضيف `qemu-guest-agent`: يحتاجان موافقة محمد.

## سجل المراجعة
- 2026-10-22، المخطِّط: ADR يطلب فحص `objdump` على `sbarhived.exe` في أول قصة بناء، لكنه لا يُربط قبل S-027، فيُفحص هنا `sbarhive.exe` ويُضاف `sbarhived.exe` في S-027. وقائمة «ما يحتاجه المالك» تطلب الإذن بـ xorriso أو genisoimage، لكن genisoimage وisoinfo مثبتان على المضيف، فلا يلزم إذن لهذه القصة.
- 2026-10-22, tester (S-026a):
  - Tests, all shell, outside the cargo workspace so `cargo test` on real Windows (S-026b) never runs bash and the reviewer's `L` starts no nested build:
    - `tests/windows/test_scripts.sh` (light, no real build, a few seconds): `ac1_build_sh_reexecs_in_a_6g_scope_with_j2_and_prints_its_cgroup`, `ac2_check_dlls_accepts_system_dlls_and_rejects_lib_dlls`, `ac3_check_sh_runs_windows_clippy_all_targets_in_a_6g_scope`, `ac4_unix_only_tests_carry_cfg_unix_and_nothing_else_changes`, `ac7_engines_md_and_readme_document_the_build_tools`, `no_long_dash_and_shell_lint_for_the_new_scripts` (`bash -n`, `shellcheck`, no em or en dash in the three scripts and the README).
    - `tests/windows/test_heavy.sh` (the real `S` and `T`; refuses to start while `pgrep` sees cargo or cargo-mutants): `ac1_real_build_sh_runs_in_a_6g_scope_and_produces_sbarhive_exe`, `ac2_check_dlls_passes_on_the_real_sbarhive_exe`, `ac3_real_check_sh_exits_0`, `ac5_windows_test_binaries_build_one_exe_per_test_target`.
    - `tests/windows/lib.sh` (helpers, sources `tests/integrations/lib.sh` for the asserts), fixtures `tests/windows/fixtures/objdump-ok.txt` and `objdump-mingw.txt`, trimmed from the real `x86_64-w64-mingw32-objdump -p` of the current `sbarhive.exe` (header, Import Tables, the trailing empty entry; the path on the first line is cut to `sbarhive.exe`). `objdump-ok.txt` has 9 distinct system DLLs; `objdump-mingw.txt` is the same plus `libgcc_s_seh-1.dll` and `libwinpthread-1.dll`.
    - Both scripts must run from outside any memory scope (a plain terminal or the Claude app scope, whose `memory.max` is `max`).
  - Contract pinned by the light tests (the implementer reads this, not the tests, to know what is checked):
    - AC1 and AC3: the scripts call `cargo` from `PATH` (a fake `cargo` first on `PATH` logs each call's argv, its cgroup, that cgroup's `memory.max`, `CARGO_TARGET_DIR` and `CARGO_BUILD_JOBS`). The scope must keep `PATH` (a `systemd-run --scope` child inherits the environment). Two jobs are accepted as `-j 2`, `-j2`, `--jobs 2`, `--jobs=2` or `CARGO_BUILD_JOBS=2`. The build call has `build`, `--release` and `--target x86_64-pc-windows-gnu` (or `--target=`). A `CARGO_TARGET_DIR` from the caller is kept, and with it unset the build uses `/home/sbarah/.cache/sbarhive-win-target` (env or `--target-dir`, both read). The `cgroup:` line names the cgroup the cargo call runs in, with `memory.max` 6442450944, and it is not the caller's cgroup. With `CI=true`, `systemd-run` is never called (a fake that exits 99 is first on `PATH`) and the `cgroup:` line is the caller's own. AC3 checks `clippy` with `-p sbarhive-core -p sbarhive-engines -p sbarhive-cli --all-targets` and `-- -D warnings` in the 6G cgroup.
    - AC2: `--text FILE` on `objdump-ok.txt` exits 0 and prints the line `dll check: ok <name> 9 imports`, where `<name>` may be `sbarhive.exe` (from the objdump header), `objdump-ok.txt` or the path given, and N counts `DLL Name` lines (DLLs, not functions). `objdump-mingw.txt` exits 1, names both DLLs, prints no `dll check: ok`, and does not name `kernel32` or `msvcrt`. A generated copy with `DLL Name: LIBSTDC++-6.DLL` exits 1 and names it (case-insensitive rule). Without `--text`, the script runs `x86_64-w64-mingw32-objdump -p <exe>` from `PATH` (a fake that prints `objdump-mingw.txt` gives exit 1). The heavy test on the real exe asks only for exit 0 and the line shape, because the real exe imports `ntdll.dll` twice and `KERNEL32.dll` beside `kernel32.dll`.
    - AC4: inner `#![cfg(unix)]` lines in `crates/cli/tests/cli.rs` and `settings.rs`; `#[cfg(unix)]` among the attributes directly above `ac7_dangling_symlink_counts_as_taken` (output.rs), `socket_path_lives_in_the_runtime_dir` (contracts.rs), `office_find_program_respects_name_order_and_exec_bit` (office.rs unit tests), the four `office_engine.rs` tests that run `/bin/sh` as the program (`office_timeout_kills_the_process_group`, `office_cancel_kills_the_process_group`, `office_already_cancelled_starts_nothing` through `fake_engine`, and `office_missing_program_is_unavailable_error`), and `pdf_ghostscript_identity_and_capabilities` (`with_program("/bin/sh")`). `git diff -U0 5be5270` of the seven files has no `-` line, and every `+` line is a `#[cfg(...)]`, `#![cfg(...)]` or `#[cfg_attr(...)]` line, so helpers left unused on Windows may also get a cfg line.
    - AC5 (heavy): T exits 0, and the number of distinct `(... .exe)` paths it prints equals the lib, bin and test targets with `test = true` in the three packages from `cargo metadata --no-deps`: today 35 (core 17, engines 13, cli 5). `cli.rs` and `settings.rs` under `#![cfg(unix)]` still build an empty test exe, so 35 holds.
    - AC1 heavy: the first stdout line is read from a fifo and `memory.max` read at once, because the scope's cgroup is removed when the build ends; `file` must say `PE32+ executable ... x86-64` for the exe.
  - The cfg(unix) edits in the seven files are left to the implementer. The story names them in the S-026a file list without giving them to the tester, and AC4 is how they are checked. Adding them here would turn AC4, and maybe AC5, green before implementation. The manager must give the implementer explicit leave for these cfg lines only, since the chain forbids the implementer to edit tests.
  - Ambiguities and choices:
    - (1) "N imports" is read as the count of `DLL Name` lines; the ok fixture has no duplicates, so raw and distinct counts agree there.
    - (2) `<name>` with `--text` is open (see AC2 above).
    - (3) For office_engine.rs and pdf_ghostscript.rs, the AC's "fake programs with sh" is pinned to the five tests above, the ones that pass `/bin/sh` as the program. `office_run_child_*` in office.rs are not required to be guarded, because the AC names only the `PermissionsExt` test there.
    - (4) The AC1 "outside a scope" detection is the implementer's choice (env marker or memory.max); only the observable result is tested.
    - (5) AC6 already holds today (671 `[[package]]`, `Cargo.lock` clean), so a test for it would pass before implementation. AC6 is left to the reviewer: `grep -c '^\[\[package\]\]' Cargo.lock` and `git diff --quiet 5be5270 -- Cargo.lock`. The same goes for the CHANGELOG line, `git diff --stat main...HEAD -- crates/ui`, `C` and design-lint.
    - (6) AC7 is review in the table. The light test adds only a static floor: engines.md has a line with `mingw64-gcc`, `16.2.1` and `GPL`, and one with `genisoimage` and `GPL`. The README exists, names the three scripts, and has Arabic and English text.
  - AC4 baseline for the equal pass count in `L`: 697 passed, 0 failed, 1 ignored at `d6779fb` (S-025b reviewer). `d6779fb..5be5270` touches only the backlog, and this commit adds no Rust, so the count holds before the implementer's cfg lines.
  - Logic check: in a throwaway `git clone --shared` in the session scratch dir (deleted, not merged), I wrote stub scripts and added the cfg lines. With them the light suite passed (63 checks). Mutating the stub made it fail at the expected checks: dropping `-j 2`; skipping the scope in check.sh (memory.max `max`); dropping `<name>`; changing an assertion in contracts.rs, which gave a `-` and a non-cfg `+` line. The heavy suite was not run against the stub (see below).
  - Failing output (light suite, here, before implementation):
```
$ bash tests/windows/test_scripts.sh
== AC1: build.sh outside a scope
  FAIL: AC1: missing or not executable scripts/windows/build.sh
== AC2: check-dlls.sh
  FAIL: AC2: missing or not executable scripts/windows/check-dlls.sh
== AC3: check.sh
  FAIL: AC3: missing or not executable scripts/windows/check.sh
== AC4: cfg(unix) guards
  FAIL: AC4: crates/cli/tests/cli.rs starts with the inner attribute #![cfg(unix)]
  FAIL: AC4: crates/cli/tests/settings.rs starts with the inner attribute #![cfg(unix)]
  FAIL: AC4: output.rs ac7_dangling_symlink_counts_as_taken has #[cfg(unix)]
  FAIL: AC4: contracts.rs socket_path_lives_in_the_runtime_dir has #[cfg(unix)]
  FAIL: AC4: office.rs office_find_program_respects_name_order_and_exec_bit has #[cfg(unix)]
  FAIL: AC4: office_engine.rs office_timeout_kills_the_process_group (a /bin/sh fake program) has #[cfg(unix)]
  FAIL: AC4: office_engine.rs office_cancel_kills_the_process_group (a /bin/sh fake program) has #[cfg(unix)]
  FAIL: AC4: office_engine.rs office_already_cancelled_starts_nothing (a /bin/sh fake program) has #[cfg(unix)]
  FAIL: AC4: office_engine.rs office_missing_program_is_unavailable_error (a /bin/sh fake program) has #[cfg(unix)]
  FAIL: AC4: pdf_ghostscript.rs pdf_ghostscript_identity_and_capabilities (/bin/sh) has #[cfg(unix)]
== AC7: docs
  FAIL: AC7: engines.md names mingw64-gcc 16.2.1 with its licence
  FAIL: AC7: engines.md names genisoimage with its licence
  FAIL: AC7: missing scripts/windows/README.md
== AC1 AC2 AC3: bash -n, shellcheck, no long dash
  FAIL: lint: missing scripts/windows/build.sh
  FAIL: lint: missing scripts/windows/check-dlls.sh
  FAIL: lint: missing scripts/windows/check.sh
FAIL S-026a windows scripts (light) (19 of 21 checks failed)
```
