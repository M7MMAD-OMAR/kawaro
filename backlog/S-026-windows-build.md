# S-026 البناء المتقاطع لويندوز وبوابته وCI وصورة الاختبار
الحالة: S-026a: منتهية (المراجع: PASS، 2026-10-22)؛ S-026b لم تبدأ (فرع windows/0.6.0 بطلب محمد، 2026-10-22)؛ ADR 0005 ما زال مقترحاً، ولا دمج في main

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
  - Heavy suite, here, before implementation. Ran when no other cargo was running. T fails at the first crate, and cargo prints no `Executable` line on failure, hence 0 listed:
```
$ bash tests/windows/test_heavy.sh
== AC1: real build.sh (S)
  FAIL: AC1: missing or not executable scripts/windows/build.sh
== AC2: check-dlls.sh on the real sbarhive.exe
  FAIL: AC2: missing or not executable scripts/windows/check-dlls.sh
== AC3: real check.sh
  FAIL: AC3: missing or not executable scripts/windows/check.sh
== AC5: cargo test --no-run for Windows (T)
  FAIL: AC5: exit code: got [101] want [0]
  AC5: test targets 35, listed .exe files 0
  FAIL: AC5: listed .exe files equal test targets: got [0] want [35]
      1 error: could not compile `sbarhive-cli` (test "settings") due to 1 previous error
      1 error[E0433]: cannot find `unix` in `os`
FAIL S-026a windows build (heavy) (5 of 6 checks failed)
```
  - BLOCKER for the manager: the AC4 file list does not cover every Windows compile error under `--all-targets`. Full list from `cargo check --tests --keep-going -j 2 --target x86_64-pc-windows-gnu -p sbarhive-core -p sbarhive-engines -p sbarhive-cli`, inside the scope with the Windows target dir:
    - Listed in AC4: `cli/tests/cli.rs` (3 errors), `cli/tests/settings.rs` (1), `core/tests/output.rs` (1), `engines/src/office.rs` unit tests (3).
    - NOT listed:
      - `crates/core/tests/transport.rs`, 4 errors: `Endpoint::from_path` at lines 35, 77 and 272, and `std::os::unix` at line 267.
      - `crates/core/src/output_impl.rs:401`, 1 error: `std::os::unix` inside its `#[cfg(test)] mod tests`.
    - AC5 and AC3 (clippy `--all-targets`) cannot pass without cfg lines in these two files too. The implementer may not edit tests, and these two are outside the story. So the story needs its AC4 list (and the S-026a file list) extended before implementation; I did not extend the AC4 test on my own.
- 2026-10-22, implementer (S-026a):
  - Manager decisions, recorded as given:
    1. "You are allowed to add cfg lines only (`#[cfg(...)]`, `#![cfg(...)]`, `#[cfg_attr(...)]`, no deleted lines, no changed assertion) to the seven test files AC4 lists. This is the configuration the story describes, not a test edit."
    2. "The S-026a file list and AC4's scope are extended with crates/core/tests/transport.rs (unix-only tests using Endpoint::from_path and std::os::unix) and the test module of crates/core/src/output_impl.rs (line ~401), under the same cfg-lines-only rule, so AC3 and AC5 can pass. The AC4 test itself covers the seven files; the reviewer checks the two extra files by diff."
  - Scripts: `scripts/windows/build.sh` and `check.sh` re-exec through `systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200% env SBARHIVE_WIN_SCOPE=1 "$0"` unless `CI=true` or that marker is set, print `cgroup: <path>` from `/proc/self/cgroup`, default `CARGO_TARGET_DIR` to `/home/sbarah/.cache/sbarhive-win-target` and keep a caller's, then exec `cargo build -j 2 --release --target x86_64-pc-windows-gnu -p sbarhive-cli` or the AC3 clippy line. `check-dlls.sh` counts `DLL Name` lines (raw, not distinct), names the file by its basename, and rejects `^lib.*\.dll$` case-insensitive with exit 1; usage or objdump failure is exit 2. `scripts/windows/README.md` in English and Arabic.
  - AC2 on the real exe: `dll check: ok sbarhive.exe 11 imports`, exit 0. No mingw DLL, so no `.cargo/config.toml` and no `crt-static`.
  - AC4 cfg lines beyond the tester's named tests, all cfg lines only, needed by Windows clippy `-D warnings` once the guarded tests drop out:
    - `office_engine.rs`: `#[cfg(unix)]` on `fake_engine`, `read_pid`, `wait_gone` and `use std::ffi::OsString;`; `#[cfg_attr(not(unix), allow(unused_imports))]` on `use std::path::{Path, PathBuf};`.
    - `pdf_ghostscript.rs`: `#[cfg_attr(not(unix), allow(unused_imports))]` on the `sbarhive_core` use line (`MediaKind`).
    - `transport.rs` (decision 2): `#[cfg(unix)]` on `ep`, `listen`, `read_line`, the `Path`, `thread` and `time` imports, and on every test that uses `Endpoint::from_path` or a socket (`ac1_endpoint_from_path_displays_the_path`, `ac1_ac3_server_pid_is_none_on_unix`, `ac3_*`, `ac4_*`, `ac6_*`), plus `ac1_endpoint_on_unix_is_the_socket_path`, which compiles on Windows but asserts the Unix socket path and would fail on real Windows in S-026b; `#[cfg_attr(not(unix), allow(unused_imports))]` on the three grouped use lines. Left on Windows: `ac1_stream_is_read_write_send_and_listener_is_send`, `ac1_socket_name_and_socket_path_keep_their_value` and the three `ac2_pipe_name*` tests.
    - `output_impl.rs` (decision 2): `#[cfg(unix)]` on `ac7_linux_place_refuses_a_dangling_link_name`.
    - `git diff -U0 5be5270` of all nine files: no `-` line, every `+` line is a cfg line.
  - Note for S-026b: `processes_mentioning` in `office_engine.rs` reads `/proc`; it compiles on Windows and is reached only by `office_cancel_stops_real_libreoffice_and_cleans_up`, which skips itself without LibreOffice.
  - AC5 count: 35 test executables for 35 test targets (core 17, engines 13, cli 5).
  - Results: light suite `PASS (63 checks)`; heavy suite `PASS (11 checks)` in 56 s, run from the app scope (memory.max `max`) with no other cargo; `cargo fmt --check` and Linux `cargo clippy -j 2 --workspace --all-targets -- -D warnings` clean; `cargo test -j 2 --workspace` 697 passed, 0 failed, 1 ignored (equal to the baseline); `shellcheck scripts/windows/*.sh` clean; `scripts/design-lint.sh` clean; AC6 671 `[[package]]`, `Cargo.lock` unchanged; `crates/ui` untouched.
  - One Windows clippy attempt overlapped with a `cargo build --release` another session started at the same moment; it failed on a real warning and was re-run alone. Every later run, the heavy suite and the Linux gates started with `pgrep -x cargo` empty.
- 2026-10-22, reviewer (S-026a): **PASS**.
  - Light suite `tests/windows/test_scripts.sh`: PASS (63 checks). Heavy suite `tests/windows/test_heavy.sh` from the app scope (memory.max `max`), `pgrep -x cargo` empty: PASS (11 checks), incremental (0.95 s, the implementer's artifacts were fingerprinted fresh). AC5: 35 test exe files for 35 test targets.
  - AC1 run by hand through a fifo: first line `cgroup: .../app.slice/run-p3239487-i20022953.scope`, its `memory.max` 6442450944 and `cpu.max` `200000 100000`; exit 0; `file` says `PE32+ executable ... x86-64`.
  - AC2 on the real exe: `dll check: ok sbarhive.exe 11 imports`, exit 0 (no `lib*.dll`, so no `.cargo/config.toml` is right). `objdump-mingw.txt`: exit 1 with both DLLs. AC3 `check.sh`: exit 0 in its own 6G scope.
  - `cargo fmt --check` and `cargo clippy -j 2 --workspace --all-targets -- -D warnings`: clean. `cargo test -j 2 --all`: 697 passed, 0 failed, 1 ignored in 45 suites, equal to the baseline (AC4).
  - `git diff -U0 5be5270` of the nine files: 0 deleted lines, 35 added lines, all cfg lines (26 `#[cfg(unix)]`, 2 indented `#[cfg(unix)]`, 2 `#![cfg(unix)]`, 5 `#[cfg_attr(not(unix), allow(unused_imports))]`). `cargo mutants --list --in-diff` on the crates diff: 0 mutants (cfg lines only).
  - AC6: 671 `[[package]]`, `Cargo.lock` identical to `5be5270` (it differs from main only through S-022b, S-024 and S-025a). `crates/ui` diff against main empty. `tests/windows/*` unchanged since e6fd108. `shellcheck scripts/windows/*.sh` and `scripts/design-lint.sh` clean; no em or en dash in `main...HEAD`.
  - Scripts: re-exec through `systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200%` unless `CI=true` or the `SBARHIVE_WIN_SCOPE` marker; `"$0" "$@"` quoted; `-j 2`; caller `CARGO_TARGET_DIR` kept. CHANGELOG line present (added by the implementer, not duplicated).
  - Deviations accepted: extra cfg and `cfg_attr` lines on helpers and imports, the guarded `ac1_endpoint_on_unix_is_the_socket_path` (asserts the Unix socket path), and no `.cargo/config.toml`; all inside the manager's cfg-lines-only decisions.
  - Findings, none blocking:
    - Low, `docs/engines.md:285`: `x86_64-w64-mingw32-objdump` is called the tool of mingw64-gcc, but `rpm -qf` gives mingw64-binutils 2.46.0 (GPL-3.0-or-later and others), which is not recorded. Fix in S-026b, which must record Ubuntu's `mingw-w64` anyway; add `docs/engines.md` to its file list.
    - Low, `scripts/windows/check-dlls.sh:12`: `--text` on a missing file exits 1 (the rejection code), while the header at line 8 says input errors are exit 2.
    - Low, `scripts/windows/check-dlls.sh:20-28`: input with no `DLL Name` line prints `ok <name> 0 imports` and exits 0, so the gate fails open on unparseable output; worth a guard before CI (S-026b) uses another objdump build.
    - Info, `scripts/windows/build.sh:9`, `check.sh:8`: a caller with `SBARHIVE_WIN_SCOPE` already set skips the scope.
    - Info, `scripts/windows/build.sh:14-15`, `check.sh:13-14`: a relative caller `CARGO_TARGET_DIR` is resolved after the `cd` to the repo root.
    - Info for S-026b: with `CI=true` the default target dir is still `/home/sbarah/.cache/...`, which a GitHub runner cannot create; the CI job should set `CARGO_TARGET_DIR`.
