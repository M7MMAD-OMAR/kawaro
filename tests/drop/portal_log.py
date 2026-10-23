#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Reads the log of tests/drop/shortcuts_portal.py for tests/drop/portal.sh
(S-014e1, S-014f2). Standard library only.

  portal_log.py order LOG [FROM]
      Only the lines after line FROM (default 0) count. For the daemon's
      portal connection there (the sender of the first CreateSession, else
      of the first Register), prints
        SENDER|FIRST|APPID|NEXT_GS|NBIND|MAXPER|BINDSESS|NSC|ID|TRIGGER|DESC
      FIRST: its first portal method call (a "call" line on an
      org.freedesktop.portal.* or org.freedesktop.host.portal.* interface;
      property reads are not method calls), as IFACE.METHOD; APPID: its
      first argument when it is Register; NEXT_GS: the first
      GlobalShortcuts method after FIRST; NBIND: its BindShortcuts calls;
      MAXPER: the most on one session; BINDSESS: "ok" when every one names
      the session_handle of a CreateSession Response to this sender, else
      "bad" ("none" without BindShortcuts); NSC, ID, TRIGGER, DESC: the
      number of shortcuts in the first BindShortcuts, and the first one's
      id, preferred_trigger and description ("-" when missing).
  portal_log.py calls LOG FROM [METHOD]
      The "call" and "get" lines after line FROM (only "call" lines of
      IFACE.METHOD ending in .METHOD when METHOD is given).
  portal_log.py emit LOG FROM SIGNAL
      The "[HH:MM:SS.ffffff]" stamp of the first "emit SIGNAL" line after
      line FROM, or "none".
  portal_log.py held LOG
      "yes" when a "held N" line has no "response N" line after it, else "no".
  portal_log.py senders LOG FROM
      The number of distinct senders of "call" and "get" lines on portal
      interfaces after line FROM.
  portal_log.py by LOG FROM SENDER
      The number of "call" and "get" lines of SENDER after line FROM.
  portal_log.py first LOG FROM METHOD
      The "[HH:MM:SS.ffffff]" stamp of the first "call" line of IFACE.METHOD
      ending in .METHOD after line FROM, or "none".
  portal_log.py session LOG [FROM]
      The session_handle the first CreateSession of the daemon's portal
      connection after line FROM was given (as for order), or "none".
  portal_log.py closes LOG FROM TO SESSION
      The number of org.freedesktop.portal.Session.Close calls on SESSION in
      the lines after FROM up to TO (a line number, or "end").
  portal_log.py sessions LOG FROM
      "MAX FINAL": the most sessions open at once after line FROM (the
      count at FROM when no session opened or closed after it) and the
      count at the end, from the "opened" and "closed" lines.
  portal_log.py lang LANG TEXT
      Exit 0 when TEXT is non-empty and in LANG: "en" Latin letters and no
      Arabic letter, "ar" Arabic letters."""
import json
import re
import sys

LINE = re.compile(r"^\[([0-9:.]+)\] (\S+) (.*)$")
CALL = re.compile(r"^(\d+) (\S+) (\S+) (.*)$")
PORTAL = ("org.freedesktop.portal.", "org.freedesktop.host.portal.")
GS = "org.freedesktop.portal.GlobalShortcuts."


def lines(path):
    with open(path, encoding="utf-8", errors="replace") as f:
        return [l.rstrip("\n") for l in f]


def parse(path):
    out = []
    for i, raw in enumerate(lines(path), 1):
        m = LINE.match(raw)
        if not m:
            continue
        stamp, kind, rest = m.groups()
        e = {"line": i, "stamp": stamp, "kind": kind, "rest": rest}
        if kind in ("call", "get"):
            c = CALL.match(rest) if kind == "call" else re.match(r"^(\d+) (\S+) (\S+)()$", rest)
            if not c:
                continue
            e["n"], e["member"], e["sender"], args = int(c.group(1)), c.group(2), c.group(3), c.group(4)
            if kind == "call":
                if " on /" in args:
                    args, on = args.rsplit(" on /", 1)
                    e["on"] = "/" + on
                try:
                    e["args"] = json.loads(args)
                except ValueError:
                    e["args"] = None
        elif kind == "response":
            r = re.match(r"^(\d+) (\d+) (\S+) (.*)$", rest)
            if r:
                e["n"], e["code"], e["path"] = int(r.group(1)), int(r.group(2)), r.group(3)
                try:
                    e["results"] = json.loads(r.group(4))
                except ValueError:
                    e["results"] = None
        elif kind == "held":
            e["n"] = int(rest.split()[0])
        elif kind in ("opened", "closed"):
            e["open"] = int(rest.split()[-1])
        out.append(e)
    return out


def daemon_conn(path, frm=0):
    """(events after FROM, the daemon connection's calls, its sessions)."""
    ev = [e for e in parse(path) if e["line"] > frm]
    calls = [e for e in ev if e["kind"] == "call" and e["member"].startswith(PORTAL)]
    sender = next((e["sender"] for e in calls if e["member"] == GS + "CreateSession"), None)
    if sender is None:
        sender = next((e["sender"] for e in calls if e["member"].endswith(".Register")), None)
    if sender is None:
        return ev, [], []
    mine = [e for e in calls if e["sender"] == sender]
    # Sessions this sender was given, from the Responses to its CreateSession.
    made = [e["n"] for e in mine if e["member"] == GS + "CreateSession"]
    sessions = []
    for n in made:
        for e in ev:
            if e["kind"] == "response" and e.get("n") == n and e.get("code") == 0 and e.get("results"):
                s = e["results"].get("session_handle")
                if s:
                    sessions.append(s)
    return ev, mine, sessions


def order(path, frm=0):
    _ev, mine, sessions = daemon_conn(path, frm)
    if not mine:
        return "none|none|-|none|0|0|none|0|-|-|-"
    sender = mine[0]["sender"]
    first = mine[0]
    appid = "-"
    if first["member"].endswith(".Register") and first["args"]:
        appid = str(first["args"][0])
    next_gs = next((e["member"][len(GS):] for e in mine[1:] if e["member"].startswith(GS)), "none")
    sessions = set(sessions)
    binds = [e for e in mine if e["member"] == GS + "BindShortcuts"]
    per = {}
    for b in binds:
        key = b["args"][0] if b["args"] else None
        per[key] = per.get(key, 0) + 1
    bsess = "none" if not binds else ("ok" if all(b["args"] and b["args"][0] in sessions for b in binds) else "bad")
    nsc, sid, trig, desc = 0, "-", "-", "-"
    if binds and binds[0]["args"] and len(binds[0]["args"]) > 1:
        sc = binds[0]["args"][1] or []
        nsc = len(sc)
        if sc:
            sid = str(sc[0][0])
            props = sc[0][1] if len(sc[0]) > 1 and isinstance(sc[0][1], dict) else {}
            trig = str(props.get("preferred_trigger", "-"))
            desc = str(props.get("description", "-")).replace("|", "/").replace("\n", " ")
    return "|".join(
        str(x)
        for x in (sender, first["member"], appid, next_gs, len(binds), max(per.values(), default=0), bsess, nsc, sid, trig, desc)
    )


def main():
    cmd = sys.argv[1]
    if cmd == "order":
        print(order(sys.argv[2], int(sys.argv[3]) if len(sys.argv) > 3 else 0))
    elif cmd == "senders":
        frm = int(sys.argv[3])
        print(len({e["sender"] for e in parse(sys.argv[2]) if e["line"] > frm and e["kind"] in ("call", "get") and e["member"].startswith(PORTAL)}))
    elif cmd == "by":
        frm, who = int(sys.argv[3]), sys.argv[4]
        print(sum(1 for e in parse(sys.argv[2]) if e["line"] > frm and e["kind"] in ("call", "get") and e["sender"] == who))
    elif cmd == "first":
        frm, method = int(sys.argv[3]), sys.argv[4]
        for e in parse(sys.argv[2]):
            if e["line"] > frm and e["kind"] == "call" and e["member"].endswith("." + method):
                print(e["stamp"])
                return
        print("none")
    elif cmd == "session":
        _ev, _mine, sessions = daemon_conn(sys.argv[2], int(sys.argv[3]) if len(sys.argv) > 3 else 0)
        print(sessions[0] if sessions else "none")
    elif cmd == "closes":
        frm, to, sess = int(sys.argv[3]), sys.argv[4], sys.argv[5]
        print(sum(
            1
            for e in parse(sys.argv[2])
            if e["line"] > frm and (to == "end" or e["line"] <= int(to)) and e["kind"] == "call"
            and e["member"] == "org.freedesktop.portal.Session.Close" and e.get("on") == sess
        ))
    elif cmd == "sessions":
        frm = int(sys.argv[3])
        now, most = 0, None
        for e in parse(sys.argv[2]):
            if "open" not in e:
                continue
            if e["line"] > frm and most is None:
                most = now
            now = e["open"]
            if e["line"] > frm:
                most = max(most, now)
        print(f"{now if most is None else most} {now}")
    elif cmd == "calls":
        frm = int(sys.argv[3])
        method = sys.argv[4] if len(sys.argv) > 4 else None
        n = 0
        for e in parse(sys.argv[2]):
            if e["line"] <= frm or e["kind"] not in ("call", "get"):
                continue
            if method and (e["kind"] != "call" or not e["member"].endswith("." + method)):
                continue
            n += 1
        print(n)
    elif cmd == "emit":
        frm, sig = int(sys.argv[3]), sys.argv[4]
        for e in parse(sys.argv[2]):
            if e["line"] > frm and e["kind"] == "emit" and e["rest"].split()[0] == sig:
                print(e["stamp"])
                return
        print("none")
    elif cmd == "held":
        ev = parse(sys.argv[2])
        answered = {e["n"] for e in ev if e["kind"] == "response" and "n" in e}
        print("yes" if any(e["kind"] == "held" and e["n"] not in answered for e in ev) else "no")
    elif cmd == "lang":
        t = sys.argv[3]
        ar = any("؀" <= c <= "ۿ" for c in t)
        latin = any(c.isascii() and c.isalpha() for c in t)
        ok = t.strip() != "" and t != "-" and ((sys.argv[2] == "en" and latin and not ar) or (sys.argv[2] == "ar" and ar))
        sys.exit(0 if ok else 1)
    else:
        sys.exit(f"unknown command {cmd}")


main()
