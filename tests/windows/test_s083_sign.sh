#!/usr/bin/env bash
# S-083, heavy test: the signing dry run with a throwaway certificate,
# run inside registry.fedoraproject.org/fedora:44 because osslsigncode is not
# on the host. The real work and the checks are in
# tests/windows/s083_dryrun_inner.sh (this script only prepares the fixture
# executables and the container). Needs podman or docker and python3. Run it
# inside the memory scope:
#   systemd-run --user --scope -p MemoryMax=6G -p CPUQuota=200% \
#     bash tests/windows/test_s083_sign.sh
# The dry run proves, with a throwaway self-signed certificate and a local
# RFC 3161 timestamp authority (no network beyond 127.0.0.1):
#   - sign-local.sh CERT_SOURCE --sign signs and every signature carries a
#     verified RFC 3161 timestamp (osslsigncode verify reports
#     "Timestamp Server Signature verification: ok"),
#   - the pipeline order of packages.yml on real inputs: sign the exes, then
#     make-zip.sh (its inner SHA256SUMS hashes signed bytes), then make-msi.sh
#     (the signed exes land in the MSI), then sign the MSI, and SHA256SUMS is
#     generated only after signing,
#   - --verify passes on every signed artifact and fails on an unsigned one,
#   - a missing certificate exits 2.
# The fixtures are minimal PE32+ executables (one section, 1024 bytes) built
# byte by byte: osslsigncode signs them like any mingw output, so no binary
# fixture is committed.
# shellcheck disable=SC2016
set -u
# shellcheck source=tests/windows/lib.sh
. "$(dirname "$0")/lib.sh"
cd "$REPO_ROOT" || exit 2

RT=""
for c in podman docker; do
  if command -v "$c" >/dev/null; then RT="$c"; break; fi
done
if [ -z "$RT" ]; then
  echo "SKIP S-083 signing dry run: no podman and no docker here"
  finish "S-083 signing dry run"
  exit $?
fi
if ! command -v python3 >/dev/null; then
  echo "SKIP S-083 signing dry run: python3 not installed here"
  finish "S-083 signing dry run"
  exit $?
fi

T="$(mktemp -d "${TMPDIR:-/tmp}/kawaro-s083.XXXXXX")"
trap 'rm -rf -- "$T"' EXIT

# Minimal PE32+: MZ header, PE signature, COFF header, PE32+ optional header
# with sixteen zero data directories, one .text section. Valid enough for
# osslsigncode to sign and verify (only the PE checksum stays zero).
python3 - "$T" <<'PY'
import struct, sys
def tiny_pe(path, chars):
    dos = bytearray(0x40)
    dos[0:2] = b'MZ'
    struct.pack_into('<I', dos, 0x3C, 0x40)
    coff = struct.pack('<HHIIIHH', 0x8664, 1, 0, 0, 0, 0xF0, chars)
    opt = bytearray(0xF0)
    struct.pack_into('<H', opt, 0, 0x20B)
    opt[2] = 14
    struct.pack_into('<I', opt, 4, 0x200)
    struct.pack_into('<I', opt, 16, 0x1000)
    struct.pack_into('<I', opt, 20, 0x1000)
    struct.pack_into('<Q', opt, 24, 0x140000000)
    struct.pack_into('<I', opt, 32, 0x1000)
    struct.pack_into('<I', opt, 36, 0x200)
    struct.pack_into('<H', opt, 40, 6)
    struct.pack_into('<H', opt, 48, 6)
    struct.pack_into('<I', opt, 56, 0x2000)
    struct.pack_into('<I', opt, 60, 0x200)
    struct.pack_into('<H', opt, 68, 3)
    struct.pack_into('<I', opt, 108, 16)
    sec = struct.pack('<8sIIIIIIHHI', b'.text\0\0\0', 0x10, 0x1000, 0x200,
                      0x200, 0, 0, 0, 0, 0x60000020)
    header = bytes(dos) + b'PE\0\0' + coff + bytes(opt) + sec
    open(path, 'wb').write(header.ljust(0x200, b'\0') + bytes([0xC3]).ljust(0x200, b'\0'))
out = sys.argv[1]
tiny_pe(out + '/kawaro.exe', 0x0022)
tiny_pe(out + '/kawarod.exe', 0x0002)
PY

check "fixtures: both tiny PE executables exist" test -s "$T/kawaro.exe" -a -s "$T/kawarod.exe"

ARGS=(run --rm)
[ "$RT" = podman ] && ARGS+=(--security-opt label=disable)
timeout 600 "$RT" "${ARGS[@]}" -v "$T:/t" -v "$REPO_ROOT:/w:ro" -w /t \
  registry.fedoraproject.org/fedora:44 bash /w/tests/windows/s083_dryrun_inner.sh
RC=$?
check "the dry run inside the container exits 0" test "$RC" -eq 0

finish "S-083 signing dry run"
