#!/usr/bin/env python3
"""A maximised GTK 4 window under the hive: logs its clicks and focus, so the
run can tell whether a click outside the hive reached it."""
import sys
import gi
gi.require_version("Gtk", "4.0")
from gi.repository import Gtk

def log(m):
    print(m, flush=True)

app = Gtk.Application(application_id="org.sbarhive.OverlayOther")
def on_activate(a):
    w = Gtk.ApplicationWindow(application=a, title="other window")
    w.set_child(Gtk.Label(label="other window"))
    def pressed(*x):
        log("other: click")
        # A token as a file manager would hand it over (Nautilus integration path).
        ctx = w.get_display().get_app_launch_context()
        tok = ctx.get_startup_notify_id(None, [])
        if tok:
            open(sys.argv[1] if len(sys.argv) > 1 else "/dev/null", "w").write(tok)
            log("other: token written")
    c = Gtk.GestureClick(); c.connect("pressed", pressed)
    w.add_controller(c)
    w.connect("notify::is-active", lambda w, p: log(f"other: active {w.is_active()}"))
    w.maximize(); w.present()
app.connect("activate", on_activate)
app.run([])
