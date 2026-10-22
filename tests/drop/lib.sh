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
# and set ANCHOR_SETTLE=1 (S-019d, layer path): a drop-mode hive shown with
# the drag standing still sits in the middle, and after 300 ms from its
# first frame its catcher takes only the hive rectangle and it never moves
# (manager decision 12, 2026-10-21). Each show then waits 400 ms, so every
# one of these shows takes that path, first moves the drag to the hive's
# centre (inside the rectangle, so the drag enters there), and reads the
# geometry again before going on.

#   on_centre N X Y W H  the drag is over the centre cell after entering
#                        (S-019 AC15: the preview in the centre), for the
#                        shows whose TARGET is centre
#
# S-019f (AC14, AC15): every show also records what its centre must show
# after enter (PREVIEW_WANT, from preview_want), and preview_checks, after
# protocol_checks, reads the daemon's "hive preview: ..." lines per show.
# A show whose file is $THUMBED (set by the caller, with a thumbnail seeded
# in the sandbox's XDG_CACHE_HOME) wants "thumb W H".

FAILS=0
ANCHOR_SETTLE=${ANCHOR_SETTLE:-0}
MARKS=()
EXPECT=()
PREVIEW_WANT=()
THUMBED=${THUMBED:-}
on_shown() { :; }
on_over() { :; }
on_centre() { :; }

# preview_want FILE: what the drop hive's centre shows for FILE after enter
# (S-018 AC24 rules): "thumb" for the seeded thumbnail, else the kind icon,
# "icon pdf" for a PDF and "icon image" for a PNG.
preview_want() {
  if [ -n "$THUMBED" ] && [ "$1" = "$THUMBED" ]; then echo thumb
  else case "$1" in *.pdf) echo "icon pdf" ;; *) echo "icon image" ;; esac; fi
}

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

# Press on the source and drag past GTK's threshold.
drag_past_threshold() {
  local i
  ptr_move 150 150; sleep 0.2; ptr_down; sleep 0.2
  for i in 1 2 3 4 5 6; do ptr_move $((150 + i * 10)) $((150 + i * 5)); sleep 0.05; done
  sleep 1
}

# Enter the hive at GX GY GW GH at its transparent corner (KWin sends enter
# only after motion), then move over the WEBP cell, where it stays.
move_to_webp() { # move_to_webp GX GY GW GH
  local dx dy; read -r dx dy <<<"$(webp_offset)"
  local wx=$(($1 + $3 / 2 + dx)) wy=$(($2 + $4 / 2 + dy))
  ptr_move $(($1 + 4)) $(($2 + 4)); sleep 0.2
  ptr_move $((($1 + 4 + wx) / 2)) $((($2 + 4 + wy) / 2)); sleep 0.2
  ptr_move "$wx" "$wy"; sleep 0.4
}

# settle_at_centre GX GY GW GH: with ANCHOR_SETTLE=1, the first drag
# position is the hive's centre; prints the geometry after it ("X Y W H").
settle_at_centre() {
  if [ "$ANCHOR_SETTLE" = 1 ]; then
    sleep 0.4
    ptr_move $(($1 + $3 / 2)) $(($2 + $4 / 2)); sleep 0.5
    hive_geo
  else
    echo "$1 $2 $3 $4"
  fi
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
  PREVIEW_WANT+=("$(preview_want "$file")")
  smark=$(wc -l <"$SLOG")
  local sha_before; sha_before=$(sha256sum "$file" | cut -d' ' -f1)
  local before; before=$(find "$W" -type f | sort)

  drag_past_threshold
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
  local settled; settled=$(settle_at_centre "$gx" "$gy" "$gw" "$gh")
  [ -n "$settled" ] && [ "$settled" != "$gx $gy $gw $gh" ] && echo "INFO show $n: hive moved to $settled at the first drag position"
  [ -n "$settled" ] && read -r gx gy gw gh <<<"$settled"
  local cx=$((gx + gw / 2)) cy=$((gy + gh / 2)) dx dy tx ty
  read -r dx dy <<<"$(webp_offset)"
  case "$target" in
    webp) tx=$((cx + dx)); ty=$((cy + dy)) ;;
    corner) tx=$((gx + 4)); ty=$((gy + 4)) ;;
    centre) tx=$cx; ty=$cy ;;
  esac
  # A refusal on the corner or centre is then a real switch from accepted
  # to declined, then to the target.
  move_to_webp "$gx" "$gy" "$gw" "$gh"
  on_over "$n" "$gx" "$gy" "$gw" "$gh"
  [ "$target" != webp ] && { ptr_move "$tx" "$ty"; sleep 0.4; }
  [ "$target" = centre ] && on_centre "$n" "$gx" "$gy" "$gw" "$gh"
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
  # The PNG of show 1 and the PDF of a later show (the last one, or the
  # first that differs when PNG shows follow it, as in tests/drop/kwin.sh):
  # two different, non-empty lists, so cells kept from an earlier drag
  # cannot pass.
  first=${EXPECT[0]}; other=${EXPECT[${#EXPECT[@]} - 1]}
  local e; for e in "${EXPECT[@]}"; do [ "$e" != "$first" ] && { other=$e; break; }; done
  check "AC8: the PNG and PDF action lists are non-empty and differ" \
    '[ -n "$first" ] && [ -n "$other" ] && [ "$first" != "$other" ]'
}

# S-019f AC15, after protocol_checks (which closes MARKS). The daemon runs
# with SBARHIVE_DEBUG_FOCUS=1 and prints one "hive preview: thumb W H" or
# "hive preview: icon NAME" line per preview it draws (S-018h). In a drop
# show the preview comes with the enter (AC14), so per show:
#   - no "hive preview" line before the first wl_data_device.enter (before
#     enter: the waiting text and no preview, S-014c AC10);
#   - one "hive preview" line per enter: as many as "drop cells:" lines
#     (one per enter, S-014 AC8), and at least one;
#   - every one is PREVIEW_WANT's: "icon image" for a PNG, "icon pdf" for
#     the PDF, "thumb W H" with W and H 1 to 128 for $THUMBED.
preview_checks() {
  local i
  for ((i = 0; i + 1 < ${#MARKS[@]}; i++)); do
    local seg want lines n cells e1 early bad
    seg=$(sed -n "$((MARKS[i] + 1)),$((MARKS[i + 1]))p" "$DLOG")
    want=${PREVIEW_WANT[i]:-?}
    lines=$(grep -E '^hive preview: ' <<<"$seg")
    n=$(grep -c . <<<"$lines")
    cells=$(grep -cE '^drop cells:' <<<"$seg")
    e1=$(grep -nE '^\[[0-9:.]+\] wl_data_device[#@][0-9]+\.enter\(' <<<"$seg" | head -1 | cut -d: -f1)
    early=0
    [ -n "$e1" ] && early=$(head -n "$e1" <<<"$seg" | grep -cE '^hive preview: ')
    [ -z "$e1" ] && early=$n
    if [ "$want" = thumb ]; then
      bad=$(grep -vE '^hive preview: thumb ([1-9][0-9]?|1[01][0-9]|12[0-8]) ([1-9][0-9]?|1[01][0-9]|12[0-8])$' <<<"$lines" | grep -c .)
    else
      bad=$(grep -vxF "hive preview: $want" <<<"$lines" | grep -c .)
    fi
    echo "INFO S-019 AC15 show $((i + 1)): want \"$want\"; preview lines $n ($(head -1 <<<"$lines")), drop cells lines $cells, before the first enter $early"
    check "S-019 AC14 show $((i + 1)): no preview before the drag enters (no \"hive preview\" line before the first enter)" \
      '[ -n "$e1" ] && [ "$early" -eq 0 ]'
    check "S-019 AC15 show $((i + 1)): one \"hive preview\" line per enter (as many as \"drop cells\" lines, at least one)" \
      '[ "$n" -ge 1 ] && [ "$n" -eq "$cells" ]'
    check "S-019 AC15 show $((i + 1)): every \"hive preview\" line is \"$want\"" '[ "$n" -ge 1 ] && [ "$bad" -eq 0 ]'
  done
}

# S-014d AC11: the shortcut pressed mid-drag. The caller (shortcut mode of
# kwin.sh and gnome.sh) binds Super+Z in its private compositor to the
# wrapper $SHORTCUT_WRAP, which appends one UTC time line to $SHORTCUT_MARKS
# and execs `sbarhive drop`. This script never runs `sbarhive drop` itself in
# that mode. The caller also defines:
#   press_shortcut   Super+Z through the compositor's input path
#   press_esc        Esc through the compositor's input path
# and sets HIVE_DESTROY_RE, the daemon trace request that ends a show (KWin:
# the layer surface's destroy; GNOME: the toplevel's destroy), and
# ESC_NO_KEYBOARD=1 when the hive is a layer that takes no keyboard in drop
# mode (S-018 AC13), so Esc must never reach the daemon there.

# Writes the wrapper the caller binds Super+Z to: it appends one UTC time
# line to $SHORTCUT_MARKS and execs `sbarhive drop`.
make_shortcut_wrap() {
  SHORTCUT_MARKS="$XDG_RUNTIME_DIR/shortcut-marks"
  SHORTCUT_WRAP="$XDG_RUNTIME_DIR/sbh-drop"
  printf '#!/bin/sh\ndate -u +%%T.%%6N >>"%s"\nexec "%s" drop >>"%s" 2>&1\n' \
    "$SHORTCUT_MARKS" "$BIN/sbarhive" "$OUT_LOG-cli.log" >"$SHORTCUT_WRAP"
  chmod +x "$SHORTCUT_WRAP"
}

# Seconds of the day of a "[HH:MM:SS.ffffff]" trace stamp or "HH:MM:SS.ffffff".
tod() { awk -F: '{gsub(/[][]/, ""); printf "%.6f\n", $1 * 3600 + $2 * 60 + $3}' <<<"$1"; }
marks_count() { if [ -f "$SHORTCUT_MARKS" ]; then grep -c . "$SHORTCUT_MARKS"; else echo 0; fi; }

# Press on the source and drag past GTK's threshold; checks the drag is on.
start_drag() { # start_drag N SMARK
  drag_past_threshold
  local src; src=$(tail -n +$(($2 + 1)) "$SLOG")
  check "AC11 $1: the drag is under way before the shortcut (drag-begin, no drag-end or drag-cancel yet)" \
    'grep -q "drag-begin" <<<"$src" && ! grep -qE "drag-end|drag-cancel" <<<"$src"'
}

# The shortcut mid-drag: the hive appears, shown by the binding, not by us.
# Sets GEO to the hive geometry, empty when it did not appear.
shortcut_show() { # shortcut_show N
  local n=$1 m0 m1
  check "AC11 $n: no hive is shown before the shortcut" '[ -z "$(hive_geo)" ]'
  m0=$(marks_count)
  press_shortcut
  GEO=$(wait_geo 8)
  m1=$(marks_count)
  echo "INFO AC11 $n: binding ran $((m1 - m0)) time(s), hive ${GEO:-not shown}"
  check "AC11 $n: Super+Z mid-drag ran the bound \`sbarhive drop\` exactly once" '[ $((m1 - m0)) -eq 1 ]'
  check "AC11 $n: the hive appeared within 8 s of the shortcut" '[ -n "$GEO" ]'
}

# One shortcut run: drag a PNG, Super+Z, enter the hive, drop on WEBP.
shortcut_drop() { # shortcut_drop N FILE
  local n="run $1" file=$2 base="${2%.*}" smark
  echo "== AC11 $n: $(basename "$file"), Super+Z mid-drag, drop on WEBP"
  printf '%s\n' "$file" >"$NEXT"
  smark=$(wc -l <"$SLOG")
  local sha_before; sha_before=$(sha256sum "$file" | cut -d' ' -f1)
  start_drag "$n" "$smark"
  shortcut_show "$n"
  [ -n "$GEO" ] || { ptr_up; sleep 1; wait_gone 12; return; }
  # shellcheck disable=SC2086
  GEO=$(settle_at_centre $GEO)
  # shellcheck disable=SC2086
  move_to_webp $GEO
  ptr_up; sleep 0.3
  check "AC11 $n: $(basename "$base").webp written next to the original within 5 s" 'wait_file "$base.webp" 5'
  check "AC11 $n: output is image/webp" '[ "$(file --mime-type -b "$base.webp" 2>/dev/null)" = image/webp ]'
  local src; src=$(tail -n +$((smark + 1)) "$SLOG")
  check "AC11 $n: source reports deleted=False and the original's sha256 is unchanged" \
    'grep -q "drag-end deleted=False" <<<"$src" && [ "$(sha256sum "$file" 2>/dev/null | cut -d" " -f1)" = "$sha_before" ]'
  wait_gone 5 || { ptr_move 1000 700; sleep 0.5; wait_gone 5; } || true
  sleep 1
}

# Esc mid-drag. WHERE=over: the drag has entered the hive and is over the
# WEBP cell (a release would write a file), then Esc. The compositor cancels
# the drag (the source sees drag-cancel) and the hive gets leave: it must be
# gone within 500 ms of the Esc, timed from the daemon's trace stamps.
# WHERE=before: Esc before the drag enters the hive. On the layer path
# nothing reaches the hive (keyboard 0 in drop mode, S-018 AC13), so it may
# close only by its 10 s timeout (AC3): checked within 12 s, the close delay
# after Esc is INFO; and with ESC_NO_KEYBOARD=1 (the layer path, S-021) the
# show's one probe line is "hive probe: hidden (cap)". In both, no file is
# written.
shortcut_esc() { # shortcut_esc N FILE over|before
  local n="esc $3" file=$2 where=$3 smark dmark
  echo "== AC11 $n: $(basename "$file"), Super+Z mid-drag, Esc $([ "$where" = over ] && echo 'over the WEBP cell' || echo 'before entering the hive')"
  printf '%s\n' "$file" >"$NEXT"
  smark=$(wc -l <"$SLOG")
  local before; before=$(find "$W" -type f | sort)
  start_drag "$n" "$smark"
  shortcut_show "$n"
  [ -n "$GEO" ] || { ptr_up; sleep 1; wait_gone 12; return; }
  if [ "$where" = over ]; then
    # shellcheck disable=SC2086
    GEO=$(settle_at_centre $GEO)
    # shellcheck disable=SC2086
    move_to_webp $GEO; sleep 0.2
  fi
  dmark=$(wc -l <"$DLOG")
  local t_esc; t_esc=$(date -u +%T.%6N)
  press_esc
  if [ "$where" = over ]; then
    sleep 1.5
  else
    wait_gone 12
  fi
  ptr_up; sleep 0.3
  local seg; seg=$(tail -n +$((dmark + 1)) "$DLOG")
  local entered; entered=$(sed -n "$((DSHOW + 1)),${dmark}p" "$DLOG" | grep -cE '^\[[0-9:.]+\] wl_data_device[#@][0-9]+\.enter\(')
  local gone_line gone_ms=""
  gone_line=$(grep -m1 -E -- "$HIVE_DESTROY_RE" <<<"$seg" | grep -oE '^\[[0-9:.]+\]')
  if [ -n "$gone_line" ]; then
    gone_ms=$(awk -v a="$(tod "$t_esc")" -v b="$(tod "$gone_line")" 'BEGIN{d=b-a; if (d < -43200) d += 86400; printf "%.1f\n", d * 1000}')
  fi
  echo "INFO AC11 $n: drag entered the hive before Esc: $entered enter event(s); hive surface destroyed ${gone_ms:-never} ms after Esc ($t_esc UTC)"
  local src; src=$(tail -n +$((smark + 1)) "$SLOG")
  # GTK 4 follows drag-cancel with drag-end deleted=False; a drop would
  # give drag-end without drag-cancel.
  check "AC11 $n: Esc cancels the drag (source drag-cancel, no drag-end deleted=True)" \
    'grep -q "drag-cancel" <<<"$src" && ! grep -q "drag-end deleted=True" <<<"$src"'
  if [ "$where" = over ]; then
    check "AC11 $n: the drag had entered the hive before Esc (a release would have dropped)" '[ "$entered" -ge 1 ]'
    # After the Esc, not before it (a destroy before the stamp is a bug).
    check "AC11 $n: the hive is gone within 500 ms of Esc (daemon trace)" \
      '[ -n "$gone_ms" ] && awk -v d="$gone_ms" "BEGIN{exit !(d >= 0 && d <= 500)}"'
  else
    check "AC11 $n: the drag never entered the hive before Esc" '[ "$entered" -eq 0 ]'
    check "AC11 $n: the hive closes by its timeout (within 12 s of Esc)" \
      '[ -n "$gone_ms" ] && awk -v d="$gone_ms" "BEGIN{exit !(d >= 0 && d <= 12000)}"'
    # S-021: on the layer path the catcher was never revealed, so its 10 s
    # bound prints the one probe line of the show, "hidden (cap)".
    if [ "${ESC_NO_KEYBOARD:-0}" = 1 ]; then
      local probe; probe=$(sed -n "$((DSHOW + 1)),\$p" "$DLOG" | grep -E '^hive (reveal|probe):')
      echo "INFO S-021 AC11 $n: probe lines: $(tr '\n' '|' <<<"$probe")"
      check "S-021 AC11 $n (layer path): the timeout prints exactly one probe line, \"hive probe: hidden (cap)\"" \
        '[ "$probe" = "hive probe: hidden (cap)" ]'
    fi
  fi
  check "AC11 $n: the hive is no longer shown" 'wait_gone 2'
  if [ "${ESC_NO_KEYBOARD:-0}" = 1 ]; then
    check "AC11 $n: Esc never reaches the daemon (no wl_keyboard enter or key in drop mode)" \
      '! grep -qE "^\[[0-9:.]+\] wl_keyboard[#@][0-9]+\.(enter|key)\(" <<<"$(tail -n +$((DSHOW + 1)) "$DLOG")"'
  fi
  # The drag is cancelled and the hive gone: nothing is left that could
  # start a conversion, so one second covers a late write.
  sleep 1
  check "AC11 $n: no file written" '[ "$(find "$W" -type f | sort)" = "$before" ]'
  wait_gone 5 || true
  sleep 1
}

# AC11 in one private compositor session: RUNS shortcut drops (3 of 3), then
# Esc over the hive, then Esc before the hive.
shortcut_shows() {
  local i ok=0
  for i in $(seq "${RUNS:-3}"); do
    cp "$ROOT/tests/fixtures/images/alpha.png" "$W/run $i.png"
    DSHOW=$(wc -l <"$DLOG")
    shortcut_drop "$i" "$W/run $i.png"
    [ "$(file --mime-type -b "$W/run $i.webp" 2>/dev/null)" = image/webp ] && ok=$((ok + 1))
  done
  echo "INFO AC11: $ok of ${RUNS:-3} shortcut drops wrote a WEBP"
  check "AC11: ${RUNS:-3} of ${RUNS:-3} shortcut runs wrote the file" '[ "$ok" -eq "${RUNS:-3}" ]'
  cp "$ROOT/tests/fixtures/images/alpha.png" "$W/esc over.png"
  DSHOW=$(wc -l <"$DLOG")
  shortcut_esc 1 "$W/esc over.png" over
  cp "$ROOT/tests/fixtures/images/alpha.png" "$W/esc before.png"
  DSHOW=$(wc -l <"$DLOG")
  shortcut_esc 2 "$W/esc before.png" before
}

summary() {
  if [ "$FAILS" -eq 0 ]; then echo "RESULT PASS"; else echo "RESULT FAIL ($FAILS)"; fi
}
