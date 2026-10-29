# shellcheck shell=bash
# shellcheck disable=SC2016,SC2034,SC2154
# S-095: real Alt+drag runs in the private Hyprland of tests/hyprland/repro.sh
# --altdrag. Sourced inside its inner(); uses its helpers (move, ptr_down,
# ptr_up, shown, now_us, check, say, start_daemon, stop_daemon, start_keys)
# and variables (BIN, OUT, W, NEXT, DLOG, KLOG, ALTN, HYPR_HANG_MS).
#
# Per drag, in order "alt-first" (Alt down, then the drag begins; the gap is
# 50 ms on odd drags, before hold_ms, and 600 ms on even ones, after it) and
# "drag-first" (the drag begins, Alt goes down 300 ms later):
#   outcome  shown    the layer `kawaro` is listed within 1500 ms of the later
#                     of "hold_ms elapsed" and "drag entered"
#            nothing  it is not
#   then the Alt key goes up, 300 ms later the button; hang = the layer is
#   still listed HYPR_HANG_MS (default 15000) after the Alt release.
# Result file $OUT/altdrag-results.tsv, one row per drag:
#   order n gap_ms outcome show_ms gone_ms hang
# show_ms: ms from the later of the two events to the layer listed (-1 if
# never); gone_ms: ms from the Alt release to the layer gone (-1 on hang).

ALT=56 TABKEY=15 AKEY=30
HOLD_MS=400

# One drag in ORDER (alt-first|drag-first), number N. Appends a row.
altdrag_one() {
  local order=$1 n=$2 gap=0 t_alt t_drag t_ref mark shown_ms=-1 gone_ms=-1 hang=no outcome=nothing t i
  local lg="$OUT/altdrag-$order-$n.log"
  mark=$(wc -l <"$DLOG")
  printf '%s\n' "$W/ad.png" >"$NEXT"
  if [ "$order" = alt-first ]; then
    [ $((n % 2)) = 1 ] && gap=50 || gap=600
    echo "keydown $ALT" >&3; t_alt=$(now_us)
    sleep "0.$(printf '%03d' "$gap")"
    ptr_move 150 150; sleep 0.2; ptr_down; sleep 0.2
    for i in 1 2 3 4 5 6; do ptr_move $((150 + i * 10)) $((150 + i * 5)); sleep 0.05; done
    t_drag=$(now_us)
  else
    ptr_move 150 150; sleep 0.2; ptr_down; sleep 0.2
    for i in 1 2 3 4 5 6; do ptr_move $((150 + i * 10)) $((150 + i * 5)); sleep 0.05; done
    t_drag=$(now_us)
    sleep 0.3
    echo "keydown $ALT" >&3; t_alt=$(now_us)
  fi
  t_ref=$t_drag
  [ $((t_alt + HOLD_MS * 1000)) -gt "$t_ref" ] && t_ref=$((t_alt + HOLD_MS * 1000))
  # Wait for the layer until 1500 ms after the reference.
  while [ $(( ($(now_us) - t_ref) / 1000 )) -lt 1500 ]; do
    if shown; then shown_ms=$(( ($(now_us) - t_ref) / 1000 )); outcome=shown; break; fi
    sleep 0.02
  done
  hyprctl layers >"$lg" 2>&1
  { echo "--- daemon log"; sed -n "$((mark + 1)),\$p" "$DLOG" | grep -v '^\['; } >>"$lg"
  sleep 0.2
  echo "keyup $ALT" >&3; t=$(now_us)
  # The button goes up 300 ms after the key; the layer is polled meanwhile,
  # so gone_ms is the time from the Alt release, not from the button.
  while [ $(( ($(now_us) - t) / 1000 )) -lt 300 ]; do
    shown || { gone_ms=$(( ($(now_us) - t) / 1000 )); break; }
    sleep 0.02
  done
  ptr_up
  local hang_ms=${HYPR_HANG_MS:-15000}
  while [ "$gone_ms" = -1 ] && [ $(( ($(now_us) - t) / 1000 )) -lt "$hang_ms" ]; do
    shown || { gone_ms=$(( ($(now_us) - t) / 1000 )); break; }
    sleep 0.02
  done
  if [ "$gone_ms" = -1 ]; then
    if shown; then hang=yes; else gone_ms=$(( ($(now_us) - t) / 1000 )); fi
  fi
  { echo "--- after release"; hyprctl layers; sed -n "$((mark + 1)),\$p" "$DLOG" | grep -v '^\['; } >>"$lg" 2>&1
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$order" "$n" "$gap" "$outcome" "$shown_ms" "$gone_ms" "$hang" >>"$RES"
  if [ "$hang" = yes ]; then
    say "INFO altdrag $order $n: HANG, restarting the daemon"
    stop_daemon; start_daemon 0
  fi
  sleep 0.6
}

altdrag_run() {
  RES="$OUT/altdrag-results.tsv"
  : >"$RES"
  cp "$ROOT/tests/fixtures/images/alpha.png" "$W/ad.png"
  stop_daemon; start_daemon 0
  local order n
  for order in alt-first drag-first; do
    for n in $(seq "$ALTN"); do altdrag_one "$order" "$n"; done
  done
  say "INFO altdrag results: $RES"
  local o
  for o in alt-first drag-first; do
    say "INFO altdrag $o: shown $(awk -F'\t' -v o="$o" '$1==o && $4=="shown"' "$RES" | wc -l), nothing $(awk -F'\t' -v o="$o" '$1==o && $4=="nothing"' "$RES" | wc -l), hang $(awk -F'\t' -v o="$o" '$1==o && $7=="yes"' "$RES" | wc -l), show_ms $(awk -F'\t' -v o="$o" '$1==o && $5>=0 {printf "%s ", $5}' "$RES"), gone_ms $(awk -F'\t' -v o="$o" '$1==o && $6>=0 {printf "%s ", $6}' "$RES")"
    say "INFO altdrag $o by gap: $(awk -F'\t' -v o="$o" '$1==o {c[$3" "$4]++} END {for (k in c) printf "[gap %s: %d] ", k, c[k]}' "$RES")"
  done

  # AC1: a row per drag, the three counts add up.
  check "S-095 AC1: $((2 * ALTN)) rows, shown + nothing + hang accounted" \
    '[ "$(wc -l <"$RES")" -eq $((2 * ALTN)) ] && [ $(( $(awk -F"\t" "\$4==\"shown\"" "$RES" | wc -l) + $(awk -F"\t" "\$4==\"nothing\"" "$RES" | wc -l) )) -eq $((2 * ALTN)) ]'
  for o in alt-first drag-first; do
    # AC2: every drag shows the hive, within 150 ms of the later event.
    check "S-095 AC2 ($o): $ALTN of $ALTN drags show the hive within 150 ms of the later of hold_ms and drag entered" \
      '[ "$(awk -F"\t" -v o="$o" "\$1==o && \$4==\"shown\" && \$5<=150" "$RES" | wc -l)" -eq "$ALTN" ]'
    # AC3: no hang, the layer is gone within 500 ms of the Alt release.
    check "S-095 AC3 ($o): 0 hangs, the layer is gone within 500 ms of the release in every drag" \
      '[ "$(awk -F"\t" -v o="$o" "\$1==o && \$7==\"yes\"" "$RES" | wc -l)" -eq 0 ] && [ "$(awk -F"\t" -v o="$o" "\$1==o && \$6>=0 && \$6<=500" "$RES" | wc -l)" -eq "$ALTN" ]'
  done

  # AC5 (harness half): no Alt left held, a plain key reaches the client
  # with no modifier. keys.py logs "keys: key NAME CODE mods=STATE".
  kill "$SRCPID" 2>/dev/null; sleep 0.5
  start_keys; sleep 2
  local tries; for tries in 1 2 3 4 5 6; do
    click 100 100; sleep 0.6
    grep -q '^keys: active True' "$KLOG" && break
  done
  local k0; k0=$(wc -l <"$KLOG")
  key $AKEY; sleep 0.5
  check "S-095 AC5: after the drags a plain a reaches the client with no modifier (1 of 1)" \
    'sed -n "$((k0 + 1)),\$p" "$KLOG" | grep -qE "^keys: key a [0-9]+ mods=$"'
  check "S-095 AC5: no hive layer and the daemon answers after the run" '! shown && "$BIN/kawaro" ping >/dev/null 2>&1'

  # AC7 (harness half): a bare Alt tap shows nothing; Alt+Tab reaches the client.
  local tap_seen=no t0
  t0=$(now_us)
  echo "keydown $ALT" >&3; sleep 0.15; echo "keyup $ALT" >&3
  while [ $(( ($(now_us) - t0) / 1000 )) -lt 1000 ]; do shown && tap_seen=yes; sleep 0.05; done
  check "S-095 AC7: a bare Alt tap shows nothing (0 layers, 1 of 1)" '[ "$tap_seen" = no ]'
  k0=$(wc -l <"$KLOG")
  echo "keydown $ALT" >&3; echo "sleep 40" >&3; key $AKEY; echo "keyup $ALT" >&3; sleep 0.5
  check "S-095 AC7: a combination with Alt (Alt+a) reaches the client with the Alt modifier (1 of 1); Alt+Tab, Alt+F4 and the Super drag binds are not covered here" \
    'sed -n "$((k0 + 1)),\$p" "$KLOG" | grep -qE "^keys: key a [0-9]+ mods=ALT"'

  # S-095 re-show limits. (a) Esc with Alt held: no new hive surface after
  # the Esc. (b) 2 s of continuous pointer motion with Alt held: at most
  # 1 + 4 = 5 "hive surface" lines (the first show and REARM_MAX re-shows).
  local m surf
  # Alt+a above loses the Alt release (Hyprland fires a release bind only
  # when no other key came between), so the gate may hold a stale press: one
  # plain tap releases it before the Esc case starts.
  echo "keydown $ALT" >&3; sleep 0.05; echo "keyup $ALT" >&3; sleep 1.2
  m=$(wc -l <"$DLOG")
  echo "keydown $ALT" >&3; sleep 0.8
  key 1; sleep 1
  surf=$(sed -n "$((m + 1)),\$p" "$DLOG" | grep -c '^hive surface:')
  echo "keyup $ALT" >&3; sleep 0.6
  say "INFO S-095 Esc with Alt held: hive surface lines $surf"
  check "S-095 re-show: Esc with Alt held shows nothing again (1 hive surface line in all, got $surf)" '[ "$surf" -eq 1 ]'
  m=$(wc -l <"$DLOG")
  echo "keydown $ALT" >&3; sleep 0.6
  for i in $(seq 100); do ptr_move $((200 + (i % 2) * 6)) $((200 + (i % 3) * 4)); sleep 0.02; done
  surf=$(sed -n "$((m + 1)),\$p" "$DLOG" | grep -c '^hive surface:')
  echo "keyup $ALT" >&3; sleep 0.6
  say "INFO S-095 2 s of pointer motion with Alt held: hive surface lines $surf (bound 5)"
  check "S-095 re-show: 2 s of continuous pointer motion with Alt held gives at most 5 hive surface lines (got $surf)" '[ "$surf" -le 5 ] && [ "$surf" -ge 1 ]'
}
