#!/usr/bin/env bash
# S-085, tests from the acceptance criteria only: the Windows app identity.
#   tests/windows/test_s085.sh [RELEASE_DIR]
# RELEASE_DIR holds kawaro.exe and kawarod.exe. The default is the release
# tree of scripts/windows/build.sh:
#   $CARGO_TARGET_DIR/x86_64-pc-windows-gnu/release when CARGO_TARGET_DIR is set,
#   else ${XDG_CACHE_HOME:-$HOME/.cache}/kawaro-win-target/x86_64-pc-windows-gnu/release.
# Exit 2 when either exe is absent.
#
# AC1 (D4): a stdlib-only python3 PE parser reads both built exes. Each must
# carry an RT_GROUP_ICON whose entries resolve to RT_ICON resources with 16,
# 32, 48 and 256 px images, and an RT_VERSION whose StringFileInfo has
# ProductName and FileDescription both equal to "Kawaro | كوارو". The strings
# are read back from the exe; no toolchain is trusted (D5).
# AC2, AC3, D6: static checks of installer/windows/kawaro.wxs (ASCII only, no
# numeric character entities, the Start shortcut icon, the RemoveFile row for
# the renamed shortcut, the pinned UpgradeCode).
# Light: no cargo, no wixl.
# shellcheck disable=SC2016
set -u
# shellcheck source=tests/windows/lib.sh
. "$(dirname "$0")/lib.sh"
cd "$REPO_ROOT" || exit 2
export LC_ALL=C.UTF-8

if [ "$#" -gt 1 ]; then
  echo "usage: tests/windows/test_s085.sh [RELEASE_DIR]" >&2
  exit 2
fi
if [ "$#" -eq 1 ]; then
  REL="$1"
elif [ -n "${CARGO_TARGET_DIR:-}" ]; then
  REL="$CARGO_TARGET_DIR/x86_64-pc-windows-gnu/release"
else
  REL="$WIN_TARGET_DEFAULT/x86_64-pc-windows-gnu/release"
fi
for exe in kawaro.exe kawarod.exe; do
  if [ ! -f "$REL/$exe" ]; then
    echo "test_s085: missing $REL/$exe, run scripts/windows/build.sh first" >&2
    exit 2
  fi
done

# S085_WXS only proves this checker against a scratch copy; CI never sets it.
WXS="${S085_WXS:-installer/windows/kawaro.wxs}"
NAME='Kawaro | كوارو'
UPGRADE_CODE=6f2d5c1e-8a47-4b39-9d0e-5b1c7a3e2f84
T="$(mktemp -d "${TMPDIR:-/tmp}/kawaro-s085.XXXXXX")"
trap 'case "$T" in "${TMPDIR:-/tmp}"/kawaro-s085.*) rm -rf -- "$T" ;; esac' EXIT

cat > "$T/pe.py" <<'PY'
"""S-085 AC1: icon and version resources of one PE file, stdlib only."""
import struct
import sys

RT_ICON, RT_GROUP_ICON, RT_VERSION = 3, 14, 16
NEEDED_SIZES = (16, 32, 48, 256)
STRINGS = ("ProductName", "FileDescription")


def out(ok, name, why=""):
    print(("ok " if ok else "FAIL ") + name + ("" if ok else ": " + why))


def parse_sections(data):
    if data[:2] != b"MZ":
        raise ValueError("no MZ header")
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    if data[pe:pe + 4] != b"PE\0\0":
        raise ValueError("no PE signature")
    count = struct.unpack_from("<H", data, pe + 6)[0]
    opt_size = struct.unpack_from("<H", data, pe + 20)[0]
    opt = pe + 24
    magic = struct.unpack_from("<H", data, opt)[0]
    if magic == 0x20B:
        dirs = opt + 112
    elif magic == 0x10B:
        dirs = opt + 96
    else:
        raise ValueError("unknown optional header magic %#x" % magic)
    # Data directory 2 is the resource table.
    rsrc_rva, rsrc_size = struct.unpack_from("<II", data, dirs + 2 * 8)
    sections = []
    table = opt + opt_size
    for i in range(count):
        base = table + i * 40
        vsize, vaddr, rawsize, rawptr = struct.unpack_from("<IIII", data, base + 8)
        sections.append((vaddr, max(vsize, rawsize), rawptr, rawsize))
    return rsrc_rva, rsrc_size, sections


def rva_to_offset(rva, sections):
    for vaddr, span, rawptr, rawsize in sections:
        if vaddr <= rva < vaddr + span and rva - vaddr < rawsize:
            return rawptr + (rva - vaddr)
    raise ValueError("rva %#x outside every section" % rva)


def resources(data):
    """{type id: {name id: bytes}} using the first language of every name."""
    rsrc_rva, rsrc_size, sections = parse_sections(data)
    if rsrc_rva == 0 or rsrc_size == 0:
        return {}
    root = rva_to_offset(rsrc_rva, sections)

    def entries(offset):
        named, ids = struct.unpack_from("<HH", data, root + offset + 12)
        start = root + offset + 16
        for i in range(named + ids):
            name, target = struct.unpack_from("<II", data, start + i * 8)
            yield name, target

    found = {}
    for type_id, type_target in entries(0):
        if type_id & 0x80000000 or not type_target & 0x80000000:
            continue
        for name_id, name_target in entries(type_target & 0x7FFFFFFF):
            if name_id & 0x80000000 or not name_target & 0x80000000:
                continue
            for _lang, leaf in entries(name_target & 0x7FFFFFFF):
                if leaf & 0x80000000:
                    continue
                rva, size = struct.unpack_from("<II", data, root + leaf)
                at = rva_to_offset(rva, sections)
                found.setdefault(type_id, {})[name_id] = data[at:at + size]
                break
    return found


def icon_image_size(payload):
    """(width, height) of one RT_ICON payload: PNG IHDR or BITMAPINFOHEADER."""
    if payload[:8] == b"\x89PNG\r\n\x1a\n":
        return struct.unpack_from(">II", payload, 16)
    width, height = struct.unpack_from("<ii", payload, 4)
    return width, abs(height) // 2


def check_icons(label, res):
    groups = res.get(RT_GROUP_ICON, {})
    icons = res.get(RT_ICON, {})
    out(bool(groups), label + " AC1: RT_GROUP_ICON present", "no group icon resource")
    out(bool(icons), label + " AC1: RT_ICON present", "no icon image resource")
    sizes = set()
    dangling = []
    for _gid, group in sorted(groups.items()):
        if len(group) < 6:
            continue
        _reserved, kind, count = struct.unpack_from("<HHH", group, 0)
        if kind != 1:
            continue
        for i in range(count):
            # GRPICONDIRENTRY: 14 bytes, the last WORD is the RT_ICON id.
            entry = 6 + i * 14
            if entry + 14 > len(group):
                break
            bw, bh = group[entry], group[entry + 1]
            nid = struct.unpack_from("<H", group, entry + 12)[0]
            if nid not in icons:
                dangling.append(nid)
                continue
            w = 256 if bw == 0 else bw
            h = 256 if bh == 0 else bh
            iw, ih = icon_image_size(icons[nid])
            if (iw, ih) == (w, h):
                sizes.add(w)
            else:
                dangling.append(nid)
    out(not dangling, label + " AC1: every group entry resolves to a matching RT_ICON",
        "unresolved or mismatched ids %s" % dangling)
    for px in NEEDED_SIZES:
        out(px in sizes, "%s AC1: %d px icon image" % (label, px),
            "sizes found %s" % sorted(sizes))


def align4(n):
    return (n + 3) & ~3


def version_node(blob, at):
    """(length, value_length, type, key, value_start, children_start)."""
    length, value_length, kind = struct.unpack_from("<HHH", blob, at)
    key_start = at + 6
    end = key_start
    while end + 1 < len(blob) and blob[end:end + 2] != b"\0\0":
        end += 2
    key = blob[key_start:end].decode("utf-16-le", "replace")
    value_start = align4(end + 2)
    value_bytes = value_length * 2 if kind == 1 else value_length
    children = align4(value_start + value_bytes)
    return length, value_length, kind, key, value_start, children


def children_of(blob, at):
    length, _vl, _k, _key, _vs, child = version_node(blob, at)
    end = at + length
    while child + 6 <= end:
        clen = struct.unpack_from("<H", blob, child)[0]
        if clen == 0:
            break
        yield child
        child = align4(child + clen)


def version_strings(blob):
    found = {}
    _l, _vl, _k, key, _vs, _c = version_node(blob, 0)
    if key != "VS_VERSION_INFO":
        return found
    for block in children_of(blob, 0):
        if version_node(blob, block)[3] != "StringFileInfo":
            continue
        for table in children_of(blob, block):
            for item in children_of(blob, table):
                _len, vlen, kind, name, start, _ch = version_node(blob, item)
                raw = blob[start:start + vlen * 2] if kind == 1 else blob[start:start + vlen]
                found.setdefault(name, raw.decode("utf-16-le", "replace").split("\0", 1)[0])
    return found


def check_version(label, res, want):
    blobs = res.get(RT_VERSION, {})
    out(bool(blobs), label + " AC1: RT_VERSION present", "no version resource")
    found = {}
    for _vid, blob in sorted(blobs.items()):
        try:
            found.update(version_strings(blob))
        except (struct.error, IndexError) as error:
            out(False, label + " AC1: version resource parses", str(error))
    for key in STRINGS:
        got = found.get(key)
        out(got == want, "%s AC1: %s is the bilingual name" % (label, key),
            "got %r want %r" % (got, want))


def main():
    path, label, want = sys.argv[1], sys.argv[2], sys.argv[3]
    with open(path, "rb") as handle:
        data = handle.read()
    try:
        res = resources(data)
    except (ValueError, struct.error) as error:
        out(False, label + " AC1: PE resource directory parses", str(error))
        res = {}
    check_icons(label, res)
    check_version(label, res, want)


main()
PY

ac1_exe_resources () {
  echo "== AC1: icon and version resources of the built exes ($REL)"
  local exe line
  for exe in kawaro.exe kawarod.exe; do
    while IFS= read -r line; do
      case "$line" in
        "ok "*) pass ;;
        *) fail "${line#FAIL }" ;;
      esac
    done < <(python3 -I "$T/pe.py" "$REL/$exe" "$exe" "$NAME" 2>&1 || echo "FAIL $exe AC1: the PE parser crashed")
  done
}

ac2_wxs_is_ascii_without_numeric_entities () {
  echo "== AC2: kawaro.wxs stays ASCII (wixl segfaults on anything else)"
  check "AC2: $WXS exists" test -f "$WXS"
  check "AC2: $WXS has no byte >= 0x80" bash -c '! LC_ALL=C grep -qP "[\x80-\xFF]" "$1"' _ "$WXS"
  check "AC2: $WXS has no numeric character entity" bash -c '! grep -qiE "&#(x[0-9a-f]+|[0-9]+);" "$1"' _ "$WXS"
}

# element WXS NAME: every NAME start tag on one line each (attributes may wrap).
element () {
  python3 -I -c '
import re, sys
text = open(sys.argv[1], encoding="ascii", errors="replace").read()
text = re.sub(r"<!--.*?-->", "", text, flags=re.S)
for tag in re.findall(r"<" + sys.argv[2] + r"\b[^>]*>", text):
    print(" ".join(tag.split()))
' "$1" "$2"
}

# component_body WXS ID: the inner text of the Component with Id=ID.
component_body () {
  python3 -I -c '
import re, sys
text = open(sys.argv[1], encoding="ascii", errors="replace").read()
text = re.sub(r"<!--.*?-->", "", text, flags=re.S)
m = re.search(r"<Component\b[^>]*\bId=\"" + re.escape(sys.argv[2]) + r"\"[^>]*>(.*?)</Component>", text, re.S)
print(" ".join(m.group(1).split()) if m else "")
' "$1" "$2"
}

d6_start_shortcut_icon_and_cleanup () {
  echo "== D6: the Start shortcut icon and the renamed shortcut cleanup"
  local sc icon body
  sc="$(element "$WXS" Shortcut | grep -F 'Id="StartShortcut"')"
  check "D6: one StartShortcut element" test "$(printf '%s\n' "$sc" | grep -c .)" = 1
  check "D6: the Start shortcut keeps the ASCII name Kawaro" grep -qF 'Name="Kawaro"' <<<"$sc"
  check "D6: the Start shortcut has Icon=\"StartIcon\"" grep -qF 'Icon="StartIcon"' <<<"$sc"
  check "D6: the Start shortcut has an IconIndex" grep -qE 'IconIndex="[0-9]+"' <<<"$sc"
  icon="$(element "$WXS" Icon | grep -F 'Id="StartIcon"')"
  check "D6: one Icon element with Id=\"StartIcon\"" test "$(printf '%s\n' "$icon" | grep -c .)" = 1
  check "D6: the Icon element reads SourceFile=\"\$(var.Ico)\" (wixl ignores Source)" grep -qF 'SourceFile="$(var.Ico)"' <<<"$icon"
  check "D6: the icon is required, no <?ifdef Ico?> block" bash -c '! grep -qF "<?ifdef Ico?>" "$1"' _ "$WXS"
  body="$(component_body "$WXS" StartMenuShortcut)"
  check "D6: the StartMenuShortcut component exists" test -n "$body"
  check "D6: StartMenuShortcut removes Kawaro*.lnk on uninstall" \
    bash -c 'grep -qE "<RemoveFile\b[^>]*\bName=\"Kawaro\*\.lnk\"[^>]*\bOn=\"(uninstall|both)\"|<RemoveFile\b[^>]*\bOn=\"(uninstall|both)\"[^>]*\bName=\"Kawaro\*\.lnk\"" <<<"$1"' _ "$body"
}

ac3_upgrade_code_is_pinned () {
  echo "== AC3: the UpgradeCode never changes"
  local codes
  codes="$(grep -oE 'UpgradeCode="[^"]*"' "$WXS")"
  assert_eq "AC3: exactly one UpgradeCode, byte for byte" "$codes" "UpgradeCode=\"$UPGRADE_CODE\""
}

ac1_exe_resources
ac2_wxs_is_ascii_without_numeric_entities
d6_start_shortcut_icon_and_cleanup
ac3_upgrade_code_is_pinned
finish "S-085 windows identity"
