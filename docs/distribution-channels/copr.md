# Fedora COPR channel (S-103d)

Kawaro is built for Fedora in a COPR project from the release tag. Users
get signed packages from COPR's own repository and updates through `dnf`.

## Install

```
sudo dnf copr enable ACCOUNT/kawaro
sudo dnf install kawaro
```

`ACCOUNT` is the COPR account that owns the project. The command comes from
the `copr` entry of `packaging/install-commands.json`, and
`scripts/release/install-commands.py check` keeps this page equal to it.

## How a release reaches COPR

`scripts/channels/copr.sh vX.Y.Z` (run from the repository root):

1. Refuses a prerelease tag (`SKIP copr: prerelease tag`, exit 10).
2. Renders and lints `packaging/copr/kawaro.spec` (`rpmspec -P`, and
   `rpmlint` when installed). `--dry-run` stops here and prints the plan.
3. Makes two sources: `git archive` of the tag, and `cargo vendor --locked`
   into `vendor-crates/` (not `vendor/`, which holds the in tree softbuffer
   patch). `rpmbuild -bs` turns them into a source RPM.
4. Writes `COPR_CONFIG` to a file inside a private temporary directory that
   an `EXIT` trap removes, also after a failure.
5. Runs `copr-cli build --nowait OWNER/kawaro <srpm>` for exactly the three
   chroots `fedora-43-x86_64`, `fedora-44-x86_64`, `fedora-45-x86_64`
   (S-016 decision 4), then `copr-cli watch-build`, then requires
   `copr-cli status` to print `succeeded`. Any other result exits nonzero.

COPR builds without network, so the spec builds with
`cargo build --release --locked --offline`. It lays out the files with
`packaging/linux/stage.sh`, the same script as the release RPM, and
`tests/release/s103d_copr.sh` checks that the file list, the weak
dependencies and the scriptlets equal `packaging/linux/kawaro.spec`.

## Environment

| Name | Kind | Meaning |
| --- | --- | --- |
| `COPR_CONFIG` | secret | Contents of the COPR API config file. Never printed. |
| `COPR_PROJECT` | variable | `OWNER/kawaro`. |

A missing name prints `SKIP copr: <NAME> not configured` and exits 10.
`COPR_SOURCE_REF` is a test hook (the git ref archived instead of the tag).

## Status: waiting for an account (platform limit)

The channel is a clean `SKIP copr: COPR_CONFIG not configured`. A COPR account needs a Fedora Account System login, which a person has to create. Once it exists: create the project `kawaro` with the three chroots, download the API config from the COPR API page, run `gh secret set COPR_CONFIG --env copr-publishing < copr-config-file`, and run `gh variable set COPR_PROJECT --body ACCOUNT/kawaro`. The API config expires, so record its expiry and rotate it before then.

## Not yet verified

- No real COPR build has run. The spec and the script are tested offline with
  stubs; `BuildRequires` and the vendored offline build are confirmed only by
  the first live build (S-016c mock tests and S-016d live checks).
- The check "all three chroots succeeded" relies on `copr-cli status` for the
  whole build. Whether it reports per chroot is to confirm on the first run.
- A rerun for the same tag submits another build of the same version; it does
  not report `UNCHANGED`.
- The per package licence list of the vendored crates is not generated yet.
