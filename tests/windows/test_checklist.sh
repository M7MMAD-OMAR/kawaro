#!/usr/bin/env bash
# S-079, tests from the acceptance criteria only: the owner-run Windows beta
# checklist, its smoke and log scripts, and the static privacy checks.
# Light, no cargo and no live Windows execution.
set -u
# shellcheck source=tests/windows/lib.sh
. "$(dirname "$0")/lib.sh"
cd "$REPO_ROOT" || exit 2
export LC_ALL=C.UTF-8

CL=docs/windows-beta-checklist.md
SMOKE=scripts/windows/beta-smoke.cmd
LOGS=scripts/windows/collect-logs.cmd
T="$(mktemp -d "${TMPDIR:-/tmp}/kawaro-s079.XXXXXX")"
trap 'case "$T" in "${TMPDIR:-/tmp}"/kawaro-s079.*) rm -rf -- "$T" ;; esac' EXIT

cat > "$T/check.py" <<'PY'
import re
import sys

CL, SMOKE, LOGS = sys.argv[1:4]


def read(path):
    try:
        with open(path, encoding="utf-8", newline="") as handle:
            return handle.read()
    except OSError:
        return None


def out(ok, name, why=""):
    print(("ok " if ok else "FAIL ") + name + ("" if ok else ": " + why))


def section(text, heading):
    lines = text.splitlines()
    start = None
    for index, line in enumerate(lines):
        if start is None and re.match(r"^## +" + heading, line):
            start = index
        elif start is not None and line.startswith("## "):
            return "\n".join(lines[start:index])
    return None if start is None else "\n".join(lines[start:])


cl = read(CL)
ID = re.compile(r"^(?:#+ +)?W(\d+)\s+(.+?)\s*$")
steps = []
step_lines = []
pre_steps = ""
if cl is None:
    for name in (
        "AC1 checklist file",
        "AC1 consecutive steps and fields",
        "AC1 40 step and 160 line envelope",
        "AC2 stale and skipped shipped features",
    ):
        out(False, name, CL + " missing")
else:
    out(True, "AC1 checklist file")
    lines = cl.splitlines()
    first = next((i for i, line in enumerate(lines) if ID.match(line)), None)
    report = next(
        (i for i, line in enumerate(lines) if re.match(r"^## +Report\b", line)),
        None,
    )
    if first is None or report is None or report < first:
        out(False, "AC1 consecutive steps and fields", "need steps and a later Report heading")
        out(False, "AC1 40 step and 160 line envelope", "step range missing")
    else:
        pre_steps = "\n".join(lines[:first])
        step_lines = lines[first:report]
        current = None
        for line in step_lines:
            match = ID.match(line)
            if match:
                current = {
                    "id": "W" + match.group(1),
                    "title": match.group(2),
                    "body": [],
                }
                steps.append(current)
            elif current is not None:
                current["body"].append(line)
        numbers = [step["id"] for step in steps]
        expected = ["W%02d" % number for number in range(1, len(steps) + 1)]
        bad = []
        for step in steps:
            tags = [
                line for line in step["body"]
                if re.match(r"^(?:Do:|Expect:|Result:)", line)
            ]
            kinds = [tag.split(":", 1)[0] for tag in tags]
            step["do"] = "\n".join(
                tag[3:].strip() for tag in tags if tag.startswith("Do:")
            )
            step["expect"] = "\n".join(
                tag[7:].strip() for tag in tags if tag.startswith("Expect:")
            )
            step["result"] = "\n".join(
                tag[7:].strip() for tag in tags if tag.startswith("Result:")
            )
            if (
                kinds != ["Do", "Expect", "Result"]
                or step["result"] != "PASS | FAIL"
                or not step["do"]
                or not step["expect"]
            ):
                bad.append(step["id"])
        out(
            numbers == expected and not bad and bool(steps),
            "AC1 consecutive steps and fields",
            "ids=%s, malformed=%s" % (numbers, bad),
        )
        nonblank = [line for line in step_lines if line.strip()]
        out(
            0 < len(steps) <= 40 and len(nonblank) <= 160,
            "AC1 40 step and 160 line envelope",
            "%d steps, %d nonblank lines" % (len(steps), len(nonblank)),
        )
        out(
            bool(re.search(r"\bowner\b", pre_steps, re.I))
            and bool(re.search(r"real Windows", pre_steps, re.I)),
            "AC1 owner-run live checklist",
            "introduction must assign the live run to the owner on real Windows",
        )

step_text = "\n".join(step_lines)
if cl is not None:
    stale = re.compile(
        r"(?:drop|tray|shortcut|icon|notifications?|toast)"
        r".{0,100}(?:not built|not in this build|absent|not available|skipped)"
        r"|(?:not built|not in this build|absent|not available|skipped)"
        r".{0,100}(?:drop|tray|shortcut|icon|notifications?|toast)",
        re.I | re.S,
    )
    out(
        stale.search(cl) is None and re.search(r"\bSKIPPED\b", step_text, re.I) is None,
        "AC2 stale and skipped shipped features",
        "remove stale absence claims and every SKIPPED step",
    )
    vague = []
    for step in steps:
        if re.search(
            r"\b(?:nice|fast|as expected|works correctly|correctly|successfully)\b",
            step.get("expect", ""),
            re.I,
        ):
            vague.append(step["id"])
    out(not vague, "AC2 observable expectations", "vague Expect in %s" % vague)
    forbidden_scope = re.findall(
        r"IExplorerCommand|sparse package|public(?:ly)?\s+(?:code\s+)?sign"
        r"|modern\s+(?:first.level\s+)?(?:Windows\s+11\s+)?(?:Explorer\s+)?menu",
        step_text,
        re.I,
    )
    out(
        not forbidden_scope,
        "AC2 preserve signing and menu scope",
        "out-of-scope live steps: %s" % forbidden_scope,
    )


# Each row maps one acceptance-criterion item to a unique step title. Every
# pattern is checked only in that step's Do or Expect field, never globally.
COVERAGE = [
    # AC3: installation, Explorer entry, hive, conversion and language.
    ("AC3", "zip and MSI checksums", r"^Install packages$",
     [r"SHA256SUMS", r"certutil|Get-FileHash", r"\.zip", r"\.msi"],
     [r"checksum.*match|hash.*match"]),
    ("AC3", "unsigned SmartScreen and per-user MSI", r"^Install packages$",
     [r"SmartScreen", r"More info", r"Run anyway"],
     [r"unsigned", r"per.user", r"without elevat|no (?:administrator|admin|elevation)"]),
    ("AC3", "Start menu and icon", r"^Install packages$",
     [r"Start menu"], [r"Kawaro", r"icon"]),
    # S-085 AC2, D8: W01 is extended in place; the owner sees the icon and the
    # fixed bilingual name (Latin part first) in Start.
    ("S-085 AC2", "Start icon and bilingual name", r"^Install packages$",
     [r"Start menu"], [r"Kawaro \| كوارو", r"icon"]),
    ("AC3", "portable integration install and uninstall", r"^Portable integration$",
     [r"integration install", r"integration uninstall", r"\.zip|portable"],
     [r"Explorer|context menu", r"removed|gone"]),
    ("AC3", "missing-file refusal", r"^Portable integration faults",
     [r"missing.file|missing .*kawaro.*\.exe", r"integration install"],
     [r"refus|nonzero|non.zero", r"missing"]),
    ("AC3", "stop at first registry failure", r"^Portable integration faults",
     [r"registry|reg(?:\.exe)?", r"first"],
     [r"stop|abort", r"first", r"no (?:further|later|subsequent)"]),
    ("AC3", "one and ten selections", r"^Explorer selections",
     [r"\bone\b", r"\bten\b|\b10\b", r"Show more options"],
     [r"one hive", r"ten|\b10\b"]),
    ("AC3", "pointer and screen-edge placement", r"^Hive placement and appearance",
     [r"pointer", r"screen edge"], [r"pointer", r"fully on screen|inside the screen"]),
    ("AC3", "transparent corners and no chrome", r"^Hive placement and appearance",
     [r"corners|border|taskbar"],
     [r"transparent corners", r"no shadow", r"no frame", r"no taskbar button"]),
    ("AC3", "150 percent scaling", r"^Hive placement and appearance",
     [r"150%"], [r"sharp|not blurry", r"inside the screen|fully on screen"]),
    ("AC3", "focus Esc and Narrator", r"^Hive focus and accessibility",
     [r"focus|letter key", r"\bEsc\b", r"Narrator"],
     [r"focus", r"Esc closes", r"Narrator.*label|label.*Narrator"]),
    ("AC3", "three trial timing budget", r"^Hive timing",
     [r"60\s*fps", r"(?:three|3) trials"],
     [r"150\s*ms", r"(?:9|nine) frames or fewer", r"all (?:three|3)"]),
    ("AC3", "conversion preserves original", r"^Convert and preserve",
     [r"hash|size and date", r"convert"],
     [r"new .*next to the original|new .*beside the original", r"original.*unchanged"]),
    ("AC3", "Arabic RTL and filenames", r"^Arabic",
     [r"language\s*=\s*[\"']ar[\"']", r"Arabic.*(?:file|name)|(?:file|name).*Arabic"],
     [r"Arabic labels", r"right to left|\bRTL\b", r"Arabic name.*intact|name.*unchanged"]),

    # AC4: named-pipe security, deadlines and daemon start.
    ("AC4", "second account pipe denial", r"^Pipe second account",
     [r"second (?:Windows )?(?:user|account)", r"runas|Sign in", r"pipe"],
     [r"ERROR_ACCESS_DENIED", r"request.*not|no request"]),
    ("AC4", "remote client rejection", r"^Pipe remote client",
     [r"remote", r"\\\\[^\\\s]+\\pipe\\"],
     [r"remote.*reject|reject.*remote", r"ERROR_ACCESS_DENIED|access denied"]),
    ("AC4", "foreign owner rejected before bytes", r"^Pipe foreign owner",
     [r"foreign|other (?:user|account)", r"owner", r"request bytes"],
     [r"before.*request bytes|request bytes.*before", r"reject|denied"]),
    ("AC4", "instance storm leaves one server", r"^Daemon instance storm",
     [r"second daemon|second .*kawarod", r"ten concurrent|\b10 concurrent"],
     [r"one (?:named.pipe )?server|one kawarod"]),
    ("AC4", "client ten second timeout", r"^Pipe timeout and cancellation",
     [r"10\s*s|10.second", r"unreachable|hung|does not respond"],
     [r"10\s*s|10.second", r"timeout|timed out", r"responsive|returns|exits"]),
    ("AC4", "daemon five hundred ms check", r"^Daemon start$",
     [r"500\s*ms", r"kawaro open"], [r"500\s*ms", r"daemon|kawarod"]),
    ("AC4", "CancelIoEx does not hang", r"^Pipe timeout and cancellation",
     [r"CancelIoEx", r"pending|blocked"],
     [r"returns|completes", r"no hung|not hung|responsive", r"client", r"daemon"]),
    ("AC4", "open survives terminal close", r"^Daemon start$",
     [r"kawaro open", r"close.*terminal|terminal.*close"],
     [r"survives|still running|remains running", r"kawarod|daemon"]),
    ("AC4", "retry without job breakaway", r"^Daemon start$",
     [r"job", r"breakaway.*forbid|forbid.*breakaway|without.*BREAKAWAY", r"retry"],
     [r"one daemon|one kawarod", r"retry.*succeed|starts|connects"]),

    # AC5: paths, known-folder fallback, exFAT and doctor.
    ("AC5", "atomic settings save", r"^Settings and action paths",
     [r"%APPDATA%\\Kawaro", r"save|write", r"atomic|temporary|temp"],
     [r"atomic|rename|replace", r"config(?:uration)?.*valid|valid.*config", r"no.*temp"]),
    ("AC5", "logs and user action discovery", r"^Settings and action paths",
     [r"%LOCALAPPDATA%\\Kawaro\\logs", r"user action|actions[\\/]", r"restart|reload"],
     [r"kawarod\.log|log", r"custom action|user action", r"appears|listed"]),
    ("AC5", "empty and missing environment fallback", r"^Known folder fallback",
     [r"APPDATA", r"LOCALAPPDATA", r"empty", r"missing|unset|remove", r"known folder"],
     [r"actual known folder|known.folder path", r"APPDATA", r"LOCALAPPDATA"]),
    ("AC5", "exFAT occupied output names", r"^exFAT collision",
     [r"exFAT", r"occupied|already exist", r"hash|size and date"],
     [r"next free name", r"no replacement|does not replace|not replaced", r"unchanged"]),
    ("AC5", "doctor endpoint and paths", r"^Doctor paths",
     [r"kawaro doctor"], [r"endpoint|named pipe", r"config", r"logs?", r"actions?"]),

    # AC6: discovery, waiting, complete process-tree cleanup and consoles.
    ("AC6", "real engine discovery", r"^Engine discovery",
     [r"LibreOffice", r"Ghostscript", r"FFmpeg", r"registry", r"bundled"],
     [r"discover|path|found", r"LibreOffice", r"Ghostscript", r"FFmpeg"]),
    ("AC6", "missing engine messages", r"^Engine discovery",
     [r"missing|uninstall|rename"], [r"names? (?:LibreOffice|Ghostscript|FFmpeg)|missing.*(?:engine|LibreOffice|Ghostscript|FFmpeg)"]),
    ("AC6", "drive path with spaces and Arabic", r"^Engine path and wait",
     [r"[A-Z]:\\", r"spaces?", r"Arabic"],
     [r"result|output", r"Arabic.*intact|path.*Arabic|Arabic.*path"]),
    ("AC6", "soffice.com waits", r"^Engine path and wait",
     [r"soffice\.com", r"wait|completion"], [r"waits?|does not return", r"complete|finished"]),
    ("AC6", "all exit paths reap process tree", r"^Engine process tree and consoles",
     [r"normal exit", r"cancel", r"timeout", r"daemon termination", r"grandchild|process tree"],
     [r"no .*child|no .*process|none.*remain", r"soffice\.exe", r"soffice\.bin", r"FFmpeg", r"ffprobe", r"Ghostscript"]),
    ("AC6", "no console windows or flashes", r"^Engine process tree and consoles",
     [r"watch|record|observe", r"daemon|kawarod", r"FFmpeg", r"ffprobe", r"Ghostscript"],
     [r"no .*console.*(?:window|flash)|console.*(?:never|does not)", r"daemon|kawarod", r"FFmpeg", r"ffprobe", r"Ghostscript"]),

    # AC7: shipped drop, shortcut, tray, notifications and Open folder.
    ("AC7", "Explorer drag highlight and safe drop", r"^Explorer drop$",
     [r"drag", r"highlight", r"drop", r"hash|unchanged"],
     [r"highlight", r"original.*unchanged", r"new result|result.*next to"]),
    ("AC7", "drag cancel and fresh retry", r"^Explorer drop$",
     [r"cancel", r"close", r"new drag|drag again"],
     [r"no stale|fresh|current files", r"no stale|only.*new|only.*current"]),
    ("AC7", "shortcut threshold timing and release", r"^Shortcut$",
     [r"hold_ms", r"release", r"60\s*fps|record"],
     [r"hold_ms.*(?:elapsed|threshold)|(?:elapsed|threshold).*hold_ms", r"150\s*ms", r"(?:9|nine) frames", r"release"]),
    ("AC7", "OLE drag and swapped buttons", r"^Shortcut$",
     [r"OLE", r"swapped primary|swap.*primary"],
     [r"OLE", r"swapped primary|swap.*primary", r"hive|drag"]),
    ("AC7", "taken shortcut stays alive", r"^Shortcut$",
     [r"taken|already.*registered|conflict", r"shortcut"],
     [r"shortcut <string> unavailable", r"daemon.*(?:alive|responsive|running)"]),
    ("AC7", "tray job and download progress", r"^Tray progress and resources",
     [r"tray", r"job progress|conversion progress", r"download progress"],
     [r"job progress|conversion progress", r"download progress", r"tooltip|menu|tray"]),
    ("AC7", "tray bilingual menus and outside click", r"^Tray menus and actions",
     [r"English", r"Arabic", r"outside click|click outside"],
     [r"Arabic.*(?:RTL|right to left)|(?:RTL|right to left).*Arabic", r"outside.*clos|clos.*outside"]),
    ("AC7", "tray actions and clean quit", r"^Tray menus and actions",
     [r"Cancel all", r"Settings", r"Quit"],
     [r"Cancel all.*cancel|jobs?.*cancel", r"Settings.*open", r"no ghost|ghost.*no|icon.*gone"]),
    ("AC7", "Explorer restart restores tray", r"^Tray lifecycle",
     [r"restart Explorer|Explorer restart"], [r"one tray icon|tray icon.*return|tray.*restored"]),
    ("AC7", "flat GDI handles", r"^Tray progress and resources",
     [r"GDI handles?", r"repeated|many|100|1000", r"progress updates"],
     [r"flat|does not grow|no growth|same count"]),
    ("AC7", "exactly one toast without focus theft", r"^Job notifications",
     [r"finish|complete", r"focus", r"count|notification|toast"],
     [r"exactly one Kawaro toast|one Kawaro toast", r"focus.*(?:unchanged|remains|does not)|(?:does not|no).*focus"]),
    ("AC7", "Open folder selects spaced result", r"^Open folder$",
     [r"Open folder", r"path.*spaces|spaces.*path"],
     [r"selects?.*result|result.*selected", r"Explorer"]),
    ("AC7", "Open folder spawn failure and child reaping", r"^Open folder$",
     [r"spawn.*fail|fail.*spawn|invalid.*Explorer", r"Open folder"],
     [r"log.*spawn.*fail|spawn.*fail.*log", r"reap|no .*child|child.*gone"]),

    # AC8: real updater, deferral, apply evidence, failures and responsiveness.
    ("AC8", "signed feed manual check", r"^Updater manual check",
     [r"real signed beta feed|signed beta feed", r"maintainer.supplied newer MSI", r"update --check"],
     [r"newer|available", r"signed|signature"]),
    ("AC8", "download and verify", r"^Updater manual check",
     [r"download", r"verify|signature"], [r"download", r"verif|signature.*valid"]),
    ("AC8", "status offline without network", r"^Updater status and configuration",
     [r"update --status", r"offline|network.*(?:disabled|blocked)|without network"],
     [r"no network|does not.*network|without network", r"cached|last.*(?:status|result)"]),
    ("AC8", "manual check with automatic checks off", r"^Updater status and configuration",
     [r"update_check\s*=\s*false", r"update --check|manual check"],
     [r"manual.*(?:works|runs|checks)|check.*(?:available|completes)", r"no automatic|automatic.*not"]),
    ("AC8", "enabled tray Update now", r"^Updater tray and deferral",
     [r"update_check\s*=\s*true|enable", r"Update now"],
     [r"Update now.*enabled|enabled.*Update now", r"check|download|update"]),
    ("AC8", "busy job and open hive defer", r"^Updater tray and deferral",
     [r"busy job|job.*running", r"open hive|hive.*open"],
     [r"defer|postpone|wait", r"job", r"hive"]),
    ("AC8", "ask and idle automatic modes", r"^Updater apply modes",
     [r"ask.mode|ask mode", r"idle.*auto|auto.*idle"],
     [r"ask.*(?:prompt|confirm)|prompt.*ask", r"ask.*apply|apply.*ask", r"idle.*apply|apply.*idle"]),
    ("AC8", "detached apply helper evidence", r"^Updater live apply",
     [r"copied helper|detached helper", r"daemon quit|quit.*daemon", r"maintainer.supplied newer MSI"],
     [r"helper.*surviv|surviv.*helper", r"msiexec\.exe.*SystemRoot|SystemRoot.*msiexec\.exe", r"per.user", r"restart.*kawarod\.exe|kawarod\.exe.*restart"]),
    ("AC8", "apply logs result cleanup and notification", r"^Updater live apply",
     [r"log|one.shot|temporary"],
     [r"log", r"one.shot result", r"clean.*temp|temp.*clean|no.*temp", r"one update notification|exactly one.*notification"]),
    ("AC8", "MSI exit code mappings", r"^Updater MSI exit codes",
     [r"\b0\b", r"3010", r"1602", r"1618", r"1603"],
     [r"0.*success|success.*\b0\b", r"3010.*(?:restart|reboot)|(?:restart|reboot).*3010", r"1602.*cancel|cancel.*1602", r"1618.*(?:another|busy|retry)|(?:another|busy|retry).*1618", r"1603.*fail|fail.*1603"]),
    ("AC8", "copied helper block is reported", r"^Updater failure handling",
     [r"antivirus|SmartScreen", r"copied helper", r"block"],
     [r"antivirus|SmartScreen", r"block", r"log|error|message", r"responsive|no hung|not hung"]),
    ("AC8", "offline and hung checks stay responsive", r"^Updater failure handling",
     [r"offline", r"hung|hang|does not respond", r"update --check|check"],
     [r"timeout|timed out|offline", r"daemon.*responsive|responsive.*daemon"]),

    # AC9: native ISO, exact-commit CI and MSI lifecycle.
    ("AC9", "read-only native ISO runner", r"^Native ISO runner",
     [r"S-044.*ISO|ISO.*S-044", r"read.only", r"run-tests\.cmd", r"from (?:the )?(?:disc|ISO)"],
     [r"one result per test exe|each test exe.*result", r"total N failed 0", r"logs?.*outside.*(?:disc|ISO)|outside.*(?:disc|ISO).*logs?"]),
    ("AC9", "exact commit Windows CI", r"^Release readiness",
     [r"exact.*commit|tested commit|commit.*SHA", r"windows-cross", r"windows-native"],
     [r"windows-cross.*success|success.*windows-cross", r"windows-native.*success|success.*windows-native", r"exact.*commit|same commit|tested commit"]),
    ("AC9", "MSI upgrade downgrade logon and uninstall", r"^MSI lifecycle",
     [r"upgrade", r"downgrade", r"next logon|sign out.*sign in|log off.*log on", r"uninstall", r"Run value"],
     [r"upgrade", r"downgrade.*refus|refus.*downgrade", r"next logon|after.*logon", r"only Kawaro", r"files.*keys|keys.*files", r"Run value.*remov|remov.*Run value", r"converted files.*preserv|preserv.*converted files", r"other Run values?.*preserv|preserv.*other Run values?"]),
]

for ac, item, title_pattern, do_patterns, expect_patterns in COVERAGE:
    name = "%s %s" % (ac, item)
    matches = [step for step in steps if re.search(title_pattern, step["title"], re.I)]
    if len(matches) != 1:
        out(False, name, "need exactly one step title matching %r, got %s" % (
            title_pattern, [step["id"] + " " + step["title"] for step in matches]
        ))
        continue
    step = matches[0]
    missing_do = [pattern for pattern in do_patterns if not re.search(pattern, step.get("do", ""), re.I)]
    missing_expect = [pattern for pattern in expect_patterns if not re.search(pattern, step.get("expect", ""), re.I)]
    why = []
    if missing_do:
        why.append("%s Do missing %s" % (step["id"], missing_do))
    if missing_expect:
        why.append("%s Expect missing %s" % (step["id"], missing_expect))
    out(not why, name, "; ".join(why))


# AC10: retain the useful S-052 smoke, log, report, privacy and format checks.
def crlf_ok(text):
    if not text:
        return False
    parts = text.split("\n")
    body = parts[:-1] if parts[-1] == "" else parts
    return bool(body) and all(part.endswith("\r") for part in body)


NET = re.compile(
    r"\b(?:curl|wget|bitsadmin|ftp|scp|certutil|powershell|pwsh|Invoke-\w+)\b"
    r"|https?:|\\\\[A-Za-z]|net use",
    re.I,
)
smoke = read(SMOKE)
if smoke is None:
    for name in (
        "AC10 smoke CRLF", "AC10 smoke commands", "AC10 smoke order",
        "AC10 smoke error handling", "AC10 smoke write scope", "AC10 smoke no network",
    ):
        out(False, name, SMOKE + " missing")
else:
    out(crlf_ok(smoke), "AC10 smoke CRLF", "every line must end CRLF")
    needed = [
        "--version", "integration install --dry-run", "kawarod.exe", "ping",
        "smoke.txt", r"%USERPROFILE%\kawaro-beta-test", "total", "failed",
    ]
    missing = [item for item in needed if item.lower() not in smoke.lower()]
    out(not missing, "AC10 smoke commands", "missing %s" % missing)
    positions = [smoke.lower().find(item) for item in ("--version", "--dry-run", "kawarod.exe", " ping")]
    out(
        all(position >= 0 for position in positions) and positions == sorted(positions),
        "AC10 smoke order",
        str(positions),
    )
    out(
        len(re.findall(r"errorlevel", smoke, re.I)) >= 4,
        "AC10 smoke error handling",
        "fewer than four errorlevel uses",
    )
    assignments = {}
    for match in re.finditer(r'(?im)^\s*set\s+"?(\w+)=([^"\r\n]*)', smoke):
        assignments[match.group(1).upper()] = match.group(2)
    bad_targets = []
    for line in smoke.splitlines():
        if re.match(r"(?i)^\s*(?:rem|::)", line):
            continue
        for match in re.finditer(r'(?<![0-9])>>?\s*("[^"]+"|[^\s&|)>]+)', line):
            target = match.group(1).strip('"')
            if target.startswith("&") or target.lower() == "nul":
                continue
            variable = re.fullmatch(r"%(\w+)%.*", target)
            allowed = "kawaro-beta-test" in target.lower() or (
                variable is not None
                and "kawaro-beta-test" in assignments.get(variable.group(1).upper(), "").lower()
            )
            if not allowed:
                bad_targets.append(target)
    outside = re.findall(
        r"(?i)%(?:temp|tmp|appdata|localappdata|windir|systemroot)%|\bC:\\", smoke
    )
    out(
        not bad_targets and not outside,
        "AC10 smoke write scope",
        "targets=%s, outside=%s" % (bad_targets, outside),
    )
    out(NET.search(smoke) is None, "AC10 smoke no network", "network command or URL found")

logs = read(LOGS)
if logs is None:
    for name in (
        "AC10 logs CRLF", "AC10 log sources and target", "AC10 log folder output", "AC10 logs no upload",
    ):
        out(False, name, LOGS + " missing")
else:
    out(crlf_ok(logs), "AC10 logs CRLF", "every line must end CRLF")
    needed = [
        r"%LOCALAPPDATA%\Kawaro\logs\kawarod.log",
        r"%APPDATA%\Kawaro\config.toml",
        r"%USERPROFILE%\kawaro-beta-test\report",
    ]
    missing = [item for item in needed if item.lower() not in logs.lower()]
    out(not missing, "AC10 log sources and target", "missing %s" % missing)
    out(
        bool(re.search(r"(?im)^\s*echo\b.*report", logs))
        or bool(re.search(r"(?im)^\s*echo\b.*%\w+%", logs)),
        "AC10 log folder output",
        "no echo of the report folder",
    )
    out(NET.search(logs) is None, "AC10 logs no upload", "network command or URL found")

if cl is None:
    out(False, "AC10 report contents and privacy", "checklist missing")
else:
    headings = [line for line in cl.splitlines() if line.startswith("## ")]
    out(
        bool(headings)
        and re.match(r"^## +Before you start\b", headings[0]) is not None
        and re.match(r"^## +Report\b", headings[-1]) is not None,
        "AC10 first and last sections",
        "first/last headings=%s" % ([headings[0], headings[-1]] if headings else []),
    )
    report_text = section(cl, r"Report") or ""
    if steps:
        range_pattern = r"%s\s*(?:to|through|-)\s*%s" % (steps[0]["id"], steps[-1]["id"])
    else:
        range_pattern = r"(?!)"
    report_requirements = [
        ("full resulting ID range", range_pattern),
        ("PASS or FAIL list", r"PASS\s*(?:or|\|)\s*FAIL"),
        ("smoke.txt", r"smoke\.txt"),
        ("daemon log", r"kawarod\.log"),
        ("config", r"config\.toml"),
        ("hive screenshots", r"screenshot.*hive|hive.*screenshot"),
        ("privacy review", r"personal paths|user names|machine names"),
        ("no extra files", r"no other files|nothing else"),
    ]
    for label, pattern in report_requirements:
        out(
            re.search(pattern, report_text, re.I) is not None,
            "AC10 report " + label,
            "missing from Report",
        )

long_dash = re.compile("[\u2013\u2014]")
for name, text in (("checklist", cl), ("beta-smoke.cmd", smoke), ("collect-logs.cmd", logs)):
    if text is None:
        out(False, "AC10 clean " + name, "missing")
        continue
    reasons = []
    if "/home/" in text:
        reasons.append("/home/")
    if re.search(r"C:\\Users\\(?![<%])[A-Za-z0-9_.-]+", text, re.I):
        reasons.append("personal C:\\Users path")
    if long_dash.search(text):
        reasons.append("long dash")
    out(not reasons, "AC10 clean " + name, ", ".join(reasons))
PY

while IFS= read -r line; do
  case "$line" in
    "ok "*) pass ;;
    "FAIL "*) fail "${line#FAIL }" ;;
    *) fail "checker output: $line" ;;
  esac
done < <(python3 -I "$T/check.py" "$CL" "$SMOKE" "$LOGS" 2>&1)

for file in "$SMOKE" "$LOGS"; do
  if [ -f "$file" ]; then
    case "$(file -b "$file")" in
      *"CRLF line terminators"*) pass ;;
      *) fail "AC10 file says no CRLF for $file" ;;
    esac
  else
    fail "AC10 $file missing"
  fi
done

if command -v shellcheck > /dev/null 2>&1; then
  check "AC10 shellcheck clean" shellcheck -x "$0"
else
  fail "AC10 shellcheck not installed"
fi
check "AC10 design-lint clean" bash scripts/design-lint.sh > /dev/null 2>&1

echo "test_checklist: $CHECKS checks, $FAILS failed"
[ "$FAILS" -eq 0 ]
