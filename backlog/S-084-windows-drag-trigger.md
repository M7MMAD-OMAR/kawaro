# S-084 Windows drag trigger without the Explorer drag modifiers
Status: approved under manager delegation (2026-10-26); the default is named only after the measurements below (S-075 rule)

## Story
As a Windows user, I want one key or one button that opens the hive while I drag a document, and it must never change what Explorer does with my drag, so that no drag silently creates a shortcut, copies or moves my file.

## Research (2026-10-26)
Explorer reserves the drag modifiers while a drag is in progress: Ctrl copies, Shift moves, Alt creates a shortcut, and Ctrl+Shift also creates a shortcut. Any held chord containing one of them flips the drag badge ("Copy to", "Move to", "Create link in") and changes what a drop outside the hive does. This is the owner's report: holding Alt to drag created file shortcuts in the system. On Windows the hive comes from the S-048 chord shortcut (default Ctrl+Alt+H) pressed during a drag, so a bare Alt hold shows nothing of Kawaro and is pure Explorer behavior.

`RegisterHotKey` needs at least one modifier and offers only Ctrl, Shift, Alt and Win. Every chord that avoids the three drag modifiers must therefore use the Windows key, and Win chords are crowded by the shell (Win+H dictation, Win+Z snap layouts, Win+C chat), while pressing and releasing Win opens Start.

The only native one-button drag Explorer leaves alone is the right-button drag: its "Copy here / Move here / Create shortcuts here" menu is the shell folder's own drop behavior and does not appear over an outside drop target such as the hive window. The leading candidate is therefore the right-button drag, the second is the tray icon as a hover drop target, and the fallback is a Win chord.

## Measurements before any implementation (a real Windows machine or the S-044 VM)
- M1: right-button drag over the hive: OLE `DragOver` and `Drop` arrive with `MK_RBUTTON`, the hive appears within 150 ms and 9 frames of the drag start, dropping on a cell runs the action and the original file stays untouched.
- M2: the same with swapped primary buttons and at 150% display scaling.
- M3: hovering the tray icon while dragging shows the hive at the pointer within 150 ms and the drop works.
- M4: Win chord candidates: whether the held chord changes the Explorer drag badge, whether Win release opens Start after the drop, and which Win+letter combinations `RegisterHotKey` can take without breaking a shell binding the machine shows in use.
- M5: the current chord (Ctrl+Alt+H) recorded for comparison: the badge state during the drag and what a drop on a folder does.

## Needs from the maintainer
A real Windows machine or the S-044 ISO virtual machine for the M1 to M5 measurements. Nothing else in the repo has been verified on a live Windows machine (S-079 owns that gap), so this story's spike is owner-run or run by whoever holds that machine; the implementation after the spike is ordinary code and tests.

## Split, one work day or less each
| Part | Scope | Criteria |
| --- | --- | --- |
| S-084a | Spike M1 to M5 on the machine, the report, and the manager decision line naming the default | AC1, the decision line, AC2's measured clause |
| S-084b | Implementation of the chosen trigger, S-076 picker changes, docs and tests | AC2a to AC6 |

## Acceptance criteria
- AC1: `spikes/windows-drag-trigger/out/report.md` records M1 to M5 with pass or fail per candidate and the measured numbers. The default is then chosen in a `## Manager decision` line added to this story before any TDD. No story, doc or UI string may name a Windows trigger key as shipped before that line exists (the S-075 rule against unmeasured claims). Contingency: if every candidate fails M1 to M5, the decision line records that fact and names the least harmful fallback (the current chord with its warning, per AC5), S-087 and S-088 keep their key-free wording permanently, and the story reports the gap to the maintainer instead of inventing a default.
- AC2: the chosen trigger contains none of Ctrl, Shift and Alt while the drag is in progress, or is the right-button drag itself. The Explorer-side behavior (a drop outside the hive leaves the original untouched and creates no shortcut) is proven by the M1 to M5 spike measurements on the real machine and recorded in the spike report; CI keeps only what it can observe: the pure logic of `drop_hive_allowed` and the gate. AC2a: the invariant covers every shipped trigger, not only the default. The documented fallback chord is part of the spike M5 comparison: if it still holds a drag modifier, the manager decision either replaces it with a chord that holds none (Win-based, measured in M4) or records a plain bilingual warning beside it in the docs; the story's "never changes Explorer's drag" claim may not be published while any documented trigger holds Ctrl, Shift or Alt during a drag.
- AC3: the S-021 rule and `drop_hive_allowed` accept the chosen trigger as a real drag, including the swapped-primary-buttons case and the right-button drag; a menu invocation still always shows the hive.
- AC4: a fresh Windows config uses the chosen trigger; Alt is never the Windows default. The S-076 drag-key picker on Windows offers only measured options, and its helper text never tells a Windows user to hold Alt while dragging. S-076 AC5 and AC6 are amended for Windows by D1 and D2 below.
- AC5: `docs/drop-shortcut.md`, `docs/drop-shortcut.ar.md`, the README Windows paragraphs and the S-087 demo captions describe only the measured Windows trigger. The S-048 chord keeps its taken-conflict behavior (`shortcut <string> unavailable`, daemon alive), but a chord that holds a drag modifier during the drag is never shipped as the documented trigger: the manager decision either re-bases the default chord on the M4 measurement (Win-based, no drag modifier) or leaves the chord as an undocumented internal path with a plain bilingual warning wherever it is mentioned. The S-087 caption consistency check lives in S-087's own tests (dynamic captions), not as a static text check here.
- AC6: a custom Windows `shortcut` recorded in the config survives S-076 saves, as S-076 AC3 already requires.

## Landing order (fixed by the goldfish review, 2026-10-26)
S-084 lands before or together with S-076b: D1 and D2 amend S-076 AC5 and AC6, so if S-076b lands first its tests encode the superseded "missing drop_key means Alt, Alt accepted on Windows" semantics and must be rewritten by the S-084 tester. S-087 AC3 and S-088 AC1 consume the manager decision line of AC1; those stories may not name a Windows trigger before that line exists in this file.

## Manager decisions for the tester
- D1: this story supersedes the Windows side of S-076 AC5 (a missing `drop_key` means Alt). On Windows a missing `drop_key` resolves to the default named in the manager decision of AC1. Capture refuses bare Alt and any single-modifier choice of Alt, Ctrl or Shift (each is an Explorer drag modifier on Windows), and refuses a chord whose modifiers would all be held during the drag; the refusal text names the Windows reason in both languages. A stored `drop_key` of Alt written before this story re-resolves to the Windows default on read (no migration write), so a stale value never resurrects the bug. S-075 AC4's "Alt is the default everywhere" is amended to "Alt is the default where it was measured", and on Windows the S-076 "Reset to Alt" button becomes "Reset to the Windows default" with the same behavior as capture.
- D2: this story supersedes the Windows side of S-076 AC6's helper text. On Windows the helper names the measured trigger instead of the `drop_key` value.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | static check that the spike report exists with the five measurements and that the manager decision line exists before any Windows trigger constant | static |
| AC2, AC3 | pure tests of `drop_hive_allowed` and the hold gate with the chosen button state, including swapped buttons and a menu source | unit |
| AC4, AC6 | per-platform config default, picker options, Alt refusal text, and a whole-Config save that keeps a custom `shortcut` | unit, UI |
| AC5 | doc and caption checks naming only the measured trigger | static |

## Out of scope
- GNOME, KDE and Hyprland triggers (S-075 owns them), the GlobalShortcuts portal, X11 (S-015), changing what Explorer itself does with drags (the `DefaultDropEffect` registry is not ours to touch).

## Review log
- 2026-10-26, planner: written from the owner Windows beta report; three goldfish reviews (design, readiness, zoom-out) applied to the text.
