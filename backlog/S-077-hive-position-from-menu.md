# S-077 A normal open can use the pointer position on the layer path
Status: split: S-110 (open --at, agents), S-077b (KDE and GNOME pointer source, needs private KWin and GNOME sessions)
Pointer: S-077a is now `backlog/S-110-open-at-position.md` (plumbing, Hyprland launcher, fast loop). The remaining work here is S-077b (AC7 to AC10) plus the private-compositor measurement of AC1 to AC4 and AC6. Earlier status: approved (2026-10-26, manager), re-baselined against main 25f9254.

## Story
As a user opening selected files from a file-manager menu or launcher, I want the layer hive to appear at the pointer when that position is available, with a centred fallback on the focused screen otherwise, so that it is near my work instead of unexpectedly placed elsewhere.

## Current code, without assuming the reported cause
- `kawaro open` has no `--at`; `Request::Open` carries only `files` and `token`. `kawaro drop --at` already parses global logical coordinates, carries them over IPC and places immediately through the layer path.
- A normal open reaches `ui::present` and `Hive::present_files`, which call `show(..., at: None, drop: false, ...)`. On the layer path, no pointer event before the 50 ms timeout reaches `decide(None)`. `plan_at(..., None)` uses the existing centred placement; it is not evidence of a top-edge fallback.
- `kawaro-open-selection` runs `kawaro open --from-clipboard` without a position. Dolphin runs `kawaro open -- %F`; Nautilus obtains an activation token but no pointer position.
- The reported top-edge symptom therefore still needs a real reproduction with the actual launcher, compositor, pointer state and logs. Do not claim that `kawaro-open-selection` selects a top default, and do not change placement based on that unproven explanation.

## Decisions
1. Before implementation, reproduce the report on the maintainer's live Hyprland session if available, and independently in a private compositor harness. Never experiment on the live desktop with the private GUI tests. Record the exact menu or launcher command, compositor and version, pointer coordinates, output geometry, `hive anchor` and `hive surface` lines, and a screenshot or surface rectangle. If the top-edge symptom does not reproduce, record that result and treat the remaining work as explicit pointer plumbing, not a confirmed top-fallback bug fix.
2. Add optional `--at X,Y` to normal `kawaro open`, with the same syntax and global logical coordinate meaning as `kawaro drop --at`. Add optional `at` to `Request::Open` with `#[serde(default)]`, so older clients and requests without it remain valid. Thread it through CLI, daemon and normal UI presentation.
3. Reuse the existing `layout::plan_at`, output selection, edge adaptation and immediate `--at` placement used by drop mode. Do not duplicate placement maths. A normal open remains normal mode: selected files and previews, one initial ring, normal click/focus/dismiss behaviour, and no drop receiver, drag probe, hold gate, two-ring waiting layout or later drop movement.
4. With no `--at`, keep the existing normal-open pointer wait and centred no-anchor behaviour except for the KDE focused-screen fallback required below. On the ordinary window path, `at` has no positioning effect.
5. Hyprland's `kawaro-open-selection` passes the pointer read at invocation from `hyprctl cursorpos` through normal `open --at`. Read it once, preserve negative and fractional coordinates, and fail plainly without opening at an invented position if the read or parse fails.
6. KDE gets a private pointer-source spike for the actual Dolphin service-menu process. Test available sources rather than assuming KWin scripting, `org.kde.KWin` `queryWindowInfo` or the activation-token surface works. Record commands, versions, results and whether coordinates are global logical coordinates. If a reliable source exists, pass it to normal `open --at`; if none exists, centre the normal layer hive on the output containing the focused window. Do not substitute the pointer's current output unless that is what the measured focused-window source provides.
7. GNOME remains on the ordinary centred window path, with no anchor log and no new pointer or positioning mechanism. Existing focus-token behaviour stays unchanged on every desktop.

## Split, one work day or less each
| Part | Scope | Criteria |
| --- | --- | --- |
| S-077a | Reproduction, normal-open `--at` plumbing, shared placement without mode mixing, Hyprland launcher | AC1 to AC6 |
| S-077b | KDE pointer-source spike and chosen integration or focused-screen centred fallback, GNOME regression, docs | AC7 to AC10 |

## Acceptance criteria
- AC1: The story log contains the live reproduction or a recorded reason it was unavailable, plus a private-compositor reproduction attempt before the placement change. Each record includes command, compositor/version, pointer and output coordinates, anchor and surface logs, and the observed rectangle. It states either the measured top-edge result or that the symptom did not reproduce; neither case asserts the old top-default theory as fact.
- AC2: `kawaro open --at 400,500 --token T FILE` sends one `Request::Open` with the same file and token and `at = [400, 500]`. Omitted `at` deserializes as None in old JSON. Fractional and negative coordinates round-trip. Malformed, non-finite or wrong-arity values exit 2 and send no Open request, matching drop's parser behaviour.
- AC3: In private KWin and nested Hyprland layer harnesses, normal `open --at` at `(400,500)` centres the hive at that anchor within 1 px and matches `plan_at`; corner and edge cases keep every placed cell inside the output and adapt exactly as the existing placement oracle specifies. There is no initial centred frame followed by a jump to `at`.
- AC4: The same normal-open `at` show retains selected-file centre content and thumbnail behaviour, normal focus and Esc dismissal, and normal click action execution. Its trace has no drop probe, drop receiver, hold or release behaviour. Existing drop `at`, no-anchor drop movement, held-drop and two-ring tests pass unchanged.
- AC5: `kawaro-open-selection` reads `hyprctl cursorpos` once and invokes normal `kawaro open --from-clipboard --at X,Y` with the copied-selection flow intact. A fake `hyprctl` test covers printed spaces, negative and fractional coordinates, and failure; failure sends no Open request and produces no `(0,0)` or centred substitute presented as the pointer.
- AC6: Normal `open` without `--at` still accepts an actual pointer event before the existing 50 ms deadline. With no pointer and outside the KDE fallback case, it keeps the current centred no-anchor placement. Existing ordinary window behaviour and the 150 ms first-frame budget remain satisfied.
- AC7: The KDE spike runs only in a private session against the real service-menu launch path. Its log identifies each tested pointer source, the exact result and coordinate space, and a positive control proving the measurement is not vacuous. The selected implementation uses a measured reliable source or explicitly selects the focused-screen fallback; an unproven source is not shipped.
- AC8: KDE with a proven pointer source places normal menu opens at that pointer within 1 px in 3 of 3 runs, including one run with a still pointer. Without a proven source, 3 of 3 menu opens with no usable pointer centre on the focused window's output, verified against output geometry, and none is described as pointer-positioned. Repeat the fallback on a second output with the focused window there.
- AC9: Private GNOME normal opens remain centred ordinary windows in 3 of 3 runs, with no `hive anchor` line and unchanged activation-token focus behaviour. Supplying `open --at` does not move the GNOME window or turn it into a layer or drop surface.
- AC10: Existing normal-open, drop-placement, focus and integration tests pass after the D-item expectation updates. English and Arabic docs describe optional normal-open `--at`, the Hyprland launcher and the measured KDE outcome without claiming an untested pointer source. One CHANGELOG entry names S-077 in both languages; design lint and no-long-dash checks pass.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Live observation if available, private reproduction, logs and rectangle evidence | integration |
| AC2 | CLI fake-daemon JSON and IPC compatibility tests using normal Open | unit, integration |
| AC3, AC4, AC6 | Private KWin and nested Hyprland normal-open placement and mode-separation checks | integration, performance |
| AC5 | Fake `hyprctl` launcher tests and one private nested-Hyprland run | unit, integration |
| AC7, AC8 | Private Dolphin/KWin pointer-source spike, positive control and two-output fallback | spike, integration |
| AC9 | Existing private GNOME normal-open and strict-focus runs, with and without `at` | integration |
| AC10 | Regression suites, bilingual docs and CHANGELOG | static, integration |

## Out of scope
- Changing drop-mode layout, drag capture, hold timing or portal activation behaviour.
- A universal Wayland pointer API, which does not exist, or positioning ordinary GNOME windows.
- X11 and Windows pointer work, and any change to their existing normal-open behaviour.
- Treating the KDE spike candidates as proven APIs before the spike records results.

## Manager decisions for the tester, 2026-10-26
- D1: Update existing `Request::Open` construction expectations mechanically with `at: None` where the new field is required, including `crates/core/src/ipc.rs`, `crates/cli/src/spawn.rs` tests and CLI fake-daemon assertions. These updates do not authorize changing Drop JSON, hold semantics or foreground-grant behaviour. The tester will rewrite the affected tests from these criteria before implementation.
- D2: `tests/integrations/test_hyprland.sh` currently expects the final command `kawaro open --from-clipboard`; replace that stale expectation with AC5's normal-open `--at` command and failure cases. Keep the clipboard delay and copy-selection assertions.
- D3: Do not bulk-add `--at` to Nemo, Thunar, Nautilus or Dolphin commands. Their current expectations in `tests/integrations/test_static.sh`, `test_install.sh` and `test_nautilus.py` change only where AC7 and AC8 prove a KDE source. GNOME/Nautilus command and token expectations remain unchanged.
- D4: Existing tests that intentionally put the pointer at `(640,0)`, including `tests/e2e/run.sh`, are not evidence of a no-anchor top fallback and must not be weakened. Add separate no-anchor reproduction evidence under AC1 and explicit normal-open `at` checks under AC3.
- D5: Existing drop tests expecting `hive anchor: none`, a later move, probe behaviour or two rings remain unchanged. A normal-open implementation that makes those tests pass by reclassifying normal mode as drop mode is not acceptable.
- D6: KDE snapshots and integration expectations are written only after the AC7 spike chooses pointer placement or focused-screen fallback. The tester will record that choice and write the corresponding tests later; no preemptive assertion about `queryWindowInfo`, KWin scripting or activation-token surfaces is approved.

## Review log
- 2026-10-26, planner: re-baselined against `cli/src/main.rs`, `core/src/ipc.rs`, `daemon/src/server.rs`, `daemon/src/ui.rs`, `ui/src/lib.rs`, `ui/src/layer.rs`, the Linux integrations and current tests. The former claim that no-anchor normal opens default to the top was removed because the code shows a centred fallback instead. No code or tests changed by planning.
