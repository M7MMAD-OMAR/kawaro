# S-144: Batch results and safe retry

Status: planned, implementation not started. Executable checks deferred by owner order.
Date: 2026-11-15.
Spec: ../docs/specs/2026-11-15-next-features.md
Implementation packet: ../docs/superpowers/plans/2026-11-15-next-features.md

## Story
I want to see which selected files succeeded, failed or were skipped, and retry eligible failures without processing successful files again.

## Current facts
- Queue events, cancellation, aggregate daemon state and completion notifications already exist.
- A combined action may represent multiple ordered inputs in one task.
- Output paths and notification strings alone cannot reconstruct immutable task identity.

## Acceptance criteria
- AC1: Capture production task-unit identity, ordered inputs and resolved action/params snapshots before execution; update all event producers/consumers additively where identity is absent.
- AC2: Keep success, skip, failure, cancellation and interruption distinct. Preserve S-133 no-smaller-result skips.
- AC3: Provide an explicitly opened tray/results surface, no focus-stealing completion window and no full paths in unsolicited notifications.
- AC4: Session-only bounded retention:100 completed jobs or1000 task records, evicting whole completed jobs. A single oversized job is explicitly summary-only with Retry unavailable, not a partial retained task list. No disk history, active-job eviction or restart replay.
- AC5: Retry failed non-cancelled per-file units only; a failed combined task retains the full original ordered group. Revalidate input identity, current registry, options and engine.
- AC6: Submit a new child job once per pending Retry action, retain parent link, preserve collision-safe output finalization and never overwrite successful outputs.

## Future focused cases, not run
- Mixed success/failure/skip retries only the failed independent unit.
- Failed three-input merge retries all three in original order.
- Cancel, input replacement/removal, missing action and duplicate clicks have explicit safe outcomes.
- Output deletion is shown unavailable; Clear removes records only, not output files.

## Boundaries
No persistent history, automatic retries, crash resume, second queue or new MCP replay API.

## Delivery and review
Parent owns architecture, packet approval and final review. Selected Pi/MiniMax-M3.1-Flash executes only a later authorized bounded packet. Native platform qualification follows stable feature behavior. Source implementation, executed acceptance and public release are recorded separately.

## Planning log
- 2026-11-15: parent authored this story from current source research and the owner request. No product code or executable check changed or ran.
