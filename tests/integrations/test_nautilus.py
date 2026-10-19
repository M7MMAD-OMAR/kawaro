#!/usr/bin/env python3
"""S-006 AC2, AC3, AC4: the Nautilus extension.

Usage: test_nautilus.py real|stub

real: imports the module with the system gi (skips when Nautilus 4.x
      typelibs are missing). stub: puts a fake gi package first on sys.path
      that refuses Nautilus 4.1 and accepts 4.0, to prove the fallback.
Display variables are removed so Gdk cannot reach the running compositor.
"""

import atexit
import importlib.util
import os
import shutil
import subprocess
import sys
import tempfile
import textwrap
import time

HERE = os.path.dirname(os.path.abspath(__file__))
MODULE = os.path.join(HERE, "..", "..", "integrations", "linux", "nautilus", "sbarhive.py")

STUB_GI = {
    "gi/__init__.py": """
        REQUIRED = []
        def require_version(name, version):
            if (name, version) == ("Nautilus", "4.1"):
                raise ValueError("Namespace Nautilus not available for version 4.1")
            REQUIRED.append((name, version))
    """,
    "gi/repository/__init__.py": """
        class _GObjectMeta(type):
            pass

        class _GObject:
            def __init__(self, **kwargs):
                pass

        class GObject:
            GObject = _GObject

        class _MenuItem:
            def __init__(self, name=None, label=None, tip=None, icon=None):
                self.props = type("P", (), {})()
                self.props.name, self.props.label = name, label
                self.props.tip, self.props.icon = tip, icon
                self._handlers = []

            def connect(self, signal, handler, *data):
                assert signal == "activate"
                self._handlers.append((handler, data))

            def emit(self, signal):
                for handler, data in self._handlers:
                    handler(self, *data)

        class _MenuProvider:
            pass

        class Nautilus:
            MenuItem = _MenuItem
            MenuProvider = _MenuProvider

        class _Ctx:
            def get_startup_notify_id(self, info, files):
                return "STUBTOKEN"

        class _Display:
            def get_app_launch_context(self):
                return _Ctx()

        class Gdk:
            class Display:
                @staticmethod
                def get_default():
                    return _Display()
    """,
}


class FakeLocation:
    def __init__(self, path):
        self._path = path

    def get_path(self):
        return self._path


class FakeFile:
    def __init__(self, scheme, path):
        self._scheme, self._path = scheme, path

    def get_uri_scheme(self):
        return self._scheme

    def get_location(self):
        return FakeLocation(self._path)


FAILS = []


def check(cond, msg):
    if not cond:
        FAILS.append(msg)
        print("  FAIL:", msg, file=sys.stderr)


def load_module():
    spec = importlib.util.spec_from_file_location("sbarhive_nautilus", MODULE)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def label_of(item):
    return item.props.label


def run(mode):
    for var in ("WAYLAND_DISPLAY", "DISPLAY", "XDG_ACTIVATION_TOKEN", "DESKTOP_STARTUP_ID"):
        os.environ.pop(var, None)

    if mode == "stub":
        stub = tempfile.mkdtemp(prefix="sbarhive-gi-stub.")
        atexit.register(shutil.rmtree, stub, True)
        for rel, body in STUB_GI.items():
            path = os.path.join(stub, rel)
            os.makedirs(os.path.dirname(path), exist_ok=True)
            with open(path, "w", encoding="utf-8") as fh:
                fh.write(textwrap.dedent(body))
        sys.path.insert(0, stub)
    else:
        try:
            import gi

            try:
                gi.require_version("Nautilus", "4.1")
            except ValueError:
                gi.require_version("Nautilus", "4.0")
            from gi.repository import Nautilus  # noqa: F401
        except (ImportError, ValueError) as exc:
            print(f"SKIP test_nautilus real ({exc})")
            return 0

    mod = load_module()

    if mode == "stub":
        import gi

        check(("Nautilus", "4.0") in gi.REQUIRED, "fallback to Nautilus 4.0 not used")

    provider = mod.SbarHiveMenuProvider()

    # AC2: local files give one item; nothing local gives no item.
    local = [FakeFile("file", "/tmp/a b.png"), FakeFile("sftp", None), FakeFile("file", "/tmp/c.pdf")]
    os.environ["LANG"] = "en_US.UTF-8"
    for var in ("LANGUAGE", "LC_ALL", "LC_MESSAGES"):
        os.environ.pop(var, None)
    items = provider.get_file_items(local)
    check(len(items) == 1, f"expected one item, got {len(items)}")
    check(provider.get_file_items([FakeFile("sftp", None)]) == [], "remote only should give no item")
    check(provider.get_file_items([]) == [], "empty selection should give no item")
    # Nautilus 3.x style call (window, files) still works.
    check(len(provider.get_file_items(None, local)) == 1, "legacy (window, files) signature")

    # AC3: label follows the locale.
    check(label_of(items[0]) == "SbarHive", f"English label: {label_of(items[0])!r}")
    os.environ["LANG"] = "ar_SY.UTF-8"
    check(label_of(provider.get_file_items(local)[0]) == "صبار هايف", "Arabic label from LANG")
    os.environ["LANG"] = "en_US.UTF-8"
    os.environ["LANGUAGE"] = "ar:en"
    check(label_of(provider.get_file_items(local)[0]) == "صبار هايف", "Arabic label from LANGUAGE")
    os.environ.pop("LANGUAGE")

    # AC4: argv and Popen options, with and without a token.
    calls = []

    class RecordingPopen:
        def __init__(self, argv, **kwargs):
            calls.append((argv, kwargs))

        def wait(self):
            return 0

    # Token helper never raises and gives a string or None. With the stub gi
    # it returns the stub token; with real gi and no display it gives None.
    tok = mod.activation_token()
    check(tok is None or isinstance(tok, str), f"activation token type: {tok!r}")
    if mode == "stub":
        check(tok == "STUBTOKEN", f"stub token not read: {tok!r}")

    real_popen = mod.subprocess.Popen
    mod.subprocess.Popen = RecordingPopen
    mod.SBARHIVE_BIN = "sbarhive"
    mod.activation_token = lambda: "TOK"
    item = provider.get_file_items([FakeFile("file", "a"), FakeFile("file", "b")])[0]
    item.emit("activate")
    check(len(calls) == 1, f"one launch expected, got {len(calls)}")
    if calls:
        argv, kw = calls[-1]
        check(argv[1:] == ["open", "--token", "TOK", "--", "a", "b"], f"argv with token: {argv}")
        check(kw.get("start_new_session") is True, "start_new_session not set")
        for stream in ("stdin", "stdout", "stderr"):
            check(kw.get(stream) == subprocess.DEVNULL, f"{stream} not DEVNULL")
    mod.activation_token = lambda: None
    item.emit("activate")
    if len(calls) == 2:
        check(calls[-1][0][1:] == ["open", "--", "a", "b"], f"argv without token: {calls[-1][0]}")
    else:
        check(False, "second launch missing")
    check(mod.build_command(["-x"], None)[1:] == ["open", "--", "-x"], "paths after --")
    mod.subprocess.Popen = real_popen

    # AC4 timing: a command that runs 5 seconds must not block.
    slow = tempfile.NamedTemporaryFile("w", delete=False, suffix=".sh")
    slow.write("#!/bin/sh\nsleep 5\n")
    slow.close()
    os.chmod(slow.name, 0o755)
    mod.SBARHIVE_BIN = slow.name
    start = time.monotonic()
    proc = mod.launch(["x"])
    elapsed = time.monotonic() - start
    check(elapsed < 0.1, f"launch blocked for {elapsed:.3f}s")
    check(proc is not None, "launch returned no process")
    if proc is not None:
        try:
            os.killpg(proc.pid, 15)
        except OSError:
            pass
    os.unlink(slow.name)

    if FAILS:
        print(f"FAIL test_nautilus {mode} ({len(FAILS)} failed)")
        return 1
    print(f"PASS test_nautilus {mode}")
    return 0


if __name__ == "__main__":
    sys.exit(run(sys.argv[1] if len(sys.argv) > 1 else "real"))
