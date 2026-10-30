# S-108 Hive cells for unsupported files (S-096b)
Status: approved (2026-10-30, manager)

## Story
As a user who selects files Kawaro cannot fully convert, I want the hive to show a reason line, one greyed cell per skipped file and a request cell, so that I see what is skipped and can ask for the format with one click.

## Scope
Only what S-096 AC2 to AC9 already specify; no new behaviour. `kawaro_core::unsupported` (done) is the single source of every string. Two small additions it needs: `skipped_label(ext, reason, lang)` (accessible text, `.xyz: unavailable. <reason>` / Arabic) and `more_label(k, lang)` (`K more files are not supported` / Arabic). Where S-096 is silent, the decisions below were made to keep every criterion testable; the manager confirms them at approval.

## Design seams (so nothing needs a compositor)
- `daemon::server::plan(outcome, files, state) -> HivePlan { centre, message, suggestion, cells, pending }` is pure; `open` calls it and then `ui::present`. Used on every OS, so the Windows `shell_open::selection` path gets the Mixed split too (S-096 review note).
- `HiveCell` gains `enabled: bool` and `reason: String` (a mechanical update of the existing struct literals in `ui_tests.rs`, `drop_hive_tests.rs` and `ui.rs` is allowed). `HiveWindow` gains `centre-message` and `centre-suggestion` string properties, drawn as their own elements with ids `HiveWindow::message` and `HiveWindow::suggestion`.
- Cell ids: real action id; `skipped:<n>`; `more`; `request:<ext>` (ext already sanitised by `request_url` rules). Pure `choice(id) -> Choice { Run(id) | Request(url) | Ignore }`.
- The opener is the existing module: `notify_logic::open_url_command(os, url)` next to `open_folder_command` (Linux `xdg-open <url>`, Windows `explorer.exe <url>`), started by `notify::open_folder`. `xdg-open` stays spelled only in `notify_logic.rs`. The choose handler takes an injectable `Fn(&str)` so a test counts calls.
- Decisions: the REQ cell carries the extension of the first skipped file (uniform for Unsupported, MissingEngine and Mixed). Each skipped file gets its own reason (classify that one file). Total cells are capped at 18 (the hive limit): real actions are never dropped; greyed cells shrink first (all collapse into one `+K` cell when room is short); REQ is omitted only when real actions plus one `+K` cell already fill 18.

## Acceptance criteria
- AC1 (centre message, S-096 AC2): `plan` for `[a.xyz]` gives `centre = "File .xyz"` and `message = "This format is not supported yet"` (en), `ملف ‎.xyz` and `هذه الصيغة غير مدعومة بعد` (ar), as two separate values with no newline in either. The window shows `message` in `HiveWindow::message` (text equals the value, accessible label non empty); with an empty message that element count is 0. The daemon no longer joins them (`format!("{}\n{message}"` count 0 in `server.rs`).
- AC2 (zero side effects, S-096 AC3): building `plan` and presenting a MissingEngine selection makes 0 opener calls and 0 `notify::open_folder` calls (counter before and after), and the message holds the tool name exactly once. Source scan: `core/src/unsupported.rs` and the `plan` module contain 0 of `Command`, `std::process`, `std::net`, `TcpStream`, `UdpSocket`, `reqwest`.
- AC3 (greyed cells, S-096 AC4): table over selections, assert the exact cell list: `[a.png,b.png,c.xyz]` gives the real actions plus 1 skipped cell with code `XYZ`; 6 skipped files give 6 cells and no `+K`; 7 give 6 plus `+1`; 9 give 6 plus `+3`; a file with no extension gives code `?`. Invariant for every row: skipped cells represented (6 or fewer, plus the `+K` count) plus supported files equals the selection size. Cap rows: 10 real actions with 9 skipped files give at most 18 cells total with all 10 real actions present and `+K` counts the rest.
- AC4 (not activatable): in the headless window, choosing a skipped or `+K` cell by click and by Return fires `choose` 0 times and `choice` returns `Ignore`; the `on_choose` log stays empty. Highlighting it (pointer move) sets the centre text to its `reason` exactly. Choosing a real action in `[a.png,b.png,c.xyz]` submits a job whose files equal `[a.png, b.png]` (`pending`), and `c.xyz` is in no job.
- AC5 (REQ cell, S-096 AC5): every plan for Unsupported, MissingEngine and Mixed has exactly 1 cell with code `REQ`, label `Request this format` / `اطلب هذه الصيغة`, and id `request:xyz` for `a.xyz`; Supported plans have 0. Its hover text equals `request_hover(lang)` in both languages and contains `telemetry`/`بيانات` wording from core. Opener spy: 0 calls after `plan` and present; click on the cell gives exactly 1 call whose argument equals `request_url("xyz")`; Return on it, 1 more call (2 total); clicking a skipped cell adds 0. `open_url_command` table: Linux gives `("xdg-open", [url])`, Windows `("explorer.exe", [url])`, URL kept as one argument (query string with `&` intact). `s049_sources.rs` still passes.
- AC6 (suggestion, S-096 AC7): `plan` for a `.heic`-family stand-in with an image action and engine on gives `suggestion = Some(<that label>)` and `HiveWindow::suggestion` count 1 with that text; `.xyz`, family present with engine off, and `Other` family each give count 0 (never a generic line).
- AC7 (tokens, S-096 AC8): `scripts/design-lint.sh` passes. Source scan of `hex-cell.slint` and `hive.slint`: the greyed branch fills with `Tokens.surface-2` and inks with `Tokens.text-secondary` (or the generated equivalents), contains 0 `opacity` on that branch, 0 `#` colour literals, 0 `drop-shadow`, 0 `@linear-gradient`. A greyed cell never takes the accent fill when highlighted. `HiveCell.code` of every skipped and REQ cell has 16 characters or fewer and holds no space (code only, no names inside cells).
- AC8 (access, S-096 AC8): for each of Unsupported, MissingEngine, Mixed and each of en and ar, every cell has a non empty accessible label (found through `ElementHandle`, role button); a skipped cell's label equals `skipped_label(...)` and contains `unavailable` (en) or `غير متاح` (ar) plus its reason; the message element label is non empty. Both new core strings exist in en and ar with equal keys, Western digits only, 0 U+2013 or U+2014 in changed files.
- AC9 (reduced motion, S-096 AC8): with reduced motion on (as in `s017_motion_tests.rs`), after present and 0 ms of mocked time, every new cell has opacity 1 and scale 1 (0 animated frames); with it off, the same cells start at opacity 0 and reach 1 only after `hive-open` plus `ring * ring-delay`.
- AC10 (budget, unit part): `plan` for 1000 paths (500 supported, 500 skipped) takes 20 ms or less in a plain `cargo test` run; the request-to-present path adds 0 blocking calls (no file I/O in `plan`, source scan for `std::fs`, 0).
- AC11: no changed file contains an em or en dash; `CHANGELOG.md` already has the S-096 line, extended by one clause in English and Arabic.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | `plan` strings exact in en and ar; headless window: element `HiveWindow::message` text and count 1 or 0; grep `server.rs` | unit, interface |
| AC2 | counters around `plan` and a missing engine `open`; source scan list | unit |
| AC3 | 8 row table above, exact `Vec<HiveCell>` ids and codes, size invariant | unit |
| AC4 | headless click, Return, hover; queue receives `[a.png, b.png]` | interface |
| AC5 | table of 3 outcomes x 2 languages; spy opener 0, 1, 2 calls; `open_url_command` table | unit, interface |
| AC6 | 4 fixtures (family on, `.xyz`, engine off, `Other`) | unit, interface |
| AC7 | `design-lint.sh`; source scans | unit |
| AC8 | 3 outcomes x 2 languages label sweep | interface |
| AC9 | mocked time on/off | interface |
| AC10 | timed `plan`; source scan | unit, performance |

## Manual release step (private compositor, NOT a gate for the fast loop)
Run once before a release by the maintainer or manager; findings become fix stories, never block a push.
- M1: 12 screenshots (Unsupported, MissingEngine, Mixed x ar/en x light/dark) compared to baselines and checked by eye for RTL order, clipped text, no text inside small cells (private compositor via `tests/hyprland/repro.sh`, `Hive::take_snapshot` hook).
- M2: hive presented within 150 ms of the request for an unsupported selection, 3 trials, worst 150 ms or less (`KAWARO_DEBUG_FOCUS` timing line).
- M3: Windows mixed selection through Explorer shows the same greyed cells (owner pending).

## Out of scope
New formats or engines, auto install, sending a request without a click, reading file names or content, telemetry, changing strings already in `kawaro_core::unsupported`, the engine contract.

## Review log
- 2026-10-30, planner: made concrete from S-096 AC2 to AC9 and its review log; seams chosen from `server.rs`, `ui.rs`, `hive.slint`, `notify_logic.rs`. Waiting for the manager's approval. If a working day is exceeded, split after AC5 (model and opener) from AC6 to AC10 (window properties).
- 2026-10-30, manager: approved with the seven decisions (REQ uses the first skipped file's extension and also appears in Mixed; per-file reasons; 18 cell cap; `?` for no extension; `skipped_label` and `more_label`; tokens `Tokens.surface-2` and `Tokens.text-secondary`, both confirmed to exist in the generated tokens; the mechanical `HiveCell` literal update is for the implementer). Split: part A is AC1 to AC5 (pure `plan`, cell model, `choice`, opener); part B is AC6 to AC10 (window properties, a later round).
- 2026-10-30, tester (part A): tests written from AC1 to AC5 plus the unit parts of AC8 (strings) and AC10 (budget, purity scan). Files: `crates/core/tests/s108_strings.rs`, `crates/core/tests/s108_sources.rs`, `crates/daemon/src/s108_tests.rs` (registered in `main.rs`), `crates/ui/src/s108_tests.rs` (registered in `lib.rs`). Not written yet (part B): the `HiveWindow::message` and `HiveWindow::suggestion` elements (AC1 window half, AC6), token scans (AC7), the accessibility sweep (AC8), reduced motion (AC9), and the click-on-REQ interface test is covered through `on_choice` at daemon level.
  Hooks the implementer must provide:
  - core `unsupported`: `skipped_label(ext, reason, lang) -> String` (en exactly `.xyz: unavailable. <reason>`, ar holds ext, `غير متاح`, reason) and `more_label(k, lang) -> String` (en exactly `K more files are not supported`, ar holds K in Western digits).
  - daemon: new `crates/daemon/src/plan.rs` (add `mod plan;` to `main.rs`, re-export from `server`; pure, no `Command`, `std::process`, `std::net`, `std::fs`): `plan(&Outcome, &[PathBuf], &State) -> HivePlan { centre, message, suggestion: Option<String>, cells: Vec<HiveCell>, pending: Option<Vec<PathBuf>> }`; `enum Choice { Run(String), Request(String), Ignore }` and `choice(&str) -> Choice`; `on_choice(&str, &dyn Fn(&str)) -> Choice` (one opener call for `Request`, none otherwise; the `on_choose` handler in `ui.rs` goes through it and submits only on `Run`). Cell ids `skipped:<n>`, `more`, `request:<ext>`; codes `XYZ`/`?`, `+K`, `REQ`; real actions first is not pinned, only that none is dropped and the total is 18 or less. `server::open` and `explain` call `plan` and no longer `format!("{}\n{message}")`.
  - daemon `notify_logic`: `open_url_command(Os, &str) -> (String, Vec<String>)` (`xdg-open` / `explorer.exe`, URL one argument).
  - ui: `HiveCell` gains `enabled: bool` and `reason: String`; `CellData` carries both; `HiveWindow` gets `out property <string> centre-shown` (centre text when no highlight, the cell's `reason` when it has one, else its `label`); a disabled cell never fires `choose` by click, Return or Space and does not dismiss.
  Existing tests the implementer must update mechanically (new `HiveCell` fields `enabled: true, reason: String::new()`), not touched by the tester: `crates/ui/src/ui_tests.rs` (`cells()` and any literal), `crates/ui/src/drop_hive_tests.rs` (`cells()`), the `cell_carries_the_action_fields_in_the_chosen_language` test and `cell()` in `crates/daemon/src/ui.rs`, and any other `HiveCell {` literal (`grep -rn "HiveCell {" crates`).
  Failure output (fast loop, 6 GB scope, nice 15, one job, 2 test threads):
  - `cargo test -p kawaro-core --test s108_strings`: `error[E0432]: unresolved imports kawaro_core::unsupported::more_label, kawaro_core::unsupported::skipped_label`.
  - `cargo test -p kawaro-core --test s108_sources`: 3 tests, 3 FAILED at run time: `ac1_server_does_not_join_centre_and_message` (left: 1, right: 0), `ac2_ac10_plan_and_unsupported_are_pure` (`crates/daemon/src/plan.rs: No such file or directory`), `ac5_open_url_command_lives_next_to_open_folder_command` (`assertion failed: logic.contains("fn open_url_command")`).
  - `cargo test -p kawaro-ui --lib s108`: does not compile: `E0560` no field `enabled`/`reason` on `HiveCell`, `E0599`/`E0609` no `get_centre_shown` and no `enabled`/`reason` on the window model.
  - `cargo test -p kawaro-daemon --bin kawarod s108`: does not compile: `E0432`/`E0433` (no `crate::plan`, no `notify_logic::open_url_command`, no `skipped_label`/`more_label`) and `E0609` no field `enabled` or `reason` on `HiveCell`.
  All fail for the missing hooks above, none for a test mistake.
