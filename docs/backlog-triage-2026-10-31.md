# Backlog triage, 2026-10-31

Read-only triage of the open backlog stories against the code, the tags (v0.1.0 to v0.4.7), `CHANGELOG.md` and the tests present. Method: grep and reading only; no cargo command and no test suite was run. A story counts as done only when its acceptance criteria have passing tests or a recorded manual result. "Agents can finish" means with the fast loop (fmt, clippy and tests of touched crates, plus scan tests and shell tests) on a Linux machine with a private Hyprland, KWin or GNOME harness; anything else names what is needed.

## Status lines changed in this pass

| Story | New status | Evidence |
| --- | --- | --- |
| S-040 | done (released in 0.4.0) | `crates/core/src/transport/windows.rs` (397 lines) exists on main; `CHANGELOG.md` 0.4.0 lists "IPC behind one transport (S-040)"; both reviewer PASS records are in the story log; tag v0.4.0 contains the merge. The old "0.6.0 branch" wording was history only. |
| S-056 | done | The site is built by `site/build.py`, deployed by S-059 (done), revised by S-061, S-063, S-064, S-071 (all done); `tests/site/test_static.py` and the browser tests exist; `CHANGELOG.md` has the S-056 line. |
| S-057 | done | `scripts/build-site.sh` builds docs, releases, Pagefind index (`site/pagefind`), sitemap, `llms.txt`, `llms-full.txt`; `CHANGELOG.md` has the S-057 line; the "Pages workflow by hand" criterion is superseded by S-059 (Cloudflare Workers Builds). |

## Old stories: already carry a done status

S-001 to S-004, S-006 to S-013, S-018 to S-022 carry `الحالة: منتهية` and S-014, S-027 to S-030 carry `done`; each has its CHANGELOG line (S-001, S-002, S-003, S-004, S-006, S-007, S-008, S-009, S-010, S-011, S-012, S-013, S-017 (partial), S-020, S-021, S-022, S-027) and was released in 0.1.0 to 0.3.3. No edit needed. Two notes:

- S-013 is done for private compositors only; the real Nautilus and Dolphin token path is open and owned by S-082. `CLAUDE.md` already says so.
- S-017 (glass hive, not in the request) has a mixed status: S-017a to S-017c done at the UI level, S-017d (compositor blur) and S-017e (snapshots, performance, release gates) open. Leave as is.
- S-058 to S-071, S-073, S-074, S-076a, S-079 to S-081, S-083 to S-088, S-090, S-093 to S-108 are out of scope here; S-076b is approved and pending (not triaged).

## Verdicts for the live stories

### S-015 X11 hive (status: draft, Arabic file)

- Evidence: none in code. `crates/ui/src/layer.rs` returns `false` when `Connection::connect_to_env` fails and no X11 positioning, XDND or `QueryPointer` code exists; `tests/x11/` does not exist; no CHANGELOG line. The only X11 references in `crates` are the Wayland-first fall-throughs.
- Missing: everything (S-015a to S-015c, AC1 to AC14). Three manager decisions are still marked as waiting for the maintainer (corner shape by X Shape or opaque rectangle, drop hive position, highlight by polling), plus the X11 harness tool (`x11rb` helper outside the workspace).
- Needs: Xvfb and xfwm4 are installed on this machine, so the harness is possible without a person. The decisions are the only human gate, and the delegation rule says the manager may decide them. Real X11 desktops (GNOME and KDE on X11) stay out of scope in the story.
- Can agents finish: yes for AC1 to AC13 (private Xvfb, not the live desktop), AC14 docs by agents. Risk: winit XDND and GTK4 `GDK_BACKEND=x11` behaviour is unverified.
- Size: L (about 3 days, three substories). Slated for 0.5.0.

### S-016 Signed APT and COPR repositories (status: approved)

- Evidence: the APT builder `packaging/apt/build-repo.sh` and its upload `scripts/channels/apt.sh` exist, with `tests/release/s103f_apt.sh`; the public key `packaging/keys/kawaro-archive.asc` is committed; `packaging/copr/kawaro.spec` and `scripts/channels/copr.sh` with `tests/release/s103d_copr.sh` exist (built under S-103d, not S-016c); `docs/distribution-status.md` records apt as live (0.4.7, served from GitHub Pages at the `kawaro-apt` repository, not Cloudflare R2 as decision 1 says) and COPR as skipped (no account).
- Missing: `packaging/install-commands.json` and its renderer (decision 8, AC11); the container matrix for Ubuntu 22.04, 24.04 and Debian 12 with tamper cases (AC2, AC3); the signing hygiene tests (AC4); signed `SHA256SUMS.asc` and `kawaro-archive.asc` on releases (AC10; a grep for `SHA256SUMS.asc` finds nothing); the Fedora 43, 44, 45 test matrix and live COPR checks (AC8, AC9); live second-tag upgrade checks (AC6). The story text still names R2, a different key path and COPR chroots that have since shifted, so it needs a re-baseline against S-103 before more work.
- Needs: AC10, AC11, AC1 to AC4 are agent work. A COPR account (person) gates AC8 and AC9. Containers for the apt matrix need podman or docker on this machine (heavy; run inside the 6 GB scope). A person must confirm whether the R2 plan is dropped for good.
- Can agents finish: partly (AC1 to AC4, AC7, AC10 to AC12 yes; AC5, AC6 superseded by GitHub Pages and need an amended story; AC8, AC9 need the account).
- Size: M for the agent part, L overall.

### S-040 Windows IPC

Done, see the table above. No remaining work.

### S-072 Launch assets and rename announcement (status: planned)

- Evidence: the story file is a 6 line stub with no acceptance criteria or tests. `docs/launch/` holds `rename-announcement.md`, `intro-video-plan.md`, carousel sources (`carousel-0.4.1`, `carousel-day2` to `carousel-day4`) and `build.sh`.
- Missing: a real definition of done. There is nothing to test against.
- Needs: a person to say which assets and which channels count; publishing is not an agent action. Assets can be produced by agents.
- Can agents finish: no, not as written. Recommend a planner pass to write criteria first (asset list, sizes, bilingual copy review), then agents build the assets; posting stays with the maintainer.
- Size: S for criteria, M for assets. Lowest priority, non-blocking by decision.

### S-075 Alt drag by default (status: re-baselined)

- Evidence: Hyprland Alt works through user written binds documented in `integrations/linux/hyprland/README.md` and `docs/drop-shortcut.md`; real Alt drag runs exist in `tests/hyprland/altdrag.sh` (S-095, S-102, S-104, released in 0.4.3 and 0.4.4); `kawaro doctor` prints a `drop key:` line (`crates/core/src/hold.rs:174`) but not the four states of AC7; `PREFERRED_TRIGGER = "LOGO+z"` is the portal fallback (`crates/daemon/src/portal.rs:16`); `kawaro integration install` exists (`crates/cli/src/integration.rs`). The two untracked directories under `spikes/` are the start of S-075b and S-075d; they are not recorded in the story log and were not touched.
- Missing: S-075a (the compositor bind generated by `integration install`, daemon first start repair, AC5 grep test, AC6 and AC8 tests; the README binds still contain `~/.local/bin`); S-075b and S-075d spike results (no recorded log); S-075c and S-075e (blocked on spikes); S-075f (`conflict:` and `native Alt active` doctor states, none in code).
- Needs: S-075a, S-075f and the unit tests of AC5 to AC8 are agent work. The KDE and GNOME spikes need a private KWin and a private GNOME Shell session with a real drag; both harnesses exist (`tests/overlay/kwin.sh`, `tests/drop/gnome.sh`, `tests/drop/kwin.sh`) but are compositors other than Hyprland and heavy. The portal fallback was already measured (Super+Z).
- Can agents finish: S-075a and S-075f yes; S-075b and S-075d yes as spikes if the harness runs (a negative result is valid); S-075c and S-075e only if a spike is positive. AC9 timings need the same compositors.
- Size: L overall (S-075a M, S-075f S, each spike M, each native registration M). Maintainer's top priority.

### S-077 Hive position from the menu (status: approved)

- Evidence: `Request::Open` carries only `files` and `token` (`crates/core/src/ipc.rs:25`); `kawaro open` has no `--at` (the `at` option in `crates/cli/src/main.rs:62` is for `drop`); `integrations/linux/hyprland/kawaro-open-selection.sh` ends with `kawaro open --from-clipboard` without a position; no commit mentions S-077 beyond planning.
- Missing: all of S-077a (reproduction record, `--at` plumbing, Hyprland launcher with `hyprctl cursorpos`, fake `hyprctl` test, `tests/integrations/test_hyprland.sh` update per D2) and S-077b (KDE pointer-source spike, focused-screen fallback, GNOME regression, docs, CHANGELOG).
- Needs: S-077a plumbing, the fake `hyprctl` test and AC2 are unit and shell tests (fast loop). AC3 and AC4 need the private nested Hyprland and private KWin. The KDE spike (S-077b) needs a private KWin session and Dolphin; AC9 a private GNOME session. AC1 asks for a live reproduction on the maintainer's session, which only the maintainer can give; a recorded reason it was unavailable is allowed by the criterion.
- Can agents finish: S-077a yes (AC1 with the "not available live" record). S-077b only partly: the spike and fallback in private KWin yes if the harness runs, GNOME regression yes in the private GNOME harness.
- Size: M for S-077a, M to L for S-077b.

### S-078 Free distribution (status: in progress)

- Evidence: winget half submitted ([winget-pkgs#462805](https://github.com/microsoft/winget-pkgs/pull/462805)); `scripts/release/check-winget-scope.sh`, `scripts/release/check-winget-versions.sh` with `tests/release/s078_winget_scope.sh` and `s078_winget_versions.sh`; AUR generator `packaging/aur/generate.sh` with templates and `known_hosts`, `scripts/channels/aur.sh`, `tests/release/s103e_aur.sh` (built under S-103e, channel skipped: no AUR account); winget channel `scripts/channels/winget.sh` and `tests/release/s103b_winget.sh`. The AppStream file `packaging/io.github.m7mmad_omar.Kawaro.metainfo.xml` has English release entries only, with no `<icon>` and no `<screenshot>` (zero matches).
- Missing: S-078a (stock icon, screenshots with English and Arabic captions, bilingual releases, `appstreamcli validate-tree`); AC2 and AC6 (install command JSON shared with S-016, not built; PR automation `S-078e` partly covered by S-103b, idempotency unproven); AC4 (live AUR publication, needs the account); AC7 (merged winget catalog, waits for the Microsoft review); AC8 finish.
- Needs: AppStream work (AC1) is agent work except the live screenshot check, which needs the screenshot files on the site (site deploy by the maintainer's Cloudflare build). AUR needs a person to create the account and register a key. winget needs only time (a Microsoft moderator) and a Windows machine or sandbox for AC5/AC7 live checks.
- Can agents finish: S-078a yes (needs `appstreamcli` in the container), S-078b offline artifact checks yes (clean Arch container needed for the install part), AC2 together with S-016 yes. S-078c, AC7 no.
- Size: M for the agent part, L overall because of external gates.

### S-082 Real focus test with Nautilus and Dolphin (status: draft)

- Evidence: none; `docs/focus-live-validation.md` does not exist; the review log is empty. The private compositor tests (`tests/focus/gnome.sh`, `tests/focus/kwin.sh`) are the S-013 evidence and explicitly do not count.
- Missing: the whole deliverable (report with commands, versions, log lines, screenshots, token flow on the integration side and the daemon side, for GNOME and for KDE at default and at Extreme focus-stealing prevention).
- Needs: a virtual machine with a real GNOME session and another with real KDE Plasma (GNOME Boxes or virt-manager), plus a person to drive or at least approve the run and capture screenshots. This cannot be done in a private compositor by definition.
- Can agents finish: no. Agents can prepare a runbook and a log checklist (S) but not the evidence.
- Size: M (a day per desktop), needs a person and a VM.

### S-089 First run and settings recovery (status: draft)

- Evidence: shipped in 0.4.1 by e32ce3a: immediate language choice (`choose-language` in `crates/ui/ui/welcome.slint`, `crates/ui/src/welcome.rs:572`), one window with the introduction and the settings pages (`crates/daemon/src/ui.rs`), the `notify_long_only` choice (`crates/core/src/config.rs:86`, job timing in `crates/daemon/src/jobs.rs`). Later stories cover other criteria: S-094 (shell, done), S-094c (live vector film instead of a video), S-099, S-106, S-107 (foreground), S-085 (icon).
- Missing against its own criteria: AC3 asked for a bundled real video; the shipped introduction is a live vector scene (S-094c), so AC3 is deliberately not met as written. AC7 asked for Off, All and Long (10 s) as a tri-state; the code still has two booleans `notify` and `notify_long_only`, with the 10 second rule but no recorded fake clock tests for 9.999 s and 10 s found in this read. AC1, AC6, AC8, AC9 have no per-criterion test mapping or recorded matrix; the live Windows matrix was only partly run (a VM run exists for S-106 and S-107; physical hardware not checked per `docs/distribution-status.md`).
- Needs: a person (or VM) for the Windows matrix and the owner review; agents can write the AC to test matrix, close AC7 tests and settle AC3 by an amended decision.
- Can agents finish: no as written. Recommended: mark it superseded in part by S-094, S-094c, S-099, S-106, S-107, and cut a small remaining story (notification tri-state tests, config key inventory AC8, accessibility checklist AC9).
- Size: M for the remainder.

### S-094b About page actions (status: draft)

- Evidence: S-094 shell and the About page exist (`crates/ui/src/shell.rs`, `crates/ui/ui/settings.slint`). `open_url_command` exists in `crates/daemon/src/notify_logic.rs:32` (checked by `crates/core/tests/s108_sources.rs:65`). No `kawaro_ui::open_url`, no About links, no Copy version button, no "Check for updates" shortcut on About (no matches); the `open_url` in `crates/ui/src/platform.rs:85` is the Slint platform pass-through, not this feature.
- Missing: AC1 to AC5 entirely.
- Needs: nothing outside agents for the code, tests and snapshots (unit tests with a spy launcher, fake clipboard, fake IPC). The Windows live click check is left to the owner by the story, and is not a gate.
- Can agents finish: yes.
- Size: S to M.

### S-091 and S-092

Absorbed by S-099, leave.

## Recommended order

1. S-094b (S, no external gate, finishes a visible gap).
2. S-089 remainder, after cutting it into a small story (S to M): closes owner reported Windows items with tests.
3. S-077a (M): plumbing and the Hyprland launcher, fast loop only; then S-077b in private KWin and GNOME.
4. S-075a and S-075f (M): the Hyprland bind generator, repair, doctor states; this is the maintainer's top priority. Then the KDE and GNOME spikes (S-075b, S-075d), then S-075c and S-075e only for positive results.
5. S-078a (AppStream) and S-016 re-baseline with the shared `install-commands.json` and `SHA256SUMS.asc` (M each); both feed S-078 AC2.
6. S-015 (L, 0.5.0): independent of the others, needs the three decisions, runs in private Xvfb.
7. S-082 (needs a person and VMs): schedule when a VM is available; it closes the phase 0 gap and blocks no story.
8. S-072 (planner first, then assets); S-078c, S-016 live checks and AC7 of S-078 wait for accounts and the Microsoft review.

## Summary table

| Story | State | Agents can finish | Size |
| --- | --- | --- | --- |
| S-001 to S-014, S-018 to S-022 | done (already marked) | n/a | n/a |
| S-040 | done (0.4.0), status updated | n/a | n/a |
| S-056, S-057 | done, status updated | n/a | n/a |
| S-015 | not started | yes (private Xvfb), decisions by manager | L |
| S-016 | partly built under S-103, needs re-baseline | partly (COPR live needs an account) | M agent, L total |
| S-072 | stub, no criteria | no, needs criteria and a person to publish | S + M |
| S-075 | Hyprland measured, rest not started | partly (spikes need private KWin and GNOME) | L |
| S-077 | not started | partly (S-077a yes, KDE spike needs private KWin) | M to L |
| S-078 | winget submitted, AUR scripted, AppStream not started | partly (accounts and Microsoft review gate the rest) | M agent, L total |
| S-082 | not started | no (needs VMs and a person) | M |
| S-089 | mostly shipped, remainder unverified | partly (live Windows matrix needs a person) | M |
| S-094b | not started | yes | S to M |
| S-091, S-092 | absorbed by S-099 | leave | n/a |
