"""S-063 browser checks (Playwright, Chromium, own local server). AC1, AC3, AC4, AC6, AC7, AC8.

Run WITHOUT -I and from a neutral cwd (see run_all.sh):  python3 test_s063_browser.py
Scene DOM contract (home page, both languages):
  section#scene                   tall section (>= 4 viewport heights), attribute data-state, one of
                                  folder select open pick result, set by scroll progress p in [0, 1]:
                                  state index = min(4, floor(p * 4)); p = 0, .25, .5, .75, 1
  #scene .scene-stage             position sticky, top 0, exactly the viewport size
  #scene .scene-file              >= 3 files; .scene-file[data-selected] is the chosen one (from "select" on)
  #scene .scene-cell[data-ring]   hive cells, ring 0 is the centre; opacity grows ring by ring in "open"
  #scene .scene-cell[data-picked] exactly one from "pick" on
  #scene .scene-original          the untouched file; #scene .scene-result follows it (next sibling)
  #scene ol.scene-steps > li      5 static numbered steps, visible with reduced motion or JS off
  a.try-it (or button.try-it)     after #scene, tabbable, named, brings #sim into view on Enter
"""
import io
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import lib  # noqa: E402
from lib import Browser, LANGS, page_path  # noqa: E402

B = None
STATES = ["folder", "select", "open", "pick", "result"]
PROGRESS = [0.0, 0.26, 0.51, 0.76, 1.0]  # a hair inside each quarter so rounding cannot flip a state
PAGES = ["", "download", "windows", "privacy", "docs", "releases"]


def setUpModule():
    global B
    B = Browser("0").__enter__()


def tearDownModule():
    B.__exit__(None, None, None)


def scroll_scene(pg, p, settle=900):
    pg.evaluate("""p => { const s = document.querySelector('#scene'); const r = s.getBoundingClientRect();
        const top = r.top + scrollY; const span = s.offsetHeight - innerHeight;
        scrollTo(0, top + span * p); }""", p)
    pg.wait_for_timeout(settle)


def scene_state(pg):
    return pg.get_attribute("#scene", "data-state")


def op(pg, sel):
    return pg.eval_on_selector_all(sel, "els => els.map(e => { let o = 1; for (let n = e; n; n = n.parentElement) o *= parseFloat(getComputedStyle(n).opacity); return o; })")


class AC1Pixels(unittest.TestCase):
    def test_AC1_forced_dark_renders_same_pixels(self):
        for lang in LANGS:
            shots = {}
            for scheme in ("light", "dark"):
                pg = B.page(page_path(lang), scheme=scheme, reduced=True)
                shots[scheme] = [pg.screenshot(full_page=False)]
                pg.evaluate("scrollTo(0, 1400)")
                pg.wait_for_timeout(300)
                shots[scheme].append(pg.screenshot(full_page=False))
                pg.context.close()
            self.assertEqual(shots["light"], shots["dark"], f"{lang}: dark colour scheme changed the render")

    def test_AC1_forced_dark_computed_colours_equal(self):
        js = """() => [...document.querySelectorAll('main *, header *, footer *')].slice(0, 400).map(e => { const s = getComputedStyle(e); return s.color + '|' + s.backgroundColor; })"""
        a = B.page(page_path("en"), scheme="light"); la = a.evaluate(js); a.context.close()
        b = B.page(page_path("en"), scheme="dark"); lb = b.evaluate(js); b.context.close()
        self.assertGreater(len(la), 50)
        self.assertEqual(la, lb)
        c = B.page(page_path("en"), scheme="dark")
        self.assertEqual(c.evaluate("getComputedStyle(document.documentElement).colorScheme"), "light")
        c.context.close()


class AC3Sharp(unittest.TestCase):
    JS = """() => {
      const bad = [];
      for (const e of document.querySelectorAll('body, body *')) {
        if (e.tagName === 'SOURCE') continue; // S-068: not rendered, so it has no computed style
        const s = getComputedStyle(e);
        const id = e.tagName + '.' + (e.getAttribute('class') || '');
        for (const k of ['borderTopLeftRadius', 'borderTopRightRadius', 'borderBottomLeftRadius', 'borderBottomRightRadius'])
          if (parseFloat(s[k]) !== 0) bad.push(id + ' radius ' + s[k]);
        if (s.boxShadow !== 'none') bad.push(id + ' box-shadow');
        if (s.textShadow !== 'none') bad.push(id + ' text-shadow');
        if (/gradient/.test(s.backgroundImage)) bad.push(id + ' gradient');
        if (/drop-shadow/.test(s.filter)) bad.push(id + ' drop-shadow');
        const sides = ['Top', 'Right', 'Bottom', 'Left'].map(d => parseFloat(s['border' + d + 'Width']) > 0 && s['border' + d + 'Style'] !== 'none');
        const n = sides.filter(Boolean).length;
        if (n !== 0 && n !== 4) bad.push(id + ' one or more sided border ' + sides);
      }
      return bad;
    }"""

    def test_AC3_every_element_every_page(self):
        for lang in LANGS:
            for page in PAGES:
                for w in (1280, 375):
                    pg = B.page(page_path(lang, page), width=w)
                    self.assertEqual(pg.evaluate(self.JS)[:10], [], f"{lang}/{page} {w}")
                    pg.context.close()

    @unittest.skip("superseded by S-064 AC1: the pinned scene is removed")
    def test_AC3_scene_and_hover_states_also_sharp(self):
        pg = B.page(page_path("en"))
        for p in PROGRESS:
            scroll_scene(pg, p, settle=300)
            self.assertEqual(pg.evaluate(self.JS)[:10], [], f"progress {p}")
        pg.context.close()


class AC4Big(unittest.TestCase):
    def test_AC4_type_sizes(self):
        for lang in LANGS:
            for w, body_min, h2_min in ((1280, 20, 56), (375, 18, 36)):
                pg = B.page(page_path(lang), width=w, height=800 if w > 400 else 667)
                self.assertGreaterEqual(float(pg.evaluate("getComputedStyle(document.body).fontSize").replace("px", "")), body_min)
                small = pg.evaluate("""min => [...document.querySelectorAll('main p, main li')].filter(e => e.getClientRects().length && !e.closest('svg') && !e.classList.contains('sr-only') && e.textContent.trim())
                    .map(e => [parseFloat(getComputedStyle(e).fontSize), e.tagName + '.' + e.className + ' ' + e.textContent.trim().slice(0, 20)]).filter(x => x[0] < min)""", body_min)
                self.assertEqual(small, [], f"{lang} {w}: body text below {body_min}px")
                if w == 1280:
                    h1 = pg.evaluate("parseFloat(getComputedStyle(document.querySelector('main h1')).fontSize)")
                    self.assertGreaterEqual(h1, 72)
                secs = pg.evaluate("""() => [...document.querySelectorAll('main > section:not(.hero)')].map(s => [s.id || s.className, [...s.querySelectorAll('h2')].map(h => parseFloat(getComputedStyle(h).fontSize))])""")
                self.assertGreaterEqual(len(secs), 4, f"{lang} {w}: sections")  # S-065: four sections after the hero
                for name, h2s in secs:
                    self.assertEqual(len(h2s), 1, f"{lang} {w}: section {name} needs exactly one h2")
                    self.assertGreaterEqual(h2s[0], h2_min, f"{lang} {w}: {name} h2")
                pg.context.close()

    def test_AC4_targets_at_least_56_high(self):
        for lang in LANGS:
            for page in ("", "download"):
                for w in (1280, 375):
                    pg = B.page(page_path(lang, page), width=w, height=800 if w > 400 else 667)
                    small = pg.evaluate("""() => [...document.querySelectorAll('a, button, input, summary, [role=button]')]
                        .filter(e => e.getClientRects().length && !e.classList.contains('sr-only') && getComputedStyle(e).visibility !== 'hidden'
                            && !(e.closest('p, li > p') && getComputedStyle(e).display === 'inline'))
                        .map(e => { const r = e.getBoundingClientRect(); return [Math.round(r.height), e.tagName + '.' + e.className + ' ' + (e.textContent || '').trim().slice(0, 15)]; })
                        .filter(x => x[0] < 56)""")
                    self.assertEqual(small, [], f"{lang}/{page} {w}")
                    self.assertLessEqual(pg.evaluate("document.documentElement.scrollWidth"), w)
                    pg.context.close()


@unittest.skip("superseded by S-064 AC1: the pinned scene is removed")
class AC6Scene(unittest.TestCase):
    def test_AC6_stage_is_pinned_and_covers_viewport(self):
        for lang in LANGS:
            pg = B.page(page_path(lang))
            self.assertEqual(pg.locator("section#scene").count(), 1)
            st = pg.eval_on_selector("#scene .scene-stage", "e => { const s = getComputedStyle(e); return [s.position, s.top]; }")
            self.assertEqual(st, ["sticky", "0px"])
            h = pg.evaluate("[document.querySelector('#scene').offsetHeight, innerHeight]")
            self.assertGreaterEqual(h[0], 4 * h[1])
            for p in (0.0, 0.5, 1.0):
                scroll_scene(pg, p, settle=200)
                r = pg.eval_on_selector("#scene .scene-stage", "e => { const r = e.getBoundingClientRect(); return [r.x, r.y, r.width, r.height]; }")
                self.assertEqual([round(v) for v in r], [0, 0, 1280, 800], f"{lang} p={p}")
            pg.context.close()

    def test_AC6_states_at_0_25_50_75_100(self):
        for lang in LANGS:
            pg = B.page(page_path(lang))
            for p, want in zip(PROGRESS, STATES):
                scroll_scene(pg, p, settle=500)
                self.assertEqual(scene_state(pg), want, f"{lang} p={p}")
            pg.context.close()

    def test_AC6_scene_content_per_state(self):
        pg = B.page(page_path("en"))
        counts = {}
        for p, state in zip(PROGRESS, STATES):
            scroll_scene(pg, p)
            sel = pg.locator("#scene .scene-file[data-selected]").count()
            cells = sum(1 for o in op(pg, "#scene .scene-cell") if o > 0.5)
            picked = pg.locator("#scene .scene-cell[data-picked]").count()
            res = op(pg, "#scene .scene-result")[0]
            orig = op(pg, "#scene .scene-original")[0]
            counts[state] = cells
            self.assertGreaterEqual(pg.locator("#scene .scene-file").count(), 3)
            self.assertEqual(sel, 0 if state == "folder" else 1, state)
            self.assertEqual(picked, 1 if state in ("pick", "result") else 0, state)
            self.assertGreater(orig, 0.99, f"{state}: the original stays visible and untouched")
            if state == "result":
                self.assertGreater(res, 0.99)
            else:
                self.assertLess(res, 0.05)
            if state in ("folder", "select"):
                self.assertEqual(cells, 0, state)
        self.assertGreater(counts["pick"], counts["open"], "the hive opens ring by ring: fewer cells at 51% than at 76%")
        self.assertEqual(pg.eval_on_selector("#scene .scene-original", "e => e.nextElementSibling && e.nextElementSibling.classList.contains('scene-result')"), True)
        pg.context.close()

    def test_AC6_rtl_mirror(self):
        got = {}
        for lang in LANGS:
            pg = B.page(page_path(lang))
            got[lang] = []
            for p in (0.0, 0.76):
                scroll_scene(pg, p)
                got[lang].append(pg.evaluate("""() => [...document.querySelectorAll('#scene .scene-file')].map(e => {
                    const r = e.getBoundingClientRect(); const m = new DOMMatrix(getComputedStyle(e).transform);
                    return [r.x + r.width / 2, m.e]; })"""))
            pg.context.close()
        for i in range(2):
            self.assertEqual(len(got["ar"][i]), len(got["en"][i]))
            self.assertGreaterEqual(len(got["en"][i]), 3)
            for (cxa, txa), (cxe, txe) in zip(got["ar"][i], got["en"][i]):
                self.assertAlmostEqual(cxa + cxe, 1280, delta=2)
                self.assertAlmostEqual(txa + txe, 0, delta=0.5)
        # at least the files must really sit off centre so mirroring is observable
        self.assertTrue(any(abs(cx - 640) > 40 for cx, _ in got["en"][0]))

    def test_AC6_only_transform_and_opacity_change(self):
        pg = B.page(page_path("en"))
        js = """() => { const keep = ['color', 'backgroundColor', 'width', 'height', 'top', 'left', 'right', 'bottom', 'marginTop', 'marginLeft', 'marginRight',
              'marginBottom', 'fontSize', 'clipPath', 'filter', 'display', 'visibility', 'borderTopWidth', 'position', 'zIndex'];
            return [...document.querySelectorAll('#scene, #scene *')].map(e => { const s = getComputedStyle(e); return keep.map(k => s[k]).join('|'); }); }"""
        scroll_scene(pg, 0.0)
        a = pg.evaluate(js)
        scroll_scene(pg, 1.0)
        b = pg.evaluate(js)
        self.assertEqual(len(a), len(b))
        diff = [i for i, (x, y) in enumerate(zip(a, b)) if x != y]
        self.assertEqual(diff, [], "only transform and opacity may change with scroll")
        props = pg.evaluate("""() => [...document.querySelectorAll('#scene, #scene *')].map(e => getComputedStyle(e).transitionProperty).filter(v => v !== 'all')""")
        for v in props:
            for part in v.split(","):
                self.assertIn(part.strip(), ("transform", "opacity", "none"), v)
        pg.context.close()


@unittest.skip("superseded by S-064: no scene; the hero links to #sim and test_browser.py AC7 covers the keyboard demo")
class AC7Access(unittest.TestCase):
    def test_AC7_try_it_control_after_scene_keyboard_reachable(self):
        for lang in LANGS:
            pg = B.page(page_path(lang))
            ctl = pg.locator("#scene ~ * .try-it, #scene ~ .try-it, .try-it")
            self.assertEqual(ctl.count(), 1, lang)
            order = pg.evaluate("""() => { const s = document.querySelector('#scene'), c = document.querySelector('.try-it');
                return !!(s.compareDocumentPosition(c) & Node.DOCUMENT_POSITION_FOLLOWING); }""")
            self.assertTrue(order, "control comes after the scene")
            self.assertGreaterEqual(int(ctl.evaluate("e => e.tabIndex")), 0)
            self.assertTrue((ctl.get_attribute("aria-label") or ctl.inner_text()).strip())
            self.assertGreaterEqual(ctl.bounding_box()["height"], 56)
            ctl.focus()
            pg.keyboard.press("Enter")
            pg.wait_for_timeout(800)
            r = pg.eval_on_selector("#sim", "e => { const r = e.getBoundingClientRect(); return r.top < innerHeight && r.bottom > 0; }")
            self.assertTrue(r, f"{lang}: #sim in view after Enter")
            pg.context.close()

    def test_AC7_existing_sim_still_works_by_keyboard(self):
        expect = {"image": 12, "video": 10, "document": 2, "audio": 4, "pdfs": 5}
        for kind, n in expect.items():
            pg = B.page(page_path("en"))
            pg.locator(f'#sim .sim-file[data-kind="{kind}"]').focus()
            pg.keyboard.press("Enter")
            pg.wait_for_function(f"document.querySelectorAll('#sim-hive [data-cell]').length === {n}", timeout=2000)
            pg.locator("#sim-hive [data-cell]").first.focus()
            pg.keyboard.press("Enter")
            self.assertEqual(pg.locator("#sim-files li[data-result]").count(), 1, kind)
            self.assertGreaterEqual(pg.locator("#sim-files li[data-original]").count(), 1, kind)
            self.assertEqual(pg.locator(".try-it").count(), 1, "sim stays behind the try-it control")
            pg.context.close()

    def test_AC7_reduced_motion_static_numbered_steps(self):
        for lang in LANGS:
            pg = B.page(page_path(lang), reduced=True)
            steps = pg.locator("#scene ol.scene-steps > li")
            self.assertEqual(steps.count(), 5, lang)
            for i in range(5):
                self.assertTrue(steps.nth(i).is_visible(), f"{lang} step {i}")
                self.assertTrue(steps.nth(i).inner_text().strip())
            self.assertEqual(pg.eval_on_selector("#scene ol.scene-steps", "e => getComputedStyle(e).listStyleType !== 'none' || e.querySelector('li > .num, li > [data-num]') !== null"), True)
            pg.evaluate("scrollTo(0, document.querySelector('#scene').offsetTop + 600)")
            pg.wait_for_timeout(400)
            self.assertEqual(scene_state(pg) in (None, "result"), True, "reduced motion shows the final state, not a scroll progress")
            pg.context.close()

    def test_AC7_js_off_static_numbered_steps(self):
        for lang in LANGS:
            pg = B.page(page_path(lang), js=False)
            steps = pg.locator("#scene ol.scene-steps > li")
            self.assertEqual(steps.count(), 5)
            for i in range(5):
                self.assertTrue(steps.nth(i).is_visible(), f"{lang} step {i}")
            pg.context.close()


def is_blank(png):
    from PIL import Image
    im = Image.open(io.BytesIO(png)).convert("RGB").resize((160, 100))
    px = list(im.get_flattened_data()) if hasattr(im, 'get_flattened_data') else list(im.getdata())
    top = max(set(px), key=px.count)
    other = sum(1 for p in px if p != top)
    return other / len(px) < 0.01


class AC8Feedback(unittest.TestCase):
    def test_AC8_no_blank_viewport_every_100px(self):
        pg0 = B.page(page_path("en"))
        self.assertEqual(pg0.locator(".demo").count(), 1, "hero demo missing (S-064 replaced the scene)")
        pg0.context.close()
        for lang, w, h in (("en", 1280, 800), ("ar", 375, 667)):
            pg = B.page(page_path(lang), width=w, height=h)
            total = pg.evaluate("document.documentElement.scrollHeight")
            blanks = []
            y = 0
            while y < total:
                pg.evaluate("y => scrollTo(0, y)", y)
                pg.wait_for_timeout(60)
                if is_blank(pg.screenshot()):
                    blanks.append(y)
                y += 100
            self.assertEqual(blanks, [], f"{lang} {w}: blank viewport at scrollY {blanks[:10]}")
            pg.context.close()

    def test_AC8_content_visible_with_js_off(self):
        for lang in LANGS:
            pg = B.page(page_path(lang), js=False)
            self.assertEqual(pg.locator(".demo, .film").count(), 1, "hero demo or film missing")
            hidden = pg.evaluate("""() => [...document.querySelectorAll('main h1, main h2, main p')].filter(e => e.textContent.trim() && !e.closest('.sim-nojs'))
                .filter(e => { let o = 1; for (let n = e; n; n = n.parentElement) { const s = getComputedStyle(n); o *= parseFloat(s.opacity); if (s.visibility === 'hidden') o = 0; } return o < 0.99; })
                .map(e => e.tagName + '.' + e.className)""")
            self.assertEqual(hidden, [], lang)
            pg.context.close()

    def test_AC8_hover_and_focus_change_fill_at_once(self):
        pg = B.page(page_path("en"))
        btn = pg.locator(".hero .hex-button").first
        self.assertEqual(btn.count(), 1, "hero uses the hex_button component")
        fill = lambda: btn.evaluate("e => getComputedStyle(e).backgroundColor + '|' + getComputedStyle(e).getPropertyValue('--fill')")
        rest = fill()
        btn.hover()
        pg.wait_for_timeout(60)
        self.assertNotEqual(fill(), rest, "hover must change the fill within 60 ms")
        pg.mouse.move(5, 5)
        pg.wait_for_timeout(500)
        self.assertEqual(fill(), rest)
        btn.focus()
        pg.keyboard.press("Shift+Tab")
        pg.keyboard.press("Tab")
        pg.wait_for_timeout(60)
        self.assertNotEqual(fill(), rest, "keyboard focus must change the fill")
        self.assertNotEqual(btn.evaluate("e => getComputedStyle(e).outlineStyle"), "none")
        pg.context.close()

    @unittest.skip("superseded by S-064: no scene; test_s064_browser.py checks reduced motion on the hero demo")
    def test_AC8_reduced_motion_zero_durations_in_scene(self):
        pg = B.page(page_path("en"), reduced=True)
        self.assertGreater(pg.locator("#scene *").count(), 5, "scene missing")
        bad = pg.evaluate("""() => [...document.querySelectorAll('#scene, #scene *')].map(e => { const s = getComputedStyle(e);
            const nz = v => v.split(',').some(x => parseFloat(x) > 0);
            return (nz(s.transitionDuration) || nz(s.animationDuration)) ? e.tagName + '.' + e.className : null; }).filter(Boolean)""")
        self.assertEqual(bad, [])
        pg.context.close()


if __name__ == "__main__":
    unittest.main(verbosity=2)
