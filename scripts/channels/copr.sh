#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-103d: submit a stable release to Fedora COPR.
#
#   scripts/channels/copr.sh vX.Y.Z [--dry-run]
#
# Run from the repository root. Environment (names only, never printed):
#   COPR_CONFIG   the contents of the COPR API config file (secret)
#   COPR_PROJECT  OWNER/kawaro, the COPR project (not secret)
# Test hook: COPR_SOURCE_REF is the git ref archived as the source; it
# defaults to the tag.
#
# Exit 0 done, 10 clean SKIP (nothing configured, or a prerelease tag), any
# other code a failure. The three chroots are fixed by S-016 decision 4.
set -euo pipefail

CHROOTS=(fedora-43-x86_64 fedora-44-x86_64 fedora-45-x86_64)
SKIP=10

tag=""
dry=0
for a in "$@"; do
  case "$a" in
    --dry-run) dry=1 ;;
    -*) echo "copr: unknown option $a" >&2; exit 2 ;;
    *) tag="$a" ;;
  esac
done

if ! [[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
  echo "copr: usage: copr.sh vX.Y.Z [--dry-run]" >&2
  exit 2
fi
if [[ "$tag" == *-* ]]; then
  echo "SKIP copr: prerelease tag"
  exit "$SKIP"
fi
version="${tag#v}"

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
spec="$root/packaging/copr/kawaro.spec"

work="$(mktemp -d "${TMPDIR:-/tmp}/kawaro-copr.XXXXXX")"
# The API config lives only inside $work, which is removed on every exit.
trap 'rm -rf "$work"' EXIT

lint_spec() {
  if command -v rpmspec >/dev/null 2>&1; then
    rpmspec -P --define "kawaro_version $version" "$spec" >"$work/rendered.spec"
    grep -qx "Version:        $version" "$work/rendered.spec" \
      || { echo "copr: rendered spec does not carry version $version" >&2; exit 1; }
    echo "copr: spec renders (rpmspec) for $version"
  else
    echo "copr: rpmspec not installed, spec render skipped"
  fi
  if command -v rpmlint >/dev/null 2>&1; then
    if rpmlint "$spec" 2>&1 | grep -E '(^|[ :])E:'; then
      echo "copr: rpmlint reports errors" >&2
      exit 1
    fi
    echo "copr: rpmlint reports 0 errors"
  fi
}

copr_missing=""
[ -n "${COPR_CONFIG:-}" ] || copr_missing="COPR_CONFIG"
[ -n "$copr_missing" ] || [ -n "${COPR_PROJECT:-}" ] || copr_missing="COPR_PROJECT"

if [ "$dry" -eq 1 ]; then
  lint_spec
  if [ -n "$copr_missing" ]; then
    echo "SKIP copr: $copr_missing not configured"
    exit "$SKIP"
  fi
  rs=()
  for c in "${CHROOTS[@]}"; do rs+=(-r "$c"); done
  echo "DRYRUN copr: would publish $version"
  echo "DRYRUN copr: would run: copr-cli --config <temporary file> build --nowait ${COPR_PROJECT} <srpm> ${rs[*]}"
  echo "DRYRUN copr: would wait with copr-cli watch-build and require status succeeded for: ${CHROOTS[*]}"
  exit 0
fi

if [ -n "$copr_missing" ]; then
  echo "SKIP copr: $copr_missing not configured"
  exit "$SKIP"
fi

lint_spec

# Sources: the tagged tree and its locked vendored crates.
ref="${COPR_SOURCE_REF:-$tag}"
src="$work/src"
mkdir -p "$src" "$work/rpm/SOURCES" "$work/rpm/SRPMS"
git -C "$root" archive --prefix="kawaro-$version/" "$ref" | tar -x -C "$src"
tar -C "$src" -czf "$work/rpm/SOURCES/kawaro-$version.tar.gz" "kawaro-$version"
(
  cd "$src/kawaro-$version"
  # vendor-crates, not vendor: vendor/ holds the in tree softbuffer patch.
  cargo vendor --locked vendor-crates >/dev/null
  tar -cJf "$work/rpm/SOURCES/kawaro-$version-vendor.tar.xz" vendor-crates
)
rpmbuild -bs "$spec" --define "_topdir $work/rpm" --define "kawaro_version $version" >/dev/null
srpm="$(ls "$work"/rpm/SRPMS/kawaro-"$version"-*.src.rpm)"

# The API config, owner only, never echoed.
cfg="$work/copr.conf"
( umask 077; printf '%s\n' "$COPR_CONFIG" >"$cfg" )

rs=()
for c in "${CHROOTS[@]}"; do rs+=(-r "$c"); done
out="$(copr-cli --config "$cfg" build --nowait "$COPR_PROJECT" "$srpm" "${rs[@]}")"
id="$(sed -n 's/^Created builds: \([0-9][0-9]*\).*/\1/p' <<<"$out" | head -1)"
[ -n "$id" ] || { echo "copr: could not read the build id from copr-cli" >&2; exit 1; }
echo "copr: build $id submitted for ${CHROOTS[*]}"

if ! copr-cli --config "$cfg" watch-build "$id"; then
  echo "copr: build $id did not succeed in every chroot" >&2
  exit 1
fi
state="$(copr-cli --config "$cfg" status "$id")"
if [ "$state" != "succeeded" ]; then
  echo "copr: build $id state is $state, expected succeeded" >&2
  exit 1
fi
echo "copr: build $id succeeded in ${CHROOTS[*]}"
