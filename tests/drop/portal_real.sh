# shellcheck shell=bash
# Conditions are passed to check() in single quotes and expanded by eval.
# shellcheck disable=SC2016,SC2034
# S-014e2 (backlog/S-014-drop-hive.md, AC18 part c, AC20 for GNOME and KDE,
# AC21): the "portal" and "portal-real" modes of tests/drop/gnome.sh and
# tests/drop/kwin.sh. Sourced inside the private compositor session only,
# after tests/drop/lib.sh.
#
# portal       (gnome.sh only) the fake tests/drop/shortcuts_portal.py "ok"
#              owns org.freedesktop.portal.Desktop on the private bus; the
#              /bin/false service of tests/no_portal.sh stays, so the
#              installed portal can never start. AC18 (c).
# portal-real  the installed xdg-desktop-portal and its backend
#              (xdg-desktop-portal-gnome, with gnome-control-center's
#              global shortcuts provider for the dialog, or
#              xdg-desktop-portal-kde), D-Bus activated by the private bus
#              (dbus-run-session runs the service file's Exec directly, never
#              the systemd unit), with XDG_CURRENT_DESKTOP GNOME or KDE set
#              before the bus starts. The only place where the real portal
#              runs (the S-014e1 harness rule of 984bb22): a private
#              headless compositor, a private session bus and a throwaway
#              HOME, never the live desktop or its bus. The shipped
#              packaging/io.github.m7mmad_omar.SbarHive.desktop is copied to
#              the throwaway XDG_DATA_HOME/applications, as an install puts
#              it in a data directory. AC20, AC21.
#
# The daemon's output goes through `real_portal.py stamp`: DLOG is the plain
# log, as in the other modes, and $DLOG.stamps has every "portal..." line
# with its UTC arrival stamp (AC20 (b) reads those). `dbus-monitor` on the
# private bus logs the Activated and Deactivated signals of the backend
# (org.freedesktop.impl.portal.GlobalShortcuts) and of the frontend
# (org.freedesktop.portal.GlobalShortcuts), so a missing Deactivated can be
# told apart: never sent by the backend, or not forwarded.
#
# The caller defines, for its compositor:
#   key_down, key_up   press and hold Super+Z (Super, then Z 50 ms later),
#                      and release it (Z, then Super 50 ms later), through
#                      the compositor's input path, timed by the shell
#   hold_motion        what the drag does right after key_down (KWin: a
#                      1 px jitter at (400, 500), the hold path's anchor
#                      needs motion; GNOME: nothing)
#   dialog_now         prints "X Y W H|PID|CLASS|TITLE" of the first mapped
#                      window that is neither the daemon's ($d) nor the drag
#                      source's ($src), or nothing
#   dialog_raise PID   raises and activates PID's window, so the drag source
#                      window (full screen under GNOME) cannot cover it
#   accept_click X Y W H   clicks the accepting button of the backend dialog
#                      (GNOME "Add", top right; KDE "OK", bottom right)
#   accept_key         Return through the compositor's input path
#   watch_start        starts recording every window the compositor creates
#   watch_read         prints "PID|CLASS|TITLE" per window created since
#                      watch_start
#   hive_shows SEG     the number of hive surfaces made in a daemon log
#                      segment (KWin: get_layer_surface; GNOME: "hive
#                      surface: window" lines)
#   screen_shot PATH   (gnome.sh, AC18 (c)) a screenshot of the private shell
#   screen_middle      (gnome.sh, AC18 (c)) "CX CY", the centre of the work
#                      area of the monitor, where Mutter centres a window
#                      (below the top bar)

PR="$HERE/real_portal.py"
PORTAL_PROCS='xdg-desktop-portal|xdg-permission-store|xdg-document-portal|global-shortcuts-provider'

# PID COMM of every portal process on this private bus (by its environment).
bus_procs() {
  local p
  for p in $(pgrep -f "$PORTAL_PROCS"); do
    tr '\0' '\n' <"/proc/$p/environ" 2>/dev/null | grep -qxF "DBUS_SESSION_BUS_ADDRESS=$DBUS_SESSION_BUS_ADDRESS" \
      && echo "$p $(cat "/proc/$p/comm" 2>/dev/null)"
  done
}
# Sorted PIDs of the xdg-desktop-portal processes that are not on this bus:
# the live desktop's, which must be the same before and after.
live_portal_pids() {
  local p
  for p in $(pgrep -f '^/usr/libexec/xdg-desktop-portal'); do
    tr '\0' '\n' <"/proc/$p/environ" 2>/dev/null | grep -qxF "DBUS_SESSION_BUS_ADDRESS=$DBUS_SESSION_BUS_ADDRESS" || echo "$p"
  done | sort -n | tr '\n' ' '
}
pkg_version() { rpm -q --qf '%{VERSION}' "$1" 2>/dev/null || echo unknown; }

# Before the first daemon: the monitor, the window watch, and in mode
# "portal" the fake.
portal_setup() {
  LIVE0=$(live_portal_pids)
  MONLOG="$OUT_LOG-monitor.log"
  dbus-monitor --session \
    "type='signal',interface='org.freedesktop.portal.GlobalShortcuts'" \
    "type='signal',interface='org.freedesktop.impl.portal.GlobalShortcuts'" \
    "type='signal',interface='org.freedesktop.portal.Request',member='Response'" >"$MONLOG" 2>&1 &
  MON=$!
  FAKE_PID=""
  if [ "$MODE" = portal ]; then
    PLOG="$OUT_LOG-portal.log"
    local fifo="$XDG_RUNTIME_DIR/portal-fifo"
    mkfifo "$fifo"
    python3 -I "$HERE/shortcuts_portal.py" "$fifo" ok >"$PLOG" 2>&1 &
    FAKE_PID=$!
    exec 4>"$fifo"
    for _ in $(seq 50); do grep -q '\] owned org.freedesktop.portal.Desktop' "$PLOG" && break; sleep 0.1; done
  fi
  watch_start
  sleep 0.5
}
fake() { echo "$*" >&4; }

# portal_start_daemon NAME: the daemon, English, light, reduced motion, with
# its log through the stamper. Sets d, DLOG and STAMPS.
portal_start_daemon() {
  DLOG="$OUT_LOG-$1-daemon.log"; STAMPS="$DLOG.stamps"
  : >"$DLOG"; : >"$STAMPS"
  SBARHIVE_LANG=en SBARHIVE_COLOR_SCHEME=light SBARHIVE_REDUCED_MOTION=1 \
    SBARHIVE_DEBUG_FOCUS=1 WAYLAND_DEBUG=1 "$BIN/sbarhived" > >(python3 -I "$PR" stamp "$DLOG" "$STAMPS") 2>&1 &
  d=$!
  for _ in $(seq 50); do "$BIN/sbarhive" ping >/dev/null 2>&1 && break; sleep 0.1; done
}
portal_stop_daemon() {
  "$BIN/sbarhive" quit >/dev/null 2>&1
  timeout 5 tail --pid="$d" -f /dev/null || kill "$d" 2>/dev/null
  sleep 0.5
}
wait_bound() { # wait_bound SECONDS: the daemon printed a "portal shortcuts:" line
  local end; end=$(deadline "$1")
  while [ "$(now_ms)" -lt "$end" ]; do
    grep -qE '^portal shortcuts: ' "$DLOG" && return 0
    sleep 0.1
  done
  return 1
}
bound_lines() { grep -E '^portal (shortcuts|registry):' "$DLOG"; }
# Windows created since watch line MARK, owned by neither the daemon nor the
# drag source.
others_since() { # others_since MARK
  watch_read | tail -n +$(($1 + 1)) | awk -F'|' -v a="$d" -v b="${src:-0}" '$1 != a && $1 != b'
}

# One AC20 (a) run: drag FILE, press and hold Super+Z until the hive is
# shown, drop on the WEBP cell with the key still held, then release it.
# Sets REAL_OK to yes when the WEBP was written.
real_drop_cycle() { # real_drop_cycle NAME FILE
  local name=$1 file=$2 base="${2%.*}" smark sha geo wrote=no mime src_seg mark
  REAL_OK=no
  echo "== $name: $(basename "$file"), Super+Z held mid-drag, drop on WEBP, then release"
  printf '%s\n' "$file" >"$NEXT"
  smark=$(wc -l <"$SLOG")
  sha=$(sha256sum "$file" | cut -d' ' -f1)
  drag_past_threshold
  mark=$(wc -l <"$DLOG")
  key_down
  hold_motion
  geo=$(wait_geo 4)
  if [ -z "$geo" ]; then
    key_up; ptr_up; sleep 1
    echo "INFO $name: no hive within 4 s of the press; daemon lines: $(tail -n +$((mark + 1)) "$DLOG" | grep -E '^(portal|hive)' | tr '\n' '|')"
    check "$name: the hive appears while Super+Z is held mid-drag" 'false'
    wait_gone 12 || true
    return
  fi
  sleep 1
  local g2; g2=$(hive_geo); [ -n "$g2" ] && geo=$g2
  # shellcheck disable=SC2086
  move_to_webp $geo
  ptr_up
  wait_file "$base.webp" 5 && wrote=yes
  key_up
  sleep 0.5
  mime=$(file --mime-type -b "$base.webp" 2>/dev/null)
  src_seg=$(tail -n +$((smark + 1)) "$SLOG")
  echo "INFO $name: hive at $geo; $(basename "$base").webp ${mime:-not written}; daemon lines: $(tail -n +$((mark + 1)) "$DLOG" | grep -E '^(portal|hive (surface|reveal|probe|anchor))' | sort | uniq -c | awk '{ $1 = $1 "x"; print }' | tr '\n' '|')"
  check "$name: $(basename "$base").webp written within 5 s of the drop (image/webp), the key still held" \
    '[ "$wrote" = yes ] && [ "$mime" = image/webp ]'
  check "$name: the source reports deleted=False and the original's sha256 is unchanged" \
    'grep -q "drag-end deleted=False" <<<"$src_seg" && [ "$(sha256sum "$file" | cut -d" " -f1)" = "$sha" ]'
  [ "$wrote" = yes ] && [ "$mime" = image/webp ] && REAL_OK=yes
  wait_gone 5 || { ptr_move 1000 700; sleep 0.5; wait_gone 5; } || true
  sleep 1
}

# AC20 and AC21 with the real portal, on one compositor.
real_portal_run() { # real_portal_run DESKTOP BACKEND_PACKAGE
  local desk=$1 pkg=$2
  echo "INFO AC20 $desk: xdg-desktop-portal $(pkg_version xdg-desktop-portal), $pkg $(pkg_version "$pkg")$([ "$desk" = GNOME ] && echo ", gnome-control-center $(pkg_version gnome-control-center)"), XDG_CURRENT_DESKTOP=$XDG_CURRENT_DESKTOP"

  # --- AC21, first daemon on the fresh profile ---------------------------
  echo "== AC21 run 1: first daemon on a fresh private profile"
  local end dlg="" seen="" accepted=n/a
  end=$(deadline 20)
  while [ "$(now_ms)" -lt "$end" ]; do
    dlg=$(dialog_now)
    [ -n "$dlg" ] && { seen=$dlg; break; }
    grep -qE '^portal shortcuts: ' "$DLOG" && break
    sleep 0.2
  done
  if [ -n "$seen" ]; then
    # A click right after the map can be lost; try a click, then Return,
    # up to three times, and record which one closed the dialog.
    local gx gy gw gh try cur
    accepted=no
    for try in 1 2 3; do
      sleep 2
      cur=$(dialog_now); [ -n "$cur" ] || { accepted="${accepted/no/gone}"; break; }
      dialog_raise "$(cut -d'|' -f2 <<<"$cur")"
      sleep 0.5
      cur=$(dialog_now); [ -n "$cur" ] || continue
      read -r gx gy gw gh <<<"${cur%%|*}"
      accept_click "$gx" "$gy" "$gw" "$gh"
      sleep 2
      [ -z "$(dialog_now)" ] && { accepted="click (try $try)"; break; }
      accept_key
      sleep 2
      [ -z "$(dialog_now)" ] && { accepted="Return (try $try)"; break; }
    done
  fi
  wait_bound 10 || true
  local procs1; procs1=$(bus_procs)
  local added; added=$(others_since 0)
  echo "INFO AC21 run 1: the backend dialog appeared: $([ -n "$seen" ] && echo "yes (${seen#*|})" || echo no); accepted by ${accepted}; windows from others since the daemon started: $(tr '\n' ';' <<<"${added:-none}")"
  echo "INFO AC20 $desk: portal processes on the private bus: $(tr '\n' ';' <<<"$procs1")"
  local lines t1
  lines=$(bound_lines)
  t1=$(sed -nE 's/^portal shortcuts: bound drop \((.+)\)$/\1/p' <<<"$lines")
  echo "INFO AC20 $desk: daemon portal lines: $(tr '\n' '|' <<<"${lines:-none}"); trigger_description \"${t1:-none}\" for preferred_trigger LOGO+z"
  check "setup $desk: the installed xdg-desktop-portal runs on this private bus (activated here, not through the live session's systemd)" \
    'grep -qE "^[0-9]+ xdg-desktop-por" <<<"$procs1"'
  check "AC21 run 1: any backend dialog is accepted with injected input (none left open)" '[ "$accepted" != no ]'
  check "AC20 $desk: the daemon prints \"portal shortcuts: bound drop (T)\" exactly once, and no other \"portal shortcuts:\" line" \
    '[ -n "$t1" ] && [ "$(grep -c "^portal shortcuts:" <<<"$lines")" -eq 1 ]'

  # --- AC20 (a) ----------------------------------------------------------
  local i ok=0
  for i in 1 2 3; do
    cp "$ROOT/tests/fixtures/images/alpha.png" "$W/held $i.png"
    real_drop_cycle "AC20 (a) $desk run $i" "$W/held $i.png"
    [ "$REAL_OK" = yes ] && ok=$((ok + 1))
  done
  echo "INFO AC20 (a) $desk: $ok of 3 held drops wrote the file"
  check "AC20 (a) $desk: 3 of 3 drags with Super+Z held, dropped on WEBP with the key held, wrote the file" '[ "$ok" -eq 3 ]'

  # --- AC20 (b) ----------------------------------------------------------
  echo "== AC20 (b) $desk: no drag, Super+Z held 600 ms"
  ptr_move 100 700; sleep 0.5
  local tp trel act deact held after nact sigs
  tp=$(date -u +%T.%6N)
  key_down; sleep 0.6
  trel=$(date -u +%T.%6N)
  key_up; sleep 1.5
  read -r act deact held after nact <<<"$(python3 -I "$PR" hold "$STAMPS" "$tp" "$trel")"
  sigs=$(python3 -I "$PR" signals "$MONLOG" | awk -v t="$tp" '$1 >= t' | head -40)
  echo "INFO AC20 (b) $desk: press $tp, release $trel; daemon: activated lines before the deactivated $nact, deactivated $held ms after the first activated and $after ms after the release"
  echo "INFO AC20 (b) $desk: signals on the bus since the press (UTC, side, member, id): $(tr '\n' ';' <<<"${sigs:-none}")"
  check "AC20 (b) $desk: \"portal: deactivated drop\" comes 500 ms or more after \"portal: activated drop\" (the backend sends Deactivated at the release, not at once)" \
    '[ "$held" != none ] && awk -v d="$held" "BEGIN { exit !(d >= 500) }"'
  check "AC20 (b) $desk: and within 200 ms of the key release" \
    '[ "$after" != none ] && awk -v d="$after" "BEGIN { exit !(d >= 0 && d <= 200) }"'
  wait_gone 12 || true
  sleep 1

  # --- AC20 (c) ----------------------------------------------------------
  echo "== AC20 (c) $desk: a 150 ms tap on Super+Z"
  local mark seg n na nd
  mark=$(wc -l <"$DLOG")
  key_down; sleep 0.15; key_up; sleep 1.0
  seg=$(tail -n +$((mark + 1)) "$DLOG")
  n=$(hive_shows "$seg")
  na=$(grep -cE '^portal: activated drop$' <<<"$seg")
  nd=$(grep -cE '^portal: deactivated drop$' <<<"$seg")
  echo "INFO AC20 (c) $desk: activated $na, deactivated $nd; hive surfaces made $n"
  check "AC20 (c) $desk: both signals reach the daemon and no hive surface is made within 1000 ms ($([ "$desk" = GNOME ] && echo 'no hive window' || echo 'no get_layer_surface'))" \
    '[ "$na" -ge 1 ] && [ "$nd" -ge 1 ] && [ "$n" -eq 0 ] && [ -z "$(hive_geo)" ]'
  wait_gone 12 || true

  # --- AC21, second daemon on the same profile ---------------------------
  echo "== AC21 run 2: a new daemon on the same profile, the portal restarted"
  portal_stop_daemon
  # As at a new login: the frontend, the backend and its helpers end too and
  # are activated again by the new daemon, so a consent kept only in their
  # memory does not count.
  local p
  for p in $(bus_procs | cut -d' ' -f1); do kill "$p" 2>/dev/null; done
  local end2; end2=$(deadline 5)
  while [ "$(now_ms)" -lt "$end2" ] && [ -n "$(bus_procs)" ]; do sleep 0.2; done
  local procs_mid; procs_mid=$(bus_procs)
  local wmark; wmark=$(watch_read | grep -c .)
  portal_start_daemon run2
  sleep 5
  local new2 dlg2 lines2 procs2
  new2=$(others_since "$wmark")
  # Open now and owned by a portal process of this bus (a notification
  # popup of an earlier conversion, mako under KWin, is not the backend's).
  dlg2=$(dialog_now)
  local dlg2pid=""; [ -n "$dlg2" ] && dlg2pid=$(cut -d'|' -f2 <<<"$dlg2")
  [ -n "$dlg2pid" ] && ! bus_procs | cut -d' ' -f1 | grep -qxF "$dlg2pid" && dlg2="${dlg2} (not a portal process)"
  lines2=$(bound_lines)
  procs2=$(bus_procs)
  echo "INFO AC21 run 2: windows from others within 5 s: $(tr '\n' ';' <<<"${new2:-none}"); open now: ${dlg2:-none}; daemon portal lines: $(tr '\n' '|' <<<"${lines2:-none}")"
  echo "INFO AC21 run 2: portal processes left after the stop: $(tr '\n' ';' <<<"${procs_mid:-none}"); after the second daemon's start: $(tr '\n' ';' <<<"${procs2:-none}")"
  check "AC21 run 2 setup: the portal processes of run 1 were ended and the second daemon activated new ones" \
    '[ -z "$procs_mid" ] && grep -qE "^[0-9]+ xdg-desktop-por" <<<"$procs2"'
  check "AC21 run 2: no new window from the backend within 5 s of the second daemon's start, and it prints \"portal shortcuts: bound drop\"" \
    '[ -z "$new2" ] && { [ -z "$dlg2" ] || [[ "$dlg2" == *"(not a portal process)" ]]; } && grep -qE "^portal shortcuts: bound drop \(.+\)$" <<<"$lines2"'
  check "AC21 run 2: exactly one \"portal shortcuts:\" line, the bound one" \
    '[ "$(grep -c "^portal shortcuts:" <<<"$lines2")" -eq 1 ] && grep -qE "^portal shortcuts: bound drop \(.+\)$" <<<"$lines2"'
  cp "$ROOT/tests/fixtures/images/alpha.png" "$W/again.png"
  real_drop_cycle "AC21 run 2" "$W/again.png"
  check "AC21 run 2: the shortcut shows the hive and the drop writes the file, 1 of 1" '[ "$REAL_OK" = yes ]'
}

pxcount() { python3 -I "$HERE/pixels.py" count "$1" "$2" "$3" "$4" "$5" "$ROOT/design/tokens.json" light "$6" "$7"; }

# AC18 (c): the fake portal on GNOME's window path. The drag is held, the
# fake sends Activated, the hive is a centred window with the waiting text,
# the drop on WEBP comes before Deactivated.
fake_window_case() {
  local lines; lines=$(bound_lines)
  local owner
  owner=$(gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
    --method org.freedesktop.DBus.GetConnectionUnixProcessID org.freedesktop.portal.Desktop 2>&1 | grep -oE '[0-9]+' | tail -1)
  echo "INFO AC18 (c): org.freedesktop.portal.Desktop owned by pid ${owner:-none} (fake $FAKE_PID); daemon lines: $(tr '\n' '|' <<<"${lines:-none}")"
  check "AC18 (c) setup: the fake owns org.freedesktop.portal.Desktop (no installed portal)" '[ -n "$owner" ] && [ "$owner" = "$FAKE_PID" ]'
  check "AC18 (c) setup: the daemon bound the shortcut on the fake (\"portal shortcuts: bound drop (Fake Super+Z)\")" \
    '[ "$lines" = "portal shortcuts: bound drop (Fake Super+Z)" ]'

  echo "== AC18 (c): drag, Activated, the waiting window, drop on WEBP, Deactivated"
  local file="$W/portal window.png" base smark sha mark geo seg wrote=no mime
  base="${file%.*}"
  cp "$ROOT/tests/fixtures/images/alpha.png" "$file"
  printf '%s\n' "$file" >"$NEXT"
  smark=$(wc -l <"$SLOG"); sha=$(sha256sum "$file" | cut -d' ' -f1)
  drag_past_threshold
  mark=$(wc -l <"$DLOG")
  fake "signal Activated drop"
  geo=$(wait_geo 4)
  sleep 0.5
  local shot="$OUT_LOG-shot-waiting.png" flat="$OUT_LOG-shot-waiting-rgba.png" ink="" s2="" ac="" early=""
  rm -f "$shot" "$flat"
  [ -n "$geo" ] && screen_shot "$shot"
  # pixels.py reads filter 0 RGBA PNGs; GNOME's screenshot is re-encoded.
  [ -s "$shot" ] && ffmpeg -loglevel error -y -i "$shot" -pix_fmt rgba -pred none "$flat" 2>/dev/null
  local gx gy gw gh cx="" cy="" dx dy mx my
  read -r gx gy gw gh <<<"$geo"
  read -r mx my <<<"$(screen_middle)"
  if [ -n "$geo" ]; then
    cx=$((gx + gw / 2)); cy=$((gy + gh / 2))
    read -r dx dy <<<"$(webp_offset)"
    if [ -s "$flat" ]; then
      s2=$(pxcount "$flat" $((cx + dx - 28)) $((cy + dy - 28)) 56 56 surface-2 8)
      ac=$(pxcount "$flat" $((cx + dx - 28)) $((cy + dy - 28)) 56 56 accent 8)
      [ "${s2:-1}" -le 10 ] && [ "${ac:-1}" -le 10 ] && early=1
      ink=$(pxcount "$flat" $((cx - 36)) $((cy - 24)) 72 48 text-primary 40)
    fi
    # shellcheck disable=SC2086
    move_to_webp $geo
  fi
  ptr_up
  wait_file "$base.webp" 5 && wrote=yes
  fake "signal Deactivated drop"
  sleep 1; wait_gone 12 || true
  seg=$(tail -n +$((mark + 1)) "$DLOG")
  mime=$(file --mime-type -b "$base.webp" 2>/dev/null)
  local src_seg; src_seg=$(tail -n +$((smark + 1)) "$SLOG")
  echo "INFO AC18 (c): hive ${geo:-not shown} (centre ${cx:-?} ${cy:-?}, the work area's ${mx:-?} ${my:-?}); $(grep -E '^hive surface: ' <<<"$seg" | tr '\n' ' '); probe or reveal lines $(grep -cE '^hive (probe|reveal):' <<<"$seg"); WEBP box before enter surface-2 ${s2:-none}, accent ${ac:-none}; centre text-primary ${ink:-none}; $(basename "$base").webp ${mime:-not written}"
  check "AC18 (c): the portal's Activated mid-drag shows the hive as a window (a \"hive surface: window\" line, no get_layer_surface)" \
    '[ -n "$geo" ] && grep -qE "^hive surface: window " <<<"$seg" && ! grep -qE -- "-> zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(" <<<"$seg"'
  check "AC18 (c): no \"hive probe:\" and no \"hive reveal:\" line (window path)" \
    '[ -n "$geo" ] && ! grep -qE "^hive (probe|reveal):" <<<"$seg"'
  check "AC18 (c): the window is in the middle of the screen (its centre within 2 px of the work area's centre)" \
    '[ -n "$cx" ] && [ -n "${my:-}" ] && [ $((cx - mx)) -ge -2 ] && [ $((cx - mx)) -le 2 ] && [ $((cy - my)) -ge -2 ] && [ $((cy - my)) -le 2 ]'
  check "AC18 (c): the waiting shot was taken before enter (no cells yet)" '[ "$early" = 1 ]'
  check "AC18 (c): the waiting window's centre shows the waiting text (text-primary ink, as S-021 AC10)" '[ "${ink:-0}" -ge 15 ]'
  check "AC18 (c): the drop on the WEBP cell before Deactivated writes $(basename "$base").webp within 5 s (image/webp)" \
    '[ "$wrote" = yes ] && [ "$mime" = image/webp ]'
  check "AC18 (c): the source reports deleted=False and the original's sha256 is unchanged" \
    'grep -q "drag-end deleted=False" <<<"$src_seg" && [ "$(sha256sum "$file" | cut -d" " -f1)" = "$sha" ]'
}

# After the cases, before summary: stop the monitor and the fake, end every
# portal process on this bus, and check the live desktop's portal processes.
portal_teardown() {
  kill "$MON" 2>/dev/null
  if [ -n "$FAKE_PID" ]; then echo quit >&4; exec 4>&-; timeout 5 tail --pid="$FAKE_PID" -f /dev/null || kill "$FAKE_PID" 2>/dev/null; fi
  local p
  for p in $(bus_procs | cut -d' ' -f1); do kill "$p" 2>/dev/null; done
  sleep 1
  local left live1; left=$(bus_procs); live1=$(live_portal_pids)
  echo "INFO teardown: live xdg-desktop-portal pids before ${LIVE0:-none}, after ${live1:-none}; private portal processes left: ${left:-none}"
  check "teardown: the live desktop's portal processes are the same before and after (pids)" '[ "$LIVE0" = "$live1" ]'
  [ "$MODE" = portal-real ] && check "teardown: no portal process of this private bus is left" '[ -z "$left" ]'
}
