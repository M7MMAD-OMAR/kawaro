#!/usr/bin/env bash
# Refuses a Windows executable that imports a mingw runtime DLL (any name
# matching lib*.dll, case-insensitive, such as libwinpthread-1.dll), since
# those would have to be shipped beside it. System DLLs pass.
#   scripts/windows/check-dlls.sh <exe>
#   scripts/windows/check-dlls.sh --text <objdump -p output>
# Exit 0 prints "dll check: ok <name> <N> imports" (N counts DLL Name lines);
# exit 1 lists the rejected DLLs; exit 2 is an input error: bad usage, a
# missing --text file, an objdump failure, or text with no DLL Name line.
set -eu
if [ "${1:-}" = --text ] && [ $# -eq 2 ]; then
  [ -f "$2" ] || { echo "dll check: no such file: $2" >&2; exit 2; }
  name="$(basename "$2")"
  text="$(cat "$2")" || exit 2
elif [ $# -eq 1 ] && [ "$1" != --text ]; then
  name="$(basename "$1")"
  text="$(x86_64-w64-mingw32-objdump -p "$1")" || exit 2
else
  echo "usage: $0 <exe> | --text <file>" >&2
  exit 2
fi
dlls="$(printf '%s\n' "$text" | tr -d '\r' | sed -n 's/^[[:space:]]*DLL Name:[[:space:]]*//p')"
n="$(printf '%s\n' "$dlls" | grep -c . || true)"
if [ "$n" -eq 0 ]; then
  echo "dll check: no DLL Name line in the objdump output for $name" >&2
  exit 2
fi
bad="$(printf '%s\n' "$dlls" | grep -iE '^lib.*\.dll$' | sort -u || true)"
if [ -n "$bad" ]; then
  echo "dll check: $name needs mingw DLLs that are not shipped:"
  printf '%s\n' "$bad" | sed 's/^/  /'
  exit 1
fi
echo "dll check: ok $name $n imports"
