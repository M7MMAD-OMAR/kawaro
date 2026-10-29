# S-099 Update experience: progress, what is new, restart and reopen
Status: approved (2026-10-29, manager)

## Story
As a user updating Kawaro from the settings window, I want to see my version, the new version, a download percent, a clear finished and installing message, and a short list of what is new, and I want the app to restart and reopen settings by itself showing "Updated to X", so that I never wonder what is happening or reopen anything by hand.

## Why
Owner requirements of 2026-10-29. Today the state line shows 0% for the whole download (S-090 limit, S-091), there is no installing state, the app restarts silently, and the Windows helper reports success when `msiexec` returned 0 even if `kawarod.exe` is gone (the S-097 failure). S-091 (percent and size over IPC, 1000 ms poll) is kept as A1, A2, B3. S-092 (tray About with version, Check, Settings) is kept as B7, but opens the S-094 settings window on the Updates page instead of a new popup window, so no new window type exists.

## S-099a Core and IPC
- A1 (IPC shape): `Response::Update` gains `percent: Option<u8>`, `size: Option<u64>` and `notes: Option<Notes>` (`Notes { en: Vec<String>, ar: Vec<String> }`, the notes of `latest`), each `#[serde(default, skip_serializing_if = "Option::is_none")]`. A reply with all three `None` is byte for byte the current JSON. A downloading reply is `{"status":"update","state":"downloading","current":"0.4.2","latest":"0.4.3","percent":42,"size":52428800}`. An old client parses a new reply; a new client parses an old reply with three `None`.
- A2 (percent): `percent = progress::percent(part_len, size)`: (0,1000)=0, (420,1000)=42, (1000,1000)=100, (5000,1000)=100, (10,0)=0, missing part file=0. Only `downloading` carries `percent` and `size`; every other state has both `None`.
- A3 (installing word): a new state word `installing` is reported from the start of `apply` until the daemon exits or the install fails (then `failed`). `ready` means downloaded and verified, so it is the "download finished" signal; it is never reported before size and SHA-256 matched.
- A4 (manifest notes): schema 1 gains an optional `notes {en, ar}`. Valid means 3 to 5 lines per language, each non empty, one line, at most 80 characters. Invalid or absent notes become empty and never reject the manifest or hide the update. `Update` carries `notes`. `scripts/make-update-manifest.sh` adds `notes` from `docs/whats-new/<version>.json` before signing.
- A5 (bundled notes): `docs/whats-new/<version>.json` (`{"en":[...],"ar":[...]}`) is embedded by `crates/core/build.rs` for `CARGO_PKG_VERSION`. `whats_new::installed(lang)` returns its lines with zero network or fetcher calls. A test fails the build gate when the workspace version has no file or it breaks the A4 rule.
- A6 (helper recovery, S-097 regression): `HelperOps` gains `daemon_present() -> bool`. After `Installed` (exit 0 or 3010), `run_helper` checks it. If false it runs `msiexec /fa <msi> /qn /norestart /l*v <log>` once (repair, `msiexec_repair_args`), checks again, and if still false writes `Failed(Text("kawarod.exe is missing after the update. Run <msi path> by hand."))` and does not start a daemon. Failed, cancelled and busy codes skip the check (the MSI rolled back). At most one repair.
- A7: fmt, clippy, tests, `design-lint.sh`, mutants, one CHANGELOG line.

## S-099b UI, restart and reopen
- B1 (messages, Western digits): downloading `Downloading 0.4.3: 42% (12.0 of 50.0 MB)` / `جارٍ تنزيل 0.4.3: 42% (12.0 من 50.0 ميغابايت)` (size in MiB, 1 decimal, omitted when `size` is `None`); ready `Download finished. Ready to install 0.4.3.` / `اكتمل التنزيل. التحديث 0.4.3 جاهز للتثبيت.`; installing `Installing 0.4.3. Kawaro will restart by itself.` / `جارٍ تثبيت 0.4.3. سيعيد كوارو التشغيل تلقائياً.`; updated `Updated to 0.4.3` / `تم التحديث إلى 0.4.3`. `UpdateState` gains `Installing` and `Updated(String)`; `Downloading(u8)` is unchanged and the size is a model field. While Downloading or Installing, Check and Update now are disabled.
- B2 (versions): the Updates page always shows `Installed version: X`; in downloading, ready and installing it also shows `Available version: Y`; in other states the available line is absent.
- B3 (poll): in Downloading or Installing the window sends `UpdateCheck` every 1000 ms (a busy reply fetches nothing), stops on any other state and on window close.
- B4 (What is new): a card titled `What is new in X` / `الجديد في X` with 3 to 5 bullets: the installed version from A5 always (offline); when an update is known, a second card for the available version from `notes` (omitted when empty). Reserved height of title plus 5 lines: starting a check moves every row below by 0 px; no scrollbar, no text longer than the A4 limit is drawn. Accessible label on each card.
- B5 (restart and reopen): when Update now is pressed with the settings window visible, the daemon writes `update-reopen.json` (`{from,to,at}`, atomic) next to `config.toml` before installing. The next daemon start reads and deletes it: if the installed version equals `to` and `at` is under 3600 s old it opens the settings window on the Updates page showing `Updated to X` and the new version's notes; otherwise it opens nothing. The banner clears on the next check or window close. Linux uses `restart_daemon`; Windows uses the helper's `start_daemon`. The user presses nothing after Update now.
- B6 (no background focus): the marker is written only from that click. Tray Update now with settings closed, auto install and background ticks write none and open no window (window opens by an update tick: 0).
- B7 (tray About, cuttable to S-099c if the day is exceeded): the Linux tray menu gains `About` / `حول كوارو` after Get started (7 entries) and `Command::About` after `GetStarted` on Windows; both call `open_settings` on the Updates page. No new window.
- B8 (labels, gates): every string in Arabic and English with RTL mirroring, labelled controls, tokens only, no shadows, no long dashes. Screenshots in `docs/screenshots/s099/` via `crates/ui/tests/updates_screenshots.sh`: downloading 42%, ready, installing, updated with notes, each in en and ar, light and dark (16 images). Windows capture and live upgrade: owner pending (S-079 checklist).

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| A1 | Serialize downloading reply, compare the exact JSON above; serialize up-to-date, compare to today's string; parse old string, assert three `None`; parse new string into a struct with the old fields only | unit (core) |
| A2 | `percent` table above; temp part file of 420 bytes with size 1000 gives 42 through the daemon reply path; the six other statuses give `None`, `None` | unit (core, daemon) |
| A3 | Fake apply: reply is `installing` while the install closure runs, `failed` after it errors; `ready` is never returned with a part file present or a wrong hash | unit (daemon) |
| A4 | Manifest with 4 en and 4 ar lines parses, `Update.notes` equal; 2 lines, 6 lines, a 81 character line, an empty line and a missing key each give empty notes and the update is still `Available`; schema 2 is still `UnsupportedSchema` | unit (core) |
| A5 | `installed(En)` and `installed(Ar)` equal the 3 to 5 lines of the file with a fetcher that panics on any call; a temp tree without the version's file fails the gate test; make-update-manifest on a fixture carries the same lines | unit, script |
| A6 | Fake ops, exit 0 and `daemon_present` false then true: calls are wait, install, repair, write, start(New), result `Installed`. False twice: result `Failed(Text)` containing `kawarod.exe` and the msi path, no `start_daemon`, exactly 2 `run_msiexec` calls. Exit 3010 behaves like 0. Exit 1603, 1602, 1618: 1 call, no presence check. Present at once: 1 call | unit (core) |
| B1 | `update_state_line()` equals each exact string above in en and ar; with `size` `None` the parenthesis is absent; check and apply disabled in Downloading and Installing | ui (testing backend) |
| B2 | Version line equals the package version; available line present in the three states and absent in the others | ui |
| B3 | Fake clock: three polls 1000 ms apart in Downloading, none after `ready` or close | integration (ui) |
| B4 | Offline fake IPC: card shows the bundled bullets (3 to 5); with `notes` in the reply a second card shows its bullets; y of the row below is equal before and after a check | ui, integration |
| B5 | Pure `reopen_decision(marker, installed, now)`: matching version and 10 s old gives Updated(to); 3601 s old, other version, no file and a corrupt file give None and the file is gone; write then read round trip. Fake daemon start with a marker opens the window once on the Updates page with the banner | unit, integration (daemon) |
| B6 | Update now with settings hidden writes no marker; tick that finds a version: window-open spy 0, notification spy 1 | integration (daemon) |
| B7 | Linux menu has `About` at index 2 (en, ar) and 7 entries; `shell_logic::menu` has `Command::About` after `GetStarted`; handler calls the `open_settings` spy once | unit (daemon) |
| B8 | Strings non empty and differ per language; RTL snapshot mirrors; 16 screenshots viewed by the reviewer; fmt, clippy, tests, design-lint | snapshot, gate |

## Mutants plan
Per changed file, never the whole tree: `cargo mutants --in-place --in-diff <diff of the file>` for `ipc.rs`, `manifest.rs`, `flow.rs`, `apply.rs`, `crates/ui/src/updates.rs`, `daemon/src/update.rs`, `ui.rs`. Put the commands in a script and run it as `systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200%` with `CARGO_BUILD_JOBS=2`. `apply/windows.rs` stays excluded by `.cargo/mutants.toml`, so A6 lives in the pure `run_helper`. No surviving mutant in changed code.

## Out of scope
Full release notes viewer, download speed and time left, skip this version, a push channel (polling is enough), a new window or popup, notes fetched from anywhere except the signed manifest, macOS. On GNOME the reopened window may open without focus (no activation token); real GNOME and KDE focus stays with S-082.

## CHANGELOG plan
- Update experience (S-099): settings show the versions, the real download percent, finished and installing messages and a short "What is new", and the app restarts and reopens settings with "Updated to X". The Windows helper now repairs once or reports a clear message if `kawarod.exe` is missing. «تجربة التحديث: تعرض الإعدادات النسختين ونسبة التنزيل ورسائل الاكتمال والتثبيت وقسم «الجديد»، ويعيد التطبيق التشغيل ويفتح الإعدادات مع «تم التحديث إلى X»؛ ويصلح مساعد ويندوز مرة واحدة أو يعرض رسالة واضحة إذا غاب kawarod.exe».

## Review log
-
