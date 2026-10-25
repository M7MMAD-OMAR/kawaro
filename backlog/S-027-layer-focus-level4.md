# S-027 Keep the layer hive off the keyboard at KWin level 4 without a valid token
Status: done (2026-10-23)

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
- AC3: at level 4, for `notoken` and `bogus`, the show is a layer surface, with no `wl_keyboard.enter` on the catcher, no `hive focus: true`, and `focus=false` in the TABLE line. The bogus case still sends `activate("kawaro-bogus-token", <catcher>)`.
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
      PASS AC10 level 4 bogus: activate("kawaro-bogus-token", <catcher>) still sent
      PASS AC10 level 1 notoken: the measured show is a layer surface
      PASS AC10 level 1 notoken: wl_keyboard.enter on the catcher within 500 ms of get_layer_surface
      PASS AC10 level 1 notoken: hive focus: true
      ```
    - `fcb5a18`, runs 1 and 2: `FAIL AC10 level 4 notoken, pointer still: the hive does not take the keyboard (no hive focus: true)` and `FAIL AC10 level 4 bogus, pointer still: ...`, both times; `PASS` on every other check, `1:notoken` `RESULT PASS`; TABLE `focus=true` at level 4 for both cases.
    - Mechanism in the traces of the measured show: in `02c56cb` the first `set_keyboard_interactivity(1)` on the layer surface is at line 6 of the show and the catcher's first `wl_surface.commit` at line 12, with none after; in `fcb5a18` (and HEAD) the first commit is at line 11 and `set_keyboard_interactivity(1)` follows at line 27. So `fcb5a18` is the first failing commit; the other `layer.rs` commits in `49bfbcb..f42dd04` were not searched, as the story allows.
  - Contract (what the implementer must make true; check names start with `S-027`):
    - AC2, every case: in the measured show's trace, from the catcher's `wl_compositor.create_surface` (or `get_layer_surface`) to that layer surface's `destroy`, the first `zwlr_layer_surface_v1.set_keyboard_interactivity` is `(1)` and its line comes before the catcher's first `wl_surface.commit`; no `set_keyboard_interactivity` on that layer surface after that commit. Order is by line, not stamp.
    - AC3, `4:notoken` and `4:bogus`: layer surface, `$enter` empty (no `wl_keyboard.enter` on the catcher), no `hive focus: true`, `focus=false` in the TABLE line; bogus still sends `activate("kawaro-bogus-token", <catcher>)` (existing check kept).
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
- 2026-10-23, implementer: fix in `crates/ui/src/layer.rs` only, plus the ADR 0004 section 3 bullet and one CHANGELOG line; no test touched.
  - Fix (decision 2): `map` sets `set_keyboard_interactivity` once, at creation and before the catcher's first `wl_surface.commit`: `Exclusive` for a normal show, `None` for a drop show. The `Exclusive` request in `decide` is gone, so nothing sets it on the mapped catcher. The `no_drag` early return, the S-019 anchor wait and `--at` placement and the S-021 probe path are unchanged; `--at` also commits after the creation block. No other `set_keyboard_interactivity` in `crates/`.
  - Gates, verified by running, in the 6G scope with `-j 1`, one private compositor at a time: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all` (585 passed, 0 failed, 1 ignored, 34 suites) and `scripts/design-lint.sh` (clean) pass; fresh `cargo build --release` passes. `cargo mutants --in-place --in-diff` lists no mutants (`layer.rs` is in `exclude_globs`).
  - `tests/overlay/focus.sh` (default cases): 51 PASS, 0 FAIL, all 8 cases and the final line `RESULT PASS`; TABLE `focus=true` at `0:notoken 1:token 1:notoken 2:notoken 3:notoken 4:token` (enter 0 to 4 ms), `focus=false enter_ms=none` at `4:notoken` and `4:bogus`; Esc to destroy 0 to 31 ms in the 6 AC5 cases.
  - `tests/overlay/kwin.sh` 104 PASS 0 FAIL; `tests/drop/kwin.sh level=1` 354 PASS 0 FAIL; `tests/hyprland/repro.sh --json` `{"total": 55, "failed": 0, "skipped": 0}`; `tests/drop/window_path.sh` 4 PASS; `tests/drop/portal.sh` 57 PASS; `tests/drop/hold_docs.sh` 21 PASS; all `RESULT PASS`. `tests/overlay/budget.sh` (wraps `crates/ui/tests/measure.sh 20 60`): layer path warm median 53.0 ms plain, 53.2 ms with a token, RSS 30.0 MB, `RESULT PASS`.
  - For the manager, outside the AC6 list: `tests/e2e/run.sh` (a release gate in `scripts/release-gates.sh`) ran once and failed 1 of 37: `FAIL S-018 AC18: the hive is a layer surface with the keyboard`. Cause, verified in its daemon log: the first show prints `hive focus: true` (line 443) before `hive surface: layer` (line 478), because the keyboard now comes at map and the surface line only with the first frame after the up to 50 ms anchor wait; the check breaks on the focus line and greps the surface line at once. This is the 9 ms race the S-019 log gives as the reason `fcb5a18` moved the request into `decide`, now wider. The two later shows print the surface line first. Not worked around (decision 6): no test edited and the debug line not delayed. Holding the `hive focus` debug line until the show's `hive surface` line is printed is one option for the manager to weigh.
- 2026-10-23, manager under the maintainer's delegation, on the `tests/e2e/run.sh` S-018 AC18 race: the daemon is right and the check is racy, because it stops waiting at the first `hive focus: true` line and then greps once for `hive surface: layer`, which can print up to `ANCHOR_WAIT` later now that the keyboard is granted at map. The debug line is not delayed in the product. The tester changes that one check to wait (bounded, same timeout) until both lines are present for the show, with no other change to the check's meaning.
- 2026-10-23, tester, on the manager's `tests/e2e/run.sh` decision: only the `S-018 AC18: the hive is a layer surface with the keyboard` check changed, plus this entry; no crate source and no other test touched.
  - Change: the wait loop now stops only when both `hive surface: layer ` and `hive focus: true` are in the daemon log, with the same bound (30 polls of 0.1 s, 3 s), and the check asserts the same two lines through the same predicate (`ac18_lines`). The daemon started for this block shows the hive once before the check, so its whole log is that show. Still 37 checks.
  - `tests/overlay/kwin.sh` unchanged: every show sleeps a fixed 1.5 s between marks and all line checks (AC11, AC9 at the `hive focus: true` grep, S-019 AC5) read the segments after `finish`, so no check stops early on a focus line. `tests/hyprland/repro.sh` unchanged: on the layer path `show()` waits for the `hive surface:` line, which now prints after the focus line, and the focus check follows a 0.5 s sleep; the window path is not touched by S-027; the segments at the anchor and corner checks are read after `show` plus `sleep 1`.
  - HEAD (`b3e4a60`, `git status crates/` clean, `cargo build --release -j 1 -p kawaro-cli -p kawaro-daemon` in the 6G scope rebuilt `kawaro-ui` and `kawaro-daemon`), `tests/e2e/run.sh` 3 times, one private KWin at a time: each run 37 PASS, 0 FAIL, exit 0. The race happened in all three: first `hive focus: true` at daemon log line 443, first `hive surface: layer` at line 462, 54 to 58 ms later. Cutting run 1's log at line 443: the old wait stops there and the old check FAILs; the new wait keeps waiting. On the full log both pass.
  - `f258720` (before the fix), exported with `git archive` to the scratchpad and built there with its own `CARGO_TARGET_DIR`, `-j 1`, in the scope; HEAD's `run.sh` run on it: 37 PASS, 0 FAIL, exit 0, AC18 PASS; there the surface line (459) comes before the focus line (465), as expected. The exported folder and its target directory were removed.
  - Portal rule: `no_real_portal` unchanged; every daemon log (3 HEAD, 1 old) says `portal shortcuts: unavailable (off)`.
- 2026-10-23, reviewer (review 1): **PASS**. Range `f258720..HEAD` (`897b5b2`), all commands verified by running in the 6G scope with `CARGO_BUILD_JOBS=1`, one private compositor at a time, none on the live desktop.
  - Test integrity: after `f258720` the only change under `tests/` is the S-018 AC18 block of `tests/e2e/run.sh` (`897b5b2`), as the manager decided. Same two predicates (`^hive surface: layer ` and `^hive focus: true$` on the whole daemon log), same bound (30 polls of 0.1 s), now through `ac18_lines` in both the wait and the check; the block follows the daemon's first show (`open shows the hive`), so the meaning is kept. No other test changed.
  - Fix, `crates/ui/src/layer.rs`: the only `set_keyboard_interactivity` in `crates/` is in `map` (line 977), right after `get_layer_surface` and before the catcher's first commit (line 1032), `Exclusive` for a normal show and `None` for a drop show; the request in `decide` is removed. A new catcher is made on every show, so no mapped catcher is reused across modes. The `no_drag` return, the anchor wait, `--at`, the S-021 probe path and the activation token are unchanged. The focus.sh AC2 pin (16 checks) passes on all 8 cases.
  - Gates: `cargo fmt --check` clean; `cargo clippy --all-targets -- -D warnings` clean; `cargo test --all` 585 passed, 0 failed, 1 ignored, 34 suites; `scripts/design-lint.sh` clean. `cargo mutants --in-place --in-diff` on the crates diff: "No mutants to filter", because `crates/ui/src/layer.rs` is in `exclude_globs` (`.cargo/mutants.toml:26`). So no mutant tests this fix; its guard is the trace level AC2 pin and the AC3 checks in `tests/overlay/focus.sh`, which the tester showed fail on `c5e5995` and pass on `02c56cb`. Acceptable under the existing exclusion.
  - Fresh `cargo build --release` (whole workspace) passed; `git status --porcelain crates/ tests/` empty before and after the build and all runs, and after the mutants run. Every script used `target/release`.
  - Runs, each `RESULT PASS` and exit 0: `tests/overlay/focus.sh` default cases 51 PASS 0 FAIL, TABLE `focus=true enter_ms=1` at `0:notoken 1:token 1:notoken 2:notoken 3:notoken`, `focus=true enter_ms=3` at `4:token`, `focus=false enter_ms=none` at `4:notoken` and `4:bogus`, Esc to destroy 1 ms in all 6 AC5 cases; `tests/overlay/kwin.sh` 104 PASS 0 FAIL; `tests/drop/kwin.sh level=1` 354 PASS 0 FAIL; `tests/hyprland/repro.sh --json` `{"total": 55, "failed": 0, "skipped": 0}`; `tests/drop/window_path.sh` 4 PASS 0 FAIL; `tests/drop/portal.sh` 57 PASS 0 FAIL; `tests/e2e/run.sh` 37 PASS 0 FAIL, exit 0, AC18 PASS; `tests/overlay/budget.sh` layer warm median 53.1 ms plain and 53.5 ms with a token (under 150 ms), RSS 30.0 MB, 8 PASS; `tests/drop/hold_docs.sh` 21 PASS 0 FAIL.
  - Docs: ADR 0004 section 3 has the bullet with the rule, the bisect "verified by running" and the KWin mechanism "assumed (not read in source)". `CHANGELOG.md` has one line under Unreleased, Fixed, in English and Arabic, so no second line is added.
  - Not applicable: screenshots and UI strings (no UI or string change). Style: no em dash or en dash and no personal details in the range; the code matches the file's style.
