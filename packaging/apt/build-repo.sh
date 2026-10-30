#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Builds or extends a signed apt repository (S-016a part of S-103f).
#   packaging/apt/build-repo.sh DEB OUTDIR [--keep N]
# Layout: dists/stable/main/binary-amd64, pool/main/k/kawaro. The DEB is added
# to OUTDIR/pool (which may already hold older versions), only the N newest
# versions stay (default 3, older ones are reported as "PRUNED path"), then
# Packages, Release, Release.gpg and InRelease are regenerated and both
# signatures are verified with gpgv against the exported public key.
# Environment:
#   GNUPGHOME             keyring that holds the signing key (required)
#   APT_SIGN_FPR          fingerprint of the signing key (required)
#   APT_SIGNING_PASSPHRASE  optional passphrase, read through a pipe only
# Also writes kawaro-archive-keyring.gpg and kawaro-archive.asc in OUTDIR.
set -euo pipefail

[ "$#" -ge 2 ] || { echo "usage: build-repo.sh DEB OUTDIR [--keep N]" >&2; exit 2; }
deb="$1" out="$2" keep=3
shift 2
while [ "$#" -gt 0 ]; do
  case "$1" in
    --keep) [ "$#" -ge 2 ] && [[ "$2" =~ ^[1-9][0-9]*$ ]] || { echo "build-repo: bad --keep" >&2; exit 2; }; keep="$2"; shift 2 ;;
    *) echo "usage: build-repo.sh DEB OUTDIR [--keep N]" >&2; exit 2 ;;
  esac
done
: "${GNUPGHOME:?build-repo: GNUPGHOME is not set}"
: "${APT_SIGN_FPR:?build-repo: APT_SIGN_FPR is not set}"
[ -f "$deb" ] || { echo "build-repo: no such DEB: $deb" >&2; exit 2; }

pkg="$(dpkg-deb -f "$deb" Package)"
ver="$(dpkg-deb -f "$deb" Version)"
arch="$(dpkg-deb -f "$deb" Architecture)"
[ "$arch" = amd64 ] || { echo "build-repo: architecture '$arch' is not amd64" >&2; exit 1; }

pooldir="pool/main/${pkg:0:1}/$pkg"
mkdir -p "$out/$pooldir"
cp -- "$deb" "$out/$pooldir/${pkg}_${ver}_${arch}.deb"

# Retention: newest N by version, older files are reported and removed.
mapfile -t versions < <(
  for f in "$out/$pooldir/${pkg}_"*"_${arch}.deb"; do
    v="${f##*/"${pkg}"_}"; echo "${v%_"${arch}".deb}"
  done | sort -V
)
if [ "${#versions[@]}" -gt "$keep" ]; then
  for v in "${versions[@]:0:${#versions[@]}-keep}"; do
    rm -f -- "$out/$pooldir/${pkg}_${v}_${arch}.deb"
    echo "PRUNED $pooldir/${pkg}_${v}_${arch}.deb"
  done
fi

idx="dists/stable/main/binary-amd64"
mkdir -p "$out/$idx"
(
  cd "$out"
  rm -rf -- "$idx/by-hash" dists/stable/Release dists/stable/Release.gpg dists/stable/InRelease
  apt-ftparchive --arch amd64 packages pool >"$idx/Packages"
  gzip -9n -c "$idx/Packages" >"$idx/Packages.gz"
  printf 'Archive: stable\nComponent: main\nOrigin: Kawaro\nLabel: Kawaro\nArchitecture: amd64\n' >"$idx/Release"
  apt-ftparchive \
    -o APT::FTPArchive::Release::Origin=Kawaro \
    -o APT::FTPArchive::Release::Label=Kawaro \
    -o APT::FTPArchive::Release::Suite=stable \
    -o APT::FTPArchive::Release::Codename=stable \
    -o APT::FTPArchive::Release::Architectures=amd64 \
    -o APT::FTPArchive::Release::Components=main \
    -o APT::FTPArchive::Release::Description="Kawaro stable repository" \
    -o APT::FTPArchive::Release::Acquire-By-Hash=yes \
    release dists/stable >dists/stable/Release.new
  mv dists/stable/Release.new dists/stable/Release
  # by-hash copies come after the Release file so they are not listed in it.
  mkdir -p "$idx/by-hash/SHA256"
  for f in Packages Packages.gz Release; do
    h="$(sha256sum "$idx/$f" | cut -d' ' -f1)"
    cp -- "$idx/$f" "$idx/by-hash/SHA256/$h"
  done
)

gpg_sign=(gpg --batch --yes --pinentry-mode loopback --passphrase-fd 3 --local-user "$APT_SIGN_FPR")
"${gpg_sign[@]}" --armor --detach-sign -o "$out/dists/stable/Release.gpg" "$out/dists/stable/Release" \
  3< <(printf '%s' "${APT_SIGNING_PASSPHRASE:-}") 2>/dev/null
"${gpg_sign[@]}" --clearsign -o "$out/dists/stable/InRelease" "$out/dists/stable/Release" \
  3< <(printf '%s' "${APT_SIGNING_PASSPHRASE:-}") 2>/dev/null

gpg --batch --export "$APT_SIGN_FPR" >"$out/kawaro-archive-keyring.gpg" 2>/dev/null
gpg --batch --armor --export "$APT_SIGN_FPR" >"$out/kawaro-archive.asc" 2>/dev/null
[ -s "$out/kawaro-archive-keyring.gpg" ] || { echo "build-repo: the public key could not be exported" >&2; exit 1; }

gpgv --keyring "$out/kawaro-archive-keyring.gpg" "$out/dists/stable/Release.gpg" "$out/dists/stable/Release" >/dev/null 2>&1 \
  || { echo "build-repo: Release.gpg does not verify" >&2; exit 1; }
gpgv --keyring "$out/kawaro-archive-keyring.gpg" "$out/dists/stable/InRelease" >/dev/null 2>&1 \
  || { echo "build-repo: InRelease does not verify" >&2; exit 1; }
echo "BUILT $pkg $ver"
