#!/usr/bin/env bash
# Signs Windows release artifacts through one audited path: every signature
# carries an RFC 3161 timestamp and is verified right after signing (S-083).
#   scripts/windows/sign-local.sh CERT_SOURCE RELEASE_ZIP [OUT_DIR]
#   scripts/windows/sign-local.sh CERT_SOURCE --sign FILE...
#   scripts/windows/sign-local.sh CERT_SOURCE --verify FILE...
# CERT_SOURCE names the certificate:
#   test          self-signed throwaway leaf in KAWARO_WIN_SIGN_DIR (default
#                 ~/.config/kawaro-signing/windows), created on demand and
#                 encrypted with the passphrase in signing.env there. It is
#                 not a public code signing certificate: Windows trusts the
#                 result only where you import the certificate (see
#                 scripts/windows/README.md, "Signed test build"). For
#                 testing on your own Windows machines and for the dry run.
#   pkcs12:PATH   a PKCS#12 bundle with the certificate and its key; the
#                 passphrase comes from the environment variable
#                 KAWARO_WIN_SIGN_PASS. This is the release path: the
#                 packages.yml windows job writes the release-signing
#                 secrets to a file and calls this script with it.
# RELEASE_ZIP is kawaro-<version>-windows-x64.zip from a release. The release
# mode signs kawaro.exe and kawarod.exe, repacks the zip (its inner
# SHA256SUMS rewritten over the signed bytes), builds the MSI from the signed
# programs with make-msi.sh, signs the MSI and writes into OUT_DIR (default
# ./dist-signed): the zip, the MSI, kawaro-test-signing.cer and SHA256SUMS.
# SHA256SUMS is always generated after signing, so it names the delivered
# bytes. --sign signs each FILE in place; --verify only checks each FILE
# (signature and RFC 3161 timestamp) and exits 1 when any check fails.
# KAWARO_WIN_SIGN_TS is the RFC 3161 timestamp server (default
# http://timestamp.digicert.com). It may not be empty: a signature without a
# timestamp dies with the certificate, so every run timestamps, and every
# verification requires a verified timestamp. KAWARO_WIN_SIGN_CAFILE and
# KAWARO_WIN_SIGN_TSA_CAFILE set the trust used when verifying (defaults: the
# test certificate itself, and the system store for a public timestamp
# authority).
# Needs osslsigncode and openssl; the release mode also needs wixl, zip and
# unzip. Exit 2 on a bad input or a missing certificate, 1 on a signing or
# verification failure.
set -eu
usage () {
  echo "usage: sign-local.sh CERT_SOURCE RELEASE_ZIP [OUT_DIR]" >&2
  echo "       sign-local.sh CERT_SOURCE --sign FILE..." >&2
  echo "       sign-local.sh CERT_SOURCE --verify FILE..." >&2
}
if [ $# -lt 2 ]; then
  usage
  exit 2
fi
cert_source="$1"
shift
mode=release
case "$1" in
  --sign)
    mode=sign
    shift
    [ $# -ge 1 ] || { usage; exit 2; }
    ;;
  --verify)
    mode=verify
    shift
    [ $# -ge 1 ] || { usage; exit 2; }
    ;;
  *)
    [ $# -le 2 ] || { usage; exit 2; }
    ;;
esac

# Refusals before any tool runs or file is created: the message names the
# missing input and nothing is written.
ts="${KAWARO_WIN_SIGN_TS-http://timestamp.digicert.com}"
if [ -z "$ts" ]; then
  echo "sign-local: KAWARO_WIN_SIGN_TS is empty; every signature needs an RFC 3161 timestamp" >&2
  exit 2
fi
kind=""
p12=""
case "$cert_source" in
  test)
    kind="test"
    ;;
  pkcs12:*)
    kind="pkcs12"
    p12="${cert_source#pkcs12:}"
    if [ ! -s "$p12" ]; then
      echo "sign-local: certificate $p12 not found or empty" >&2
      exit 2
    fi
    if [ -z "${KAWARO_WIN_SIGN_PASS+x}" ]; then
      echo "sign-local: KAWARO_WIN_SIGN_PASS is not set; pkcs12:$p12 needs its passphrase" >&2
      exit 2
    fi
    ;;
  *)
    echo "sign-local: unknown CERT_SOURCE $cert_source (want test or pkcs12:PATH)" >&2
    exit 2
    ;;
esac

for tool in osslsigncode openssl; do
  command -v "$tool" >/dev/null || { echo "sign-local: $tool not found" >&2; exit 2; }
done
if [ "$mode" = release ]; then
  for tool in wixl zip unzip; do
    command -v "$tool" >/dev/null || { echo "sign-local: $tool not found" >&2; exit 2; }
  done
fi

root="$(cd "$(dirname "$0")/../.." && pwd)"
umask 077
pass_file="$(mktemp "${TMPDIR:-/tmp}/kawaro-pass.XXXXXX")"
stage="$(mktemp -d "${TMPDIR:-/tmp}/kawaro-sign.XXXXXX")"
trap 'rm -rf -- "$stage" "$pass_file"' EXIT

# The passphrase goes to the tools through a file, never on a command line.
case "$kind" in
  test)
    sign_dir="${KAWARO_WIN_SIGN_DIR:-$HOME/.config/kawaro-signing/windows}"
    key="$sign_dir/key.pem"
    crt="$sign_dir/cert.pem"
    env_file="$sign_dir/signing.env"
    mkdir -p "$sign_dir"
    [ -f "$env_file" ] || printf 'KAWARO_WIN_SIGN_PASS=%s\n' "$(openssl rand -hex 32)" > "$env_file"
    sed -n 's/^KAWARO_WIN_SIGN_PASS=//p' "$env_file" > "$pass_file"
    [ -s "$pass_file" ] || { echo "sign-local: no KAWARO_WIN_SIGN_PASS in $env_file" >&2; exit 2; }
    if [ ! -f "$key" ]; then
      openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:3072 -aes-256-cbc \
        -pass "file:$pass_file" -out "$key"
      rm -f "$crt"
    fi
    if [ ! -f "$crt" ]; then
      openssl req -x509 -new -key "$key" -passin "file:$pass_file" -sha256 -days 1095 -out "$crt" \
        -subj "/CN=Kawaro test signing (Muhmad Omar Haj Hmdo)" \
        -addext "basicConstraints=critical,CA:FALSE" \
        -addext "keyUsage=critical,digitalSignature" \
        -addext "extendedKeyUsage=codeSigning"
    fi
    cred=(-certs "$crt" -key "$key" -readpass "$pass_file")
    cert_pem="$crt"
    ;;
  pkcs12)
    printf '%s\n' "$KAWARO_WIN_SIGN_PASS" > "$pass_file"
    cred=(-pkcs12 "$p12" -readpass "$pass_file")
    cert_pem="$stage/cert.pem"
    if ! openssl pkcs12 -in "$p12" -passin "file:$pass_file" -clcerts -nokeys -out "$cert_pem" 2>/dev/null; then
      echo "sign-local: cannot read the certificate from $p12 (wrong KAWARO_WIN_SIGN_PASS?)" >&2
      exit 2
    fi
    ;;
esac
umask 022

trust=()
[ -z "${KAWARO_WIN_SIGN_CAFILE:-}" ] || trust+=(-CAfile "$KAWARO_WIN_SIGN_CAFILE")
[ -z "${KAWARO_WIN_SIGN_TSA_CAFILE:-}" ] || trust+=(-TSA-CAfile "$KAWARO_WIN_SIGN_TSA_CAFILE")
if [ "$kind" = test ] && [ -z "${KAWARO_WIN_SIGN_CAFILE:-}" ]; then
  trust+=(-CAfile "$cert_pem")
fi

# verify FILE: passes only with a valid signature and a verified RFC 3161
# timestamp. osslsigncode exits 0 even when only the timestamp chain fails,
# so the timestamp line is checked by name.
verify () {
  local f="$1" out rc
  if out="$(osslsigncode verify "${trust[@]}" -in "$f" 2>&1)"; then rc=0; else rc=$?; fi
  if [ "$rc" -ne 0 ] || ! grep -q 'Timestamp Server Signature verification: ok' <<<"$out"; then
    echo "$out" >&2
    echo "sign-local: verification failed: $f (a valid signature with a verified RFC 3161 timestamp is required)" >&2
    return 1
  fi
}

# sign FILE: sign in place with an RFC 3161 timestamp, then verify.
sign () {
  local f="$1"
  [ -f "$f" ] || { echo "sign-local: $f not found" >&2; exit 2; }
  echo "sign-local: signing $f"
  if ! osslsigncode sign "${cred[@]}" -h sha256 \
      -n "Kawaro" -i "https://kawaro.sbarah.com" -ts "$ts" \
      -in "$f" -out "$f.signed" >/dev/null; then
    rm -f "$f.signed"
    echo "sign-local: signing failed: $f" >&2
    exit 1
  fi
  mv "$f.signed" "$f"
  verify "$f" || exit 1
}

case "$mode" in
  sign)
    for f in "$@"; do
      sign "$f"
    done
    ;;
  verify)
    rc=0
    for f in "$@"; do
      [ -f "$f" ] || { echo "sign-local: $f not found" >&2; exit 2; }
      if verify "$f"; then
        echo "sign-local: verified $f"
      else
        rc=1
      fi
    done
    exit "$rc"
    ;;
  release)
    zip_in="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
    out_dir="${2:-dist-signed}"
    [ -f "$zip_in" ] || { echo "sign-local: $zip_in not found" >&2; exit 2; }
    name="$(basename "$zip_in" .zip)"
    case "$name" in
      kawaro-*-windows-x64) ;;
      *) echo "sign-local: expected kawaro-<version>-windows-x64.zip" >&2; exit 2 ;;
    esac
    version="${name#kawaro-}"
    version="${version%-windows-x64}"
    repo_version="$(awk '/^\[workspace.package\]/{p=1;next} /^\[/{p=0} p && /^version *=/{gsub(/"/,"",$3); print $3; exit}' "$root/Cargo.toml")"
    if [ "$version" != "$repo_version" ]; then
      echo "sign-local: zip is $version but this checkout is $repo_version; check out the release tag" >&2
      exit 2
    fi

    unzip -q "$zip_in" -d "$stage"
    dir="$stage/$name"
    for exe in kawaro.exe kawarod.exe; do
      [ -f "$dir/$exe" ] || { echo "sign-local: $exe missing from the zip" >&2; exit 2; }
      sign "$dir/$exe"
    done
    # The inner SHA256SUMS is rewritten after signing, over the signed bytes.
    (cd "$dir" && LC_ALL=C find . -type f ! -name SHA256SUMS | sed 's|^\./||' | sort | xargs sha256sum > ../sums.tmp)
    mv "$stage/sums.tmp" "$dir/SHA256SUMS"
    find "$dir" -exec touch -h -d '2026-01-01 00:00:00 UTC' {} +

    mkdir -p "$out_dir"
    out_dir="$(cd "$out_dir" && pwd)"
    rm -f "$out_dir/$name.zip" "$out_dir/$name.msi"
    (cd "$stage" && LC_ALL=C find "$name" -type f | sort | TZ=UTC zip -X -D -q -9 "$out_dir/$name.zip" -@)

    rel="$stage/target/x86_64-pc-windows-gnu/release"
    mkdir -p "$rel"
    cp "$dir/kawaro.exe" "$dir/kawarod.exe" "$rel/"
    msi="$(CARGO_TARGET_DIR="$stage/target" "$root/scripts/windows/make-msi.sh" 2>/dev/null | tail -n 1)"
    cp "$msi" "$out_dir/$name.msi"
    sign "$out_dir/$name.msi"

    openssl x509 -in "$cert_pem" -outform der -out "$out_dir/kawaro-test-signing.cer"
    # The published hashes come last: they name the signed, delivered bytes.
    (cd "$out_dir" && sha256sum "$name.zip" "$name.msi" kawaro-test-signing.cer > SHA256SUMS)
    echo "$out_dir"
    ;;
esac
