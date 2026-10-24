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

if ! printf '%s\n' "$UPDATE_SIGNING_PASSWORD" | minisign -S -s "$KEY" -m "$MANIFEST" >/dev/null 2>&1; then
  rm -f "$MANIFEST.minisig"
  echo "sign-update-manifest: minisign failed" >&2
  exit 1
fi
