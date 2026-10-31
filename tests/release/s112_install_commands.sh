#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-112 AC5, AC6, AC8 (part): one source of install commands. Static, offline.
# Contract the script must keep: it finds the repository root two levels above
# itself (scripts/release/install-commands.py), reads packaging/install-commands.json
# and the doc files below that root, `render NAME` prints one command per line,
# `check` exits 0 or 1. The mutation checks run a COPY of the script inside a
# throwaway tree, so the real files are never touched.
# shellcheck disable=SC2016
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

R="$REPO_ROOT"
SCRIPT="$R/scripts/release/install-commands.py"
JSON="$R/packaging/install-commands.json"
DOCS=(README.md README.ar.md docs/distribution.md docs/distribution.ar.md
  docs/distribution-channels/apt.md docs/distribution-channels/copr.md docs/distribution-channels/aur.md
  docs/guide/install.md docs/guide/install.ar.md)
NOT_EN='Not usable yet'
NOT_AR='غير قابل للاستعمال بعد'

check "S-112 the test is part of run_all" grep -qw s112_install_commands "$R/tests/release/run_all.sh"
check "AC5 the script exists" test -f "$SCRIPT"
check "AC5 the JSON exists" test -f "$JSON"

# AC5: the JSON content.
check "AC5 schema 1 and the four channels with statuses" python3 -I - "$JSON" <<'PY'
import json, sys
d = json.load(open(sys.argv[1], encoding="utf-8"))
assert d["schema"] == 1
ch = d["channels"]
assert sorted(ch) == ["apt", "aur", "copr", "winget"], sorted(ch)
want = {"apt": "live", "winget": "submitted", "copr": "not_live", "aur": "not_live"}
for k, v in ch.items():
    assert v["status"] == want[k], (k, v["status"])
    assert isinstance(v["commands"], list) and v["commands"] and all(isinstance(c, str) and c for c in v["commands"]), k
assert ch["aur"]["commands"] == ["yay -S kawaro-bin"]
assert ch["winget"]["commands"] == ["winget install Sbarah.Kawaro"]
assert any("ACCOUNT/kawaro" in c for c in ch["copr"]["commands"])
assert not any("OWNER" in c for c in ch["copr"]["commands"])
PY

# AC5: render.
APT_LINE="$(grep -E '^sudo install -d -m 0755 /etc/apt/keyrings' "$R/docs/distribution-channels/apt.md" | head -n 1)"
check "AC5 apt.md has the one apt line" test -n "$APT_LINE"
got="$(python3 -I "$SCRIPT" render apt 2>/dev/null)"
check "AC5 render apt equals the single line in apt.md" test "$got" = "$APT_LINE"
check "AC5 render aur prints yay -S kawaro-bin" test "$(python3 -I "$SCRIPT" render aur 2>/dev/null)" = "yay -S kawaro-bin"
expect_status "AC5 render of an unknown channel exits 1" 1 python3 -I "$SCRIPT" render nosuchchannel

# AC6: check on the real docs and mutations on a copy of the tree.
expect_status "AC6 check passes on the real docs" 0 python3 -I "$SCRIPT" check

mktree() { # mktree DIR : a copy of the script, the JSON and the docs
  local d="$1" f
  mkdir -p "$d/scripts/release" "$d/packaging"
  cp "$SCRIPT" "$d/scripts/release/" 2>/dev/null
  cp "$JSON" "$d/packaging/" 2>/dev/null
  for f in "${DOCS[@]}"; do mkdir -p "$d/$(dirname "$f")"; cp "$R/$f" "$d/$f" 2>/dev/null; done
}
runchk() { python3 -I "$1/scripts/release/install-commands.py" check; }
# MUT: a mutated tree fails (exit 1) while the untouched tree T1 passes (exit 0).
mutfail() {
  python3 -I "$T1/scripts/release/install-commands.py" check >/dev/null 2>&1 || return 1
  python3 -I "$1/scripts/release/install-commands.py" check >/dev/null 2>&1
  [ $? -eq 1 ]
}

T1="$TMP/t1"; mktree "$T1"
expect_status "AC6 check passes on an untouched copy" 0 runchk "$T1"

T2="$TMP/t2"; mktree "$T2"
sed -i 's/sudo apt install kawaro/sudo apt install kawarX/' "$T2/packaging/install-commands.json" 2>/dev/null
check "AC6 one changed character in the JSON apt command fails" mutfail "$T2"

T3="$TMP/t3"; mktree "$T3"
sed -i 's/curl -fsSL/curl -fsSX/' "$T3/docs/distribution-channels/apt.md" 2>/dev/null
check "AC6 one changed character in the apt doc fails" mutfail "$T3"

T4="$TMP/t4"; mktree "$T4"
python3 -I - "$T4/packaging/install-commands.json" <<'PY' 2>/dev/null
import json, sys
p = sys.argv[1]
d = json.load(open(p, encoding="utf-8"))
d["channels"]["aur"]["status"] = "live"
json.dump(d, open(p, "w", encoding="utf-8"))
PY
check "AC6 aur flipped to live while the page says Not usable yet fails" mutfail "$T4"

T5="$TMP/t5"; mktree "$T5"
python3 -I - "$T5/packaging/install-commands.json" <<'PY' 2>/dev/null
import json, sys
p = sys.argv[1]
d = json.load(open(p, encoding="utf-8"))
d["channels"]["apt"]["status"] = "not_live"
json.dump(d, open(p, "w", encoding="utf-8"))
PY
check "AC6 apt flipped to not_live while the page says Live fails" mutfail "$T5"

T6="$TMP/t6"; mktree "$T6"
printf '\nyay -S evil-bin\n' >> "$T6/README.md" 2>/dev/null
check "AC6 a README line with a foreign channel command fails" mutfail "$T6"

T7="$TMP/t7"; mktree "$T7"
printf '\nwinget install Sbarah.Kawaro\n' >> "$T7/README.md" 2>/dev/null
expect_status "AC6 a README line with the exact JSON command passes" 0 runchk "$T7"

T8="$TMP/t8"; mktree "$T8"
sed -i "s/$NOT_AR/متاح/" "$T8/docs/distribution.ar.md" 2>/dev/null
check "AC6 a missing Arabic Not usable yet phrase fails" mutfail "$T8"

# AC6: the pages themselves.
D="$R/docs/distribution-channels"
check "AC6 distribution.md says Not usable yet" grep -qF "$NOT_EN" "$R/docs/distribution.md"
check "AC6 distribution.ar.md says the Arabic phrase" grep -qF "$NOT_AR" "$R/docs/distribution.ar.md"
check "AC6 the apt row of distribution.md does not say Not usable yet" \
  bash -c '! grep -F "apt (Debian" "$1" | grep -qF "$2"' _ "$R/docs/distribution.md" "$NOT_EN"
check "AC6 aur.md code block holds the command alone" \
  bash -c 'awk "/^\`\`\`/{f=!f; next} f" "$1" | grep -qx "yay -S kawaro-bin" && ! awk "/^\`\`\`/{f=!f; next} f" "$1" | grep -qv "^yay -S kawaro-bin\$"' _ "$D/aur.md"
check "AC6 copr.md uses ACCOUNT, not OWNER, for the user command" \
  bash -c 'grep -qF "sudo dnf copr enable ACCOUNT/kawaro" "$1" && ! grep -qF "dnf copr enable OWNER" "$1"' _ "$D/copr.md"
check "AC6 the pages no longer say the JSON does not exist" \
  bash -c '! grep -qE "does not exist yet|once S-016 adds" "$1" "$2" "$3"' _ "$R/docs/distribution.md" "$R/docs/distribution.ar.md" "$D/copr.md"

# AC8 part.
for f in "$SCRIPT" "$JSON" "$0" "$R/docs/distribution.md" "$R/docs/distribution.ar.md" "$D/apt.md" "$D/copr.md" "$D/aur.md"; do
  check "AC8 no long dash in $(basename "$f")" bash -c '! grep -qP "\x{2013}|\x{2014}" "$1"' _ "$f"
done
check "AC8 install-commands.py parses" python3 -I -c 'import ast,sys;ast.parse(open(sys.argv[1],encoding="utf-8").read())' "$SCRIPT"
check "AC8 this test passes bash -n" bash -n "$0"
if command -v shellcheck >/dev/null 2>&1; then
  check "AC8 this test passes shellcheck" shellcheck -S warning -x "$0"
else
  echo "SKIP S-112 shellcheck is not installed"
fi
finish
