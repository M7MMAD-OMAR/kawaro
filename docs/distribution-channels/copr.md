# Fedora COPR channel (S-103d)

Kawaro is built for Fedora in a COPR project from the release tag. Users
get signed packages from COPR's own repository and updates through `dnf`.

## Install

```
sudo dnf copr enable OWNER/kawaro
sudo dnf install kawaro
```

`OWNER` is the COPR account that owns the project. The command is rendered
from the `copr` entry of `packaging/install-commands.json` once S-016 adds
that file; until then this page is the only place that states it.

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

## Owner actions

Create the COPR project, enable the three chroots, create the API config and
store it as `COPR_CONFIG`; record its expiry and rotate it before then.

## Not yet verified

- No real COPR build has run. The spec and the script are tested offline with
  stubs; `BuildRequires` and the vendored offline build are confirmed only by
  the first live build (S-016c mock tests and S-016d live checks).
- The check "all three chroots succeeded" relies on `copr-cli status` for the
  whole build. Whether it reports per chroot is to confirm on the first run.
- A rerun for the same tag submits another build of the same version; it does
  not report `UNCHANGED`.
- The per package licence list of the vendored crates is not generated yet.
