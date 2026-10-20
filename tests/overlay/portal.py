#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""A fake desktop portal for S-018 AC15, on the private session bus only.

Owns org.freedesktop.portal.Desktop and serves /org/freedesktop/portal/desktop
with org.freedesktop.portal.Settings (version 2):
  ReadAll(as) -> a{sa{sv}}
  Read(ss)    -> v   (the value wrapped in one more variant, as in version 1)
  ReadOne(ss) -> v
  SettingChanged(s namespace, s key, v value), emitted on a change.
The only setting is org.freedesktop.appearance color-scheme (u). Any other key
answers org.freedesktop.portal.Error.NotFound, as the real portal does.
Commands from the control file, one per line: "0", "1", "2" set the colour
scheme and emit SettingChanged, "quit" stops. Logs every call it gets.
Usage: portal.py CONTROL_FIFO INITIAL_VALUE"""
import sys
import threading

import gi

gi.require_version("Gio", "2.0")
from gi.repository import Gio, GLib  # noqa: E402

PATH = "/org/freedesktop/portal/desktop"
IFACE = "org.freedesktop.portal.Settings"
NS = "org.freedesktop.appearance"
KEY = "color-scheme"
XML = """
<node>
  <interface name="org.freedesktop.portal.Settings">
    <method name="ReadAll">
      <arg type="as" name="namespaces" direction="in"/>
      <arg type="a{sa{sv}}" name="value" direction="out"/>
    </method>
    <method name="Read">
      <arg type="s" name="namespace" direction="in"/>
      <arg type="s" name="key" direction="in"/>
      <arg type="v" name="value" direction="out"/>
    </method>
    <method name="ReadOne">
      <arg type="s" name="namespace" direction="in"/>
      <arg type="s" name="key" direction="in"/>
      <arg type="v" name="value" direction="out"/>
    </method>
    <signal name="SettingChanged">
      <arg type="s" name="namespace"/>
      <arg type="s" name="key"/>
      <arg type="v" name="value"/>
    </signal>
    <property name="version" type="u" access="read"/>
  </interface>
</node>
"""

state = {"scheme": int(sys.argv[2])}
loop = GLib.MainLoop()
conn = None


def log(m):
    print(m, flush=True)


def value():
    return GLib.Variant("u", state["scheme"])


def wanted(patterns):
    # An empty list or "" means every namespace; a trailing "*" is a prefix.
    if not patterns or "" in patterns:
        return True
    for p in patterns:
        if p == NS or (p.endswith("*") and NS.startswith(p[:-1])):
            return True
    return False


def on_call(_c, sender, _path, _iface, method, params, inv):
    args = params.unpack()
    log(f"call {method}{args} from {sender} (color-scheme {state['scheme']})")
    if method == "ReadAll":
        out = {NS: {KEY: value()}} if wanted(list(args[0])) else {}
        inv.return_value(GLib.Variant("(a{sa{sv}})", (out,)))
    elif method in ("Read", "ReadOne"):
        if tuple(args) != (NS, KEY):
            inv.return_dbus_error("org.freedesktop.portal.Error.NotFound", "Requested setting not found")
        elif method == "Read":
            inv.return_value(GLib.Variant("(v)", (GLib.Variant("v", value()),)))
        else:
            inv.return_value(GLib.Variant("(v)", (value(),)))
    else:
        inv.return_dbus_error("org.freedesktop.DBus.Error.UnknownMethod", method)


def on_get(_c, sender, _path, _iface, prop):
    log(f"get {prop} from {sender}")
    return GLib.Variant("u", 2) if prop == "version" else None


def set_scheme(v):
    if state["scheme"] == v:
        return
    state["scheme"] = v
    conn.emit_signal(None, PATH, IFACE, "SettingChanged", GLib.Variant("(ssv)", (NS, KEY, value())))
    log(f"SettingChanged {NS} {KEY} = {v}")


def command(line):
    line = line.strip()
    if line in ("0", "1", "2"):
        set_scheme(int(line))
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
    c.register_object(PATH, node.interfaces[0], on_call, on_get, None)


def on_name(_c, name):
    log(f"owned {name}")


def on_lost(_c, name):
    log(f"lost {name}")
    loop.quit()


Gio.bus_own_name(
    Gio.BusType.SESSION, "org.freedesktop.portal.Desktop", Gio.BusNameOwnerFlags.NONE, on_bus, on_name, on_lost
)
threading.Thread(target=control, daemon=True).start()
loop.run()
