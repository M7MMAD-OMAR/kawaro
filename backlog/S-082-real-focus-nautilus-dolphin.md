# S-082 Focus and activation with real Nautilus and Dolphin tokens
Status: draft (2026-10-26, planner). Closes the phase-0 gap recorded in S-013 and CLAUDE.md. Needs a virtual machine.

## Story
As a user opening files from Nautilus on GNOME or Dolphin on KDE, I want the hive to take keyboard focus with the token the real file manager passes, so that Esc works, focus returns to my previous window, and no "Kawaro is ready" notification replaces the hive.

## Background
- S-013 (done) proved xdg-activation token focus in private compositors with tokens from a synthetic GTK window. Its out-of-scope section records that a real GNOME or KDE session and real Nautilus and Dolphin token delivery remain unverified (audit AUDIT-V2-003).
- The unverified path is end to end: the Nautilus python extension and the Dolphin service menu obtain an activation token, the request carries it over IPC, and the daemon activates the hive surface with it. Private compositor evidence does not close this gap, so the roadmap keeps it open until it has its own story and evidence.

## Decisions
1. All runs happen in a virtual machine (GNOME Boxes or virt-manager) with recorded distribution, session and package versions. Never on the maintainer's live desktop.
2. The deliverable is a written report at `docs/focus-live-validation.md` (English) with the exact commands, versions, log lines and one screenshot per passing case. A failure is recorded as a defect with its logs and becomes a new story; this story fixes nothing.
3. No PASS is claimed without the recorded evidence for that case. The report states the token flow observed (extension or service menu obtains a token, request carries it, daemon activates with it), not just the final focus result.

## Acceptance criteria
- AC1: in a virtual machine running a real GNOME session with real Nautilus and the installed Kawaro extension, opening selected files from the context menu gives a hive that takes keyboard focus (`hive focus: true` in the log), Esc closes it, focus returns to the previous window, and no "Kawaro is ready" notification appears. Recorded as log lines and a screenshot in the report.
- AC2: the same case passes in a real KDE Plasma session with Dolphin and the installed service menu, at the default focus-stealing prevention level and at Extreme (4).
- AC3: the report shows the token flow with log lines from the integration side (token obtained and passed with the request) and the daemon side (token received and `xdg_activation_v1.activate` called with the hive surface), so focus success is attributed to the token and not to a lenient policy.
- AC4: a failure at any step is recorded as a defect with the exact compositor versions and logs, and the report claims PASS for that case only when AC1 to AC3 evidence exists. The report names the full range of cases run (C1, C2 and so on).
- AC5: the report contains no forbidden personal paths and no em dash or en dash; `scripts/design-lint.sh` passes over the changed files.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | VM run on GNOME with Nautilus: recorded log lines, screenshot and the notification check | live, in a VM |
| AC2 | VM run on KDE with Dolphin at both focus-stealing levels | live, in a VM |
| AC3 | Read the integration and daemon logs for the token lines in both VM runs | live, in a VM |
| AC4, AC5 | Reviewer checks the report structure, evidence per case and wording | review |

## Out of scope
- Fixing any focus or activation defect found; those go to a new story with the failing evidence attached.
- The maintainer's live desktop, the shortcut paths (S-075) and the hive placement work (S-077).
- Windows focus behaviour.

## Review log
-
