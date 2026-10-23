#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""S-014f4 (backlog/S-014-drop-hive.md, AC16): changes the drop shortcut in
the desktop's own store, the way the desktop's settings do, on the private
session bus of tests/drop/kwin.sh or tests/drop/gnome.sh only (the caller's
DBUS_SESSION_BUS_ADDRESS and throwaway HOME). Every line it prints is one
JSON object. Needs python3-gobject.

  rebind_store.py kde XML [set]
      Introspects org.kde.kglobalaccel /kglobalaccel, writes the XML to XML,
      and takes from it the reader (shortcutKeys, (as) -> a(ai)) and the
      foreign setter (setForeignShortcutKeys, (as, a(ai)), the call the
      Shortcuts module of System Settings makes); neither is assumed: a
      missing one is reported as null. The action id of "drop" comes from
      allComponents and that component's allShortcutInfos (the component
      object is introspected too). The current keys are read and decoded
      (Qt: the key in the low 25 bits, Meta 0x10000000). With "set": the
      same structure with Qt Key_Y (0x59) in place of Key_Z (0x5a), written
      with the setter and read back.
  rebind_store.py gnome [set]
      Walks the private dconf with `dconf list` from "/" and finds every
      directory that has a "shortcuts" key whose value holds an entry
      "drop" (the a(sa{sv}) of the relocatable schema
      org.gnome.settings-daemon.global-shortcuts.application); the path is
      never written in here. With "set" (exactly one such path needed):
      every string of the drop entry that is an accelerator ending in z
      (for example "<Super>z") ends in y instead, the other entries are
      kept, the value is written with `dconf write`, and read back.
"""
import json
import re
import subprocess
import sys
import xml.etree.ElementTree as ET

import gi

gi.require_version("Gio", "2.0")
from gi.repository import Gio, GLib  # noqa: E402

KEY_Z, KEY_Y = 0x5A, 0x59
KEYMASK = 0x01FFFFFF
MODS = [(0x10000000, "Meta"), (0x04000000, "Ctrl"), (0x08000000, "Alt"), (0x02000000, "Shift")]


def out(**kw):
    print(json.dumps(kw, ensure_ascii=False))


def plain(seqs):
    """a(ai) as unpacked by GLib, [([int, ...],), ...], as [[int, ...], ...]."""
    return [list(s[0]) if isinstance(s, tuple) else list(s) for s in seqs]


def decode(seqs):
    """a(ai) as "Meta+Y, ..." for the INFO lines."""
    names = []
    for seq in plain(seqs):
        parts = []
        for k in seq:
            if not k:
                continue
            mods = [n for bit, n in MODS if k & bit]
            key = k & KEYMASK
            parts.append("+".join(mods + [chr(key) if 0x20 < key < 0x7F else hex(key)]))
        if parts:
            names.append(", ".join(parts))
    return "; ".join(names)


def bus():
    return Gio.bus_get_sync(Gio.BusType.SESSION, None)


def call(conn, path, iface, method, sig, args, reply=None):
    v = GLib.Variant(sig, args) if sig else None
    r = conn.call_sync("org.kde.kglobalaccel", path, iface, method, v,
                       GLib.VariantType.new(reply) if reply else None,
                       Gio.DBusCallFlags.NONE, 5000, None)
    return r.unpack() if r is not None else None


def introspect(conn, path):
    (xml,) = call(conn, path, "org.freedesktop.DBus.Introspectable", "Introspect", None, None, "(s)")
    return xml


def methods(xml, iface):
    """{name: (in signature list, out signature list)} of IFACE in XML."""
    res = {}
    for i in ET.fromstring(xml).iter("interface"):
        if i.get("name") != iface:
            continue
        for m in i.iter("method"):
            ins = [a.get("type") for a in m.iter("arg") if a.get("direction", "in") == "in"]
            outs = [a.get("type") for a in m.iter("arg") if a.get("direction") == "out"]
            res[m.get("name")] = (ins, outs)
    return res


def kde(xml_out, do_set):
    conn = bus()
    xml = introspect(conn, "/kglobalaccel")
    with open(xml_out, "w", encoding="utf-8") as f:
        f.write(xml)
    ms = methods(xml, "org.kde.KGlobalAccel")
    listed = sorted("%s(%s)->(%s)" % (n, ",".join(i), ",".join(o)) for n, (i, o) in ms.items())
    reader = "shortcutKeys" if ms.get("shortcutKeys", ([], []))[0] == ["as"] else None
    setter = "setForeignShortcutKeys" if ms.get("setForeignShortcutKeys", ([], []))[0] == ["as", "a(ai)"] else None
    comps = call(conn, "/kglobalaccel", "org.kde.KGlobalAccel", "allComponents", None, None)[0] \
        if "allComponents" in ms else []
    action_id, info_raw, comp_path = None, None, None
    for p in comps:
        cxml = introspect(conn, p)
        cms = methods(cxml, "org.kde.kglobalaccel.Component")
        if "allShortcutInfos" not in cms:
            continue
        ins = cms["allShortcutInfos"][0]
        infos = call(conn, p, "org.kde.kglobalaccel.Component", "allShortcutInfos",
                     "(" + "".join(ins) + ")" if ins else None,
                     tuple("default" if t == "s" else None for t in ins) if ins else None)[0]
        for inf in infos:
            # KGlobalShortcutInfo on the bus: action unique, action friendly,
            # component unique, component friendly, context unique, context
            # friendly, keys, default keys.
            if inf[0] == "drop":
                action_id = [inf[2], inf[0], inf[3], inf[1]]
                info_raw = [str(x) for x in inf]
                comp_path = p
    before = None
    if reader and action_id:
        before = plain(call(conn, "/kglobalaccel", "org.kde.KGlobalAccel", reader, "(as)", (action_id,))[0])
    res = dict(methods=listed, reader=reader, setter=setter, components=list(comps),
               component=comp_path, info=info_raw, action_id=action_id,
               before=before, before_text=decode(before) if before is not None else None)
    if do_set:
        new = None
        if before:
            new = [[(k & ~KEYMASK) | KEY_Y if (k & KEYMASK) == KEY_Z else k for k in seq] for seq in before]
        res["wrote"] = new
        res["wrote_text"] = decode(new) if new else None
        res["readback"] = None
        if setter and new and new != before:
            call(conn, "/kglobalaccel", "org.kde.KGlobalAccel", setter, "(asa(ai))",
                 (action_id, [(seq,) for seq in new]))
            rb = plain(call(conn, "/kglobalaccel", "org.kde.KGlobalAccel", reader, "(as)", (action_id,))[0])
            res["readback"] = rb
            res["readback_text"] = decode(rb)
            res["ok"] = rb == new
        else:
            res["ok"] = False
    out(**res)


def dconf(*args):
    return subprocess.run(["dconf", *args], capture_output=True, text=True, check=False).stdout


def walk(d="/"):
    for e in dconf("list", d).split():
        if e.endswith("/"):
            yield from walk(d + e)
        else:
            yield d, e


ACCEL_Z = re.compile(r"^((?:<[A-Za-z0-9_]+>)+)z$")


def swap(v):
    """A copy of the unpacked a{sv} value of the drop entry with every
    accelerator string ending in z ending in y; returns (value, changed)."""
    if isinstance(v, str):
        m = ACCEL_Z.match(v)
        return (m.group(1) + "y", True) if m else (v, False)
    if isinstance(v, list):
        items = [swap(x) for x in v]
        return [x for x, _ in items], any(c for _, c in items)
    return v, False


def gnome(do_set):
    found = []
    for d, k in walk():
        if k != "shortcuts":
            continue
        text = dconf("read", d + k).strip()
        try:
            val = GLib.Variant.parse(None, text, None, None)
        except GLib.Error:
            continue
        if val.get_type_string() != "a(sa{sv})":
            continue
        if any(e[0] == "drop" for e in val.unpack()):
            found.append((d, text, val))
    res = dict(paths=[d for d, _, _ in found], before=found[0][1] if found else None)
    if do_set:
        res["ok"] = False
        if len(found) == 1:
            d, text, val = found[0]
            entries, changed = [], False
            for i in range(val.n_children()):
                e = val.get_child_value(i)
                sid = e.get_child_value(0).get_string()
                if sid != "drop":
                    entries.append(e)
                    continue
                opts = e.get_child_value(1)
                new = {}
                for j in range(opts.n_children()):
                    kv = opts.get_child_value(j)
                    key = kv.get_child_value(0).get_string()
                    inner = kv.get_child_value(1).get_variant()
                    py, c = swap(inner.unpack())
                    changed = changed or c
                    new[key] = GLib.Variant(inner.get_type_string(), py) if c else inner
                entries.append(GLib.Variant("(sa{sv})", (sid, new)))
            newval = GLib.Variant.new_array(GLib.VariantType.new("(sa{sv})"), entries)
            wrote = newval.print_(False)
            res["wrote"] = wrote
            res["changed"] = changed
            if changed:
                subprocess.run(["dconf", "write", d + "shortcuts", wrote], check=False)
                rb = dconf("read", d + "shortcuts").strip()
                res["readback"] = rb
                rbv = GLib.Variant.parse(None, rb, None, None)
                others_b = [e for e in val.unpack() if e[0] != "drop"]
                others_a = [e for e in rbv.unpack() if e[0] != "drop"]
                drop_a = [e for e in rbv.unpack() if e[0] == "drop"]
                res["others_kept"] = others_a == others_b
                res["ok"] = rbv.equal(newval) and others_a == others_b and len(drop_a) == 1
    out(**res)


def main():
    a = sys.argv[1:]
    if a and a[0] == "kde" and len(a) >= 2:
        kde(a[1], a[2:3] == ["set"])
    elif a and a[0] == "gnome":
        gnome(a[1:2] == ["set"])
    else:
        raise SystemExit(__doc__)


main()
