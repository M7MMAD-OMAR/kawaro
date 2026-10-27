# S-084a spike: the Windows drag trigger, measurements M1 to M5

Date: 2026-10-27. Run from the S-084 chain on Linux. This report contains no
invented measurement. A real Windows machine and the S-044 ISO virtual machine
are not reachable from this chain (the gap S-079 owns), so the five live runs
could not be executed here. Each measurement below records what the evidence
already on record establishes, what a live run must still record, and the
verdict the manager decision rests on. The live numbers are owner-run and land
in the S-079 checklist results or a follow-up run of this spike.

## Evidence on record (measured before this report, not new measurements)

- Explorer reserves the drag modifiers while a drag is in progress: Ctrl
  copies, Shift moves, Alt creates a shortcut, Ctrl+Shift creates a shortcut.
  Any held chord containing one of them flips the drag badge and changes what
  a drop outside the hive does. Source: `references/windows-platform-facts.md`
  (measured constraints) and the S-084 research of 2026-10-26.
- The owner's real Windows machine report, 2026-10-26: holding Alt to drag
  created file shortcuts in the system. This is a live measurement of the
  Alt behavior named in M5, and it fails the invariant of the story.
- The right-button drag menu ("Copy here / Move here / Create shortcuts here")
  is the shell folder's own drop behavior and does not appear over an outside
  drop target such as the hive window. Source: the same measured constraints.
- `RegisterHotKey` accepts only Ctrl, Shift, Alt and Win as modifiers; a Win
  press and release opens Start; Win+H, Win+Z and Win+C are in shell use.
  Source: the same measured constraints.

## M1: right-button drag over the hive

What a live run must record: OLE `DragOver` and `Drop` arrive with
`MK_RBUTTON`, the hive appears within 150 ms and 9 frames of the drag start,
dropping on a cell runs the action and the original file stays untouched. M1
also records that an ordinary short right-drag never shows the hive (the
right-drag trigger requires the drag to have started, not a momentary button
press).

Live status: not run, owner-run (S-079). Established by evidence: the
right-button drag holds no Ctrl, Shift or Alt at any moment, so it cannot
change the drag badge or the drop effect by the measured Explorer semantics
above. The trigger moment is fixed below before implementation, as AC3 asks.

Verdict for the candidate: PASS on the invariant by construction (no drag
modifier is held and no Explorer menu appears over the hive); the timing
numbers (150 ms, 9 frames) and the live drop behavior stay owner-run.

## M2: the same with swapped primary buttons and at 150% display scaling

Live status: not run, owner-run (S-079). Established by evidence: the trigger
is the secondary button, that is the one whose drag shows the shell's own
menu; with `SM_SWAPBUTTON` the caller maps it to the other physical button,
exactly as the S-047 primary mapping already does. The scaling affects only
the hive placement, which S-047 covers.

Verdict for the candidate: PASS on the invariant by construction; the live run
records the swapped mapping and the scaled placement.

## M3: hovering the tray icon while dragging

Live status: not run, owner-run (S-079). Established by evidence: a hover
holds no key, so the invariant holds by construction.

Verdict for the candidate: PASS on the invariant by construction, but the
wiring and its 150 ms timing are not shipped in this story (the manager
decision defers the tray hover to S-084c until this measurement runs).

## M4: Win chord candidates

What a live run must record: whether the held chord changes the Explorer drag
badge, whether Win release opens Start after the drop, and which Win+letter
combinations `RegisterHotKey` can take without breaking a shell binding the
machine shows in use.

Live status: not run, owner-run (S-079). Established by evidence: every
`RegisterHotKey` chord avoiding the three drag modifiers must hold Win, Win
chords are crowded by the shell (Win+H, Win+Z, Win+C), and a Win press and
release opens Start.

Verdict for the candidate: NOT MEASURED, cannot be named as the shipped
trigger. Per AC2a the manager decision keeps the chord out of the documented
triggers.

## M5: the current chord (Ctrl+Alt+H) recorded for comparison

What a live run must record: the badge state during the drag and what a drop
on a folder does.

Live status: not run, owner-run (S-079). Established by evidence: the chord
holds Ctrl and Alt during the drag, both Explorer drag modifiers, and the
owner's report shows a held Alt drag creating file shortcuts.

Verdict for the candidate: FAIL as a documented trigger. It stays functional
with its taken-conflict behavior as an undocumented internal path with a
plain bilingual warning wherever it is mentioned (manager decision).

## Candidates ruled out before any run

- Holding Alt (the Linux way): FAIL. The owner's 2026-10-26 real-machine
  report created file shortcuts in the system. No shipped wording may tell a
  Windows user to hold Alt while dragging.
- Any held chord of Ctrl, Shift or Alt: FAIL by the measured Explorer drag
  semantics (the badge flips and the drop effect changes).

## Trigger moments fixed before implementation (AC3)

- Right-button drag (the chosen trigger): the trigger moment is the drag
  start, that is the secondary button held and the pointer moved past the
  system drag threshold (`SM_CXDRAG`, `SM_CYDRAG`) from the press point. A
  momentary button press, or movement under the threshold, never shows the
  hive. There is no hold threshold and no `hold_ms`: the drag itself is the
  trigger and the S-021 real-drag rule is its gate. The drag ending (button
  up) is not itself a close signal: the OLE drop and cancel events close the
  hive and the drop hive timeout is the backstop.
- Tray hover (candidate two, deferred to S-084c): the hover over the tray
  icon while a drag is in progress, no threshold.
- Key hold (the internal chord and any captured key): the S-019e long press
  gate with `hold_ms`.

## What remains owner-run (the live numbers of M1 to M5)

A real Windows machine or the S-044 VM records: M1 the `MK_RBUTTON` arrival,
the 150 ms and 9 frame timing, the drop running the action and the untouched
original, and the ordinary short right-drag showing nothing; M2 the swapped
buttons and 150% scaling; M3 the tray hover timing and drop; M4 the Win chord
badge, Start behavior and `RegisterHotKey` conflicts; M5 the chord badge state
and the folder drop. Until those numbers exist, no timing claim is published
and the Explorer-side behavior claim is published only as the mechanism states
it: the trigger holds no Ctrl, Shift or Alt while the drag is in progress.
