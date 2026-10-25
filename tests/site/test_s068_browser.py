"""S-068 browser checks: the film plays, respects reduced motion, never overflows."""
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from lib import Browser, page_path  # noqa: E402

B = None


def setUpModule():
    global B
    B = Browser("0").__enter__()


def tearDownModule():
    B.__exit__(None, None, None)


STATE = """() => { const v = document.querySelector('.film-video'); const r = v.getBoundingClientRect();
  return { paused: v.paused, ready: v.readyState, t: v.currentTime, controls: v.controls, w: r.width, h: r.height,
           src: v.currentSrc.split('/').pop(), scroll: document.documentElement.scrollWidth }; }"""


class AC2Plays(unittest.TestCase):
    def test_AC2_plays_without_errors_and_fits(self):
        for w in (1440, 390):
            pg = B.page(page_path("ar"), width=w, height=900)
            errs = []
            pg.on("pageerror", lambda e: errs.append(str(e)))
            pg.on("console", lambda m: errs.append(m.text) if m.type == "error" else None)
            pg.wait_for_timeout(3500)
            st = pg.evaluate(STATE)
            self.assertFalse(st["paused"], w)
            self.assertGreater(st["t"], 0.5, w)
            self.assertGreaterEqual(st["ready"], 3, w)
            self.assertFalse(st["controls"], w)
            self.assertEqual(st["src"], "hero.webm", w)
            self.assertLessEqual(st["scroll"], w, f"{w}: overflow")
            self.assertAlmostEqual(st["w"] / st["h"], 16 / 9, delta=0.08, msg=f"{w}: ratio {st}")
            self.assertEqual(errs, [], w)
            pg.context.close()

    def test_AC2_reduced_motion_pauses_and_shows_controls(self):
        pg = B.page(page_path("ar"), reduced=True)
        pg.wait_for_timeout(1500)
        st = pg.evaluate(STATE)
        self.assertTrue(st["paused"], st)
        self.assertTrue(st["controls"], st)
        pg.context.close()


if __name__ == "__main__":
    unittest.main(verbosity=1)
