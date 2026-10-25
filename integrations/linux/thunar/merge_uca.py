#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Add or remove the Kawaro custom action in Thunar's uca.xml.

    merge_uca.py add    [--file F] [--snippet S] [--command CMD] [--seed SYS]
    merge_uca.py remove [--file F] [--seed SYS]

The action is keyed by <unique-id>kawaro</unique-id>. Other actions and
comments are kept. Both commands are idempotent: a run that changes nothing
does not rewrite the file. A file that does not parse is never rewritten.

--seed names the system file (normally /etc/xdg/Thunar/uca.xml). A user
uca.xml replaces the system one completely, so when the user file is missing
`add` starts from the seed to keep Thunar's default actions. `remove --seed`
deletes the user file when, without our action, it is identical to the seed
(or empty when the seed is absent): that is a file `add` created.

Python standard library only. Exit codes: 0 ok, 2 unreadable or invalid XML.
"""

import argparse
import copy
import os
import sys
import tempfile
import xml.etree.ElementTree as ET

UNIQUE_ID = "kawaro"
HERE = os.path.dirname(os.path.abspath(__file__))
DEFAULT_SNIPPET = os.path.join(HERE, "kawaro-uca.xml")
DECLARATION = '<?xml version="1.0" encoding="UTF-8"?>\n'


def default_file():
    base = os.environ.get("XDG_CONFIG_HOME") or os.path.join(os.path.expanduser("~"), ".config")
    return os.path.join(base, "Thunar", "uca.xml")


def parse(path):
    parser = ET.XMLParser(target=ET.TreeBuilder(insert_comments=True))
    return ET.parse(path, parser=parser).getroot()


def serialize(root):
    """Stable text form: whitespace-only text dropped, then re-indented."""
    clone = copy.deepcopy(root)
    for elem in clone.iter():
        if elem.text is not None and not elem.text.strip():
            elem.text = None
        if elem.tail is not None and not elem.tail.strip():
            elem.tail = None
    ET.indent(clone, space="\t")
    return DECLARATION + ET.tostring(clone, encoding="unicode") + "\n"


def find_ours(root):
    for index, child in enumerate(list(root)):
        if child.tag == "action" and (child.findtext("unique-id") or "").strip() == UNIQUE_ID:
            return index, child
    return None, None


def read_text(path):
    with open(path, encoding="utf-8") as handle:
        return handle.read()


def write_atomic(path, text):
    directory = os.path.dirname(path) or "."
    os.makedirs(directory, exist_ok=True)
    mode = os.stat(path).st_mode & 0o777 if os.path.exists(path) else 0o644
    fd, tmp = tempfile.mkstemp(prefix=".uca.", dir=directory)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as handle:
            handle.write(text)
        os.chmod(tmp, mode)
        os.replace(tmp, path)
    except BaseException:
        if os.path.exists(tmp):
            os.unlink(tmp)
        raise


def seed_root(seed):
    if seed and os.path.isfile(seed):
        return parse(seed)
    return ET.Element("actions")


def load_snippet(path, command):
    action = parse(path)
    if action.tag != "action" or (action.findtext("unique-id") or "").strip() != UNIQUE_ID:
        raise ValueError(f"{path}: expected <action> with unique-id {UNIQUE_ID}")
    if command is not None:
        action.find("command").text = command
    return action


def cmd_add(args):
    action = load_snippet(args.snippet, args.command)
    exists = os.path.exists(args.file)
    root = parse(args.file) if exists else seed_root(args.seed)
    if root.tag != "actions":
        raise ValueError(f"{args.file}: root element is <{root.tag}>, expected <actions>")
    index, _ = find_ours(root)
    if index is None:
        root.append(action)
    else:
        root[index] = action
    text = serialize(root)
    if exists and read_text(args.file) == text:
        print("thunar: action already present")
        return 0
    write_atomic(args.file, text)
    print("thunar: action updated" if index is not None else "thunar: action added")
    return 0


def cmd_remove(args):
    if not os.path.exists(args.file):
        print("thunar: no uca.xml, nothing to remove")
        return 0
    root = parse(args.file)
    index, _ = find_ours(root)
    if index is not None:
        del root[index]
    if args.seed is not None and serialize(root) == serialize(seed_root(args.seed)):
        os.unlink(args.file)
        print("thunar: removed uca.xml created by the installer")
        return 0
    if index is None:
        print("thunar: action not present")
        return 0
    write_atomic(args.file, serialize(root))
    print("thunar: action removed")
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(description="Merge the Kawaro action into Thunar's uca.xml.")
    sub = parser.add_subparsers(dest="command_name", required=True)
    for name in ("add", "remove"):
        p = sub.add_parser(name)
        p.add_argument("--file", default=default_file(), help="uca.xml to edit")
        p.add_argument("--seed", default=None, help="system uca.xml used when the user file is missing")
        if name == "add":
            p.add_argument("--snippet", default=DEFAULT_SNIPPET, help="file holding our <action>")
            p.add_argument("--command", default=None, help="command line to store in the action")
    args = parser.parse_args(argv)
    try:
        return cmd_add(args) if args.command_name == "add" else cmd_remove(args)
    except (ET.ParseError, ValueError, OSError) as exc:
        print(f"merge_uca: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
