#!/usr/bin/env bash
# S-052, tests from the acceptance criteria only: the owner-run Windows beta
# checklist, its two .cmd scripts, the README section and the CHANGELOG line.
# Light (a few seconds), no cargo. Run: bash tests/windows/test_checklist.sh
#
# Contracts the acceptance criteria leave open, fixed here:
# - a step starts with a line "Wnn <title>" (optionally after "#" marks); its
#   Do:, Expect: and Result: PASS | FAIL lines start at column 0, in that
#   order, before the next step id;
# - "lines of steps" (AC1) are the non blank lines from the first step id to
#   the heading "Report" (at most 60);
# - the "Before you start" and "Report" headings are "## " headings, the first
#   and the last section;
# - the README sections are "## Windows beta" (English) and a "## " heading
#   with Windows and an Arabic word for beta (Arabic).
#
# Reduced or deferred (S-047 drop, S-048 tray and hotkey, S-049 toast are not
# built): AC2 keywords drag, shortcut, tray, toast and Open folder are NOT
# required; AC3 shortcut timing from hold_ms is NOT tested; AC7 toast
# screenshot is NOT required (the hive screenshot is). Everything else in
# AC1 to AC8 is tested.
#
# Extra, requested by the manager for an honest tonight checklist (not an AC):
# the steps also cover upgrade, logon, pointer, shadow, focus and the result
# saved next to the original.
set -u
# shellcheck source=tests/windows/lib.sh
. "$(dirname "$0")/lib.sh"
cd "$REPO_ROOT" || exit 2
export LC_ALL=C.UTF-8

CL=docs/windows-beta-checklist.md
SMOKE=scripts/windows/beta-smoke.cmd
LOGS=scripts/windows/collect-logs.cmd
RD=README.md
RA=README.ar.md
T="$(mktemp -d "${TMPDIR:-/tmp}/sbarhive-s052.XXXXXX")"
trap 'case "$T" in "${TMPDIR:-/tmp}"/sbarhive-s052.*) rm -rf -- "$T" ;; esac' EXIT

cat > "$T/check.py" <<'PY'
import re, sys

CL, SMOKE, LOGS, RD, RA = sys.argv[1:6]


def read(p):
    try:
        with open(p, encoding="utf-8", newline="") as f:
            return f.read()
    except OSError:
        return None


def out(ok, name, why=""):
    print(("ok " if ok else "FAIL ") + name + ("" if ok else ": " + why))


def section(text, head_re):
    """Lines from the matching '## ' heading to the next '## ' heading."""
    lines = text.splitlines()
    start = None
    for i, l in enumerate(lines):
        if start is None and re.match(r"^## +" + head_re, l):
            start = i
        elif start is not None and l.startswith("## "):
            return "\n".join(lines[start:i])
    return None if start is None else "\n".join(lines[start:])


cl = read(CL)
ID = re.compile(r"^(?:#+ +)?W(\d+)\b")
steps = []  # (num, [lines])
rng = []
if cl is None:
    for n in ("AC1 file", "AC1 steps", "AC1 parts", "AC1 ids", "AC1 max 60 lines"):
        out(False, n, CL + " missing")
else:
    out(True, "AC1 file")
    lines = cl.splitlines()
    first = next((i for i, l in enumerate(lines) if ID.match(l)), None)
    rep = next((i for i, l in enumerate(lines) if re.match(r"^#+ +Report\b", l)), None)
    if first is None or rep is None or rep < first:
        out(False, "AC1 steps", "need step ids and a later Report heading")
    else:
        rng = lines[first:rep]
        cur = None
        for l in rng:
            m = ID.match(l)
            if m:
                cur = (m.group(1), [])
                steps.append(cur)
            elif cur:
                cur[1].append(l)
        out(len(steps) >= 10, "AC1 steps", "only %d steps" % len(steps))
        nums = [s[0] for s in steps]
        want = ["%02d" % i for i in range(1, len(steps) + 1)]
        out(nums == want, "AC1 ids consecutive W01..., two digits", "got %s" % nums)
        bad = []
        for n, body in steps:
            tags = [l for l in body if re.match(r"^(Do:|Expect:|Result:)", l)]
            kinds = [t.split(":")[0] for t in tags]
            res = [t for t in tags if t.startswith("Result:")]
            if kinds != ["Do", "Expect", "Result"] or res[0].strip() != "Result: PASS | FAIL":
                bad.append("W" + n)
            for t in tags:
                if t.startswith("Expect:") and re.search(r"\b(fast|nice)\b", t, re.I) and not re.search(r"\d", t):
                    bad.append("W%s vague Expect" % n)
        out(not bad, "AC1 Do/Expect/Result per step, no vague Expect", ", ".join(bad))
        used = [l for l in rng if l.strip()]
        out(len(used) <= 60, "AC1 at most 60 lines of steps", "%d lines" % len(used))

stext = "\n".join(rng)


def kw(name, pattern, flags=re.I):
    out(re.search(pattern, stext, flags) is not None, name, "not found in any step")


# AC2 keywords (drag, shortcut, tray, toast, Open folder deferred).
for label, pat, fl in [
    ("transparent corners", r"transparent corners", re.I),
    ("console window", r"console window", re.I),
    ("150%", r"150%", 0),
    ("Esc", r"\bEsc\b", 0),
    ("ten files", r"ten files", re.I),
    ("original", r"original", re.I),
    ("Arabic", r"Arabic", re.I),
    ("Show more options", r"Show more options", re.I),
    ("uninstall", r"uninstall", re.I),
    ("SmartScreen", r"SmartScreen", re.I),
    ("Narrator", r"Narrator", re.I),
    ("soffice.bin", r"soffice\.bin", re.I),
    ("ffmpeg", r"ffmpeg", re.I),
    ("Ghostscript", r"Ghostscript", re.I),
    ("LibreOffice", r"LibreOffice", re.I),
]:
    kw("AC2 keyword " + label, pat, fl)

# Extra, from the manager (not an AC).
for label, pat in [
    ("upgrade", r"upgrade"),
    ("logon", r"log ?on|log ?in"),
    ("pointer", r"pointer"),
    ("shadow", r"shadow"),
    ("focus", r"focus"),
    ("next to the original", r"next to the original"),
    ("edges", r"edge"),
    ("zip and MSI", r"\.msi|MSI"),
]:
    kw("EXTRA keyword " + label, pat)

# AC3
for label, pat in [
    ("150 ms", r"150 ?ms"),
    ("9 frames or fewer", r"(9 frames|nine frames)"),
    ("60 fps", r"60 ?fps"),
    ("three trials", r"(three|3) trials"),
    ("all three must pass", r"all three"),
]:
    kw("AC3 " + label, pat)

# AC4
if cl is None:
    out(False, "AC4 first section", "checklist missing")
else:
    heads = [l for l in cl.splitlines() if l.startswith("## ")]
    first_ok = bool(heads) and re.match(r"^## +Before you start\b", heads[0]) is not None
    out(first_ok, "AC4 first section is Before you start", "first ## heading: %s" % (heads[:1],))
    sec = section(cl, r"Before you start") or ""
    for label, pat in [
        ("Windows 10 22H2", r"22H2"),
        ("Windows 11", r"Windows 11|11\b"),
        ("test folder", r"%USERPROFILE%\\sbarhive-beta-test"),
        ("bounded size rule", r"bounded"),
        ("only windows the test opened are closed", r"(only|exact).*(clos)|clos.*(only|exact)"),
        ("zip", r"\.zip|\bzip\b"),
        ("MSI", r"\.msi|\bMSI\b"),
        ("run-tests.cmd", r"scripts[\\/]windows[\\/]run-tests\.cmd"),
        ("ISO", r"\bISO\b"),
    ]:
        out(re.search(pat, sec, re.I) is not None, "AC4 " + label, "not in Before you start")

# AC7
if cl is None:
    out(False, "AC7 Report section", "checklist missing")
else:
    lastm = [l for l in cl.splitlines() if l.startswith("## ")]
    out(bool(lastm) and re.match(r"^## +Report\b", lastm[-1]) is not None,
        "AC7 last section is Report", "last ## heading: %s" % (lastm[-1:],))
    rsec = section(cl, r"Report") or ""
    for label, pat in [
        ("PASS or FAIL list", r"PASS"),
        ("smoke.txt", r"smoke\.txt"),
        ("log", r"sbarhived\.log|\blog\b"),
        ("screenshots of the hive", r"screenshot.*hive|hive.*screenshot"),
    ]:
        out(re.search(pat, rsec, re.I) is not None, "AC7 report lists " + label, "missing")


# AC5
def crlf_ok(text):
    if not text:
        return False
    parts = text.split("\n")
    body = parts[:-1] if parts[-1] == "" else parts
    return all(p.endswith("\r") for p in body) and len(body) > 0


NET = re.compile(r"\b(curl|wget|bitsadmin|ftp|scp|certutil|powershell|pwsh|Invoke-\w+)\b|https?:|\\\\[A-Za-z]|net use", re.I)
smoke = read(SMOKE)
if smoke is None:
    for n in ("AC5 CRLF", "AC5 commands", "AC5 order", "AC5 output lines", "AC5 writes only to the test folder", "AC5 no network"):
        out(False, n, SMOKE + " missing")
else:
    out(crlf_ok(smoke), "AC5 CRLF on every line", "")
    need = ["--version", "integration install --dry-run", "sbarhived.exe", "ping", "smoke.txt", r"%USERPROFILE%\sbarhive-beta-test", "total", "failed"]
    miss = [n for n in need if n.lower() not in smoke.lower()]
    out(not miss, "AC5 commands and paths", "missing %s" % miss)
    pos = [smoke.lower().find(k) for k in ("--version", "--dry-run", "sbarhived.exe", " ping")]
    out(all(p >= 0 for p in pos) and pos == sorted(pos), "AC5 order version, dry-run, daemon, ping", str(pos))
    out(len(re.findall(r"errorlevel", smoke, re.I)) >= 4, "AC5 errorlevel written per command", "fewer than 4 errorlevel uses")
    sets = {}
    for m in re.finditer(r'(?im)^\s*set\s+"?(\w+)=([^"\r\n]*)', smoke):
        sets[m.group(1).upper()] = m.group(2)
    badt = []
    for l in smoke.splitlines():
        if re.match(r"(?i)^\s*(rem|::)", l):
            continue
        for m in re.finditer(r'(?<![0-9])>>?\s*("[^"]+"|[^\s&|)>]+)', l):
            t = m.group(1).strip('"')
            if t.startswith("&") or t.lower() == "nul":
                continue
            v = re.fullmatch(r"%(\w+)%.*", t)
            ok = "sbarhive-beta-test" in t.lower() or (
                v is not None and "sbarhive-beta-test" in sets.get(v.group(1).upper(), "").lower())
            if not ok:
                badt.append(t)
    outside = re.findall(r"(?i)%(?:temp|tmp|appdata|localappdata|windir|systemroot)%|\bC:\\", smoke)
    out(not badt and not outside, "AC5 writes only to the test folder", "targets %s, outside %s" % (badt, outside))
    out(NET.search(smoke) is None, "AC5 no network", "")

# AC6
logs = read(LOGS)
if logs is None:
    for n in ("AC6 CRLF", "AC6 sources and target", "AC6 prints the folder", "AC6 no upload"):
        out(False, n, LOGS + " missing")
else:
    out(crlf_ok(logs), "AC6 CRLF on every line", "")
    need = [r"%LOCALAPPDATA%\SbarHive\logs\sbarhived.log", r"%APPDATA%\SbarHive\config.toml", r"%USERPROFILE%\sbarhive-beta-test\report"]
    miss = [n for n in need if n.lower() not in logs.lower()]
    out(not miss, "AC6 sources and target", "missing %s" % miss)
    out(re.search(r"(?im)^\s*echo\b.*report", logs) is not None or re.search(r"(?im)^\s*echo\b.*%\w+%", logs) is not None,
        "AC6 prints the folder", "no echo of the report folder")
    out(NET.search(logs) is None, "AC6 no upload", "")

# AC7 forbidden patterns, in every new file and the README Windows sections.
dash = re.compile("[\u2013\u2014]")
rd, ra = read(RD), read(RA)
rds = section(rd, r"Windows beta") if rd else None
ras = section(ra, r"Windows") if ra else None
for name, text in [("checklist", cl), ("beta-smoke.cmd", smoke), ("collect-logs.cmd", logs),
                   ("README Windows beta", rds), ("README.ar Windows beta", ras)]:
    if text is None:
        out(False, "AC7 clean " + name, "missing")
        continue
    why = []
    if "/home/" in text:
        why.append("/home/")
    if re.search(r"C:\\Users\\(?![<%])[A-Za-z0-9_.-]+", text, re.I):
        why.append("C:\\Users\\name")
    if dash.search(text):
        why.append("long dash")
    out(not why, "AC7 no personal paths or long dashes in " + name, ", ".join(why))

# AC8
EN = r"Windows beta"
if rd is None or rds is None:
    out(False, "AC8 README section Windows beta", "no '## Windows beta' heading")
else:
    out(True, "AC8 README section Windows beta")
    for label, pat in [("SmartScreen", r"SmartScreen"), ("Show more options", r"Show more options"),
                       ("Windows 11", r"Windows 11"), ("unsigned", r"unsigned"),
                       ("LibreOffice", r"LibreOffice"), ("Ghostscript", r"Ghostscript"), ("FFmpeg", r"FFmpeg"),
                       ("not bundled", r"not bundled")]:
        out(re.search(pat, rds, re.I) is not None, "AC8 README EN " + label, "missing")
if ra is None:
    out(False, "AC8 README.ar section", "README.ar.md missing")
else:
    ar = None
    for l in ra.splitlines():
        if re.match(r"^## .*Windows", l) and re.search("\u062a\u062c\u0631\u064a\u0628|\u0628\u064a\u062a\u0627", l):
            ar = section(ra, re.escape(l[3:].strip()))
            break
    if ar is None:
        out(False, "AC8 README.ar section Windows beta", "no '## ... Windows ...' heading with an Arabic word for beta")
    else:
        out(True, "AC8 README.ar section Windows beta")
        for label, pat in [("SmartScreen", r"SmartScreen"), ("Windows 11", r"Windows 11"),
                           ("Show more options", r"Show more options|\u0627\u0644\u0645\u0632\u064a\u062f \u0645\u0646 \u0627\u0644\u062e\u064a\u0627\u0631\u0627\u062a"),
                           ("LibreOffice", r"LibreOffice"), ("Ghostscript", r"Ghostscript"), ("FFmpeg", r"FFmpeg")]:
            out(re.search(pat, ar, re.I) is not None, "AC8 README.ar " + label, "missing")

cg = read("CHANGELOG.md") or ""
unrel = section(cg, r"\[Unreleased\]") or ""
n = len([l for l in unrel.splitlines() if re.match(r"^- [^(]*\(S-052\):", l)])
out(n == 1, "AC8 exactly one CHANGELOG line for S-052 under Unreleased", "found %d" % n)
PY

while IFS= read -r line; do
  case "$line" in
    "ok "*) pass ;;
    "FAIL "*) fail "${line#FAIL }" ;;
    *) fail "checker output: $line" ;;
  esac
done < <(python3 -I "$T/check.py" "$CL" "$SMOKE" "$LOGS" "$RD" "$RA" 2>&1)

# AC5, AC6: `file` says CRLF.
for f in "$SMOKE" "$LOGS"; do
  if [ -f "$f" ]; then
    case "$(file -b "$f")" in *"CRLF line terminators"*) pass ;; *) fail "AC5/AC6 file says no CRLF for $f" ;; esac
  else
    fail "AC5/AC6 $f missing"
  fi
done

# AC8: this test is shellcheck clean, and the design lint is clean.
if command -v shellcheck > /dev/null 2>&1; then
  check "AC8 shellcheck clean" shellcheck -x "$0"
else
  fail "AC8 shellcheck not installed"
fi
check "AC8 design-lint clean" bash scripts/design-lint.sh > /dev/null 2>&1

echo "test_checklist: $CHECKS checks, $FAILS failed"
[ "$FAILS" -eq 0 ]
