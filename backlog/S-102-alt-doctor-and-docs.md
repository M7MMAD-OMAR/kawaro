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

## Review log
- 2026-10-30, tester: tests written from AC1, AC2, AC3 and AC5.
  - Measured in the private Hyprland (`repro.sh --altdrag layer`, release build at 05:45, ALTN=2, CPUQuota=100%): after Alt+a and after Alt+Tab, 1.5 s later there are 5 `hive surface` lines each (the stale press shows at hold_ms, then the 4 pointer re-shows of S-095), 0 `hold: tap` or `hold: chord` lines, so the gate is not Idle: the daemon log shows `hold: no-drag` at the end of the re-shows and `hold: tap` only at the next plain Alt tap. Expected 0 lines and Idle. (The same run also showed other FAILs in S-021 checks and S-095 AC2 alt-first and Esc, with ALTN=2 on a loaded machine; not analysed, not part of this story.)
  - New tests: `crates/core/tests/s102_chord_cancel.rs` (AC5 unit: `Signal::OtherKey`, `Outcome::Chord`; the contract is in its header), `tests/hyprland/altdrag.sh` (AC5 harness: Alt+a and Alt+Tab, 1.5 s, 0 `hive surface` lines and a `hold: chord` or `hold: tap` line), `crates/cli/tests/s102_doctor_hold.rs` (AC1 doctor line in the no daemon state, AC2 `hold: daemon-down` from `drop --hold`), `crates/core/tests/s102_docs.rs` (AC3).
  - Failing output: `cargo test -p kawaro-core --test s102_chord_cancel`: 9 compile errors (E0599 no `Signal::OtherKey`, no `Outcome::Chord`). `--test s102_docs`: 2 failed (`no heading with Alt tap`; no Arabic heading with Alt and النقرة), 1 passes as a guard (language switch and no long dash). `-p kawaro-cli --test s102_doctor_hold`: 2 failed (doctor prints no `drop key:` line; `drop --hold` with no daemon prints `kawaro: cannot start kawarod` and no `hold: daemon-down`). Harness: `FAIL S-102 AC5 (Alt+a)` and `(Alt+Tab)`, surface 5, ended 0.
  - Not written: the `no-action-file` emission (AC2) and the doctor line states with a running daemon (AC1: bind present, daemon up, last hold n ms ago) need a status source and a trigger the story does not define; the implementer adds those tests with the source. Alt+F4 is not in the harness (it would close the test client).
