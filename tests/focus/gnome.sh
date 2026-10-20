#!/usr/bin/env bash
# Does the hive get keyboard focus on GNOME (Mutter) Wayland when the daemon
# opens it, with and without an xdg-activation token?
# Runs in a private headless gnome-shell with a throwaway HOME, XDG dirs,
# runtime dir and session bus. Never touches the live desktop or profile.
# Usage: tests/focus/gnome.sh [notoken|token|bogus|nautilus ...] [smart|strict] [target/release]
set -u
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
CASES=()
FOCUS_NEW=smart
BIN="$ROOT/target/release"
for a in "$@"; do
  case "$a" in notoken|token|bogus|nautilus) CASES+=("$a") ;; strict|smart) FOCUS_NEW="$a" ;; *) BIN="$a" ;; esac
done
[ ${#CASES[@]} -eq 0 ] && CASES=(notoken token)
[ -x "$BIN/sbarhive" ] && [ -x "$BIN/sbarhived" ] || { echo "build first: cargo build --release"; exit 2; }
OUT="${FOCUS_OUT:-$HERE/out}"
mkdir -p "$OUT"
export BIN HERE ROOT OUT FOCUS_NEW

inner() {
  set -u
  unset DISPLAY XDG_ACTIVATION_TOKEN DESKTOP_STARTUP_ID HYPRLAND_INSTANCE_SIGNATURE XDG_CURRENT_DESKTOP XDG_SESSION_TYPE
  local log="$OUT/gnome-$FOCUS_NEW-$CASE"
  # Mutter's new-window focus policy: smart (default) or strict. This writes
  # the throwaway HOME's dconf through the private bus only.
  gsettings set org.gnome.desktop.wm.preferences focus-new-windows "$FOCUS_NEW"
  # --unsafe-mode only opens Shell.Eval and Shell.Screenshot to this test on
  # the private bus; it does not change window focus policy.
  gnome-shell --headless --wayland --no-x11 --unsafe-mode --wayland-display "$WAYLAND_DISPLAY" \
    --virtual-monitor 1280x800 >"$log-shell.log" 2>&1 &
  local shell=$!
  for _ in $(seq 150); do [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] && break; sleep 0.1; done
  [ -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY" ] || { echo "gnome-shell did not start"; tail "$log-shell.log"; return 1; }
  sleep 3

  shell_eval() {
    gdbus call --session --dest org.gnome.Shell --object-path /org/gnome/Shell \
      --method org.gnome.Shell.Eval "$1" 2>&1
  }
  state() {
    shell_eval '(() => {
      const f = global.display.focus_window;
      const wins = global.get_window_actors().map(a => a.meta_window);
      return JSON.stringify({
        focus: f ? f.get_title() : null,
        attention: wins.filter(w => w.demands_attention).map(w => w.get_title()),
        windows: wins.map(w => { const r = w.get_frame_rect(); return `${w.get_title()}(pid ${w.get_pid()} ${r.width}x${r.height}+${r.x}+${r.y}${w.minimized ? " min" : ""})`; }),
        notifications: Main.messageTray.getSources().flatMap(s => s.notifications.map(n => n.title)),
        overview: Main.overview.visible,
      }); })()'
  }
  shot() {
    gdbus call --session --dest org.gnome.Shell --object-path /org/gnome/Shell/Screenshot \
      --method org.gnome.Shell.Screenshot.Screenshot false false "$1" >/dev/null 2>&1
  }

  # GNOME starts in the overview, where no window has focus and typed keys
  # go to the search entry. Leave it, as a user would after logging in.
  for _ in $(seq 20); do
    shell_eval 'Main.overview.hide(); Main.overview.visible' | grep -q false && break
    sleep 0.5
  done
  sleep 1

  local fifo="$XDG_RUNTIME_DIR/input.fifo"
  mkfifo "$fifo"
  python3 "$HERE/input_gnome.py" "$fifo" >"$log-input.log" 2>&1 &
  local input=$!
  key() { echo "key $1" >"$fifo"; sleep 0.4; }
  sleep 1
  # Mutter adds the virtual keyboard to the seat on the first event. Clients
  # started before that never get key events, so plug it in now.
  key Shift_L

  local W="$XDG_RUNTIME_DIR/work"; mkdir -p "$W"
  ffmpeg -loglevel error -f lavfi -i testsrc=size=80x60:rate=1 -frames:v 1 "$W/b.jpg"

  SBARHIVE_DEBUG_FOCUS=1 WAYLAND_DEBUG=1 "$BIN/sbarhived" >"$log-daemon.log" 2>&1 &
  local d=$!
  for _ in $(seq 50); do "$BIN/sbarhive" ping >/dev/null 2>&1 && break; sleep 0.1; done
  # The first show after the daemon starts can take several seconds (ADR 0001,
  # finding 8). Show once and wait, so the measured show is a warm one.
  # NO_WARMUP=1 skips it, so the measured show is the first one.
  if [ -z "${NO_WARMUP:-}" ]; then
    "$BIN/sbarhive" open "$W/b.jpg"; sleep 10
    echo "warm-up: $(state)"
    key Escape
  else
    # Same timing without the show: the virtual keyboard needs this settling
    # time before a client started later gets key events.
    sleep 10; key Escape
  fi
  # Without focus Esc does not reach the hive; a new present hides it anyway.

  case "$CASE" in
    notoken|token|bogus)
      SBARHIVE="$BIN/sbarhive" python3 "$HERE/client.py" "$W/b.jpg" >"$log-client.log" 2>&1 &
      local client=$!
      sleep 3
      # Under the strict policy a window started without a token never gets
      # focus; the file manager would have it from the user's click. Give it
      # focus the way a click would, then type into it.
      for _ in $(seq 20); do
        shell_eval 'global.get_window_actors().map(a => a.meta_window)
          .filter(w => w.get_title() === "focus-client")
          .forEach(w => w.activate(global.get_current_time()))' >/dev/null
        sleep 0.5
        grep -q "focus True" "$log-client.log" && break
      done
      key a; key b
      echo "before: $(state)"
      local mark; mark=$(wc -l <"$log-daemon.log")
      case "$CASE" in token) key F5 ;; bogus) key F7 ;; *) key F6 ;; esac
      sleep 4
      echo "after:  $(state)"
      shot "$log.png"
      echo "daemon focus lines: $(tail -n +$((mark + 1)) "$log-daemon.log" | grep 'hive focus' | tr '\n' ' ')"
      echo "client: $(grep -E 'token|focus' "$log-client.log" | tr '\n' ' ')"
      # Esc reaches the hive only when it has the keyboard.
      key Escape; sleep 1
      echo "after Esc: $(state)"
      kill $client 2>/dev/null
      ;;
    nautilus)
      local ext="$XDG_DATA_HOME/nautilus-python/extensions"
      mkdir -p "$ext"
      sed "s|^SBARHIVE_BIN = .*|SBARHIVE_BIN = \"$BIN/sbarhive\"|" \
        "$ROOT/integrations/linux/nautilus/sbarhive.py" >"$ext/sbarhive.py"
      nautilus --new-window "$W" >"$log-nautilus.log" 2>&1 &
      local naut=$!
      sleep 6
      # Other nautilus-python extensions installed system-wide can open their
      # own windows (Nextcloud's setup wizard did); close them, then give the
      # Nautilus window focus the way a click would.
      shell_eval 'global.get_window_actors().map(a => a.meta_window)
        .filter(w => w.get_title() !== "work" && w.get_title() !== "SbarHive")
        .forEach(w => w.delete(global.get_current_time()))' >/dev/null
      sleep 1
      shell_eval 'global.get_window_actors().map(a => a.meta_window)
        .filter(w => w.get_title() === "work")
        .forEach(w => w.activate(global.get_current_time()))' >/dev/null
      sleep 1
      echo "before: $(state)"
      # Select the only file, open the context menu, pick the SbarHive entry.
      key Home; key Control_L+a; key Shift_L+F10; sleep 1
      shot "$log-menu.png"
      local mark; mark=$(wc -l <"$log-daemon.log")
      echo "menu:   $(state)"
      # SbarHive is the entry above Properties, the last one (Nautilus 49).
      for k in ${NAUTILUS_MENU_KEYS:-End Up}; do key "$k"; done
      key Return
      sleep 4
      echo "after:  $(state)"
      shot "$log.png"
      echo "daemon focus lines: $(tail -n +$((mark + 1)) "$log-daemon.log" | grep 'hive focus' | tr '\n' ' ')"
      key Escape; sleep 1
      echo "after Esc: $(state)"
      kill $naut 2>/dev/null
      ;;
  esac

  "$BIN/sbarhive" quit >/dev/null 2>&1
  echo quit >"$fifo"
  sleep 0.5
  kill $d $input 2>/dev/null
  kill $shell 2>/dev/null
  timeout 5 tail --pid=$shell -f /dev/null || kill -9 $shell
}
export -f inner

for CASE in "${CASES[@]}"; do
  echo "== gnome $FOCUS_NEW $CASE"
  export CASE
  SANDBOX="$(mktemp -d)"
  # Environment set BEFORE dbus-run-session, so nothing the private bus
  # activates (portals, dconf) can reach the live desktop or the real profile.
  env -u DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE -u XDG_CURRENT_DESKTOP \
    HOME="$SANDBOX" XDG_CONFIG_HOME="$SANDBOX/.config" XDG_DATA_HOME="$SANDBOX/.local/share" \
    XDG_CACHE_HOME="$SANDBOX/.cache" XDG_STATE_HOME="$SANDBOX/.local/state" \
    XDG_RUNTIME_DIR="$(mktemp -d)" WAYLAND_DISPLAY=wayland-sbh-focus \
    dbus-run-session -- bash -c inner 2>"$OUT/gnome-$FOCUS_NEW-$CASE-bus.log" | tee "$OUT/gnome-$FOCUS_NEW-$CASE.txt"
done
