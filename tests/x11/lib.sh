# SPDX-License-Identifier: GPL-3.0-or-later
# shellcheck shell=bash
# Pure helpers of tests/x11/run.sh (S-114). Sourced, no side effects, no
# display needed. Tested by tests/x11/s114.sh.

# x11_parse_surface LINE: prints "X Y W H" from "hive surface: window X Y W H"
# (any prefix before it). Any other line exits 1.
x11_parse_surface() {
  local re='hive surface: window (-?[0-9]+) (-?[0-9]+) (-?[0-9]+) (-?[0-9]+)[[:space:]]*$'
  [[ "$1" =~ $re ]] || return 1
  echo "${BASH_REMATCH[1]} ${BASH_REMATCH[2]} ${BASH_REMATCH[3]} ${BASH_REMATCH[4]}"
}

# x11_within A B TOL: |A - B| <= TOL.
x11_within() {
  local d=$(($1 - $2))
  [ "${d#-}" -le "$3" ]
}

# x11_geometry_match 'X Y W H' 'X Y W H': every field within 1 px.
x11_geometry_match() {
  local -a a b
  local i
  read -ra a <<<"$1"
  read -ra b <<<"$2"
  [ "${#a[@]}" -eq 4 ] && [ "${#b[@]}" -eq 4 ] || return 1
  for i in 0 1 2 3; do
    x11_within "${a[$i]}" "${b[$i]}" 1 || return 1
  done
}

# x11_box_inside 'X Y W H' SCREEN_W SCREEN_H: the box lies inside the screen.
x11_box_inside() {
  local -a b
  read -ra b <<<"$1"
  [ "${b[0]}" -ge 0 ] && [ "${b[1]}" -ge 0 ] &&
    [ $((b[0] + b[2])) -le "$2" ] && [ $((b[1] + b[3])) -le "$3" ]
}

# x11_count_choice FILE: lines equal to the surface choice line of a non
# Wayland session.
x11_count_choice() {
  grep -cxF 'hive surface choice: window, not a Wayland session' "$1" || true
}

# x11_pixel_eq A B: two colours are equal, ignoring case, "#" and "0x".
x11_pixel_eq() {
  local x="${1,,}" y="${2,,}"
  x="${x#\#}"
  x="${x#0x}"
  y="${y#\#}"
  y="${y#0x}"
  [ "$x" = "$y" ]
}

# x11_median N...: the median; with an even count the lower middle value.
x11_median() {
  local sorted
  mapfile -t sorted < <(printf '%s\n' "$@" | sort -n)
  echo "${sorted[$(((${#sorted[@]} - 1) / 2))]}"
}

# x11_rss_ok KB: the resident set is at most 40 MB (40960 KB).
x11_rss_ok() {
  [ "$1" -le 40960 ]
}
