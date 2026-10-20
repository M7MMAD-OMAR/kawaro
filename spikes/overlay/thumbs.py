#!/usr/bin/env python3
"""Writes thumbnails the way Nautilus does (gnome-desktop's thumbnail factory,
large size) for the given files, in the sandbox's XDG_CACHE_HOME."""
import sys, os
import gi
gi.require_version("GnomeDesktop", "4.0")
from gi.repository import GnomeDesktop, Gio
f = GnomeDesktop.DesktopThumbnailFactory.new(GnomeDesktop.DesktopThumbnailSize.LARGE)
for p in sys.argv[1:]:
    g = Gio.File.new_for_path(p)
    info = g.query_info("standard::content-type,time::modified", 0, None)
    uri, mime, mtime = g.get_uri(), info.get_content_type(), info.get_attribute_uint64("time::modified")
    if not f.can_thumbnail(uri, mime, mtime):
        print(f"cannot thumbnail {p} ({mime})"); continue
    pix = f.generate_thumbnail(uri, mime, None)
    if pix is None:
        f.create_failed_thumbnail(uri, mtime, None); print(f"failed {p}"); continue
    f.save_thumbnail(pix, uri, mtime, None)
    print(f"thumbnail {p} -> {GnomeDesktop.desktop_thumbnail_path_for_uri(uri, GnomeDesktop.DesktopThumbnailSize.LARGE)}")
    print(f"gio uri {uri}")
