# S-102 Alt drag: doctor line and the Alt tap evaluation
Status: draft

## Story
As a Hyprland user, I want `kawaro doctor` to say whether the drop key bind and the daemon are fine and what the last hold did, and I want a written evaluation of opening the hive from a bare Alt tap, so that a failed Alt drag is never silent and the tap question has an answer.

## Acceptance criteria
- AC1 (from S-095 AC6): `kawaro doctor` prints one line `drop key: <key> bind <present|missing>, daemon <up|down>, last hold: <outcome> <n> ms ago` using `kawaro_core::hold::doctor_line`. The bind presence and the last outcome need a real source (for example the daemon answering a status request); a test per state checks the exact line.
- AC2 (from S-095 AC6): the outcomes `daemon-down` and `no-action-file` are emitted as `hold: <reason>` lines, with a test each. `tap`, `no-drag`, `timeout` and `release-lost` already are.
- AC3 (S-095 AC8, S-095c): a written evaluation in `docs/drop-shortcut.md` (English) and `docs/drop-shortcut.ar.md` for (a) clipboard after copy, (b) `kawaro-open-selection`, (c) AT-SPI selection, stating what was measured, that a Wayland client cannot read a file manager selection directly, and one recommendation. No code ships for it unless it shows 0 side effects in the S-095 AC7 checks.
- AC5 (found by the S-095 second review): an Alt chord with a key (Alt+a, Alt+Tab, Alt+F4) never shows a hive. Today Hyprland does not fire the Alt release bind when another key came between, so the gate stays in Timing and shows the hive 400 ms later with no Alt held, up to `hold_max_ms`. The fix needs a measure first: either a bind or flag that still reports the release, or the daemon cancels a pending press on any other key or on a window focus change. Harness: Alt+a and Alt+Tab, then 1.5 s: 0 `hive surface` lines and gate Idle.
- AC4: one `CHANGELOG.md` line in English and Arabic; the fast gates pass; no long dash.

## Out of scope
The gate and the re-show logic (S-095b), KDE and GNOME changes, X11, Windows.
