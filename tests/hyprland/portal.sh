# shellcheck shell=bash
# Conditions are passed to check() in single quotes and expanded by eval.
# shellcheck disable=SC2016,SC2034,SC2154
# S-014e3 (backlog/S-014-drop-hive.md, AC20 for Hyprland): the --portal mode
# of tests/hyprland/repro.sh. Sourced inside inner (the private session bus,
# the private XDG_RUNTIME_DIR, the nested Hyprland) after tests/drop/lib.sh,
# once the nested Hyprland runs and the pointer map is known.
#
# The installed xdg-desktop-portal and xdg-desktop-portal-hyprland are D-Bus
# activated by the private bus (dbus-run-session runs the service file's
# Exec, never the systemd unit), as tests/drop/portal_real.sh does for GNOME
# and KDE: a throwaway HOME, the private bus, the nested Hyprland inside a
# private headless KWin, never the live desktop or its bus. repro.sh skips
# tests/no_portal.sh in this mode only, copies the shipped
# packaging/io.github.m7mmad_omar.Kawaro.desktop to the throwaway
# XDG_DATA_HOME/applications, starts the nested Hyprland with
# HYPRLAND_NO_SD_VARS=1 (so it runs no `systemctl --user import-environment`
# and no `dbus-update-activation-environment --systemd`), and then, on the
# private bus only (no --systemd, no --all), sets WAYLAND_DISPLAY,
# HYPRLAND_INSTANCE_SIGNATURE, XDG_CURRENT_DESKTOP=Hyprland and
# XDG_SESSION_TYPE=wayland in the activation environment before any client
# starts. Portal processes activated before that are ended, so the ones the
# daemon uses read the nested Hyprland's names.
#
# Flow (portal_hypr_run):
#   1. the daemon (config.toml: hold_ms = 400, no portal_shortcut = false)
#      registers through the real portal; one "portal shortcuts: bound drop
#      (T)" line (AC20, AC13's T is the reply's trigger_description);
#   2. the ID is read from `hyprctl globalshortcuts` of the nested instance,
#      not guessed, and must be io.github.m7mmad_omar.Kawaro (the
#      identifier AC22 documents); then the private config gets the bind for
#      the read ID and is reloaded (`hyprctl reload config-only`). The
#      private config is Lua (as every repro.sh run), so the line is the Lua
#      form of AC20's `bind = SUPER, Z, global, <ID>:drop`:
#      hl.bind("SUPER + Z", hl.dsp.global("<ID>:drop"));
#   3. AC20 (a), (b), (c) and the AC21 runs, by real_portal_run of
#      tests/drop/portal_real.sh, unchanged, with the keys through the outer
#      KWin's fake_input (Super 125, then Z 44 50 ms later; released in the
#      reverse order);
#   4. the release closes the hive: a PNG drag, Super+Z held until the drag
#      has entered and the hive is revealed, then the key let go with the
#      drag still over the hive: the layer is destroyed within 500 ms of the
#      key-up, and the button let go over the source writes no file; and
#      the same once the drag has ended over the source before the key-up
#      (the hive stays while the key is held, S-019 decision 3);
#   5. what the backend sent: every Activated and Deactivated of
#      org.freedesktop.impl.portal.GlobalShortcuts (xdph to the frontend) and
#      of org.freedesktop.portal.GlobalShortcuts (the frontend to the
#      daemon), from dbus-monitor, and every window or layer that mapped
#      during the registration (a dialog), as INFO lines. If the backend
#      never sends Deactivated, AC20 (b) fails (the AC says the story then
#      goes to the maintainer; manager decision 2 would then list Hyprland).
#   6. teardown (portal_teardown): the live desktop's xdg-desktop-portal PIDs
#      are the same before and after, and no portal process of this private
#      bus is left.
#
# The caller (repro.sh inner) provides move, ptr_move, ptr_down, ptr_up,
# key, the input fifo on fd 3, BIN, ROOT, OUT, W, the nested $sig and $wl,
# pids (killed by finish), and the private Hyprland config path HYPR_CONF.

DROP_HERE="$ROOT/tests/drop"
# portal_real.sh reads its helper scripts from $HERE (tests/drop there).
HERE_SAVED=$HERE
HERE=$DROP_HERE
# shellcheck source=../drop/portal_real.sh
source "$DROP_HERE/portal_real.sh"
HERE=$HERE_SAVED
APP_ID="io.github.m7mmad_omar.Kawaro"

# The portal_real.sh caller contract, for the nested Hyprland.
key_down() { echo "keydown 125" >&3; sleep 0.05; echo "keydown 44" >&3; }
key_up() { echo "keyup 44" >&3; sleep 0.05; echo "keyup 125" >&3; }
# The hold path anchors on motion: a 1 px jitter at the drag's position
# (drag_past_threshold leaves it at 210 180) for 800 ms, queued on the input
# client so the caller goes on.
hold_motion() {
  local i
  for ((i = 0; i < 80; i++)); do
    if ((i % 2)); then move 211 181; else move 210 180; fi
    echo "sleep 10" >&3
  done
}
# hive_geo: "X Y W H" of the hive layer (the last "hive surface:" line of
# the current daemon log) while a layer named kawaro is listed.
hive_geo() {
  hyprctl layers 2>/dev/null | grep -q 'namespace: kawaro' || return 0
  grep -E '^hive surface: ' "$DLOG" | tail -1 | cut -d' ' -f4-
}
hive_shows() { grep -cE -- '-> zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(' <<<"$1"; }
# clients_now: "X Y W H|PID|CLASS|TITLE" per mapped client.
clients_now() {
  hyprctl clients -j 2>/dev/null | python3 -I -c 'import json, sys
try:
    cs = json.load(sys.stdin)
except ValueError:
    cs = []
for c in cs:
    if c.get("mapped", True):
        print("%d %d %d %d|%s|%s|%s" % (*c["at"], *c["size"], c["pid"], c["class"], c["title"]))'
}
dialog_now() {
  clients_now | awk -F'|' -v a="${d:-0}" -v b="${src:-0}" '$2 != a && $2 != b' | head -1
}
dialog_raise() {
  hyprctl dispatch "hl.dsp.focus({ window = \"pid:$1\" })" >/dev/null 2>&1 || true
}
# Hyprland's portal is not known to show a dialog for GlobalShortcuts; if
# one maps, the accepting button is assumed bottom right, as in KDE's.
accept_click() { click $(($1 + $3 - 50)) $(($2 + $4 - 25)); }
accept_key() { key 28; }
# The window watch: a poller that appends "PID|CLASS|TITLE" for every client
# address it has not seen, and "layer NAMESPACE PID" for every new layer.
watch_start() {
  WATCH="$XDG_RUNTIME_DIR/watch.log"; : >"$WATCH"
  (
    declare -A seen=()
    while :; do
      while IFS='|' read -r addr pid cls title; do
        [ -n "$addr" ] && [ -z "${seen[$addr]:-}" ] && { seen[$addr]=1; echo "$pid|$cls|$title" >>"$WATCH"; }
      done < <(hyprctl clients -j 2>/dev/null | python3 -I -c 'import json, sys
try:
    cs = json.load(sys.stdin)
except ValueError:
    cs = []
for c in cs:
    print("%s|%s|%s|%s" % (c["address"], c["pid"], c["class"], c["title"]))')
      sleep 0.2
    done
  ) &
  pids+=("$!")
  WATCH_PID=$!
}
watch_read() { cat "$WATCH" 2>/dev/null; }
layers_now() { hyprctl layers -j 2>/dev/null | python3 -I -c 'import json, sys
try:
    ms = json.load(sys.stdin)
except ValueError:
    ms = {}
for m in ms.values():
    for lv in m.get("levels", {}).values():
        for l in lv:
            print("%s %s" % (l.get("namespace"), l.get("pid")))' | sort; }

# proc_env PID NAME: the value of NAME in PID's environment.
proc_env() { tr '\0' '\n' <"/proc/$1/environ" 2>/dev/null | sed -n "s/^$2=//p" | head -1; }
# exe_pids PATH: the PIDs on this private bus whose /proc/PID/exe is PATH.
exe_pids() {
  local p
  for p in $(bus_procs | cut -d' ' -f1); do
    [ "$(readlink "/proc/$p/exe" 2>/dev/null)" = "$1" ] && echo "$p"
  done
}

# The bind, after the daemon's bound line: AC20's ID from the nested
# instance, then the private config line and a reload. A hook of
# real_portal_run (portal_after_bound), so it runs before AC20 (a).
portal_after_bound() {
  local gs id=""
  gs=$(hyprctl globalshortcuts 2>&1)
  echo "INFO AC20 Hyprland: hyprctl globalshortcuts of the nested instance: $(tr '\n' '|' <<<"${gs:-none}")"
  id=$(grep -oE '[^[:space:]]+:drop\b' <<<"$gs" | head -1)
  READ_ID=${id%:drop}
  check "AC20 Hyprland: \`hyprctl globalshortcuts\` of the nested instance lists the daemon's shortcut as $APP_ID:drop (the ID is read, not guessed)" \
    '[ "$READ_ID" = "$APP_ID" ]'
  local bindid=${id:-$APP_ID:drop}
  printf '%s\n' "-- S-014e3: AC20's bind = SUPER, Z, global, <ID>:drop, the ID read" \
    "-- from hyprctl globalshortcuts above." \
    "hl.bind(\"SUPER + Z\", hl.dsp.global(\"$bindid\"))" >>"$HYPR_CONF"
  hyprctl reload config-only >/dev/null 2>&1
  sleep 1
  local cerr sz
  cerr=$(hyprctl configerrors 2>/dev/null | grep -v '^\s*$')
  # The Super+Z binds as `hyprctl binds -j` lists them (modmask 64 is SUPER).
  sz=$(hyprctl binds -j 2>/dev/null | python3 -I -c 'import json, sys
try:
    bs = json.load(sys.stdin)
except ValueError:
    bs = []
for b in bs:
    if b.get("key", "").lower() == "z" and b.get("modmask") == 64:
        print(json.dumps(b, sort_keys=True))')
  echo "INFO AC20 Hyprland: bind added for \"$bindid\" (appended to the private config, hyprctl reload config-only); config errors: ${cerr:-none}; Super+Z binds listed: ${sz:-none}"
  check "AC20 Hyprland: the private config's Super+Z global bind for the read ID is loaded (one Super+Z bind in \`hyprctl binds\`, no config error)" \
    '[ -n "$id" ] && [ -z "$cerr" ] && [ "$(grep -c . <<<"$sz")" -eq 1 ]'
}

# Setup checks on the portal processes of this bus, after the registration.
portal_env_checks() {
  local fe xd fe_cd fe_bus xd_wl xd_sig
  fe=$(exe_pids /usr/libexec/xdg-desktop-portal | head -1)
  xd=$(exe_pids /usr/libexec/xdg-desktop-portal-hyprland | head -1)
  [ -n "$fe" ] && { fe_cd=$(proc_env "$fe" XDG_CURRENT_DESKTOP); fe_bus=$(proc_env "$fe" DBUS_SESSION_BUS_ADDRESS); }
  [ -n "$xd" ] && { xd_wl=$(proc_env "$xd" WAYLAND_DISPLAY); xd_sig=$(proc_env "$xd" HYPRLAND_INSTANCE_SIGNATURE); }
  echo "INFO AC20 Hyprland: frontend pid ${fe:-none} (XDG_CURRENT_DESKTOP=${fe_cd:-unset}); xdg-desktop-portal-hyprland pid ${xd:-none} (WAYLAND_DISPLAY=${xd_wl:-unset}, nested $wl; signature ${xd_sig:-unset}, nested $sig)"
  check "setup Hyprland: the installed /usr/libexec/xdg-desktop-portal runs on this private bus with XDG_CURRENT_DESKTOP=Hyprland" \
    '[ -n "$fe" ] && [ "${fe_cd:-}" = Hyprland ] && [ "${fe_bus:-}" = "$DBUS_SESSION_BUS_ADDRESS" ]'
  check "setup Hyprland: /usr/libexec/xdg-desktop-portal-hyprland runs on this private bus, connected to the nested Hyprland (its WAYLAND_DISPLAY and instance signature)" \
    '[ -n "$xd" ] && [ "${xd_wl:-}" = "$wl" ] && [ "${xd_sig:-}" = "$sig" ]'
}

# 4. The release closes the hive: drag, hold until revealed, let the key go
# with the drag over the hive.
release_case() {
  echo "== Hyprland: Super+Z held mid-drag until the hive is revealed, then released over the hive"
  local file="$W/release.png" mark geo gx gy gw gh trel g gone=none rev=no web=no
  cp "$ROOT/tests/fixtures/images/alpha.png" "$file"
  printf '%s\n' "$file" >"$NEXT"
  drag_past_threshold
  mark=$(wc -l <"$DLOG")
  key_down
  hold_motion
  geo=$(wait_geo 4)
  if [ -n "$geo" ]; then
    sleep 1
    read -r gx gy gw gh <<<"$(hive_geo)"
    move $((gx + gw / 2)) $((gy + gh / 2)); sleep 0.8
  fi
  tail -n +$((mark + 1)) "$DLOG" | grep -qE '^hive reveal: [0-9]+' && rev=yes
  local rmark; rmark=$(wc -l <"$DLOG")
  trel=$(date -u +%T.%6N)
  key_up
  sleep 1
  g=$(tail -n +$((rmark + 1)) "$DLOG" | grep -m1 -E -- '-> zwlr_layer_surface_v1[#@][0-9]+\.destroy\(' | grep -oE '^\[[0-9:.]+\]')
  [ -n "$g" ] && gone=$(awk -v a="$(tod "$trel")" -v b="$(tod "$g")" 'BEGIN { d = b - a; if (d < -43200) d += 86400; printf "%.1f\n", d * 1000 }')
  move 60 60; sleep 0.3; ptr_up; sleep 1.5
  [ -e "${file%.png}.webp" ] && web=yes
  local seg; seg=$(tail -n +$((mark + 1)) "$DLOG")
  echo "INFO Hyprland release: hive ${geo:-not shown}; reveal before the release $rev; layer destroyed ${gone} ms after the key-up; release.webp written $web; daemon lines: $(grep -E '^(portal|hive (reveal|probe|anchor))' <<<"$seg" | tr '\n' '|')"
  check "AC20 Hyprland: Super+Z held mid-drag shows the hive (\"hive reveal: N\" once the drag enters), and letting the key go over it destroys the layer within 500 ms of the key-up, with no file written" \
    '[ "$rev" = yes ] && within_ms "$gone" 500 && [ "$web" = no ]'
  wait_gone 12 || true
}

# 4b. The release after the drag has ended, with no action run: a PNG
# drag, Super+Z held until the hive is revealed, the drag out to (60, 60)
# and its button let go over the source (the hive stays while the key is
# held, S-019 decision 3), then the key-up: the layer must go within 500 ms.
# If the backend loses this Deactivated, the hive stays after the user lets
# go (in this run every hold that ended in a drop on WEBP got no backend
# Deactivated, while a plain hold and a release over the hive did).
ended_release_case() {
  echo "== Hyprland: Super+Z held mid-drag, the hive revealed, the drag ended over the source, then the key released"
  local file="$W/ended.png" mark geo gx gy gw gh trel g gone=none rev=no web=no deact=none
  cp "$ROOT/tests/fixtures/images/alpha.png" "$file"
  printf '%s\n' "$file" >"$NEXT"
  drag_past_threshold
  mark=$(wc -l <"$DLOG")
  key_down
  hold_motion
  geo=$(wait_geo 4)
  if [ -n "$geo" ]; then
    sleep 1
    read -r gx gy gw gh <<<"$(hive_geo)"
    move $((gx + gw / 2)) $((gy + gh / 2)); sleep 0.8
  fi
  tail -n +$((mark + 1)) "$DLOG" | grep -qE '^hive reveal: [0-9]+' && rev=yes
  move 60 60; sleep 0.3; ptr_up; sleep 0.5
  local still=no; [ -n "$(hive_geo)" ] && still=yes
  local rmark; rmark=$(wc -l <"$DLOG")
  trel=$(date -u +%T.%6N)
  key_up
  sleep 1
  g=$(tail -n +$((rmark + 1)) "$DLOG" | grep -m1 -E -- '-> zwlr_layer_surface_v1[#@][0-9]+\.destroy\(' | grep -oE '^\[[0-9:.]+\]')
  [ -n "$g" ] && gone=$(awk -v a="$(tod "$trel")" -v b="$(tod "$g")" 'BEGIN { d = b - a; if (d < -43200) d += 86400; printf "%.1f\n", d * 1000 }')
  deact=$(python3 -I "$PR" signals "$MONLOG" | awk -v t="$trel" '$1 >= t && $2 == "impl" && $3 == "Deactivated" { print $1; exit }')
  [ -e "${file%.png}.webp" ] && web=yes
  local seg; seg=$(tail -n +$((mark + 1)) "$DLOG")
  echo "INFO Hyprland ended drag: hive ${geo:-not shown}; reveal $rev; hive still shown after the drag ended $still; backend Deactivated after the key-up at ${deact:-none} (key-up ${trel}); layer destroyed ${gone} ms after the key-up; ended.webp written $web; daemon lines: $(grep -E '^(portal|hive (reveal|probe|anchor))' <<<"$seg" | tr '\n' '|')"
  check "AC20 Hyprland: Super+Z held mid-drag, the hive revealed, the drag ended over the source with the key still held: the key-up destroys the layer within 500 ms, with no file written" \
    '[ "$rev" = yes ] && [ "$still" = yes ] && within_ms "$gone" 500 && [ "$web" = no ]'
  wait_gone 12 || true
}

# 5. What the backend sent, over the whole run.
backend_report() {
  local all ia id pa pd
  all=$(python3 -I "$PR" signals "$MONLOG")
  ia=$(grep -c ' impl Activated ' <<<"$all"); id=$(grep -c ' impl Deactivated ' <<<"$all")
  pa=$(grep -c ' portal Activated ' <<<"$all"); pd=$(grep -c ' portal Deactivated ' <<<"$all")
  echo "INFO AC20 Hyprland backend: over the run, xdg-desktop-portal-hyprland sent Activated $ia and Deactivated $id; the frontend sent the daemon Activated $pa and Deactivated $pd"
  echo "INFO AC20 Hyprland backend: shortcut ids in the signals: $(awk 'NF { print $2, $3, $4 }' <<<"$all" | sort | uniq -c | awk '{ $1 = $1 "x"; print }' | tr '\n' ';')"
  # Holds that end with no impl Deactivated before the next hold's
  # Activated. An Activated within 1 s of the last one in an open hold is
  # key repeat (GNOME's backend repeats), not a new hold.
  local lost
  lost=$(awk 'function sec(x, a) { split(x, a, ":"); return a[1] * 3600 + a[2] * 60 + a[3] }
    $2 == "impl" && $3 == "Activated" { s = sec($1); if (open && s - last < 1) { last = s; next }; if (open) { n++; printf "%s ", t }; open = 1; t = $1; last = s }
    $2 == "impl" && $3 == "Deactivated" { open = 0 }
    END { if (open) { n++; printf "%s ", t }; printf "| %d\n", n }' <<<"$all")
  echo "INFO AC20 Hyprland backend: holds with no Deactivated from the backend before the next press: ${lost##*| } of $ia (Activated at ${lost%%|*})"
  if [ "$ia" -gt 0 ] && [ "$id" -eq 0 ]; then
    echo "INFO AC20 Hyprland FINDING: xdg-desktop-portal-hyprland sent Activated but never Deactivated in this run (manager decision 2: Hyprland goes on the no-release list)"
  fi
  echo "INFO AC20 Hyprland: layers on the nested output now: $(layers_now | tr '\n' ';')"
}

portal_hypr_run() {
  echo "INFO AC20 Hyprland: nested $(Hyprland --version | head -1 | cut -d' ' -f1-2) (package $(pkg_version hyprland)); the portal versions follow"
  portal_setup
  # The drag source; its windows and the daemon's are not a dialog.
  GDK_BACKEND=wayland GTK_A11Y=none python3 -I "$ROOT/tests/drop/source.py" "$NEXT" >"$SLOG" 2>&1 &
  src=$!
  pids+=("$src")
  sleep 2
  local layers0; layers0=$(layers_now)
  portal_start_daemon first
  pids+=("$d")
  real_portal_run Hyprland xdg-desktop-portal-hyprland
  pids+=("$d")
  portal_env_checks
  echo "INFO AC20 Hyprland: layers before the first daemon: $(tr '\n' ';' <<<"${layers0:-none}")"
  release_case
  ended_release_case
  backend_report
  portal_stop_daemon
  kill "${WATCH_PID:-0}" 2>/dev/null
  portal_teardown
}
