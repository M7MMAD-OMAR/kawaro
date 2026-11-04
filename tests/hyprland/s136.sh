# shellcheck shell=bash
# shellcheck disable=SC2016,SC2034,SC2154
# S-136: the Alt-hold drop lifecycle, in the private Hyprland of
# tests/hyprland/repro.sh --s136. Sourced inside its inner(); uses its helpers
# (ptr_move, ptr_up, click, shown, wait_shown, geo, after, seg_from, now_us,
# check, say, start_daemon, stop_daemon, reset_hive) and variables (BIN, HERE,
# ROOT, OUT, W, NEXT, DLOG, SLOG, SRCPID, SURFACE).
#
# Sequence S-136 AC4 names and --altdrag does not cover: a real ALT_L is held
# while a GTK drag is dropped on a ring 1 cell and released after the drop.
#
# HOVER CONTRACT (lead finding 2026-11-04): the receiver reads a Motion only.
# The first Enter arrives before the hive has built its cells, so the first
# hover sees the empty plan and picks nothing; a repeated move to the SAME
# point emits no Motion, so the cell is never hit. Every target here is
# approached with distinct moves (surface corner, mid, target) plus a small
# +/-2 px jitter and a return to the target, after the "hive reveal:" line
# proves the cells are applied.
#
# Cell coordinates: the layer show prints "hive cells: N OX OY" (plan origin
# inside the surface) in the same segment as "hive surface: layer X Y W H", so
# the cell centre is GX + OX + dx, GY + OY + dy with (dx, dy) from lib.sh
# webp_offset (token spacing, rounded). S136_ORIGIN=box switches to the surface
# box centre (S136_ORIGIN=box), for comparison only.
#
# Evidence rules: zero cells in the hint is asserted from the UI model line
# "hive drop hint: cells=0", never from geometry or the daemon "drop cells:"
# line; a missing real Alt bind is a FAILED check; no check is skipped.

ALT=56
S136_WATCH_MS=${S136_WATCH_MS:-15000}
S136_ORIGIN=${S136_ORIGIN:-plan}

if ! declare -F seg_from >/dev/null 2>&1; then
  seg_from() { sed -n "$(($1 + 1)),\$p" "$DLOG"; }
fi
if ! declare -F wait_shown >/dev/null 2>&1; then
  wait_shown() {
    local t0; t0=$(now_us)
    while [ $(( ($(now_us) - t0) / 1000 )) -lt "$1" ]; do shown && return 0; sleep 0.05; done
    shown
  }
fi

s136_config() {
  printf 'language = "en"\ntheme = "light"\nportal_shortcut = false\ndrop_mode = "%s"\n' "$1" \
    >"$XDG_CONFIG_HOME/kawaro/config.toml"
}

s136_surfaces() { seg_from "$1" | grep -c '^hive surface:'; }

s136_outputs() {
  local n=0 f b
  shopt -s nullglob
  for f in "$W"/*; do
    b=${f##*/}
    case "$b" in
      s136.png|photo.png) ;;
      *) n=$((n + 1)) ;;
    esac
  done
  shopt -u nullglob
  echo "$n"
}

# s136_origin MARK: GX GY GW GH from the last show line, OX OY the cell grid
# origin inside the surface (the daemon cells line on layer, box centre on
# window or with S136_ORIGIN=box).
s136_origin() {
  local mark=$1 line n cx cy
  MARK=$1
  GX=0 GY=0 GW=0 GH=0
  read -r GX GY GW GH < <(geo)
  OX=$((GW / 2)); OY=$((GH / 2))
  line=$(seg_from "$mark" | grep '^hive cells: ' | tail -1 || true)
  if [ -n "$line" ] && [ "$S136_ORIGIN" = plan ]; then
    read -r _ _ n OX OY <<<"$line"
    OX=$(awk -v v="$OX" 'BEGIN{printf "%d", (v < 0 ? v - 0.5 : v + 0.5)}')
    OY=$(awk -v v="$OY" 'BEGIN{printf "%d", (v < 0 ? v - 0.5 : v + 0.5)}')
  fi
}

# s136_wait_reveal MARK SECONDS: the hive log line that proves the drop cells
# were applied to the surface.
s136_wait_reveal() {
  local mark=$1 secs=$2 t0
  t0=$(now_us)
  while [ $(( ($(now_us) - t0) / 1000 )) -lt $((secs * 1000)) ]; do
    seg_from "$mark" | grep -q '^hive reveal: ' && return 0
    sleep 0.05
  done
  return 1
}

# s136_hit MARK DX DY: approach the cell with distinct motions and set PX PY.
# Corner, mid, target, +2,+2, -2,-2, target: every step is a real Motion, and
# the last one leaves the pointer on the cell.
s136_hit() {
  local mark=$1 dx=$2 dy=$3
  s136_origin "$mark"
  PX=$((GX + OX + dx)); PY=$((GY + OY + dy))
  # A clamped layer show can leave the plan origin near a surface edge, which
  # puts the first cell outside the surface; take the mirrored cell instead,
  # which is also a ring 1 action cell.
  if [ "$PX" -lt "$GX" ] || [ "$PX" -gt $((GX + GW)) ] || [ "$PY" -lt "$GY" ] || [ "$PY" -gt $((GY + GH)) ]; then
    PX=$((GX + OX + dx)); PY=$((GY + OY - dy))
    say "INFO s136: cell point mirrored inside the surface to $PX,$PY (surface $GX $GY ${GW}x${GH})"
  fi
  ptr_move $((GX + 4)) $((GY + 4)); sleep 0.2
  ptr_move $((GX + GW / 2)) $((GY + GH / 2)); sleep 0.2
  ptr_move "$PX" "$PY"; sleep 0.15
  ptr_move $((PX + 2)) $((PY + 2)); sleep 0.08
  ptr_move $((PX - 2)) $((PY - 2)); sleep 0.08
  ptr_move "$PX" "$PY"; sleep 0.25
}

s136_shot() {
  command -v grim >/dev/null 2>&1 || return 1
  [ "$4" -gt 0 ] && [ "$5" -gt 0 ] || return 1
  grim -g "$2,$3 ${4}x${5}" "$1" 2>/dev/null
}

s136_watch() {
  local mark=$1 want=$2 want_out=$3 want_open=$4 t0 s w seen=no open
  t0=$(now_us)
  while [ $(( ($(now_us) - t0) / 1000 )) -lt "$S136_WATCH_MS" ]; do
    shown && seen=yes
    sleep 0.2
  done
  s=$(s136_surfaces "$mark")
  w=$(s136_outputs)
  if shown; then open=1; else open=0; fi
  say "INFO s136: 15 s watch: surfaces $s (want $want), outputs $w (want $want_out), open $open (want $want_open), seen during the watch: $seen (INFO)"
  check "S-136: 15 s after the release exactly $want hive show(s) and $want_out output(s), no reopen, hive open=$open (want $want_open) (got $s surfaces, $w outputs)" \
    '[ "$s" -eq "$want" ] && [ "$w" -eq "$want_out" ] && [ "$open" = "$want_open" ]'
}

s136_phase() {
  local mode=$1 mark s_before s_after w sha_before sha_after deleted dx dy f b
  local hint_w hint_h hint_cells hint_shot hive_shot px py
  say "== S-136 $mode ($SURFACE, origin $S136_ORIGIN)"
  s136_config "$mode"
  stop_daemon; start_daemon 0
  cp "$ROOT/tests/fixtures/images/alpha.png" "$W/s136.png"
  shopt -s nullglob
  for f in "$W"/*; do
    b=${f##*/}
    case "$b" in
      s136.png|photo.png) ;;
      *) rm -f "$f" ;;
    esac
  done
  shopt -u nullglob
  sha_before=$(sha256sum "$W/s136.png" | cut -d' ' -f1)
  printf '%s\n' "$W/s136.png" >"$NEXT"
  mark=$(wc -l <"$DLOG")

  drag_past_threshold
  echo "keydown $ALT" >&3
  local shown_ok=no show_via=bind
  wait_shown 3000 && shown_ok=yes
  if [ "$shown_ok" = no ]; then
    say "INFO s136 $mode: no held show from the real Alt bind; config ALT_L rows $(grep -c 'ALT_L' "$XDG_CONFIG_HOME/hypr/sbh.lua" 2>/dev/null || echo 0), hyprctl binds ALT_L rows $(hyprctl binds 2>/dev/null | grep -c 'ALT_L' || echo 0)"
    "$BIN/kawaro" drop --hold --at "$(hyprctl cursorpos)" >>"$log-cli.log" 2>&1
    show_via=direct
    wait_shown 2000 && shown_ok=yes
  fi
  check "S-136 $mode: the real Alt bind opens the held drop show (native Alt path)" '[ "$show_via" = bind ] && [ "$shown_ok" = yes ]'
  s136_origin "$mark"
  hint_w=$GW hint_h=$GH
  say "INFO s136 $mode: held show $shown_ok via $show_via at $GX $GY ${hint_w}x${hint_h}, plan origin $OX,$OY"

  local want_hives=1 want_out=1 want_open=0
  read -r dx dy <<<"$(webp_offset)"

  if [ "$mode" = drop-first ]; then
    # Enter the hint (no cells), let it reveal, then drop off its centre.
    s136_hit "$mark" "$dx" "$dy"
    px=$PX py=$PY
    s136_wait_reveal "$mark" 2 || say "INFO s136 drop-first: no reveal line within 2 s"
    s136_hit "$mark" "$dx" "$dy"
    px=$PX py=$PY
    want_hives=2
  else
    # The first Enter arrives before the cells exist: enter, wait for the
    # reveal, then hit the cell with fresh motions.
    s136_hit "$mark" "$dx" "$dy"
    if ! s136_wait_reveal "$mark" 3; then
      say "INFO s136 choose: no reveal line within 3 s before the cell hit"
    fi
    s136_hit "$mark" "$dx" "$dy"
    px=$PX py=$PY
  fi
  sleep 0.4

  hint_cells=$(seg_from "$mark" | grep -c '^hive drop hint: cells=0' || true)
  hint_shot="$OUT/s136-$mode-hint.png"
  s136_origin "$mark"
  s136_shot "$hint_shot" "$GX" "$GY" "$GW" "$GH" || true
  if [ "$mode" = drop-first ]; then
    check "S-136 drop-first: the held hint applied zero action cells (hive drop hint: cells=0 line, got $hint_cells)" '[ "$hint_cells" -ge 1 ]'
  else
    check "S-136 choose-during-drag: no drop-first hint line on the cell drop path (got $hint_cells)" '[ "$hint_cells" -eq 0 ]'
  fi

  s_before=$(s136_surfaces "$mark")
  echo "up" >&3
  sleep 0.5

  if [ "$mode" = drop-first ]; then
    w=$(s136_outputs)
    local handoff=no s2=0 i
    for i in $(seq 60); do
      s2=$(s136_surfaces "$mark")
      [ "$s2" -ge 2 ] && { handoff=yes; break; }
      sleep 0.05
    done
    sleep 0.4
    s136_origin "$mark"
    local ngx=$GX ngy=$GY ngw=$GW ngh=$GH
    hive_shot="$OUT/s136-$mode-handoff.png"
    s136_shot "$hive_shot" "$ngx" "$ngy" "$ngw" "$ngh" || true
    say "INFO s136 drop-first: hint ${hint_w}x${hint_h}, handoff ${ngw}x${ngh} at $ngx $ngy origin $OX,$OY, surfaces $s2"
    check "S-136 drop-first: the drop off the hint centre runs no action and hands off exactly one hive (got $w output(s), 0 expected; surfaces $s2, 2 expected, handoff $handoff)" \
      '[ "$w" -eq 0 ] && [ "$handoff" = yes ] && [ "$s2" -eq 2 ]'
    check "S-136 drop-first: the hint screenshot was captured for visual evidence" '[ -s "$hint_shot" ]'
    echo "keyup $ALT" >&3
    sleep 1
    check "S-136 drop-first: the Alt release after the handoff leaves the normal hive open" 'shown'
    # Wait for the normal show, then approach the cell away-and-back after the
    # release so the click lands on a cell that has a fresh hover.
    sleep 0.5
    s136_hit "$mark" "$dx" "$dy"
    click "$PX" "$PY"
    sleep 0.5
  else
    echo "keyup $ALT" >&3
    sleep 1.5
  fi

  local wcount; wcount=$(s136_outputs)
  s_after=$(s136_surfaces "$mark")
  say "INFO s136 $mode: work files after the phase: $(ls -A "$W" 2>/dev/null | tr '\n' ' ')"
  if [ "$mode" = choose-during-drag ]; then
    check "S-136 choose-during-drag: the cell drop at $px,$py writes exactly one output (got $wcount)" '[ "$wcount" -eq 1 ]'
    check "S-136 choose-during-drag: no handoff and no reopen after the commit (surfaces $s_before before, $s_after after, 1 expected)" '[ "$s_before" -eq 1 ] && [ "$s_after" -eq 1 ]'
  else
    local got=no i2
    for i2 in $(seq 50); do
      [ "$(s136_outputs)" -ge 1 ] && { got=yes; break; }
      sleep 0.1
    done
    wcount=$(s136_outputs)
    s_after=$(s136_surfaces "$mark")
    check "S-136 drop-first: the click at $PX,$PY after the handoff writes exactly one output (got $wcount, file $got)" '[ "$wcount" -eq 1 ] && [ "$got" = yes ]'
    check "S-136 drop-first: no extra hive after the click (surfaces $s_after, 2 expected)" '[ "$s_after" -eq 2 ]'
  fi

  sha_after=$(sha256sum "$W/s136.png" | cut -d' ' -f1)
  deleted=$(grep -c 'drag-end deleted=True' "$SLOG" 2>/dev/null || true)
  check "S-136: the original is untouched (sha256 equal, no move) in $mode" '[ "$sha_before" = "$sha_after" ] && [ "${deleted:-0}" -eq 0 ]'

  s136_watch "$mark" "$want_hives" "$want_out" "$want_open"
  reset_hive "the S-136 $mode phase"
  sleep 0.4
}

s136_run() {
  say "INFO s136: SURFACE $SURFACE, origin mode $S136_ORIGIN, config ALT_L rows $(grep -c 'ALT_L' "$XDG_CONFIG_HOME/hypr/sbh.lua" 2>/dev/null || echo 0), hyprctl binds ALT_L rows $(hyprctl binds 2>/dev/null | grep -c 'ALT_L' || echo 0)"
  say "INFO s136: $(wc -l <"$DLOG") daemon log lines before the run, source pid ${SRCPID:-none}, screenshots to $OUT"
  s136_phase choose-during-drag
  s136_phase drop-first
  say "INFO s136: evidence in $DLOG and $SLOG"
}
