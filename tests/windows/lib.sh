#!/usr/bin/env bash
# Shared helpers for the S-026a Windows build tests. Sourced, never executed.
# Reuses the counters and asserts of tests/integrations/lib.sh, but not its
# HOME swapping sandbox: these tests need the real systemd user manager.

# shellcheck source=tests/integrations/lib.sh
. "$(dirname "${BASH_SOURCE[0]}")/../integrations/lib.sh"
WIN="$REPO_ROOT/scripts/windows"
FIX="$REPO_ROOT/tests/windows/fixtures"
WIN_TARGET_DEFAULT=/home/sbarah/.cache/sbarhive-win-target
SIX_G=6442450944
export WIN FIX WIN_TARGET_DEFAULT SIX_G

# The cgroup path of the calling process, as in /proc/self/cgroup.
own_cgroup () { cut -d: -f3 /proc/self/cgroup | head -n 1; }

# make_fakes: creates $FSB with bin/ and log/. bin/cargo logs one line per
# call and never builds anything:
#   CALL|<cgroup>|<memory.max of that cgroup>|<CARGO_TARGET_DIR>|<CARGO_BUILD_JOBS>|<effective target dir>|<argv>
# It writes a fake sbarhive.exe only when the effective target dir is inside
# $FSB, so the real cache at $WIN_TARGET_DEFAULT is never touched.
make_fakes () {
  FSB="$(mktemp -d "${TMPDIR:-/tmp}/sbarhive-wintest.XXXXXX")"
  mkdir -p "$FSB/bin" "$FSB/log"
  export FSB
  cat > "$FSB/bin/cargo" <<'EOF'
#!/bin/sh
cg=$(cut -d: -f3 /proc/self/cgroup | head -n 1)
mm=$(cat "/sys/fs/cgroup$cg/memory.max" 2>/dev/null || echo none)
td=""
prev=""
for a in "$@"; do
  [ "$prev" = --target-dir ] && td=$a
  case $a in --target-dir=*) td=${a#--target-dir=} ;; esac
  prev=$a
done
[ -z "$td" ] && td=${CARGO_TARGET_DIR-}
echo "CALL|$cg|$mm|${CARGO_TARGET_DIR-<unset>}|${CARGO_BUILD_JOBS-<unset>}|${td:-<none>}|$*" >> "$FSB/log/cargo.log"
case "$td" in
  "$FSB"/*)
    mkdir -p "$td/x86_64-pc-windows-gnu/release"
    printf 'MZfake' > "$td/x86_64-pc-windows-gnu/release/sbarhive.exe"
    ;;
esac
exit 0
EOF
  chmod 0755 "$FSB/bin/cargo"
}

drop_fakes () {
  case "${FSB:-}" in /tmp/sbarhive-wintest.*|"${TMPDIR:-/nonexistent}"/sbarhive-wintest.*) rm -rf -- "$FSB" ;; esac
}

# cargo_call WORD: the first logged cargo call whose argv contains WORD.
cargo_call () { grep -m 1 -E "\|([^|]* )?$1( |$)" "$FSB/log/cargo.log" 2>/dev/null; }

# field N LINE: the Nth |-separated field (1 is CALL).
field () { printf '%s\n' "$2" | cut -d'|' -f"$1"; }

# has_jobs_2 LINE: the call ran with two jobs (-j 2, -j2, --jobs 2,
# --jobs=2 or CARGO_BUILD_JOBS=2).
has_jobs_2 () {
  local argv jobs
  argv=" $(field 7 "$1") "
  jobs="$(field 5 "$1")"
  case "$argv" in *" -j 2 "*|*" -j2 "*|*" --jobs 2 "*|*" --jobs=2 "*) return 0 ;; esac
  [ "$jobs" = 2 ]
}

# argv_has LINE ARG...: every ARG appears as a word sequence in the argv.
argv_has () {
  local argv a
  argv=" $(field 7 "$1") "; shift
  for a in "$@"; do
    case "$argv" in *" $a "*) ;; *) return 1 ;; esac
  done
}

# has_target_windows LINE: --target x86_64-pc-windows-gnu or --target=...
has_target_windows () {
  argv_has "$1" --target x86_64-pc-windows-gnu || argv_has "$1" --target=x86_64-pc-windows-gnu
}
