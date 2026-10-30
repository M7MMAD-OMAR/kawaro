#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Offline check of a Scoop manifest against the conventions Scoop's own
# schema and checkver rely on (the upstream schema needs the network).
#   packaging/scoop/validate.sh kawaro.json [SHA256SUMS]
# With SHA256SUMS the 64bit hash must equal the ZIP line. Exit 0 valid, 1 not.
set -euo pipefail
f="$1" sums="${2:-}"
bad() { echo "validate: $*" >&2; exit 1; }
jq -e . "$f" >/dev/null 2>&1 || bad "not JSON"
jq -e '
  (.version | type == "string" and test("^[0-9]+\\.[0-9]+\\.[0-9]+$"))
  and (.description | type == "string" and length > 0)
  and (.homepage | type == "string" and startswith("https://"))
  and (.license | type == "string" and length > 0)
  and (.architecture."64bit".url | type == "string" and startswith("https://") and endswith(".zip"))
  and (.architecture."64bit".hash | type == "string" and test("^[0-9a-f]{64}$"))
  and (.bin | type == "array" and length > 0 and all(type == "string" and endswith(".exe")))
  and (.checkver | . != null)
  and (.autoupdate.architecture."64bit".url | type == "string" and contains("$version"))
  and (.autoupdate.architecture."64bit".hash | . != null)
' "$f" >/dev/null || bad "a required field is missing or malformed"
jq -e '. as $m | .architecture."64bit".url | contains("/v" + $m.version + "/") and contains("-" + $m.version + "-")' "$f" >/dev/null \
  || bad "url does not carry the manifest version"
if [ -n "$sums" ]; then
  want="$(jq -r '.architecture."64bit".url | split("/") | last' "$f")"
  got="$(jq -r '.architecture."64bit".hash' "$f")"
  line="$(awk -v f="$want" '{n=$2; sub(/^\*/, "", n)} n == f {print $1}' "$sums")"
  [ "$line" = "$got" ] || bad "hash differs from the $want line of $sums"
fi
