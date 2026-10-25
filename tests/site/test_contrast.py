"""S-061 AC10: WCAG contrast of every text/background pair. Run with: python3 -I test_contrast.py"""
import re
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import lib  # noqa: E402


def lum(hexv):
    h = hexv.lstrip("#")
    if len(h) == 3:
        h = "".join(c * 2 for c in h)
    ch = [int(h[i:i + 2], 16) / 255 for i in (0, 2, 4)]
    ch = [c / 12.92 if c <= 0.03928 else ((c + 0.055) / 1.055) ** 2.4 for c in ch]
    return 0.2126 * ch[0] + 0.7152 * ch[1] + 0.0722 * ch[2]


def ratio(a, b):
    la, lb = sorted((lum(a), lum(b)), reverse=True)
    return (la + 0.05) / (lb + 0.05)


def schemes(tokens_css):
    css = lib.strip_css_comments(tokens_css)
    props = lambda block: dict(re.findall(r"--color-([\w-]+)\s*:\s*(#[0-9A-Fa-f]{3,8})\s*;", block))
    root = re.search(r":root\s*\{(.*?)\}", css, re.S).group(1)
    dark_media = re.search(r"@media\s*\(prefers-color-scheme:\s*dark\)\s*\{\s*:root\s*\{(.*?)\}", css, re.S).group(1)
    return props(root), props(dark_media)


def sbarah(tokens_css):
    css = lib.strip_css_comments(tokens_css)
    flat = dict(re.findall(r"--sbarah-([\w-]+)\s*:\s*(#[0-9A-Fa-f]{3,8})\s*;", css))
    return flat


def value(v, scheme):
    m = re.fullmatch(r"var\(--color-([\w-]+)\)", v.strip())
    return scheme.get(m.group(1)) if m else None


def pairs():
    tok = (lib.site() / "tokens.css").read_text(encoding="utf-8")
    light, dark = schemes(tok)
    fs = {k: int(v) for k, v in re.findall(r"--font-size-([\w-]+):\s*(\d+)px", tok)}
    out = []
    for sel, d in lib.css_rules((lib.site() / "style.css").read_text(encoding="utf-8")):
        fg = d.get("color")
        bg = d.get("background") or d.get("background-color")
        size = None
        m = re.fullmatch(r"var\(--font-size-([\w-]+)\)", d.get("font-size", ""))
        if m:
            size = fs.get(m.group(1))
        need = 3.0 if (size or 0) >= 24 else 4.5
        for name, sch in (("light", light), ("dark", dark)):
            if ".inv" in sel:
                sch = dark if name == "light" else light
            f = value(fg, sch) if fg else None
            b = value(bg, sch) if bg else (sch.get("surface-0") if fg else None)
            if f and b:
                out.append((ratio(f, b), need, name, sel, f, b))
    return out, tok


class AC10Contrast(unittest.TestCase):
    def test_AC10_all_pairs(self):
        ps, tok = pairs()
        self.assertGreater(len(ps), 20, "too few pairs found in style.css")
        low = sorted(ps)[:1]
        table = "\n".join(f"{r:5.2f} (need {n}) {s:5} {sel} {f} on {b}" for r, n, s, sel, f, b in sorted(ps))
        print(table)
        fails = [p for p in ps if p[0] < p[1]]
        self.assertEqual(fails, [], f"lowest ratio {low[0][0]:.2f}: {low[0][3]}\n" + table)

    def test_AC10_sbarah_section_pairs(self):
        tok = (lib.site() / "tokens.css").read_text(encoding="utf-8")
        css = lib.strip_css_comments(tok)
        root = re.search(r":root\s*\{(.*?)\}", css, re.S).group(1)
        dmed = re.search(r"@media\s*\(prefers-color-scheme:\s*dark\)\s*\{\s*:root\s*\{(.*?)\}", css, re.S).group(1)
        get = lambda blk: dict(re.findall(r"--sbarah-([\w-]+)\s*:\s*(#[0-9A-Fa-f]{3,8})\s*;", blk))
        s, dk = get(root), get(dmed)
        for name, t in (("light", s), ("dark", dk)):
            self.assertTrue({"bg", "text", "accent", "heart"} <= set(t), f"--sbarah-* {name} set missing in tokens.css: {t}")
        for name, t in (("light", s), ("dark", dk)):
            self.assertGreaterEqual(ratio(t["text"], t["bg"]), 4.5, f"{name} text on bg")
            self.assertGreaterEqual(ratio(t["accent"], t["bg"]), 3.0, f"{name} accent on bg")
            self.assertGreaterEqual(ratio(t["heart"], t["bg"]), 3.0, f"{name} heart on bg")


if __name__ == "__main__":
    unittest.main(verbosity=2)
