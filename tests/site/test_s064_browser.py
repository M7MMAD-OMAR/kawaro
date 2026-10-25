"""S-064 browser checks (AC2, AC3, AC6). Needs playwright: run without -I (see run_all.sh)."""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from lib import Browser, LANGS, page_path  # noqa: E402

B = None


def setUpModule():
    global B
    B = Browser("0").__enter__()


def tearDownModule():
    B.__exit__(None, None, None)


def box(pg, sel):
    return pg.eval_on_selector(sel, "e => { const r = e.getBoundingClientRect(); return {x: r.x, y: r.y, w: r.width, h: r.height}; }")


def overlap(a, b):
    w = min(a["x"] + a["w"], b["x"] + b["w"]) - max(a["x"], b["x"])
    h = min(a["y"] + a["h"], b["y"] + b["h"]) - max(a["y"], b["y"])
    return max(0, w) * max(0, h)


class AC3Hero(unittest.TestCase):
    def test_AC3_no_overflow_and_no_overlap(self):
        for lang in LANGS:
            for w in (375, 768, 1280, 1440):
                pg = B.page(page_path(lang), width=w, height=800 if w > 400 else 667)
                self.assertLessEqual(pg.evaluate("document.documentElement.scrollWidth"), w, f"{lang} {w}")
                h1 = box(pg, ".hero h1")
                self.assertLessEqual(h1["x"] + h1["w"], w, f"{lang} {w}: h1 overflows")
                self.assertEqual(overlap(box(pg, ".hero-side"), box(pg, ".demo, .film")), 0, f"{lang} {w}: copy over demo")
                self.assertEqual(overlap(h1, box(pg, ".demo, .film")), 0, f"{lang} {w}: h1 over demo")
                if w == 375:
                    cta = box(pg, ".hero a.button-primary")
                    self.assertLessEqual(cta["y"] + cta["h"], 667, f"{lang}: CTA below the first screen")
                if w >= 1280:
                    size = pg.evaluate("parseFloat(getComputedStyle(document.querySelector('.hero h1')).fontSize)")
                    self.assertGreaterEqual(size, 100, f"{lang} {w}: display size")
                    self.assertEqual(pg.evaluate("getComputedStyle(document.querySelector('.hero h1')).fontWeight"), "700")
                pg.context.close()


class AC2Demo(unittest.TestCase):
    def test_AC2_reduced_motion_shows_the_end_state(self):
        for lang in ("en",):  # S-068: the Arabic hero is the film (test_s068_film.py)
            pg = B.page(page_path(lang), reduced=True)
            st = pg.evaluate("""() => ({
                hive: getComputedStyle(document.querySelector('.demo-hive')).display,
                ptr: getComputedStyle(document.querySelector('.demo-pointer')).display,
                res: getComputedStyle(document.querySelector('.demo-result')).opacity,
                anims: document.getAnimations().filter(a => a.playState === 'running').length })""")
            self.assertEqual(st["hive"], "none", lang)
            self.assertEqual(st["ptr"], "none", lang)
            self.assertEqual(st["res"], "1", lang)
            self.assertEqual(st["anims"], 0, f"{lang}: animations still running with reduced motion")
            pg.context.close()

    @unittest.skip("superseded by S-068: the Arabic hero is the product film, so the demo is no longer drawn in both directions")
    def test_AC2_loop_runs_and_mirrors(self):
        xs = {}
        for lang in LANGS:
            pg = B.page(page_path(lang))
            pg.evaluate("""() => document.getAnimations().forEach(a => { if (a.timeline === document.timeline) { a.pause(); a.currentTime = 4700; } })""")
            pick = box(pg, ".demo-pick")
            hot = box(pg, ".demo-hot")
            win = box(pg, ".demo-window")
            self.assertGreater(float(pg.evaluate("getComputedStyle(document.querySelector('.demo-hot')).opacity")), 0.9)
            xs[lang] = ((pick["x"] + pick["w"] / 2 - win["x"]) / win["w"], (hot["x"] + hot["w"] / 2 - win["x"]) / win["w"])
            pg.context.close()
        for i in (0, 1):
            self.assertAlmostEqual(xs["ar"][i], 1 - xs["en"][i], delta=0.02, msg=f"mirror {i}: {xs}")


class AC6Header(unittest.TestCase):
    def test_AC6_header_sticks(self):
        for lang in LANGS:
            pg = B.page(page_path(lang))
            pg.evaluate("window.scrollTo(0, 2400)")
            pg.wait_for_timeout(100)
            self.assertAlmostEqual(box(pg, ".site-header")["y"], 0, delta=0.5)
            cta = box(pg, "header .nav-cta")
            self.assertGreaterEqual(cta["h"], 56)
            pg.context.close()


if __name__ == "__main__":
    unittest.main(verbosity=1)
