#!/usr/bin/env bash
# Signs a Windows release with a self-signed test certificate, for testing
# on your own Windows machines only. It is not a public code signing
# certificate: Windows trusts the result only where you import the
# certificate (see scripts/windows/README.md, "Signed test build").
#   scripts/windows/sign-local.sh RELEASE_ZIP [OUT_DIR]
# RELEASE_ZIP is kawaro-<version>-windows-x64.zip from a release. The script
# signs kawaro.exe and kawarod.exe, repacks the zip (its inner SHA256SUMS
# rewritten), builds the MSI from the signed programs with make-msi.sh,
# signs the MSI and writes into OUT_DIR (default ./dist-signed):
#   kawaro-<version>-windows-x64.zip, kawaro-<version>-windows-x64.msi,
#   kawaro-test-signing.cer and SHA256SUMS.
# The key lives in KAWARO_WIN_SIGN_DIR (default
# ~/.config/kawaro-signing/windows), encrypted with the passphrase in
# signing.env there; both are created on the first run. The certificate is
# a leaf (CA:FALSE) with the code signing usage only, so it cannot issue
# other certificates. KAWARO_WIN_SIGN_TS is the RFC 3161 timestamp server
# (default http://timestamp.digicert.com; empty means no timestamp).
# Needs osslsigncode, openssl, wixl, zip and unzip. Exit 2 on a bad input.
set -eu
if [ $# -lt 1 ] || [ $# -gt 2 ]; then
  echo "usage: sign-local.sh RELEASE_ZIP [OUT_DIR]" >&2
  exit 2
fi
zip_in="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
out_dir="${2:-dist-signed}"
root="$(cd "$(dirname "$0")/../.." && pwd)"
for tool in osslsigncode openssl wixl zip unzip; do
  command -v "$tool" >/dev/null || { echo "sign-local: $tool not found" >&2; exit 2; }
done
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

sign_dir="${KAWARO_WIN_SIGN_DIR:-$HOME/.config/kawaro-signing/windows}"
ts="${KAWARO_WIN_SIGN_TS-http://timestamp.digicert.com}"
key="$sign_dir/key.pem"
crt="$sign_dir/cert.pem"
env_file="$sign_dir/signing.env"
umask 077
mkdir -p "$sign_dir"
[ -f "$env_file" ] || printf 'KAWARO_WIN_SIGN_PASS=%s\n' "$(openssl rand -hex 32)" > "$env_file"
# The passphrase goes to the tools through a file, never on a command line.
pass_file="$(mktemp "${TMPDIR:-/tmp}/kawaro-pass.XXXXXX")"
stage="$(mktemp -d "${TMPDIR:-/tmp}/kawaro-sign.XXXXXX")"
trap 'rm -rf -- "$stage" "$pass_file"' EXIT
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
umask 022

sign () {
  ts_args=()
  [ -z "$ts" ] || ts_args=(-ts "$ts")
  osslsigncode sign -certs "$crt" -key "$key" -readpass "$pass_file" -h sha256 \
    -n "Kawaro" -i "https://kawaro.sbarah.com" "${ts_args[@]}" \
    -in "$1" -out "$1.signed" >/dev/null
  mv "$1.signed" "$1"
  osslsigncode verify -CAfile "$crt" -in "$1" >/dev/null
}

unzip -q "$zip_in" -d "$stage"
dir="$stage/$name"
for exe in kawaro.exe kawarod.exe; do
  [ -f "$dir/$exe" ] || { echo "sign-local: $exe missing from the zip" >&2; exit 2; }
  sign "$dir/$exe"
done
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

openssl x509 -in "$crt" -outform der -out "$out_dir/kawaro-test-signing.cer"
(cd "$out_dir" && sha256sum "$name.zip" "$name.msi" kawaro-test-signing.cer > SHA256SUMS)
echo "$out_dir"
