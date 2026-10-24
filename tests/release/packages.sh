#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# S-053c AC5 and AC6: packaging/linux/build-packages.sh version mapping and
# Recommends: curl. Needs rpmbuild and rpm. dpkg-deb and dpkg are used when
# installed (CI); otherwise a shim records DEBIAN/ and the dpkg only checks
# are reported SKIP.
#
# Contract: the package version is the Cargo.toml `version` with every `-`
# replaced by `~`. File names use the mapped version:
#   sbarhive-VERSION-1.x86_64.rpm and sbarhive_VERSION_amd64.deb.
# The script is run from a copy of the working tree whose Cargo.toml version
# line is rewritten, with copies of /usr/bin/true as the two binaries.
# shellcheck disable=SC2016
set -u
# shellcheck source-path=SCRIPTDIR
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

SPEC="$REPO_ROOT/packaging/linux/sbarhive.spec"
check "AC6 the spec has a Recommends: curl line" \
  test "$(grep -cE '^Recommends:[[:space:]]+curl[[:space:]]*$' "$SPEC")" -eq 1
check "AC6 the spec %post is unchanged" \
  grep -qxF 'systemctl --global enable sbarhive.service >/dev/null 2>&1 || :' "$SPEC"

tree() { # tree DIR VERSION: a copy of the working tree with a given version
  local d="$1"
  mkdir -p "$d"
  (cd "$REPO_ROOT" && git ls-files -z --cached --others --exclude-standard \
    | tar --null -T - --ignore-failed-read -cf - 2>/dev/null) | tar -xf - -C "$d"
  sed -i "0,/^version = \".*\"/s//version = \"$2\"/" "$d/Cargo.toml"
  mkdir -p "$d/bin"
  cp /usr/bin/true "$d/bin/sbarhive"
  cp /usr/bin/true "$d/bin/sbarhived"
}
SHIM="$TMP/shim"
mkdir -p "$SHIM"
REAL_DEB="$(command -v dpkg-deb || true)"
cat > "$SHIM/dpkg-deb" <<'SHIM_EOF'
#!/usr/bin/env bash
# records DEBIAN/ of the tree being built, then builds for real when possible
for a in "$@"; do last_prev="${last:-}"; last="$a"; done
src="$last_prev"; dst="$last"
rm -rf "$dst.debian"; cp -a "$src/DEBIAN" "$dst.debian"
if [ -n "$REAL_DEB" ]; then exec "$REAL_DEB" "$@"; fi
: > "$dst"
SHIM_EOF
chmod +x "$SHIM/dpkg-deb"
export REAL_DEB

build() { # build VERSION -> $TMP/out-VERSION
  local v="$1" t="$TMP/tree-$1"
  tree "$t" "$v"
  PATH="$SHIM:$PATH" "$t/packaging/linux/build-packages.sh" "$t/bin" "$TMP/out-$v" >"$TMP/build-$v.log" 2>&1
  echo $? > "$TMP/rc-$v"
}

PREV="0.4.0-beta.1"; MAPPED="0.4.0~beta.1"
build "$PREV"
check "AC5 build-packages.sh succeeds for $PREV" test "$(cat "$TMP/rc-$PREV")" = 0
RPM="$TMP/out-$PREV/sbarhive-$MAPPED-1.x86_64.rpm"
DEB="$TMP/out-$PREV/sbarhive_${MAPPED}_amd64.deb"
check "AC5 rpm file name uses the mapped version" test -f "$RPM"
check "AC5 deb file name uses the mapped version" test -f "$DEB"
check "AC5 no file name keeps the hyphen version" \
  test "$(find "$TMP/out-$PREV" -mindepth 1 -name '*0.4.0-beta.1*' | wc -l)" -eq 0
check "AC5 rpm VERSION is 0.4.0~beta.1" \
  test "$(rpm -qp --qf '%{VERSION}' "$RPM" 2>/dev/null)" = "$MAPPED"
CTL="$DEB.debian/control"
check "AC5 DEBIAN/control Version is 0.4.0~beta.1" grep -qx "Version: $MAPPED" "$CTL"
if [ -n "$REAL_DEB" ]; then
  check "AC5 dpkg-deb -f Version is 0.4.0~beta.1" test "$(dpkg-deb -f "$DEB" Version)" = "$MAPPED"
else echo "SKIP AC5 dpkg-deb -f Version (no dpkg-deb here; CI runs it)"; fi
if command -v dpkg >/dev/null; then
  expect_status "AC5 dpkg --compare-versions 0.4.0~beta.1 lt 0.4.0" 0 dpkg --compare-versions "$MAPPED" lt 0.4.0
else echo "SKIP AC5 dpkg --compare-versions (no dpkg here; CI runs it)"; fi

# AC6
check "AC6 rpm Recommends curl" bash -c "rpm -qp --recommends '$RPM' 2>/dev/null | grep -qx curl"
check "AC6 DEBIAN/control Recommends has curl as an item" \
  bash -c "grep '^Recommends:' '$CTL' | tr ',' '\n' | sed 's/^Recommends: *//; s/^ *//; s/ *\$//' | grep -qx curl"
check "AC6 DEBIAN/control still recommends ffmpeg and wl-clipboard" \
  grep -Eq '^Recommends:.*ffmpeg.*wl-clipboard|^Recommends:.*wl-clipboard.*ffmpeg' "$CTL"
check "AC6 postinst is unchanged" test "$(cat "$DEB.debian/postinst")" = "$(printf '%s\n' '#!/bin/sh' 'set -e' 'if [ "$1" = configure ]; then' '  systemctl --global enable sbarhive.service >/dev/null 2>&1 || true' 'fi')"
check "AC6 prerm is unchanged" test "$(cat "$DEB.debian/prerm")" = "$(printf '%s\n' '#!/bin/sh' 'set -e' 'if [ "$1" = remove ]; then' '  systemctl --global disable sbarhive.service >/dev/null 2>&1 || true' 'fi')"
check "AC6 rpm %post is unchanged" \
  bash -c "rpm -qp --scripts '$RPM' 2>/dev/null | grep -qxF 'systemctl --global enable sbarhive.service >/dev/null 2>&1 || :'"

# a plain version keeps its file names
build 0.4.0
check "AC5 build-packages.sh succeeds for 0.4.0" test "$(cat "$TMP/rc-0.4.0")" = 0
check "AC5 0.4.0 rpm name unchanged" test -f "$TMP/out-0.4.0/sbarhive-0.4.0-1.x86_64.rpm"
check "AC5 0.4.0 deb name unchanged" test -f "$TMP/out-0.4.0/sbarhive_0.4.0_amd64.deb"
check "AC5 0.4.0 rpm VERSION is 0.4.0" \
  test "$(rpm -qp --qf '%{VERSION}' "$TMP/out-0.4.0/sbarhive-0.4.0-1.x86_64.rpm" 2>/dev/null)" = 0.4.0
check "AC5 0.4.0 has no tilde" test "$(find "$TMP/out-0.4.0" -name '*~*' | wc -l)" -eq 0

finish
