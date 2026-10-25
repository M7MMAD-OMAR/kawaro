# SPDX-License-Identifier: GPL-3.0-or-later
"""GTK 4 drag source standing in for a file manager (from the drop spike):
a maximized window whose whole area drags one file as text/uri-list,
offering copy and move like Dolphin and Nautilus do.

The file to drag is read from NEXT (a text file holding one path) at the
start of every drag, so one window serves several drags. When a drag ends
with deleted=True (the target took a move), the file is deleted, as a file
manager would, so a move shows up in the sha256 check.

The drag icon is one transparent pixel (S-014c), so the screenshots of the
hive under the drag (tests/drop/kwin.sh, AC9) show the hive only.

S-021 AC6: a NEXT line "text:WORDS" drags the plain string WORDS instead of a
file, so the offer has text/plain types and no text/uri-list.

Prints: shown, drag-begin PATH (or text:WORDS), drag-end deleted=BOOL,
drag-cancel REASON.
Usage: source.py NEXT"""
import os
import sys

import gi

gi.require_version("Gtk", "4.0")
gi.require_version("Gdk", "4.0")
from gi.repository import Gdk, Gio, GLib, Gtk  # noqa: E402

NEXT = sys.argv[1]
current = {"path": None}


def prepare(src, _x, _y):
    blank = Gdk.MemoryTexture.new(1, 1, Gdk.MemoryFormat.R8G8B8A8, GLib.Bytes.new(b"\0\0\0\0"), 4)
    src.set_icon(blank, 0, 0)
    with open(NEXT) as f:
        path = f.read().strip()
    current["path"] = path
    if path.startswith("text:"):
        return Gdk.ContentProvider.new_for_value(path[len("text:"):])
    files = Gdk.FileList.new_from_list([Gio.File.new_for_path(path)])
    return Gdk.ContentProvider.new_for_value(files)


def drag_end(_src, _drag, deleted):
    print(f"source: drag-end deleted={deleted}", flush=True)
    if deleted and current["path"] and not current["path"].startswith("text:") and os.path.exists(current["path"]):
        os.unlink(current["path"])
        print(f"source: deleted {current['path']}", flush=True)


def on_activate(app):
    win = Gtk.ApplicationWindow(application=app, title="drag-source")
    win.set_default_size(1200, 760)
    label = Gtk.Label(label="drag me")
    label.set_hexpand(True)
    label.set_vexpand(True)
    src = Gtk.DragSource(actions=Gdk.DragAction.COPY | Gdk.DragAction.MOVE)
    src.connect("prepare", prepare)
    src.connect("drag-begin", lambda *_: print(f"source: drag-begin {current['path']}", flush=True))
    src.connect("drag-end", drag_end)
    src.connect("drag-cancel", lambda s, d, r: print(f"source: drag-cancel {r}", flush=True) or False)
    label.add_controller(src)
    win.set_child(label)
    win.maximize()
    win.present()
    print("source: shown", flush=True)


app = Gtk.Application(application_id="dev.kawaro.DropTestSource", flags=Gio.ApplicationFlags.NON_UNIQUE)
app.connect("activate", on_activate)
app.run([])
