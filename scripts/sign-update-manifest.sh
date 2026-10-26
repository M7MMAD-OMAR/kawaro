#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Signs an update manifest with minisign and writes MANIFEST.minisig.
#
#   UPDATE_SIGNING_PASSWORD=... scripts/sign-update-manifest.sh KEY_FILE MANIFEST
#
# The passphrase is read from the environment and handed to minisign on its
# standard input, never on a command line. Nothing secret is printed and
# tracing is never enabled. Exit 2 on misuse; a minisign failure leaves no
# .minisig behind.
set -euo pipefail

[ "$#" -eq 2 ] || { echo "usage: $0 KEY_FILE MANIFEST" >&2; exit 2; }
KEY="$1"; MANIFEST="$2"
[ -f "$KEY" ] || { echo "sign-update-manifest: key file not found" >&2; exit 2; }
[ -f "$MANIFEST" ] || { echo "sign-update-manifest: manifest not found: $MANIFEST" >&2; exit 2; }
[ -n "${UPDATE_SIGNING_PASSWORD+x}" ] || { echo "sign-update-manifest: UPDATE_SIGNING_PASSWORD is not set" >&2; exit 2; }

# A here-string, not a pipe: with an unencrypted key minisign never reads its
# standard input, and under pipefail a writer that loses the race with minisign's
# exit would fail the whole pipeline with SIGPIPE. minisign's own error text (it
# never contains the passphrase) is kept, so a CI failure says why.
ERR="$(mktemp)"
trap 'rm -f "$ERR"' EXIT
if ! minisign -S -s "$KEY" -m "$MANIFEST" >/dev/null 2>"$ERR" <<<"$UPDATE_SIGNING_PASSWORD"; then
  rm -f "$MANIFEST.minisig"
  echo "sign-update-manifest: minisign failed: $(head -c 300 "$ERR" | tr '\n' ' ')" >&2
  exit 1
fi
