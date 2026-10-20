# shellcheck shell=bash
# Conditions are passed to check() in single quotes and expanded by eval.
# shellcheck disable=SC2016,SC2034
# Shared part of tests/drop/kwin.sh and tests/drop/gnome.sh (S-014b: AC5,
# AC6, AC7; S-014c: AC8, and the hooks for AC9 and AC10). Sourced inside the
# private compositor session only.
#
# The caller defines, for its compositor:
#   ptr_move X Y   move the pointer (screen coordinates)
#   ptr_down, ptr_up
#   shift_down, shift_up
#   hive_geo       prints "X Y W H" of the visible hive window, or nothing
# and sets BIN, ROOT, W (work dir), NEXT (source.py's path file), DLOG
# (daemon log, with WAYLAND_DEBUG=1 and SBARHIVE_DEBUG_FOCUS=1) and SLOG
# (source log).
#
# It may also define, after sourcing this file (S-014c):
#   on_shown N X Y W H   the hive is mapped at X Y W H, the drag has not
#                        entered it yet (AC10: the waiting text)
#   on_over N X Y W H    the drag is over the first cell (AC9: highlight)

FAILS=0
MARKS=()
EXPECT=()
on_shown() { :; }
on_over() { :; }

# AC8: the cell ids `sbarhive actions --json FILE` gives, space separated, in
# order. Run in the same sandbox environment as the daemon.
action_ids() { # action_ids FILE
  "$BIN/sbarhive" actions --json "$1" 2>/dev/null |
    python3 -I -c 'import json, sys; print(" ".join(a["id"] for a in json.load(sys.stdin)))'
}

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
  EXPECT+=("$(action_ids "$file")")
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
  on_shown "$n" "$gx" "$gy" "$gw" "$gh"
  local cx=$((gx + gw / 2)) cy=$((gy + gh / 2)) dx dy tx ty
  read -r dx dy <<<"$(webp_offset)"
  case "$target" in
    webp) tx=$((cx + dx)); ty=$((cy + dy)) ;;
    corner) tx=$((gx + 4)); ty=$((gy + 4)) ;;
    centre) tx=$cx; ty=$cy ;;
  esac
  # Enter at the transparent corner (KWin sends enter only after motion),
  # then over the WEBP cell, so a refusal on the corner or centre is a real
  # switch from accepted to declined, then to the target.
  local wx=$((cx + dx)) wy=$((cy + dy))
  ptr_move $((gx + 4)) $((gy + 4)); sleep 0.2
  ptr_move $(((gx + 4 + wx) / 2)) $(((gy + 4 + wy) / 2)); sleep 0.2
  ptr_move "$wx" "$wy"; sleep 0.4
  on_over "$n" "$gx" "$gy" "$gw" "$gh"
  [ "$target" != webp ] && { ptr_move "$tx" "$ty"; sleep 0.4; }
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
      # Right after the release, so the 10 s enter timeout (AC3) cannot be
      # what closed it.
      check "AC7 show $n: drop on the $target closes the hive" 'wait_gone 2'
      sleep 5
      check "AC7 show $n: drop on the $target writes no file" '[ "$(find "$W" -type f | sort)" = "$before" ]'
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
    # Line numbers are absolute in the trace. The first show reuses the
    # toplevel winit created when the daemon started (never mapped before),
    # so its get_xdg_surface can come before the show's mark: search the
    # whole trace up to the end of this show, not only its segment.
    local xs dd
    # S-018d: on the layer path (KWin) the hive's new surface is the catcher
    # given to get_layer_surface; on the window path (GNOME) get_xdg_surface.
    xs=$(head -n "${MARKS[i + 1]}" "$DLOG" | grep -nE -- '-> (xdg_wm_base#[0-9]+\.get_xdg_surface|zwlr_layer_shell_v1#[0-9]+\.get_layer_surface)\(' | tail -1 | cut -d: -f1)
    dd=$(grep -n -- '-> wl_data_device_manager#[0-9]*\.get_data_device(' <<<"$seg" | tail -1 | cut -d: -f1)
    [ -n "$dd" ] && dd=$((dd + MARKS[i]))
    echo "INFO show $((i + 1)): hive surface (get_xdg_surface or get_layer_surface) at line ${xs:-none}, get_data_device at line ${dd:-none}, count $(grep -c -- '-> wl_data_device_manager#[0-9]*\.get_data_device(' <<<"$seg")"
    check "AC5 show $((i + 1)): wl_data_device requested for the new hive surface" \
      '[ -n "$xs" ] && [ -n "$dd" ] && [ "$dd" -gt "$xs" ]'
    # Events print without the arrow. Every show must reach the receiver,
    # so the refused drops (corner, centre) cannot pass with no receiver.
    check "AC6 show $((i + 1)): receiver got enter on its data device" \
      'grep -q "wl_data_device#[0-9]*\.enter(" <<<"$seg"'
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

# S-014c AC8, after protocol_checks (which closes MARKS): the cells are built
# from the files read at enter, before the drop. The daemon prints, on stderr,
# one line per enter of a drag into a drop-mode hive, after the cells for the
# entered files are known and before any drop:
#   drop cells: ID ID ...
# the ids of the hive's cells in cell order, separated by one space (nothing
# after the colon when no action matches). Every such line of a show must
# equal `sbarhive actions --json` for that show's file, and there must be one.
enter_cells_checks() {
  local i first="" other=""
  for ((i = 0; i + 1 < ${#MARKS[@]}; i++)); do
    local seg lines want got
    seg=$(sed -n "$((MARKS[i] + 1)),$((MARKS[i + 1]))p" "$DLOG")
    lines=$(grep -E '^drop cells:' <<<"$seg")
    want=${EXPECT[i]}
    got=$(head -1 <<<"$lines" | sed -E 's/^drop cells: ?//')
    echo "INFO AC8 show $((i + 1)): $(grep -c . <<<"$lines") \"drop cells\" lines, first: ${got:-none}; sbarhive actions: ${want:-none}"
    check "AC8 show $((i + 1)): the daemon reports the cells built at enter" '[ -n "$lines" ]'
    check "AC8 show $((i + 1)): every enter's cells equal sbarhive actions for the dragged file" \
      '[ -n "$want" ] && [ -n "$lines" ] && [ -z "$(grep -vxF "drop cells: $want" <<<"$lines")" ]'
  done
  # The PNG of show 1 and the PDF of the last show: two different, non-empty
  # lists, so cells kept from an earlier drag cannot pass.
  first=${EXPECT[0]}; other=${EXPECT[${#EXPECT[@]} - 1]}
  check "AC8: the PNG and PDF action lists are non-empty and differ" \
    '[ -n "$first" ] && [ -n "$other" ] && [ "$first" != "$other" ]'
}

summary() {
  if [ "$FAILS" -eq 0 ]; then echo "RESULT PASS"; else echo "RESULT FAIL ($FAILS)"; fi
}
