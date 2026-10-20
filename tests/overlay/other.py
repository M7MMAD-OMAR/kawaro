#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""A full screen GTK 4 window under the hive (from spikes/overlay/other.py).
It paints one flat colour with no decorations and no text, so a screenshot
looks the same whether it is focused or not: what differs between a shot
before and after a show is the hive alone. Logs its clicks and focus, and on
a click writes an xdg-activation token to the file given as argument.
Usage: other.py [TOKEN_FILE]"""
import sys

import gi

gi.require_version("Gtk", "4.0")
from gi.repository import Gtk  # noqa: E402


def log(m):
    print(m, flush=True)


def on_activate(app):
    w = Gtk.ApplicationWindow(application=app, title="other window")
    w.set_decorated(False)
    area = Gtk.DrawingArea()

    def draw(_area, cr, _w, _h):
        cr.set_source_rgb(0.5, 0.5, 0.5)
        cr.paint()

    area.set_draw_func(draw)
    w.set_child(area)

    def pressed(*_):
        log("other: click")
        ctx = w.get_display().get_app_launch_context()
        tok = ctx.get_startup_notify_id(None, [])
        if tok:
            with open(sys.argv[1] if len(sys.argv) > 1 else "/dev/null", "w") as f:
                f.write(tok)
            log("other: token written")

    c = Gtk.GestureClick()
    c.connect("pressed", pressed)
    w.add_controller(c)
    w.connect("notify::is-active", lambda w, _p: log(f"other: active {w.is_active()}"))
    w.fullscreen()
    w.present()


app = Gtk.Application(application_id="org.sbarhive.OverlayOther")
app.connect("activate", on_activate)
app.run([])
