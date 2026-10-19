#!/usr/bin/env bash
# Shared helpers for the integration tests. Sourced, never executed.
# Every test works inside a throwaway sandbox from make_sandbox; nothing here
# may read or write the real HOME, and the real systemctl and hyprctl are
# shadowed by fakes that only log their arguments.

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export REPO_ROOT
FAILS=0
CHECKS=0

pass () { CHECKS=$((CHECKS + 1)); }
fail () { CHECKS=$((CHECKS + 1)); FAILS=$((FAILS + 1)); echo "  FAIL: $*" >&2; }

check () {
  # check "description" command [args...]
  local desc="$1"; shift
  if "$@"; then pass; else fail "$desc"; fi
}

assert_file () {
  # assert_file PATH MODE
  local path="$1" mode="$2" got
  if [ ! -f "$path" ]; then fail "missing file $path"; return; fi
  got="$(stat -c '%a' "$path")"
  if [ "$got" = "$mode" ]; then pass; else fail "$path mode $got, want $mode"; fi
}

assert_absent () {
  if [ -e "$1" ]; then fail "unexpected path $1"; else pass; fi
}

assert_eq () {
  # assert_eq "description" GOT WANT
  if [ "$2" = "$3" ]; then pass; else fail "$1: got [$2] want [$3]"; fi
}

assert_contains () {
  # assert_contains "description" FILE PATTERN (fixed string)
  if grep -qF -- "$3" "$2" 2>/dev/null; then pass; else fail "$1: [$3] not in $2"; fi
}

assert_not_contains () {
  if grep -qF -- "$3" "$2" 2>/dev/null; then fail "$1: [$3] found in $2"; else pass; fi
}

# ini_get FILE SECTION KEY: prints the raw value of KEY in [SECTION].
ini_get () {
  awk -v sec="[$2]" -v key="$3" '
    /^\[/ { insec = ($0 == sec); next }
    insec {
      i = index($0, "=")
      if (i > 0 && substr($0, 1, i - 1) == key) { print substr($0, i + 1); exit }
    }' "$1"
}

# make_sandbox: creates $SB with home/, fakebin/, log/, release/ and exports
# HOME and every XDG base directory into it. PATH starts with the fakes.
make_sandbox () {
  SB="$(mktemp -d "${TMPDIR:-/tmp}/sbarhive-test.XXXXXX")"
  case "$SB" in /tmp/*|"${TMPDIR:-/nonexistent}"/*) ;; *) echo "bad sandbox $SB" >&2; exit 2 ;; esac
  mkdir -p "$SB/home" "$SB/fakebin" "$SB/log" "$SB/release"
  export SB
  export HOME="$SB/home"
  export XDG_DATA_HOME="$HOME/.local/share"
  export XDG_CONFIG_HOME="$HOME/.config"
  export XDG_CACHE_HOME="$HOME/.cache"
  export XDG_STATE_HOME="$HOME/.local/state"
  export FAKE_LOG="$SB/log"
  unset PREFIX DESTDIR WAYLAND_DISPLAY DISPLAY XDG_ACTIVATION_TOKEN DESKTOP_STARTUP_ID
  unset SBARHIVE_FILE_MANAGERS SBARHIVE_HYPRLAND SBARHIVE_SYSTEMD SBARHIVE_BRAND_DIR SBARHIVE_THUNAR_SYSTEM_UCA
  export PATH="$SB/fakebin:/usr/bin:/bin"

  cat > "$SB/fakebin/systemctl" <<'EOF'
#!/bin/sh
echo "$*" >> "$FAKE_LOG/systemctl.log"
case "$*" in
  *is-active*) if [ "${FAKE_GST_ACTIVE:-0}" = 0 ]; then exit 0; else exit 3; fi ;;
esac
exit 0
EOF
  cat > "$SB/fakebin/hyprctl" <<'EOF'
#!/bin/sh
echo "hyprctl $*" >> "$FAKE_LOG/calls.log"
exit 0
EOF
  cat > "$SB/fakebin/sbarhive" <<'EOF'
#!/bin/sh
echo "sbarhive $*" >> "$FAKE_LOG/calls.log"
exit 0
EOF
  for b in kbuildsycoca6 gtk-update-icon-cache update-desktop-database nautilus thunar nemo dolphin; do
    # shellcheck disable=SC2016 # the $ signs belong to the generated script
    printf '#!/bin/sh\necho "%s $*" >> "$FAKE_LOG/forbidden.log"\nexit 0\n' "$b" > "$SB/fakebin/$b"
  done
  chmod 0755 "$SB/fakebin/"*

  printf '#!/bin/sh\necho fake sbarhive\n' > "$SB/release/sbarhive"
  printf '#!/bin/sh\necho fake sbarhived\n' > "$SB/release/sbarhived"
  chmod 0755 "$SB/release/"*
  export SBARHIVE_BIN_DIR="$SB/release"
}

drop_sandbox () {
  case "${SB:-}" in /tmp/sbarhive-test.*|"${TMPDIR:-/nonexistent}"/sbarhive-test.*) rm -rf -- "$SB" ;; esac
}

# snapshot DIR: one line per entry with type, mode, path and, for files,
# sha256. Thunar's uca.xml content is excluded on purpose: the merge
# re-serialises it, so its content is checked separately.
snapshot () {
  (cd "$1" && find . -mindepth 1 -printf '%y %m %p\n' | sort | while read -r t m p; do
    if [ "$t" = f ] && [ "$p" != "./.config/Thunar/uca.xml" ]; then
      echo "$t $m $p $(sha256sum < "$p" | cut -c1-16)"
    else
      echo "$t $m $p"
    fi
  done)
}

finish () {
  local name="$1"
  if [ "$FAILS" -eq 0 ]; then
    echo "PASS $name ($CHECKS checks)"
    return 0
  fi
  echo "FAIL $name ($FAILS of $CHECKS checks failed)"
  return 1
}
