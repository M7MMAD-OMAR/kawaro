# Main Consolidation Implementation Plan

> **For agentic workers:** Implement this plan task-by-task using available execution tools. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve valuable outstanding work on main, verify and push it, remove all other branches and worktrees, and document the next engineering priorities.

**Architecture:** Keep current product code and selectively restore missing research sources. Preserve obsolete implementations in a verified external bundle instead of merging historical regressions. Repair only the existing candidate blockers required for a truthful, passing source push.

**Tech Stack:** Rust, Slint, Python site generator, shell gates, Git, GitHub Actions.

**Spec:** Owner request of 2026-11-15 and `docs/maintenance/2026-11-15-consolidation.md`.

## Global Constraints

- Implementation model: `pi/minimax/MiniMax-M3.1-Flash`; parent owns plans and review.
- Work on existing main. No new branches or worktrees.
- Preserve dirty work before cleanup. No whole-tree git add, no force push.
- English repository documentation, no long dashes, Western digits.
- No product feature work or release tag as part of consolidation.
- Current public release remains 0.5.0 until separate publication verification.
- Heavy checks serialized under MemoryMax=6G, CPUQuota=100%, nice 15, CARGO_BUILD_JOBS=1, test threads=2, temporary files under HOME.
- No suppressed warnings, disabled assertions, or test weakening.

### Owner exception, 2026-11-15

The owner issued a one-time exception for the source consolidation push only:

- The full local gate set (Task 3) is **deferred**, and so is the remote CI. The source
  push rests on a static review only.
- No release tag, no release pipeline, no image capture, no publication.
- Both consolidation commits carry the `[skip ci]` marker so the remote CI does not
  start. The `[skip ci]` marker is the only CI mechanism used here; no workflow is
  disabled and no test assertion is touched.
- The deferral is not a pass. No gate result may be claimed for the consolidation.
- The rule "the full local set must be green before a push to `main`" still applies to
  every later push, and in particular to the 0.5.1 publication, which needs fresh
  authorization to run the gates.
- References to an execution earlier in this plan are historical. The final closeout
  in `docs/maintenance/2026-11-15-consolidation.md` is authoritative.

## Task 1: Restore missing source and correct the inventory

**Status:** done on 2026-11-15.

**Files:** `spikes/alt-portal/`, `spikes/gnome-alt-extension/`, `docs/reviews/goldfish-zoomout-s083-s088.md`, `docs/maintenance/2026-11-15-consolidation.md`.

**Interfaces:** Consume verified external bundle and GNOME payload archive. Produce tracked experimental source, historical review, accurate inventory.

- [x] Recompute branch counts from refs. The baseline has 33 including main, 22 ancestor branches and 10 non-ancestor branches; do not confuse unique commits with summed counts across duplicate branches.
- [x] Restore alt-portal files from spike/alt-portal without changing product code.
- [x] Stage the GNOME payload outside the nested worktree and compare against backup. Remove that registered worktree only after verified preservation; restore its seven source files at the root spike path.
- [x] Restore the missing goldfish review from its exact branch blob. Mark it historical and do not represent old conclusions as current acceptance.
- [x] Add research README provenance and limitations. Preserve obsolete APT and optional duplicate tests in the external backup only.
- [x] Correct inventory claims: a merge does not inherently revert newer commits, but obsolete conflicting implementations must not be accepted. Only one remote branch other than main exists.

## Task 2: Repair current candidate documentation blockers

**Status:** done on 2026-11-15.

**Files:** `site/build.py`, `tests/site/` focused generator contract, generated site formats/HTML/LLM outputs, `README.md`, `README.ar.md`.

**Interfaces:** Consume action engine IDs and existing TOOLS mapping. Produce accurate tool labels and truthful X11 verification scope.

- [x] Reproduce generator misclassification using a focused real-action test for Poppler and qpdf, then add explicit mappings matching existing engine naming conventions. Preserve built-in labels for actual built-in engines.
- [x] Run the focused test red before implementation and green after. Regenerate through existing site tooling with RELEASE_PUBLISHED=0.
- [x] Restore explicit English X11 wording: tested only in a private virtual X server with xfwm4; real GNOME/KDE desktops and file-manager drag sources remain unverified. Match this scope in Arabic. Keep all existing source assertions unchanged.
- [x] Review Arabic changes against installed T1-T16 protocol in a separate review before push. No new commercial or privacy promises.

## Task 3: Verify the frozen candidate

**Status:** **deferred by owner order, 2026-11-15.** The full gate set and the remote CI were not run for the consolidation push, and no pass is claimed. The steps below stay as the reference for the next gated push, which is the 0.5.1 publication and needs fresh authorization to run the gates.

**Files:** Existing candidate paths, external gate script/logs, maintenance evidence.

**Interfaces:** Consume frozen source and produce exit-coded verification evidence for parent review.

- [ ] Inspect all eight existing version PNGs and compare their hashes with launch manifest. Run existing no-update settings snapshots only in private compositor.
- [ ] Run complete existing `~/kawaro-publish/verify-version-0.5.1.sh` inside the mandated resource scope. Keep native target at repo/target and Windows target override only on Windows commands; do not accidentally route Linux checks into the Windows cache.
- [ ] Record every command, exit status, source manifest and actual failure. Diagnose any new failure before edits. Parent reviews scope before further repairs.
- [ ] Parent independently checks diff, source claims, generator tests and gate logs. Required fresh independent review must pass before commit/push.

## Task 4: Push and remove redundant checkouts

**Status:** done on 2026-11-15, under the owner exception above.

**Files:** Exact completed paths and maintenance closeout only.

**Interfaces:** Consume reviewed source. Produce origin/main containing it, only main branch, one worktree.

- [x] Commit exact reviewed candidate paths and restored research/documentation with Conventional Commits.
- [x] Push main without force. No release tag. The remote CI was skipped on purpose through the `[skip ci]` commit marker, not through a disabled workflow.
- [x] Once source is safely pushed, verify every extra worktree payload against its preserved copy, then remove registered worktrees. Dirty superseded paths may be removed only after verified backup.
- [x] Delete 32 local non-main branches and the one remote non-main branch. Keep tags, stashes, unrelated archival refs and external backup.
- [x] Verify remote heads, local heads, worktree list, git status and main/origin equality.

## Task 5: Update the next-work plan

**Status:** done on 2026-11-15.

**Files:** `docs/PLAN.md`, `CLAUDE.md`, maintenance record and a focused follow-up plan if useful.

**Interfaces:** Consume actual consolidation evidence and current backlog. Produce ordered, actionable remaining work without unsupported completion claims.

- [x] Prioritize S-141 publication separately, S-082 real desktops, S-138 baseline/semantics, native Alt/pointer spikes, real X11 and Windows runtime evidence, container installs and blocked channels.
- [x] For each priority state prerequisites, existing story/source, concrete acceptance evidence, and owner/external blockers.
- [x] Correct documentation errors found during parent review, commit and push exact documentation paths with the `[skip ci]` marker.
- [x] Report concise Arabic outcome with actual main SHA, cleanup counts, gate status and plan links.
