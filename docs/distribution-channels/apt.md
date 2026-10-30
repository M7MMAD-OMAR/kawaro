# apt channel (S-103f)

A signed apt repository for Debian, Ubuntu and Kali on amd64, served by GitHub Pages from the public repository `kawaro-apt` and published by `scripts/channels/apt.sh vX.Y.Z [--dry-run]`.

## Install (users)

The repository is `https://m7mmad-omar.github.io/kawaro-apt`. One command (key, source line, update, install):

```
sudo install -d -m 0755 /etc/apt/keyrings && curl -fsSL https://m7mmad-omar.github.io/kawaro-apt/kawaro-archive-keyring.gpg | sudo tee /etc/apt/keyrings/kawaro-archive-keyring.gpg >/dev/null && echo 'deb [arch=amd64 signed-by=/etc/apt/keyrings/kawaro-archive-keyring.gpg] https://m7mmad-omar.github.io/kawaro-apt stable main' | sudo tee /etc/apt/sources.list.d/kawaro.list >/dev/null && sudo apt update && sudo apt install kawaro
```

The same source as a deb822 file, `/etc/apt/sources.list.d/kawaro.sources`, replaces the `.list` line if you prefer it:

```
Types: deb
URIs: https://m7mmad-omar.github.io/kawaro-apt
Suites: stable
Components: main
Architectures: amd64
Signed-By: /etc/apt/keyrings/kawaro-archive-keyring.gpg
```

Upgrades come with the normal `sudo apt upgrade`. The key is scoped to this repository by `Signed-By`; it is never added to the global trust store and no unauthenticated option is used or needed. An armored copy of the same key is at `https://m7mmad-omar.github.io/kawaro-apt/kawaro-archive.asc`. Its fingerprint is `4723 C2DC 6CB7 A504 61B5 1D3D 6C36 8B7D 1054 F87D`; compare it with `gpg --show-keys kawaro-archive.asc` before trusting a copy. The same key is committed in this repository as `packaging/keys/kawaro-archive.asc`.

Kali is Debian based and uses the same lines.

## Layout

```
dists/stable/{InRelease,Release,Release.gpg}
dists/stable/main/binary-amd64/{Packages,Packages.gz,Release,by-hash/SHA256/...}
pool/main/k/kawaro/kawaro_X.Y.Z_amd64.deb
kawaro-archive-keyring.gpg   kawaro-archive.asc   .nojekyll
```

The suite is `stable` and the component `main`. The pool keeps the 3 newest versions; an older one is removed in the same commit that adds the new one. Release carries `Acquire-By-Hash: yes`. GitHub Pages serves the files as they are (`.nojekyll`).

## How the channel works

1. The DEB comes from `APT_DEB_FILE`, else `dist/`, else the release asset (checked against `SHA256SUMS`).
2. The key from `APT_SIGNING_KEY` is imported into a temporary `GNUPGHOME` (mode 0700, under `mktemp`) that an `EXIT` trap wipes. The fingerprint must equal the committed public key `packaging/keys/kawaro-archive.asc`, or the run fails before anything is built. A `--dry-run` that has the key does the same check.
3. The Pages repository is cloned over ssh with the deploy key (written to a 0600 file, removed on exit; `StrictHostKeyChecking=yes` with GitHub's published host keys in `packaging/release/github_known_hosts`).
4. `packaging/apt/build-repo.sh DEB OUTDIR [--keep N]` adds the DEB to the existing pool, prunes to the newest 3, regenerates `Packages` and `Release` with `apt-ftparchive`, signs `Release.gpg` and `InRelease`, and verifies both with `gpgv` against the exported public key.
5. Everything is one commit pushed to `main` (never forced), so a reader never sees a half published index. A rerun of the same tag finds the version in the published `Packages` with an identical DEB and prints `UNCHANGED apt: X.Y.Z` without a commit. A prerelease tag prints `SKIP apt: prerelease tag`.

Without a name below the script prints `SKIP apt: NAME not configured` and exits 10. `--dry-run` prints the plan and touches nothing.

## Secrets and variables (names only)

| Name | Kind | Meaning |
| --- | --- | --- |
| `APT_SIGNING_KEY` | secret | armored private key, user id `Kawaro release signing` |
| `APT_SIGNING_PASSPHRASE` | secret | its passphrase; unset, because the key has none |
| `APT_DEPLOY_KEY` | secret | ssh private key, a deploy key with write access to the Pages repository only |
| `APT_REPO` | variable | `OWNER/NAME` of the Pages repository |
| `CANONICAL_REPO` | variable | `OWNER/NAME` of this project |

## Key custody

The signing key is a dedicated Ed25519 key with the user id `Kawaro release signing` (no email address) that expires in 20 years. It has no passphrase. That is acceptable only because the private half lives in the `apt-publishing` environment, which is limited to `v*` tags, and in a private backup outside the repository; a passphrase stored next to the key in the same place would add nothing. Rotating the key means a new committed public key and a new keyring file for users, and a deploy key can be replaced at any time in the settings of the Pages repository. The history of the Pages repository grows by one DEB per release; if it ever gets too large, the repository can be recreated, because every file in it is generated.

## Debian and Kali official archives (manual, slow path)

Getting into the official archives is a maintainer process, not automated here and no job exists for it. Our own repository works on Debian, Ubuntu and Kali meanwhile.

1. Package the source to Debian policy (`debian/` directory, source package, a licence review of the vendored crates, since Debian builds offline from packaged dependencies).
2. Find a sponsor: file an intent to package (ITP) bug against `wnpp`, upload the source to mentors.debian.net, and ask on the Debian mentors list for a Debian Developer to sponsor the upload. Expect review rounds over weeks or months.
3. Once in Debian unstable it reaches Ubuntu through the normal sync, and Kali takes Debian packages automatically when they fit its toolset.
4. For Kali specifically, a tool must be requested at bugs.kali.org and accepted by the Kali team; the manual steps above come first.
