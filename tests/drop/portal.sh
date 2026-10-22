#!/usr/bin/env bash
# Conditions are passed to check() in single quotes and expanded by eval.
# shellcheck disable=SC2016,SC2034,SC2329
# S-014e1 (backlog/S-014-drop-hive.md, AC13, AC17, AC18 parts a and b, AC19,
# and the manager's answers 4 and 5 of 2026-10-22): the daemon's
# GlobalShortcuts portal client against a fake portal
# (tests/drop/shortcuts_portal.py: Registry version 1, GlobalShortcuts
# version 2, every call logged with a UTC stamp), in a private kwin_wayland
# --virtual with a throwaway HOME, XDG dirs, runtime dir and session bus.
# The bus runs with its own configuration whose only service directory is
# empty, so nothing on it can be D-Bus activated: no real
# xdg-desktop-portal, backend or notification daemon starts, and with no
# fake running nothing owns org.freedesktop.portal.Desktop. Never touches
# the live desktop, its bus or the real portal.
#
# The daemon runs with SBARHIVE_DEBUG_FOCUS=1 and WAYLAND_DEBUG=1, and the
# contract is its stderr (tester entry of 2026-10-22 in the story):
#   portal shortcuts: bound drop (T)        T the trigger_description of the
#                                           BindShortcuts reply ("Fake
#                                           Super+Z" from the fake)
#   portal shortcuts: unavailable (R)       R no-portal, no-interface,
#                                           denied, error, no-release
#                                           (answer 4) or off (answer 5)
#   portal registry: unavailable            Register failed; registration
#                                           goes on
#   portal: activated drop                  one per Activated or Deactivated
#   portal: deactivated drop                of "drop" in the daemon's own
#                                           session (debug only); nothing
#                                           for an ignored signal
# and config.toml's key portal_shortcut (false after a denial).
#
# Phases, in this order (all by default):
#   ac13    fake "ok": the daemon's portal connection makes Registry.Register
#           (io.github.m7mmad_omar.SbarHive) its first portal method call
#           (property reads are not method calls), then CreateSession, then
#           one BindShortcuts on that session with one shortcut: id drop,
#           preferred_trigger LOGO+z, an English description; one line,
#           "portal shortcuts: bound drop (Fake Super+Z)". Then fake
#           "no-registry": one "portal registry: unavailable" line, and
#           CreateSession, BindShortcuts and the bound line still follow.
#   ac17    on the "ok" daemon, hold_ms 400, the pointer at (100, 700), the
#           signals timed by the fake ("pulse"), surfaces counted in the
#           trace from the fake's Activated stamp: 600 ms (one
#           get_layer_surface, 400 to 600 ms after Activated), 150 ms (none
#           within 1000 ms), three Activated at 0, 40 and 80 ms then
#           Deactivated at 600 (one), id "other" (none, no portal line),
#           another session's path (none, no portal line); one gate:
#           `drop --hold` then Deactivated 150 ms later (none), Activated
#           then `drop --release` 150 ms later (none).
#   ac18    (a) the 600 ms show above: exactly one "hive reveal:" or "hive
#           probe:" line, "hive probe: hidden (release)" or "(pointer)",
#           and no attach on the hive subsurface; the repeat show also has
#           exactly one such line. (b) a PNG drag held, its drag moving
#           around (400, 500), Activated, the hive, the drop on the WEBP
#           cell before Deactivated: "hive reveal: N" with N the lines of
#           `sbarhive actions` for the file (tests/drop/layer.sh
#           probe_checks), the WEBP written within 5 s, and every "hive
#           anchor:" line "none" or within 2 px of the first drag enter on
#           the catcher in the trace.
#   ac19    one daemon per mode, config.toml holding hold_ms = 500 only:
#           no fake (no-portal; a fake "ok" is started 1 s after the line,
#           so a retry would show), "no-gs" (no-interface), "version0"
#           (no-interface), "deny" (denied), "error" (error, the daemon in
#           Arabic: the description is Arabic), "empty" (denied). Each: one
#           "portal shortcuts:" line with its reason, no new call or
#           property read on the fake for 30 s after it, then `drop --hold
#           --at 640,400` (pointer at (100, 700)) shows one layer surface
#           450 to 700 ms after the press and `drop --release` destroys it
#           within 500 ms. Answer 5: after "deny" and "empty" config.toml
#           has portal_shortcut = false and still hold_ms = 500, after
#           "error" not; a new daemon on that profile makes no call (not
#           even Register) and prints "portal shortcuts: unavailable (off)";
#           with portal_shortcut = true written back, the next daemon
#           registers again.
#   hold    fake "hold" keeps the BindShortcuts Response: five `sbarhive
#           open` shows while it is held, the median from a UTC stamp before
#           the command to the hive's first attach(wl_buffer) in the trace
#           under 150 ms (the real daemon's path; crates/ui/tests/measure.sh
#           drives hive_bench, which has no portal client), no "portal
#           shortcuts:" line before the Response, and the bound line after
#           it.
# Usage: tests/drop/portal.sh [ac13] [ac17] [ac19] [hold] [target/release]
#   (ac17 runs ac18 too; ac17 needs the ac13 daemon and starts one itself)
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
BIN="$ROOT/target/release"
PHASES=""
for a in "$@"; do
  case "$a" in ac13 | ac17 | ac19 | hold) PHASES+="$a " ;; *) BIN="$a" ;; esac
done
[ -n "$PHASES" ] || PHASES="ac13 ac17 ac19 hold "
TAG="kwin-portal"
INPUT="$HERE/kwin-input/target/release/kwin-input"
[ -x "$BIN/sbarhive" ] && [ -x "$BIN/sbarhived" ] || { echo "build first: cargo build --release"; exit 2; }
[ -x "$INPUT" ] || { echo "build first: cargo build --release --manifest-path tests/drop/kwin-input/Cargo.toml"; exit 2; }
OUT="${DROP_OUT:-$HERE/out}"
mkdir -p "$OUT"
export BIN HERE ROOT OUT INPUT TAG PHASES

inner() {
  set -u
  unset DISPLAY XDG_ACTIVATION_TOKEN DESKTOP_STARTUP_ID HYPRLAND_INSTANCE_SIGNATURE XDG_CURRENT_DESKTOP XDG_SESSION_TYPE
  # shellcheck source=lib.sh
  source "$HERE/lib.sh"
  # shellcheck source=layer.sh
  source "$HERE/layer.sh"
  OUT_LOG="$OUT/$TAG"
  : >"$OUT_LOG-cli.log"
  local PL="$HERE/portal_log.py"

  # Setup guard: the bus activates nothing, so no real portal can start.
  local act
  act=$(gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
    --method org.freedesktop.DBus.ListActivatableNames 2>&1)
  echo "INFO setup: activatable names on the private bus: $act"
  check "setup: the private bus can activate nothing but itself (no org.freedesktop.portal.Desktop to start)" \
    'grep -q "org.freedesktop.DBus" <<<"$act" && ! grep -qiE "portal|notif" <<<"$act"'
  if grep -qi portal <<<"$act"; then summary; return 1; fi

  mkdir -p "$XDG_CONFIG_HOME/sbarhive"
  printf '[Windows]\nFocusStealingPreventionLevel=1\n\n[Plugins]\nscaleEnabled=false\n' >"$XDG_CONFIG_HOME/kwinrc"
  KWIN_WAYLAND_NO_PERMISSION_CHECKS=1 QT_FORCE_STDERR_LOGGING=1 \
    kwin_wayland --virtual --no-lockscreen --socket "$WAYLAND_DISPLAY" --width 1280 --height 800 >"$OUT_LOG-kwin.log" 2>&1 &
  local kwin=$!
  for _ in $(seq 150); do [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] && break; sleep 0.1; done
  [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] || { echo "kwin did not start"; tail "$OUT_LOG-kwin.log"; return 1; }
  sleep 2

  # As tests/drop/kwin.sh: the hive is shown while the trace has more
  # get_layer_surface than layer surface destroys; its rectangle is the
  # last "hive surface: layer" line after the last get_layer_surface.
  hive_geo() {
    local shown gone gl
    shown=$(grep -cE -- '-> zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(' "$DLOG")
    gone=$(grep -cE -- '-> zwlr_layer_surface_v1[#@][0-9]+\.destroy\(' "$DLOG")
    [ "$shown" -gt "$gone" ] || return 0
    gl=$(grep -nE -- '-> zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(' "$DLOG" | tail -1 | cut -d: -f1)
    tail -n +"$gl" "$DLOG" | grep -E '^hive surface: layer -?[0-9]+ -?[0-9]+ [0-9]+ [0-9]+$' | tail -1 | cut -d' ' -f4-
  }

  local f="$XDG_RUNTIME_DIR/in"; mkfifo "$f"
  "$INPUT" <"$f" >"$OUT_LOG-input.log" 2>&1 &
  exec 3>"$f"
  ptr_move() { echo "move $1 $2" >&3; }
  ptr_down() { echo "down" >&3; }
  ptr_up() { echo "up" >&3; }
  press_esc() { echo "keydown 1" >&3; echo "sleep 30" >&3; echo "keyup 1" >&3; }
  jitter() { # jitter X Y MS, as tests/drop/kwin.sh
    local i dx=1 dy=1
    for ((i = 0; i < $3 / 10; i++)); do
      if ((i % 2)); then echo "move $(($1 + dx)) $(($2 + dy))"; else echo "move $1 $2"; fi
      echo "sleep 10"
    done >&3
  }

  W="$XDG_RUNTIME_DIR/work"; mkdir -p "$W"
  cp "$ROOT/tests/fixtures/images/alpha.png" "$W/open me.png"
  NEXT="$XDG_RUNTIME_DIR/next"; printf '%s\n' "$W/open me.png" >"$NEXT"
  SLOG="$OUT_LOG-source.log"
  DLOG="$OUT_LOG-none-daemon.log"; : >"$DLOG"
  GTK_A11Y=none python3 "$HERE/source.py" "$NEXT" >"$SLOG" 2>&1 &
  local src=$!
  sleep 3

  # The time helpers of tests/drop/kwin.sh (S-019e).
  cli_ms() {
    local t0; t0=$(now_ms)
    "$BIN/sbarhive" "$@" >>"$OUT_LOG-cli.log" 2>&1
    CLI_RC=$?
    CLI_MS=$(($(now_ms) - t0))
  }
  ms_after() { awk -v a="$(tod "$1")" -v b="$(tod "$(grep -oE '^\[[0-9:.]+\]' <<<"$2")")" 'BEGIN { if (b == "" || a == "") { print "none"; exit } d = b - a; if (d < -43200) d += 86400; printf "%.1f\n", d * 1000 }'; }
  GLS_RE='-> zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\('
  XDG_RE='-> xdg_wm_base[#@][0-9]+\.get_xdg_surface\('
  DESTROY_RE='-> zwlr_layer_surface_v1[#@][0-9]+\.destroy\('
  # surfaces_by MARK STAMP MS: "LAYERS XDGS FIRST" after line MARK of DLOG
  # whose trace stamp is at most MS after STAMP (none when STAMP is none).
  surfaces_by() {
    [ "$2" = none ] && { echo "0 0 none"; return; }
    local seg; seg=$(tail -n +$(($1 + 1)) "$DLOG")
    local l=0 x=0 first="none" line d
    while IFS= read -r line; do
      [ -z "$line" ] && continue
      d=$(ms_after "$2" "$line")
      awk -v d="$d" -v m="$3" 'BEGIN { exit !(d != "none" && d + 0 <= m) }' || continue
      if grep -qE -- "$GLS_RE" <<<"$line"; then l=$((l + 1)); [ "$first" = none ] && first=$d; else x=$((x + 1)); fi
    done < <(grep -E -- "$GLS_RE|$XDG_RE" <<<"$seg")
    echo "$l $x $first"
  }
  destroyed_after() {
    local line; line=$(tail -n +$(($1 + 1)) "$DLOG" | grep -m1 -E -- "$DESTROY_RE")
    [ -n "$line" ] && ms_after "$2" "$line" || echo none
  }
  in_ms() { awk -v d="$1" -v lo="$2" -v hi="$3" 'BEGIN { exit !(d != "none" && d + 0 >= lo && d + 0 <= hi) }'; }
  cli_ok() { [ "$1" = 0 ] && [ "$2" -le 200 ]; }

  DPID="" FAKE_PID="" PLOG="$OUT_LOG-none-portal.log"; : >"$PLOG"
  start_daemon() { # start_daemon NAME [LANG]
    DLOG="$OUT_LOG-$1-daemon.log"
    SBARHIVE_LANG=${2:-en} SBARHIVE_COLOR_SCHEME=light SBARHIVE_REDUCED_MOTION=1 \
      SBARHIVE_DEBUG_FOCUS=1 WAYLAND_DEBUG=1 "$BIN/sbarhived" >"$DLOG" 2>&1 &
    DPID=$!
    for _ in $(seq 50); do "$BIN/sbarhive" ping >/dev/null 2>&1 && break; sleep 0.1; done
  }
  stop_daemon() {
    [ -n "$DPID" ] || return 0
    "$BIN/sbarhive" quit >/dev/null 2>&1
    timeout 5 tail --pid="$DPID" -f /dev/null || kill "$DPID" 2>/dev/null
    DPID=""
  }
  owned() {
    gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
      --method org.freedesktop.DBus.NameHasOwner org.freedesktop.portal.Desktop 2>/dev/null | grep -q true
  }
  start_fake() { # start_fake NAME MODE
    PLOG="$OUT_LOG-$1-portal.log"
    local fifo="$XDG_RUNTIME_DIR/portal-$1"
    rm -f "$fifo"; mkfifo "$fifo"
    python3 -I "$HERE/shortcuts_portal.py" "$fifo" "$2" >"$PLOG" 2>&1 &
    FAKE_PID=$!
    exec 4>"$fifo"
    for _ in $(seq 50); do grep -q '\] owned org.freedesktop.portal.Desktop' "$PLOG" && break; sleep 0.1; done
  }
  fake() { echo "$*" >&4; }
  stop_fake() {
    [ -n "$FAKE_PID" ] || return 0
    echo quit >&4; exec 4>&-
    timeout 5 tail --pid="$FAKE_PID" -f /dev/null || kill "$FAKE_PID" 2>/dev/null
    FAKE_PID=""
    for _ in $(seq 50); do owned || break; sleep 0.1; done
  }
  stop_all() { stop_daemon; stop_fake; sleep 0.5; }
  reset_config() { printf '%s\n' "$@" >"$XDG_CONFIG_HOME/sbarhive/config.toml"; }
  wait_line() { # wait_line ERE SECONDS: on the daemon's stderr
    local end; end=$(deadline "$2")
    while [ "$(now_ms)" -lt "$end" ]; do
      grep -qE "$1" "$DLOG" && return 0
      sleep 0.1
    done
    return 1
  }
  plines() { grep -E '^portal (shortcuts|registry):' "$DLOG"; }
  nlines() { grep -cE "$1" <<<"$2"; }
  config_of() {
    python3 -I -c 'import sys, tomllib
try:
    c = tomllib.load(open(sys.argv[1], "rb"))
except (OSError, ValueError) as e:
    print("unreadable unreadable"); sys.exit()
print(c.get("portal_shortcut", "absent"), c.get("hold_ms", "absent"))' "$XDG_CONFIG_HOME/sbarhive/config.toml"
  }

  # --- AC13 -------------------------------------------------------------
  OK_UP=no
  ok_daemon() {
    [ "$OK_UP" = yes ] && return 0
    stop_all
    reset_config 'hold_ms = 400'
    start_fake ok ok
    start_daemon ok en
    wait_line '^portal shortcuts: ' 10 || true
    sleep 1
    OK_UP=yes
  }
  phase_ac13() {
    echo "== AC13: fake portal \"ok\", daemon in English"
    ok_daemon
    local o snd first appid nextgs nbind maxper bsess nsc scid trig desc lines
    o=$(python3 -I "$PL" order "$PLOG")
    IFS='|' read -r snd first appid nextgs nbind maxper bsess nsc scid trig desc <<<"$o"
    lines=$(plines)
    echo "INFO AC13 ok: portal connection $snd: first portal method $first ($appid), next GlobalShortcuts $nextgs, BindShortcuts x$nbind (most on one session $maxper, session $bsess), shortcuts $nsc: id $scid, preferred_trigger $trig, description \"$desc\"; daemon lines: $(tr '\n' '|' <<<"$lines")"
    check "AC13 ok: the first portal method call of the daemon's portal connection is org.freedesktop.host.portal.Registry.Register with io.github.m7mmad_omar.SbarHive" \
      '[ "$first" = org.freedesktop.host.portal.Registry.Register ] && [ "$appid" = io.github.m7mmad_omar.SbarHive ]'
    check "AC13 ok: its next GlobalShortcuts method call is CreateSession" '[ "$nextgs" = CreateSession ]'
    check "AC13 ok: exactly one BindShortcuts, on the session CreateSession gave (at most one per session)" \
      '[ "$nbind" = 1 ] && [ "$maxper" = 1 ] && [ "$bsess" = ok ]'
    check "AC13 ok: one shortcut, id \"drop\", preferred_trigger \"LOGO+z\", a non-empty English description" \
      '[ "$nsc" = 1 ] && [ "$scid" = drop ] && [ "$trig" = LOGO+z ] && python3 -I "$PL" lang en "$desc"'
    check "AC13 ok: stderr has exactly one portal line, \"portal shortcuts: bound drop (Fake Super+Z)\" (T is the reply's trigger_description)" \
      '[ "$lines" = "portal shortcuts: bound drop (Fake Super+Z)" ]'
  }
  phase_ac13_noreg() {
    echo "== AC13: fake portal \"no-registry\""
    stop_all; OK_UP=no
    reset_config 'hold_ms = 400'
    start_fake noreg no-registry
    start_daemon noreg en
    wait_line '^portal shortcuts: ' 10 || true
    sleep 1
    local o snd first appid nextgs nbind maxper bsess nsc scid trig desc lines
    o=$(python3 -I "$PL" order "$PLOG")
    IFS='|' read -r snd first appid nextgs nbind maxper bsess nsc scid trig desc <<<"$o"
    lines=$(plines)
    echo "INFO AC13 no-registry: first portal method $first ($appid), next GlobalShortcuts $nextgs, BindShortcuts x$nbind ($bsess); daemon lines: $(tr '\n' '|' <<<"$lines")"
    check "AC13 no-registry: Register is tried first and fails, then CreateSession and one BindShortcuts still follow" \
      '[ "$first" = org.freedesktop.host.portal.Registry.Register ] && grep -qE "\] error [0-9]+ " "$PLOG" && [ "$nextgs" = CreateSession ] && [ "$nbind" = 1 ] && [ "$bsess" = ok ]'
    check "AC13 no-registry: exactly two portal lines, \"portal registry: unavailable\" then \"portal shortcuts: bound drop (Fake Super+Z)\"" \
      '[ "$lines" = "portal registry: unavailable
portal shortcuts: bound drop (Fake Super+Z)" ]'
  }

  # --- AC17, AC18 -------------------------------------------------------
  # pulse_case NAME LINE SECONDS: sends LINE to the fake, waits, and sets
  # SEG (the daemon's lines after the mark), MARK, TA (the fake's first
  # Activated stamp after the mark), NACT and NDEACT (the portal: lines).
  pulse_case() {
    MARK=$(wc -l <"$DLOG")
    local pf; pf=$(wc -l <"$PLOG")
    fake "$2"
    sleep "$3"
    SEG=$(tail -n +$((MARK + 1)) "$DLOG")
    TA=$(python3 -I "$PL" emit "$PLOG" "$pf" Activated)
    NACT=$(grep -cE '^portal: activated drop$' <<<"$SEG")
    NDEACT=$(grep -cE '^portal: deactivated drop$' <<<"$SEG")
    NPORTAL=$(grep -cE '^portal: ' <<<"$SEG")
  }
  settle() { wait_gone 12 || true; "$BIN/sbarhive" drop --release >/dev/null 2>&1; sleep 1; }
  phase_ac17() {
    echo "== AC17, AC18 (a): portal signals on the \"ok\" daemon, hold_ms 400"
    ok_daemon
    local bound l x first all rev hid n r pre att
    bound=$(grep -c '^portal shortcuts: bound drop ' "$DLOG")
    ptr_move 100 700; sleep 0.5

    pulse_case 600 "pulse drop own 600" 2
    read -r l x first <<<"$(surfaces_by "$MARK" "$TA" 1500)"
    IFS='|' read -r all rev hid n r _ pre att _ <<<"$(probe_show "$SEG")"
    echo "INFO AC17 600 ms: Activated at $TA, portal lines activated $NACT deactivated $NDEACT; within 1500 ms: layer $l xdg $x, the first $first ms after Activated; probe lines $all ($(grep -E '^hive (reveal|probe):' <<<"$SEG" | tr '\n' '|')), hive attach $att"
    check "AC17 600 ms: one \"portal: activated drop\" and one \"portal: deactivated drop\" line" '[ "$NACT" -eq 1 ] && [ "$NDEACT" -eq 1 ]'
    check "AC17 600 ms: exactly one get_layer_surface, 400 to 600 ms after Activated (the hold_ms threshold, before the release)" \
      '[ "$l" -eq 1 ] && [ "$x" -eq 0 ] && in_ms "$first" 400 600'
    check "AC18 (a): Activated with no drag, Deactivated after the threshold: exactly one probe line, \"hive probe: hidden (release)\" or \"hive probe: hidden (pointer)\"" \
      '[ "$l" -eq 1 ] && [ "$all" -eq 1 ] && [ "$hid" -eq 1 ] && { [ "$r" = release ] || [ "$r" = pointer ]; }'
    check "AC18 (a): no attach(wl_buffer) on the hive subsurface (nothing drawn before a \"hive reveal:\")" '[ "$l" -eq 1 ] && [ "$att" -eq 0 ]'
    settle

    pulse_case 150 "pulse drop own 150" 1.2
    read -r l x first <<<"$(surfaces_by "$MARK" "$TA" 1000)"
    echo "INFO AC17 150 ms: Activated at $TA, portal lines activated $NACT deactivated $NDEACT; within 1000 ms: layer $l xdg $x"
    check "AC17 150 ms: both signals reach the daemon (one activated, one deactivated line) and no get_layer_surface within 1000 ms" \
      '[ "$NACT" -eq 1 ] && [ "$NDEACT" -eq 1 ] && [ "$l" -eq 0 ] && [ "$x" -eq 0 ]'
    settle

    pulse_case repeat "pulse drop own 600 40 80" 2
    read -r l x first <<<"$(surfaces_by "$MARK" "$TA" 1500)"
    IFS='|' read -r all _ <<<"$(probe_show "$SEG")"
    echo "INFO AC17 repeat: Activated at $TA (and at 40, 80 ms), portal lines activated $NACT deactivated $NDEACT; within 1500 ms: layer $l xdg $x, the first $first ms after the first Activated; probe lines $all"
    check "AC17 repeat: three activated lines and one deactivated line" '[ "$NACT" -eq 3 ] && [ "$NDEACT" -eq 1 ]'
    check "AC17 repeat: three Activated (key repeat) then Deactivated at 600 ms give exactly one get_layer_surface, 400 to 600 ms after the first" \
      '[ "$l" -eq 1 ] && [ "$x" -eq 0 ] && in_ms "$first" 400 600'
    check "AC18 repeat: that show prints exactly one \"hive reveal:\" or \"hive probe:\" line" '[ "$l" -eq 1 ] && [ "$all" -eq 1 ]'
    settle

    pulse_case other "pulse other own 600" 1.2
    read -r l x first <<<"$(surfaces_by "$MARK" "$TA" 1000)"
    echo "INFO AC17 other id: bound lines $bound, portal lines $NPORTAL; within 1000 ms: layer $l xdg $x"
    check "AC17 other id: with the shortcut bound, Activated and Deactivated of \"other\" are ignored (no portal line, no get_layer_surface within 1000 ms)" \
      '[ "$bound" -eq 1 ] && [ "$NPORTAL" -eq 0 ] && [ "$l" -eq 0 ] && [ "$x" -eq 0 ]'
    settle

    pulse_case session "pulse drop other 600" 1.2
    read -r l x first <<<"$(surfaces_by "$MARK" "$TA" 1000)"
    echo "INFO AC17 other session: bound lines $bound, portal lines $NPORTAL; within 1000 ms: layer $l xdg $x"
    check "AC17 other session: with the shortcut bound, \"drop\" signals on another session's path are ignored (no portal line, no get_layer_surface within 1000 ms)" \
      '[ "$bound" -eq 1 ] && [ "$NPORTAL" -eq 0 ] && [ "$l" -eq 0 ] && [ "$x" -eq 0 ]'
    settle

    # One gate: a press from IPC and a release from the portal, and back.
    MARK=$(wc -l <"$DLOG"); local t0; t0=$(date -u +%T.%6N)
    cli_ms drop --hold
    sleep 0.15; fake "signal Deactivated drop"; sleep 1.2
    SEG=$(tail -n +$((MARK + 1)) "$DLOG")
    NDEACT=$(grep -cE '^portal: deactivated drop$' <<<"$SEG")
    read -r l x first <<<"$(surfaces_by "$MARK" "$t0" 1000)"
    echo "INFO AC17 one gate, IPC press: exit $CLI_RC, deactivated lines $NDEACT; within 1000 ms: layer $l xdg $x"
    check "AC17 one gate: \`drop --hold\`, then the portal's Deactivated 150 ms later, cancels that press's timer (a deactivated line, no get_layer_surface within 1000 ms)" \
      '[ "$CLI_RC" = 0 ] && [ "$NDEACT" -eq 1 ] && [ "$l" -eq 0 ] && [ "$x" -eq 0 ]'
    settle
    MARK=$(wc -l <"$DLOG"); local pf; pf=$(wc -l <"$PLOG")
    fake "signal Activated drop"; sleep 0.15
    cli_ms drop --release; sleep 1.2
    SEG=$(tail -n +$((MARK + 1)) "$DLOG")
    TA=$(python3 -I "$PL" emit "$PLOG" "$pf" Activated)
    NACT=$(grep -cE '^portal: activated drop$' <<<"$SEG")
    read -r l x first <<<"$(surfaces_by "$MARK" "$TA" 1000)"
    echo "INFO AC17 one gate, portal press: Activated at $TA, activated lines $NACT, release exit $CLI_RC; within 1000 ms: layer $l xdg $x"
    check "AC17 one gate: the portal's Activated, then \`drop --release\` 150 ms later, cancels its timer (an activated line, no get_layer_surface within 1000 ms)" \
      '[ "$CLI_RC" = 0 ] && [ "$NACT" -eq 1 ] && [ "$l" -eq 0 ] && [ "$x" -eq 0 ]'
    settle

    # AC18 (b): a held PNG drag, Activated, the drop on WEBP, Deactivated.
    echo "== AC18 (b): drag, Activated, drop on WEBP, Deactivated"
    local file="$W/portal drag.png" geo="" wrote=no mime nact
    cp "$ROOT/tests/fixtures/images/alpha.png" "$file"
    printf '%s\n' "$file" >"$NEXT"
    nact=$("$BIN/sbarhive" actions "$file" 2>/dev/null | grep -c .)
    drag_past_threshold
    MARK=$(wc -l <"$DLOG")
    jitter 400 500 1000
    fake "signal Activated drop"
    geo=$(wait_geo 3) || true
    sleep 1
    [ -n "$geo" ] && geo=$(hive_geo)
    # shellcheck disable=SC2086
    [ -n "$geo" ] && move_to_webp $geo
    ptr_up
    wait_file "$W/portal drag.webp" 5 && wrote=yes
    fake "signal Deactivated drop"
    sleep 1; wait_gone 12 || true
    SEG=$(tail -n +$((MARK + 1)) "$DLOG")
    mime=$(file --mime-type -b "$W/portal drag.webp" 2>/dev/null)
    probe_checks "AC18 (b)" "$SEG" reveal
    IFS='|' read -r _ _ _ n _ <<<"$PROBE"
    check "AC18 (b): \"hive reveal: N\" with N the $nact lines of \`sbarhive actions\` for the PNG" '[ "$n" != none ] && [ "$nact" -ge 1 ] && [ "$n" -eq "$nact" ]'
    check "AC18 (b): the drop on the WEBP cell before Deactivated writes portal drag.webp within 5 s (image/webp)" '[ "$wrote" = yes ] && [ "$mime" = image/webp ]'
    local cat ent ex="" ey="" anchors bad=0 a
    cat=$(grep -oE -- '-> zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(new id zwlr_layer_surface_v1[#@][0-9]+, wl_surface[#@][0-9]+' <<<"$SEG" | head -1 | grep -oE 'wl_surface[#@][0-9]+$')
    [ -n "$cat" ] && ent=$(grep -m1 -E -- "^\[[0-9:.]+\] wl_data_device[#@][0-9]+\.enter\([0-9]+, ${cat}, " <<<"$SEG")
    [ -n "${ent:-}" ] && read -r ex ey < <(sed -nE 's/.*enter\([0-9]+, wl_surface[#@][0-9]+, (-?[0-9.]+), (-?[0-9.]+),.*/\1 \2/p' <<<"$ent")
    anchors=$(grep -E '^hive anchor: ' <<<"$SEG")
    while IFS= read -r a; do
      [ -z "$a" ] && continue
      [ "$a" = "hive anchor: none" ] && continue
      awk -v l="$a" -v x="${ex:-}" -v y="${ey:-}" 'BEGIN { split(l, p, " "); if (x == "" || y == "") exit 1; dx = p[3] - x; dy = p[4] - y; if (dx < 0) dx = -dx; if (dy < 0) dy = -dy; exit !(dx <= 2 && dy <= 2) }' || bad=$((bad + 1))
    done <<<"$anchors"
    echo "INFO AC18 (b): hive ${geo:-not shown}; first drag enter on the catcher ${cat:-none} at ${ex:-?} ${ey:-?}; anchor lines $(tr '\n' '|' <<<"${anchors:-none}"); $(basename "$W/portal drag.webp") ${mime:-not written}"
    check "AC18 (b): at least one \"hive anchor:\" line, each \"none\" or within 2 px of the first drag enter on the catcher" \
      '[ -n "$anchors" ] && [ "$bad" -eq 0 ]'
    settle
  }

  # --- AC19 -------------------------------------------------------------
  # hold_cycle NAME: `drop --hold --at 640,400` with the pointer outside the
  # hive, then `drop --release` (S-019e, hold_ms 500).
  hold_cycle() {
    local l x first d mp tp mr trel prc pms
    ptr_move 100 700; sleep 0.5
    mp=$(wc -l <"$DLOG"); tp=$(date -u +%T.%6N)
    cli_ms drop --hold --at 640,400; prc=$CLI_RC; pms=$CLI_MS
    sleep 1
    read -r l x first <<<"$(surfaces_by "$mp" "$tp" 1000)"
    mr=$(wc -l <"$DLOG"); trel=$(date -u +%T.%6N)
    cli_ms drop --release
    sleep 0.8
    d=$(destroyed_after "$mr" "$trel")
    echo "INFO AC19 $1 hold: press exit $prc in $pms ms, within 1000 ms layer $l xdg $x, the first $first ms after it; release exit $CLI_RC, layer destroyed $d ms after it"
    check "AC19 $1: after the line, \`drop --hold\` still shows one layer surface 450 to 700 ms after the press (hold_ms 500) and \`drop --release\` destroys it within 500 ms" \
      '[ "$HAS_LINE" = yes ] && cli_ok "$prc" "$pms" && [ "$l" -eq 1 ] && [ "$x" -eq 0 ] && in_ms "$first" 450 700 && cli_ok "$CLI_RC" "$CLI_MS" && in_ms "$d" 0 500'
    wait_gone 12 || true
  }
  # ac19_mode NAME MODE REASON LANG ("absent": no fake)
  ac19_mode() {
    local name=$1 mode=$2 reason=$3 lang=$4
    echo "== AC19 $name: fake ${mode}, daemon in $lang"
    stop_all; OK_UP=no
    reset_config 'hold_ms = 500'
    if [ "$mode" = absent ]; then
      PLOG="$OUT_LOG-$name-portal.log"; : >"$PLOG"
      check "AC19 $name setup: nothing owns org.freedesktop.portal.Desktop" '! owned'
    else
      start_fake "$name" "$mode"
    fi
    start_daemon "$name" "$lang"
    HAS_LINE=no
    wait_line '^portal shortcuts: ' 10 && HAS_LINE=yes
    local t_line pf
    t_line=$(now_ms); pf=$(wc -l <"$PLOG")
    if [ "$mode" = absent ]; then
      # A fake appears after the line: any retry would reach it.
      sleep 1; start_fake "$name-late" ok; pf=0
    fi
    sleep "$(awk -v e="$((t_line + 30000 - $(now_ms)))" 'BEGIN { s = e / 1000; if (s < 0) s = 0; printf "%.3f\n", s }')"
    local lines ncalls
    lines=$(grep -E '^portal shortcuts:' "$DLOG")
    ncalls=$(python3 -I "$PL" calls "$PLOG" "$pf")
    echo "INFO AC19 $name: daemon lines $(plines | tr '\n' '|'); fake calls and reads in the 30 s after the line: $ncalls"
    check "AC19 $name: exactly one \"portal shortcuts:\" line, \"portal shortcuts: unavailable ($reason)\"" \
      '[ "$lines" = "portal shortcuts: unavailable ($reason)" ]'
    check "AC19 $name: for 30 s after it, no new call or property read on the portal (the fake's counter) and no other \"portal shortcuts:\" line" \
      '[ "$HAS_LINE" = yes ] && [ "$ncalls" -eq 0 ] && [ "$(nlines "^portal shortcuts:" "$lines")" -eq 1 ]'
    if [ "$lang" = ar ] && [ "$mode" != absent ]; then
      local desc; IFS='|' read -r _ _ _ _ _ _ _ _ _ _ desc <<<"$(python3 -I "$PL" order "$PLOG")"
      echo "INFO AC13 $name (Arabic daemon): BindShortcuts description \"$desc\""
      check "AC13 $name: the daemon in Arabic sends an Arabic description" 'python3 -I "$PL" lang ar "$desc"'
    fi
    hold_cycle "$name"
    local cfg; cfg=$(config_of)
    echo "INFO AC19 $name: config.toml (portal_shortcut hold_ms): $cfg"
    case "$reason" in
      denied)
        check "AC19 $name (answer 5): the denial is remembered, config.toml has portal_shortcut = false and still hold_ms = 500" '[ "$cfg" = "False 500" ]' ;;
      *)
        check "AC19 $name (answer 5): only a denial is remembered, config.toml has no portal_shortcut = false and still hold_ms = 500" \
          '[ "$HAS_LINE" = yes ] && [ "${cfg#* }" = 500 ] && [ "${cfg%% *}" != False ]' ;;
    esac
  }
  phase_ac19() {
    ac19_mode no-portal absent no-portal en
    ac19_mode no-gs no-gs no-interface en
    ac19_mode version0 version0 no-interface en
    ac19_mode error error error ar
    ac19_mode empty empty denied en
    ac19_mode deny deny denied en
    # Answer 5: the next daemon on this profile does not register.
    echo "== AC19 deny, answer 5: a new daemon on the same profile"
    stop_daemon
    local pf lines n
    pf=$(wc -l <"$PLOG")
    start_daemon deny-again en
    wait_line '^portal shortcuts: ' 10 || true
    sleep 5
    lines=$(grep -E '^portal shortcuts:' "$DLOG")
    n=$(python3 -I "$PL" calls "$PLOG" "$pf")
    echo "INFO AC19 deny again: daemon lines $(tr '\n' '|' <<<"$lines"); fake calls and reads since its start: $n"
    check "AC19 deny again (answer 5): with portal_shortcut = false the daemon makes no portal call or read (not even Register) and prints exactly \"portal shortcuts: unavailable (off)\"" \
      '[ "$lines" = "portal shortcuts: unavailable (off)" ] && [ "$n" -eq 0 ]'
    stop_daemon
    reset_config 'hold_ms = 500' 'portal_shortcut = true'
    pf=$(wc -l <"$PLOG")
    start_daemon deny-on en
    wait_line '^portal shortcuts: ' 10 || true
    sleep 1
    lines=$(grep -E '^portal shortcuts:' "$DLOG")
    n=$(python3 -I "$PL" calls "$PLOG" "$pf" Register)
    echo "INFO AC19 deny, turned back on: daemon lines $(tr '\n' '|' <<<"$lines"); Register calls since its start: $n"
    check "AC19 deny, turned back on (answer 5): with portal_shortcut = true the next daemon registers again (Register, and \"portal shortcuts: unavailable (denied)\" from this fake)" \
      '[ "$n" -ge 1 ] && [ "$lines" = "portal shortcuts: unavailable (denied)" ]'
    stop_all
  }

  # --- AC19, the held BindShortcuts Response ------------------------------
  # first_frame SEG STAMP: ms from STAMP to the first attach(wl_buffer) on
  # the hive surface of the show (the subsurface when the show has one,
  # else the layer surface's wl_surface).
  first_frame() {
    local seg=$1 ls child target a
    ls=$(grep -oE -- '-> zwlr_layer_shell_v1[#@][0-9]+\.get_layer_surface\(new id zwlr_layer_surface_v1[#@][0-9]+, wl_surface[#@][0-9]+' <<<"$seg" | head -1 | grep -oE 'wl_surface[#@][0-9]+$')
    child=$(grep -oE -- '-> wl_subcompositor[#@][0-9]+\.get_subsurface\(new id wl_subsurface[#@][0-9]+, wl_surface[#@][0-9]+' <<<"$seg" | head -1 | grep -oE 'wl_surface[#@][0-9]+$')
    target=${child:-$ls}
    [ -n "$target" ] || { echo none; return; }
    a=$(grep -m1 -E -- "-> ${target}\.attach\(wl_buffer" <<<"$seg")
    [ -n "$a" ] && ms_after "$2" "$a" || echo none
  }
  phase_hold() {
    echo "== AC19 hold: the fake holds the BindShortcuts Response, five \`sbarhive open\`"
    stop_all; OK_UP=no
    reset_config 'hold_ms = 400'
    start_fake hold hold
    start_daemon hold en
    local end held_at=no; end=$(deadline 10)
    while [ "$(now_ms)" -lt "$end" ]; do grep -qE '\] held [0-9]+ ' "$PLOG" && { held_at=yes; break; }; sleep 0.1; done
    local i m t0 seg ms shown=0 dts=()
    for i in 1 2 3 4 5; do
      m=$(wc -l <"$DLOG"); t0=$(date -u +%T.%6N)
      "$BIN/sbarhive" open "$W/open me.png" >>"$OUT_LOG-cli.log" 2>&1
      wait_geo 3 >/dev/null && shown=$((shown + 1))
      sleep 0.3
      seg=$(tail -n +$((m + 1)) "$DLOG")
      ms=$(first_frame "$seg" "$t0"); dts+=("$ms")
      press_esc; sleep 0.3
      wait_gone 3 || { ptr_move 5 5; ptr_down; ptr_up; wait_gone 3; } || true
      sleep 0.5
    done
    local still before median
    still=$(python3 -I "$PL" held "$PLOG")
    before=$(grep -cE '^portal shortcuts:' "$DLOG")
    median=$(printf '%s\n' "${dts[@]}" | python3 -I -c 'import statistics, sys
v = [l.strip() for l in sys.stdin if l.strip()]
print("none" if any(x == "none" for x in v) or len(v) != 5 else "%.1f" % statistics.median(float(x) for x in v))')
    echo "INFO AC19 hold: BindShortcuts held ${held_at} before the shows, still held after them $still, portal shortcuts lines before the Response $before; shows $shown of 5, ms from the command to the first frame: ${dts[*]} (median $median)"
    check "AC19 hold: while the fake holds the BindShortcuts Response (held before the shows and after them, no \"portal shortcuts:\" line yet), \`sbarhive open\` shows the hive 5 of 5 times" \
      '[ "$held_at" = yes ] && [ "$still" = yes ] && [ "$before" -eq 0 ] && [ "$shown" -eq 5 ]'
    check "AC19 hold: the median of the 5 shows from the request to the first frame is under 150 ms" \
      '[ "$held_at" = yes ] && [ "$median" != none ] && awk -v m="$median" "BEGIN { exit !(m + 0 < 150) }"'
    fake respond
    wait_line '^portal shortcuts: ' 5 || true
    local lines; lines=$(grep -E '^portal shortcuts:' "$DLOG")
    check "AC19 hold: after the Response, exactly one line \"portal shortcuts: bound drop (Fake Super+Z)\"" \
      '[ "$lines" = "portal shortcuts: bound drop (Fake Super+Z)" ]'
    stop_all
  }

  local p
  for p in $PHASES; do
    case "$p" in
      ac13) phase_ac13 ;;
      ac17) phase_ac17 ;;
      ac19) phase_ac19 ;;
      hold) phase_hold ;;
    esac
  done
  case "$PHASES" in *ac13*) phase_ac13_noreg ;; esac
  stop_all
  summary

  exec 3>&-
  kill "$src" 2>/dev/null
  kill "$kwin" 2>/dev/null
  timeout 5 tail --pid="$kwin" -f /dev/null || kill -9 "$kwin"
}
export -f inner

echo "== kwin portal: $PHASES"
SANDBOX="$(mktemp -d)"
RUNDIR="$(mktemp -d)"
trap 'rm -rf "$SANDBOX" "$RUNDIR"' EXIT
# The private bus's configuration: the stock session policy, and one empty
# service directory instead of the standard ones, so nothing is activated.
mkdir -p "$SANDBOX/dbus-services"
cat >"$RUNDIR/bus.conf" <<EOF
<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig>
  <type>session</type>
  <keep_umask/>
  <listen>unix:dir=$RUNDIR</listen>
  <auth>EXTERNAL</auth>
  <servicedir>$SANDBOX/dbus-services</servicedir>
  <policy context="default">
    <allow send_destination="*" eavesdrop="true"/>
    <allow eavesdrop="true"/>
    <allow own="*"/>
  </policy>
</busconfig>
EOF
# Environment set BEFORE dbus-run-session, so nothing on the private bus
# can reach the live desktop or the real profile.
env -u DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u XDG_CURRENT_DESKTOP -u DBUS_SESSION_BUS_ADDRESS \
  HOME="$SANDBOX" XDG_CONFIG_HOME="$SANDBOX/.config" XDG_DATA_HOME="$SANDBOX/.local/share" \
  XDG_CACHE_HOME="$SANDBOX/.cache" XDG_STATE_HOME="$SANDBOX/.local/state" \
  XDG_DATA_DIRS="$SANDBOX/.local/share:/usr/local/share:/usr/share" \
  XDG_RUNTIME_DIR="$RUNDIR" WAYLAND_DISPLAY=wayland-sbh-drop-portal \
  dbus-run-session --config-file="$RUNDIR/bus.conf" -- bash -c inner 2>"$OUT/$TAG-bus.log" | tee "$OUT/$TAG.txt"
grep -q '^RESULT PASS' "$OUT/$TAG.txt"
