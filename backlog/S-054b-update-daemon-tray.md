# S-054b Daemon: scheduled checks, tray item, tooltip and notifications
Status: approved (manager, by delegation, 2026-10-24)

Second surface story of ADR 0006. Depends on S-053b and S-054a. Touches `crates/core/src/ipc.rs`, `crates/daemon` and the tray strings; no `crates/ui` window, and no window that takes focus. The Windows tray is S-048 and gets the same interface later. About one work day.

## Story
As a user, I want the app to check for updates by itself and show a calm "Update now" in the tray, so that updating takes one click and never interrupts my work.

## Acceptance criteria
- AC1 (`ipc.rs`): `Request::UpdateCheck` and `Request::UpdateApply` serialise to `{"cmd":"update-check"}` and `{"cmd":"update-apply"}`; `Response::Update { status, current, latest }` serialises with `"status":"update"`; all round trip, and `Ping` is unchanged.
- AC2 (`crates/daemon/src/update.rs`): `tick(&mut UpdateState, &Config, &dyn Fetcher, now)` is the whole scheduler step. Over a simulated 48 h with a tick every hour and a fetcher that always has no update: 2 checks (4 fetch calls) at hour 0 and hour 24; with `update_check = false`: 0 calls. With a fetcher that is offline: calls follow the 1 h, 2 h, 4 h backoff of S-053a, not every tick.
- AC3 (`daemon/main.rs`): the update thread starts after the socket is bound and sleeps `FIRST_CHECK_DELAY = 30 s` before its first tick; startup is not blocked: the existing hive latency test still passes with the thread present, and `main` never calls `check`.
- AC4 (`update.rs`): when `check` returns `Available` and `download_verified` succeeds, the state is `Ready(version)`; a hash mismatch gives `Failed(version)` and no file remains. In `ask` mode on Linux and Windows the daemon never calls the apply function without an `UpdateApply` request; in `auto` mode on Windows it calls it only when `can_apply(0, false)`.
- AC5 (`tray.rs`, `text.rs`): `menu_labels(lang, &UpdateState) -> Vec<String>` has "Update now" (Arabic «حدّث الآن») as the first item, enabled only in `Ready`; in other states the item is disabled. `tooltip_text` shows `Downloading update 42%` (Arabic «تنزيل التحديث 42٪») while downloading, with the percent rounded from the downloaded bytes over `size`, and the idle text otherwise. Both strings exist in Arabic and English, and the menu entry has a descriptive label.
- AC6 (`notify.rs`): `notify_update(lang, kind, version)` with kind `Ready | Updated | Failed` logs one `sbarhived: notify:` line and shows one notification; a second call with the same kind and version does nothing (0 lines). `Updated` or `Failed` is sent at start from `take_result()` of S-054a, once.
- AC7 (`update.rs`): `UpdateApply` with state not `Ready` answers `Response::Error` whose message contains `no verified update`, and starts nothing. With running jobs it answers `Ok` and defers; the deferred apply runs on the first tick with `can_apply` true.
- AC8 (gates): fmt, clippy, tests, `design-lint.sh`, mutants clean on changed files, one CHANGELOG line, English and Arabic; a screenshot of the Linux tray menu with the item enabled, in both languages.

## Test cases
The scheduler uses an injected clock and the fake `Fetcher` of S-053b, so no test waits in real time. The notification and apply functions are injected recorders.
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | The exact JSON strings and the round trip | unit |
| AC2 | 48 simulated ticks with three fetchers; assert the call counts and the tick numbers of the calls | unit |
| AC3 | A test that the first tick time is 30 s after start; the latency test file runs unchanged | unit, performance |
| AC4 | Fixture packages good and tampered; recorder shows 0 apply calls in `ask`, 1 in `auto` only when idle, 0 while a job runs | unit |
| AC5 | The label lists in both languages compared to expected text; the tooltip at 0, 42 and 100 percent | unit |
| AC6 | Call twice, count log lines (1) and recorder shows (1); a different version sends again | unit |
| AC7 | Request in `Idle` state: the error text; request with 1 running job, then 0 | integration |
| AC8 | Tray screenshot in a private compositor | snapshot |

## Out of scope
- Settings window rows for the three keys (ui work, later); the keys are edited in `config.toml`.
- The Windows tray menu itself (S-048).
- A "skip this version" button.

## CHANGELOG plan
- Update in the tray (S-054b): the app checks once a day, downloads in the background, and offers "Update now" in the tray with one notification. «التحديث من الدرج: يفحص التطبيق مرة في اليوم وينزّل في الخلفية ويعرض «حدّث الآن» في الدرج مع إشعار واحد».

## Review log
- 2026-10-24, planner: written from ADR 0006; approved by the manager under the maintainer's delegation. Notifications are counted through the existing `sbarhived: notify:` line.
