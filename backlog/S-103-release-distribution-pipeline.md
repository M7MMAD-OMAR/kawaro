# S-103 Automated release distribution pipeline (umbrella)

Status: review (S-103a, S-103b done 2026-10-30; the other parts are with their own implementers)

## Story

As the maintainer, I want every stable release to be mirrored to all package channels by one idempotent, isolated, documented pipeline, so that a release needs no manual channel work and a user can install Kawaro with one command from the channel they already use.

## Relation to S-016 and S-078 (decisions that must not be silently reversed)

- S-078 decision 1 says "do not create `kawaro-bin`". S-103e adds it beside the source package `kawaro` (with `provides` and `conflicts`). Manager sign-off given 2026-10-30: the owner wants the `kawaro-bin` channel.
- S-016 chose a Cloudflare R2 APT repository and a COPR SRPM build. S-103 keeps both and does not move to GitHub Pages. S-103d and S-103f only add orchestration, status rows and docs on top of S-016a to S-016d, and are blocked on them.
- S-103b is the same work as S-078e and replaces it; it goes live only after the open winget-pkgs submission for 0.4.0 is merged.
- Environment and secret names are reused from those stories: `winget-publishing`, `aur-publishing`, `apt-publishing`, `copr-publishing`, `release-signing`; `WINGET_CREATE_GITHUB_TOKEN`, `AUR_SSH_PRIVATE_KEY`, `COPR_CONFIG`, `APT_SIGNING_KEY`. New names: `SCOOP_BUCKET_TOKEN` (S-103c).
- All install commands come from `packaging/install-commands.json` (S-016 decision 8). S-103 adds the entries `scoop` and `aur-bin`; docs and tests render from the JSON.
- `kawaro-bin` needs a Linux binary. Release assets today are RPM, DEB, ZIP, MSI, and `packages.yml` asserts exactly 4 lines in `SHA256SUMS`. S-103e must extract the DEB (no new asset), so that check and S-016 AC10 stay valid.

## Order and dependencies

S-103a first. S-103g starts at once (needs no channel). S-103c and S-103e after a. S-103d and S-103f after a and S-016. S-103b after a, live after the winget merge. S-103h last. Each part is at most one working day; if one grows, split it.

## Acceptance criteria

Common to b to f: a channel job runs only for a stable `vX.Y.Z` tag in the repository named by the repository variable `CANONICAL_REPO` (never a hard-coded name), reads secrets only as whole `env:` values, prints no secret, and every `uses:` in the new workflow is pinned to a 40 hex character commit SHA.

- **S-103a orchestration.**
  - AC-a1: a reusable workflow `release-distribute.yml` (`workflow_call`) is called by a final job of `packages.yml` after `release` and `update-manifest`, and also has `workflow_dispatch` with inputs `tag` and `dry_run`. (A workflow triggered by `release: published` does not fire when the release is created with the job token, so that trigger is not used.)
  - AC-a2: one job per channel (`winget scoop copr aur apt`), each with `continue-on-error` semantics so that a forced failure of one leaves the other four with result `success` or `skipped`.
  - AC-a3: a channel without its secret prints exactly `SKIP <channel>: <SECRET_NAME> not configured` and exits 0. A prerelease tag prints `SKIP <channel>: prerelease tag` for all five.
  - AC-a4: `dry_run=true` prints `DRYRUN <channel>: would publish <version>` and performs no network write; a second run with the same tag reports `UNCHANGED` per channel.
  - AC-a5: `scripts/release/channel-status.sh` generates a table (channel, state, version, checked date) from `packaging/channels.json` between `<!-- channels:start -->` and `<!-- channels:end -->` in `docs/distribution-status.md`; the workflow never pushes to `main`.
- **S-103b WinGet.**
  - AC-b1: `scripts/release/winget-manifests.sh` generates the four manifests (version, installer, en-US, ar) from the release asset list and `SHA256SUMS`; output equals the golden directory `tests/release/golden/winget/` byte for byte, and `winget validate` or the JSON schema check passes.
  - AC-b2: PR automation (wingetcreate or Komac, whichever has a licence compatible with GPLv3 use as a CI tool, recorded in `docs/engines.md`) is tested against a mock: first run creates 1 branch and 1 PR on the fork, rerun with the same tag updates it and creates 0 new PRs. The token is passed only by environment variable; the log contains no token.
  - AC-b3: `check-winget-versions.sh` and `check-winget-scope.sh` (S-078) run on the generated set and exit 0.
- **S-103c Scoop.**
  - AC-c1: `scripts/release/scoop-manifest.sh` generates `kawaro.json` (version, url, hash, `bin`, `checkver`, `autoupdate`) from the MSI-free ZIP asset; the hash equals the ZIP line in `SHA256SUMS`; output equals the golden file.
  - AC-c2: the file passes the Scoop manifest JSON schema check, and a test with a one-character wrong hash fails.
  - AC-c3: the job commits to the bucket repository `OWNER/scoop-kawaro` by a mocked push in a temporary clone; a rerun creates 0 new commits.
- **S-103d COPR (on S-016c, S-016d).**
  - AC-d1: the job triggers a COPR build from the tag SRPM with `copr-cli`, with a stub `copr-cli` in tests; it waits and fails unless the 3 chroots of S-016 decision 4 report `succeeded`.
  - AC-d2: `spec` lint (`rpmlint` with 0 errors) passes on the SRPM spec; `COPR_CONFIG` is written to a temporary file removed by an `EXIT` trap (test: file absent after the step, also after a forced failure).
  - AC-d3: `dnf copr enable OWNER/kawaro` appears in the docs from the JSON entry `copr`.
- **S-103e AUR `kawaro-bin`.**
  - AC-e1: `scripts/release/aur-bin.sh` generates `PKGBUILD` and `.SRCINFO` from the release DEB; the `sha256sums` entry equals the DEB line in `SHA256SUMS`; output equals the golden files for 2 fixture versions.
  - AC-e2: `makepkg --printsrcinfo` reproduces `.SRCINFO` byte for byte; `namcap` reports 0 errors; `shellcheck` passes on the script.
  - AC-e3: push to a local bare repository standing in for the AUR is fast forward only (a forced non fast forward fails the test); `StrictHostKeyChecking` is on and the committed `known_hosts` is used.
- **S-103f APT (on S-016a, S-016b, S-016e).**
  - AC-f1: the job runs the S-016 builder and R2 upload in dry run and lists the 3 upload phases in order (pool, metadata, Release files last).
  - AC-f2: the signing key lives only in `release-signing`; the `apt-publishing` job has no private key (test greps the job env). Docs show the `apt` setup lines from the JSON entry with `Signed-By`, and no `trusted=yes`.
  - AC-f3: Kali and Debian official inclusion is documented as a manual slow path with the steps; no job exists for it.
- **S-103g Flathub assessment (parallel, no dependency).**
  - AC-g1: `packaging/flatpak/` holds a manifest that passes `flatpak-builder-lint manifest` with 0 errors.
  - AC-g2: `docs/flathub-assessment.md` rates each limit as works, degraded or blocked: the systemd user daemon, file manager integration files on the host, writing next to the original (filesystem access), host engines (FFmpeg, LibreOffice, Ghostscript), layer-shell, xdg-activation, the tray icon, the GlobalShortcuts portal. It ends with one line `Decision: GO` or `Decision: NO-GO` and a reason. Flathub review policy points are recorded as "to confirm", not asserted.
- **S-103h Public docs.**
  - AC-h1: `docs/distribution.md` and `docs/distribution.ar.md` start with the line `[English](distribution.md) | [العربية](distribution.ar.md)` and cover the pipeline diagram (text), each channel with its one command, how to add a channel, the secrets per channel (names only), the OWNER ACTIONS section and the review checklist. The Arabic copy has the same 8 top level headings as the English file, Western digits, no guillemets.
  - AC-h2: every command in both files equals its entry in `packaging/install-commands.json` (rendered, not copied).
  - AC-h3: a lint finds 0 matches in the 3 new files and in this story for: long dashes, an email address, `/home/`, a token or key block, and any real account name (list kept in the test as the project owner's deny list file outside the repo).

## Test cases

| Criterion | Test | Type |
| --- | --- | --- |
| AC-a1 | Parse both workflows with a YAML check: `packages.yml` has a job calling `release-distribute.yml` with `needs` on `release` and `update-manifest`; the called file has both triggers | unit |
| AC-a2, a3 | Run the channel scripts with a stubbed failing `copr` job, then without each secret, then with tag `v1.0.0-beta.1`; compare stdout with the exact lines | integration |
| AC-a4 | Run twice with `dry_run`; assert zero network calls through a recording stub, and `UNCHANGED` on the second run | integration |
| AC-a5 | Regenerate the table; `diff` with the committed block must be empty (drift check) | unit |
| AC-b1, c1, e1 | Generate from fixture assets and fixture `SHA256SUMS`, `diff -r` with golden files, plus a fixture whose hash differs and must fail | unit |
| AC-b2, c3, e3 | Mock GitHub API, mock bucket repo and a bare repo; count branches, PRs, commits across a rerun; grep logs for the fixture token string | integration, security |
| AC-b3 | Run the two existing S-078 checks on the output | integration |
| AC-c2, d2, e2, g1 | Run schema check, `rpmlint`, `namcap`, `shellcheck`, `flatpak-builder-lint` on the generated files, and on a deliberately broken copy that must fail with the expected message | integration |
| AC-d1 | Stub `copr-cli` returns `failed` for one chroot; job exit code must be nonzero | integration |
| AC-d3, f2, h2 | Render docs from the JSON, compare; change one JSON command and the docs check must fail | unit |
| AC-f1, f2 | Dry run transcript compared with the expected order; grep job env for key names | integration, security |
| AC-g2 | Parse the assessment: 8 limits each with a rating, and one `Decision:` line | unit |
| AC-h1, h3 | Heading count compare of EN and AR, switch line check, lint patterns above | unit |
| Common | grep the new workflow: every `uses:` matches `@[0-9a-f]{40}`; no `${{ secrets.` outside a whole `env:` value; channel jobs carry the tag and `CANONICAL_REPO` guard | unit |

## OWNER ACTIONS (human only; automation never creates accounts or stores credentials)

Create the GitHub environments listed above with tag protection; create a classic token for the winget fork and the winget fork; create the Scoop bucket repository `OWNER/scoop-kawaro` and its token; register the AUR account, upload an SSH public key, and register `kawaro-bin` once by hand (first push); create the COPR project and its API config; create the R2 bucket and token (S-016); add all secrets and `CANONICAL_REPO`; submit to Flathub only after a GO. Rotate expiring tokens. Each secret is a name in the docs, never a value.

## Risks

- Supply chain: pin actions by full commit SHA, update through reviewed PRs; least privilege tokens per channel; protected environments for stable tags only; no secrets in logs, `set +x`, and a log scan test.
- A channel registry rejecting a submission (winget review takes weeks): the status table shows `submitted`, never `published`.
- Third party tool licences and Flathub, Scoop and COPR rules are not verified here: confirm and record in `docs/engines.md` before use.

## Review plan

One reviewer agent per sub-part, in a fresh context, fast gates only: `cargo fmt --check`, `cargo clippy`, `cargo test --all`, `scripts/design-lint.sh`, ShellCheck, the new unit tests, and the dash and privacy lint. Mutation testing is not a gate. After two REWORKs the sub-part goes to the maintainer. CI is read with `scripts/ci-status.sh --wait` after each push.

## Out of scope

Chocolatey and Homebrew (Linuxbrew tap): later optional channels, each added by the same recipe as S-103c if wanted (Chocolatey needs a moderated community account; a tap is a plain repository like the Scoop bucket). Snap, Microsoft Store (see `docs/distribution-status.md`), macOS, and re-pinning the older workflows by SHA (follow-up story). Debian and Kali official inclusion beyond the written guide.

## Review log

- 2026-10-30: manager rulings: S-078 decision 1 is overridden (kawaro-bin is wanted, S-103e); R2 stays the APT host (S-016). Channel contract fixed: `scripts/channels/NAME.sh TAG [--dry-run]`, secrets by environment only, exit 0 success, 10 clean SKIP with one line `SKIP NAME: VARIABLE not configured`, other codes failure; `scripts/channels/run-all.sh` runs every script found by glob in isolation and writes the status table between the markers in `docs/distribution-status.md`.
- 2026-10-30: S-103a and S-103b implemented, status review. Deviations from the AC text, for the reviewer: (1) the table comes from `run-all.sh`, not `scripts/release/channel-status.sh` and `packaging/channels.json`; (2) channels are scripts with the exit 10 SKIP contract rather than inline workflow steps; the prerelease line is `SKIP NAME: prerelease tag`; (3) the `UNCHANGED` rerun report (AC-a4) is not built, dry runs print what they would do; (4) the winget preview manifests are a local sanity set (version, installer, en-US, ar) and Komac builds the real set from the previous version, so AC-b1 golden and AC-b2 PR rerun semantics rely on Komac; (5) the workflow uses one matrix job per channel with `environment: NAME-publishing` and lists all channel secrets as whole env values, so secrets MUST be environment secrets, never repository secrets; (6) a failing channel shows as a red matrix job but cancels no other (fail-fast off). The actions pinned by SHA in `release-distribute.yml` are checkout v7, upload-artifact v7, download-artifact v8 (SHAs resolved with the GitHub API). `winget.sh` is a SKIP until the secret exists and must not be enabled before winget-pkgs#462805 is merged. Komac 2.17.1 (GPL-3.0) recorded in `docs/engines.md`.

- 2026-10-30: drafted by the planner, not yet reviewed.
- 2026-10-30: S-103c and S-103e implemented (status review). Files: `scripts/channels/scoop.sh`, `scripts/channels/aur.sh`, templates and generators in `packaging/scoop/` and `packaging/aur/`, tests `tests/release/s103c_scoop.sh` (25 checks) and `s103e_aur.sh` (35 checks) with goldens in `tests/release/golden/`, docs in `docs/distribution-channels/`. Both tests failed first (generators and scripts absent), then passed offline. Decisions: Scoop installs the ZIP, not the MSI (per user, no admin, clean uninstall; the cost is no Explorer menu, documented). `kawaro-bin` extracts the DEB with bsdtar, no new asset, the `-eq 4` check is untouched. A `--dry-run` never skips: it prints `DRYRUN <channel>: would publish X` plus a NOTE line when config is missing. Sums come from `SHA256SUMS_FILE`, else `dist/SHA256SUMS`, else the release download. The AUR secret is `AUR_SSH_KEY` (contract) not `AUR_SSH_PRIVATE_KEY` (S-078). Pending live checks, not claimed: `makepkg --printsrcinfo` byte equality and `namcap` (no Arch tooling here; `.SRCINFO` is tested against the sourced PKGBUILD instead), `packaging/aur/known_hosts` (owner must verify and commit the AUR host keys; without it the channel skips), and first registration of `kawaro-bin` (owner action). The `install-commands.json` entries `scoop` and `aur-bin` are not added because that file does not exist yet; the commands are in the two docs fragments.
- 2026-10-30, S-103d (implementer): `scripts/channels/copr.sh` and `packaging/copr/kawaro.spec` done; offline tests `tests/release/s103d_copr.sh` (43 checks, failed first, now green). Deviations from the AC text, for the reviewer: (1) the spec is `packaging/copr/kawaro.spec`, not `packaging/linux/kawaro-copr.spec`, by the task's ownership rule; (2) the vendor dir is `vendor-crates/` because `vendor/` holds the in tree softbuffer patch, and the spec appends to the tree's `.cargo/config.toml` instead of replacing it (it carries an `[env]` table); (3) "3 chroots report succeeded" is enforced by `watch-build` plus `copr-cli status` for the whole build, per chroot output is to confirm on the first live run; (4) `rpmlint` was not installed here, so AC-d2 lint is covered by `rpmspec -P` locally and by rpmlint in CI; (5) no real COPR or mock build ran, and the `BuildRequires` list is unproven; (6) no UNCHANGED reporting on rerun; (7) `docs/distribution-channels/copr.md` states the command by hand until `packaging/install-commands.json` exists.
- 2026-10-30, S-103g (implementer): assessment `docs/distribution-channels/flathub.md` (not `docs/flathub-assessment.md`, by the task's ownership rule) rates 8 limits and ends `Decision: NO-GO`. Manifest draft `packaging/flatpak/io.github.m7mmad_omar.Kawaro.yml` passes YAML checks and `flatpak-builder --show-manifest`; `flatpak-builder-lint` is not installed here, so AC-g1 is open until CI runs it (command in the doc). `cargo-sources.json` is an empty placeholder. Two code changes the GO path needs are in `crates/` and not made here. Tests: `tests/release/s103g_flathub.sh` (28 checks).

- 2026-10-30, reviewer (parts a, b, c, d, e, g): gates run: `bash -n` and `shellcheck` clean on every `scripts/channels/*.sh`, `packaging/*/generate.sh`, `packaging/scoop/validate.sh` (test scripts only info and style notes: SC1091, SC2086, SC2010); `scripts/design-lint.sh` clean; no Rust changed, so no fmt or clippy; tests s103a_channels (20), s103a_workflow (12), s103b_winget (19), s103c_scoop (25), s103d_copr (43), s103e_aur (35), s103g_flathub (28), s073_asset_names, s078_winget_scope, workflow, notes_manifest all green, s078_winget_versions SKIP (no wixl). Ran with TMPDIR under the home directory, deleted afterwards. Verified: the three pinned SHAs exist (`gh api .../commits/<sha>` returns each one), no `pull_request_target` anywhere, tag regex runs before use and the tag only reaches the shell through `env:`, `permissions: {}` at top and `contents: read` per job, `-eq 4` in `packages.yml` untouched, `distribute` has `needs: [release, update-manifest]`, matrix fail-fast false, environment `NAME-publishing`. Privacy: no email, `/home/`, token or key block, no personal name in the added lines; the only account string is the already public repository URL `github.com/M7MMAD-OMAR/kawaro` (spec `URL:`, flatpak source, CHANGELOG links) and the public app id, acceptable. Secrets: never in a URL or argument (Scoop token travels as `GIT_CONFIG_VALUE_0` header, AUR key in a mode 077 file removed by trap, COPR config in a temp file removed by trap, Komac gets `GITHUB_TOKEN` for one child only).
  VERDICT S-103a: REWORK. The workflow and the scripts disagree, so the tests (which call the scripts directly) are green while the real workflow cannot publish: (1) `release-distribute.yml` passes `AUR_SSH_PRIVATE_KEY` but `aur.sh` reads `AUR_SSH_KEY`, so aur always prints SKIP; (2) `scoop.sh` requires `BUCKET_REPO` and `copr.sh` requires `COPR_PROJECT`, neither is in the step `env:`, so both always SKIP; (3) the job installs nothing: no `komac`, `copr-cli`, `rpmbuild`/`rpmspec`, `rpmlint`, so winget exits 1 ("komac is not installed") and copr fails the moment a secret is set; (4) the test s103a_workflow should assert that every env var a channel script reads is present in the workflow `env:` (a drift check), which would have caught 1 and 2. Also dry run for copr prints SKIP when unconfigured while scoop and aur print DRYRUN plus a NOTE (AC-a4 asks for DRYRUN); make the three consistent. Minor: `checkout ref: inputs.tag` runs before the tag regex (harmless, no secret in that step), but a dispatch on a non tag ref should be rejected by a first step; document that the environments must restrict deployment to tags.
  VERDICT S-103b: script PASS (19 checks, token only via env, Komac licence recorded, merge gate documented); the channel cannot work live until the a fixes (3) land. Not verified: Komac against a real fork. Marked REWORK only through part a.
  VERDICT S-103c: script PASS (golden, wrong hash test, rerun makes 0 commits, token not in args); live path blocked by a(2). Not verified: Scoop schema against the official one beyond the local validate.sh.
  VERDICT S-103d: REWORK for a(2) and a(3) (cargo vendor, rpmbuild, copr-cli on the runner) and the dry run inconsistency above. Honest docs: no real COPR or mock build, rpmlint not run, per chroot state unconfirmed, BuildRequires unproven, all stated. The `status` check `[ "$state" != succeeded ]` depends on the copr-cli output format, unverified.
  VERDICT S-103e: REWORK for a(1) (secret name mismatch; either rename in the workflow or in the script and docs, and pick one with the S-078 name noted). Not verified and stated: `makepkg --printsrcinfo`, `namcap`; `packaging/aur/known_hosts` is absent so the channel skips by design until the owner commits verified host keys.
  VERDICT S-103g: PASS. Eight limits rated, `Decision: NO-GO` with reason, policy points marked to confirm, `flatpak-builder-lint` honestly not run (AC-g1 stays open until CI), `cargo-sources.json` is a placeholder and said so. Spec and manifest hardcode the public repo URL instead of an OWNER placeholder; fine for a public project, note only.
  Status: parts a to e go to rework (one small fix set: env names and variables in the workflow, tool install step, drift test, DRYRUN consistency); g done. Not set to done and no CHANGELOG line added for the reworked parts. First REWORK for a to e.
