# apt channel (S-103f)

A signed apt repository for Debian, Ubuntu and Kali on amd64, published to S3 compatible object storage (Cloudflare R2, decided in S-016) by `scripts/channels/apt.sh vX.Y.Z [--dry-run]`.

## Install (users)

`OWNER-REPO-URL` stands for the public repository URL once the owner has set it up (for example `https://apt.OWNER.example`).

```
sudo install -d -m 0755 /etc/apt/keyrings
curl -fsSL OWNER-REPO-URL/kawaro-archive-keyring.gpg | sudo tee /etc/apt/keyrings/kawaro-archive-keyring.gpg >/dev/null
```

Then either a deb822 file, `/etc/apt/sources.list.d/kawaro.sources`:

```
Types: deb
URIs: OWNER-REPO-URL
Suites: stable
Components: main
Architectures: amd64
Signed-By: /etc/apt/keyrings/kawaro-archive-keyring.gpg
```

or the one line form, `/etc/apt/sources.list.d/kawaro.list`:

```
deb [arch=amd64 signed-by=/etc/apt/keyrings/kawaro-archive-keyring.gpg] OWNER-REPO-URL stable main
```

Finally `sudo apt update && sudo apt install kawaro`. Upgrades come with the normal `sudo apt upgrade`. The key is scoped to this repository by `Signed-By`; it is never added to the global trust store and no unauthenticated option is used or needed. An armored copy of the same key is at `OWNER-REPO-URL/kawaro-archive.asc`. Check its fingerprint against the one in the README before trusting it.

Kali is Debian based and uses the same lines.

## Layout

```
dists/stable/{InRelease,Release,Release.gpg}
dists/stable/main/binary-amd64/{Packages,Packages.gz,Release,by-hash/SHA256/...}
pool/main/k/kawaro/kawaro_X.Y.Z_amd64.deb
kawaro-archive-keyring.gpg   kawaro-archive.asc
```

The suite is `stable` and the component `main`. The pool keeps the 3 newest versions; older objects are removed only after the new metadata is published. Release carries `Acquire-By-Hash: yes`.

## How the channel works

1. The DEB comes from `APT_DEB_FILE`, else `dist/`, else the release asset (checked against `SHA256SUMS`).
2. The key from `APT_SIGNING_KEY` is imported into a temporary `GNUPGHOME` (mode 0700, under `mktemp`) that an `EXIT` trap wipes. The fingerprint must equal the committed public key `packaging/keys/kawaro-archive.asc`, or the run fails before any upload.
3. `packaging/apt/build-repo.sh DEB OUTDIR [--keep N]` adds the DEB to the existing pool (synced down first), prunes to the newest 3, regenerates `Packages` and `Release` with `apt-ftparchive`, signs `Release.gpg` and `InRelease`, and verifies both with `gpgv` against the exported public key.
4. Upload order: phase 1 the new pool object, phase 2 `Packages`, `Packages.gz`, the by-hash entries, the keyring files, phase 3 `Release`, `Release.gpg` and `InRelease` last, then removal of pruned pool objects.
5. A rerun of the same tag finds the version in the published `Packages` with an identical DEB and prints `UNCHANGED apt: X.Y.Z` without uploading. A prerelease tag prints `SKIP apt: prerelease tag`.

Without a variable below the script prints `SKIP apt: NAME not configured` and exits 10. `--dry-run` prints the three phases and touches nothing.

## Secrets and variables (names only)

| Name | Kind | Meaning |
| --- | --- | --- |
| `APT_SIGNING_KEY` | secret | armored private key, user id `Kawaro release signing` |
| `APT_SIGNING_PASSPHRASE` | secret | its passphrase, if any |
| `APT_R2_ACCESS_KEY_ID`, `APT_R2_SECRET_ACCESS_KEY` | secret | object storage token limited to the one bucket |
| `APT_R2_ACCOUNT_ID` | variable | forms the endpoint `https://ID.r2.cloudflarestorage.com` |
| `APT_R2_BUCKET` | variable | bucket name |
| `CANONICAL_REPO` | variable | `OWNER/NAME` of this project |

## OWNER ACTIONS

No key material and no personal identity belong in the repository.

1. On a trusted machine create a signing key with the generic user id `Kawaro release signing` (Ed25519 or RSA 4096), a passphrase, an offline backup and a revocation certificate.
2. Export the private key armored and store it as the secret `APT_SIGNING_KEY` (and the passphrase as `APT_SIGNING_PASSPHRASE`) in the protected channel environment. Never paste it into a log, issue or chat.
3. First public key publication: commit only the armored public key as `packaging/keys/kawaro-archive.asc`, record its 40 character fingerprint in the README, and push. The channel skips until this file exists.
4. Create the bucket, attach the public custom domain (leave any development public URL disabled), create a token limited to this bucket, and set the four `APT_R2_*` names.
5. After the first stable tag, run the install lines above in a clean Debian, Ubuntu and Kali container.
6. Rotate the token before it expires. A key rotation means a new committed public key and a new keyring file for users.

## Debian and Kali official archives (manual, slow path)

Getting into the official archives is a maintainer process, not automated here and no job exists for it. Our own repository works on Debian, Ubuntu and Kali meanwhile.

1. Package the source to Debian policy (`debian/` directory, source package, a licence review of the vendored crates, since Debian builds offline from packaged dependencies).
2. Find a sponsor: file an intent to package (ITP) bug against `wnpp`, upload the source to mentors.debian.net, and ask on the Debian mentors list for a Debian Developer to sponsor the upload. Expect review rounds over weeks or months.
3. Once in Debian unstable it reaches Ubuntu through the normal sync, and Kali takes Debian packages automatically when they fit its toolset.
4. For Kali specifically, a tool must be requested at bugs.kali.org and accepted by the Kali team; the manual steps above come first.
