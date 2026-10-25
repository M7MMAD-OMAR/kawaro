"""S-064 static checks (AC1 to AC7, AC9). Run with: python3 -I test_s064_static.py

Builds a copy of the repo (lib.built) and reads the generated pages and CSS.
"""
import re
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import lib  # noqa: E402
from lib import REPO, LANGS, page_file, site, html_files  # noqa: E402

ORDER = ["hero", "mq", "why", "formats", "download", "open"]  # S-065 removed #sim


def home(lang):
    return page_file(lang).read_text(encoding="utf-8")


def css(name):
    return (site() / "css" / name).read_text(encoding="utf-8")


def site_css():
    return "\n".join(p.read_text(encoding="utf-8") for p in sorted(site().rglob("*.css"))
                     if "pagefind" not in p.parts and p.name != "tokens.css")


def main_part(text):
    return text.split('<main id="main">', 1)[1].split("</main>", 1)[0]


class AC1Structure(unittest.TestCase):
    def test_AC1_sections_in_order(self):
        for lang in LANGS:
            m = main_part(home(lang))
            marks = []
            for key in ORDER:
                pat = {"hero": r'<section class="hero"', "mq": r'<div class="mq"'}.get(key, f'id="{key}"')
                hit = re.search(pat, m)
                self.assertIsNotNone(hit, f"{lang}: {key} missing")
                marks.append(hit.start())
            self.assertEqual(marks, sorted(marks), f"{lang}: sections out of order")

    def test_AC1_one_h1_and_one_h2_per_section(self):
        for lang in LANGS:
            m = main_part(home(lang))
            self.assertEqual(len(re.findall(r"<h1\b", m)), 1, lang)
            for sec in re.findall(r"<section\b.*?</section>", m, flags=re.S)[1:]:
                self.assertEqual(len(re.findall(r"<h2\b", sec)), 1, f"{lang}: {sec[:80]}")

    def test_AC1_no_scene(self):
        for lang in LANGS:
            t = home(lang)
            self.assertNotIn('id="scene"', t)
            self.assertNotIn("scene.js", t)
            self.assertNotIn("scene.css", t)
        self.assertFalse((site() / "js" / "scene.js").exists())
        self.assertFalse((site() / "css" / "scene.css").exists())


class AC2HeroDemo(unittest.TestCase):
    def test_AC2_demo_markup(self):
        # S-067: the Arabic hero is the product film; the demo window stays on the English page.
        for lang in ("en",):
            t = home(lang)
            demo = t.split('<figure class="demo"', 1)[1].split("</figure>", 1)[0]
            self.assertEqual(len(re.findall(r'class="demo-cell', demo)), 7, lang)
            self.assertEqual(demo.count("demo-result"), 1, lang)
            self.assertEqual(demo.count('class="demo-step"'), 3, lang)
            self.assertIn("photo.png", demo)
            self.assertIn("photo.webp", demo)
            self.assertRegex(t, r'<figure class="demo" role="img" aria-label="[^"]{20,}"')

    def test_AC2_demo_css_rest_state_and_rtl(self):
        c = lib.strip_css_comments(css("demo.css"))
        self.assertIn("@keyframes demo-cursor-rtl", c)
        self.assertRegex(c, r'\[dir="rtl"\] \.demo-cursor \{ animation-name: demo-cursor-rtl; \}')
        rest = c.split("@media (prefers-reduced-motion: no-preference)", 1)[0]
        self.assertRegex(rest, r"\.demo-hive \{ display: none;")
        self.assertRegex(rest, r"\.demo-pointer \{ display: none;")
        self.assertNotRegex(rest, r"animation\s*:")
        self.assertIn("var(--duration-loop)", c)


class AC3Display(unittest.TestCase):
    def test_AC3_bold_font_and_tokens(self):
        f = site() / "fonts" / "IBMPlexSansArabic-Bold.woff2"
        self.assertTrue(f.exists())
        self.assertEqual(f.read_bytes()[:4], b"wOF2")
        self.assertRegex(css("../style.css"), r"(?s)IBMPlexSansArabic-Bold\.woff2.*?font-weight: 700")
        tok = (site() / "tokens.css").read_text(encoding="utf-8")
        self.assertIn("--type-display: 176px;", tok)
        m = re.search(r"--type-display-sm: (\d+)px;", tok)
        self.assertGreaterEqual(int(m.group(1)), 44)
        self.assertIn("--font-weight-bold: 700;", tok)
        self.assertIn("var(--type-display)", css("hero.css"))  # S-065: capped below the token for a calmer hero


class AC4Formats(unittest.TestCase):
    def total(self):
        return sum(p.read_text(encoding="utf-8").count("[[action]]") for p in (REPO / "actions").glob("*.toml"))

    def test_AC4_total_and_badges_match_actions(self):
        n = self.total()
        for lang in LANGS:
            t = home(lang)
            sec = t.split('id="formats"', 1)[1].split("</section>", 1)[0]
            self.assertRegex(sec, rf'<h2 id="formats-title"><bdi>{n}</bdi>')
            badges = [int(x) for x in re.findall(r"<sup[^>]*><bdi>(\d+)</bdi></sup>", sec)]
            self.assertEqual(len(badges), 5, lang)
            self.assertEqual(sum(badges), n, lang)
            chips = [len(re.findall(r"<li><bdi>", fam)) for fam in sec.split('<li class="fam"')[1:]]
            self.assertEqual(chips, badges, f"{lang}: every action shows one code")

    def test_AC4_arabic_number_agreement(self):
        sec = home("ar").split('id="formats"', 1)[1]
        for n, word in re.findall(r'aria-label="(\d+) (إجراء|إجراءات)"', sec):
            self.assertEqual(word, "إجراءات" if 3 <= int(n) <= 10 else "إجراء", n)


class AC5Motion(unittest.TestCase):
    def test_AC5_curves_and_durations_from_tokens(self):
        c = lib.strip_css_comments(site_css())
        curves = set(re.findall(r"var\((--ease[\w-]*)\)", c))
        self.assertEqual(curves, {"--ease-out", "--ease-steady"})
        for sel, d in lib.css_rules(c):
            for k, v in d.items():
                if k.startswith(("transition", "animation")):
                    w = re.sub(r"var\([^)]*\)", "", v)
                    self.assertNotRegex(w, r"cubic-bezier|\bease\b|\blinear\b|\d+m?s\b", f"{sel} {k}: {v}")
        self.assertIn("--ease-steady: linear;", (site() / "tokens.css").read_text(encoding="utf-8"))

    def test_AC5_view_timeline_gated_and_never_fades(self):
        # S-065: scroll choreography lives in motion.css, the hero exit in hero.css.
        for p in sorted(site().rglob("*.css")):
            t = lib.strip_css_comments(p.read_text(encoding="utf-8"))
            if "animation-timeline" not in t:
                continue
            self.assertIn(p.name, ("motion.css", "hero.css"), f"{p.name}: scroll timeline outside the gated files")
            gate = t.index("@supports (animation-timeline: view())")
            self.assertRegex(t[gate:], r"^@supports \(animation-timeline: view\(\)\) \{\s*@media \(prefers-reduced-motion: no-preference\)")
            self.assertGreater(t.index("animation-timeline"), gate, p.name)
            for m in re.finditer(r"@keyframes [\w-]+ \{(.*?)\n\}", t, flags=re.S):
                self.assertNotIn("opacity", m.group(1), f"{p.name}: a scroll effect fades content")

    def test_AC5_loops_stop_with_reduced_motion(self):
        for name in ("demo.css", "marquee.css", "hero.css"):
            c = lib.strip_css_comments(css(name))
            for m in re.finditer(r"animation:[^;]*infinite", c):
                before = c[: m.start()]
                self.assertGreater(before.rfind("@media (prefers-reduced-motion: no-preference)"), -1, f"{name}: ungated loop")

    def test_AC5_hover_motion_gated(self):
        for name in ("bento.css", "formats.css"):
            c = lib.strip_css_comments(css(name))
            for m in re.finditer(r"([^{}]*:hover[^{]*)\{([^}]*)\}", c):
                if re.search(r"translate|scale|rotate", m.group(2)):
                    self.assertGreater(c[: m.start()].rfind("(hover: hover)"), -1, f"{name}: {m.group(1).strip()}")


class AC6Shell(unittest.TestCase):
    def test_AC6_header_cta_and_footer_mark(self):
        for f in html_files():
            t = f.read_text(encoding="utf-8")
            if "<header" not in t:
                continue
            header = t.split("<header", 1)[1].split("</header>", 1)[0]
            self.assertRegex(header, r'class="hex-button button-primary hex-button-sm nav-cta"', str(f))
            self.assertIn('class="footer-mark"', t, str(f))
        self.assertRegex(lib.strip_css_comments(css("header.css")), r"\.site-header \{ position: sticky;")


class AC7Kept(unittest.TestCase):
    def test_AC7_no_try_it_demo(self):
        # S-065: the owner removed the try-it section and its script.
        for lang in LANGS:
            t = home(lang)
            self.assertNotIn('id="sim"', t, lang)
            self.assertNotIn("sim.js", t, lang)
            self.assertNotIn('href="#sim"', t, lang)
        self.assertFalse((site() / "js" / "sim.js").exists())
        self.assertFalse((site() / "css" / "sim.css").exists())

    def test_AC7_no_radius_shadow_gradient_one_sided_border(self):
        c = lib.strip_css_comments(site_css())
        for sel, d in lib.css_rules(c):
            for k, v in d.items():
                if "radius" in k:
                    self.assertRegex(v.strip(), r"^0(px)?$", f"{sel} {k}")
        self.assertNotRegex(c, r"box-shadow\s*:\s*(?!none)|text-shadow\s*:\s*(?!none)|drop-shadow\(")
        self.assertNotRegex(c, r"(linear|radial|conic)-gradient")
        self.assertNotRegex(c, r"\bborder-(top|bottom|left|right|block-start|block-end|inline-start|inline-end)(-width|-style|-color)?\s*:")

    def test_AC7_no_raw_values(self):
        bad = []
        for sel, d in lib.css_rules(lib.strip_css_comments(site_css())):
            for k, v in d.items():
                w = re.sub(r"var\([^)]*\)", "", v)
                if re.search(r"#[0-9a-fA-F]{3,8}\b|\brgba?\(|\bhsla?\(", w):
                    bad.append(f"{sel} {k}: {v}")
                if not k.startswith("--") and re.search(r"(?<![\w.#-])(\d*\.?\d+)(px|rem|em|ms|s)\b", w):
                    if any(float(x) for x in re.findall(r"(?<![\w.#-])(\d*\.?\d+)(?:px|rem|em|ms|s)\b", w)):
                        bad.append(f"{sel} {k}: {v}")
        self.assertEqual(bad, [], "\n".join(bad[:20]))


class AC9Finish(unittest.TestCase):
    def test_AC9_no_long_dashes(self):
        dash = re.compile("[" + chr(0x2013) + chr(0x2014) + "]")
        for p in [REPO / "site/build.py", REPO / "backlog/S-064-home-restructure.md"] + sorted((REPO / "site/css").glob("*.css")):
            self.assertIsNone(dash.search(p.read_text(encoding="utf-8")), str(p))

    def test_AC9_changelog_line(self):
        lines = [x for x in (REPO / "CHANGELOG.md").read_text(encoding="utf-8").splitlines() if "S-064" in x]
        self.assertEqual(len(lines), 1)
        self.assertRegex(lines[0], r"[\u0600-\u06FF]")


if __name__ == "__main__":
    unittest.main(verbosity=1)
