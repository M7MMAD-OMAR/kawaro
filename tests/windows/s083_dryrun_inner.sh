#!/usr/bin/env bash
# S-083 signing dry run: the body of tests/windows/test_s083_sign.sh, run
# inside registry.fedoraproject.org/fedora:44 (osslsigncode, wixl, zip and
# openssl come from the image's repositories). /w is the repository (read
# only), /t the work dir: the fixture executables arrive in /t, everything
# else is created here. Throwaway certificate and a local RFC 3161 timestamp
# authority (openssl ts -reply behind a tiny HTTP server on 127.0.0.1), so
# nothing leaves the container and no real certificate is read or named.
# Checks reuse tests/integrations/lib.sh (check, assert_eq, pass, fail,
# finish). Exit 0 only when every check passes.
# shellcheck disable=SC2016,SC2012
set -u
dnf install -y -q osslsigncode msitools zip unzip openssl python3 >/dev/null
. /w/tests/integrations/lib.sh
export LC_ALL=C.UTF-8
W=/t/work
SIGN=/w/scripts/windows/sign-local.sh
TARGET=/t/target
TARGET2=/t/target2
mkdir -p "$W"
cd "$W" || exit 2

# want_rc DESC WANT COMMAND...: runs COMMAND with its output in $W/last.log.
want_rc () {
  local d="$1" want="$2" got
  shift 2
  "$@" > "$W/last.log" 2>&1
  got=$?
  if [ "$got" = "$want" ]; then
    pass
  else
    fail "$d (exit $got, want $want)"
    sed 's/^/    /' "$W/last.log" | tail -5 >&2
  fi
}

echo "=== setup: throwaway certificate, local RFC 3161 timestamp authority"
umask 077
export KAWARO_WIN_SIGN_DIR="$W/sign"
export KAWARO_WIN_SIGN_TS=http://127.0.0.1:32080
export KAWARO_WIN_SIGN_TSA_CAFILE="$W/tsa.pem"
# The signing leaf is created on demand by sign-local.sh (CERT_SOURCE test).
# The timestamp authority is a throwaway leaf with the timeStamping usage.
openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out "$W/tsa.key" 2>/dev/null
openssl req -x509 -new -key "$W/tsa.key" -sha256 -days 365 -out "$W/tsa.pem" \
  -subj "/CN=Kawaro test timestamp authority" \
  -addext "basicConstraints=critical,CA:FALSE" \
  -addext "keyUsage=critical,digitalSignature" \
  -addext "extendedKeyUsage=critical,timeStamping" 2>/dev/null
check "setup: the timestamp authority certificate exists" test -s "$W/tsa.pem"
# fedora's openssl.cnf points `openssl ts -reply` at /etc/pki/CA/cacert.pem;
# give it the timestamp authority certificate (container local, not the repo).
mkdir -p /etc/pki/CA
cp "$W/tsa.pem" /etc/pki/CA/cacert.pem
cat > "$W/tsa_server.py" <<'PY'
import http.server, subprocess, sys
PORT = int(sys.argv[1])
WORK = sys.argv[2]

class H(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        n = int(self.headers.get("Content-Length", 0))
        with open(WORK + "/q.tsq", "wb") as f:
            f.write(self.rfile.read(n))
        r = subprocess.run(
            ["openssl", "ts", "-reply", "-queryfile", WORK + "/q.tsq",
             "-inkey", WORK + "/tsa.key", "-signer", WORK + "/tsa.pem",
             "-out", WORK + "/r.tsr"],
            capture_output=True)
        if r.returncode != 0:
            self.send_response(400)
            self.end_headers()
            self.wfile.write(r.stderr)
            return
        with open(WORK + "/r.tsr", "rb") as f:
            data = f.read()
        self.send_response(200)
        self.send_header("Content-Type", "application/timestamp-reply")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def log_message(self, *a):
        pass


http.server.HTTPServer(("127.0.0.1", PORT), H).serve_forever()
PY
python3 "$W/tsa_server.py" 32080 "$W" >/dev/null 2>&1 &
TSA_PID=$!
trap 'kill "$TSA_PID" 2>/dev/null || true' EXIT
sleep 1

echo "=== phase 1: a missing certificate fails"
echo x > "$W/ok.pfx"
: > "$W/empty.pfx"
cp /t/kawaro.exe "$W/p1.exe"
want_rc "missing pkcs12 certificate exits 2" 2 env KAWARO_WIN_SIGN_PASS=x \
  "$SIGN" "pkcs12:$W/missing.pfx" --sign "$W/p1.exe"
want_rc "empty pkcs12 certificate exits 2" 2 env KAWARO_WIN_SIGN_PASS=x \
  "$SIGN" "pkcs12:$W/empty.pfx" --sign "$W/p1.exe"
want_rc "pkcs12 without KAWARO_WIN_SIGN_PASS exits 2" 2 env -u KAWARO_WIN_SIGN_PASS \
  "$SIGN" "pkcs12:$W/ok.pfx" --sign "$W/p1.exe"
untouched () { # untouched FILE: it still hashes like the unsigned fixture
  test "$(sha256sum < /t/kawaro.exe)" = "$(sha256sum < "$1")"
}
check "a refusal leaves the file untouched" untouched "$W/p1.exe"

echo "=== phase 2: every signature is timestamped, so no timestamp server is refused"
want_rc "empty KAWARO_WIN_SIGN_TS exits 2" 2 env KAWARO_WIN_SIGN_TS= \
  "$SIGN" test --sign "$W/p1.exe"
check "the timestamp refusal leaves the file untouched" untouched "$W/p1.exe"

echo "=== phase 3: sign the executables (before any packing)"
mkdir -p "$TARGET/x86_64-pc-windows-gnu/release"
cp /t/kawaro.exe /t/kawarod.exe "$TARGET/x86_64-pc-windows-gnu/release/"
REL="$TARGET/x86_64-pc-windows-gnu/release"
want_rc "sign the executables" 0 "$SIGN" test --sign "$REL/kawaro.exe" "$REL/kawarod.exe"
want_rc "verify the signed executables" 0 "$SIGN" test --verify "$REL/kawaro.exe" "$REL/kawarod.exe"
cp /t/kawaro.exe "$W/unsigned.exe"
want_rc "verify refuses an unsigned file" 1 "$SIGN" test --verify "$W/unsigned.exe"
for f in "$REL/kawaro.exe" "$REL/kawarod.exe"; do
  osslsigncode verify -CAfile "$KAWARO_WIN_SIGN_DIR/cert.pem" -TSA-CAfile "$W/tsa.pem" \
    -in "$f" > "$W/direct.log" 2>&1
  rc=$?
  check "osslsigncode verify accepts $f" test "$rc" -eq 0
  grep -q 'Timestamp Server Signature verification: ok' "$W/direct.log"
  rc=$?
  check "$f carries a verified RFC 3161 timestamp" test "$rc" -eq 0
done

echo "=== phase 4: pack with make-zip.sh (its inner SHA256SUMS hashes signed bytes)"
want_rc "make-zip.sh packs the signed executables" 0 env CI=true KAWARO_WIN_SCOPE=1 \
  CARGO_TARGET_DIR="$TARGET" bash /w/scripts/windows/make-zip.sh
ZIP="$(ls "$TARGET"/dist/kawaro-*-windows-x64.zip | head -1)"
check "make-zip.sh wrote a zip" test -s "$ZIP"
rm -rf "$W/zip-x"
mkdir -p "$W/zip-x"
unzip -q "$ZIP" -d "$W/zip-x"
DIR="$(find "$W/zip-x" -mindepth 1 -maxdepth 1 -type d | head -1)"
want_rc "the zip's inner SHA256SUMS verifies (signed bytes)" 0 \
  bash -c 'cd "$1" && sha256sum -c SHA256SUMS' _ "$DIR"
want_rc "the zip carries signed executables" 0 "$SIGN" test --verify \
  "$DIR/kawaro.exe" "$DIR/kawarod.exe"
want_rc "osslsigncode cannot sign a zip container (no archive signature)" 1 \
  "$SIGN" test --sign "$ZIP"

echo "=== phase 5: pack with make-msi.sh (the signed executables are embedded)"
want_rc "make-msi.sh embeds the signed executables" 0 env CI=true KAWARO_WIN_SCOPE=1 \
  CARGO_TARGET_DIR="$TARGET" bash /w/scripts/windows/make-msi.sh
MSI="$(ls "$TARGET"/dist/kawaro-*-windows-x64.msi | head -1)"
check "make-msi.sh wrote an msi" test -s "$MSI"
rm -rf "$W/msi-x"
mkdir -p "$W/msi-x"
(cd "$W/msi-x" && msiextract "$MSI" >/dev/null 2>&1)
want_rc "the msi carries the signed executables" 0 "$SIGN" test --verify \
  "$(find "$W/msi-x" -name kawaro.exe | head -1)" "$(find "$W/msi-x" -name kawarod.exe | head -1)"

echo "=== phase 6: sign the msi and verify every signed artifact"
want_rc "sign the msi" 0 "$SIGN" test --sign "$MSI"
want_rc "verify the signed msi" 0 "$SIGN" test --verify "$MSI"
osslsigncode verify -CAfile "$KAWARO_WIN_SIGN_DIR/cert.pem" -TSA-CAfile "$W/tsa.pem" \
  -in "$MSI" > "$W/direct.log" 2>&1
rc=$?
check "osslsigncode verify accepts the msi" test "$rc" -eq 0
grep -q 'Timestamp Server Signature verification: ok' "$W/direct.log"
rc=$?
check "the msi carries a verified RFC 3161 timestamp" test "$rc" -eq 0
want_rc "verify every signed artifact at once" 0 "$SIGN" test --verify \
  "$REL/kawaro.exe" "$REL/kawarod.exe" "$DIR/kawaro.exe" "$DIR/kawarod.exe" "$MSI"

echo "=== phase 7: SHA256SUMS is generated after signing (release mode)"
mkdir -p "$TARGET2/x86_64-pc-windows-gnu/release"
cp /t/kawaro.exe /t/kawarod.exe "$TARGET2/x86_64-pc-windows-gnu/release/"
OUTZIP="$(env CI=true KAWARO_WIN_SCOPE=1 CARGO_TARGET_DIR="$TARGET2" \
  bash /w/scripts/windows/make-zip.sh | tail -n 1)"
check "the fresh release zip exists" test -s "$OUTZIP"
rm -rf "$W/fresh-x"
mkdir -p "$W/fresh-x"
unzip -q "$OUTZIP" -d "$W/fresh-x"
FDIR="$(find "$W/fresh-x" -mindepth 1 -maxdepth 1 -type d | head -1)"
want_rc "verify refuses the unsigned fresh release" 1 "$SIGN" test --verify \
  "$FDIR/kawaro.exe" "$FDIR/kawarod.exe"
want_rc "sign-local.sh signs the release zip end to end" 0 "$SIGN" test "$OUTZIP" "$W/out"
for f in SHA256SUMS kawaro-test-signing.cer; do
  check "the output holds $f" test -s "$W/out/$f"
done
OUTZIP2="$(ls "$W"/out/kawaro-*-windows-x64.zip | head -1)"
OUTMSI="$(ls "$W"/out/kawaro-*-windows-x64.msi | head -1)"
check "the output holds a zip and an msi" test -s "$OUTZIP2" -a -s "$OUTMSI"
want_rc "SHA256SUMS matches the delivered signed files" 0 \
  bash -c 'cd "$1" && sha256sum -c SHA256SUMS' _ "$W/out"
rm -rf "$W/out-x"
mkdir -p "$W/out-x"
unzip -q "$OUTZIP2" -d "$W/out-x"
ODIR="$(find "$W/out-x" -mindepth 1 -maxdepth 1 -type d | head -1)"
want_rc "the repacked zip's inner SHA256SUMS verifies" 0 \
  bash -c 'cd "$1" && sha256sum -c SHA256SUMS' _ "$ODIR"
want_rc "the repacked zip carries signed executables" 0 "$SIGN" test --verify \
  "$ODIR/kawaro.exe" "$ODIR/kawarod.exe"
want_rc "the signed msi verifies" 0 "$SIGN" test --verify "$OUTMSI"

finish "S-083 signing dry run"
