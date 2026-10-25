"""S-063 static checks (AC1, AC2, AC3, AC5, AC6, AC8, AC9, AC10). Run with: python3 -I test_s063_static.py

Contracts the implementation must follow (they come from the acceptance criteria
and are fixed here so the tests are exact):
  design/tokens.json            top level "site": color (flat name -> hex), radius (all 0), type (h1 >= 72),
                                space, motion ("easing" plus >= 2 numeric "duration*" entries)
  site/css/**/*.css             all site CSS; per component file site/css/components/NAME.css
  site/js/*.js                  scroll scene code lives in a file whose name contains "scene"
  site/build.py                 helpers hex_button, section_head, hex_plate, step_cell, format_cell, download_slab
  site/assets/*.webp            generated art, < 200 KB, alt text in both languages
"""
import hashlib
import json
import re
import shutil
import subprocess
import sys
import time
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import lib  # noqa: E402
from lib import REPO, LANGS, TMP, parse_file, page_file, site, built, html_files, make_copy, run_build  # noqa: E402

APP_KEYS_SHA = "3dffa48b9a2bd42332992740608e936c80250a75e8f9e05fc7e2fa239f41c50b"
HELPERS = ("hex_button", "section_head", "hex_plate", "step_cell", "format_cell", "download_slab")
COMPONENTS = {  # helper -> (css file stem, base class)
    "hex_button": "hex-button", "section_head": "section-head", "hex_plate": "hex-plate",
    "step_cell": "step-cell", "format_cell": "format-cell", "download_slab": "download-slab",
}
DASH = re.compile("[" + chr(0x2013) + chr(0x2014) + "]")


def css_files(root=None):
    s = root or site()
    return sorted(p for p in s.rglob("*.css") if "pagefind" not in p.parts)


def all_css(root=None):
    return "\n".join(p.read_text(encoding="utf-8") for p in css_files(root))


def site_css_no_tokens(root=None):
    return "\n".join(p.read_text(encoding="utf-8") for p in css_files(root) if p.name != "tokens.css")


def js_files(root=None):
    s = root or site()
    return sorted(p for p in s.rglob("*.js") if "pagefind" not in p.parts)


class AC1LightOnly(unittest.TestCase):
    def test_AC1_meta_color_scheme_light_everywhere(self):
        files = html_files()
        self.assertGreater(len(files), 10)
        for f in files:
            m = re.search(r'<meta name="color-scheme" content="([^"]*)"', f.read_text(encoding="utf-8"))
            if "docs/" in str(f) and not m:
                continue
            self.assertIsNotNone(m, f"no color-scheme meta in {f}")
            self.assertEqual(m.group(1), "light", str(f))

    def test_AC1_no_dark_in_css_html_js(self):
        css = lib.strip_css_comments(all_css())
        self.assertGreater(len(css_files()), 1, "site CSS must be split into site/css/")
        self.assertNotIn("prefers-color-scheme", css)
        self.assertNotRegex(css, r"\.inv\b")
        self.assertNotIn('data-scheme="dark"', css)
        self.assertNotIn("[data-scheme=dark]", css)
        self.assertNotRegex(css, r"\.dark\.svg")
        for f in html_files():
            t = f.read_text(encoding="utf-8")
            self.assertNotIn("prefers-color-scheme", t, str(f))
            self.assertNotRegex(t, r"\.dark\.svg", str(f))
            self.assertNotRegex(t, r'class="[^"]*\binv\b', str(f))
            self.assertNotIn('data-scheme="dark"', t, str(f))
        for j in js_files():
            self.assertNotIn("prefers-color-scheme", j.read_text(encoding="utf-8"), str(j))

    def test_AC1_tokens_css_has_no_dark_block(self):
        tok = (site() / "tokens.css").read_text(encoding="utf-8")
        self.assertNotIn("prefers-color-scheme", tok)
        self.assertIn("--", tok)


class AC2SiteTokens(unittest.TestCase):
    def tokens(self):
        return json.loads((REPO / "design/tokens.json").read_text(encoding="utf-8"))

    def test_AC2_site_block_shape(self):
        s = self.tokens().get("site")
        self.assertIsInstance(s, dict, "design/tokens.json needs a top level 'site' block")
        for k in ("color", "radius", "type", "space", "motion"):
            self.assertIn(k, s, f"site.{k}")
        self.assertGreaterEqual(len(s["color"]), 6)
        for name, v in s["color"].items():
            self.assertRegex(v, r"^#[0-9A-Fa-f]{6}$", name)
        for name, v in s["radius"].items():
            self.assertEqual(v, 0, f"site.radius.{name}")
        self.assertGreaterEqual(s["type"]["h1"], 72)
        self.assertGreaterEqual(s["type"].get("h2", 0), 56)
        self.assertGreaterEqual(s["type"].get("body", 0), 20)
        self.assertIsInstance(s["motion"].get("easing"), str)
        durs = [v for k, v in s["motion"].items() if k.startswith("duration")]
        self.assertGreaterEqual(len(durs), 2)
        for v in durs:
            self.assertIsInstance(v, (int, float))

    def test_AC2_app_keys_unchanged(self):
        d = self.tokens()
        d.pop("site", None)
        h = hashlib.sha256(json.dumps(d, sort_keys=True, ensure_ascii=False, separators=(",", ":")).encode()).hexdigest()
        self.assertEqual(h, APP_KEYS_SHA, "app keys of design/tokens.json changed")
        self.assertIn("site", self.tokens(), "site block missing")

    def test_AC2_generator_reads_site_block_only(self):
        root = make_copy("s063-gen")
        p = root / "design/tokens.json"
        t = json.loads(p.read_text(encoding="utf-8"))
        self.assertIn("site", t, "site block missing")
        first = next(iter(t["site"]["color"]))
        t["site"]["color"][first] = "#0A1B2C"
        t["color"]["light"][next(iter(t["color"]["light"]))] = "#C0FFEE"
        p.write_text(json.dumps(t, ensure_ascii=False, indent=2), encoding="utf-8")
        r = subprocess.run(["python3", "scripts/tokens-to-css.py"], cwd=root, capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stderr)
        tok = (root / "site/tokens.css").read_text(encoding="utf-8").lower()
        self.assertIn("#0a1b2c", tok)
        self.assertNotIn("#c0ffee", tok)
        self.assertNotRegex(tok, r"--radius[\w-]*:\s*[1-9]")

    def test_AC2_no_raw_values_in_site_css(self):
        bad = []
        files = [p for p in css_files() if p.name != "tokens.css"]
        self.assertTrue(any("components" in p.parts for p in files), "no site/css/components/*.css")
        for p in files:
            for sel, d in lib.css_rules(p.read_text(encoding="utf-8")):
                for k, v in d.items():
                    w = re.sub(r"var\([^)]*\)", "", v)
                    if re.search(r"#[0-9a-fA-F]{3,8}\b|\brgba?\(|\bhsla?\(", w):
                        bad.append(f"{p.name} {sel} {k}: {v} (colour)")
                    if not k.startswith("--"):
                        for m in re.finditer(r"(?<![\w.#-])(\d*\.?\d+)(px|rem|em|ms|s)\b", w):
                            if float(m.group(1)) != 0:
                                bad.append(f"{p.name} {sel} {k}: {v} (length or duration)")
                                break
        self.assertEqual(bad, [], "\n".join(bad[:25]))


class AC3SharpShapes(unittest.TestCase):
    def test_AC3_no_radius_shadow_gradient_in_css(self):
        css = lib.strip_css_comments(all_css())
        self.assertGreater(len(css_files()), 1, "site CSS must be split into site/css/")
        for sel, d in lib.css_rules(css):
            for k, v in d.items():
                if "radius" in k:
                    self.assertRegex(v.strip(), r"^0(px)?$", f"{sel} {k}")
        self.assertNotRegex(css, r"box-shadow\s*:\s*(?!none)")
        self.assertNotRegex(css, r"text-shadow\s*:\s*(?!none)")
        self.assertNotRegex(css, r"drop-shadow\(")
        self.assertNotRegex(css, r"(linear|radial|conic)-gradient")
        self.assertNotRegex(css, r"\bborder-(top|bottom|left|right|block-start|block-end|inline-start|inline-end)(-width|-style|-color)?\s*:")
        self.assertNotRegex(css, r"\bborder-(block|inline)(-width|-style|-color)?\s*:")

    def test_AC3_clip_path_polygons_used(self):
        css = lib.strip_css_comments(all_css())
        self.assertRegex(css, r"clip-path\s*:\s*polygon\(")

    def test_AC3_svg_has_no_rounding_or_filters(self):
        self.assertTrue((site() / "css/components").is_dir(), "site/css/components missing")
        for f in html_files():
            t = f.read_text(encoding="utf-8")
            self.assertNotRegex(t, r"<fe(?:DropShadow|GaussianBlur)\b", str(f))
            self.assertNotRegex(t, r"(?:Gradient)\b", str(f))


class AC5Components(unittest.TestCase):
    def test_AC5_helpers_exist(self):
        src = (built() / "site/build.py").read_text(encoding="utf-8")
        for h in HELPERS:
            self.assertRegex(src, rf"^def {h}\(", f"build.py needs def {h}(")

    def test_AC5_component_css_files_exist_and_are_own(self):
        d = site() / "css/components"
        for stem in COMPONENTS.values():
            f = d / f"{stem}.css"
            self.assertTrue(f.is_file(), f"site/css/components/{stem}.css")
            css = lib.strip_css_comments(f.read_text(encoding="utf-8"))
            sels = [s for s, _ in lib.css_rules(css)]
            self.assertGreater(len(sels), 0, stem)
            for s in sels:
                for part in re.split(r",", s):
                    classes = re.findall(r"\.([\w-]+)", part)
                    if classes:
                        self.assertTrue(classes[0].startswith(stem), f"{stem}.css styles {part.strip()}")
            for other in COMPONENTS.values():
                if other != stem:
                    self.assertNotRegex(css, rf"\.{other}\b", f"{stem}.css styles {other}")

    def test_AC5_other_css_does_not_style_components(self):
        self.assertTrue((site() / "css/components").is_dir(), "site/css/components missing")
        for p in css_files():
            if "components" in p.parts or p.name == "tokens.css":
                continue
            css = lib.strip_css_comments(p.read_text(encoding="utf-8"))
            for stem in COMPONENTS.values():
                self.assertNotRegex(css, rf"\.{stem}\b", f"{p.name} styles .{stem}")

    def test_AC5_each_file_linked_once_and_classes_used_on_home(self):
        for lang in LANGS:
            raw = page_file(lang).read_text(encoding="utf-8")
            for stem in COMPONENTS.values():
                n = len(re.findall(rf'<link[^>]+href="[^"]*css/components/{stem}\.css"', raw))
                self.assertEqual(n, 1, f"{lang}: {stem}.css linked {n} times")
                self.assertRegex(raw, rf'class="[^"]*\b{stem}\b', f"{lang}: nothing uses .{stem}")


class AC6SceneStatic(unittest.TestCase):
    def scene_js(self):
        fs = [p for p in js_files() if "scene" in p.name]
        self.assertTrue(fs, "site/js/*scene*.js missing")
        return "\n".join(p.read_text(encoding="utf-8") for p in fs)

    def test_AC6_one_raf_loop_scroll_only(self):
        js = self.scene_js()
        self.assertRegex(js, r"requestAnimationFrame")
        self.assertRegex(js, r"addEventListener\(\s*['\"]scroll['\"]")
        self.assertEqual(re.findall(r"pointermove|mousemove|touchmove|wheel", js), [])
        self.assertLessEqual(len(re.findall(r"requestAnimationFrame\s*\(", js)), 2, "one loop: schedule plus its re-arm")

    def test_AC6_scene_css_animates_transform_and_opacity_only(self):
        n = 0
        css = lib.strip_css_comments(all_css())
        for sel, d in lib.css_rules(css):
            if "scene" not in sel:
                continue
            n += 1
            for k in ("transition", "transition-property", "will-change"):
                if k in d:
                    for part in d[k].split(","):
                        prop = part.split()[0] if part.split() else ""
                        self.assertIn(prop, ("transform", "opacity", "none"), f"{sel} {k}: {d[k]}")
        self.assertGreater(n, 3, "no scene rules in site CSS")
        for m in re.finditer(r"@keyframes[^{]*\{(.*?)\}\s*\}", css, re.S):
            props = set(re.findall(r"([\w-]+)\s*:", m.group(1)))
            self.assertLessEqual(props, {"transform", "opacity"}, props)

    def test_AC6_scene_markup(self):
        for lang in LANGS:
            root = parse_file(page_file(lang))
            sc = root.find_all("section#scene")
            self.assertEqual(len(sc), 1, lang)
            self.assertEqual(len(sc[0].find_all(".scene-stage")), 1)
            self.assertGreaterEqual(len(sc[0].find_all(".scene-file")), 3)
            self.assertEqual(len(sc[0].find_all(".scene-original")), 1)
            self.assertEqual(len(sc[0].find_all(".scene-result")), 1)
            self.assertEqual(len(sc[0].find_all("ol.scene-steps")), 1)
            self.assertEqual(len(sc[0].find_all("ol.scene-steps")[0].find_all("li")), 5)


class AC8MotionStatic(unittest.TestCase):
    def test_AC8_durations_and_curve_from_tokens(self):
        css = lib.strip_css_comments(site_css_no_tokens())
        self.assertTrue(css.strip())
        used = set()
        for sel, d in lib.css_rules(css):
            for k, v in d.items():
                if k.startswith(("transition", "animation")):
                    w = re.sub(r"var\([^)]*\)", "", v)
                    self.assertNotRegex(w, r"cubic-bezier|\bease\b|ease-in|ease-out|\blinear\b|\d+m?s\b", f"{sel} {k}: {v}")
                    used.update(re.findall(r"var\((--[\w-]+)\)", v))
        self.assertTrue(used, "no transition or animation uses tokens")
        curves = {u for u in used if re.search(r"ease|curve|easing", u)}
        self.assertEqual(len(curves), 1, f"exactly one entrance curve token expected, got {curves}")
        tok = (site() / "tokens.css").read_text(encoding="utf-8")
        for u in used:
            self.assertIn(u + ":", tok, f"{u} not defined in tokens.css")

    def test_AC8_no_pointer_listeners_outside_sim_in_all_js(self):
        self.assertTrue((site() / "js").is_dir(), "site/js missing")
        for p in js_files():
            rest = re.sub(r"//\s*sim:begin.*?//\s*sim:end", "", p.read_text(encoding="utf-8"), flags=re.S)
            self.assertEqual(re.findall(r"pointermove|mousemove", rest), [], str(p))

    def test_AC8_no_reveal_hiding_rules(self):
        css = lib.strip_css_comments(site_css_no_tokens())
        for sel, d in lib.css_rules(css):
            if d.get("opacity", "").strip() == "0" and "scene" not in sel and "sim" not in sel and ":focus" not in sel:
                self.assertTrue(".js " in sel or ".anim" in sel or "[data-" in sel, f"content hidden by default: {sel}")


class AC9Assets(unittest.TestCase):
    def webps(self):
        fs = sorted((site() / "assets").glob("*.webp"))
        self.assertGreaterEqual(len(fs), 2, "site/assets needs the generated hero and section art as .webp")
        return fs

    def test_AC9_webp_format_and_size(self):
        for f in self.webps():
            b = f.read_bytes()
            self.assertEqual(b[:4], b"RIFF", f.name)
            self.assertEqual(b[8:12], b"WEBP", f.name)
            self.assertLess(len(b), 200 * 1024, f.name)

    def test_AC9_alt_text_both_languages(self):
        names = [f.name for f in self.webps()]
        for lang in LANGS:
            root = parse_file(page_file(lang))
            imgs = [i for i in root.find_all("img") if i.attrs.get("src", "").split("?")[0].endswith(".webp")]
            seen = {i.attrs["src"].split("/")[-1].split("?")[0] for i in imgs}
            self.assertEqual(seen, set(names), f"{lang}: every webp must be used on the home page")
            for i in imgs:
                alt = i.attrs.get("alt", "").strip()
                self.assertGreater(len(alt), 5, f"{lang}: alt for {i.attrs['src']}")
                if lang == "ar":
                    self.assertRegex(alt, "[؀-ۿ]", alt)
                else:
                    self.assertNotRegex(alt, "[؀-ۿ]", alt)
                    self.assertRegex(alt, "[A-Za-z]")


class AC10Finish(unittest.TestCase):
    def test_AC10_no_long_dashes(self):
        bad = []
        for p in [REPO / "site/build.py", REPO / "scripts/tokens-to-css.py", REPO / "design/tokens.json", REPO / "backlog/S-063-site-redesign.md"]:
            if DASH.search(p.read_text(encoding="utf-8")):
                bad.append(str(p))
        for d in ("css", "js"):
            self.assertTrue((REPO / "site" / d).is_dir(), f"site/{d}/ missing")
            for p in (REPO / "site" / d).rglob("*"):
                if p.is_file() and DASH.search(p.read_text(encoding="utf-8")):
                    bad.append(str(p))
        for p in (REPO / "tests/site").glob("test_s063_*.py"):
            if DASH.search(p.read_text(encoding="utf-8")):
                bad.append(str(p))
        for f in html_files():
            if DASH.search(f.read_text(encoding="utf-8")):
                bad.append(str(f.relative_to(site())))
        self.assertEqual(bad, [])

    def test_AC10_changelog_line(self):
        lines = [l for l in (REPO / "CHANGELOG.md").read_text(encoding="utf-8").splitlines() if "S-063" in l]
        self.assertEqual(len(lines), 1, "exactly one CHANGELOG line for S-063")
        self.assertRegex(lines[0], r"[A-Za-z]")
        self.assertRegex(lines[0], "[؀-ۿ]")

    def test_AC10_design_lint_covers_split_css(self):
        self.assertTrue((REPO / "site/css/components").is_dir(), "site/css/components missing")
        r = subprocess.run(["bash", str(REPO / "scripts/design-lint.sh")], cwd=str(REPO), capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stdout)
        planted = TMP / "lint-plant"
        shutil.rmtree(planted, ignore_errors=True)
        (planted / "site/css/components").mkdir(parents=True)
        (planted / "site/css/components/x.css").write_text(".a { box-shadow: 0 0 4px #000; }\n", encoding="utf-8")
        r2 = subprocess.run(["bash", str(REPO / "scripts/design-lint.sh"), str(planted)], capture_output=True, text=True)
        self.assertNotEqual(r2.returncode, 0, "design-lint must flag shadows in site/css/**/*.css")

    def test_AC10_build_reproducible_with_split_sources(self):
        self.assertTrue((site() / "css/components").is_dir(), "site/css/components missing in the build")
        a = built("0")
        time.sleep(2)
        b = built("rep")
        r = subprocess.run(["diff", "-r", str(a / "site"), str(b / "site")], capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stdout[:600])


if __name__ == "__main__":
    unittest.main(verbosity=2)
