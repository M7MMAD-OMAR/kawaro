# AUR channel: kawaro-bin (S-103e)

Install on Arch based systems:

5. The secret name is `AUR_SSH_KEY` everywhere (S-078 and S-103 used `AUR_SSH_PRIVATE_KEY` before the scripts fixed the name).
yay -S kawaro-bin
```

## What the package does

`kawaro-bin` downloads the release DEB (`kawaro_X.Y.Z_amd64.deb`), verifies its SHA-256 from the release `SHA256SUMS`, and unpacks its `data.tar.*` into the package, so no extra Linux asset exists and the four asset check of the release workflow stays valid. It provides `kawaro` and conflicts with a future source package `kawaro` (S-078), so the two can never be installed together. License `GPL-3.0-or-later`.

The DEB runs `systemctl --global enable kawaro.service` in its post install script; the Arch package does not. After installing, run `systemctl --user enable --now kawaro.service`, or just use the right click menu, which starts the daemon on demand.

## Files

- `packaging/aur/PKGBUILD.tpl` and `packaging/aur/SRCINFO.tpl`: templates.
- `packaging/aur/generate.sh vX.Y.Z SHA256SUMS OWNER/REPO OUTDIR`: writes `PKGBUILD` and `.SRCINFO` from the same values, so they cannot disagree. `makepkg --printsrcinfo` is not needed offline; a test checks `.SRCINFO` against the values bash computes from the `PKGBUILD`. Confirming byte equality with real `makepkg --printsrcinfo` and `namcap` needs an Arch container and is a pending live check.
- `scripts/channels/aur.sh vX.Y.Z [--dry-run]`: the channel script.
- `packaging/aur/known_hosts`: the three AUR host keys (ed25519, rsa, ecdsa). They were read with `ssh-keyscan` and compared with the fingerprints the Arch Linux wiki publishes (`SHA256:RFzBCUItH9LZS0cKB5UE6ceAYhBD5C8GeOBip8Z11+4` for ed25519, `SHA256:5s5cIyReIfNNVGRFdDbe3hdYiI5OelHGpw2rOUud3Q8` for rsa, `SHA256:uTa/0PndEgPZTf76e1DFqXKJEXKsn7m9ivhLQtzGOCI` for ecdsa); all three matched.

## Secrets and variables (names only)

| Name | Kind | Meaning |
| --- | --- | --- |
| `AUR_SSH_KEY` | secret | private key whose public half is on the AUR account |
| `AUR_ENABLED` | variable | must be `true`; unset until an AUR account exists |
| `CANONICAL_REPO` | variable | `OWNER/NAME` of this project |

Missing `AUR_SSH_KEY`, `known_hosts` or `AUR_ENABLED` prints one `SKIP aur: ... not configured` line and exits 10. The script writes the key to a private temporary file removed on exit, connects with `StrictHostKeyChecking=yes` and `UserKnownHostsFile` set to the committed file, clones, replaces `PKGBUILD` and `.SRCINFO`, commits and pushes to `master` without force. A rerun prints `UNCHANGED aur: X.Y.Z` and creates no commit.

## Status: waiting for an account (platform limit)

The channel is a clean `SKIP aur: AUR_ENABLED not configured`. Everything else is in place: the host keys are committed and verified, an ed25519 key pair was generated (its private half is the secret `AUR_SSH_KEY` of the `aur-publishing` environment and a private backup, never in the repository), and the script and the package generator are tested offline.

An AUR account has to be created by a person, because the sign up form uses a CAPTCHA, which automation must not pass. Once an account exists: paste the public half of the generated key into the account's SSH Public Key setting (the key file is the private backup `aur/aur_ed25519.pub` next to the others), then run `gh variable set AUR_ENABLED --body true`. The next stable tag creates the `kawaro-bin` package repository with its first push; the script clones the not yet existing repository (AUR creates it on the first push) and pushes `PKGBUILD` and `.SRCINFO` to `master` without force.
