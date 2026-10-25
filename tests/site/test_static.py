"""S-061 static checks on the generated site. Run with: python3 -I test_static.py"""
import json
import re
import subprocess
import time
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import lib  # noqa: E402
from lib import REPO, RELEASES, SUPPORT, LANGS, VERSION, parse_file, page_file, site, built, html_files  # noqa: E402

CSS = lambda: (site() / "style.css").read_text(encoding="utf-8")
JS = lambda: (site() / "app.js").read_text(encoding="utf-8")
FILEPATHS = ("README.md", "README.ar.md")


def hero(lang):
    root = parse_file(page_file(lang))
    h = root.find_all("section.hero") or root.find_all(".hero")
    assert h, f"no .hero in the {lang} home page"
    return h[0]


def hero_raw(lang):
    raw = page_file(lang).read_text(encoding="utf-8")
    m = re.search(r'<section[^>]*class="[^"]*\bhero\b.*?</section>', raw, re.S)
    assert m, "no hero section"
    return re.sub(r"<svg\b.*?</svg>", "", m.group(0), flags=re.S)


class AC1Hero(unittest.TestCase):
    @unittest.skip("superseded by S-064: the hero is restructured (live demo, try-it link, 'Free download' label); S-064 AC2 covers it")
    def test_AC1_hero_counts(self):
        for lang in LANGS:
            h = hero(lang)
            prim = h.find_all("a.button-primary")
            self.assertEqual(len(prim), 1, f"{lang}: primary buttons")
            self.assertTrue(prim[0].attrs.get("href", "").rstrip("?").endswith("download/"), f"{lang}: no-JS primary href")
            self.assertEqual(prim[0].attrs.get("data-os-default"), "linux")
            others = [a for a in h.find_all("a") if a is not prim[0] and a.attrs.get("href", "").endswith("download/")]
            self.assertEqual(len(others), 1, f"{lang}: 'all downloads' link")
            self.assertEqual(len(h.find_all("p.hero-note")), 1, f"{lang}: hero-note")
            self.assertEqual(len(h.find_all(".pill")), 0)
            self.assertEqual(len(h.find_all(".facts")), 0)
            self.assertNotIn("releases/download/", hero_raw(lang))

    def test_AC1_hero_note_short_and_only_windows_link(self):
        for lang in LANGS:
            note = hero(lang).find_all("p.hero-note")
            self.assertTrue(note, f"{lang}: no p.hero-note")
            n = note[0]
            self.assertLessEqual(len(" ".join(n.text().split())), 90)
            for a in n.find_all("a"):
                self.assertIn("windows", a.attrs.get("href", ""), "only the Windows link may sit in the note")

    @unittest.skip("superseded by S-064: the hero is restructured (live demo, try-it link, 'Free download' label); S-064 AC2 covers it")
    def test_AC1_hero_size(self):
        for lang in LANGS:
            self.assertLessEqual(len(hero_raw(lang).encode()), 1200, f"{lang}: hero bytes outside svg")

    @unittest.skip("superseded by S-064: the hero is restructured (live demo, try-it link, 'Free download' label); S-064 AC2 covers it")
    def test_AC1_nojs_label_is_download(self):
        for lang, word in (("en", "Download"), ("ar", "تنزيل")):
            prim = hero(lang).find_all("a.button-primary")
            self.assertTrue(prim)
            self.assertEqual(prim[0].text().strip(), word)

    def test_AC1_app_js_swaps_on_windows_ua(self):
        js = JS()
        self.assertIn("data-os-default", js)
        self.assertRegex(js, r"navigator\.userAgent")
        self.assertIn("Windows", js)


class AC2Spacing(unittest.TestCase):
    def test_AC2_tokens_space_7_8(self):
        t = json.loads((REPO / "design/tokens.json").read_text(encoding="utf-8"))
        self.assertEqual(t["space"].get("7"), 48)
        self.assertEqual(t["space"].get("8"), 64)
        tok = (site() / "tokens.css").read_text(encoding="utf-8")
        self.assertIn("--space-7: 48px;", tok)
        self.assertIn("--space-8: 64px;", tok)

    def test_AC2_style_spacing_only_tokens(self):
        bad = lib.spacing_violations(CSS())
        self.assertEqual(bad, [], "hand written spacing:\n" + "\n".join(bad[:20]))

    def test_AC2_checker_catches_planted_value(self):
        for planted in ("margin: 13px", "padding: 1rem", "gap: 0.5em", "inset: 10px", "margin: 0 var(--space-2) 3px"):
            self.assertTrue(lib.spacing_violations(".x { %s }" % planted), planted)
        for ok in ("margin: 0 auto", "padding: var(--space-2) var(--space-4)", "gap: calc(var(--space-2) * 2)", "inset: 0"):
            self.assertEqual(lib.spacing_violations(".x { %s }" % ok), [], ok)


class AC4Mouse(unittest.TestCase):
    @staticmethod
    def outside_sim(js):
        # contract: the simulation code sits between "// sim:begin" and "// sim:end" comment lines
        return re.sub(r"//\s*sim:begin.*?//\s*sim:end", "", js, flags=re.S)

    def test_AC4_no_pointer_listeners_outside_sim(self):
        rest = self.outside_sim(JS())
        self.assertEqual(re.findall(r"(?:pointermove|mousemove)", rest), [])

    def test_AC4_no_pointer_fed_custom_property(self):
        rest = self.outside_sim(JS())
        for line in rest.splitlines():
            if "setProperty" in line:
                self.assertNotRegex(line, r"client[XY]|offset[XY]|page[XY]|pointer|mouse")
        self.assertNotRegex(CSS(), r"--(?:mx|my|pointer|mouse|cursor)[\w-]*\s*:")

    def test_AC4_hover_only_background_surface_token(self):
        n = 0
        for sel, d in lib.css_rules(CSS()):
            if ":hover" in sel:
                n += 1
                for k, v in d.items():
                    self.assertIn(k, ("background", "background-color"), f"{sel}: {k}")
                    self.assertRegex(v, r"^var\(--color-surface-\d\)$", f"{sel}: {v}")
        for sel, d in lib.css_rules(CSS()):
            tr = d.get("transition")
            if tr and ("hover" in sel or "button" in sel):
                self.assertRegex(tr, r"^background(-color)?\s+var\(--motion-hover\)")


def support_violations(text, site_mode=True):
    """Return problems for every support link in text (html anchors and markdown urls)."""
    bad = []
    for m in re.finditer(r"<a\b([^>]*)>", text):
        attrs = m.group(1)
        h = re.search(r'href="([^"]*)"', attrs)
        is_support = bool(h) and (h.group(1).startswith("https://sbarah.com/support") or re.match(r"^(\.\.?/|/)?[\w./-]*/support/?$", h.group(1)) or h.group(1) in ("/support", "/support/"))
        if is_support:
            if h.group(1) != SUPPORT:
                bad.append("href " + h.group(1))
            elif site_mode:
                if 'target="_blank"' not in attrs:
                    bad.append("no target")
                r = re.search(r'rel="([^"]*)"', attrs)
                if not r or not {"noopener", "noreferrer"} <= set(r.group(1).split()):
                    bad.append("no rel")
    for m in re.finditer(r"\(?(https?://sbarah\.com/support[^\s)\"']*)", text):
        if m.group(1) != SUPPORT:
            bad.append("url " + m.group(1))
    return bad


class AC5Support(unittest.TestCase):
    def test_AC5_checker_catches_planted_links(self):
        good = f'<a href="{SUPPORT}" target="_blank" rel="noopener noreferrer">x</a>'
        self.assertEqual(support_violations(good), [])
        self.assertTrue(support_violations('<a href="https://sbarah.com/support/" target="_blank" rel="noopener noreferrer">x</a>'))
        self.assertTrue(support_violations('<a href="/support/" target="_blank" rel="noopener noreferrer">x</a>'))
        self.assertTrue(support_violations(f'<a href="{SUPPORT}">x</a>'))

    def test_AC5_every_support_link_on_site(self):
        bad = []
        for f in html_files():
            for v in support_violations(f.read_text(encoding="utf-8")):
                bad.append(f"{f.relative_to(site())}: {v}")
        self.assertEqual(bad, [])

    def test_AC5_readmes_exact_href(self):
        for name in FILEPATHS:
            txt = (REPO / name).read_text(encoding="utf-8")
            self.assertIn(SUPPORT, txt)
            self.assertEqual(support_violations(txt, site_mode=False), [], name)

    def test_AC5_no_support_page_on_site(self):
        for p in site().rglob("*"):
            rel = p.relative_to(site())
            if "pagefind" in rel.parts:
                continue
            for part in rel.parts:
                # the two owner SVGs (support-sbarah-*.svg) are allowed; a page or folder named support is not
                self.assertNotRegex(part, r"^support(\.\w+)?$", str(rel))
        self.assertTrue((site() / "assets/support-sbarah-dark.svg").is_file())
        self.assertTrue((site() / "assets/support-sbarah-light.svg").is_file())

    def test_AC5_counts_per_page(self):
        for lang in LANGS:
            for page, has_section in (("", True), ("download", False)):
                root = parse_file(page_file(lang, page))
                links = [a for a in root.find_all("a") if a.attrs.get("href") == SUPPORT]
                where = {"header": 0, "footer": 0, "support": 0}
                for a in links:
                    anc = [x.tag for x in a.ancestors()] + [x.attrs.get("id") for x in a.ancestors()]
                    for key in where:
                        if key in anc:
                            where[key] += 1
                self.assertEqual(where["header"], 1, f"{lang}/{page}")
                self.assertEqual(where["footer"], 1, f"{lang}/{page}")
                if has_section:
                    self.assertEqual(where["support"], 1, f"{lang}/{page} #support")
                    self.assertEqual(len(links), 3)

    @unittest.skip("superseded by S-063")
    def test_AC5_support_section_structure(self):
        for lang in LANGS:
            root = parse_file(page_file(lang))
            sec = root.find_all("#support")
            self.assertEqual(len(sec), 1, f"{lang}: #support")
            s = sec[0]
            self.assertEqual(len(s.find_all("a")), 1)
            self.assertEqual(len(s.find_all("button")), 0)
            a = s.find_all("a")[0]
            pic = a.find_all("picture")
            self.assertEqual(len(pic), 1, "the link wraps one <picture>")
            srcs = [x.attrs.get("srcset", "") for x in pic[0].find_all("source")] + [x.attrs.get("src", "") for x in pic[0].find_all("img")]
            self.assertTrue(any(x.endswith("assets/support-sbarah-dark.svg") for x in srcs), srcs)
            self.assertTrue(any(x.endswith("assets/support-sbarah-light.svg") for x in srcs), srcs)
            self.assertNotIn("--color-", site().joinpath(page_file(lang).relative_to(site())).read_text(encoding="utf-8").split('id="support"')[-1].split("</section>")[0])

    def test_AC5_support_css_uses_sbarah_tokens_only(self):
        found = 0
        for sel, d in lib.css_rules(CSS()):
            if "#support" in sel or "s-support" in sel:
                found += 1
                for k, v in d.items():
                    self.assertNotIn("--color-", v, f"{sel} {k}")
        self.assertGreater(found, 0, "no CSS rule for #support / .s-support")
        css = lib.strip_css_comments(CSS())
        self.assertRegex(css, r"var\(--sbarah-(bg|accent|heart|text)\)")

    def test_AC5_sbarah_tokens_match_svgs(self):
        t = json.loads((REPO / "design/tokens.json").read_text(encoding="utf-8"))
        self.assertIn("sbarah", t, "design/tokens.json needs a top level 'sbarah' object with 'dark' and 'light'")
        s = t["sbarah"]
        self.assertEqual({k: v.upper() for k, v in s["dark"].items() if k in ("bg", "accent", "heart", "text")},
                         {"bg": "#161616", "accent": "#BBD686", "heart": "#B2675E", "text": "#FDFBF7"})
        fills = lambda n: {m.upper() for m in re.findall(r'fill="(#[0-9A-Fa-f]{6})"', (REPO / f"docs/assets/support-sbarah-{n}.svg").read_text())}
        self.assertTrue({v.upper() for v in s["dark"].values()} <= fills("dark"))
        light = {k: v.upper() for k, v in s["light"].items()}
        self.assertEqual(set(light), {"bg", "accent", "heart", "text"})
        self.assertTrue(set(light.values()) <= fills("light"), (light, fills("light")))
        self.assertEqual(light["heart"], "#B2675E")
        tok = (site() / "tokens.css").read_text(encoding="utf-8")
        for k in ("bg", "accent", "heart", "text"):
            self.assertIn(f"--sbarah-{k}:", tok)


class AC6Downloads(unittest.TestCase):
    def dl_anchors(self, variant):
        out = []
        for f in html_files(variant):
            for a in parse_file(f).find_all("a"):
                if "data-dl" in a.attrs:
                    out.append((f, a))
        return out

    def test_AC6_unpublished_no_asset_links(self):
        for variant in ("0", "e0"):
            for f in html_files(variant):
                self.assertNotIn("/releases/download/", f.read_text(encoding="utf-8"), f"{variant}: {f}")
            for p in (lib.site(variant)).glob("**/*.md"):
                self.assertNotIn("/releases/download/", p.read_text(encoding="utf-8"), f"{variant}: {p}")

    def test_AC6_unpublished_buttons_point_to_releases(self):
        anchors = self.dl_anchors("0")
        self.assertGreaterEqual(len(anchors), 4, "download buttons carry data-dl=msi|zip|deb|rpm")
        for f, a in anchors:
            self.assertEqual(a.attrs["href"], RELEASES, str(f))

    def test_AC6_coming_soon_both_languages(self):
        en = page_file("en", "download").read_text(encoding="utf-8")
        ar = page_file("ar", "download").read_text(encoding="utf-8")
        self.assertRegex(en, r"(?i)coming soon")
        self.assertIn("قريبا", ar)
        self.assertIn("soon", en)

    def test_AC6_published_uses_asset_urls(self):
        anchors = self.dl_anchors("1")
        self.assertGreaterEqual(len(anchors), 4)
        kinds = set()
        for f, a in anchors:
            self.assertRegex(a.attrs["href"], r"^" + re.escape(RELEASES) + r"/download/" + lib.TAG + r"/kawaro[-_].*\.(msi|zip|deb|rpm)$")
            kinds.add(a.attrs["data-dl"])
        self.assertEqual(kinds, {"msi", "zip", "deb", "rpm"})

    def test_AC6_published_without_tag_exits_1(self):
        root = lib.make_copy("notag")
        r = lib.run_build(root, 1)
        self.assertEqual(r.returncode, 1, r.stdout[-300:] + r.stderr[-300:])
        self.assertIn(lib.TAG, r.stdout + r.stderr)

    def test_AC6_build_reads_env_default_zero(self):
        self.assertRegex((REPO / "site/build.py").read_text(encoding="utf-8"), r"RELEASE_PUBLISHED")


class AC9Rtl(unittest.TestCase):
    def test_AC9_no_physical_properties(self):
        css = lib.strip_css_comments(CSS())
        pats = [r"(?<![\w-])(left|right)\s*:", r"margin-(left|right)\b", r"padding-(left|right)\b",
                r"text-align\s*:\s*(left|right)\b", r"border-(left|right)\b", r"\bfloat\s*:\s*(left|right)"]
        hits = [p for p in pats if re.search(p, css)]
        self.assertEqual(hits, [])

    def test_AC9_latin_tokens_in_arabic_wrapped(self):
        token = re.compile(r"\b(MSI|DEB|RPM|ZIP|EXE)\b|\d+\.\d+\.\d+")
        bad = []
        skip = {"bdi", "code", "pre", "svg", "script", "style", "head", "title"}
        for f in html_files():
            rel = f.relative_to(site())
            if rel.parts[0] == "en":
                continue
            root = parse_file(f)
            for n in root.walk():
                if n.tag in skip or n.attrs.get("dir") == "ltr":
                    continue
                if any(a.tag in skip or a.attrs.get("dir") == "ltr" for a in n.ancestors()):
                    continue
                for c in n.children:
                    if isinstance(c, str) and token.search(c):
                        bad.append(f"{rel}: {c.strip()[:60]}")
        self.assertEqual(bad, [])

    def test_AC9_footer_lists(self):
        for lang in LANGS:
            root = parse_file(page_file(lang))
            foot = root.find_all("footer")[0]
            uls = foot.find_all("ul")
            self.assertGreaterEqual(len(uls), 2, f"{lang}: footer needs ul.footer-product then ul.footer-lang")
            self.assertEqual(len(foot.find_all("ul.footer-product")), 1)
            self.assertEqual(len(foot.find_all("ul.footer-lang")), 1)
            order = [u.cls() for u in uls]
            self.assertLess(next(i for i, c in enumerate(order) if "footer-product" in c),
                            next(i for i, c in enumerate(order) if "footer-lang" in c))
            sw = foot.find_all("ul.footer-lang")[0].find_all("a")
            self.assertEqual(len(sw), 1)
            self.assertTrue(sw[0].attrs.get("hreflang"))
            self.assertNotIn("·", foot.text())

    def test_AC9_footer_link_text_language(self):
        allowed = re.compile(r"GitHub|SHA256SUMS|MSI|DEB|RPM|ZIP")
        for lang in LANGS:
            root = parse_file(page_file(lang))
            foot = root.find_all("footer")[0]
            prod = foot.find_all("ul.footer-product")
            self.assertTrue(prod, "no ul.footer-product")
            for a in prod[0].find_all("a"):
                t = a.text()
                if lang == "ar":
                    self.assertIsNone(re.search(r"[A-Za-z]", allowed.sub("", t)), t)
                else:
                    self.assertIsNone(re.search(r"[؀-ۿ]", t), t)


class AC12Finish(unittest.TestCase):
    def test_AC12_no_inline_handlers_and_style_attr(self):
        for f in html_files():
            txt = f.read_text(encoding="utf-8")
            self.assertIsNone(re.search(r"<[^>]+\son\w+\s*=", txt), f"on*= in {f}")
            for m in re.finditer(r'\sstyle="([^"]*)"', txt):
                self.assertRegex(m.group(1), r"^(--[\w-]+:\s*-?[\d.]+;?\s*)+$", f"{f}")

    def test_AC12_csp_unchanged(self):
        # run against the built copy so inline script hashes come from the new pages
        r = subprocess.run(["python3", str(built() / "scripts/site-headers.py"), str(site())], capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stderr)
        h = (site() / "_headers").read_text(encoding="utf-8")
        # S-059 AC2 replaced the S-061 values: no data: images, inline style only as attributes.
        for d in ("default-src 'none'", "style-src 'self';", "style-src-attr 'unsafe-inline'", "img-src 'self';", "font-src 'self'",
                  "connect-src 'self'", "frame-ancestors 'none'"):
            self.assertIn(d, h)
        self.assertNotIn("https://", h.split("Content-Security-Policy:")[1].split("\n")[0])
        for f in html_files():
            scripts = re.findall(r"<script(?![^>]*\bsrc=)([^>]*)>", f.read_text(encoding="utf-8"))
            self.assertLessEqual(len([s for s in scripts if "json" not in s]), 1, f)

    def test_AC12_no_long_dashes(self):
        bad = []
        for rel in ("site/build.py", "site/style.css", "site/app.js", "design/tokens.json", "README.md", "README.ar.md",
                    "CHANGELOG.md", "backlog/S-061-site-polish.md"):
            p = REPO / rel
            if p.exists() and re.search("[\u2013\u2014]", p.read_text(encoding="utf-8")):
                bad.append(rel)
        for f in html_files():
            if re.search("[\u2013\u2014]", f.read_text(encoding="utf-8")):
                bad.append(str(f.relative_to(site())))
        self.assertEqual(bad, [])

    def test_AC12_changelog_line(self):
        lines = [l for l in (REPO / "CHANGELOG.md").read_text(encoding="utf-8").splitlines() if "S-061" in l]
        self.assertEqual(len(lines), 1, "exactly one CHANGELOG line for S-061")
        self.assertRegex(lines[0], r"[A-Za-z]")
        self.assertRegex(lines[0], r"[؀-ۿ]")

    def test_AC12_design_lint_clean(self):
        r = subprocess.run(["bash", str(REPO / "scripts/design-lint.sh")], cwd=str(REPO), capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stdout)

    def test_AC12_build_reproducible(self):
        a = built("0")
        time.sleep(2)  # a timestamp baked into any file must show up as a diff
        b = built("rep")
        r = subprocess.run(["diff", "-r", str(a / "site"), str(b / "site")], capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stdout[:600])


if __name__ == "__main__":
    unittest.main(verbosity=2)
