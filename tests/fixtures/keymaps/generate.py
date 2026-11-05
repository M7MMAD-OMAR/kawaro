#!/usr/bin/env python3
"""Generate real XKB fixtures with the local MIT libxkbcommon and xkeyboard-config.
Run from this directory. No production dependency or hand-authored key table.
"""
import ctypes as c
from pathlib import Path

lib = c.CDLL("libxkbcommon.so.0")
class Names(c.Structure):
    _fields_ = [(name, c.c_char_p) for name in ("rules", "model", "layout", "variant", "options")]
lib.xkb_context_new.argtypes = [c.c_int]
lib.xkb_context_new.restype = c.c_void_p
lib.xkb_keymap_new_from_names.argtypes = [c.c_void_p, c.POINTER(Names), c.c_int]
lib.xkb_keymap_new_from_names.restype = c.c_void_p
lib.xkb_keymap_get_as_string.argtypes = [c.c_void_p, c.c_int]
lib.xkb_keymap_get_as_string.restype = c.c_void_p
lib.xkb_keymap_unref.argtypes = [c.c_void_p]
lib.xkb_context_unref.argtypes = [c.c_void_p]
libc = c.CDLL(None)
libc.free.argtypes = [c.c_void_p]
ctx = lib.xkb_context_new(0)
assert ctx
for layout, output in [(b"us", "us.xkb"), (b"us,fr", "us-fr.xkb")]:
    names = Names(b"evdev", b"pc105", layout, b"", b"")
    keymap = lib.xkb_keymap_new_from_names(ctx, c.byref(names), 0)
    assert keymap
    text = lib.xkb_keymap_get_as_string(keymap, 1)
    assert text
    Path(__file__).with_name(output).write_bytes(c.string_at(text) + b"\0")
    libc.free(text)
    lib.xkb_keymap_unref(keymap)
lib.xkb_context_unref(ctx)
