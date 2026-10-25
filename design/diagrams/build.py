#!/usr/bin/env python3
"""Draw the Kawaro architecture diagrams as SVG, from design/tokens.json.

Standard library only. Every colour comes from the tokens (light and dark
schemes are two files), no shadows, no gradients. Each diagram is written in
English and Arabic (mirrored for right to left), so the README and the docs
can show the same picture in both languages.

Output: docs/diagrams/NAME.LANG.SCHEME.svg  (LANG en or ar, SCHEME light or dark)
Run: python3 design/diagrams/build.py
"""

import html
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "docs" / "diagrams"
TOK = json.loads((ROOT / "design" / "tokens.json").read_text(encoding="utf-8"))
SQ3 = math.sqrt(3)
FONT = '"IBM Plex Sans Arabic", "IBM Plex Sans", system-ui, sans-serif'
MONO = "ui-monospace, 'IBM Plex Mono', monospace"


class Canvas:
    def __init__(self, w, h, rtl):
        self.w, self.h, self.rtl = w, h, rtl
        self.items = []

    def x(self, x, width=0):
        """Logical (left to right) x to drawn x; boxes pass their width."""
        return self.w - x - width if self.rtl else x

    def add(self, s):
        self.items.append(s)

    def box(self, x, y, w, h, lines, cls="box", mono=False, size=15, sub=None):
        bx = self.x(x, w)
        self.add(f'<rect class="{cls}" x="{bx:.1f}" y="{y}" width="{w}" height="{h}" rx="12"/>')
        n = len(lines)
        total = n * (size + 5)
        cy = y + h / 2 - total / 2 + size
        for i, ln in enumerate(lines):
            fam = ' font-family="' + MONO + '"' if (mono and i == 0) else ""
            weight = ' font-weight="600"' if i == 0 and not mono else ""
            self.add(f'<text class="t {cls}-t" x="{bx + w / 2:.1f}" y="{cy + i * (size + 5):.1f}" '
                     f'text-anchor="middle" font-size="{size}"{weight}{fam}>{html.escape(ln)}</text>')

    def label(self, x, y, text, anchor="middle", cls="t2", size=13, mono=False):
        ax = self.x(x)
        fam = ' font-family="' + MONO + '"' if mono else ""
        self.add(f'<text class="{cls}" x="{ax:.1f}" y="{y}" text-anchor="{anchor}" font-size="{size}"{fam}>{html.escape(text)}</text>')

    def arrow(self, x1, y1, x2, y2, dashed=False):
        d = ' stroke-dasharray="6 6"' if dashed else ""
        self.add(f'<line class="ln" x1="{self.x(x1):.1f}" y1="{y1}" x2="{self.x(x2):.1f}" y2="{y2}" marker-end="url(#a)"{d}/>')

    def hexagon(self, cx, cy, r, cls="hex"):
        pts = [(self.x(cx) + r * math.cos(math.radians(a)), cy + r * math.sin(math.radians(a))) for a in (-90, -30, 30, 90, 150, 210)]
        self.add(f'<path class="{cls}" d="M' + "L".join(f"{px:.1f},{py:.1f}" for px, py in pts) + 'Z"/>')

    def svg(self, scheme, title, desc):
        c = TOK["color"][scheme]
        css = f"""
.box{{fill:{c['surface-1']};stroke:{c['border']};stroke-width:1}}
.box2{{fill:{c['surface-2']};stroke:{c['border']};stroke-width:1}}
.zone{{fill:{c['surface-0']};stroke:{c['border']};stroke-width:1;stroke-dasharray:6 6}}
.hot{{fill:{c['accent']};stroke:{c['accent']}}}
.hex{{fill:{c['surface-2']};stroke:{c['border']};stroke-width:1}}
.hexhot{{fill:{c['accent']};stroke:{c['accent']}}}
.hexc{{fill:{c['surface-3']};stroke:{c['border']};stroke-width:1}}
.t{{fill:{c['text-primary']}}} .t2{{fill:{c['text-secondary']}}}
.hot-t{{fill:{c['accent-ink']}}} .hexhot-t{{fill:{c['accent-ink']}}}
.ln{{stroke:{c['text-secondary']};stroke-width:2;fill:none}}
.head{{fill:{c['text-secondary']}}}
text{{font-family:{FONT}{';direction:rtl;unicode-bidi:embed' if self.rtl else ''}}}
"""
        return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {self.w} {self.h}" role="img" '
                f'aria-labelledby="t d" width="{self.w}" height="{self.h}">'
                f'<title id="t">{html.escape(title)}</title><desc id="d">{html.escape(desc)}</desc>'
                f'<style>{css}</style><rect width="{self.w}" height="{self.h}" fill="{c["surface-0"]}" rx="16"/>'
                f'<defs><marker id="a" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="8" markerHeight="8" orient="auto-start-reverse">'
                f'<path d="M0 0L10 5L0 10Z" fill="{c["text-secondary"]}"/></marker></defs>'
                + "".join(self.items) + "</svg>\n")


def T(lang, en, ar):
    return ar if lang == "ar" else en


# ---------------------------------------------------------------------------
# 1. Overview: processes and crates
# ---------------------------------------------------------------------------

def overview(lang):
    c = Canvas(960, 640, lang == "ar")
    t = lambda en, ar: T(lang, en, ar)
    c.label(30, 34, t("Who talks to whom", "من يكلّم من"), "start", "head", 15)
    c.box(30, 56, 190, 74, [t("File manager", "مدير الملفات"), "Dolphin Nautilus", "Nemo Thunar Explorer"], size=14)
    c.box(300, 56, 190, 74, ["kawaro open", t("CLI, starts the daemon", "سطر الأوامر، يشغّل الخدمة")], mono=True, size=14)
    c.box(570, 56, 190, 74, [t("IPC", "قناة الاتصال"), t("Unix socket", "مقبس يونكس"), t("named pipe on Windows", "وفي ويندوز: أنبوب مسمّى")], size=14)
    c.arrow(220, 93, 300, 93)
    c.arrow(490, 93, 570, 93)
    # daemon zone
    c.add(f'<rect class="zone" x="{c.x(30, 900):.1f}" y="170" width="900" height="300" rx="16"/>')
    c.label(50, 198, "kawarod  (crates/daemon)  " + t("the only process with UI", "العملية الوحيدة التي تعرض واجهة"), "start", "head", 15)
    c.box(60, 220, 250, 84, [t("Hive", "الخلية"), t("Slint, software renderer", "Slint بمصيّر برمجي"), "crates/ui"], size=14)
    c.box(350, 220, 250, 84, [t("Queue and events", "الطابور والأحداث"), t("cancel, progress, atomic rename", "إلغاء وتقدّم وتسمية ذرية"), "crates/core"], size=14)
    c.box(640, 220, 250, 84, [t("Tray and notifications", "الصينية والإشعارات"), t("progress in the tooltip", "التقدم في التلميح"), "crates/daemon"], size=14)
    c.box(60, 350, 250, 84, [t("Action registry", "سجل الإجراءات"), "actions/*.toml", t("what fits this selection", "ما يناسب هذا التحديد")], size=14)
    c.box(350, 350, 250, 84, [t("Engines", "المحركات"), t("one adapter per engine", "محوّل لكل محرك"), "crates/engines"], size=14)
    c.box(640, 350, 250, 84, [t("Design tokens", "رموز التصميم"), "design/tokens.json", t("generated into the UI", "تُولَّد في الواجهة")], cls="box2", size=14)
    c.arrow(665, 130, 665, 170)
    c.arrow(185, 350, 185, 304)
    c.arrow(310, 262, 350, 262)
    c.arrow(475, 304, 475, 350)
    c.arrow(600, 262, 640, 262, dashed=True)
    c.arrow(765, 350, 765, 304, dashed=True)
    # external tools
    c.label(30, 626, t("Heavy engines run as separate processes", "المحركات الثقيلة تعمل كعمليات منفصلة"), "start", "head", 14)
    for i, (name, note) in enumerate([("FFmpeg", t("video, audio", "فيديو وصوت")), ("LibreOffice", t("documents", "مستندات")), ("Ghostscript", "PDF")]):
        c.box(60 + i * 290, 524, 250, 74, [name, note], cls="box2", size=15)
        c.arrow(475, 434, 185 + i * 290, 524, dashed=True)
    c.label(930, 626, t("Cancel kills every child and its whole tree", "الإلغاء يقتل كل ابن مع شجرته كاملة"), "end", "t2", 12)
    return lambda s: c.svg(s, t("Kawaro processes", "عمليات كوارو"),
                 t("A file manager entry calls kawaro open, which sends a request over a local socket to the daemon. The daemon holds the hive, the queue, the tray, the action registry and the engines. Heavy tools run as separate child processes.",
                   "إدخال مدير الملفات يستدعي kawaro open فيرسل طلبا عبر مقبس محلي إلى الخدمة. تضم الخدمة الخلية والطابور والصينية وسجل الإجراءات والمحركات، وتعمل الأدوات الثقيلة كعمليات ابنة منفصلة."))


# ---------------------------------------------------------------------------
# 2. Request flow: seven steps
# ---------------------------------------------------------------------------

def flow(lang):
    c = Canvas(960, 560, lang == "ar")
    t = lambda en, ar: T(lang, en, ar)
    steps = [
        (t("Select files", "حدّد الملفات"), t("in the file manager", "في مدير الملفات")),
        (t("Right click", "انقر يمين"), "kawaro open"),
        (t("Filter actions", "تصفية الإجراءات"), t("kind, count, tools found", "النوع والعدد والأدوات")),
        (t("Show the hive", "اعرض الخلية"), t("only cells that fit", "ما يناسب فقط")),
        (t("Click a cell", "انقر خلية"), t("a job joins the queue", "تدخل مهمة الطابور")),
        (t("Engine runs", "يعمل المحرك"), t("writes a temp file", "يكتب ملفا مؤقتا")),
        (t("Result beside the original", "الناتج بجانب الأصل"), t("atomic rename, tray note", "تسمية ذرية وإشعار")),
    ]
    pos = [(110, 130), (330, 130), (550, 130), (770, 130), (220, 370), (440, 370), (660, 370)]
    for i, ((a, b), (x, y)) in enumerate(zip(steps, pos)):
        hot = i in (3, 6)
        c.hexagon(x, y, 62, "hexhot" if hot else "hex")
        c.add(f'<text class="{"hexhot-t" if hot else "t"}" x="{c.x(x):.1f}" y="{y + 11}" text-anchor="middle" font-size="32" font-weight="600">{i + 1}</text>')
        c.label(x, y + 92, a, "middle", "t", 16)
        c.label(x, y + 114, b, "middle", "t2", 13, mono=(i == 1))
    for i in range(3):
        c.arrow(pos[i][0] + 70, 130, pos[i + 1][0] - 70, 130)
    c.add(f'<line class="ln" x1="{c.x(770):.1f}" y1="262" x2="{c.x(770):.1f}" y2="282"/>')
    c.add(f'<line class="ln" x1="{c.x(770):.1f}" y1="282" x2="{c.x(220):.1f}" y2="282"/>')
    c.arrow(220, 282, 220, 304)
    for i in range(4, 6):
        c.arrow(pos[i][0] + 70, 370, pos[i + 1][0] - 70, 370)
    c.label(480, 526, t("The original file is never modified.", "الملف الأصلي لا يتغير أبدا."), "middle", "t", 16)
    return lambda s: c.svg(s, t("From right click to result", "من النقر الأيمن إلى الناتج"),
                 t("Seven steps: select files, right click, filter actions, show the hive, click a cell, the engine runs, the result lands beside the original.",
                   "سبع خطوات: حدّد الملفات، انقر يمين، تُصفّى الإجراءات، تظهر الخلية، انقر خلية، يعمل المحرك، ويظهر الناتج بجانب الأصل."))


# ---------------------------------------------------------------------------
# 3. Adding a format: TOML plus adapter
# ---------------------------------------------------------------------------

def extend(lang):
    c = Canvas(960, 560, lang == "ar")
    t = lambda en, ar: T(lang, en, ar)
    c.label(30, 34, t("A new format is a TOML file and a small adapter", "صيغة جديدة تعني ملف TOML ومحوّلا صغيرا"), "start", "head", 15)
    c.box(30, 60, 260, 150, ["actions/pdf.toml", 'id = "pdf.merge"', 'accepts = ["pdf"]', "min_files = 2", 'label = { ar, en }'], mono=True, size=14)
    c.box(350, 60, 260, 150, ["trait Engine", "capabilities()", "estimate(file, device)", "run(job, progress, cancel)"], mono=True, size=14)
    c.box(670, 60, 260, 150, [t("Registry", "السجل"), t("loads built in actions", "يحمّل الإجراءات المضمّنة"), t("and the user's own", "وإجراءات المستخدم")], size=15)
    c.arrow(290, 135, 350, 135)
    c.arrow(610, 135, 670, 135)
    c.hexagon(800, 320, 56, "hexhot")
    c.add(f'<text class="hexhot-t" x="{c.x(800):.1f}" y="{325}" text-anchor="middle" font-size="18" font-weight="600">{t("MERGE", "دمج")}</text>')
    c.arrow(800, 210, 800, 262)
    c.label(800, 400, t("a cell in the hive", "خلية في الخلية"), "middle", "t", 15)
    c.label(30, 300, t("Engines in this build", "المحركات في هذه النسخة"), "start", "head", 15)
    rows = [("image", t("in process, pure Rust", "داخل العملية، Rust خالص")), ("pdf", t("in process, lopdf", "داخل العملية، lopdf")),
            ("ffmpeg", t("child process", "عملية ابنة")), ("office", t("child process, LibreOffice", "عملية ابنة، LibreOffice")),
            ("ghostscript", t("child process", "عملية ابنة"))]
    for i, (name, note) in enumerate(rows):
        c.box(30 + (i % 2) * 300, 320 + (i // 2) * 62, 280, 50, [name + "   " + note], size=14)
    c.label(930, 536, t("Never put action logic in the UI.", "لا منطق للإجراءات في الواجهة."), "end", "t", 14)
    return lambda s: c.svg(s, t("Adding a format", "إضافة صيغة"),
                 t("An action file in TOML names the input kind and the label. An engine adapter implements capabilities, estimate and run. The registry joins them into a hive cell.",
                   "ملف إجراء بصيغة TOML يحدّد نوع المدخل والاسم، ومحوّل المحرك ينفّذ capabilities وestimate وrun، ويضمّهما السجل في خلية."))


# ---------------------------------------------------------------------------
# 4. Hive surface choice
# ---------------------------------------------------------------------------

def surface(lang):
    c = Canvas(960, 480, lang == "ar")
    t = lambda en, ar: T(lang, en, ar)
    c.label(30, 34, t("Where the hive is drawn, chosen at every show", "أين تُرسم الخلية، يُحسم عند كل عرض"), "start", "head", 15)
    c.box(30, 70, 250, 100, [t("Forced?", "هل فُرض؟"), "hive_surface", "KAWARO_SURFACE", t("yes: that path wins", "نعم: يغلب ذلك المسار")], mono=True, size=14)
    c.box(350, 70, 250, 100, [t("Wayland with four globals?", "ويلاند مع أربع واجهات؟"), "layer-shell, subcompositor", "viewporter, single-pixel-buffer"], size=13)
    c.box(670, 70, 250, 100, [t("Screen reader running?", "هل يعمل قارئ شاشة؟"), t("not just the flag", "لا العلَم وحده")], size=14)
    c.arrow(280, 120, 350, 120)
    c.label(315, 108, t("no", "لا"), "middle", "t2", 13)
    c.arrow(600, 120, 670, 120)
    c.label(635, 108, t("yes", "نعم"), "middle", "t2", 13)
    c.box(350, 290, 250, 100, [t("Window path", "مسار النافذة"), t("centred, ordinary window", "نافذة عادية في الوسط"), t("GNOME, X11, Windows", "GNOME وX11 وويندوز")], cls="box2", size=15)
    c.box(670, 290, 250, 100, [t("Layer path", "مسار الطبقة"), t("at the pointer or the drag", "عند المؤشر أو السحب"), t("Hyprland, KDE, wlroots", "هايبرلاند وKDE وwlroots")], cls="hot", size=15)
    c.arrow(475, 170, 475, 290)
    c.label(490, 235, t("no", "لا"), "start", "t2", 13)
    c.arrow(795, 170, 795, 290)
    c.label(810, 235, t("no", "لا"), "start", "t2", 13)
    c.arrow(700, 170, 600, 290, dashed=True)
    c.label(668, 245, t("yes", "نعم"), "middle", "t2", 13)
    c.label(30, 440, t("Never assume layer shell exists. Every change of choice is logged once.", "لا يُفترض وجود layer shell أبدا. كل تغيير في الاختيار يُسجَّل مرة."), "start", "t", 14)
    return lambda s: c.svg(s, t("Choosing the hive surface", "اختيار سطح الخلية"),
                 t("A forced setting wins. Otherwise the layer path needs Wayland with four protocol globals and no running screen reader; anything else uses an ordinary window.",
                   "الإعداد المفروض يغلب. وإلا فمسار الطبقة يحتاج ويلاند مع أربع واجهات بروتوكول وبلا قارئ شاشة يعمل، وغير ذلك يُستعمل نافذة عادية."))


# ---------------------------------------------------------------------------
# 5. Updates and releases
# ---------------------------------------------------------------------------

def updates(lang):
    c = Canvas(960, 440, lang == "ar")
    t = lambda en, ar: T(lang, en, ar)
    c.label(30, 34, t("Signed updates, nothing runs unverified", "تحديثات موقّعة، ولا شيء يعمل دون تحقق"), "start", "head", 15)
    c.box(30, 70, 200, 100, [t("Tag v*", "وسم الإصدار"), t("CI builds and tests", "البناء والاختبار تلقائيا")], size=15)
    c.box(280, 70, 200, 100, [t("GitHub release", "إصدار GitHub"), "DEB RPM MSI ZIP", "SHA256SUMS"], size=14)
    c.box(530, 70, 200, 100, [t("Signed manifest", "بيان موقّع"), "minisign", t("size and SHA-256", "الحجم وSHA-256")], size=14)
    c.box(780, 70, 150, 100, [t("App", "التطبيق"), t("checks daily", "يفحص يوميا")], cls="hot", size=15)
    for a, b in ((230, 280), (480, 530)):
        c.arrow(a, 120, b, 120)
    c.arrow(780, 120, 730, 120)
    steps = [([t("1. Verify the signature", "1. تحقق من التوقيع"), t("with the public key", "بالمفتاح العام"), t("built into the app", "المدمج في التطبيق")]),
             ([t("2. Download", "2. نزّل"), t("only then; check", "بعد ذلك فقط؛ وتحقق"), t("size and SHA-256", "من الحجم والبصمة")]),
             ([t("3. Install", "3. ثبّت"), t("Windows: quiet, per user", "ويندوز: صامت لحسابك"), t("Linux: you approve the prompt", "لينكس: توافق على الطلب")])]
    for i, lines in enumerate(steps):
        c.box(30 + i * 310, 240, 290, 110, lines, cls="box2", size=14)
    c.arrow(855, 170, 855, 205)
    c.add(f'<line class="ln" x1="{c.x(855):.1f}" y1="205" x2="{c.x(175):.1f}" y2="205"/>')
    c.arrow(175, 205, 175, 240)
    c.label(30, 402, t("Beta builds follow the beta channel; stable builds follow stable. update_check = false turns it off.", "نسخ البيتا تتبع قناة beta والمستقرة تتبع stable. وupdate_check = false يوقف الفحص."), "start", "t", 13)
    return lambda s: c.svg(s, t("How updates are checked", "كيف تُفحص التحديثات"),
                 t("A release is built from a tag and published with a signed manifest. The app checks it once a day, verifies the signature before downloading, then checks size and hash before installing.",
                   "يُبنى الإصدار من وسم وينشر مع بيان موقّع. يفحصه التطبيق مرة في اليوم، ويتحقق من التوقيع قبل التنزيل ثم من الحجم والبصمة قبل التثبيت."))


# ---------------------------------------------------------------------------
# 6. Hero: the hive itself (same geometry as the app and the website)
# ---------------------------------------------------------------------------

def hero(lang):
    rtl = lang == "ar"
    R = TOK["hex"]["size"]["md"] / 2
    gap = TOK["hex"]["gap"]
    S = R + gap / SQ3
    w, h = 960, 420
    c = Canvas(w, h, False)
    cells = ["WEBP", "PNG", "JPG", "AVIF", "PDF", "GIF"] + ([T(lang, "RESIZE", "تحجيم"), T(lang, "ROTATE", "تدوير"), T(lang, "FLIP", "قلب"),
             T(lang, "GRAY", "رمادي"), T(lang, "SHRINK", "ضغط"), T(lang, "STRIP", "تنظيف")])
    dirs = [(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)]
    coords = [(0, 0)]
    for k in (1, 2):
        q, r = -k, k
        for dq, dr in dirs:
            for _ in range(k):
                coords.append((q, r))
                q, r = q + dq, r + dr
    cx0 = 480 + (170 if rtl else -170)
    for i, (q, r) in enumerate(coords):
        x = S * (SQ3 * q + SQ3 / 2 * r)
        if rtl:
            x = -x
        cx, cy = cx0 + x, 210 + S * 1.5 * r
        if i == 0:
            c.add(f'<path class="hexc" d="M' + "L".join(f"{cx + R * math.cos(math.radians(a)):.1f},{cy + R * math.sin(math.radians(a)):.1f}" for a in (-90, -30, 30, 90, 150, 210)) + 'Z"/>')
            c.add(f'<text class="t" x="{cx:.1f}" y="{cy + 5:.1f}" text-anchor="middle" font-size="13">{T(lang, "One image", "صورة واحدة")}</text>')
            continue
        cls = "hexhot" if i == 1 else "hex"
        c.add(f'<path class="{cls}" d="M' + "L".join(f"{cx + R * math.cos(math.radians(a)):.1f},{cy + R * math.sin(math.radians(a)):.1f}" for a in (-90, -30, 30, 90, 150, 210)) + 'Z"/>')
        if i - 1 < len(cells):
            c.add(f'<text class="{"hexhot-t" if i == 1 else "t"}" x="{cx:.1f}" y="{cy + 5:.1f}" text-anchor="middle" font-size="14" font-weight="600">{html.escape(cells[i - 1])}</text>')
    tx = 730 if not rtl else 230
    anchor = "start" if not rtl else "end"
    tx = 520 if not rtl else 440
    c.add(f'<text class="t" x="{tx + (0 if not rtl else 0)}" y="190" text-anchor="{anchor}" font-size="46" font-weight="600">{T(lang, "Kawaro", "كوارو")}</text>')
    c.add(f'<text class="t2" x="{tx}" y="232" text-anchor="{anchor}" font-size="19">{T(lang, "Select. Pick a cell.", "حدّد. اختر خلية.")}</text>')
    c.add(f'<text class="t2" x="{tx}" y="262" text-anchor="{anchor}" font-size="19">{T(lang, "The result lands beside the original.", "الناتج بجانب الأصل.")}</text>')
    return lambda s: c.svg(s, T(lang, "The Kawaro hive for one image", "خلية كوارو لصورة واحدة"),
                 T(lang, "A pointy top hexagon hive: the centre cell names the selection, the first ring holds target formats, the second ring holds image actions.",
                   "خلية سداسية الرؤوس: الخلية المركزية تسمّي التحديد، والحلقة الأولى فيها صيغ الهدف، والثانية إجراءات الصورة."))


DIAGRAMS = {"hero": hero, "overview": overview, "flow": flow, "extend": extend, "surface": surface, "updates": updates}


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for name, fn in DIAGRAMS.items():
        for lang in ("en", "ar"):
            render = fn(lang)
            for scheme in ("light", "dark"):
                (OUT / f"{name}.{lang}.{scheme}.svg").write_text(render(scheme), encoding="utf-8")
    print("wrote", len(DIAGRAMS) * 4, "diagrams to", OUT.relative_to(ROOT))


if __name__ == "__main__":
    main()
