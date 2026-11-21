# S-148: job visibility and individual cancellation without a tray

Status: source implemented and parent source-reviewed on 2026-11-21: jobs query and single-job cancel (from S-144-C), daemon instance identity on Version/Jobs, checked INSTANCE:JOB references, stale-reference refusal, explicit missing-job reply, older-daemon refusal in the CLI. Runtime checks deferred by owner order.
Date: 2026-11-15.

## Story
As a user running a long conversion or OCR operation, I want to inspect jobs and cancel one without cancelling unrelated work or depending on a tray icon.

## Evidence
- core/src/ipc.rs Request currently exposes CancelAll but no job list or per-job cancel request.
- core Queue already has individual cancellation, so a second runner is unnecessary.
- daemon tray visibility is optional; a missing icon leaves the daemon running without that progress surface.
- S-144 preserves session job records but deliberately keeps the public IPC mirror unchanged. This story is a separate additive protocol design, not an unreviewed addition to that packet.

## Proposed scope and acceptance
- AC1: Add a read-only jobs query with active/queued progress and an explicit CLI entry. The default view shows useful labels/state without dumping complete source paths; detailed local output is opt-in.
- AC2: Add an explicit individual-cancel command routed to the existing queue token. Keep CancelAll and existing clients working. Cancelling one job leaves others running.
- AC3: References include a daemon-instance identity plus job ID. Reject references from a previous daemon lifecycle; never silently interpret them as a current numeric ID.
- AC4: Missing/already-ended jobs are safe explicit responses. Repeated cancellation is idempotent. Cancellation does not claim outputs already committed were removed.
- AC5: An older daemon lacking these additive requests produces a clear unsupported-feature result. Never fall back to CancelAll or restart the daemon to make the command work.
- AC6: Provide explicit user access independent of tray presence, no mandatory popup, focus steal, installer, background polling service or automatic retry. The session-only privacy/retention rules of S-144 still apply.

## Proposed file scope
crates/core/src/ipc.rs; daemon request handling/state/jobs; CLI command parsing/output; existing queue cancellation caller; later optional tray/results controls; focused protocol/client fixtures and bilingual guidance. Identify all exhaustive Request/Reply matches before the implementation packet.

## Future cases, not executed
Two active jobs and cancel one; stale daemon-instance token with reused numeric ID; missing/finished job; repeated cancel; older daemon; absent tray; cancelled queued task; output already committed; unchanged originals and no accidental CancelAll.

## Boundaries
This story is not an automatic crash-resume mechanism, persistent history, daemon replacement or a promise of cross-platform runtime acceptance. It receives a detailed implementation packet after the S-144 ledger is source-reviewed. No test/build/network operation runs merely by writing this proposal.
