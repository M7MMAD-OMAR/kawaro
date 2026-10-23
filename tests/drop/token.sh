# shellcheck shell=bash
# Conditions are passed to check() in single quotes and expanded by eval.
# shellcheck disable=SC2016,SC2034
# S-014f3 (backlog/S-014-drop-hive.md, AC15 part a): the activation_token of
# the portal's Activated reaches the show, with the fake portal
# (tests/drop/shortcuts_portal.py "ok", control lines "seq" and "signal ...
# token=T"). Sourced by tests/drop/portal.sh (phase ac15, KWin, layer path)
# and by tests/drop/gnome.sh smart portal (GNOME, window path), inside the
# private compositor session only, after tests/drop/lib.sh.
#
# Every case drags a PNG and holds the drag, so the show is a real drop
# hive on both paths (on the layer path a hold with no drag only probes and
# hides); then the fake's timed presses (or `sbarhive drop --hold --token
# X`), the hive, a drop on the centre cell (no file, the hive closes), and
# only then the release, as a key still held over the drop.
#
# The observable is the daemon's WAYLAND_DEBUG trace: every
#   -> xdg_activation_v1#N.activate("TOKEN", wl_surface#M)
# between the case's mark and the hive's close. Each case needs, as its
# precondition, a "portal: activated drop" line (the fake cases) and a hive
# surface in the same segment, so a case where nothing was shown cannot
# pass on zero activate.
#
# The caller defines, for its compositor:
#   fake LINE          sends LINE to the fake's control fifo
#   hive_shows SEG     the number of hive surfaces made in a daemon log
#                      segment (KWin: get_layer_surface; GNOME: "hive
#                      surface: window" lines)
#   token_motion       what the drag does right before the presses (KWin:
#                      jitter around (400, 500), the hold path's anchor
#                      needs motion; GNOME: nothing)
#   token_settle       waits for the hive to be gone between cases
# and sets BIN, ROOT, W, NEXT, DLOG.

ACT_RE='-> xdg_activation_v1[#@][0-9]+\.activate\("[^"]*", '

# activates SEG: the token of every activate request in SEG, one per line.
activates() { grep -oE -- "$ACT_RE" <<<"$1" | sed -E 's/.*activate\("([^"]*)", $/\1/'; }

# token_case NAME HOW WANT
#   HOW   "seq STEP..." for the fake (STEP as shortcuts_portal.py: MS:A,
#         MS:A:TOKEN, MS:D), or "cli TOKEN" for `drop --hold --token TOKEN`
#   WANT  the one token the show must activate with, or "none" for zero
#         activate requests
token_case() {
  local name=$1 how=$2 want=$3 file="$W/token $4.png"
  echo "== AC15 (a) $name"
  cp "$ROOT/tests/fixtures/images/alpha.png" "$file"
  printf '%s\n' "$file" >"$NEXT"
  drag_past_threshold
  local mark; mark=$(wc -l <"$DLOG")
  token_motion
  case "$how" in
    seq\ *) fake "seq drop own ${how#seq }" ;;
    cli\ *) env -u XDG_ACTIVATION_TOKEN -u DESKTOP_STARTUP_ID "$BIN/sbarhive" drop --hold --token "${how#cli }" >>"$OUT_LOG-cli.log" 2>&1 ;;
  esac
  local geo; geo=$(wait_geo 4)
  sleep 0.8
  local g2; g2=$(hive_geo); [ -n "$g2" ] && geo=$g2
  if [ -n "$geo" ]; then
    local gx gy gw gh
    read -r gx gy gw gh <<<"$geo"
    ptr_move $((gx + gw / 2)) $((gy + gh / 2)); sleep 0.3
  fi
  ptr_up
  sleep 0.5
  case "$how" in
    seq\ *) fake "signal Deactivated drop" ;;
    cli\ *) "$BIN/sbarhive" drop --release >>"$OUT_LOG-cli.log" 2>&1 ;;
  esac
  sleep 1
  token_settle
  local seg na hs toks n
  seg=$(tail -n +$((mark + 1)) "$DLOG")
  na=$(grep -cE '^portal: activated drop$' <<<"$seg")
  hs=$(hive_shows "$seg")
  toks=$(activates "$seg")
  n=$(grep -c . <<<"$toks")
  echo "INFO AC15 (a) $name: portal activated lines $na; hive surfaces $hs (at ${geo:-not shown}); activate requests $n: $(tr '\n' ' ' <<<"${toks:-none}")"
  local pre='[ "$hs" -ge 1 ]'
  [ "${how%% *}" = seq ] && pre='[ "$na" -ge 1 ] && [ "$hs" -ge 1 ]'
  if [ "$want" = none ]; then
    check "AC15 (a) $name: zero xdg_activation_v1.activate in that show (precondition: the portal: activated drop line and a hive surface)" \
      "$pre"' && [ "$n" -eq 0 ]'
  else
    check "AC15 (a) $name: exactly one xdg_activation_v1.activate(\"$want\", ...) in that show and no other (precondition: $([ "${how%% *}" = seq ] && echo 'the portal: activated drop line and ')a hive surface)" \
      "$pre"' && [ "$n" -eq 1 ] && [ "$toks" = "$want" ]'
  fi
}

# The five cases of AC15 (a), hold_ms 400 in the caller's config.
token_cases() { # token_cases DESKTOP
  local desk=$1
  token_case "$desk fake-A" "seq 0:A:fake-A" fake-A a
  token_case "$desk fake-A then fake-B 100 ms later" "seq 0:A:fake-A 100:A:fake-B" fake-B b
  token_case "$desk fake-A then a press with no token" "seq 0:A:fake-A 100:A" fake-A c
  token_case "$desk a fake-A tap released at 150 ms, then a held press with no token" "seq 0:A:fake-A 150:D 300:A" none d
  token_case "$desk drop --hold --token X" "cli X" X e
}
