#!/usr/bin/env bash
# Fails if .slint sources break the SbarHive design rules.
# The generated tokens file is the only place allowed to hold raw values.
set -u
ROOT="${1:-crates}"
SITE_BASE="${1:-.}"
EXCLUDE="tokens.slint"
# The vendored Material library (crates/ui/material/, MIT, see its README and
# docs/adr/0002-settings-components.md) keeps its own raw colours and lengths,
# so only the value checks skip it. Shadows, gradients and dashes are still
# checked there, which proves the shadows were removed.
VENDOR='(^|/)crates/ui/material/'
fail=0
check () {
  local label="$1" pattern="$2" skip_vendor="${3:-}"
  local hits
  hits=$(grep -rnE --include='*.slint' "$pattern" "$ROOT" | grep -v "$EXCLUDE" || true)
  if [ -n "$skip_vendor" ] && [ -n "$hits" ]; then
    hits=$(printf '%s\n' "$hits" | grep -vE "$VENDOR" || true)
  fi
  if [ -n "$hits" ]; then echo "[design-lint] $label:"; echo "$hits"; fail=1; fi
}
check "hard-coded color"     '#[0-9a-fA-F]{3,8}\b' vendor
check "hard-coded length"    '\b[0-9]+(\.[0-9]+)?(px|phx)\b' vendor
check "shadow"               'drop[-_]shadow'
check "gradient"             '@(linear|radial|conic)[-_]gradient'
check "em dash"              $'\xe2\x80\x94'
check "en dash"              $'\xe2\x80\x93'
check "native style"         'style:[[:space:]]*"native"'
# The website: site/style.css and every file under site/css/ (tokens.css is generated
# and is the only place allowed to hold raw values).
css_check () {
  local label="$1" pattern="$2" hits
  hits=$(find "$SITE_BASE/site/css" "$SITE_BASE/site/style.css" -name '*.css' 2>/dev/null \
    | xargs -r grep -nE "$pattern" | grep -v 'tokens.css' || true)
  if [ -n "$hits" ]; then echo "[design-lint] site css $label:"; echo "$hits"; fail=1; fi
}
css_check "hard-coded color"  '#[0-9a-fA-F]{3,8}\b|rgba?\(|hsla?\('
css_check "shadow"            'box-shadow[[:space:]]*:[[:space:]]*[^n]|text-shadow[[:space:]]*:[[:space:]]*[^n]|drop-shadow'
css_check "gradient"          '(linear|radial|conic)-gradient'
css_check "em dash"           $'\xe2\x80\x94'
css_check "en dash"           $'\xe2\x80\x93'
[ $fail -eq 0 ] && echo "[design-lint] clean"
exit $fail
