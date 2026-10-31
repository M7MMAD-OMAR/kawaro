# S-075 Alt then drag works out of the box from the installed package
Status: split: S-111 (Hyprland bind generator and doctor states, agents), S-075b (KDE and GNOME native paths, needs private KWin and GNOME sessions). See `backlog/S-111-hyprland-alt-bind-generator.md`.
Previous status: re-baselined (2026-10-26, maintainer direction, after audit AUDIT-V2-001). The goal is unchanged and stays the maintainer's top priority; the Alt-only GlobalShortcuts mechanism of the first draft is disproven and removed.

## Story
As a user of the installed package, I want to hold Alt while dragging files and drop them on the hive, with nothing to set up, so that the drag path (my main way to use Kawaro) works the day I install it, as it did in the developer install.

## Evidence base (measured, 2026-10-26)
The portal spike at `a local scratch path` (base 25f9254) measured real portals and real file drags on the installed versions:

- KWin 6.7.5 with xdg-desktop-portal-kde 6.7.5 and GNOME Shell 50.6 with xdg-desktop-portal-gnome 50.0.
- Alt-only GlobalShortcuts: BindShortcuts replied success but produced zero Activated and Deactivated signals and zero working Alt-drag conversions (0 of 4 drags on each desktop), including a 600 ms hold and a 150 ms tap. GNOME reported trigger value `0x00` and lost the binding after restart; KDE reported no key description at all.
- Super+Z (`LOGO+z`) control on the same backends: 21 PASS and 0 FAIL on each desktop, 3 of 3 real drags converting plus 1 of 1 after restart.
- Consequence: no story may claim Alt through the GlobalShortcuts portal on KDE or GNOME. `PREFERRED_TRIGGER = "LOGO+z"` in `crates/daemon/src/portal.rs` stays as the named portal fallback. Hyprland compositor binds were outside this spike and must be measured before the Hyprland Alt claim counts.

## Per-desktop contract (the only valid Alt claims are the measured ones)
| Desktop | Drag-key mechanism | State | Evidence that makes the claim count |
| --- | --- | --- | --- |
| Hyprland | Compositor bind of the drag key, written by `kawaro integration install` and repaired by the daemon | To be measured (S-075a) | A recorded run of the private Hyprland harness (`tests/hyprland/repro.sh`) showing Alt held past `hold_ms` during a real drag opens the drop hive |
| KDE Plasma | Native shortcut API, KGlobalAccel first, only if measured | Spike first (S-075b) | The spike log records the exact D-Bus calls, the installed versions and a real drag producing Activated, Deactivated and the hive |
| GNOME | Native keybinding API, Shell extension first, only if measured | Spike first (S-075d) | Same as KDE, on the installed GNOME Shell in a private session |
| Any desktop where native Alt is impossible or the spike is negative | Portal fallback, named non-Alt trigger (Super+Z) | Measured PASS (spike control) | The Super+Z control results above, reproduced by the existing portal harness |

## Owner priority and conflict semantics (maintainer decision 2026-10-26)
1. Alt held during a drag is THE way to use Kawaro. The right-click menu is secondary.
2. Per desktop exactly one drag trigger is registered: the native Alt binding where a measured native API exists, otherwise the named portal fallback. Never both, since two triggers could open two hives.
3. The portal fallback is named and visible: the docs and `kawaro doctor` say which key the user presses on which desktop. The fallback is never presented as Alt.
4. The key is configurable (`drop_key`, S-076) and Alt is the default everywhere.
5. Conflicts: a conflict is any existing binding of the chosen key by the desktop or another application. The daemon never steals, overwrites or rewrites a foreign binding. On a conflict it leaves the foreign binding alone, registers nothing for that key, and `kawaro doctor` prints `conflict: <key> taken` in its one state line.
6. Failure of a native registration (API missing, extension refused, key reserved by the compositor) falls back to the named portal fallback and reports `fallback <key> active`, never silence. A remembered portal denial (`portal_shortcut == false`, S-014f) is not re-prompted and is reported as `unavailable (denied)`.
7. Repair: at start the daemon checks that the binding it owns is present and working for this session and repairs only its own artifacts (the Hyprland bind snippet and files it generated). Generated content never contains a path under `$HOME/.local`; it uses the binary found through `PATH` or `/usr/bin`.

## Acceptance criteria
- AC1 (Hyprland): after `dnf install` of the RPM on a fresh user with Hyprland, holding Alt past `hold_ms` during a real file drag shows the drop hive at the drag position, with no manual step. Verified by a recorded run of the private Hyprland harness. No Hyprland Alt claim counts without that record.
- AC2 (KDE): the KDE spike records, for KGlobalAccel and any other candidate native API, the exact calls, versions and whether Alt held during a real drag produces Activated, Deactivated and the hive on KWin. A recorded negative result is a valid outcome and moves KDE to the fallback. Native KDE code exists only on a measured API.
- AC3 (GNOME): the GNOME spike gives the same record for a Shell extension keybinding and any other candidate native API on GNOME Shell. A recorded negative result moves GNOME to the fallback. Native GNOME code exists only on a measured API.
- AC4 (fallback): wherever the contract ends in the fallback, the named trigger is Super+Z, the docs name it per desktop, and the portal path reproduces the spike control: 3 of 3 real drags converting plus 1 of 1 after restart on both KWin and GNOME, with the 600 ms hold showing Activated then Deactivated at release.
- AC5: no generated file or bind snippet contains `.local/bin`; a test greps the generator output.
- AC6: removing the binding the daemon owns and restarting the daemon restores it; a foreign conflicting binding is never rewritten (both proven by test).
- AC7: `kawaro doctor` prints exactly one state line per desktop with one of `native Alt active`, `fallback <key> active`, `conflict: <key> taken`, `unavailable (denied)`. A taken key logs one line and the daemon stays alive.
- AC8: an install over an old developer install (binds pointing to `~/.local/bin`) rewrites only the binds Kawaro owns to the installed binary.
- AC9: the 150 ms budget after `hold_ms` holds on every measured desktop: three 60 fps trials, all within 150 ms and 9 frames or fewer.
- AC10: one `CHANGELOG.md` line in English and Arabic, gates in CLAUDE.md pass, no changed file contains an em dash or en dash.

## Execution split (one work day at most per substory)
| Story | Scope | Criteria |
| --- | --- | --- |
| S-075a | Hyprland compositor bind for Alt: `kawaro integration install` snippet, package post-install for the active user, daemon first-start repair, and the private Hyprland harness measurement that makes the Alt claim count | AC1, AC5, AC6, AC8 |
| S-075b | KDE native Alt spike (prerequisite to S-075c): measure KGlobalAccel and other candidate native APIs in a private KWin with a real drag. Output is the evidence log and the chosen API or a recorded negative. No product code | AC2 |
| S-075c | KDE native Alt registration from the API measured in S-075b, or fallback wiring when the spike is negative | AC2, AC4, AC7, AC9 |
| S-075d | GNOME native Alt spike (prerequisite to S-075e): measure a Shell extension keybinding and other candidate native APIs in a private GNOME Shell with a real drag. Output is the evidence log and the chosen API or a recorded negative. No product code | AC3 |
| S-075e | GNOME native Alt registration from the API measured in S-075d, or fallback wiring when the spike is negative | AC3, AC4, AC7, AC9 |
| S-075f | Fallback and conflict contract: named Super+Z fallback, owner priority and conflict semantics in `docs/drop-shortcut.md` and the install guides, the `kawaro doctor` state line, the remembered portal denial | AC4, AC7, AC10 |

The native spikes S-075b and S-075d are prerequisites: S-075c and S-075e do not start before their spike result is recorded in this story's review log. Spikes run in private compositors only, never on the live desktop.

## Test cases
| Criterion | Test | Type |
| --- | --- | --- |
| AC1 | Private Hyprland harness run with the installed package and a real drag; recorded log and screenshot in the story log | integration |
| AC2, AC3 | Spike harness runs per candidate API with a real drag and a D-Bus monitor log; the result is copied into the review log | spike |
| AC4 | Existing portal harness with `LOGO+z` on KWin and GNOME: conversions, restart binding, hold and tap timings | integration |
| AC5 | Grep test over generated bind snippets and integration files | unit |
| AC6 | Remove the owned binding, restart the daemon, assert restoration; plant a foreign binding on the same key, assert it is untouched | integration |
| AC7 | `kawaro doctor` output test per simulated state; daemon log test for the taken-key line | unit |
| AC8 | Fixture of a developer-install bind file, run the installer step, assert only owned binds changed | integration |
| AC9 | Timing measurement script on each measured desktop path | performance |

## Out of scope
The shortcut picker UI (S-076), X11 (S-015), any claim of Alt through the GlobalShortcuts portal on KDE or GNOME (measured impossible on the tested versions), experiments on the live desktop.

## Review log
- 2026-10-26, planner: re-baselined after audit AUDIT-V2-001. The Alt-only portal mechanism is removed; the contract is per desktop with measured evidence, native spikes as prerequisites, and a named non-Alt portal fallback with recorded owner priority and conflict semantics.
