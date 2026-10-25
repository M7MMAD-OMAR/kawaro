# shellcheck shell=bash
# Sourced by build.sh, check.sh and make-iso.sh, never executed. Sourcing it
# has no side effect: it only sets the default target dir and defines
# win_scope.
#   KAWARO_WIN_TARGET_DEFAULT  ${XDG_CACHE_HOME:-$HOME/.cache}/kawaro-win-target,
#                                the CARGO_TARGET_DIR when the caller sets none
#   win_scope "$@"               outside CI, re-runs the calling script inside
#                                a 6 GB, two CPU systemd scope, once
# Read by the scripts that source this file.
# shellcheck disable=SC2034
KAWARO_WIN_TARGET_DEFAULT="${XDG_CACHE_HOME:-$HOME/.cache}/kawaro-win-target"

win_scope () {
  if [ "${CI:-}" != true ] && [ -z "${KAWARO_WIN_SCOPE:-}" ]; then
    exec systemd-run --user --scope -q -p MemoryMax=6G -p CPUQuota=200% \
      env KAWARO_WIN_SCOPE=1 "$0" "$@"
  fi
}
