#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Another client for tests/hyprland/repro.sh: a full screen GTK 4 window
that logs every key press, click and focus change, so the test can tell
whether input reached it (before, during or after a hive show).
Usage: keys.py"""
import gi

gi.require_version("Gtk", "4.0")
gi.require_version("Gdk", "4.0")
from gi.repository import Gdk, Gtk  # noqa: E402


def log(m):
    print(m, flush=True)


def on_activate(app):
    w = Gtk.ApplicationWindow(application=app, title="keys window")
    w.set_decorated(False)
    area = Gtk.DrawingArea()

    def draw(_area, cr, _w, _h):
        cr.set_source_rgb(0.5, 0.5, 0.5)
        cr.paint()

    area.set_draw_func(draw)
    area.set_focusable(True)
    w.set_child(area)

    k = Gtk.EventControllerKey()
    k.connect("key-pressed", lambda _c, kv, code, _s: log(f"keys: key {Gdk.keyval_name(kv)} {code}") or False)
    w.add_controller(k)
    c = Gtk.GestureClick()
    c.connect("pressed", lambda *_: log("keys: click"))
    w.add_controller(c)
    w.connect("notify::is-active", lambda w, _p: log(f"keys: active {w.is_active()}"))
    w.fullscreen()
    w.present()


app = Gtk.Application(application_id="org.sbarhive.HyprKeys")
app.connect("activate", on_activate)
app.run([])
