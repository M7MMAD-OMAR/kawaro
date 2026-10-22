# S-027 Keep the layer hive off the keyboard at KWin level 4 without a valid token
Status: tests written (2026-10-22), waiting for the implementer

`tests/overlay/focus.sh` fails `AC10 level 4 notoken` and `AC10 level 4 bogus` (S-013 AC4, S-018 AC10): at KWin focus stealing prevention level 4, pointer still, the daemon logs `hive focus: true`. Evidence and the suspected cause (`fcb5a18`, not bisected) are in S-014e1 review 2, `backlog/S-014-drop-hive.md`.

## Story
As a KDE user with focus stealing prevention at Extreme, I want the hive to take my keyboard only with a valid activation token, so that my policy holds, while Esc and the arrow keys keep working with a token or at lower levels.

## Planner decisions (for the manager)
1. **Behaviour:** at level 4 with no token or a bogus one, pointer still, the hive does not take the keyboard. Extreme is the user's choice, and ADR 0004 section 3 records it. S-013 AC4 stands.
2. **Fix:** a normal layer show requests `Exclusive` before the catcher's first commit, never on the mapped catcher (`fcb5a18` moved it into `decide`). Drop shows keep `None`. Rejected: tying `Exclusive` to the token, because tokenless shows on Hyprland and at KWin levels 0 to 3 need the keyboard, and KWin treats `on_demand` like `exclusive`.
3. **Accepted trade-off:** the catcher holds the keyboard for up to `ANCHOR_WAIT` (50 ms) before the first frame on normal shows. The early return in `decide` (`no_drag`) is drop only, so no-freeze (S-019) is not at risk.
4. **Cause first, one rule:** the tester runs AC1 before writing any test. If the first failing commit is not `fcb5a18`, the tester records it in the review log and stops. The story returns to the manager with no fix and no pin.
5. **Levels 0 to 3:** levels 2 and 3 become hard checks, overriding "measured only" in S-018 AC10. The last pass (`tests/overlay/out/run-focus.txt`, 2026-10-20 23:18) shows focus true at both. `0:notoken` joins the default cases.
6. **A gate fails with the fix** (for example `repro.sh`): normal REWORK, and no compositor-specific branch without a new manager decision.

## Acceptance criteria
- AC1: `git archive` exports `fcb5a18^` and `fcb5a18` to temporary directories outside the repo, with no worktree and no branch. Each is built with its own `CARGO_TARGET_DIR` (6G scope, `-j 1`). HEAD's `focus.sh 4:notoken 4:bogus 1:notoken <bin>` runs twice per build. Expected: the parent passes all 3 cases twice, and `fcb5a18` fails both level 4 cases twice. If the parent fails too, the other `layer.rs` commits in `49bfbcb..f42dd04` are tested until the first failing one is found. Hashes, run counts and the PASS and FAIL lines go in the review log.
- AC2 (pin): in every `focus.sh` case, the first `set_keyboard_interactivity` on the measured show's layer surface has value 1 and comes before the catcher's first `wl_surface.commit`. None follows that commit. This check fails on HEAD.
- AC3: at level 4, for `notoken` and `bogus`, the show is a layer surface, with no `wl_keyboard.enter` on the catcher, no `hive focus: true`, and `focus=false` in the TABLE line. The bogus case still sends `activate("sbarhive-bogus-token", <catcher>)`.
- AC4: at levels 0 to 3 with `notoken`, `wl_keyboard.enter` reaches the catcher within 500 ms of `get_layer_surface`, plus `hive focus: true`. At levels 1 and 4 with `token`, `activate(<client token>, <catcher>)`, plus `hive focus: true`.
- AC5: in every AC4 case, Esc (evdev 1) is injected through `kwin-input` while `client.py` runs, and the catcher's `zwlr_layer_surface_v1.destroy` follows within 1000 ms.
- AC6: these end with `RESULT PASS`, run one at a time: `focus.sh` with the default cases `0:notoken 1:token 1:notoken 2:notoken 3:notoken 4:token 4:notoken 4:bogus`, `tests/overlay/kwin.sh`, `tests/drop/kwin.sh level=1`, `tests/hyprland/repro.sh --json` (failed 0, skipped 0), `tests/drop/window_path.sh` and `tests/drop/portal.sh`. The layer path median from `crates/ui/tests/measure.sh` is under 150 ms.
- AC7: the CLAUDE.md quality gates pass. ADR 0004 section 3 gets one bullet with the rule from decision 2 and the AC1 cause, labelled the ADR's way: the bisect is "verified by running", and the KWin mechanism is "assumed" unless read in source. `CHANGELOG.md` gets one line in English and Arabic.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Two builds, the `focus.sh` lines compared with the expectation | integration (diagnostic) |
| AC2 | New `focus.sh` check on the order and values in the `WAYLAND_DEBUG` trace | integration |
| AC3 | `focus.sh` level 4 checks, plus `$enter` empty and `focus=false` | integration |
| AC4 | `focus.sh` hard checks for levels 0, 2 and 3, plus the token checks | integration |
| AC5 | New `focus.sh` Esc step, time from Esc to `destroy` | integration |
| AC6 | The six scripts' final `RESULT` lines, and the `measure.sh` median | integration, performance |
| AC7 | The command gates, plus the reviewer reading the ADR and CHANGELOG | review |

## Out of scope
- The window path (GNOME, X11, screen reader) and `tests/focus/kwin.sh`.
- Focus at Extreme after pointer motion, and Esc not closing the hive at level 4.
- `.claude/worktrees/` and the branch `windows/0.6.0`.

## Review log
- 2026-10-22, planner: draft, for the manager's approval.
- 2026-10-22, manager under the maintainer's delegation: approved with the planner's six decisions. The tester starts after the S-014e2 tester finishes, so only one heavy private compositor job runs at a time.
- 2026-10-22, tester: AC1 confirms the cause, then the AC2 to AC5 checks in `tests/overlay/focus.sh` (the only file changed besides this one; no helper and no crate source touched).
  - AC1, verified by running: `fcb5a18^` (`02c56cb`) and `fcb5a18` (`fcb5a18`) exported with `git archive` to the session scratchpad (no worktree, no branch), each built with its own `CARGO_TARGET_DIR` in the 6G scope with `-j 1`, and HEAD's (`c5e5995`) `focus.sh 4:notoken 4:bogus 1:notoken <bin>` run twice per build, one private KWin at a time, output to the scratchpad.
    - `02c56cb`, runs 1 and 2: all 3 cases `RESULT PASS`; TABLE `level=4 case=notoken focus=false enter_ms=none`, `level=4 case=bogus focus=false enter_ms=none`, `level=1 case=notoken focus=true enter_ms=4` both times. The level 1 control prints `hive focus: true`, so the level 4 pass is not vacuous. No config warning in the daemon logs (it ignores `portal_shortcut`), 0 `xdg-desktop-portal` lines in every bus log.
      The parent's check lines, identical in runs 1 and 2:
      ```
      PASS AC10 level 4 notoken: the measured show is a layer surface
      PASS AC10 level 4 notoken, pointer still: the hive does not take the keyboard (no hive focus: true)
      PASS AC10 level 4 bogus: the measured show is a layer surface
      PASS AC10 level 4 bogus, pointer still: the hive does not take the keyboard (no hive focus: true)
      PASS AC10 level 4 bogus: activate("sbarhive-bogus-token", <catcher>) still sent
      PASS AC10 level 1 notoken: the measured show is a layer surface
      PASS AC10 level 1 notoken: wl_keyboard.enter on the catcher within 500 ms of get_layer_surface
      PASS AC10 level 1 notoken: hive focus: true
      ```
    - `fcb5a18`, runs 1 and 2: `FAIL AC10 level 4 notoken, pointer still: the hive does not take the keyboard (no hive focus: true)` and `FAIL AC10 level 4 bogus, pointer still: ...`, both times; `PASS` on every other check, `1:notoken` `RESULT PASS`; TABLE `focus=true` at level 4 for both cases.
    - Mechanism in the traces of the measured show: in `02c56cb` the first `set_keyboard_interactivity(1)` on the layer surface is at line 6 of the show and the catcher's first `wl_surface.commit` at line 12, with none after; in `fcb5a18` (and HEAD) the first commit is at line 11 and `set_keyboard_interactivity(1)` follows at line 27. So `fcb5a18` is the first failing commit; the other `layer.rs` commits in `49bfbcb..f42dd04` were not searched, as the story allows.
  - Contract (what the implementer must make true; check names start with `S-027`):
    - AC2, every case: in the measured show's trace, from the catcher's `wl_compositor.create_surface` (or `get_layer_surface`) to that layer surface's `destroy`, the first `zwlr_layer_surface_v1.set_keyboard_interactivity` is `(1)` and its line comes before the catcher's first `wl_surface.commit`; no `set_keyboard_interactivity` on that layer surface after that commit. Order is by line, not stamp.
    - AC3, `4:notoken` and `4:bogus`: layer surface, `$enter` empty (no `wl_keyboard.enter` on the catcher), no `hive focus: true`, `focus=false` in the TABLE line; bogus still sends `activate("sbarhive-bogus-token", <catcher>)` (existing check kept).
    - AC4: `[0-3]:notoken` hard checks, enter within 500 ms of `get_layer_surface` and `hive focus: true` (levels 2 and 3 are no longer "measured only", decision 5); `1:token` and `4:token` keep the existing token checks.
    - AC5, the AC4 cases only: after the show, with `client.py` still running (`kill -0`), Esc (evdev 1) goes through `kwin-input`; the injection time is taken with `date -u` (the WAYLAND_DEBUG stamps are UTC wall time: the marker came 0.27 to 0.62 ms before the daemon's `wl_keyboard.key(..., 1, 1)` in every case), the trace is read 1.5 s later, and the layer surface's `destroy` must follow within 1000 ms. No Esc in `4:notoken` and `4:bogus` (out of scope).
    - AC6 default cases: `0:notoken 1:token 1:notoken 2:notoken 3:notoken 4:token 4:notoken 4:bogus`.
  - Failing output on HEAD (`c5e5995`, rebuilt in the scope after `git status crates/` was clean, default cases, output to the scratchpad): 29 PASS, 22 FAIL, every case `RESULT FAIL`, final `RESULT FAIL`.
    - Both AC2 checks fail in all 8 cases (16 FAIL), for the expected reason: `first set_keyboard_interactivity(1) at line 27, catcher's first commit at line 11, set_keyboard_interactivity after that commit: 1`.
    - `4:notoken` and `4:bogus` fail `AC10 ... no hive focus: true`, `S-027 AC3 ... no wl_keyboard.enter on the catcher` and `S-027 AC3 ... focus=false in the TABLE line` (6 FAIL; `RESULT FAIL (5)` each with AC2); the bogus activate check passes.
    - The `4:notoken` block, verbatim:
      ```
      FAIL S-027 AC2 level 4 notoken: the first set_keyboard_interactivity on the layer surface is 1 and comes before the catcher's first commit
      FAIL S-027 AC2 level 4 notoken: no set_keyboard_interactivity on the layer surface after the catcher's first commit
      PASS AC10 level 4 notoken: the measured show is a layer surface
      FAIL AC10 level 4 notoken, pointer still: the hive does not take the keyboard (no hive focus: true)
      FAIL S-027 AC3 level 4 notoken, pointer still: no wl_keyboard.enter on the catcher
      FAIL S-027 AC3 level 4 notoken, pointer still: focus=false in the TABLE line
      RESULT FAIL (5)
      ```
    - AC4 and AC5 pass on HEAD by design: they guard behaviour the fix must keep. Esc to destroy was 0 to 1 ms in all 6 AC5 cases, enter 2 to 5 ms. Each guard was shown to fail by running the new parser offline on the `02c56cb` level 4 notoken trace labelled `3:notoken` with an Esc marker: both AC4 checks and the AC5 check FAIL. The AC2 parser passes on the `02c56cb` traces and fails on the `fcb5a18` ones.
  - Harness: `no_real_portal` unchanged; every HEAD daemon log says `portal shortcuts: unavailable (off)` and every bus log has 0 `xdg-desktop-portal` lines. The exported folders and their target directories were removed.
