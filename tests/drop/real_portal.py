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
  real_portal.py triggers MONLOG
      Reads `dbus-monitor` output and prints, for every
      org.freedesktop.portal.Request Response that binds the shortcut "drop"
      (the BindShortcuts reply the daemon gets), one line: the
      trigger_description of "drop" in that reply, as sent (possibly
      empty). S-014e3: xdg-desktop-portal-hyprland 1.4.1 sends "".
  real_portal.py tokens MONLOG
      S-014f3 (AC15): as signals, with a fifth field, the activation_token
      in the signal's options ("-" when it has none): "HH:MM:SS.ffffff SIDE
      MEMBER SHORTCUT_ID TOKEN", in UTC.
  real_portal.py last MONLOG FROM BEFORE
      S-014f3 (AC15 b): the activation_token of the last frontend to client
      ("portal" side) Activated of "drop" stamped at or after FROM and
      before BEFORE (HH:MM:SS.ffffff), and the one before it: "TOKEN
      PREVIOUS COUNT", "none" for what is missing, COUNT the Activated
      signals in that span.
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


TOKEN = re.compile(r'^\s+string "activation_token"$')
VSTR = re.compile(r'^\s+variant\s+string "(.*)"$')


def token_signals(path):
    """[(seconds of day, side, member, id, token or "-")] in log order."""
    out, cur, want = [], None, False
    with open(path, encoding="utf-8", errors="replace") as f:
        for line in f:
            m = HEAD.match(line)
            if m:
                if cur:
                    out.append(cur)
                t = datetime.datetime.fromtimestamp(float(m.group(1)), datetime.timezone.utc)
                side = "impl" if ".impl." in m.group(2) else "portal"
                cur = [t.strftime("%H:%M:%S.%f"), side, m.group(3), "-", "-"]
                want = False
                continue
            if cur is None:
                continue
            if not line.startswith(" "):
                out.append(cur)
                cur = None
                continue
            if want:
                v = VSTR.match(line)
                if v:
                    cur[4] = v.group(1)
                want = False
            elif TOKEN.match(line):
                want = True
            elif cur[3] == "-":
                s = STR.match(line)
                if s:
                    cur[3] = s.group(1)
    if cur:
        out.append(cur)
    return out


def tokens(path):
    for row in token_signals(path):
        print(*row)


def last(path, frm, before):
    f, b = tod(frm), tod(before)
    toks = [r[4] for r in token_signals(path)
            if r[1] == "portal" and r[2] == "Activated" and r[3] == "drop" and f <= tod(r[0]) < b]
    print(toks[-1] if toks else "none", toks[-2] if len(toks) > 1 else "none", len(toks))


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


RESP = re.compile(r"^signal time=[0-9.]+ .*interface=org\.freedesktop\.portal\.Request; member=Response")


def triggers(path):
    """The trigger_description of "drop" in each Response that binds it."""
    blocks, cur = [], None
    with open(path, encoding="utf-8", errors="replace") as f:
        for line in f:
            if line.startswith("signal ") or line.startswith("method "):
                if cur is not None:
                    blocks.append(cur)
                cur = [] if RESP.match(line) else None
            elif cur is not None:
                cur.append(line.rstrip("\n"))
    if cur is not None:
        blocks.append(cur)
    for b in blocks:
        in_drop = False
        for i, line in enumerate(b):
            s = STR.match(line)
            if not s:
                continue
            if s.group(1) == "drop":
                in_drop = True
            elif in_drop and s.group(1) == "trigger_description" and i + 1 < len(b):
                v = re.match(r'^\s+variant\s+string "(.*)"$', b[i + 1])
                if v:
                    print(v.group(1))
                    break


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
    elif cmd == "triggers":
        triggers(*args)
    elif cmd == "tokens":
        tokens(*args)
    elif cmd == "last":
        last(*args)
    else:
        raise SystemExit(__doc__)


main()
