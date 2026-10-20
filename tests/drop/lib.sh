# shellcheck shell=bash
# Conditions are passed to check() in single quotes and expanded by eval.
# shellcheck disable=SC2016,SC2034
# Shared part of tests/drop/kwin.sh and tests/drop/gnome.sh (S-014b: AC5,
# AC6, AC7). Sourced inside the private compositor session only.
#
# The caller defines, for its compositor:
#   ptr_move X Y   move the pointer (screen coordinates)
#   ptr_down, ptr_up
#   shift_down, shift_up
#   hive_geo       prints "X Y W H" of the visible hive window, or nothing
# and sets BIN, ROOT, W (work dir), NEXT (source.py's path file), DLOG
# (daemon log, with WAYLAND_DEBUG=1) and SLOG (source log).

FAILS=0
MARKS=()

check() { # check NAME CONDITION
  if eval "$2"; then echo "PASS $1"; else echo "FAIL $1"; FAILS=$((FAILS + 1)); fi
}

# Window-local positions from the shipped layout rule (crates/ui/src/layout.rs
# and geometry.rs): cell radius is hex.size.md / sqrt(3), spacing is radius +
# gap / sqrt(3), the centre cell sits in the middle of the window, and the
# first action (WEBP for a PNG) is the first cell of ring 1, axial (1, -1),
# top right of the centre in the English (left to right) layout.
# Prints "WEBP_DX WEBP_DY" relative to the window centre.
webp_offset() {
  python3 -I - "$ROOT/design/tokens.json" <<'PY'
import json, math, sys
t = json.load(open(sys.argv[1]))["hex"]
r = t["size"]["md"] / math.sqrt(3)
s = r + t["gap"] / math.sqrt(3)
print(round(s * math.sqrt(3) / 2), round(-1.5 * s))
PY
}

# Deadlines in wall time: hive_geo itself can take half a second.
deadline() { echo $(( ${EPOCHREALTIME/./} / 1000 + $1 * 1000 )); }
now_ms() { echo $(( ${EPOCHREALTIME/./} / 1000 )); }

wait_geo() { # wait_geo SECONDS: prints the hive geometry once it is mapped
  local end g; end=$(deadline "$1")
  while [ "$(now_ms)" -lt "$end" ]; do
    g=$(hive_geo)
    [ -n "$g" ] && { echo "$g"; return 0; }
    sleep 0.1
  done
  return 1
}

wait_gone() { # wait_gone SECONDS
  local end; end=$(deadline "$1")
  while [ "$(now_ms)" -lt "$end" ]; do
    [ -z "$(hive_geo)" ] && return 0
    sleep 0.1
  done
  return 1
}

wait_file() { # wait_file PATH SECONDS
  local end; end=$(deadline "$2")
  while [ "$(now_ms)" -lt "$end" ]; do
    [ -s "$1" ] && return 0
    sleep 0.1
  done
  return 1
}

# One show: drag FILE from the source, run `sbarhive drop` mid-drag (as the
# shortcut would), enter the hive, move to TARGET (webp, corner or centre)
# and release, with Shift held when SHIFT=1.
cycle() { # cycle N FILE TARGET SHIFT
  local n=$1 file=$2 target=$3 shift=$4
  local base="${file%.*}" smark
  echo "== show $n: $(basename "$file") on $target$([ "$shift" = 1 ] && echo ', Shift held')"
  printf '%s\n' "$file" >"$NEXT"
  MARKS+=("$(wc -l <"$DLOG")")
  smark=$(wc -l <"$SLOG")
  local sha_before; sha_before=$(sha256sum "$file" | cut -d' ' -f1)
  local before; before=$(find "$W" -type f | sort)

  # Press on the source and drag past GTK's threshold.
  ptr_move 150 150; sleep 0.2; ptr_down; sleep 0.2
  for i in 1 2 3 4 5 6; do ptr_move $((150 + i * 10)) $((150 + i * 5)); sleep 0.05; done
  sleep 1
  "$BIN/sbarhive" drop >>"$OUT_LOG-cli.log" 2>&1
  # Poll instead of a fixed sleep: the hive closes 10 s after showing
  # without an enter (AC3), and the first show can be slow (ADR 0001, 8).
  local geo
  if ! geo=$(wait_geo 8); then
    echo "FAIL AC5 show $n: hive window did not appear within 8 s"
    FAILS=$((FAILS + 1))
    ptr_up; sleep 1
    return
  fi
  local gx gy gw gh; read -r gx gy gw gh <<<"$geo"
  echo "INFO show $n: hive at $gx $gy ${gw}x$gh"
  local cx=$((gx + gw / 2)) cy=$((gy + gh / 2)) dx dy tx ty
  read -r dx dy <<<"$(webp_offset)"
  case "$target" in
    webp) tx=$((cx + dx)); ty=$((cy + dy)) ;;
    corner) tx=$((gx + 4)); ty=$((gy + 4)) ;;
    centre) tx=$cx; ty=$cy ;;
  esac
  # Enter at the transparent corner (KWin sends enter only after motion),
  # then go to the target in two steps.
  ptr_move $((gx + 4)) $((gy + 4)); sleep 0.2
  ptr_move $(((gx + 4 + tx) / 2)) $(((gy + 4 + ty) / 2)); sleep 0.2
  ptr_move "$tx" "$ty"; sleep 0.4
  [ "$shift" = 1 ] && { shift_down; sleep 0.2; }
  ptr_up; sleep 0.3
  [ "$shift" = 1 ] && shift_up

  local src
  case "$target" in
    webp)
      check "AC7 show $n: $(basename "$base").webp written next to the original within 5 s" \
        'wait_file "$base.webp" 5'
      check "AC7 show $n: output is image/webp" '[ "$(file --mime-type -b "$base.webp" 2>/dev/null)" = image/webp ]'
      src=$(tail -n +$((smark + 1)) "$SLOG")
      check "AC6 show $n: source reports deleted=False" 'grep -q "drag-end deleted=False" <<<"$src"'
      check "AC6 show $n: original still present" '[ -f "$file" ]'
      check "AC6 show $n: original sha256 unchanged" '[ "$(sha256sum "$file" 2>/dev/null | cut -d" " -f1)" = "$sha_before" ]'
      wait_gone 3 || echo "INFO show $n: hive still shown 3 s after the drop"
      ;;
    corner|centre)
      sleep 5
      check "AC7 show $n: drop on the $target writes no file" '[ "$(find "$W" -type f | sort)" = "$before" ]'
      check "AC7 show $n: drop on the $target closes the hive" 'wait_gone 2'
      src=$(tail -n +$((smark + 1)) "$SLOG")
      check "AC6 show $n: drop on the $target is refused (source drag-cancel, no drag-end)" \
        'grep -q "drag-cancel" <<<"$src" && ! grep -q "drag-end deleted=True" <<<"$src"'
      check "AC6 show $n: original sha256 unchanged" '[ "$(sha256sum "$file" 2>/dev/null | cut -d" " -f1)" = "$sha_before" ]'
      ;;
  esac
  # The next drag needs the hive gone, or it is the same surface.
  wait_gone 5 || { ptr_move 1000 700; sleep 0.5; wait_gone 5; } || true
  sleep 1
}

# AC5 and AC6 from the daemon's WAYLAND_DEBUG=1 trace, after all shows.
protocol_checks() {
  MARKS+=("$(wc -l <"$DLOG")")
  local i
  for ((i = 0; i + 1 < ${#MARKS[@]}; i++)); do
    local seg; seg=$(sed -n "$((MARKS[i] + 1)),$((MARKS[i + 1]))p" "$DLOG")
    # The hive's new toplevel surface in this show, then a data device
    # requested after it (Slint destroys the surface on hide, ADR 0001, 5).
    local xs dd
    xs=$(grep -n -- '-> xdg_wm_base#[0-9]*\.get_xdg_surface(' <<<"$seg" | tail -1 | cut -d: -f1)
    dd=$(grep -n -- '-> wl_data_device_manager#[0-9]*\.get_data_device(' <<<"$seg" | tail -1 | cut -d: -f1)
    echo "INFO show $((i + 1)): get_xdg_surface at +${xs:-none}, get_data_device at +${dd:-none}, count $(grep -c -- '-> wl_data_device_manager#[0-9]*\.get_data_device(' <<<"$seg")"
    check "AC5 show $((i + 1)): wl_data_device requested for the new hive surface" \
      '[ -n "$xs" ] && [ -n "$dd" ] && [ "$dd" -gt "$xs" ]'
  done
  local sa; sa=$(grep -o -- '-> wl_data_offer#[0-9]*\.set_actions([^)]*)' "$DLOG" | sed 's/.*set_actions//' | sort -u | tr '\n' ' ')
  echo "INFO set_actions seen: ${sa:-none}"
  check "AC6: set_actions(Copy, Copy) sent" 'grep -qF "(1, 1)" <<<"$sa"'
  # Refusing with set_actions(0, 0) is allowed; offering move or ask is not.
  check "AC6: set_actions never offers anything but copy" \
    '[ -z "$(tr " " "\n" <<<"${sa//, /,}" | grep -v -e "^$" -e "^(1,1)$" -e "^(0,0)$")" ]'
  local acc; acc=$(grep -o -- '-> wl_data_offer#[0-9]*\.accept([^)]*)' "$DLOG" | grep -o '"[^"]*"' | sort -u | tr '\n' ' ')
  echo "INFO accept mime types: ${acc:-none}"
  check 'AC6: accepts "text/uri-list" and no other type' '[ "$acc" = "\"text/uri-list\" " ]'
}

summary() {
  if [ "$FAILS" -eq 0 ]; then echo "RESULT PASS"; else echo "RESULT FAIL ($FAILS)"; fi
}
