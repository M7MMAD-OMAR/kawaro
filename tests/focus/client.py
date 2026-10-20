# SPDX-License-Identifier: GPL-3.0-or-later
"""A focused GTK 4 window that stands in for the file manager.

It logs its own focus, and on a key press it launches `sbarhive open` the way
the Nautilus extension does:
  F5  with an xdg-activation token from Gdk.AppLaunchContext
  F6  without a token
  F7  with a made-up token (control: a policy that honours tokens rejects it)
Usage: client.py FILE   (SBARHIVE names the CLI binary, CLIENT_AUTO below)
"""

import os
import subprocess
import sys

import gi

gi.require_version("Gtk", "4.0")
gi.require_version("Gdk", "4.0")
from gi.repository import Gdk, Gio, GLib, Gtk  # noqa: E402

SBARHIVE = os.environ.get("SBARHIVE", "sbarhive")
FILE = sys.argv[1]


def log(*parts):
    print("client:", *parts, flush=True)


def launch(with_token, bogus=False):
    argv = [SBARHIVE, "open"]
    if bogus:
        argv += ["--token", "sbarhive-bogus-token"]
    elif with_token:
        ctx = Gdk.Display.get_default().get_app_launch_context()
        token = ctx.get_startup_notify_id(None, [])
        log("token", token or "none")
        if token:
            argv += ["--token", token]
    argv += ["--", FILE]
    env = dict(os.environ)
    env.pop("XDG_ACTIVATION_TOKEN", None)
    env.pop("DESKTOP_STARTUP_ID", None)
    log("launch", " ".join(argv))
    subprocess.Popen(argv, env=env, start_new_session=True)


def on_activate(app):
    win = Gtk.ApplicationWindow(application=app, title="focus-client")
    win.set_default_size(400, 200)
    win.set_child(Gtk.Entry())
    win.connect("notify::is-active", lambda w, _p: log("focus", w.is_active()))

    keys = Gtk.EventControllerKey()
    keys.set_propagation_phase(Gtk.PropagationPhase.CAPTURE)

    def on_key(_c, keyval, _code, _state):
        name = Gdk.keyval_name(keyval)
        log("key", name)
        if name == "F5":
            launch(True)
            return True
        if name == "F6":
            launch(False)
            return True
        if name == "F7":
            launch(False, bogus=True)
            return True
        return False

    keys.connect("key-pressed", on_key)
    win.add_controller(keys)
    win.present()

    # CLIENT_AUTO=token|notoken|bogus launches once, 3 s after start, for
    # compositors where this test cannot inject keys.
    auto = os.environ.get("CLIENT_AUTO")
    if auto in ("token", "notoken", "bogus"):
        GLib.timeout_add(3000, lambda: launch(auto == "token", auto == "bogus") and False)


app = Gtk.Application(
    application_id="org.sbarhive.FocusClient",
    flags=Gio.ApplicationFlags.NON_UNIQUE,
)
app.connect("activate", on_activate)
app.run([])
