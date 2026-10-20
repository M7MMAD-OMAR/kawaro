# SPDX-License-Identifier: GPL-3.0-or-later
"""Pointer and keyboard for a private headless GNOME Shell through Mutter's
RemoteDesktop API, on the private session bus only (from the drop spike).
Commands from a FIFO, one per line:
  to X Y        move the pointer to X Y (relative steps from the current
                position, read with Shell.Eval; absolute motion needs a
                ScreenCast stream)
  down | up     left button
  key A+B       press keysyms in order, release in reverse
  keydown A     press and hold one keysym (Shift held at the drop)
  keyup A       release it
  quit
Usage: input_gnome.py FIFO"""
import json
import sys
import time

from gi.repository import Gio, GLib

KEYSYMS = {"Super_L": 0xFFEB, "Control_L": 0xFFE3, "Shift_L": 0xFFE1, "Escape": 0xFF1B, "z": 0x7A}
BTN_LEFT = 0x110


def main():
    fifo = sys.argv[1]
    bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)

    def rd(path, method, args=None, sig=None, iface="org.gnome.Mutter.RemoteDesktop.Session"):
        return bus.call_sync("org.gnome.Mutter.RemoteDesktop", path, iface, method, args,
                             GLib.VariantType(sig) if sig else None, Gio.DBusCallFlags.NONE, 5000, None)

    def pointer():
        ok, out = bus.call_sync("org.gnome.Shell", "/org/gnome/Shell", "org.gnome.Shell", "Eval",
                                GLib.Variant("(s)", ("JSON.stringify(global.get_pointer().slice(0, 2))",)),
                                GLib.VariantType("(bs)"), Gio.DBusCallFlags.NONE, 5000, None).unpack()
        return json.loads(json.loads(out) if out.startswith('"') else out)

    (session,) = rd("/org/gnome/Mutter/RemoteDesktop", "CreateSession", None, "(o)",
                    iface="org.gnome.Mutter.RemoteDesktop")
    rd(session, "Start")
    print("input: session", session, flush=True)

    def sym(name):
        return KEYSYMS.get(name, ord(name[0]))

    def keysym(name, down):
        rd(session, "NotifyKeyboardKeysym", GLib.Variant("(ub)", (sym(name), down)))

    def to(x, y):
        for _ in range(20):
            cx, cy = pointer()
            dx, dy = x - cx, y - cy
            if abs(dx) < 1 and abs(dy) < 1:
                return
            # Small steps, like a hand, so clients see motion events.
            n = max(1, int(max(abs(dx), abs(dy)) // 20))
            for _ in range(n):
                rd(session, "NotifyPointerMotionRelative", GLib.Variant("(dd)", (dx / n, dy / n)))
                time.sleep(0.02)
            time.sleep(0.05)

    while True:
        with open(fifo) as f:
            for line in f:
                p = line.split()
                if not p:
                    continue
                if p[0] == "quit":
                    rd(session, "Stop")
                    return
                if p[0] == "to":
                    to(float(p[1]), float(p[2]))
                    print("input: at", pointer(), flush=True)
                elif p[0] in ("down", "up"):
                    rd(session, "NotifyPointerButton", GLib.Variant("(ib)", (BTN_LEFT, p[0] == "down")))
                    print("input:", p[0], flush=True)
                elif p[0] in ("keydown", "keyup"):
                    keysym(p[1], p[0] == "keydown")
                    print("input:", p[0], p[1], flush=True)
                elif p[0] == "key":
                    names = p[1].split("+")
                    for n in names:
                        keysym(n, True)
                        time.sleep(0.03)
                    for n in reversed(names):
                        keysym(n, False)
                        time.sleep(0.03)
                    print("input: key", p[1], flush=True)


main()
