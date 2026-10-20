#!/usr/bin/env python3
"""Screenshot of the private KWin's output through org.kde.KWin.ScreenShot2
(allowed with KWIN_WAYLAND_NO_PERMISSION_CHECKS=1, private compositor only).
Usage: shot.py OUT.png"""
import os, struct, sys, zlib, threading
import gi
gi.require_version("Gio", "2.0")
from gi.repository import Gio, GLib

out = sys.argv[1]
bus = Gio.bus_get_sync(Gio.BusType.SESSION)
r, w = os.pipe()
fds = Gio.UnixFDList.new()
idx = fds.append(w)
os.close(w)
data = bytearray()
def reader():
    while True:
        b = os.read(r, 1 << 20)
        if not b:
            break
        data.extend(b)
t = threading.Thread(target=reader); t.start()
res, _ = bus.call_with_unix_fd_list_sync(
    "org.kde.KWin", "/org/kde/KWin/ScreenShot2", "org.kde.KWin.ScreenShot2",
    "CaptureActiveScreen", GLib.Variant("(a{sv}h)", ({"native-resolution": GLib.Variant("b", True)}, idx)),
    GLib.VariantType("(a{sv})"), Gio.DBusCallFlags.NONE, 10000, fds, None)
fds.close(idx) if hasattr(fds, "close") else None
t.join(5)
meta = res.unpack()[0]
wd, ht, stride, fmt = meta["width"], meta["height"], meta["stride"], meta["format"]
# QImage ARGB32 / RGB32 / ARGB32_Premultiplied are B, G, R, A in memory.
rows = []
for y in range(ht):
    row = data[y * stride : y * stride + wd * 4]
    px = bytearray(len(row))
    px[0::4] = row[2::4]; px[1::4] = row[1::4]; px[2::4] = row[0::4]; px[3::4] = b"\xff" * wd
    rows.append(b"\x00" + bytes(px))
def chunk(t, d):
    return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)
png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", wd, ht, 8, 6, 0, 0, 0))
png += chunk(b"IDAT", zlib.compress(b"".join(rows), 6)) + chunk(b"IEND", b"")
open(out, "wb").write(png)
print(f"shot {out} {wd}x{ht} format {fmt}")
