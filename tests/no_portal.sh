# shellcheck shell=bash
# Keeps the installed xdg-desktop-portal out of a test's private session bus.
#
# Every private compositor script runs dbus-run-session with the default
# service directories, so anything on that bus could activate the real
# org.freedesktop.portal.Desktop: the daemon (its GlobalShortcuts client is on
# by default since S-014e1), GTK test helpers and ibus-extension-gtk3 under
# gnome-shell. The regression harness must never start it (manager decision,
# S-014e1 review 1). Only tests/drop/portal.sh talks to a portal, its fake.
#
# no_real_portal SANDBOX, called before dbus-run-session, with the session's
# XDG_DATA_HOME="$SANDBOX/.local/share" and XDG_CONFIG_HOME="$SANDBOX/.config":
#   1. A user service file for org.freedesktop.portal.Desktop whose Exec is
#      /bin/false. dbus-daemon reads $XDG_DATA_HOME/dbus-1/services before
#      the system directories, so an activation fails at once instead of
#      starting /usr/libexec/xdg-desktop-portal. A fake that owns the name
#      itself (tests/overlay/portal.py) is not affected.
#   2. config.toml with portal_shortcut = false, so the daemon makes no portal
#      call and prints "portal shortcuts: unavailable (off)". A script that
#      rewrites config.toml later keeps that line in every write.
no_real_portal() {
  local svc="$1/.local/share/dbus-1/services"
  mkdir -p "$svc" "$1/.config/sbarhive"
  printf '[D-BUS Service]\nName=org.freedesktop.portal.Desktop\nExec=/bin/false\n' \
    >"$svc/org.freedesktop.portal.Desktop.service"
  printf 'portal_shortcut = false\n' >"$1/.config/sbarhive/config.toml"
}

# No test daemon may send a real desktop notification (notify.rs reads this);
# every compositor script that starts the daemon sources this file.
export SBARHIVE_NO_NOTIFY=1
