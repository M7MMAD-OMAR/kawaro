#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""A fake accessibility bus for S-018 AC12, on the private session bus only.

Owns org.a11y.Bus and serves /org/a11y/bus with:
  org.a11y.Bus     GetAddress() -> s (a dummy address, nothing listens there)
  org.a11y.Status  IsEnabled (b), ScreenReaderEnabled (b), read and write
through org.freedesktop.DBus.Properties (Get, GetAll, Set), and emits
org.freedesktop.DBus.Properties.PropertiesChanged on /org/a11y/bus when a
value changes. Commands from the control file, one per line: "on", "off",
"quit". Logs every call it gets.
Usage: a11y.py CONTROL_FIFO true|false"""
import sys
import threading

import gi

gi.require_version("Gio", "2.0")
from gi.repository import Gio, GLib  # noqa: E402

PATH = "/org/a11y/bus"
STATUS = "org.a11y.Status"
XML = """
<node>
  <interface name="org.a11y.Bus">
    <method name="GetAddress"><arg type="s" direction="out"/></method>
  </interface>
  <interface name="org.a11y.Status">
    <property name="IsEnabled" type="b" access="readwrite"/>
    <property name="ScreenReaderEnabled" type="b" access="readwrite"/>
  </interface>
</node>
"""

state = {"IsEnabled": sys.argv[2] == "true", "ScreenReaderEnabled": sys.argv[2] == "true"}
loop = GLib.MainLoop()
conn = None


def log(m):
    print(m, flush=True)


def on_call(_c, sender, _path, iface, method, params, inv):
    log(f"call {iface}.{method} from {sender}")
    if method == "GetAddress":
        inv.return_value(GLib.Variant("(s)", ("unix:path=/nonexistent/sbh-fake-a11y",)))
    else:
        inv.return_dbus_error("org.freedesktop.DBus.Error.UnknownMethod", method)


def on_get(_c, sender, _path, _iface, prop):
    log(f"get {prop} from {sender} = {state.get(prop)}")
    return GLib.Variant("b", state.get(prop, False))


def on_set(_c, sender, _path, _iface, prop, value):
    log(f"set {prop} from {sender} = {value.unpack()}")
    set_state(prop, value.unpack())
    return True


def set_state(prop, value):
    if state.get(prop) == value:
        return
    state[prop] = value
    conn.emit_signal(
        None, PATH, "org.freedesktop.DBus.Properties", "PropertiesChanged",
        GLib.Variant("(sa{sv}as)", (STATUS, {prop: GLib.Variant("b", value)}, [])),
    )
    log(f"PropertiesChanged {prop} = {value}")


def command(line):
    line = line.strip()
    if line in ("on", "off"):
        v = line == "on"
        set_state("ScreenReaderEnabled", v)
        set_state("IsEnabled", v)
    elif line == "quit":
        loop.quit()
    return False


def control():
    with open(sys.argv[1]) as f:
        for line in f:
            GLib.idle_add(command, line)
    GLib.idle_add(loop.quit)


def on_bus(c, _name):
    global conn
    conn = c
    node = Gio.DBusNodeInfo.new_for_xml(XML)
    for iface in node.interfaces:
        c.register_object(PATH, iface, on_call, on_get, on_set)


def on_name(_c, name):
    log(f"owned {name}")


def on_lost(_c, name):
    log(f"lost {name}")
    loop.quit()


Gio.bus_own_name(Gio.BusType.SESSION, "org.a11y.Bus", Gio.BusNameOwnerFlags.NONE, on_bus, on_name, on_lost)
threading.Thread(target=control, daemon=True).start()
loop.run()
