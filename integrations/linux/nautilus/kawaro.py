# SPDX-License-Identifier: GPL-3.0-or-later
"""Kawaro entry in the Nautilus context menu (nautilus-python, Nautilus 4.x).

Install to ~/.local/share/nautilus-python/extensions/ and restart Nautilus
(`nautilus -q`). The extension only collects the selected local paths and
an xdg-activation token, then runs `kawaro open` detached. No logic lives
here, and nothing may block Nautilus.
"""

import os
import shutil
import subprocess
import threading

import gi

try:
    gi.require_version("Nautilus", "4.1")
except ValueError:
    gi.require_version("Nautilus", "4.0")

from gi.repository import GObject, Nautilus  # noqa: E402

try:
    gi.require_version("Gdk", "4.0")
    from gi.repository import Gdk  # noqa: E402
except (ImportError, ValueError):
    Gdk = None

# scripts/install.sh rewrites this line with the absolute installed path.
KAWARO_BIN = "kawaro"

LABEL_EN = "Kawaro"
LABEL_AR = "كوارو"
TIP_EN = "Open the selected files in Kawaro"
TIP_AR = "افتح الملفات المحددة في كوارو"


def is_arabic_locale(env=None):
    """True when the first effective UI language is Arabic.

    Same precedence as gettext: LANGUAGE, then LC_ALL, LC_MESSAGES, LANG.
    """
    env = os.environ if env is None else env
    for var in ("LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"):
        value = env.get(var, "")
        if value:
            first = value.split(":")[0]
            return first == "ar" or first.startswith(("ar_", "ar.", "ar@"))
    return False


def resolve_binary():
    if os.path.isabs(KAWARO_BIN) and os.access(KAWARO_BIN, os.X_OK):
        return KAWARO_BIN
    found = shutil.which(os.path.basename(KAWARO_BIN))
    return found or KAWARO_BIN


def local_paths(files):
    paths = []
    for info in files:
        try:
            if info.get_uri_scheme() != "file":
                continue
            location = info.get_location()
            path = location.get_path() if location is not None else None
        except Exception:  # a vanished file must not break the menu
            continue
        if path:
            paths.append(path)
    return paths


def activation_token():
    """An xdg-activation token from the GTK 4 display, or None.

    The token lets the daemon focus the hive window on GNOME and KDE.
    """
    if Gdk is None:
        return None
    try:
        display = Gdk.Display.get_default()
        if display is None:
            return None
        context = display.get_app_launch_context()
        token = context.get_startup_notify_id(None, [])
    except Exception:
        return None
    return token or None


def build_command(paths, token):
    argv = [resolve_binary(), "open"]
    if token:
        argv += ["--token", token]
    return argv + ["--"] + list(paths)


def _reap(proc):
    try:
        proc.wait()
    except Exception:
        pass


def launch(paths):
    """Start `kawaro open` detached and return at once."""
    try:
        proc = subprocess.Popen(
            build_command(paths, activation_token()),
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
            close_fds=True,
        )
    except OSError:
        return None
    # Reap in the background so no zombie stays in the Nautilus process.
    threading.Thread(target=_reap, args=(proc,), daemon=True).start()
    return proc


class KawaroMenuProvider(GObject.GObject, Nautilus.MenuProvider):
    def _on_activate(self, _item, paths):
        launch(paths)

    def get_file_items(self, *args):
        # Nautilus 4: (files). Nautilus 3: (window, files).
        files = args[-1] if args else []
        paths = local_paths(files or [])
        if not paths:
            return []
        arabic = is_arabic_locale()
        item = Nautilus.MenuItem(
            name="Kawaro::open",
            label=LABEL_AR if arabic else LABEL_EN,
            tip=TIP_AR if arabic else TIP_EN,
            icon="kawaro",
        )
        item.connect("activate", self._on_activate, paths)
        return [item]

    def get_background_items(self, *args):
        return []
