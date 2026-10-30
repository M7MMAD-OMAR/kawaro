# Scoop channel (S-103c)

Install on Windows, per user, no administrator rights:

```
scoop bucket add kawaro https://github.com/M7MMAD-OMAR/scoop-kawaro
scoop install kawaro/kawaro
```

## What the manifest installs

The portable ZIP of the release (`kawaro-X.Y.Z-windows-x64.zip`), not the MSI. Scoop installs under the user's profile and uninstalls by deleting that folder, which is exactly what the ZIP is for. The MSI needs Windows Installer, writes machine and per-user registry entries and an Explorer menu, and Scoop can only unpack it, not run its custom actions. The consequence: the Scoop install has the `kawaro` and `kawarod` commands on the path but not the Explorer right click entry the MSI registers. Users who want the menu use the MSI or WinGet.

## Files

- `packaging/scoop/kawaro.json.tpl`: the manifest template.
- `packaging/scoop/generate.sh vX.Y.Z SHA256SUMS OWNER/REPO`: fills version, URL and the ZIP hash from the release `SHA256SUMS`.
- `packaging/scoop/validate.sh kawaro.json [SHA256SUMS]`: offline check of the fields Scoop requires, and of the hash against `SHA256SUMS`.
- `scripts/channels/scoop.sh vX.Y.Z [--dry-run]`: the channel script.
- `checkver` is `github` and `autoupdate` rebuilds URL, `extract_dir` and hash (read from the release `SHA256SUMS`), so the bucket also updates with Scoop's own `checkver` tooling.

## Secrets and variables (names only)

| Name | Kind | Meaning |
| --- | --- | --- |
| `SCOOP_DEPLOY_KEY` | secret | ssh private key, a deploy key with write access to the bucket repository only |
| `BUCKET_REPO` | variable | `OWNER/NAME` of the bucket repository |
| `CANONICAL_REPO` | variable | `OWNER/NAME` of this project |

Without one of these names the script prints `SKIP scoop: NAME not configured` and exits 10. The key is written to a 0600 file that an `EXIT` trap removes; git runs ssh with `StrictHostKeyChecking=yes` and GitHub's published host keys from `packaging/release/github_known_hosts`. The push is a plain push of a new commit, never forced; a rerun with the same tag prints `UNCHANGED scoop: X.Y.Z` and creates no commit.

The bucket repository is public and holds `bucket/kawaro.json`. Its deploy key can be replaced at any time in the repository settings.
