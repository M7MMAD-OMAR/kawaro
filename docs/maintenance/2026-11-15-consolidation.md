# Consolidation of branches and worktrees, 2026-11-15

Record of the consolidation of every non-`main` branch and every extra worktree of this
repository into `main`.

The baseline, branch, worktree and backup sections below were written while the
repository was still untouched, so they describe a **historical** state: at that moment
nothing had been pushed, committed, merged, deleted or rewritten. Later passes then
executed the restoration, the source push and the cleanup. Where this document still
says "pending", read it as the historical baseline, and read the "Closeout" section at
the end for the authoritative final state. The final closeout wins over every earlier
statement in this file.

The implementation was done by the selected Pi implementation agent
(`pi/minimax/MiniMax-M3.1-Flash`); the parent agent owned the planning and the final
review. No software factory job was used. The full local gate set and the remote CI
were deferred by owner order for the source consolidation push, so no gate result is
claimed for it.

## Baseline

| Item | Value |
| --- | --- |
| Repository | `/home/sbarah/R/Projects/P/sbarhive` |
| Remote | `origin` = `git@github.com:M7MMAD-OMAR/kawaro.git` |
| `main` tip | `c0444b0858204370f40478eaec3726b612264ace` |
| `origin/main` | `c0444b0858204370f40478eaec3726b612264ace` (identical, no divergence) |
| Local branches | 33: `main` plus 22 ancestor branches plus 10 non-ancestor branches |
| Remote branches | `origin/main`, `origin/fix/ci-portability-and-fd-identity`, `origin/HEAD`. `origin/HEAD` is a symbolic ref, so exactly one remote branch exists besides `main` |
| Worktrees | 14 (the main checkout plus 13 extra) |
| `git fetch --all --prune` | already run and succeeded before this pass; nothing was fetched during it |
| Tags | `v0.1.0` to `v0.5.0`, plus `v0.4.0-beta.1` and `updates` |
| Stashes | 2 (`stash@{0}` and `stash@{1}`), both `On main: pre-filter-branch parallel work` |
| Extra refs | `refs/original/*` (5, the filter-branch backup) and one `refs/codex/turn-diffs/...` checkpoint |

`main` carries uncommitted work that must survive the consolidation:

- Tracked modifications: 56 paths. Eleven are outside the site and the images:
  `CHANGELOG.md`, `CLAUDE.md`, `Cargo.toml` (`version = "0.5.1"`), `Cargo.lock` (six
  `kawaro-*` entries moved to `0.5.1`), `README.md`, `README.ar.md`,
  `backlog/S-141-publish-0.5.1.md`, `docs/PLAN.md`, `docs/launch/assets.md`,
  `packaging/io.github.m7mmad_omar.Kawaro.metainfo.xml` and
  `tests/release/beta_versions.sh`. Thirty seven are under `site/`, including
  `site/build.py`. Eight are PNG files: four `crates/ui/tests/snapshots/settings-*.png`
  and four `docs/screenshots/s094/A-general-*-default.png`.
- Untracked files at the time of the baseline: `docs/release-notes-0.5.1.md`,
  `docs/release-notes-0.5.1.ar.md`, `docs/whats-new/0.5.1.json`, plus the two nested
  spike worktree directories `spikes/gnome-alt-extension/` and `spikes/kde-alt-native/`,
  which were registered worktrees rather than loose files. This document,
  `docs/maintenance/2026-11-15-consolidation.md`, is a sixth untracked entry added by
  this pass. The GNOME worktree was parked and its seven source files restored on
  2026-11-15, see "Executed restoration" below; `spikes/kde-alt-native/` is still a
  registered worktree.

That is the 0.5.1 candidate metadata set of story S-141. It is preserved in the backup
and stays untouched.

## Backup

Location: `~/kawaro-consolidation-2026-11-15` (outside the repository).

| Item | Path | Note |
| --- | --- | --- |
| Git bundle of every ref | `bundles/kawaro-all-refs.bundle` | 41,050,549 bytes, 173 recorded heads including `refs/stash`, `refs/original/*` and the 13 `worktrees/*/HEAD` refs |
| Ref inventory | `refs/all-refs.txt`, `refs/local-branches.txt`, `refs/show-ref.txt`, `refs/worktrees.txt`, `refs/all-commits.txt`, `refs/stashes.txt`, `refs/main-untracked.txt` | plain text |
| `main` tracked diff | `patches/main-tracked.patch` (1,059,562 bytes, binary safe), `patches/main-tracked.stat.txt`, `patches/main-staged.patch` | covers the 0.5.1 metadata, the eight PNGs and the site tree |
| `main` untracked release notes | `worktree-payloads/main/main-untracked-release-notes.tar.gz` | the three `0.5.1` release-note files |
| `s016a` worktree diff | `worktree-payloads/s016a/tracked.patch`, `tracked.stat.txt` | the one modified file `tests/release/run_all.sh` |
| `s016a` untracked payload | `worktree-payloads/s016a/untracked-apt-tests-and-install-commands.tar.gz` | `tests/apt/{lib,repo_builder,signing,containers,run_all}.sh` and `tests/release/install_commands.sh` |
| `s016a` branch-only sources | `worktree-payloads/s016a/build-apt-repo.sh`, `install-commands.s016a.json` | files that exist on the branch but not on `main` |
| GNOME Alt extension spike | `worktree-payloads/spikes-gnome-alt-extension/gnome-alt-extension-source.tar.gz` | the seven source files of `spikes/gnome-alt-extension/` from the detached worktree |
| GNOME Alt extension spike, working copy | `staging/gnome-alt-extension/` | a second copy of the same seven files, taken outside the repository before the worktree was moved and compared with the archive above |
| Parked GNOME worktree | `parked-worktrees/gnome-alt-extension/` | the registered detached worktree, moved out of the repository with `git worktree move` on 2026-11-15 and kept intact, not deleted |
| Branch-only files | `worktree-payloads/branch-only-files/` | five files listed in the branch table below |

### Backup verification

- `git bundle verify bundles/kawaro-all-refs.bundle` reports "The bundle records a complete history."
- A real restore was performed into `~/kawaro-bundle-verify/restore-check` by cloning
  the bundle. The restored `main` tree is byte identical to the live `main` tree
  (`f19a0ed3f2990afcf6e7151165e029416eccdaa4`, compared over `git ls-tree -r`).
- All 18 unique commits of every branch are present as commit objects in the restored
  clone, including `5b6a552` (spike/alt-portal), `1ca352f` (story/s076a tip),
  `b8522a5` (story/s017a tip), `06d5397` (story/s016a tip) and `7b63bf8`
  (the goldfish review).
- Both stashes are present in the restored clone (`d5795fd` and `337373e`).
- The restore directory was a scratch copy for verification only. It has been deleted.
- `patches/main-tracked.patch` was checked with `git apply --check --3way` against a
  fresh clone of the repository at `main`: all 56 paths apply cleanly, including the
  eight binary PNG files.
- Every payload archive was extracted and compared with its source: the `s016a`
  untracked APT tests and `install_commands.sh` are byte identical, the `s016a`
  tracked patch is byte identical, and the GNOME spike tree is byte identical.
- The verification clones were removed after the check.

### What the backup deliberately excludes

- Every `target/` directory, every `node_modules/` directory, every
  `spikes/*/out/*.log` and `*.txt`, and the `.claude/worktrees/` directory: build
  output, reproducible from source.
- The whole nested checkout of each worktree (936 tracked files per worktree): the
  content of every worktree is already in the bundle through its `worktrees/*/HEAD`
  ref and its branch tip.
- The four `debug_test*` binaries and four `debug_test*.rs` files committed by
  `10462b5` on `story/s081`: build artifacts and scratch files, never worth keeping.
- `spikes/gnome-alt-extension/spikes/gnome-alt-extension/` build products: the seven
  source files are kept, nothing else existed in that tree.

## Branch disposition

Reachability is measured against `main` at `c0444b0`. `ahead` is
`git rev-list --count main..BRANCH`. `patch+` is the count of `+` lines from
`git cherry main BRANCH`, that is, commits whose patch is not already in `main`.

Every branch below is **pending** deletion from the local repository, and the one
remote branch other than `main`, `origin/fix/ci-portability-and-fd-identity`, is
**pending** deletion on `origin`. Nothing has been deleted.

### Ancestors, patch already in `main` (22 branches)

| Branch | Tip SHA | Ancestor of `main` | ahead | behind | patch+ | Disposition and reason |
| --- | --- | --- | --- | --- | --- | --- |
| `claude/lfm3-local-models-d95de0` | `5e0ef9ac266d2cc4ba523212114116208335a563` | yes | 0 | 226 | 0 | Ancestor. Its content (the LFM3 local model research note) is in `main` at `docs/research/lfm3-local-models.md`. |
| `fix/ci-portability-and-fd-identity` | `23e7665863ceed4aa0b3924b9ec52580305b08b5` | yes | 0 | 1 | 0 | Ancestor, merged as PR #11 into `main`. The remote copy of this branch is still on `origin` and is **pending** remote deletion. It is the only remote branch other than `main`. |
| `hermes-subagent/subagent-sa-0-3abc6e46` | `8dcc37334a925343f6b75f7a5f1b9a052c64dad4` | yes | 0 | 291 | 0 | Ancestor. |
| `hermes-subagent/subagent-sa-0-3bd55809` | `679d16a28b5b149778e5143f64f09c00fe38303e` | yes | 0 | 253 | 0 | Ancestor (the old history merge). |
| `hermes-subagent/subagent-sa-0-4285d610` | `eca711faa640607e34df7274f500bf3558bea934` | yes | 0 | 264 | 0 | Ancestor. |
| `hermes-subagent/subagent-sa-0-660a521a` | `b3a6e5850c8a2941a2f46a538ae9f260bb2fde15` | yes | 0 | 262 | 0 | Ancestor. |
| `hermes-subagent/subagent-sa-0-6a6f85e3` | `cc3b88664a8eb7de544677b29d9a8f6d044d7ba5` | yes | 0 | 261 | 0 | Ancestor. |
| `hermes-subagent/subagent-sa-0-99e93a1f` | `3b96912b260c875e59083cdbffedbe1cdb671aa2` | yes | 0 | 256 | 0 | Ancestor. |
| `hermes-subagent/subagent-sa-0-bbd43063` | `cb31bd8f6a52bea1cb2e0d403551f12ae4645b80` | yes | 0 | 269 | 0 | Ancestor. |
| `hermes-subagent/subagent-sa-0-e775bbfb` | `3b96912b260c875e59083cdbffedbe1cdb671aa2` | yes | 0 | 256 | 0 | Ancestor, same tip as `subagent-sa-0-99e93a1f`. |
| `hermes-subagent/subagent-sa-0-e93a08ff` | `c4773fdfbc01272b082da07f844243b6dc970c51` | yes | 0 | 301 | 0 | Ancestor (S-017 manager decisions). |
| `hermes-subagent/subagent-sa-1-3d1b014d` | `3b96912b260c875e59083cdbffedbe1cdb671aa2` | yes | 0 | 256 | 0 | Ancestor, same tip as `subagent-sa-0-99e93a1f`. |
| `hermes-subagent/subagent-sa-1-4c5a1410` | `374edc512b3eddf39a69f31b620e690c904ad5f8` | yes | 0 | 291 | 0 | Ancestor. |
| `hermes-subagent/subagent-sa-1-ddc3f7d3` | `86e977e099ff1517b22ca293dbb94efa97ec3dd2` | yes | 0 | 274 | 0 | Ancestor. |
| `hermes-subagent/subagent-sa-2-b673735c` | `ecd6cbc5371fe4d551e5e8dea7f992def2de5d7f` | yes | 0 | 293 | 0 | Ancestor. |
| `release/0.5.1` | `c0444b0858204370f40478eaec3726b612264ace` | yes | 0 | 0 | 0 | Same SHA as `main`. A release branch that was never cut separately. **Pending** deletion; the 0.5.1 tag, when it is created, is a tag, not this branch. |
| `story/integration` | `a56a205d65dee2bf86d68acbf8fa2e61e4975f4b` | yes | 0 | 296 | 0 | Ancestor. |
| `story/roadmap-main` | `3d906011fe61a2eb352cb6e40ca90db8133bcd3a` | yes | 0 | 320 | 0 | Ancestor. |
| `story/s079` | `ac9d7e1459dbf0332fc88ceda491ccb03064a131` | yes | 0 | 315 | 0 | Ancestor. |
| `story/s083` | `f6ef1561af8409e99ab807f33135b8f9e0e3285c` | yes | 0 | 304 | 0 | Ancestor. |
| `story/s087` | `8ae0b3c7c91e12477ceaeea64a792a6537e789ea` | yes | 0 | 278 | 0 | Ancestor. |
| `story/winfix` | `60c6b7a4b15d85623db91fab03a7476028e01338` | yes | 0 | 303 | 0 | Ancestor. |

### Branches with unique commits (10 branches, 18 distinct unique commits)

None of these is a candidate for a blind merge. Every one of them is 257 to 326 commits
behind `main`, and each carries an implementation that `main` has since replaced.
Accepting one of these obsolete, conflicting implementations risks regressions in the
newer product code, so none of them is merged. The classification below is based on
reading each unique commit's diff and comparing it with the current `main`.

The count of distinct unique commits is 18, measured with
`git rev-list main..BRANCH` over all ten branches, deduplicated by commit SHA. The
per-branch column below sums to 19, because the commit `5352fd8` is the tip of both
`story/s017a` and `hermes-subagent/subagent-sa-0-0a4d945f` and is counted once per
branch. The two commits `06d5397` and `23658ec` are different SHAs with the same
change, so they count as two commits but one change.

| Branch | Tip SHA | ahead | behind | patch+ | Unique commit(s) | Classification | Reason |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `story/s016a` | `06d539760d3b50ec69feb312b3f46f6c64babe7d` | 1 | 315 | 1 | `06d5397` | superseded | Adds `packaging/linux/build-apt-repo.sh` and rewrites `packaging/install-commands.json`. `main` has a different, later APT builder at `packaging/apt/build-repo.sh` (S-103f) and a different `install-commands.json` schema (`schema: 1` with `channels` and `status`, S-112). The branch version of the JSON has the old shape (`apt` and `copr` roots, `commands` as arrays of objects with `description` and `command`), which the current `tests/release/s112_install_commands.sh` explicitly rejects. Merging would break a passing test. Branch-only file kept in the backup: `packaging/linux/build-apt-repo.sh`. |
| `story/s017a` | `b8522a5def69d04a016e51ed8e88e5fdd11259b8` | 2 | 315 | 2 | `5352fd8`, `b8522a5` | superseded | The S-017a glass and motion tokens are already in `main`: `design/tokens.json` has the `glass` section with `glass-2` and `glass-3` and the `motion` section with `respect-reduced-motion`, `crates/ui/build.rs` emits the glass colors, `crates/ui/ui/hive.slint` consumes `Tokens.glass-2` and `Tokens.glass-3`, `scripts/design-lint.sh` exempts glass keys from the 8-digit hex rule, and `site/tokens.css` carries `--color-glass-2` and `--color-glass-3`. `crates/ui/tests/glass_tokens.rs` exists in `main` and differs from the branch copy only by the S-087 `resources` directory addition. The story file records "S-017a done (2026-10-26, glass + motion merged to main)". |
| `story/s076a` | `1ca352f4a76fe28fc525f33a4c16556db1ca8899` | 6 | 325 | 6 | `6009a9d`, `3701ddc`, `642ac6e`, `5983761`, `2a4719f`, `1ca352f` | superseded | The two S-076a features are in `main` in a later form. The portal row is gone: `crates/ui/ui/shell.slint` has no portal registration row, only the `drop-shortcut` display. The conditional Folder name field is in `main` at `crates/ui/ui/shell.slint` through `property <bool> subfolder: root.output == root.subfolder-index` and the `if root.subfolder: TextField` block. The branch version is a 316 line shorter `settings.slint` that predates the S-094 app shell split into `settings.slint` plus `shell.slint`. Merging it would undo the shell split. |
| `story/s080` | `9a4f815b792e678da91bbbc96cb4dd2dcf8513b6` | 3 | 319 | 3 | `01566c2`, `c08907b`, `9a4f815` | superseded | The shared engine lookup is in `main`: `crates/cli/src/main.rs` `doctor()` resolves every engine tool through `kawaro_engines::locate::find(program)`, with the comment "Every engine tool resolves through the engines' shared lookup so doctor and the engines never disagree (S-080)". The branch version lists only four programs; `main` lists six, adding `Program::Pdftotext` and `Program::Qpdf` from the later S-126 and S-128 engines. The branch would remove those two engines. |
| `story/s081` | `10462b5543e60adf04d76e903115656ba55774e2` | 2 | 319 | 2 | `2036816`, `10462b5` | superseded, with one candidate file | The S-081 behavior is in `main`: `crates/core/src/output_impl.rs` has `pub fn dir_collision`, `crates/core/src/output_impl/s081_tests.rs` carries the AC1 to AC4 tests, and `crates/core/src/output_impl/windows.rs` sends both paths through `win_long_path`. The branch version of `win_long_path` returns `PathBuf` and panics on a non-UTF-8 path; `main` returns `std::io::Result<PathBuf>` and refuses a non-UTF-8 path instead of panicking, so the branch is a regression. The branch also adds `debug_test` binaries and `debug_test*.rs` scratch files, which are not worth keeping. The branch test pattern is the reverse of `main`: the branch expects `(S-081)` in `CHANGELOG.md`, `main` expects `[S-081]` and `main` carries the `[S-081]` line. **Candidate missing file**: `crates/core/tests/s081_output_placement.rs` (an integration test that does not exist on `main`; its coverage is duplicated by `s081_tests.rs`, so it is optional). |
| `spike/alt-portal` | `5b6a552f95759fc98f689b504b4fd55172cd693e` | 1 | 326 | 1 | `5b6a552` | **valuable, but as tracked spike source, not as a merge** | The commit adds `spikes/alt-portal/` with 9 files and 3683 lines: a copy of the daemon portal client that uses `PREFERRED_TRIGGER = "ALT"` instead of `"LOGO+z"`, its tests, and four drop scripts (`gnome.sh`, `kwin.sh`, `portal.sh`, `portal_real.sh`) that press Alt instead of Super+Z, plus three runner scripts. The measured conclusion is already recorded in `main`: `docs/PLAN.md` line 148 and `backlog/S-075-alt-drag-by-default.md` line 12 state that Alt-only GlobalShortcuts is accepted by `BindShortcuts` but never activates on GNOME 50.0 or KDE 6.7.5, and that Super+Z passes. **The source of that measurement is not in `main`.** `main` carries the Super+Z versions of the same scripts (`tests/drop/gnome.sh`, `tests/drop/kwin.sh`, `tests/drop/portal.sh`, `tests/drop/portal_real.sh`), which differ from the spike copies only in the key that is pressed. Proposed handling: add `spikes/alt-portal/` to `main` as tracked experimental source under the existing spike convention (`spikes/drop`, `spikes/hive`, `spikes/overlay` are already tracked there), with the spike excluded from the product code chain as `CLAUDE.md` requires. It is a **pending** decision for the parent, not a merge. |
| `hermes-subagent/subagent-sa-0-0a4d945f` | `5352fd828036dfc3c30840f5e936cb9527dfecc7` | 1 | 315 | 1 | `5352fd8` | superseded | The same commit as the first of `story/s017a` (`5352fd8`). Covered by the S-017a row above. |
| `hermes-subagent/subagent-sa-0-8a845e11` | `21e38892305158f6fbfb5f00da530748aeb8f828` | 1 | 261 | 1 | `21e3889` | superseded | Adds `docs/winget-pkgs-merge-requirements.md`. That file **is** in `main`, in a later and more accurate state: the branch copy still says `isDraft=true`, the `main` copy records "the pull request is no longer a draft, it carries an Arabic locale manifest next to the English one". The branch copy is the older text. Branch-only path preserved in the backup even though the file also exists on `main`. |
| `hermes-subagent/subagent-sa-0-b46d8c0d` | `23658ec5d0a31108b6a1a3691fbb00b9f4096a77` | 1 | 257 | 1 | `23658ec` | superseded | The same change as `story/s016a` (`06d5397`), with a different SHA. Covered by the S-016a row above. |
| `hermes-subagent/subagent-sa-0-e62d68ae` | `7b63bf8bba851ab3af6ce7457723cd11725dc2c2` | 1 | 314 | 1 | `7b63bf8` | **valuable missing document** | Adds `docs/reviews/goldfish-zoomout-s083-s088.md`, 75 lines, the zoom-out review of S-083 to S-088 against the product laws. That path does **not** exist on `main` and `main` has no `docs/reviews/` directory. The review is cited from `backlog/S-083` to `S-088` and from `docs/PLAN.md`, so the document that those citations point at is missing from the tree. **Candidate missing file**: `docs/reviews/goldfish-zoomout-s083-s088.md`. Adding it is a documentation-only change with no code risk, and it closes a broken citation. Pending the parent's decision. |

### The 18 unique commits, listed

| Commit | Subject | Branch |
| --- | --- | --- |
| `01566c2` | test: S-080 doctor engine discovery contract from AC1 to AC5 | `story/s080` |
| `06d5397` | [S-016a] Implement reproducible APT repo builder and install commands.json | `story/s016a` |
| `10462b5` | fix(s081): correct CHANGELOG Arabic entry format and test pattern | `story/s081` |
| `1ca352f` | S-076a: review PASS, status done | `story/s076a` |
| `2036816` | test(s081): windows long paths and directory output candidates from AC1 to AC5 | `story/s081` |
| `21e3889` | docs: winget-pkgs new-package merge requirements, sourced checklist | `hermes-subagent/subagent-sa-0-8a845e11` |
| `23658ec` | [S-016a] Implement reproducible APT repo builder and install commands.json | `hermes-subagent/subagent-sa-0-b46d8c0d` |
| `2a4719f` | feat(ui): drop the portal row and make Folder name conditional (S-076a) | `story/s076a` |
| `3701ddc` | docs: log S-076a form-state test blocker | `story/s076a` |
| `5352fd8` | S-017a tests: failing suite for AC1 to AC3 (glass and motion tokens, contrast, lint, docs) | `story/s017a`, `hermes-subagent/subagent-sa-0-0a4d945f` |
| `5983761` | test: make the S-076a unedited-Save assertion clippy-clean | `story/s076a` |
| `5b6a552` | spike: test Alt-only global shortcuts | `spike/alt-portal` |
| `6009a9d` | test: add S-076a settings acceptance tests | `story/s076a` |
| `642ac6e` | test: make the S-076a form fixtures hidden-state agnostic | `story/s076a` |
| `7b63bf8` | docs: zoom-out review of S-083 to S-088 against the product laws | `hermes-subagent/subagent-sa-0-e62d68ae` |
| `9a4f815` | feat(doctor): resolve engine tools through the engines' shared lookup (S-080) | `story/s080` |
| `b8522a5` | S-017a implement: glass hive + motion tokens complete | `story/s017a` |
| `c08907b` | fix: S-080 doctor engine discovery contract from AC1 to AC5 | `story/s080` |

## Worktree states

At the baseline, 14 worktrees: the main checkout plus 13 extra. Every extra worktree
except `s016a` and the detached `spikes/gnome-alt-extension` was clean. On 2026-11-15
the GNOME worktree was parked outside the repository with `git worktree move`, so the
registered count is still 14 while one of them now lives under
`~/kawaro-consolidation-2026-11-15/parked-worktrees/`.

| Worktree | HEAD | Branch or detached | State | Payload | Disposition |
| --- | --- | --- | --- | --- | --- |
| `/home/sbarah/R/Projects/P/sbarhive` | `c0444b0` | `main` | dirty, 56 tracked paths modified and 5 untracked entries plus this document | 0.5.1 candidate metadata, eight PNGs, site tree, three release-note files | Keep as the only worktree. Preserved in the backup. |
| `.claude/worktrees/lfm3-local-models-d95de0` | `5e0ef9a` | `claude/lfm3-local-models-d95de0` | clean | none | **Pending** removal with its branch. |
| `.claude/worktrees/roadmap-main` | `3d90601` | `story/roadmap-main` | clean | none | **Pending** removal with its branch. |
| `.claude/worktrees/s016a` | `06d5397` | `story/s016a` | dirty | modified `tests/release/run_all.sh` (one line, adds `install_commands` to the test list); untracked `tests/apt/{lib,repo_builder,signing,containers,run_all}.sh` (5 files, 758 lines) and `tests/release/install_commands.sh` (196 lines) | **Pending** removal with its branch. The dirty payload is preserved in `worktree-payloads/s016a/`. Classification of the payload: the five `tests/apt/` scripts and `tests/release/install_commands.sh` test the **old** contract, `packaging/linux/build-apt-repo.sh` and the old `install-commands.json` shape. `main` has the later `packaging/apt/build-repo.sh` and the `schema: 1` JSON, covered by `tests/release/s103f_apt.sh` and `tests/release/s112_install_commands.sh`. So this payload is a superseded test generation, not missing coverage. The one interesting part is `tests/apt/containers.sh`, which runs a real container install, an area `backlog/S-112-appstream-and-install-commands.md` still lists as remaining ("live clean container install tests of apt, COPR, AUR"). Adapting it to the current contract is a separate story, not part of the consolidation. |
| `.claude/worktrees/s017a` | `b8522a5` | `story/s017a` | clean | none | **Pending** removal with its branch. |
| `.claude/worktrees/s076a` | `1ca352f` | `story/s076a` | clean | none | **Pending** removal with its branch. |
| `.claude/worktrees/s079` | `ac9d7e1` | `story/s079` | clean | none | **Pending** removal with its branch. |
| `.claude/worktrees/s080` | `9a4f815` | `story/s080` | clean | none | **Pending** removal with its branch. |
| `.claude/worktrees/s081` | `10462b5` | `story/s081` | clean | none | **Pending** removal with its branch. |
| `.claude/worktrees/s083` | `f6ef156` | `story/s083` | clean | none | **Pending** removal with its branch. |
| `.claude/worktrees/s087` | `8ae0b3c` | `story/s087` | clean | none | **Pending** removal with its branch. |
| `.claude/worktrees/winfix` | `60c6b7` | `story/winfix` | clean | none | **Pending** removal with its branch. |
| `spikes/gnome-alt-extension` (now parked at `~/kawaro-consolidation-2026-11-15/parked-worktrees/gnome-alt-extension`) | `50715f0` | detached | dirty, 7 untracked files | `spikes/gnome-alt-extension/README.md`, `ext/kawaro-alt@kawaro.test/extension.js`, `ext/kawaro-alt@kawaro.test/metadata.json`, `heavy.sh`, `input_alt.py`, `probe.js`, `run.sh` (about 41 KB of source) | **Parked on 2026-11-15, not deleted.** The seven source files were restored into `main` at `spikes/gnome-alt-extension/` after they were copied outside the repository and compared byte for byte with `worktree-payloads/spikes-gnome-alt-extension/gnome-alt-extension-source.tar.gz`. This is the GNOME native Alt extension spike for S-075d: a real GNOME Shell extension that captures `Alt_L` press and release and feeds `kawaro drop --hold` and `--release`. Before the restore it was **not** in `main` and **not** in any branch: it existed only as untracked files in this worktree. It is the prerequisite evidence for the S-075b and S-075d native GNOME and KDE paths that `docs/PLAN.md` line 131 lists as blocked. |
| `spikes/kde-alt-native` | `50715f0` | detached | clean | none | **Pending** removal. The KDE native spike was never written: the worktree holds a checkout of the old commit `50715f0` and no `spikes/kde-alt-native/` source directory. The S-075b KDE half has no source at all. |

Both detached worktrees sit on `50715f0` ("S-079 plan Windows beta live readiness"), a
commit that is **not** an ancestor of `main`. Its 61 commits ahead of the merge base
`3859312` are all patch equivalent to `main` (`git cherry main 50715f0` reports zero
`+` lines), so no content is lost with it. The commit itself survives in the bundle
through the `worktrees/kde-alt-native/HEAD` and `worktrees/gnome-alt-extension/HEAD`
refs.

## Classification of the dirty payloads

| Payload | Kind | Value |
| --- | --- | --- |
| `main` tracked diff (56 paths) | product and release metadata, 0.5.1 candidate | keep, it is the S-141 work in progress |
| `main` 8 PNG files | regenerated snapshots and public screenshots for the 0.5.1 header label | keep, they belong to the same S-141 set |
| `main` 3 untracked release-note files | the bilingual 0.5.1 notes and the whats-new JSON | keep |
| `s016a` modified `tests/release/run_all.sh` | one line wiring an old test into the runner | superseded by the current runner list, which already includes `s112_install_commands` and `s103f_apt` |
| `s016a` untracked `tests/apt/` (5 files) | test generation for the old APT contract | superseded by `tests/release/s103f_apt.sh`; the container half is a possible future story |
| `s016a` untracked `tests/release/install_commands.sh` | test for the old JSON schema | superseded by `tests/release/s112_install_commands.sh` |
| `gnome-alt-extension` 7 source files | GNOME native Alt extension spike | **valuable**, restored into `main` at `spikes/gnome-alt-extension/` on 2026-11-15 |
| `kde-alt-native` | nothing | the KDE native spike source was never written |

## Executed restoration, 2026-11-15

Only restoration was executed. No commit, no push, no tag, no branch deletion, no
worktree deletion and no remote write happened. The parked GNOME worktree is the only
change to the worktree list.

| Action | Result | Evidence |
| --- | --- | --- |
| Backup verified | The bundle records a complete history | `git bundle verify bundles/kawaro-all-refs.bundle` from inside the repository |
| `spikes/alt-portal/` restored | 9 source files plus a new `README.md` | The 9 files are byte identical to commit `5b6a552` on `spike/alt-portal`, checked file by file with `sha256sum` |
| `spikes/gnome-alt-extension/` restored | 7 source files | Copied to `staging/gnome-alt-extension/` outside the repository first, then compared file by file with the verified archive: the two SHA-256 lists are identical |
| GNOME worktree parked, not deleted | Moved to `~/kawaro-consolidation-2026-11-15/parked-worktrees/gnome-alt-extension` with `git worktree move` | The parked worktree still resolves and still holds its seven untracked files; `git worktree list` shows it at the new path |
| `docs/reviews/goldfish-zoomout-s083-s088.md` restored | The review body is byte identical to commit `7b63bf8` | A provenance note was added above the original text. No normalization was needed: the original contains no em dash, no en dash and no guillemet. |
| README provenance added | `spikes/alt-portal/README.md` (new) and a provenance and limitations section in `spikes/gnome-alt-extension/README.md` | Both state the source branch and commit, and both state that no measurement was performed by the restore |
| Obsolete APT and duplicate tests | Not restored | `tests/apt/` and `tests/release/install_commands.sh` stay in `worktree-payloads/s016a/` only, because they test the superseded contract |

The KDE native spike has no source and none was written. `spikes/kde-alt-native/`
is still a registered worktree holding a checkout of `50715f0` and nothing else.

## Pending consolidation actions

Items 1 to 3 below are done for the restoration part only. Everything else is
still pending, and the cleanup is not complete.

1. Done on 2026-11-15: `spikes/alt-portal/` and `spikes/gnome-alt-extension/` were
   added as tracked spike source and `docs/reviews/goldfish-zoomout-s083-s088.md`
   was added as a historical document. All three are additive and touch no product
   code. They are not committed yet.
2. Commit the 0.5.1 candidate set on `main` as its own commit (the 56 tracked paths
   and the three release-note files), keeping `CHANGELOG.md` to the S-141 line.
3. Run the local gate set on that exact commit (see the gate commands below).
4. Push `main` fast-forward only. Never force, amend, rebase or rewrite.
5. After every push, read `scripts/ci-status.sh --wait` for that exact SHA. A red run
   blocks the next step.
6. Delete the 32 non-`main` local branches, then the 12 worktrees still inside the
   repository, then the parked GNOME worktree, then the one remote branch other than
   `main`, `origin/fix/ci-portability-and-fd-identity`. The `origin/HEAD`
   symbolic ref must stay.
7. Keep the two stashes until the 0.5.1 tag exists, then decide. They hold only a
   `design/brand/kawaro.ico` change whose content is already identical to the `main`
   blob (`d845a602...` on both sides), so they are safe to drop later.
8. Keep `refs/original/*` and the `refs/codex/...` checkpoint until the tag exists.

### Owner exception for this consolidation, 2026-11-15

The owner issued a one-time exception for the source consolidation push only:

- The full local gate set is **deferred**, and so is the remote CI. The source push
  rests on a static review only.
- No release tag, no release pipeline, no image capture and no publication.
- The remote CI must not start, so both consolidation commits carry the
  `[skip ci]` marker.
- The earlier rule that the full local set must be green before any push to `main`
  does not apply to this consolidation push. It still applies to every later push,
  and in particular to the 0.5.1 publication.
- The deferral is not a pass. Nothing in this repository may claim a green gate for
  the consolidation commits.

The gate commands below stay in this document as the reference for the next gated
push. They were not run for the consolidation.

## Recommended next work, with evidence

The state below was read from `docs/PLAN.md`, the backlog status lines, the GitHub
issues and the release list. It is a proposal for the parent to choose from. None of it
is started.

### 1. Finish the 0.5.1 candidate on the current `main` (highest value, already 90 percent done)

Evidence:

- `main` is `c0444b0` and `origin/main` is the same SHA, so there is no divergence to
  reconcile. The last public CI run on it is `42058502635`, conclusion `success`, on
  2026-11-05, and the story file says "repaired main publicly green".
- The uncommitted tree already carries the version bump (`Cargo.toml` 0.5.1, six
  `Cargo.lock` entries), the metainfo release, the bilingual release notes, the
  whats-new JSON, the eight PNGs and the regenerated site.
- `backlog/S-141-publish-0.5.1.md` AC1 to AC6 still need: the commit of that tree, the
  full local gate set, the reviewer PASS on the frozen diff, the tag, the Packages run
  and the channel verification.

Acceptance criteria to propose: the 0.5.1 tree is committed on `main` as one commit
with only the S-141 paths; `scripts/ci-status.sh --wait` on the exact pushed SHA is
green; the tag `v0.5.1` exists on that SHA; the Packages run attaches RPM, DEB, ZIP,
MSI, `SHA256SUMS`, `SHA256SUMS.asc` and `kawaro-archive.asc`; the APT signed metadata
and the Scoop manifest list 0.5.1.

Blockers to record honestly: AUR and COPR stay skipped until the owner accounts
exist, the WinGet PR `microsoft/winget-pkgs#462805` still waits for a moderator, and
the Windows build stays self-signed.

### 2. S-138, the whole-output and final-position coverage (draft, needs a baseline first)

Evidence: `backlog/S-138-complete-drop-first-coverage.md` says "draft, awaiting baseline
evidence, user clarification and Lead approval; no implementation authorized", and
GitHub issue #9 is open with the same scope. The acceptance checklist in issue #9 asks
for a real-Dolphin near and far DropFirst baseline with a nonzero-origin output, the
release-before-drop semantics confirmed with the maintainer, and story approval.

Acceptance criteria to propose: a recorded baseline of the near and far DropFirst
result and the final logical position; the release-before-drop semantics confirmed in
writing; then the story, then the implementation, keeping the S-136 exactly-once
session ownership and the untouched original file.

### 3. S-082, real GNOME and KDE focus and activation (top open risk)

Evidence: GitHub issue #1 is open. `docs/PLAN.md` line 21 lists it as blocked on real
GNOME and KDE Plasma virtual machines with a file manager session, and states that
"private compositor evidence does not close it". The private `tests/focus/` harness
already covers headless GNOME strict and KWin Extreme.

Acceptance criteria to propose: on a real GNOME session with Nautilus and a real Plasma
session with Dolphin, the activation token from the file manager reaches the hive, the
hive takes focus, Escape and focus loss close it, and the run is recorded per
desktop. This is a person-and-VM task, so it needs the owner or a prepared VM image.

### 4. S-075b and S-077b, the native KDE and GNOME Alt and pointer paths

Evidence: `docs/PLAN.md` line 131 records the split, line 22 lists the native paths as
blocked on private KWin and GNOME sessions, and line 148 records the measured facts
that Alt-only is disproven and that Super+Z passes on both. The GNOME native extension
spike source exists only as untracked files in the detached worktree, and the KDE
native spike was never written.

Acceptance criteria to propose: the GNOME extension spike and the KDE native spike are
committed as tracked spike source under `spikes/`, then measured on a private GNOME
and a private KWin session, then the measured path becomes the default per desktop and
the Super+Z fallback stays named and recorded.

### 5. The live container install test for apt, COPR and AUR

Evidence: `backlog/S-112-appstream-and-install-commands.md` still lists "live clean
container install tests of apt, COPR, AUR" as remaining, and the superseded
`s016a` payload in `worktree-payloads/s016a/` holds a working container harness
(`tests/apt/containers.sh`, 335 lines) that runs the real install commands in a
container.

Acceptance criteria to propose: adapt the container harness to the current
`packaging/install-commands.json` schema and to `packaging/apt/build-repo.sh`, and
require it to install from a locally built and locally signed repository, with the
original file untouched.

### 6. The remaining distribution channels

Evidence: GitHub issue #3 is open for the COPR account, `docs/distribution-channels/aur.md`
and `copr.md` need owner credentials, and the WinGet PR is still waiting since
2026-10-27.

Acceptance criteria to propose: an owner account creates the COPR project and the AUR
repository, the channel jobs are switched on, and each channel is verified with a real
install rather than a green exit code. This is owner-blocked by credentials.

### 7. The unreleased local work block, S-117 to S-140

Evidence: GitHub issue #10 lists the locally implemented block and keeps
"Explicit approval and actual source publication/release for local unreleased work" as
unchecked. The source of S-139 and S-140 is already public on `main` (`441d8d6`,
`83fc676`), while the metadata for the release is not.

Acceptance criteria to propose: the 0.5.1 tag publishes the accepted set and the issue
checklist is updated to the published state.

Status of the items this plan earlier called local-only, checked against the tree on
2026-11-15:

| Story | Backlog status | Source on `main` | Recorded in `CHANGELOG.md` | Truthful state |
| --- | --- | --- | --- | --- |
| S-132 | `in progress` | yes: `crates/engines/src/image.rs` and `crates/engines/tests/s132_compress_to_target.rs`, plus the `TaskDetail` plumbing in core, daemon, CLI and MCP | yes | The implementation and its tests are on `main` and public at `c0444b0`. The story status line still reads `in progress`, and no published release carries it yet |
| S-133 | `in progress` | yes: `crates/engines/tests/s133_pdf_compress_skip.rs` and the `TaskOutcome` and `SkipReason` contract | yes | Same state as S-132: implemented and public, not yet released, story status not closed |
| S-134 | `in progress` | yes: `crates/mcp/tests/s134_audit_writer.rs`, `s134_config_audit_path.rs`, `s134_docs_audit_path.rs`, `s134_resolved_audit_path.rs` | yes | Same state as S-132: implemented and public, not yet released, story status not closed |
| S-138 | `draft`, no implementation authorized | no | no | Nothing exists. It needs a baseline first, see item 2 |

So no absence is to be reported for S-132 to S-134: their source is present and
public, only their release and their story status lines are open.

### Lower priority, recorded but not recommended now

- Issue #2, idle memory with very large font collections: needs a fresh measurement
  first, and no defect has been confirmed.
- Issue #8, late Escape and Alt-release cancellation: fixed and installed locally, and
  it stays open only for public source and release availability, which item 1 covers.
- The physical Windows hardware check, the real X11 desktop check, IME and physical
  hotplug, and the fresh show and preview timing measurements: all listed in
  `docs/PLAN.md` as not checked, all needing a physical session.

## Local gate commands

**Deferred for the consolidation push.** The owner deferred this set and the remote CI
for the 2026-11-15 consolidation only. The commands below were **not** run for the
consolidation commits, and no result is claimed for them. They remain the reference
for the next gated push, which is the 0.5.1 publication.

`scripts/release-gates.sh` is explicitly not an authorized gate under the owner
decision of 2026-11-05. The gate set is the one in
`~/kawaro-publish/verify-version-0.5.1.sh`, which calls
`~/kawaro-publish/full-prepush-0.5.1.sh`. It must run inside a resource-limited
scope, serialized, one cargo job at a time.

Correction to an earlier draft of this section: the draft exported
`CARGO_TARGET_DIR=/home/sbarah/.cache/kawaro-win-target` for the whole script, which
would have routed every Linux `cargo fmt`, `cargo clippy` and `cargo test` into the
Windows cache directory and away from the repository `target/`. The real scripts do
not do that. `verify-version-0.5.1.sh` exports the repository target globally and
`full-prepush-0.5.1.sh` overrides the target only for the four Windows commands with
`env CARGO_TARGET_DIR=/home/sbarah/.cache/kawaro-win-target`.

Exact command for the parent to run:

```bash
systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=100% \
  nice -n 15 /home/sbarah/kawaro-publish/verify-version-0.5.1.sh
```

What that script does, in order:

1. `crates/ui/tests/settings_snapshots.sh` with `AT_SPI_BUS_ADDRESS` and
   `SESSION_MANAGER` unset and `KWIN_WAYLAND_NO_PERMISSION_CHECKS=1`. This is the
   snapshot step and it only ever runs in the private compositor.
2. `python3 -I ~/kawaro-publish/version-test-drafts/metadata_contract.py`, the release
   metadata contract.
3. `~/kawaro-publish/full-prepush-0.5.1.sh`, which runs in this order:
   - the source manifest capture, `cargo fmt --all --check`,
     `cargo clippy --offline --locked --workspace --all-targets -- -D warnings`,
     `cargo test --offline --locked --workspace -- --test-threads=2`;
   - two repeats of `cargo test --offline --locked -p kawaro-ui --lib -- --test-threads=2`;
   - the four Windows commands: two `cargo clippy` stages, the
     `cargo check` of `s141_session_portable`, and the release build of
     `kawaro-cli` and `kawaro-daemon`;
   - `scripts/windows/check-dlls.sh` for `kawaro.exe` and `kawarod.exe`, and
     `tests/windows/test_s085.sh` on the release folder;
   - `scripts/design-lint.sh crates/ui`, `tests/release/run_all.sh`,
     `tests/docs/test_launch_assets.sh`,
     `RELEASE_PUBLISHED=0 bash scripts/site/check-deploy.sh --build`, and
     `git diff --check`;
   - the source manifest capture again and a `cmp` of the two manifests.

Notes for the parent:

- The script must live in a file, because `systemd-run` substitutes `$` in its
  arguments.
- `TMPDIR` is under `$HOME`, never under the small `/tmp` tmpfs.
- The Linux target is the existing repository `target/`, and the Windows target is the
  existing `~/.cache/kawaro-win-target`. Neither directory is copied or rebuilt from
  scratch.
- The two `cargo test` repeats for `kawaro-ui` are the flaky-snapshot repeats the
  existing prepush script already uses. They are a real test, not a suppressed one.
- No test is disabled, skipped or weakened. No `-j4` and no parallel job.
- The manifest comparison is why the site generation has to be deterministic: the site
  is rebuilt by `check-deploy.sh --build`, so the generated files must be frozen
  before the gate runs.
- After the local set is green, the reviewer runs the same scoped set on the frozen
  diff, and the public CI on the exact pushed SHA is the last gate before tagging.

## Verification notes

- `git bundle verify` passed and a real clone from the bundle restored an identical
  `main` tree with all 18 unique commits and both stashes.
- Branch and worktree inventory was read with `git for-each-ref`, `git worktree list
  --porcelain`, `git rev-list`, `git cherry` and `git merge-base --is-ancestor`.
- Each unique commit was read with `git show --stat` and compared with the current
  `main` files it touches.
- The branch counts were recomputed on 2026-11-15: 33 local branches, 22 of them
  ancestors of `main`, 10 of them not, and 18 distinct unique commits over the ten
  non-ancestor branches.
- The restoration evidence is listed in "Executed restoration".
