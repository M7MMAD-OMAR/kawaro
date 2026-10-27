"""S-088 browser checks: the film plays beside the download buttons, reduced
motion shows the poster and the numbered steps, the sample downloads through
the browser's own flow, and the pixel comparison hides the film first (the
known convention for film elements).

Run WITHOUT -I from a neutral cwd: python3 tests/site/test_s088_browser.py
"""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from lib import Browser, page_path  # noqa: E402

B = None

# The film is the only moving pixel source and its decoded frame can differ
# between two loads, so hide it before any pixel comparison (s063 convention).
FREEZE_FILM = """() => { for (const v of document.querySelectorAll('video')) { v.pause(); v.style.visibility = 'hidden'; } }"""

STATE = """() => { const v = document.querySelector('.film-video'); const r = v.getBoundingClientRect();
  return { paused: v.paused, ready: v.readyState, t: v.currentTime, controls: v.controls, w: r.width, h: r.height,
           src: v.currentSrc.split('/').pop(), scroll: document.documentElement.scrollWidth,
           steps: document.querySelectorAll('.flow li').length,
           walk: document.querySelectorAll('.walk li').length,
           still: getComputedStyle(document.querySelector('.film-still')).display }; }"""


def setUpModule():
    global B
    B = Browser("0").__enter__()


def tearDownModule():
    B.__exit__(None, None, None)


class AC1FilmBrowser(unittest.TestCase):
    def test_AC1_film_plays_with_native_controls_and_fits(self):
        for lang, stem in (("ar", "hero.webm"), ("en", "hero-en.webm")):
            for w in (1280, 390):
                pg = B.page(page_path(lang, "download"), width=w, height=900)
                errs = []
                pg.on("pageerror", lambda e: errs.append(str(e)))
                pg.on("console", lambda m: errs.append(m.text) if m.type == "error" else None)
                # The film convention (S-068) is play only while it is on screen,
                # and at 390px the D7 order can leave it below the fold, so bring
                # it into view before asserting playback (tester amendment of
                # 2026-10-27).
                pg.evaluate("() => document.querySelector('.film-video').scrollIntoView({block: 'center'})")
                pg.wait_for_timeout(3500)
                st = pg.evaluate(STATE)
                self.assertTrue(st["controls"], f"{lang} {w}: native controls")
                self.assertFalse(st["paused"], f"{lang} {w}")
                self.assertGreater(st["t"], 0.5, f"{lang} {w}")
                self.assertGreaterEqual(st["ready"], 3, f"{lang} {w}")
                self.assertEqual(st["src"], stem, f"{lang} {w}")
                self.assertEqual((st["steps"], st["walk"]), (4, 5), f"{lang} {w}: caption and walkthrough")
                self.assertLessEqual(st["scroll"], w, f"{lang} {w}: overflow")
                self.assertAlmostEqual(st["w"] / st["h"], 16 / 9, delta=0.08, msg=f"{lang} {w}: ratio {st}")
                self.assertEqual(errs, [], f"{lang} {w}")
                pg.context.close()

    def test_AC1_reduced_motion_shows_the_poster_and_the_numbered_steps(self):
        for lang in ("ar", "en"):
            pg = B.page(page_path(lang, "download"), reduced=True)
            pg.wait_for_timeout(1200)
            st = pg.evaluate(STATE)
            self.assertEqual(st["still"], "block", f"{lang}: the poster shows")
            self.assertEqual(st["steps"], 4, lang)
            vis = pg.evaluate("""() => {
                const v = document.querySelector('.film-video');
                const s = document.querySelector('.film-still');
                const f = document.querySelector('figcaption');
                const shown = e => { const r = e.getBoundingClientRect(); return r.width > 0 && r.height > 0; };
                return { video: shown(v), still: shown(s), caption: shown(f) }; }""")
            self.assertFalse(vis["video"], f"{lang}: no moving film")
            self.assertTrue(vis["still"], lang)
            self.assertTrue(vis["caption"], lang)
            pg.context.close()


class AC3DownloadFlow(unittest.TestCase):
    def test_AC3_try_button_downloads_the_sample_through_the_browser(self):
        for lang in ("ar", "en"):
            pg = B.page(page_path(lang, "download"))
            with pg.expect_download() as info:
                pg.click(".try-box a")
            dl = info.value
            self.assertEqual(dl.suggested_filename, "kawaro-sample.png", lang)
            pg.context.close()

    def test_AC3_demo_and_side_do_not_overlap(self):
        for lang in ("ar", "en"):
            for w in (1280, 375):
                pg = B.page(page_path(lang, "download"), width=w)
                boxes = pg.evaluate("""() => {
                    const box = s => { const r = document.querySelector(s).getBoundingClientRect();
                        return {x: r.x, y: r.y, width: r.width, height: r.height}; };
                    return { demo: box('.dl-demo'), side: box('.dl-side') }; }""")
                a, b = boxes["demo"], boxes["side"]
                ox = min(a["x"] + a["width"], b["x"] + b["width"]) - max(a["x"], b["x"])
                oy = min(a["y"] + a["height"], b["y"] + b["height"]) - max(a["y"], b["y"])
                self.assertEqual(max(ox, 0) * max(oy, 0), 0, f"{lang} {w}: the film column overlaps the buttons")
                pg.context.close()


class AC5Pixels(unittest.TestCase):
    def test_AC5_forced_dark_renders_same_pixels_with_the_film_hidden(self):
        for lang in ("ar", "en"):
            shots = {}
            for scheme in ("light", "dark"):
                pg = B.page(page_path(lang, "download"), scheme=scheme, reduced=True)
                pg.evaluate(FREEZE_FILM)
                shots[scheme] = [pg.screenshot(full_page=False)]
                pg.evaluate("scrollTo(0, 1200)")
                pg.wait_for_timeout(300)
                shots[scheme].append(pg.screenshot(full_page=False))
                pg.context.close()
            self.assertEqual(shots["light"], shots["dark"], f"{lang}: dark colour scheme changed the render")


if __name__ == "__main__":
    unittest.main(verbosity=1)
