# SPDX-License-Identifier: GPL-3.0-or-later
"""Keyboard for a headless GNOME Shell through Mutter's RemoteDesktop API.

Opens a RemoteDesktop session on the (private) session bus, which adds a
virtual keyboard to the seat, then reads commands from a FIFO, one per line:
  key NAME[+NAME...]   press the keysyms in order, release them in reverse
  quit
Mutter ends the session when this process leaves the bus, so it stays up.
Usage: input_gnome.py FIFO
"""

import sys
import time

from gi.repository import Gio, GLib

KEYSYMS = {
    "Escape": 0xFF1B, "Return": 0xFF0D, "Tab": 0xFF09, "Down": 0xFF54,
    "Up": 0xFF52, "Menu": 0xFF67, "Shift_L": 0xFFE1, "Control_L": 0xFFE3,
    "F5": 0xFFC2, "F6": 0xFFC3, "F7": 0xFFC4, "F10": 0xFFC7, "Home": 0xFF50, "End": 0xFF57,
}


def keysym(name):
    if name in KEYSYMS:
        return KEYSYMS[name]
    if len(name) == 1:
        return ord(name)
    raise ValueError(name)


def call(bus, path, iface, method, args=None, sig=None):
    return bus.call_sync(
        "org.gnome.Mutter.RemoteDesktop", path, iface, method, args,
        GLib.VariantType(sig) if sig else None, Gio.DBusCallFlags.NONE, 5000, None)


def main():
    fifo = sys.argv[1]
    bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    (session,) = call(bus, "/org/gnome/Mutter/RemoteDesktop",
                      "org.gnome.Mutter.RemoteDesktop", "CreateSession", None, "(o)")
    iface = "org.gnome.Mutter.RemoteDesktop.Session"
    call(bus, session, iface, "Start")
    print("input: session", session, flush=True)

    def press(names):
        syms = [keysym(n) for n in names]
        for s in syms:
            call(bus, session, iface, "NotifyKeyboardKeysym", GLib.Variant("(ub)", (s, True)))
            time.sleep(0.03)
        for s in reversed(syms):
            call(bus, session, iface, "NotifyKeyboardKeysym", GLib.Variant("(ub)", (s, False)))
            time.sleep(0.03)

    while True:
        with open(fifo) as f:
            for line in f:
                parts = line.split()
                if not parts:
                    continue
                if parts[0] == "quit":
                    call(bus, session, iface, "Stop")
                    return
                if parts[0] == "key":
                    press(parts[1].split("+"))
                    print("input: key", parts[1], flush=True)


main()
