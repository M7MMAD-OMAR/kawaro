# SPDX-License-Identifier: GPL-3.0-or-later
"""GTK 4 drag source standing in for a file manager: a large window whose
whole area drags one file (text/uri-list), offering copy and move like
Dolphin and Nautilus do. Prints drag-begin and drag-end.
Usage: source.py FILE"""
import sys
import gi
gi.require_version("Gtk", "4.0")
gi.require_version("Gdk", "4.0")
from gi.repository import Gdk, Gio, GLib, Gtk


def on_activate(app):
    win = Gtk.ApplicationWindow(application=app, title="drag-source")
    win.set_default_size(1200, 760)
    label = Gtk.Label(label="drag me")
    label.set_hexpand(True)
    label.set_vexpand(True)
    src = Gtk.DragSource(actions=Gdk.DragAction.COPY | Gdk.DragAction.MOVE)
    files = Gdk.FileList.new_from_list([Gio.File.new_for_path(sys.argv[1])])
    src.set_content(Gdk.ContentProvider.new_for_value(files))
    src.connect("drag-begin", lambda *_: print("source: drag-begin", flush=True))
    src.connect("drag-end", lambda s, d, deleted: print(f"source: drag-end deleted={deleted}", flush=True))
    src.connect("drag-cancel", lambda s, d, r: print(f"source: drag-cancel {r}", flush=True) or False)
    label.add_controller(src)
    win.set_child(label)
    win.maximize()
    win.present()
    print("source: shown", flush=True)


app = Gtk.Application(application_id="dev.sbarhive.DragSource", flags=Gio.ApplicationFlags.NON_UNIQUE)
app.connect("activate", on_activate)
app.run([])
