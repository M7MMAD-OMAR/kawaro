"""S-061 browser checks (Playwright, Chromium, headless, own local server).

Run WITHOUT -I (the user site holds playwright) and from a neutral cwd:
  python3 tests/site/test_browser.py
DOM contract for the simulation (home page, both languages):
  section#sim                       the section
  #sim-zone                         drop zone (tabbable, aria-label)
  #sim .sim-file[data-kind]         5 samples: video image document audio pdfs; draggable,
                                    tabbable, aria-label; data-name = file name (clip.mov, ...)
  #sim-hive                         hive area, hidden until a sample is chosen
  #sim-hive [data-cell]             cells (tabbable, aria-label); first video cell data-cell="mp4"
  #sim-centre                       centre cell text, "One video" for the video kind (en)
  #sim-files li                     folder list; result row li[data-result], original li[data-original]
  #stage-live                       live region announcing the result file name
  #sim .sim-nojs                    text shown with JS off ("turn on JavaScript to try it")
"""
import re
import sys
import time
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import lib  # noqa: E402
from lib import Browser, LANGS, WIDTHS, SUPPORT, page_path  # noqa: E402

B = None


def setUpModule():
    global B
    B = Browser("0").__enter__()


def tearDownModule():
    B.__exit__(None, None, None)


def rect_overlap(a, b):
    w = min(a["x"] + a["width"], b["x"] + b["width"]) - max(a["x"], b["x"])
    h = min(a["y"] + a["height"], b["y"] + b["height"]) - max(a["y"], b["y"])
    return max(0, w) * max(0, h)


def rects(pg, sel):
    return pg.eval_on_selector_all(sel, """els => els.filter(e => e.getBoundingClientRect().width > 0)
        .map(e => { const r = e.getBoundingClientRect(); return {x: r.x, y: r.y, width: r.width, height: r.height, t: (e.textContent||'').trim().slice(0,20)}; })""")


class AC1HeroBrowser(unittest.TestCase):
    def test_AC1_primary_swaps_with_user_agent(self):
        WIN = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/120 Safari/537.36"
        LIN = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/120 Safari/537.36"
        errors = []

        def run():
            # A second sync Playwright cannot start inside the module browser's
            # event loop, so the published build is driven from its own thread.
            try:
                b1 = Browser("1")
                with b1:
                    for lang, win_word, lin_word in (("en", "Windows", "Linux"), ("ar", "ويندوز", "لينكس")):
                        got = {}
                        for name, ua in (("win", WIN), ("lin", LIN)):
                            pg = b1.page(page_path(lang), user_agent=ua)
                            el = pg.locator(".hero a.button-primary")
                            got[name] = (el.inner_text(), el.get_attribute("href"))
                            pg.context.close()
                        self.assertIn(win_word, got["win"][0], got)
                        self.assertTrue(got["win"][1].endswith((".msi", ".zip")), got)
                        self.assertIn(lin_word, got["lin"][0], got)
                        self.assertTrue(got["lin"][1].endswith((".deb", ".rpm")), got)
            except BaseException as e:  # noqa: BLE001
                errors.append(e)

        import threading
        th = threading.Thread(target=run)
        th.start()
        th.join()
        if errors:
            raise errors[0]

    @unittest.skip("superseded by S-064 (new hero, 96px section rhythm, Download button at the header end); see test_s064_browser.py")
    def test_AC1_js_off_label_and_href(self):
        for lang, word in (("en", "Download"), ("ar", "تنزيل")):
            pg = B.page(page_path(lang), js=False)
            el = pg.locator(".hero a.button-primary")
            self.assertEqual(el.inner_text().strip(), word)
            self.assertTrue(el.get_attribute("href").endswith("download/"))
            pg.context.close()

    @unittest.skip("superseded by S-064 (new hero, 96px section rhythm, Download button at the header end); see test_s064_browser.py")
    def test_AC1_no_overlap_hero_copy_and_hive(self):
        for lang in LANGS:
            for w in WIDTHS:
                pg = B.page(page_path(lang), width=w)
                copy = rects(pg, ".hero-copy > *")
                svgs = rects(pg, ".hero svg:not(.os-ico)")
                self.assertGreater(len(copy), 0)
                for c in copy:
                    for s in svgs:
                        self.assertEqual(rect_overlap(c, s), 0, f"{lang} {w}: {c['t']}")
                pg.context.close()


class AC2Computed(unittest.TestCase):
    @unittest.skip("superseded by S-064 (new hero, 96px section rhythm, Download button at the header end); see test_s064_browser.py")
    def test_AC2_section_wrap_and_header_gap(self):
        for lang in LANGS:
            for path in (page_path(lang), page_path(lang, "download")):
                for w, want in ((1280, "64px"), (375, "48px")):
                    pg = B.page(path, width=w)
                    pads = pg.eval_on_selector_all("section.sec", "els => els.map(e => { const s = getComputedStyle(e); return [s.paddingTop, s.paddingBottom]; })")
                    self.assertGreater(len(pads), 0)
                    for p in pads:
                        self.assertEqual(p, [want, want], f"{path} {w}")
                    wraps = pg.eval_on_selector_all(".wrap", "els => els.map(e => { const s = getComputedStyle(e); return [s.maxWidth, s.paddingInlineStart, s.paddingInlineEnd]; })")
                    self.assertEqual(len({tuple(x[:1]) for x in wraps}), 1, f"wrap max-width differs {path}")
                    if w == 375:
                        for x in wraps:
                            self.assertEqual(x[1:], ["16px", "16px"], path)
                    if w == 1280:
                        r = sorted(rects(pg, ".site-nav > *"), key=lambda r: r["x"])
                        self.assertGreaterEqual(len(r), 3)
                        for a, b in zip(r, r[1:]):
                            self.assertAlmostEqual(b["x"] - (a["x"] + a["width"]), 8, delta=0.01, msg=f"{path} header gap")
                    pg.context.close()


class AC3Header(unittest.TestCase):
    def test_AC3_no_overflow_no_overlap_one_support(self):
        for lang in LANGS:
            heights = {}
            for page in ("", "download"):
                for w in WIDTHS:
                    pg = B.page(page_path(lang, page), width=w)
                    over = pg.eval_on_selector_all("header, header *", "els => els.filter(e => e.scrollWidth > e.clientWidth + 0).map(e => e.tagName + '.' + e.className)")
                    self.assertEqual(over, [], f"{lang}/{page} {w}")
                    links = rects(pg, "header a")
                    for i, a in enumerate(links):
                        for b in links[i + 1:]:
                            self.assertEqual(rect_overlap(a, b), 0, f"{lang}/{page} {w}: {a['t']} / {b['t']}")
                    self.assertEqual(pg.locator(f'header a[href="{SUPPORT}"]').count(), 1)
                    heights.setdefault(w, set()).add(pg.locator("header").bounding_box()["height"])
                    pg.context.close()
            for w, hs in heights.items():
                self.assertEqual(len(hs), 1, f"{lang} {w}: header height differs between pages {hs}")


class AC4Hover(unittest.TestCase):
    def test_AC4_mouse_move_changes_no_transform_or_filter(self):
        pg = B.page(page_path("en"))
        snap = lambda: pg.eval_on_selector_all(".hero *", "els => els.map(e => { const s = getComputedStyle(e); return s.transform + '|' + s.filter; })")
        before = snap()
        for x, y in ((100, 200), (500, 300), (900, 400), (300, 600)):
            pg.mouse.move(x, y, steps=4)
        pg.wait_for_timeout(200)
        self.assertEqual(snap(), before)
        pg.context.close()


class Sim:
    EXPECT = {"image": 12, "video": 10, "document": 2, "audio": 4, "pdfs": 5}

    @staticmethod
    def open_by_key(pg, kind, key="Enter"):
        f = pg.locator(f'#sim .sim-file[data-kind="{kind}"]')
        f.focus()
        pg.evaluate("""() => { window.__t = {}; addEventListener('keydown', () => { window.__t.k = performance.now(); }, true);
            const seen = () => { const h = document.querySelector('#sim-hive'); if (h && h.getClientRects().length && h.querySelectorAll('[data-cell]').length && window.__t.k && !window.__t.v) window.__t.v = performance.now(); if (!window.__t.v) requestAnimationFrame(seen); };
            requestAnimationFrame(seen); }""")
        pg.keyboard.press(key)
        pg.wait_for_function("window.__t.v", timeout=2000)
        return pg.evaluate("window.__t.v - window.__t.k")


class AC7Sim(unittest.TestCase):
    def test_AC7_structure(self):
        for lang in LANGS:
            pg = B.page(page_path(lang))
            self.assertEqual(pg.locator("section#sim").count(), 1)
            self.assertEqual(pg.locator("#sim-zone").count(), 1)
            kinds = pg.eval_on_selector_all("#sim .sim-file", "els => els.map(e => e.dataset.kind)")
            self.assertEqual(sorted(kinds), sorted(Sim.EXPECT))
            names = pg.eval_on_selector_all("#sim .sim-file", "els => els.map(e => e.dataset.name)")
            for n in ("clip.mov", "photo.png", "report.docx", "song.wav"):
                self.assertIn(n, names)
            self.assertEqual(pg.locator("#sim-hive [data-cell]").count(), 0, "hive starts empty or hidden")
            pg.context.close()

    def test_AC7_keyboard_each_kind_cell_counts(self):
        for kind, n in Sim.EXPECT.items():
            pg = B.page(page_path("en"))
            dt = Sim.open_by_key(pg, kind)
            self.assertLess(dt, 150, f"{kind}: hive took {dt} ms")
            self.assertEqual(pg.locator("#sim-hive [data-cell]").count(), n, kind)
            pg.context.close()

    def test_AC7_video_cells_and_centre(self):
        pg = B.page(page_path("en"))
        Sim.open_by_key(pg, "video")
        cells = pg.locator("#sim-hive [data-cell]")
        self.assertEqual(cells.first.get_attribute("data-cell"), "mp4")
        self.assertIn("MP4", cells.first.inner_text())
        self.assertEqual(pg.locator("#sim-centre").inner_text().strip(), "One video")
        pg.context.close()

    def test_AC7_space_also_opens(self):
        pg = B.page(page_path("en"))
        Sim.open_by_key(pg, "video", key="Space")
        self.assertEqual(pg.locator("#sim-hive [data-cell]").count(), 10)
        pg.context.close()

    def test_AC7_drag_clip_onto_zone(self):
        pg = B.page(page_path("en"))
        pg.drag_and_drop('#sim .sim-file[data-kind="video"]', "#sim-zone")
        pg.wait_for_function("document.querySelectorAll('#sim-hive [data-cell]').length === 10", timeout=2000)
        pg.context.close()

    def test_AC7_pick_cell_adds_result_after_original_and_escape_returns_focus(self):
        pg = B.page(page_path("en"))
        Sim.open_by_key(pg, "video")
        pg.locator("#sim-hive [data-cell]").first.focus()
        pg.keyboard.press("Enter")
        rows = pg.locator("#sim-files li")
        texts = [t.strip() for t in rows.all_inner_texts()]
        i_orig = next(i for i, t in enumerate(texts) if "clip.mov" in t)
        i_res = next(i for i, t in enumerate(texts) if "clip.mp4" in t)
        self.assertGreater(i_res, i_orig)
        self.assertIn("Original, untouched", texts[i_orig])
        self.assertEqual(pg.locator("#sim-files li[data-original]").count(), 1)
        self.assertEqual(pg.locator("#sim-files li[data-result]").count(), 1)
        self.assertIn("clip.mp4", pg.locator("#stage-live").inner_text())
        pg.keyboard.press("Escape")
        self.assertFalse(pg.locator("#sim-hive [data-cell]").first.is_visible() if pg.locator("#sim-hive [data-cell]").count() else False)
        self.assertEqual(pg.evaluate("document.activeElement.dataset.kind"), "video")
        pg.context.close()

    def test_AC7_mouse_click_cell(self):
        pg = B.page(page_path("en"))
        Sim.open_by_key(pg, "image")
        pg.locator("#sim-hive [data-cell]").first.click()
        self.assertEqual(pg.locator("#sim-files li[data-result]").count(), 1)
        pg.context.close()

    def test_AC7_zero_network_requests_after_load(self):
        for lang in LANGS:
            pg = B.page(page_path(lang))
            n = len(pg.requests)
            for kind in Sim.EXPECT:
                Sim.open_by_key(pg, kind)
                pg.locator("#sim-hive [data-cell]").first.focus()
                pg.keyboard.press("Enter")
                pg.keyboard.press("Escape")
            # S-059 D6: the tab icon is a same origin file now (the CSP has no data:),
            # and Chromium fetches it after load; that one request leaves nothing.
            late = [u for u in pg.requests[n:] if u != B.base + "/favicon.svg"]
            self.assertEqual(late, [])
            outside = [u for u in pg.requests if not u.startswith(B.base)]
            self.assertEqual(outside, [])
            pg.context.close()


class AC8SimAccess(unittest.TestCase):
    def test_AC8_tab_reachable_named_and_focus_ring(self):
        for lang in LANGS:
            pg = B.page(page_path(lang))
            self.assertEqual(pg.evaluate("getComputedStyle(document.documentElement).getPropertyValue('--border-focus').trim()"), "2px")
            Sim.open_by_key(pg, "video")
            sel = ["#sim-zone", "#sim .sim-file", "#sim-hive [data-cell]"]
            for s in sel:
                els = pg.locator(s)
                self.assertGreater(els.count(), 0, s)
                for i in range(els.count()):
                    e = els.nth(i)
                    self.assertGreaterEqual(int(e.evaluate("e => e.tabIndex")), 0, f"{s} #{i} not tabbable")
                    name = e.get_attribute("aria-label") or e.inner_text().strip()
                    self.assertTrue(name, f"{s} #{i} has no accessible name")
            pg.locator("#sim-hive [data-cell]").nth(1).focus()
            pg.keyboard.press("Shift+Tab")
            pg.keyboard.press("Tab")
            ring = pg.evaluate("(() => { const s = getComputedStyle(document.activeElement); return [s.outlineStyle, s.outlineWidth]; })()")
            self.assertNotEqual(ring[0], "none")
            self.assertEqual(ring[1], "2px")
            pg.context.close()

    def test_AC8_reduced_motion_zero_durations_and_instant_result(self):
        pg = B.page(page_path("en"), reduced=True)
        Sim.open_by_key(pg, "video")
        pg.locator("#sim-hive [data-cell]").first.focus()
        pg.keyboard.press("Enter")
        self.assertEqual(pg.locator("#sim-files li[data-result]").count(), 1, "result must appear at once")
        bad = pg.evaluate("""() => [...document.querySelectorAll('#sim, #sim *')].map(e => { const s = getComputedStyle(e);
            const nz = v => v.split(',').some(x => parseFloat(x) > 0);
            return (nz(s.transitionDuration) || nz(s.animationDuration)) ? e.tagName + '.' + e.className : null; }).filter(Boolean)""")
        self.assertEqual(bad, [])
        pg.context.close()

    def test_AC8_live_region_announces_result_name(self):
        pg = B.page(page_path("en"))
        Sim.open_by_key(pg, "video")
        pg.locator("#sim-hive [data-cell]").first.focus()
        pg.keyboard.press("Enter")
        self.assertIn("clip.mp4", pg.locator("#stage-live").inner_text())
        self.assertEqual(pg.locator("#stage-live").get_attribute("aria-live"), "polite")
        pg.context.close()

    def test_AC8_js_off_fallback(self):
        pg = B.page(page_path("en"), js=False)
        t = pg.locator("#sim .sim-nojs")
        self.assertEqual(t.count(), 1)
        self.assertTrue(t.is_visible())
        self.assertIn("turn on JavaScript to try it", t.inner_text())
        self.assertGreater(pg.locator("#sim svg, #sim img").count(), 0, "static picture")
        pg.context.close()
        pa = B.page(page_path("ar"), js=False)
        self.assertTrue(pa.locator("#sim .sim-nojs").inner_text().strip())
        pa.context.close()


class AC9RtlBrowser(unittest.TestCase):
    @unittest.skip("superseded by S-064 (new hero, 96px section rhythm, Download button at the header end); see test_s064_browser.py")
    def test_AC9_ar_brand_right_switch_left(self):
        pg = B.page("/", width=1280)
        brand = pg.locator("header .brand").bounding_box()
        sw = pg.locator("header .lang-switch").bounding_box()
        wrap = pg.locator("header .wrap").bounding_box()
        pad = pg.eval_on_selector("header .wrap", "e => parseFloat(getComputedStyle(e).paddingInlineStart)")
        self.assertGreater(brand["x"], 640)
        self.assertLess(sw["x"] + sw["width"], 640)
        self.assertAlmostEqual(brand["x"] + brand["width"], wrap["x"] + wrap["width"] - pad, delta=1, msg="brand at the right edge")
        self.assertAlmostEqual(sw["x"], wrap["x"] + pad, delta=1, msg="switch at the left edge")
        pg.context.close()
        pe = B.page("/en/", width=1280)
        self.assertLess(pe.locator("header .brand").bounding_box()["x"], 640)
        pe.context.close()


class AC11Mobile(unittest.TestCase):
    def test_AC11_mobile(self):
        for lang in LANGS:
            for page in ("", "download"):
                pg = B.page(page_path(lang, page), width=375, height=667)
                self.assertLessEqual(pg.evaluate("document.documentElement.scrollWidth"), 375, f"{lang}/{page}")
                small = pg.evaluate("""() => [...document.querySelectorAll('a.button, button, header a, footer a, .hero a, #sim [tabindex], input')]
                    .filter(e => e.getClientRects().length && !e.classList.contains('sr-only') && getComputedStyle(e).visibility !== 'hidden')
                    .map(e => { const r = e.getBoundingClientRect(); return [r.width, r.height, e.tagName + '.' + e.className + ' ' + (e.textContent||'').trim().slice(0,15)]; })
                    .filter(x => x[0] < 44 || x[1] < 44)""")
                self.assertEqual(small, [], f"{lang}/{page} tap targets")
                fonts = pg.evaluate("""() => [...document.querySelectorAll('main p, main li')].filter(e => e.getClientRects().length && !e.closest('svg') && !e.classList.contains('sr-only') && e.textContent.trim())
                    .map(e => parseFloat(getComputedStyle(e).fontSize)).filter(f => f < 16)""")
                self.assertEqual(fonts, [], f"{lang}/{page} body text under 16px")
                if page == "":
                    r = pg.locator(".hero a.button-primary").bounding_box()
                    self.assertGreaterEqual(r["y"], 0)
                    self.assertLessEqual(r["y"] + r["height"], 667)
                    self.assertLessEqual(r["x"] + r["width"], 375)
                pg.context.close()


if __name__ == "__main__":
    unittest.main(verbosity=2)
