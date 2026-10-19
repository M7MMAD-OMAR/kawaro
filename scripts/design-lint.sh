#!/usr/bin/env bash
# Fails if .slint sources break the SbarHive design rules.
# The generated tokens file is the only place allowed to hold raw values.
set -u
ROOT="${1:-crates}"
EXCLUDE="tokens.slint"
fail=0
check () {
  local label="$1" pattern="$2"
  local hits
  hits=$(grep -rnE --include='*.slint' "$pattern" "$ROOT" | grep -v "$EXCLUDE" || true)
  if [ -n "$hits" ]; then echo "[design-lint] $label:"; echo "$hits"; fail=1; fi
}
check "hard-coded color"     '#[0-9a-fA-F]{3,8}\b'
check "hard-coded length"    '\b[0-9]+(\.[0-9]+)?(px|phx)\b'
check "shadow"               'drop[-_]shadow'
check "gradient"             '@(linear|radial|conic)[-_]gradient'
check "em dash"              $'\xe2\x80\x94'
check "native style"         'style:[[:space:]]*"native"'
[ $fail -eq 0 ] && echo "[design-lint] clean"
exit $fail
