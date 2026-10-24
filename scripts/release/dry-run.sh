#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Rehearses a release on this machine without publishing anything (S-055).
#
#   scripts/release/dry-run.sh VERSION [--no-build]
#
# VERSION is the cargo version without a leading v (0.4.0-beta.1). The script
# copies the tracked files into a temporary folder (no branch, no worktree),
# sets the workspace version there, builds the RPM and DEB, the Windows zip and
# MSI, signs a manifest with a throwaway minisign key, runs check-versions.sh,
# writes SHA256SUMS and prints the final file list. It makes no network call and
# publishes nothing; the source tree is left untouched.
#
# Environment:
#   BIN_DIR           Linux release binaries; default $DRYRUN_LINUX_TARGET/release
#   DRYRUN_LINUX_TARGET  cargo target dir of the Linux build; default <repo>/target
#   CARGO_TARGET_DIR  Windows tree, uses $CARGO_TARGET_DIR/x86_64-pc-windows-gnu/release;
#                     default as in scripts/windows/build.sh
#   DRYRUN_OUT        output folder; default a fresh temporary folder
# --no-build reuses the trees above instead of compiling.
#
# Exit codes: 0 ok; 1 a check failed or a step broke; 2 bad version or a
# missing tool (the message names it).
#
# Run it inside the project scope (systemd-run --user --scope -p MemoryMax=6G
# -p CPUQuota=200%) when it builds.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
die() { echo "dry-run: $2" >&2; exit "$1"; }

[ "$#" -ge 1 ] || die 2 "usage: $0 VERSION [--no-build]"
VERSION="$1"; shift
BUILD=1
for a in "$@"; do
  case "$a" in
    --no-build) BUILD=0 ;;
    *) die 2 "unknown argument: $a" ;;
  esac
done
SEMVER='^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$'
[[ "$VERSION" =~ $SEMVER ]] || die 2 "not a version like 0.4.0 or 0.4.0-beta.1 (no leading v): $VERSION"

need=(git tar rpm rpmbuild dpkg-deb wixl msiinfo minisign jq unzip zip ar objdump sha256sum)
[ "$BUILD" = 0 ] || need+=(cargo x86_64-w64-mingw32-gcc)
for t in "${need[@]}"; do
  if ! command -v "$t" >/dev/null 2>&1; then
    case "$t" in
      x86_64-w64-mingw32-gcc) die 2 "missing tool: mingw (x86_64-pc-windows-gnu target, install mingw-w64)" ;;
      wixl | msiinfo) die 2 "missing tool: $t (install msitools)" ;;
      *) die 2 "missing tool: $t" ;;
    esac
  fi
done

PKGV="${VERSION//-/\~}"
OUT="${DRYRUN_OUT:-$(mktemp -d)}"
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# A copy of the tracked files, as they are in the working tree.
COPY="$WORK/src"
mkdir -p "$COPY"
(cd "$ROOT" && git ls-files -z --cached | tar --null -T - --ignore-failed-read -cf - 2>/dev/null) | tar -xf - -C "$COPY"
awk -v v="$VERSION" '/^\[workspace.package\]/{p=1} p && /^version *=/{print "version = \"" v "\""; p=0; next} {print}' \
  "$COPY/Cargo.toml" > "$WORK/Cargo.toml.new"
mv "$WORK/Cargo.toml.new" "$COPY/Cargo.toml"
awk -v v="$VERSION" '/^name = "sbarhive-/{print; getline; print "version = \"" v "\""; next} {print}' \
  "$COPY/Cargo.lock" > "$WORK/Cargo.lock.new"
mv "$WORK/Cargo.lock.new" "$COPY/Cargo.lock"

export SBARHIVE_WIN_SCOPE=1 CI=true
WIN_TARGET="${CARGO_TARGET_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/sbarhive-win-target}"
LIN_TARGET="${DRYRUN_LINUX_TARGET:-$ROOT/target}"

if [ "$BUILD" = 1 ]; then
  echo "dry-run: building the Linux binaries" >&2
  (cd "$COPY" && CARGO_TARGET_DIR="$LIN_TARGET" cargo build --offline -j 2 --release --workspace) >&2
  echo "dry-run: building the Windows binaries" >&2
  (cd "$COPY" && CARGO_TARGET_DIR="$WIN_TARGET" scripts/windows/build.sh) >&2
fi
BIN="${BIN_DIR:-$LIN_TARGET/release}"

echo "dry-run: packing" >&2
PKGS="$WORK/pkgs"
"$COPY/packaging/linux/build-packages.sh" "$BIN" "$PKGS" >/dev/null
CARGO_TARGET_DIR="$WIN_TARGET" "$COPY/scripts/windows/make-zip.sh" >/dev/null
CARGO_TARGET_DIR="$WIN_TARGET" "$COPY/scripts/windows/make-msi.sh" >/dev/null

RPM="sbarhive-$PKGV-1.x86_64.rpm"
DEB="sbarhive_${PKGV}_amd64.deb"
ZIP="sbarhive-$VERSION-windows-x64.zip"
MSI="sbarhive-$VERSION-windows-x64.msi"
cp "$PKGS/$RPM" "$PKGS/$DEB" "$OUT/"
cp "$WIN_TARGET/dist/$ZIP" "$WIN_TARGET/dist/$MSI" "$OUT/"

# Manifest assets, shaped like the release listing the CI job reads.
BASE="https://github.com/M7MMAD-OMAR/sbarhive/releases/download/v$VERSION"
{
  for f in "$RPM" "$DEB" "$ZIP" "$MSI"; do
    jq -n --arg n "$f" --arg u "$BASE/$f" --argjson s "$(stat -c %s "$OUT/$f")" '{name: $n, url: $u, size: $s}'
  done
} | jq -s '{assets: .}' > "$WORK/assets.json"

# A throwaway key pair: the real update key is never read here.
minisign -G -W -f -p "$OUT/dry-run.pub" -s "$WORK/dry-run.key" >/dev/null 2>&1
channels=(beta)
case "$VERSION" in *-*) ;; *) channels+=(stable) ;; esac
for c in "${channels[@]}"; do
  "$COPY/scripts/make-update-manifest.sh" --dist "$OUT" "$c" "$VERSION" "$WORK/assets.json" "$OUT/$c.json"
  UPDATE_SIGNING_PASSWORD="" "$COPY/scripts/sign-update-manifest.sh" "$WORK/dry-run.key" "$OUT/$c.json"
  minisign -V -p "$OUT/dry-run.pub" -m "$OUT/$c.json" >/dev/null
done

(cd "$OUT" && sha256sum "$RPM" "$DEB" "$ZIP" "$MSI" > SHA256SUMS && sha256sum -c SHA256SUMS >/dev/null)

for f in "$OUT"/*; do echo "$f"; done
"$HERE/check-versions.sh" "$VERSION" --root "$COPY" --out "$OUT"
