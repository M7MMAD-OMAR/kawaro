#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""A fake GlobalShortcuts desktop portal for S-014e1 (AC13, AC17 to AC19),
on the private session bus of tests/drop/portal.sh only, in the pattern of
tests/overlay/portal.py.

Owns org.freedesktop.portal.Desktop and serves /org/freedesktop/portal/desktop
with
  org.freedesktop.host.portal.Registry (version 1):
    Register(s app_id, a{sv} options)
  org.freedesktop.portal.GlobalShortcuts (version 2):
    CreateSession(a{sv} options) -> o request
    BindShortcuts(o session, a(sa{sv}) shortcuts, s parent_window, a{sv}) -> o
    ListShortcuts(o session, a{sv} options) -> o
    ConfigureShortcuts(o session, s parent_window, a{sv} options)
    signals Activated, Deactivated (o session, s id, t timestamp, a{sv})
and an org.freedesktop.portal.Session object (Close, Closed) per session.
As xdg-desktop-portal does: a request's path is
/org/freedesktop/portal/desktop/request/SENDER/handle_token and a session's
/org/freedesktop/portal/desktop/session/SENDER/session_handle_token (SENDER
the caller's unique name without ":" and with "." as "_"); the
org.freedesktop.portal.Request.Response(u, a{sv}) signal goes to the caller
only, after the method's reply; CreateSession's result carries
session_handle as a string ("s"); Activated and Deactivated go to the
session's owner only.

Usage: shortcuts_portal.py CONTROL_FIFO MODE
MODE:
  ok           every method works; BindShortcuts answers 0 with every
               shortcut asked for, each with its description and the
               trigger_description TRIGGER below
  no-registry  as ok, but Register answers
               org.freedesktop.DBus.Error.UnknownMethod (a portal older
               than the Registry interface)
  no-gs        no GlobalShortcuts interface on the object (Registry only)
  version0     GlobalShortcuts whose version property is 0
  deny         BindShortcuts answers 1 (the user cancelled the dialog)
  error        BindShortcuts answers 2
  empty        BindShortcuts answers 0 with an empty shortcuts list
  hold         as ok, but BindShortcuts' Response is held until the
               control line "respond"

Control lines, one per line, from CONTROL_FIFO:
  signal NAME ID [own|other]   emit Activated or Deactivated for shortcut ID
                               on the last session made ("own", the
                               default) or on another session path of the
                               same owner ("other")
  pulse ID SESSION MS [AT...]  Activated now and again at each AT ms (key
                               repeat), Deactivated at MS ms, all timed
                               here, not by the caller's sleeps
  respond                      send the held BindShortcuts Response
  quit                         stop

Log on stdout, one line per event, each starting with the UTC time of day
"[HH:MM:SS.ffffff]" (the clock of the daemon's WAYLAND_DEBUG trace):
  owned NAME | lost NAME
  call N IFACE.METHOD SENDER ARGS    a method call; ARGS is a JSON list of
                                     its arguments, variants unpacked
  get N IFACE.PROPERTY SENDER        a property read (Get or GetAll)
  error N NAME                       the reply of call N was that error
  response N CODE PATH RESULTS       the Response of call N (RESULTS JSON)
  held N PATH                        call N's Response is held (hold mode)
  emit SIGNAL ID SESSION DEST        a signal sent to DEST
  skip LINE (REASON)                 a control line that did nothing
N counts calls and reads together, from 1."""
import datetime
import json
import sys
import threading
import warnings

import gi

gi.require_version("Gio", "2.0")
from gi.repository import Gio, GLib  # noqa: E402

# Gio.DBusConnection.register_object, as tests/overlay/portal.py uses it.
warnings.filterwarnings("ignore", category=DeprecationWarning)

PATH = "/org/freedesktop/portal/desktop"
REGISTRY = "org.freedesktop.host.portal.Registry"
GS = "org.freedesktop.portal.GlobalShortcuts"
SESSION = "org.freedesktop.portal.Session"
REQUEST = "org.freedesktop.portal.Request"
TRIGGER = "Fake Super+Z"
XML = """
<node>
  <interface name="org.freedesktop.host.portal.Registry">
    <method name="Register">
      <arg type="s" name="app_id" direction="in"/>
      <arg type="a{sv}" name="options" direction="in"/>
    </method>
    <property name="version" type="u" access="read"/>
  </interface>
  <interface name="org.freedesktop.portal.GlobalShortcuts">
    <method name="CreateSession">
      <arg type="a{sv}" name="options" direction="in"/>
      <arg type="o" name="handle" direction="out"/>
    </method>
    <method name="BindShortcuts">
      <arg type="o" name="session_handle" direction="in"/>
      <arg type="a(sa{sv})" name="shortcuts" direction="in"/>
      <arg type="s" name="parent_window" direction="in"/>
      <arg type="a{sv}" name="options" direction="in"/>
      <arg type="o" name="request_handle" direction="out"/>
    </method>
    <method name="ListShortcuts">
      <arg type="o" name="session_handle" direction="in"/>
      <arg type="a{sv}" name="options" direction="in"/>
      <arg type="o" name="request_handle" direction="out"/>
    </method>
    <method name="ConfigureShortcuts">
      <arg type="o" name="session_handle" direction="in"/>
      <arg type="s" name="parent_window" direction="in"/>
      <arg type="a{sv}" name="options" direction="in"/>
    </method>
    <signal name="Activated">
      <arg type="o" name="session_handle"/>
      <arg type="s" name="shortcut_id"/>
      <arg type="t" name="timestamp"/>
      <arg type="a{sv}" name="options"/>
    </signal>
    <signal name="Deactivated">
      <arg type="o" name="session_handle"/>
      <arg type="s" name="shortcut_id"/>
      <arg type="t" name="timestamp"/>
      <arg type="a{sv}" name="options"/>
    </signal>
    <signal name="ShortcutsChanged">
      <arg type="o" name="session_handle"/>
      <arg type="a(sa{sv})" name="shortcuts"/>
    </signal>
    <property name="version" type="u" access="read"/>
  </interface>
  <interface name="org.freedesktop.portal.Session">
    <method name="Close"/>
    <signal name="Closed">
      <arg type="a{sv}" name="details"/>
    </signal>
    <property name="version" type="u" access="read"/>
  </interface>
</node>
"""

MODE = sys.argv[2]
MODES = ("ok", "no-registry", "no-gs", "version0", "deny", "error", "empty", "hold")
if MODE not in MODES:
    sys.exit(f"mode {MODE!r}: expected one of {', '.join(MODES)}")

loop = GLib.MainLoop()
conn = None
state = {"n": 0, "session": None, "owner": None, "bound": {}, "held": None}
NODE = Gio.DBusNodeInfo.new_for_xml(XML)


def iface(name):
    return next(i for i in NODE.interfaces if i.name == name)


def stamp():
    return datetime.datetime.now(datetime.timezone.utc).strftime("%H:%M:%S.%f")


def log(m):
    print(f"[{stamp()}] {m}", flush=True)


def plain(v):
    if isinstance(v, GLib.Variant):
        return plain(v.unpack())
    if isinstance(v, dict):
        return {str(k): plain(x) for k, x in v.items()}
    if isinstance(v, (list, tuple)):
        return [plain(x) for x in v]
    if isinstance(v, bytes):
        return v.decode("utf-8", "replace")
    return v


def jdump(v):
    return json.dumps(plain(v), ensure_ascii=False)


def ident(sender):
    return sender.lstrip(":").replace(".", "_")


def request_path(sender, options):
    token = options.get("handle_token") or f"t{state['n']}"
    return f"{PATH}/request/{ident(sender)}/{token}"


def respond(n, sender, path, code, results):
    """Response(u, a{sv}) on the request path, to the caller only."""
    conn.emit_signal(sender, path, REQUEST, "Response", GLib.Variant("(ua{sv})", (code, results)))
    log(f"response {n} {code} {path} {jdump(results)}")
    return False


def later(n, sender, path, code, results):
    # After the method's reply is on the wire, as the real portal sends it.
    GLib.timeout_add(30, respond, n, sender, path, code, results)


def on_session_call(_c, sender, path, interface, method, params, inv):
    state["n"] += 1
    log(f"call {state['n']} {interface}.{method} {sender} {jdump(params.unpack())} on {path}")
    inv.return_value(None)
    if method == "Close":
        conn.emit_signal(sender, path, SESSION, "Closed", GLib.Variant("(a{sv})", ({},)))


def on_call(_c, sender, _path, interface, method, params, inv):
    state["n"] += 1
    n = state["n"]
    args = params.unpack()
    log(f"call {n} {interface}.{method} {sender} {jdump(args)}")
    if interface == REGISTRY:
        if MODE == "no-registry":
            name = "org.freedesktop.DBus.Error.UnknownMethod"
            inv.return_dbus_error(name, f"No such interface '{REGISTRY}' on object at path {PATH}")
            log(f"error {n} {name}")
        else:
            inv.return_value(None)
        return
    if method == "CreateSession":
        opts = args[0]
        req = request_path(sender, opts)
        token = opts.get("session_handle_token") or f"s{n}"
        spath = f"{PATH}/session/{ident(sender)}/{token}"
        try:
            conn.register_object(spath, iface(SESSION), on_session_call, on_get, None)
        except GLib.Error as e:
            log(f"skip session {spath} ({e.message})")
        state["session"], state["owner"] = spath, sender
        inv.return_value(GLib.Variant("(o)", (req,)))
        later(n, sender, req, 0, {"session_handle": GLib.Variant("s", spath)})
    elif method == "BindShortcuts":
        spath, shortcuts, _parent, opts = args
        req = request_path(sender, opts)
        inv.return_value(GLib.Variant("(o)", (req,)))
        if MODE == "deny":
            later(n, sender, req, 1, {})
        elif MODE == "error":
            later(n, sender, req, 2, {})
        else:
            out = []
            if MODE != "empty":
                for sid, props in shortcuts:
                    desc = props.get("description", "")
                    out.append((sid, {"description": GLib.Variant("s", desc), "trigger_description": GLib.Variant("s", TRIGGER)}))
            state["bound"][spath] = out
            results = {"shortcuts": GLib.Variant("a(sa{sv})", out)}
            if MODE == "hold":
                state["held"] = (n, sender, req, results)
                log(f"held {n} {req}")
            else:
                later(n, sender, req, 0, results)
    elif method == "ListShortcuts":
        spath, opts = args
        req = request_path(sender, opts)
        inv.return_value(GLib.Variant("(o)", (req,)))
        later(n, sender, req, 0, {"shortcuts": GLib.Variant("a(sa{sv})", state["bound"].get(spath, []))})
    elif method == "ConfigureShortcuts":
        inv.return_value(None)
    else:
        inv.return_dbus_error("org.freedesktop.DBus.Error.UnknownMethod", method)


def on_get(_c, sender, _path, interface, prop):
    state["n"] += 1
    log(f"get {state['n']} {interface}.{prop} {sender}")
    if prop != "version":
        return None
    if interface == REGISTRY:
        return GLib.Variant("u", 1)
    if interface == GS:
        return GLib.Variant("u", 0 if MODE == "version0" else 2)
    return GLib.Variant("u", 1)


def emit(name, sid, which, line):
    if state["session"] is None:
        log(f"skip {line} (no session yet)")
        return False
    spath = state["session"] if which == "own" else state["session"].rsplit("/", 1)[0] + "/sbh_other"
    ts = GLib.get_monotonic_time() // 1000
    conn.emit_signal(state["owner"], PATH, GS, name, GLib.Variant("(osta{sv})", (spath, sid, ts, {})))
    log(f"emit {name} {sid} {spath} {state['owner']}")
    return False


def command(line):
    line = line.strip()
    parts = line.split()
    if not parts:
        return False
    if parts[0] == "quit":
        loop.quit()
    elif parts[0] == "respond":
        if state["held"] is None:
            log(f"skip {line} (nothing held)")
        else:
            n, sender, req, results = state["held"]
            respond(n, sender, req, 0, results)
            state["held"] = None
    elif parts[0] == "signal" and len(parts) in (3, 4) and parts[1] in ("Activated", "Deactivated"):
        emit(parts[1], parts[2], parts[3] if len(parts) == 4 else "own", line)
    elif parts[0] == "pulse" and len(parts) >= 4:
        sid, which, ms = parts[1], parts[2], int(parts[3])
        emit("Activated", sid, which, line)
        for at in parts[4:]:
            GLib.timeout_add(int(at), emit, "Activated", sid, which, line)
        GLib.timeout_add(ms, emit, "Deactivated", sid, which, line)
    else:
        log(f"skip {line} (unknown)")
    return False


def control():
    with open(sys.argv[1]) as f:
        for line in f:
            GLib.idle_add(command, line)
    GLib.idle_add(loop.quit)


def on_bus(c, _name):
    global conn
    conn = c
    c.register_object(PATH, iface(REGISTRY), on_call, on_get, None)
    if MODE != "no-gs":
        c.register_object(PATH, iface(GS), on_call, on_get, None)


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
