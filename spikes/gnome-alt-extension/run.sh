#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Spike (S-075 investigation): can a native GNOME mechanism, a GNOME Shell
# extension (or Shell.Eval alone), capture Alt_L press and release globally,
# including during a real GTK file drag, and feed `kawaro drop --hold` /
# `--release` so hold_ms and release reach the daemon? Does the drag stay
# copy-only with an unchanged original, and can the extension be installed
# and enabled live, without a shell restart, on Wayland?
#
# Phases, all in ONE private headless gnome-shell (--wayland, --no-x11,
# --unsafe-mode) on a private session bus with a throwaway HOME, XDG dirs
# and runtime dir. Never the live desktop, never a live setting:
#   A  Shell.Eval only: three capture mechanisms (stage captured-event,
#      stage key events, grab_accelerator), no drag then during a real GTK
#      drag, F9 as the conventional positive control key; then the probe is
#      removed and a negative control proves it no longer records.
#   B  the minimal extension shape installed in the throwaway
#      XDG_DATA_HOME/gnome-shell/extensions and enabled live (no restart),
#      recording which exact API enabled it and the raw D-Bus replies.
#   C  the extension forwards presses to `kawaro drop --hold` and releases
#      to `kawaro drop --release` (the same IPC the Hyprland bind runs),
#      against the real daemon with hold_ms 400: F9 hold (control), F9 tap,
#      Alt_L hold, Alt_L tap, then real GTK drags with F9 (control) and
#      Alt_L held mid-drag, dropped on the WEBP cell, and an Alt_L release
#      mid-drag that must close the hive without a drop.
#
# Heavy work rule: run through
#   flock /home/sbarah/.cache/kawaro-heavy.lock systemd-run --user --scope \
#     -p MemoryMax=6G -p CPUQuota=200% bash spikes/gnome-alt-extension/heavy.sh
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
BIN="$ROOT/target/release"
PY=/usr/bin/python3
# shellcheck source=../../tests/no_portal.sh
source "$ROOT/tests/no_portal.sh"
OUT="${SPIKE_OUT:-$HERE/out}"
mkdir -p "$OUT"
TAG=gnome-alt-ext
export HERE ROOT BIN OUT TAG PY

[ -x "$BIN/kawaro" ] && [ -x "$BIN/kawarod" ] || { echo "build first: cargo build --release"; exit 2; }

inner() {
  set -u
  unset DISPLAY XDG_ACTIVATION_TOKEN DESKTOP_STARTUP_ID HYPRLAND_INSTANCE_SIGNATURE XDG_CURRENT_DESKTOP XDG_SESSION_TYPE
  # shellcheck source=../../tests/drop/lib.sh
  source "$ROOT/tests/drop/lib.sh"
  OUT_LOG="$OUT/$TAG"
  local log="$OUT_LOG"
  : >"$log-cli.log"
  echo "INFO versions: $(rpm -q gnome-shell mutter xdg-desktop-portal 2>/dev/null | tr '\n' ' ')"
  # Default focus policy (smart), no first-login welcome dialog (it is
  # system modal and blocks input). --unsafe-mode only opens Shell.Eval to
  # this test on the private bus.
  gsettings set org.gnome.desktop.wm.preferences focus-new-windows smart
  gsettings set org.gnome.shell welcome-dialog-last-shown-version '999'
  gnome-shell --headless --wayland --no-x11 --unsafe-mode --wayland-display "$WAYLAND_DISPLAY" \
    --virtual-monitor 1280x800 >"$log-shell.log" 2>&1 &
  local shell=$!
  for _ in $(seq 150); do [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] && break; sleep 0.1; done
  [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] || { echo "gnome-shell did not start"; tail "$log-shell.log"; return 1; }
  sleep 3
  shell_eval() {
    # gdbus parses its argument as GVariant text, so a multi-line JavaScript
    # source must arrive as one escaped string literal. JSON string encoding
    # is the same escaping GVariant accepts.
    local arg
    arg=$("$PY" -c 'import json, sys; print(json.dumps(sys.stdin.read()))' <<<"$1")
    gdbus call --session --dest org.gnome.Shell --object-path /org/gnome/Shell \
      --method org.gnome.Shell.Eval "$arg" 2>&1
  }
  shell_str() {
    shell_eval "$1" | "$PY" -I -c 'import ast, json, sys
t = sys.stdin.read().strip().replace("(true,", "(True,", 1).replace("(false,", "(False,", 1)
try:
    ok, v = ast.literal_eval(t)
except (ValueError, SyntaxError):
    sys.exit()
if ok and v:
    print(json.loads(v))'
  }
  # GNOME starts in the overview; leave it, as a user would after login.
  for _ in $(seq 20); do
    shell_eval 'Main.overview.hide(); Main.overview.visible' | grep -q false && break
    sleep 0.5
  done
  echo "INFO private gnome-shell pid $shell (this pid must never change during the run)"

  hive_geo() {
    shell_eval 'global.get_window_actors().map(a => a.meta_window)
      .filter(w => w.get_title() === "Kawaro" && !w.minimized && w.showing_on_its_workspace())
      .filter(w => { const r = w.get_frame_rect(); return r.width > 0 && r.height > 0; })
      .map(w => { const r = w.get_frame_rect(); return [r.x, r.y, r.width, r.height].join(" "); }).join("")' \
      | grep -o '[0-9-]\+ [0-9-]\+ [0-9]\+ [0-9]\+' | head -1
  }

  local fifo="$XDG_RUNTIME_DIR/input.fifo"; mkfifo "$fifo"
  "$PY" "$HERE/input_alt.py" "$fifo" >"$log-input.log" 2>&1 &
  local input=$!
  exec 3>"$fifo"
  ptr_move() { echo "to $1 $2" >&3; sleep 0.3; }
  ptr_down() { echo "down" >&3; sleep 0.1; }
  ptr_up() { echo "up" >&3; sleep 0.1; }
  shift_down() { echo "keydown Shift_L" >&3; sleep 0.1; }
  shift_up() { echo "keyup Shift_L" >&3; sleep 0.1; }
  sleep 1
  # Mutter adds the virtual devices on the first event; plug them in now.
  echo "key Shift_L" >&3; sleep 10

  W="$XDG_RUNTIME_DIR/work"; mkdir -p "$W"
  NEXT="$XDG_RUNTIME_DIR/next"; printf '%s\n' "$W/photo one.png" >"$NEXT"
  DLOG="$log-daemon.log"; SLOG="$log-source.log"
  KAWARO_LANG=en KAWARO_COLOR_SCHEME=light KAWARO_REDUCED_MOTION=1 KAWARO_DEBUG_FOCUS=1 WAYLAND_DEBUG=1 \
    "$BIN/kawarod" >"$DLOG" 2>&1 &
  local d=$!
  for _ in $(seq 50); do "$BIN/kawaro" ping >/dev/null 2>&1 && break; sleep 0.1; done
  "$BIN/kawaro" ping >/dev/null 2>&1 || { echo "the daemon did not start"; return 1; }
  echo "INFO daemon pid $d, hold_ms $(grep -c 'hold_ms' "$XDG_CONFIG_HOME/kawaro/config.toml" >/dev/null && sed -n 's/^hold_ms *= *//p' "$XDG_CONFIG_HOME/kawaro/config.toml") ms"
  GTK_A11Y=none "$PY" "$ROOT/tests/drop/source.py" "$NEXT" >"$SLOG" 2>&1 &
  local src=$!
  sleep 4

  # ms_between LATER EARLIER: milliseconds between two HH:MM:SS.ffffff.
  ms_between() {
    awk -v a="$(tod "$1")" -v b="$(tod "$2")" 'BEGIN { d = a - b; if (d < -43200) d += 86400; printf "%.1f", d * 1000 }'
  }
  # first_ts PATTERN FILE: the HH:MM:SS.ffffff of the first line matching.
  first_ts() { grep -m1 -E "$1" "$2" 2>/dev/null | cut -d' ' -f1; }
  count_re() { grep -cE "$1" <<<"$2"; }

  # ---------------- Phase A: Shell.Eval only -----------------------------
  echo "== Phase A: Shell.Eval only (probe.js), F9 the conventional control key"
  local plog="$XDG_RUNTIME_DIR/probe-keys.log"
  shell_eval "$(cat "$HERE/probe.js")" | tee "$OUT/probe-install.txt"
  sleep 0.5
  cp "$plog" "$OUT/probe-grab.txt"
  echo "INFO probe grab results: $(grep -E 'grab-' "$OUT/probe-grab.txt" | sed 's/^[^ ]* [0-9]* //' | tr '\n' ';')"

  shell_eval 'global._sbh.reset(); true' >/dev/null
  local tp tr
  tp=$(date -u +%T.%6N); echo "keydown F9" >&3; sleep 0.6; echo "keyup F9" >&3; tr=$(date -u +%T.%6N); sleep 0.5
  local tp2; tp2=$(date -u +%T.%6N); echo "keydown Alt_L" >&3; sleep 0.6; echo "keyup Alt_L" >&3; sleep 0.5
  cp "$plog" "$OUT/probe-nodrag.txt"
  local seg; seg=$(cat "$OUT/probe-nodrag.txt")
  echo "INFO A1 no drag: F9 press $(count_re 'press F9' "$seg"), release $(count_re 'release F9' "$seg"); Alt_L press $(count_re 'press Alt_L' "$seg"), release $(count_re 'release Alt_L' "$seg"); mechanisms: $(grep -oE '(stage-cap|stage-key-press|stage-key-release|accel-activated|accel-deactivated)[^ ]*' "$OUT/probe-nodrag.txt" | sort | uniq -c | tr '\n' ';')"
  check "A1 probe (Eval only): F9 press and release are seen with no drag" \
    '[ "$(count_re "press F9" "$seg")" -ge 1 ] && [ "$(count_re "release F9" "$seg")" -ge 1 ]'
  check "A1 probe (Eval only): Alt_L press and release are seen with no drag" \
    '[ "$(count_re "press Alt_L" "$seg")" -ge 1 ] && [ "$(count_re "release Alt_L" "$seg")" -ge 1 ]'

  shell_eval 'global._sbh.reset(); true' >/dev/null
  cp "$ROOT/tests/fixtures/images/alpha.png" "$W/phase a.png"
  printf '%s\n' "$W/phase a.png" >"$NEXT"
  local smark; smark=$(wc -l <"$SLOG")
  drag_past_threshold
  local sseg; sseg=$(tail -n +$((smark + 1)) "$SLOG")
  check "A2 setup: the GTK drag is under way (drag-begin)" 'grep -q "drag-begin" <<<"$sseg"'
  tp=$(date -u +%T.%6N); echo "keydown Alt_L" >&3; sleep 0.5; echo "keyup Alt_L" >&3; sleep 0.3
  tp2=$(date -u +%T.%6N); echo "keydown F9" >&3; sleep 0.5; echo "keyup F9" >&3; sleep 0.3
  ptr_up; sleep 1
  cp "$plog" "$OUT/probe-drag.txt"
  seg=$(cat "$OUT/probe-drag.txt")
  echo "INFO A2 during a real GTK drag: Alt_L press $(count_re 'press Alt_L' "$seg"), release $(count_re 'release Alt_L' "$seg"); F9 press $(count_re 'press F9' "$seg"), release $(count_re 'release F9' "$seg"); mechanisms: $(grep -oE '(stage-cap|stage-key-press|stage-key-release|accel-activated|accel-deactivated)[^ ]*' "$OUT/probe-drag.txt" | sort | uniq -c | tr '\n' ';')"
  check "A2 probe (Eval only): Alt_L press and release are seen during a real GTK file drag" \
    '[ "$(count_re "press Alt_L" "$seg")" -ge 1 ] && [ "$(count_re "release Alt_L" "$seg")" -ge 1 ]'
  check "A2 probe (Eval only): F9 press and release are seen during a real GTK file drag (control)" \
    '[ "$(count_re "press F9" "$seg")" -ge 1 ] && [ "$(count_re "release F9" "$seg")" -ge 1 ]'
  wait_gone 5 || true

  shell_eval 'global._sbh.remove(); global._sbh.reset(); true' >/dev/null
  echo "keydown F9" >&3; sleep 0.3; echo "keyup F9" >&3; sleep 0.5
  cp "$plog" "$OUT/probe-after-remove.txt"
  seg=$(cat "$OUT/probe-after-remove.txt")
  echo "INFO A3 after the probe was removed: $(tr '\n' ';' <"$OUT/probe-after-remove.txt")"
  check "A3 probe removed: the F9 tap afterwards is not recorded (the Phase B log is the extension's alone)" \
    '! grep -qE "stage-|accel-" <<<"$seg"'

  # ---------------- Phase B: the extension, enabled live -----------------
  echo "== Phase B: the minimal extension, installed and enabled with the shell running"
  local ext_uuid='kawaro-alt@kawaro.test'
  local ext_dir="$XDG_DATA_HOME/gnome-shell/extensions/$ext_uuid"
  mkdir -p "$(dirname "$ext_dir")"
  rm -rf "$ext_dir"
  cp -r "$HERE/ext/$ext_uuid" "$ext_dir"
  MARKS="$XDG_RUNTIME_DIR/spawn-marks"; : >"$MARKS"
  EXTLOG="$XDG_RUNTIME_DIR/kawaro-ext.log"; : >"$EXTLOG"
  local wrap="$XDG_RUNTIME_DIR/kawaro-wrap.sh"
  cat >"$wrap" <<EOF
#!/bin/sh
printf '%s %s\n' "\$(date -u +%T.%6N)" "\$1" >>"$MARKS"
case "\$1" in
  hold) exec "$BIN/kawaro" drop --hold >>"$OUT_LOG-cli.log" 2>&1 ;;
  release) exec "$BIN/kawaro" drop --release >>"$OUT_LOG-cli.log" 2>&1 ;;
esac
EOF
  chmod +x "$wrap"
  printf 'log=%s\nhold_cmd=%s hold\nrelease_cmd=%s release\n' "$EXTLOG" "$wrap" "$wrap" \
    >"$XDG_RUNTIME_DIR/kawaro-ext.conf"
  export MARKS EXTLOG
  gdbus introspect --session --dest org.gnome.Shell --object-path /org/gnome/Shell \
    >"$OUT/gdbus-shell-introspect.txt" 2>&1
  gdbus call --session --dest org.gnome.Shell --object-path /org/gnome/Shell \
    --method org.gnome.Shell.Extensions.ListExtensions >"$OUT/gdbus-list-before.txt" 2>&1
  local e enabler=""
  e=$(gdbus call --session --dest org.gnome.Shell --object-path /org/gnome/Shell \
    --method org.gnome.Shell.Extensions.EnableExtension "$ext_uuid" 2>&1)
  echo "INFO B attempt 1 org.gnome.Shell.Extensions.EnableExtension: $e"
  sleep 2
  grep -q 'enabled' "$EXTLOG" && enabler="attempt 1: org.gnome.Shell.Extensions.EnableExtension on a just-copied dir"
  if [ -z "$enabler" ]; then
    e=$(gdbus call --session --dest org.gnome.Shell --object-path /org/gnome/Shell \
      --method org.gnome.Shell.Extensions.ReloadExtension "$ext_uuid" 2>&1)
    echo "INFO B attempt 2 org.gnome.Shell.Extensions.ReloadExtension: $e"
    e=$(gdbus call --session --dest org.gnome.Shell --object-path /org/gnome/Shell \
      --method org.gnome.Shell.Extensions.EnableExtension "$ext_uuid" 2>&1)
    echo "INFO B attempt 2b EnableExtension: $e"
    sleep 2
    grep -q 'enabled' "$EXTLOG" && enabler="attempt 2: org.gnome.Shell.Extensions.ReloadExtension then EnableExtension"
  fi
  if [ -z "$enabler" ]; then
    e=$(shell_str '(() => { try { Main.extensionManager._loadExtensions(); return "Main.extensionManager._loadExtensions ran"; } catch (e) { return "error " + e; } })()')
    echo "INFO B attempt 3 Shell.Eval: $e"
    e=$(gdbus call --session --dest org.gnome.Shell --object-path /org/gnome/Shell \
      --method org.gnome.Shell.Extensions.EnableExtension "$ext_uuid" 2>&1)
    echo "INFO B attempt 3b EnableExtension: $e"
    sleep 2
    grep -q 'enabled' "$EXTLOG" && enabler="attempt 3: Shell.Eval Main.extensionManager._loadExtensions then EnableExtension"
  fi
  if [ -z "$enabler" ]; then
    e=$(shell_str "(() => { try { if (Main.extensionManager.enableExtension) { Main.extensionManager.enableExtension('$ext_uuid'); return 'Main.extensionManager.enableExtension called'; } return 'no Main.extensionManager.enableExtension'; } catch (e) { return 'error ' + e; } })()")
    echo "INFO B attempt 4 Shell.Eval: $e"
    sleep 2
    grep -q 'enabled' "$EXTLOG" && enabler="attempt 4: Shell.Eval Main.extensionManager.enableExtension"
  fi
  gdbus call --session --dest org.gnome.Shell --object-path /org/gnome/Shell \
    --method org.gnome.Shell.Extensions.ListExtensions >"$OUT/gdbus-list-after-enable.txt" 2>&1
  local state
  state=$(shell_str "(() => { const EM = Main.extensionManager; const g = EM.getExtension ? EM.getExtension('$ext_uuid') : (EM.lookup ? EM.lookup('$ext_uuid') : null); return JSON.stringify({found: !!g, state: g ? String(g.state) : null}); })()")
  echo "INFO B enabled by: ${enabler:-none}; extension log after the attempts: $(tr '\n' ';' <"$EXTLOG"); manager state: $state; gnome-shell pid still $shell ($(kill -0 "$shell" 2>/dev/null && echo alive || echo dead))"
  grep -F "$ext_uuid" "$OUT/gdbus-list-after-enable.txt" >"$OUT/gdbus-our-extension.txt" || true
  echo "INFO B our entry in ListExtensions: $(head -c 400 "$OUT/gdbus-our-extension.txt" 2>/dev/null || echo none)"
  check "B: the extension is installed in the throwaway user extensions dir and enabled while the same gnome-shell keeps running (no restart)" \
    'grep -q "enabled" "$EXTLOG" && kill -0 "$shell" 2>/dev/null && grep -q "\"found\":true" <<<"$state"'
  check "B: the extension records its enable in its own log (ground truth of a live enable)" \
    'grep -q "enabled" "$EXTLOG"'

  # ---------------- Phase C: press and release at the daemon -------------
  # hold_test NAME KEY: with no drag, KEY held 0.7 s. The extension must see
  # press and release, run hold then release, and the daemon must show the
  # hive after hold_ms (400) and close it at the release.
  hold_test() {
    local name=$1 key=$2
    local m0 d0 x0 tpress g tshow="" trel gone=no
    m0=$(wc -l <"$MARKS"); d0=$(wc -l <"$DLOG"); x0=$(wc -l <"$EXTLOG")
    echo "== $name: $key held 0.7 s, no drag"
    local before; before=$(find "$W" -type f | sort)
    tpress=$(date -u +%T.%6N)
    echo "keydown $key" >&3
    local end; end=$(deadline 2)
    g=""
    while [ "$(now_ms)" -lt "$end" ]; do
      g=$(hive_geo)
      [ -n "$g" ] && { tshow=$(date -u +%T.%6N); break; }
      sleep 0.05
    done
    sleep 0.7
    trel=$(date -u +%T.%6N)
    echo "keyup $key" >&3
    sleep 0.3
    wait_gone 3 && gone=yes
    local xseg mseg dseg
    xseg=$(tail -n +$((x0 + 1)) "$EXTLOG")
    mseg=$(tail -n +$((m0 + 1)) "$MARKS")
    dseg=$(tail -n +$((d0 + 1)) "$DLOG")
    local tcap tspawn dshow dspawn
    tcap=$(first_ts "press $key" <(echo "$xseg"))
    tspawn=$(first_ts ' hold$' <(echo "$mseg"))
    dshow="none"; dspawn="none"
    [ -n "$tshow" ] && [ -n "$tcap" ] && dshow=$(ms_between "$tshow" "$tpress")
    [ -n "$tspawn" ] && [ -n "$tpress" ] && dspawn=$(ms_between "$tspawn" "$tpress")
    local dgate="none"
    [ -n "$tshow" ] && [ -n "$tspawn" ] && dgate=$(ms_between "$tshow" "$tspawn")
    echo "INFO $name: capture $(ms_between "${tcap:-$tpress}" "$tpress") ms after the press, hold command ${dspawn} ms after it, hive ${dshow} ms after it (${dgate} ms after the hold command, hold_ms 400); hive ${g:-not shown}, closed after release: $gone"
    echo "INFO $name: extension segment: $(tr '\n' ';' <<<"$xseg")"
    echo "INFO $name: spawn marks: $(tr '\n' ';' <<<"$mseg")"
    check "$name: the extension sees the press and the release of $key" \
      '[ "$(count_re "press $key" "$xseg")" -ge 1 ] && [ "$(count_re "release $key" "$xseg")" -ge 1 ]'
    check "$name: hold and release commands ran (the wrapper marks)" \
      'grep -q " hold$" <<<"$mseg" && grep -q " release$" <<<"$mseg"'
    check "$name: the hive is shown while the key is held, at hold_ms 400 after the press (measured ${dshow} ms, bound 300 to 800)" \
      '[ -n "$g" ] && awk -v d="${dshow/none/0}" "BEGIN { exit !(d >= 300 && d <= 800) }"'
    check "$name: the release reaches the daemon and closes the hive within 3 s" '[ "$gone" = yes ]'
    check "$name: no file written" '[ "$(find "$W" -type f | sort)" = "$before" ]'
  }

  # tap_test NAME KEY: a 150 ms tap must reach the daemon as hold then
  # release, and the release must cancel the pending show (no hive).
  tap_test() {
    local name=$1 key=$2
    local m0 d0 x0 tpress trel
    m0=$(wc -l <"$MARKS"); d0=$(wc -l <"$DLOG"); x0=$(wc -l <"$EXTLOG")
    echo "== $name: $key tapped 150 ms, no drag"
    tpress=$(date -u +%T.%6N)
    echo "keydown $key" >&3; sleep 0.15
    trel=$(date -u +%T.%6N)
    echo "keyup $key" >&3; sleep 1.5
    local xseg mseg dseg g
    xseg=$(tail -n +$((x0 + 1)) "$EXTLOG")
    mseg=$(tail -n +$((m0 + 1)) "$MARKS")
    dseg=$(tail -n +$((d0 + 1)) "$DLOG")
    g=$(hive_geo)
    echo "INFO $name: release $(ms_between "$trel" "$tpress") ms after the press; extension segment: $(tr '\n' ';' <<<"$xseg"); spawn marks: $(tr '\n' ';' <<<"$mseg"); hive surfaces made $(count_re '^hive surface: window ' "$dseg")"
    check "$name: the extension sees the press and the release of $key" \
      '[ "$(count_re "press $key" "$xseg")" -ge 1 ] && [ "$(count_re "release $key" "$xseg")" -ge 1 ]'
    check "$name: hold and release commands ran (the wrapper marks)" \
      'grep -q " hold$" <<<"$mseg" && grep -q " release$" <<<"$mseg"'
    check "$name: the release cancels the pending show: no hive surface within 1.5 s" \
      '[ "$(count_re "^hive surface: window " "$dseg")" -eq 0 ] && [ -z "$g" ]'
  }

  # drag_test NAME KEY FILE: a real GTK drag, KEY held past hold_ms mid-drag,
  # the drop on the WEBP cell with the key still held, then the release.
  drag_test() {
    local name=$1 key=$2 file=$3 base="${3%.*}"
    local m0 d0 x0 smark sha tpress tshow="" g trel wrote=no mime gone=no
    m0=$(wc -l <"$MARKS"); d0=$(wc -l <"$DLOG"); x0=$(wc -l <"$EXTLOG")
    smark=$(wc -l <"$SLOG")
    echo "== $name: $(basename "$file"), $key held mid-drag, drop on WEBP, then release"
    printf '%s\n' "$file" >"$NEXT"
    sha=$(sha256sum "$file" | cut -d' ' -f1)
    drag_past_threshold
    local sseg; sseg=$(tail -n +$((smark + 1)) "$SLOG")
    check "$name setup: the drag is under way before the key (drag-begin)" \
      'grep -q "drag-begin" <<<"$sseg" && ! grep -qE "drag-end|drag-cancel" <<<"$sseg"'
    tpress=$(date -u +%T.%6N)
    echo "keydown $key" >&3
    local end; end=$(deadline 4)
    g=""
    while [ "$(now_ms)" -lt "$end" ]; do
      g=$(hive_geo)
      [ -n "$g" ] && { tshow=$(date -u +%T.%6N); break; }
      sleep 0.05
    done
    if [ -z "$g" ]; then
      echo "INFO $name: no hive within 4 s of the press"
      trel=$(date -u +%T.%6N); echo "keyup $key" >&3; ptr_up; sleep 1
      check "$name: the hive appears while $key is held mid-drag" 'false'
      wait_gone 12 || true
      return
    fi
    sleep 0.7
    local g2; g2=$(hive_geo); [ -n "$g2" ] && g=$g2
    # shellcheck disable=SC2086
    move_to_webp $g
    ptr_up
    wait_file "$base.webp" 5 && wrote=yes
    trel=$(date -u +%T.%6N)
    echo "keyup $key" >&3
    sleep 0.5
    wait_gone 5 && gone=yes
    mime=$(file --mime-type -b "$base.webp" 2>/dev/null)
    local xseg mseg dseg
    xseg=$(tail -n +$((x0 + 1)) "$EXTLOG")
    mseg=$(tail -n +$((m0 + 1)) "$MARKS")
    dseg=$(tail -n +$((d0 + 1)) "$DLOG")
    sseg=$(tail -n +$((smark + 1)) "$SLOG")
    local tcap tspawn dshow dgate
    tcap=$(first_ts "press $key" <(echo "$xseg"))
    tspawn=$(first_ts ' hold$' <(echo "$mseg"))
    dshow="none"; dgate="none"
    [ -n "$tshow" ] && dshow=$(ms_between "$tshow" "$tpress")
    [ -n "$tshow" ] && [ -n "$tspawn" ] && dgate=$(ms_between "$tshow" "$tspawn")
    local sa
    sa=$(grep -o -- '-> wl_data_offer#[0-9]*\.set_actions([^)]*)' "$dseg" | sed 's/.*set_actions//' | sort -u | tr '\n' ' ')
    echo "INFO $name: capture $(ms_between "${tcap:-$tpress}" "$tpress") ms after the press, hive ${dshow} ms after it (${dgate} ms after the hold command); hive at $g; $(basename "$base").webp ${mime:-not written}; hive closed after release: $gone; set_actions seen: ${sa:-none}"
    echo "INFO $name: extension segment: $(tr '\n' ';' <<<"$xseg"); spawn marks: $(tr '\n' ';' <<<"$mseg")"
    check "$name: the extension sees the press and the release of $key during the drag" \
      '[ "$(count_re "press $key" "$xseg")" -ge 1 ] && [ "$(count_re "release $key" "$xseg")" -ge 1 ]'
    check "$name: hold and release commands ran (the wrapper marks)" \
      'grep -q " hold$" <<<"$mseg" && grep -q " release$" <<<"$mseg"'
    check "$name: the hive appeared within 4 s of the press, at hold_ms 400 after it (measured ${dshow} ms)" \
      '[ -n "$g" ]'
    check "$name: $(basename "$base").webp written within 5 s of the drop (image/webp), with the key still held" \
      '[ "$wrote" = yes ] && [ "$mime" = image/webp ]'
    check "$name: the drag is copy-only and the original is unchanged (deleted=False, sha256 kept)" \
      'grep -q "drag-end deleted=False" <<<"$sseg" && [ "$(sha256sum "$file" | cut -d" " -f1)" = "$sha" ]'
    check "$name: the offer never allows anything but copy (set_actions (1, 1) or (0, 0) only)" \
      '[ -z "$(tr " " "\n" <<<"${sa//, /,}" | grep -v -e "^$" -e "^(1,1)$" -e "^(0,0)$")" ]'
    check "$name: the release after the drop closes the hive within 5 s" '[ "$gone" = yes ]'
    wait_gone 5 || { ptr_move 1000 700; sleep 0.5; wait_gone 5; } || true
    sleep 1
  }

  # drag_release_test NAME KEY: the release mid-drag must close the hive
  # with no drop and no file.
  drag_release_test() {
    local name=$1 key=$2
    local m0 d0 x0 smark sha tpress tshow="" g trel gone=no
    m0=$(wc -l <"$MARKS"); d0=$(wc -l <"$DLOG"); x0=$(wc -l <"$EXTLOG")
    smark=$(wc -l <"$SLOG")
    echo "== $name: $(basename "$W/release.png"), $key held mid-drag, released with no drop"
    cp "$ROOT/tests/fixtures/images/alpha.png" "$W/release.png"
    printf '%s\n' "$W/release.png" >"$NEXT"
    sha=$(sha256sum "$W/release.png" | cut -d' ' -f1)
    local before; before=$(find "$W" -type f | sort)
    drag_past_threshold
    tpress=$(date -u +%T.%6N)
    echo "keydown $key" >&3
    local end; end=$(deadline 4)
    g=""
    while [ "$(now_ms)" -lt "$end" ]; do
      g=$(hive_geo)
      [ -n "$g" ] && { tshow=$(date -u +%T.%6N); break; }
      sleep 0.05
    done
    sleep 0.7
    trel=$(date -u +%T.%6N)
    echo "keyup $key" >&3
    sleep 0.3
    [ -n "$g" ] && wait_gone 3 && gone=yes
    ptr_up; sleep 1
    local xseg mseg sseg
    xseg=$(tail -n +$((x0 + 1)) "$EXTLOG")
    mseg=$(tail -n +$((m0 + 1)) "$MARKS")
    sseg=$(tail -n +$((smark + 1)) "$SLOG")
    echo "INFO $name: hive ${g:-not shown}$([ -n "$tshow" ] && echo " $(ms_between "$tshow" "$tpress") ms after the press"); closed after the release mid-drag: $gone; release $(ms_between "$trel" "$tpress") ms after the press; extension segment: $(tr '\n' ';' <<<"$xseg"); spawn marks: $(tr '\n' ';' <<<"$mseg")"
    check "$name: the hive appears while $key is held mid-drag" '[ -n "$g" ]'
    check "$name: the extension sees press and release" \
      '[ "$(count_re "press $key" "$xseg")" -ge 1 ] && [ "$(count_re "release $key" "$xseg")" -ge 1 ]'
    check "$name: the release reaches the daemon mid-drag and closes the hive within 3 s" '[ "$gone" = yes ]'
    check "$name: no drop, no file written (drag cancelled or ended with nothing accepted)" \
      '[ "$(find "$W" -type f | sort)" = "$before" ] && ! grep -q "drag-end deleted=True" <<<"$sseg"'
    check "$name: the original is unchanged" '[ "$(sha256sum "$W/release.png" | cut -d" " -f1)" = "$sha" ]'
    wait_gone 5 || { ptr_move 1000 700; sleep 0.5; wait_gone 5; } || true
    sleep 1
  }

  hold_test "C1 control F9 hold" F9
  tap_test "C2 control F9 tap" F9
  hold_test "C3 Alt_L hold" Alt_L
  tap_test "C4 Alt_L tap" Alt_L
  cp "$ROOT/tests/fixtures/images/alpha.png" "$W/drag control.png"
  drag_test "C5 drag F9 control" F9 "$W/drag control.png"
  cp "$ROOT/tests/fixtures/images/alpha.png" "$W/drag alt.png"
  drag_test "C6 drag Alt_L" Alt_L "$W/drag alt.png"
  drag_release_test "C7 drag Alt_L release mid-drag" Alt_L

  echo "INFO final: extension log lines $(wc -l <"$EXTLOG"), spawn marks $(wc -l <"$MARKS"); gnome-shell pid $shell ($(kill -0 "$shell" 2>/dev/null && echo alive || echo dead)), daemon pid $d"
  cp "$EXTLOG" "$OUT/ext-keys.txt"
  cp "$MARKS" "$OUT/spawn-marks.txt"
  cp "$OUT_LOG-cli.log" "$OUT/cli.txt" 2>/dev/null || true
  summary

  echo quit >&3; exec 3>&-
  "$BIN/kawaro" quit >/dev/null 2>&1
  sleep 0.5
  kill $src $d $input 2>/dev/null
  kill $shell 2>/dev/null
  timeout 5 tail --pid=$shell -f /dev/null || kill -9 $shell
}
export -f inner

echo "== gnome-alt-extension spike: private headless GNOME, private bus"
SANDBOX="$(mktemp -d)"
no_real_portal "$SANDBOX"
# The daemon registers nothing (portal_shortcut = false, written by
# no_real_portal), and the long press gate is 400 ms, the product default.
printf 'portal_shortcut = false\nhold_ms = 400\n' >"$SANDBOX/.config/kawaro/config.toml"
RUNDIR="$(mktemp -d)"
trap 'rm -rf "$SANDBOX" "$RUNDIR"' EXIT
env -u DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u XDG_CURRENT_DESKTOP \
  HOME="$SANDBOX" XDG_CONFIG_HOME="$SANDBOX/.config" XDG_DATA_HOME="$SANDBOX/.local/share" \
  XDG_CACHE_HOME="$SANDBOX/.cache" XDG_STATE_HOME="$SANDBOX/.local/state" \
  XDG_RUNTIME_DIR="$RUNDIR" WAYLAND_DISPLAY=wayland-sbh-alt-ext \
  dbus-run-session -- bash -c inner 2>"$OUT/$TAG-bus.log" | tee "$OUT/$TAG.txt"
grep -q '^RESULT PASS' "$OUT/$TAG.txt"
