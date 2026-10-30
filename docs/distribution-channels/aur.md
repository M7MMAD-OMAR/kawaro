# AUR channel: kawaro-bin (S-103e)

Install on Arch based systems:

```
yay -S kawaro-bin
```

## What the package does

`kawaro-bin` downloads the release DEB (`kawaro_X.Y.Z_amd64.deb`), verifies its SHA-256 from the release `SHA256SUMS`, and unpacks its `data.tar.*` into the package, so no extra Linux asset exists and the four asset check of the release workflow stays valid. It provides `kawaro` and conflicts with a future source package `kawaro` (S-078), so the two can never be installed together. License `GPL-3.0-or-later`.

The DEB runs `systemctl --global enable kawaro.service` in its post install script; the Arch package does not. After installing, run `systemctl --user enable --now kawaro.service`, or just use the right click menu, which starts the daemon on demand.

## Files

- `packaging/aur/PKGBUILD.tpl` and `packaging/aur/SRCINFO.tpl`: templates.
- `packaging/aur/generate.sh vX.Y.Z SHA256SUMS OWNER/REPO OUTDIR`: writes `PKGBUILD` and `.SRCINFO` from the same values, so they cannot disagree. `makepkg --printsrcinfo` is not needed offline; a test checks `.SRCINFO` against the values bash computes from the `PKGBUILD`. Confirming byte equality with real `makepkg --printsrcinfo` and `namcap` needs an Arch container and is a pending live check.
- `scripts/channels/aur.sh vX.Y.Z [--dry-run]`: the channel script.
- `packaging/aur/known_hosts`: the AUR host keys, committed by the owner (see below). It does not exist yet; without it the channel skips.

## Secrets and variables (names only)

| Name | Kind | Meaning |
| --- | --- | --- |
| `AUR_SSH_KEY` | secret | private key whose public half is on the AUR account |
| `CANONICAL_REPO` | variable | `OWNER/NAME` of this project |

Missing `AUR_SSH_KEY` or `known_hosts` prints one `SKIP aur: ... not configured` line and exits 10. The script writes the key to a private temporary file removed on exit, connects with `StrictHostKeyChecking=yes` and `UserKnownHostsFile` set to the committed file, clones, replaces `PKGBUILD` and `.SRCINFO`, commits and pushes to `master` without force. A rerun prints `UNCHANGED aur: X.Y.Z` and creates no commit.

## OWNER ACTIONS (first time registration, human only)

1. Register an AUR account and add a dedicated SSH public key to it.
2. Take the AUR host keys from the fingerprints published on the AUR site, check them with `ssh-keyscan aur.archlinux.org` against those fingerprints, and commit the verified lines as `packaging/aur/known_hosts`. Never disable host key checking.
3. Register the package name once by hand: generate the files with `packaging/aur/generate.sh`, clone `ssh://aur@aur.archlinux.org/kawaro-bin.git`, copy `PKGBUILD` and `.SRCINFO`, commit on `master`, and push. The channel script only updates an existing repository.
4. Store the private key as `AUR_SSH_KEY` in the protected channel environment and set `CANONICAL_REPO`.
5. Note: the S-078 story names the secret `AUR_SSH_PRIVATE_KEY`; the channel contract of S-103 uses `AUR_SSH_KEY`, which is the name in force.
