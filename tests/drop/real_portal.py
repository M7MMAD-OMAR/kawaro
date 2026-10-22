#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Helpers for the real portal runs of S-014e2 (AC20, AC21): the "portal" and
"portal-real" modes of tests/drop/gnome.sh and tests/drop/kwin.sh, sourced
through tests/drop/portal_real.sh. Standard library only.

  real_portal.py stamp RAW STAMPS
      Copies stdin (the daemon's stdout and stderr) line by line to RAW, and
      writes every line that starts with "portal" to STAMPS too, after a
      "[HH:MM:SS.ffffff]" UTC stamp taken when the line arrived (the clock
      of `date -u +%T.%6N` and of WAYLAND_DEBUG). The daemon's own "portal:"
      lines carry no stamp.
  real_portal.py signals MONLOG
      Reads `dbus-monitor` output and prints one line per Activated or
      Deactivated signal of org.freedesktop.impl.portal.GlobalShortcuts
      (the backend to the frontend, "impl") or
      org.freedesktop.portal.GlobalShortcuts (the frontend to the client,
      "portal"): "HH:MM:SS.ffffff SIDE MEMBER SHORTCUT_ID", in UTC.
  real_portal.py hold STAMPS FROM RELEASE
      In STAMPS, the first "portal: activated drop" stamped at or after FROM
      (HH:MM:SS.ffffff) and the first "portal: deactivated drop" after it.
      Prints "ACT DEACT HELD_MS AFTER_RELEASE_MS NACT": HELD_MS from that
      activated to that deactivated, AFTER_RELEASE_MS from RELEASE to that
      deactivated, NACT the activated lines between them (key repeat);
      "none" for what is missing.
  real_portal.py count STAMPS FROM LINE
      The number of lines equal to LINE stamped at or after FROM.
"""
import datetime
import re
import sys


def tod(s):
    h, m, rest = s.strip("[]").split(":")
    return int(h) * 3600 + int(m) * 60 + float(rest)


def now():
    return datetime.datetime.now(datetime.timezone.utc).strftime("%H:%M:%S.%f")


def stamp(raw, stamps):
    with open(raw, "ab", buffering=0) as r, open(stamps, "a", encoding="utf-8") as s:
        for line in sys.stdin.buffer:
            r.write(line)
            if line.startswith(b"portal"):
                s.write("[%s] %s\n" % (now(), line.decode("utf-8", "replace").rstrip("\n")))
                s.flush()


HEAD = re.compile(r"^signal time=([0-9.]+) .*interface=(org\.freedesktop\.(?:impl\.)?portal\.GlobalShortcuts); member=(Activated|Deactivated)")
STR = re.compile(r'^\s+string "(.*)"$')


def signals(path):
    cur = None
    with open(path, encoding="utf-8", errors="replace") as f:
        for line in f:
            m = HEAD.match(line)
            if m:
                if cur:
                    print(*cur)
                t = datetime.datetime.fromtimestamp(float(m.group(1)), datetime.timezone.utc)
                side = "impl" if ".impl." in m.group(2) else "portal"
                cur = [t.strftime("%H:%M:%S.%f"), side, m.group(3), "-"]
                continue
            if cur and cur[3] == "-":
                s = STR.match(line)
                if s:
                    cur[3] = s.group(1)
            elif cur and not line.startswith(" "):
                print(*cur)
                cur = None
    if cur:
        print(*cur)


def entries(path):
    out = []
    with open(path, encoding="utf-8", errors="replace") as f:
        for line in f:
            m = re.match(r"^\[([0-9:.]+)\] (.*)$", line.rstrip("\n"))
            if m:
                out.append((tod(m.group(1)), m.group(2)))
    return out


def ms(a, b):
    d = b - a
    if d < -43200:
        d += 86400
    return "%.1f" % (d * 1000)


def hold(path, frm, release):
    f, rel = tod(frm), tod(release)
    act = deact = None
    nact = 0
    for t, text in entries(path):
        if t < f:
            continue
        if act is None:
            if text == "portal: activated drop":
                act = t
                nact = 1
        elif text == "portal: activated drop":
            nact += 1
        elif text == "portal: deactivated drop":
            deact = t
            break
    fmt = lambda t: "none" if t is None else "%.6f" % t
    print(fmt(act), fmt(deact),
          ms(act, deact) if act is not None and deact is not None else "none",
          ms(rel, deact) if deact is not None else "none",
          nact)


def count(path, frm, want):
    f = tod(frm)
    print(sum(1 for t, text in entries(path) if t >= f and text == want))


def main():
    cmd, args = sys.argv[1], sys.argv[2:]
    if cmd == "stamp":
        stamp(*args)
    elif cmd == "signals":
        signals(*args)
    elif cmd == "hold":
        hold(*args)
    elif cmd == "count":
        count(*args)
    else:
        raise SystemExit(__doc__)


main()
