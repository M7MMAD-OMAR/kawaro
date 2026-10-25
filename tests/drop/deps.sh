#!/usr/bin/env bash
# Conditions are passed to check() in single quotes and expanded by eval.
# shellcheck disable=SC2016,SC2034
# S-014e1 AC14 (backlog/S-014-drop-hive.md): the portal client's
# dependencies. Static, no compositor; cargo runs --offline on the lock file.
#
# The daemon reaches the portal through one of the two clients AC14 allows:
#   ashpd   a direct dependency of crates/daemon with a full version
#           (x.y.z, optionally "="), default-features = false and exactly
#           the features "global_shortcuts" and "async-io" (the names in
#           ashpd 0.13.13's Cargo.toml, [features]); one ashpd in
#           Cargo.lock, at that version;
#   proxy   no ashpd anywhere, zbus a direct dependency of crates/daemon
#           and a hand written #[zbus::proxy] for
#           org.freedesktop.portal.GlobalShortcuts in crates/daemon/src
#           (manager answer 3).
# Then, for either client:
#   - no second zbus: `cargo tree -d` lists no zbus among its duplicated
#     packages (its lines at column 0), and Cargo.lock has exactly one
#     package named zbus, at 5.19.0 or at a version docs/engines.md records
#     for zbus (a single repository wide upgrade);
#   - no tokio in the daemon: `cargo tree -p kawaro-daemon -e normal -i
#     tokio` prints no tree (cargo says "nothing to print", or tokio is not
#     in Cargo.lock at all);
#   - every package in Cargo.lock that is not in the lock of 0de3c74 (the
#     approval of S-014e) has a row in docs/engines.md naming it, its
#     version and a licence; the workspace members (kawaro-*) are not
#     dependencies and are left out.
# The checks after the first are only meaningful with a client, so each one
# requires it.
# Usage: tests/drop/deps.sh
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
BASE=0de3c74
FAILS=0
check() { # check NAME CONDITION
  if eval "$2"; then echo "PASS $1"; else echo "FAIL $1"; FAILS=$((FAILS + 1)); fi
}
cd "$ROOT" || exit 2

# The daemon's direct dependencies from its Cargo.toml.
read -r CLIENT AVER ADEF AFEAT ZDIRECT < <(python3 -I - crates/daemon/Cargo.toml <<'PY'
import sys, tomllib
d = tomllib.load(open(sys.argv[1], "rb"))
deps = {}
for k in ("dependencies",):
    deps.update(d.get(k, {}))
for t in d.get("target", {}).values():
    deps.update(t.get("dependencies", {}))
a = deps.get("ashpd")
z = "zbus" in deps
if a is None:
    print("none" if not z else "proxy", "-", "-", "-", "yes" if z else "no")
else:
    if isinstance(a, str):
        a = {"version": a}
    if a.get("workspace"):
        # ashpd.workspace = true: version and default-features from the root.
        w = tomllib.load(open("Cargo.toml", "rb")).get("workspace", {}).get("dependencies", {}).get("ashpd", {})
        if isinstance(w, str):
            w = {"version": w}
        a = {**w, "features": sorted(set(w.get("features", [])) | set(a.get("features", [])))}
    feats = ",".join(sorted(a.get("features", []))) or "-"
    print("ashpd", a.get("version", "-"), str(a.get("default-features", True)).lower(), feats, "yes" if z else "no")
PY
)
PROXY=$(grep -rlE '#\[(zbus::)?proxy\(' crates/daemon/src 2>/dev/null | xargs -r grep -l 'org.freedesktop.portal.GlobalShortcuts' | tr '\n' ' ')
LOCK=$(python3 -I - Cargo.lock <<'PY'
import sys, tomllib
p = tomllib.load(open(sys.argv[1], "rb")).get("package", [])
for n in ("ashpd", "zbus", "tokio"):
    print(n, " ".join(x["version"] for x in p if x["name"] == n) or "-")
PY
)
LOCK_ASHPD=$(awk '$1 == "ashpd" { $1 = ""; print substr($0, 2) }' <<<"$LOCK")
LOCK_ZBUS=$(awk '$1 == "zbus" { $1 = ""; print substr($0, 2) }' <<<"$LOCK")
LOCK_TOKIO=$(awk '$1 == "tokio" { $1 = ""; print substr($0, 2) }' <<<"$LOCK")
echo "INFO AC14: crates/daemon: client $CLIENT, ashpd version $AVER default-features $ADEF features $AFEAT, zbus direct $ZDIRECT; zbus::proxy files for GlobalShortcuts: ${PROXY:-none}; Cargo.lock: ashpd $LOCK_ASHPD, zbus $LOCK_ZBUS, tokio $LOCK_TOKIO"

have=no
case "$CLIENT" in
  ashpd)
    [[ "$AVER" =~ ^=?[0-9]+\.[0-9]+\.[0-9]+$ ]] && [ "$ADEF" = false ] && [ "$AFEAT" = "async-io,global_shortcuts" ] &&
      [ "$LOCK_ASHPD" = "${AVER#=}" ] && have=yes ;;
  proxy)
    [ "$LOCK_ASHPD" = - ] && [ -n "$PROXY" ] && have=yes ;;
esac
check "AC14: crates/daemon has a portal client: ashpd at a full version with default-features = false and only the features global_shortcuts and async-io (one ashpd in Cargo.lock at that version), or zbus directly with a #[zbus::proxy] for org.freedesktop.portal.GlobalShortcuts and no ashpd" \
  '[ "$have" = yes ]'

TREE_D=$(nice -n 19 cargo tree --offline -e normal -d 2>&1); TREE_D_RC=$?
ZDUP=$(grep -cE '^zbus v' <<<"$TREE_D")
echo "INFO AC14: cargo tree -d: exit $TREE_D_RC, duplicated packages: $(grep -oE '^[a-z][a-z0-9_-]* ' <<<"$TREE_D" | sort -u | tr -d '\n')"
ZREC=no
if [ "$LOCK_ZBUS" = 5.19.0 ]; then ZREC=yes
elif [[ "$LOCK_ZBUS" =~ ^[0-9.]+$ ]] && grep -qE "^\| zbus \| ${LOCK_ZBUS//./\\.}[ ,]" docs/engines.md; then ZREC=yes; fi
check "AC14: with the client, no second zbus: cargo tree -d lists no zbus, and Cargo.lock has one zbus, at 5.19.0 or at the version docs/engines.md records" \
  '[ "$have" = yes ] && [ "$TREE_D_RC" -eq 0 ] && [ "$ZDUP" -eq 0 ] && [ "$ZREC" = yes ]'

TREE_T=$(nice -n 19 cargo tree --offline -p kawaro-daemon -e normal -i tokio 2>&1); TREE_T_RC=$?
TOKIO_OK=no
if [ "$TREE_T_RC" -eq 0 ] && grep -q 'nothing to print' <<<"$TREE_T" && ! grep -qE '^tokio v' <<<"$TREE_T"; then TOKIO_OK=yes
elif [ "$LOCK_TOKIO" = - ] && grep -q 'did not match any packages' <<<"$TREE_T"; then TOKIO_OK=yes; fi
echo "INFO AC14: cargo tree -p kawaro-daemon -e normal -i tokio: exit $TREE_T_RC, $(head -1 <<<"$TREE_T")"
check "AC14: with the client, the daemon's normal dependencies have no tokio" '[ "$have" = yes ] && [ "$TOKIO_OK" = yes ]'

NEW=$(python3 -I - <(git show "$BASE:Cargo.lock") Cargo.lock <<'PY'
import sys, tomllib
def pk(f):
    # Workspace members (kawaro-*) are not dependencies; a version bump
    # at release time (0.3.3) made them look new. The vendored softbuffer
    # has no "source" either but is a dependency, so filter by name.
    return {(p["name"], p["version"]) for p in tomllib.load(open(f, "rb")).get("package", []) if not p["name"].startswith("kawaro-")}
for n, v in sorted(pk(sys.argv[2]) - pk(sys.argv[1])):
    print(n, v)
PY
)
MISSING=""
while read -r n v; do
  [ -z "$n" ] && continue
  # A table row: | name | version... | licence | ...
  awk -F'|' -v n="$n" -v v="$v" '
    { a = $2; b = $3; c = $4; gsub(/^ +| +$/, "", a); gsub(/^ +| +$/, "", b); gsub(/^ +| +$/, "", c) }
    a == n && index(b, v) == 1 && c != "" { found = 1 } END { exit !found }' docs/engines.md || MISSING+="$n $v, "
done <<<"$NEW"
echo "INFO AC14: packages new since $BASE: $(tr '\n' ' ' <<<"${NEW:-none}"); not in docs/engines.md: ${MISSING:-none}"
check "AC14: with the client, every package new in Cargo.lock (ashpd among them, if it is the client) has a docs/engines.md row with its version and licence" \
  '[ "$have" = yes ] && [ -z "$MISSING" ] && { [ "$CLIENT" != ashpd ] || grep -q "^ashpd " <<<"$NEW"; }'

if [ "$FAILS" -eq 0 ]; then echo "RESULT PASS"; else echo "RESULT FAIL ($FAILS)"; fi
[ "$FAILS" -eq 0 ]
