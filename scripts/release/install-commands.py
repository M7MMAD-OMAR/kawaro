#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""One source of install commands (S-112).

  install-commands.py render NAME   print a channel's commands, one per line
  install-commands.py check         exit 1 when a docs file drifts from the JSON

The repository root is two levels above this file; the data is
packaging/install-commands.json.
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
NOT_EN = "Not usable yet"
NOT_AR = "غير قابل للاستعمال بعد"
# channel -> (page holding its commands, text that starts its row in the tables)
PAGES = {
    "apt": ("docs/distribution-channels/apt.md", "apt (Debian"),
    "copr": ("docs/distribution-channels/copr.md", "Fedora COPR"),
    "aur": ("docs/distribution-channels/aur.md", "Arch AUR"),
    "winget": ("docs/distribution.md", "| WinGet |"),
}
TABLES = ("docs/distribution.md", "docs/distribution.ar.md")
# Tables list the commands of every channel except apt (its line is too long).
TABLE_CHANNELS = ("copr", "aur", "winget")
GUARDED = (
    "README.md",
    "README.ar.md",
    "docs/guide/install.md",
    "docs/guide/install.ar.md",
)
MARKERS = ("yay -S", "dnf copr enable", "winget install", "/etc/apt/keyrings")


def read(rel):
    return (ROOT / rel).read_text(encoding="utf-8")


def load():
    return json.loads(read("packaging/install-commands.json"))["channels"]


def check():
    errs = []
    ch = load()
    for name, c in ch.items():
        page, row = PAGES[name]
        text = read(page)
        for cmd in c["commands"]:
            if cmd not in text:
                errs.append(f"{page}: missing command of {name}: {cmd}")
    for table in TABLES:
        text = read(table)
        lines = text.splitlines()
        for name, c in ch.items():
            row = [ln for ln in lines if PAGES[name][1] in ln and ln.startswith("|")]
            if not row:
                errs.append(f"{table}: no row for {name}")
                continue
            if name in TABLE_CHANNELS:
                for cmd in c["commands"]:
                    if cmd not in row[0]:
                        errs.append(f"{table}: row of {name} lacks: {cmd}")
            phrase = NOT_AR if table.endswith(".ar.md") else NOT_EN
            has = phrase in row[0]
            if c["status"] == "live" and has:
                errs.append(f"{table}: {name} is live but says '{phrase}'")
            if c["status"] != "live" and not has:
                errs.append(f"{table}: {name} is {c['status']} but lacks '{phrase}'")
    allowed = {cmd for c in ch.values() for cmd in c["commands"]}
    for rel in GUARDED:
        for n, ln in enumerate(read(rel).splitlines(), 1):
            s = ln.strip().strip("`").strip()
            if s.startswith("$ "):
                s = s[2:].strip()
            if any(m in s for m in MARKERS) and s not in allowed:
                errs.append(f"{rel}:{n}: channel command not in the JSON: {s}")
    for e in errs:
        print(e)
    return 1 if errs else 0


def main(argv):
    if len(argv) == 3 and argv[1] == "render":
        c = load().get(argv[2])
        if c is None:
            print(f"unknown channel: {argv[2]}", file=sys.stderr)
            return 1
        print("\n".join(c["commands"]))
        return 0
    if len(argv) == 2 and argv[1] == "check":
        return check()
    print(__doc__, file=sys.stderr)
    return 2


sys.exit(main(sys.argv))
