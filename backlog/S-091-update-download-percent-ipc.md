# S-091 The daemon reports the real download percent to the UI
Status: absorbed by S-099 (2026-10-29), do not implement separately

## Story
As a user watching an update download in the settings window, I want the state line to show the real percent and size, so that I can see the download move instead of a fixed 0%.

## Why this story exists
S-090 review (2026-10-28, point 4) accepted a declared limit: `Response::Update` carries only `state`, `current` and `latest`, so `state_from_status("downloading", ..)` in `crates/ui/src/updates.rs` returns `Downloading(0)` and the S-090 state line shows 0% for the whole download. The daemon already knows the number: `progress::shared().snapshot(&update_sys::part_len)` gives `Phase::Downloading { downloaded, size }` and `progress::percent(done, size)` is what the tray tooltip uses. Only the IPC reply and the UI mapping are missing.

## Ordering
Implement after S-090 passes review, and before S-092. S-091 and S-092 both edit `crates/core/src/ipc.rs` and the daemon (`update.rs`, `ui.rs`, `tray/`), so they must be built one after the other, never in parallel.

## Acceptance criteria
- AC1 (IPC shape): `Response::Update` gains two fields, `percent: Option<u8>` and `size: Option<u64>` (total bytes from the signed manifest), both `#[serde(default, skip_serializing_if = "Option::is_none")]`. JSON for a non-downloading reply is byte for byte what it is today (`{"status":"update","state":"up-to-date","current":"0.4.0","latest":null}`); a downloading reply is `{"status":"update","state":"downloading","current":"0.4.0","latest":null,"percent":42,"size":52428800}`. An old client that ignores unknown keys still parses it; a new client reading an old reply gets `None` for both.
- AC2 (computation): `percent` is `progress::percent(downloaded, size)` where `downloaded` is the length of the `.part` file read at the moment of the request (`update_sys::part_len`), the same function the tray tooltip uses. Values: 0 bytes of 1000 gives 0; 420 of 1000 gives 42; 1000 of 1000 gives 100; part file missing gives 0; size 0 gives 0; downloaded above size is capped at 100.
- AC3 (where it is set): only the `downloading` state carries `percent` and `size`. Every other state (`up-to-date`, `ready`, `failed`, `offline`, `unavailable`, `rejected`) has both `None`. The reply for `UpdateCheck` while the update lock is held (`check_or_busy`, `None` branch) is the one that fills them.
- AC4 (UI mapping): `state_from_status` takes the percent; `("downloading", Some(42))` gives `Downloading(42)`, `("downloading", None)` gives `Downloading(0)`. The daemon wiring in `crates/daemon/src/ui.rs` passes `percent` from the response.
- AC5 (state line): the S-090 settings state line shows the number: English `Downloading update 42%`, Arabic `جارٍ تنزيل التحديث 42%` (Western digits), with the size appended as `(12.0 of 50.0 MB)` when `size` is known. While the state is `Downloading`, the window re-sends `UpdateCheck` every 1000 ms (a busy reply fetches nothing) until the state changes, and stops when the window closes.
- AC6 (gates): fmt, clippy, tests, `design-lint.sh`, mutants clean on changed files, one CHANGELOG line in English and Arabic. No new window and no focus change.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Serialize a downloading reply with percent 42 and size 52428800 and compare the exact JSON string; serialize an up-to-date reply and compare to the old exact string; deserialize the old string and assert `percent == None` and `size == None`; deserialize the new string into a struct with the old three fields only (serde ignore) and assert it parses | unit (core) |
| AC2 | `progress::percent` table: (0,1000)=0, (420,1000)=42, (1000,1000)=100, (5000,1000)=100, (10,0)=0; with a temp part file of 420 bytes and size 1000 the snapshot-to-reply path yields 42; with the file absent it yields 0 | unit (daemon) |
| AC3 | `check_or_busy(None, ..)` with an active `progress` entry returns `state == "downloading"`, `percent == Some(42)`, `size == Some(1000)`; each of the six other statuses from `check_now` returns `None` and `None` | unit (daemon) |
| AC4 | `state_from_status("downloading", Some(42))` equals `Downloading(42)`; with `None` equals `Downloading(0)`; "ready" with `Some(50)` still gives `Ready` | unit (ui) |
| AC5 | Set the view to `Downloading(42)` with size 52428800: `update_state_line()` equals the exact English and Arabic strings above; fake clock: three polls 1000 ms apart send three `UpdateCheck` requests, and a `ready` reply stops further polls | integration (ui) |
| AC6 | Reviewer runs the gates | gate |

## Out of scope
- A new request type or a push channel; polling `UpdateCheck` is enough.
- Download speed and time remaining.
- The tray tooltip (it already shows the percent).

## CHANGELOG plan
- Real download percent in settings (S-091): the daemon now reports percent and size, and the settings state line shows them. «نسبة التنزيل الفعلية في الإعدادات: تُرسل الخدمة النسبة والحجم ويعرضهما سطر الحالة».

## Review log
-
