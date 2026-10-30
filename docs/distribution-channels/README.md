# Distribution channels

Each release channel is `scripts/channels/NAME.sh` (contract in `scripts/channels/run-all.sh`) and runs in `.github/workflows/release-distribute.yml` as one matrix job in the environment `NAME-publishing`.

## Environments (owner action)

Create `winget-publishing`, `scoop-publishing`, `copr-publishing`, `aur-publishing` and `apt-publishing`. For each one, set the deployment branches and tags rule to the tag pattern `v*` only, so no branch can reach its secrets. Put the secrets in the environment, never at repository level: the workflow lists every channel's secret names in one step, and only environment scoping keeps a channel from seeing the others.

## Names the workflow passes (taken from the scripts)

| Channel | Secrets | Variables |
| --- | --- | --- |
| winget | `WINGET_CREATE_GITHUB_TOKEN` | `CANONICAL_REPO` |
| scoop | `SCOOP_BUCKET_TOKEN` | `BUCKET_REPO`, `CANONICAL_REPO` |
| copr | `COPR_CONFIG` | `COPR_PROJECT`, `CANONICAL_REPO` |
| aur | `AUR_SSH_KEY` | `CANONICAL_REPO` |
| apt | `APT_SIGNING_KEY`, `APT_R2_ACCESS_KEY_ID`, `APT_R2_SECRET_ACCESS_KEY`, `APT_R2_ACCOUNT_ID` | `APT_R2_BUCKET`, `CANONICAL_REPO` |

## Tools installed per channel

- winget: Komac 2.17.1, release binary verified against a SHA-256 pinned in the workflow.
- copr: `rpm` and `rpmlint` from the distribution, `copr-cli` 2.6 from `packaging/release/copr-cli-requirements.txt` (every package pinned with hashes, installed with `--require-hashes`). `cargo`, `git` and `tar` come with the runner image.
- scoop, aur, apt: nothing extra.
