"""S-088 static checks: the download page teaches the first try.

DOM contract (download page, both languages), from the ACs and D1 to D7:
  section#choose .dl-stage
    .dl-side   the download slabs (win slab keeps the S-083 note)
    .dl-demo   figure.film.dl-film
                 video.film-video [controls][autoplay][muted][loop][playsinline]
                   poster media/hero-poster.webp (ar) or media/hero-en-poster.webp (en)
                   source webm then mp4
                 img.film-still (the same poster, the reduced-motion variant)
                 figcaption > ol.flow li x4 (drag, press your shortcut, drop, result)
                              p.flow-trigger (Super+Z on Linux, right mouse button and
                              no key on Windows)
               div.card.try-box (h3, p, a.hex-button[download] to assets/kawaro-sample.png)
    .dl-walk   h2 "After the download" / «بعد التنزيل» and
               ol.walk > li.step-cell x5 (install, open, drag, drop, result)
Run with: python3 -I test_s088_static.py (needs PATH=/usr/bin:$PATH like the others).
"""
import re
import shutil
import subprocess
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import lib  # noqa: E402
from lib import REPO, parse_file, page_file, site  # noqa: E402

CHECK_LINKS = REPO / "scripts" / "site" / "check-links.sh"
SAMPLE = "assets/kawaro-sample.png"
FIXTURE = REPO / "tests" / "fixtures" / "images" / "kawaro-sample.png"
SIZE_CAP = 300 * 1024

FLOW = {
    "en": ["Drag a file.", "Press your shortcut.", "Drop it on the cell.", "A new copy lands beside the original."],
    "ar": ["اسحب الملف.", "اضغط اختصارك.", "أفلته على الخلية.", "تظهر نسخة جديدة بجانب الأصل."],
}
WALK = {
    "en": [
        ("Install Kawaro", ["Windows protected your PC", "More info", "Run anyway", "MSI", "DEB", "RPM"]),
        ("Open Kawaro", ["Show more options", "Kawaro menu", "Settings"]),
        ("Drag a file", ["Super+Z", "right mouse button", "no key is held"]),
        ("Drop on the cell", ["cell of the action"]),
        ("Take the result", ["next to the original"]),
    ],
    "ar": [
        ("ثبّت كوارو", ["Windows protected your PC", "More info", "Run anyway", "MSI", "DEB", "RPM"]),
        ("افتح كوارو", ["إظهار خيارات إضافية", "قائمة كوارو", "الإعدادات"]),
        ("اسحب ملفاً", ["Super+Z", "زر الفأرة الأيمن", "دون ضغط أي مفتاح"]),
        ("أفلت على الخلية", ["خلية الإجراء"]),
        ("خذ الناتج", ["بجانب الأصل"]),
    ],
}
TRIGGER = {
    "en": ["Super+Z", "right mouse button", "no key is held"],
    "ar": ["Super+Z", "زر الفأرة الأيمن", "لا يُضغط أي مفتاح"],
}
NOTE_TEXT = {
    "en": ["Windows shows a SmartScreen warning when you open the Kawaro installer.",
           "Verify the file against SHA256SUMS first, then click More info and Run anyway."],
    "ar": ["يظهر تحذير SmartScreen عند فتح مثبّت كوارو على ويندوز.",
           "تحقق من الملف مقابل SHA256SUMS أولاً، ثم انقر More info وRun anyway."],
}
TRY = {
    "en": ("Try it on an image", ["Drag the sample image onto the hive", "result lands beside it"]),
    "ar": ("جرّب على صورة", ["اسحب الصورة العيّنة على الخلية", "يظهر الناتج بجانبها"]),
}
WALK_TITLE = {"en": "After the download", "ar": "بعد التنزيل"}


def read(rel):
    return (site() / rel).read_text(encoding="utf-8")


def page(lang):
    return page_file(lang, "download")


class AC1Film(unittest.TestCase):
    def video(self, lang):
        t = page(lang).read_text(encoding="utf-8")
        self.assertEqual(t.count("<video"), 1, f"{lang}: exactly one film")
        return t.split("<video", 1)[1].split("</video>", 1)[0]

    def test_AC1_film_controls_autoplay_loop_and_sources(self):
        stem = {"ar": "hero", "en": "hero-en"}
        for lang in ("ar", "en"):
            v = self.video(lang)
            head = v.split(">", 1)[0]
            for need in ("controls", "autoplay", "muted", "loop", "playsinline", 'preload="auto"',
                         f'poster="', f'media/{stem[lang]}-poster.webp', 'aria-label="'):
                self.assertIn(need, head, f"{lang}: {need}")
            self.assertLess(v.index(f"media/{stem[lang]}.webm"), v.index(f"media/{stem[lang]}.mp4"), f"{lang}: WebM first")
            self.assertIn('type="video/webm"', v)
            self.assertIn('type="video/mp4"', v)

    def test_AC1_film_label_is_language_and_long(self):
        for lang in ("ar", "en"):
            v = self.video(lang)
            label = v.split('aria-label="', 1)[1].split('"', 1)[0]
            self.assertGreaterEqual(len(label), 25, lang)
            if lang == "ar":
                self.assertRegex(label, r"[؀-ۿ]", "Arabic label")
            else:
                self.assertTrue(label.isascii(), "English label has no Arabic text")

    def test_AC1_reuses_the_hero_clip(self):
        for lang, other in (("ar", "index.html"), ("en", "en/index.html")):
            home = (site() / other).read_text(encoding="utf-8")
            dl = page(lang).read_text(encoding="utf-8")
            for token in ("hero", "hero-en"):
                self.assertEqual(("hero-en" in dl), ("hero-en" in home), f"{lang}: same clip as the home hero")

    def test_AC1_caption_flow_is_numbered_and_names_the_flow(self):
        for lang in ("ar", "en"):
            root = parse_file(page(lang))
            caps = root.find_all("figcaption")
            self.assertEqual(len(caps), 1, lang)
            flows = caps[0].find_all("ol.flow")
            self.assertEqual(len(flows), 1, lang)
            items = [li for li in flows[0].find_all("li")]
            self.assertEqual(len(items), 4, f"{lang}: four flow steps")
            for i, li in enumerate(items, 1):
                nums = li.find_all("span.flow-num")
                self.assertEqual(len(nums), 1, f"{lang} step {i}: one number")
                self.assertEqual(nums[0].text(), str(i), f"{lang} step {i}: number")
                self.assertIn(FLOW[lang][i - 1], li.text(), f"{lang} step {i}")

    def test_AC1_caption_reads_the_measured_keys(self):
        for lang in ("ar", "en"):
            root = parse_file(page(lang))
            trig = root.find_all("p.flow-trigger")
            self.assertEqual(len(trig), 1, lang)
            text = trig[0].text()
            for want in TRIGGER[lang]:
                self.assertIn(want, text, f"{lang}: {want}")

    def test_AC1_no_line_names_alt_or_ctrl(self):
        for lang in ("ar", "en"):
            text = parse_file(page(lang)).text()
            self.assertIsNone(re.search(r"\b(Alt|Ctrl)\b", text), f"{lang}: an unmeasured key name on the page")

    def test_AC1_reduced_motion_block_is_the_poster_and_steps(self):
        for lang in ("ar", "en"):
            root = parse_file(page(lang))
            stills = root.find_all("img.film-still")
            self.assertEqual(len(stills), 1, lang)
            st = stills[0].attrs
            self.assertIn("-poster.webp", st.get("src", ""), lang)
            self.assertEqual((st.get("width"), st.get("height")), ("1280", "720"), lang)
            self.assertEqual(st.get("alt"), "", lang)
        css = read("css/film.css")
        block = re.search(r"@media\s*\(prefers-reduced-motion:\s*reduce\)\s*\{(.*)\}", css, re.S)
        self.assertIsNotNone(block, "film.css keeps a reduced-motion block")
        inner = block.group(1)
        self.assertIn(".film-still", inner)
        self.assertIn("display: none", inner)
        self.assertIn("display: block", inner)

    def test_AC1_film_css_is_on_the_download_page_too(self):
        for lang in ("ar", "en"):
            t = page(lang).read_text(encoding="utf-8")
            self.assertIn("css/film.css", t, lang)


class AC2Walkthrough(unittest.TestCase):
    def steps(self, lang):
        root = parse_file(page(lang))
        walks = root.find_all("ol.walk")
        self.assertEqual(len(walks), 1, lang)
        return walks[0].find_all("li.step-cell")

    def test_AC2_walk_has_five_numbered_steps(self):
        for lang in ("ar", "en"):
            items = self.steps(lang)
            self.assertEqual(len(items), 5, f"{lang}: install, open, drag, drop, result")
            for i, li in enumerate(items, 1):
                nums = li.find_all("span.step-cell-num")
                self.assertEqual(len(nums), 1, f"{lang} step {i}")
                self.assertEqual(nums[0].text(), str(i), f"{lang} step {i}")
                self.assertEqual(len(li.find_all("h3")), 1, f"{lang} step {i}: one title")
                self.assertEqual(len(li.find_all("p")), 1, f"{lang} step {i}: one sentence")

    def test_AC2_heading_says_after_the_download(self):
        for lang in ("ar", "en"):
            root = parse_file(page(lang))
            titles = [h.text() for h in root.find_all("h2")]
            self.assertIn(WALK_TITLE[lang], titles, lang)

    def test_AC2_steps_name_the_controls_the_user_sees(self):
        for lang in ("ar", "en"):
            items = self.steps(lang)
            for i, (title, needs) in enumerate(WALK[lang]):
                self.assertIn(title, items[i].text(), f"{lang} step {i + 1} title")
                for want in needs:
                    self.assertIn(want, items[i].text(), f"{lang} step {i + 1}: {want}")

    def test_AC2_drag_step_holds_no_key_on_windows(self):
        for lang in ("ar", "en"):
            drag = self.steps(lang)[2].text()
            self.assertIn("Super+Z", drag, lang)
            self.assertIn("right mouse button" if lang == "en" else "زر الفأرة الأيمن", drag, lang)
            self.assertIn("no key is held" if lang == "en" else "دون ضغط أي مفتاح", drag, lang)

    def test_AC2_the_walkthrough_names_the_tray_menu_until_the_welcome_window_exists(self):
        # D6: S-087 has not landed, so the Settings pointer is the tray menu.
        self.assertFalse((REPO / "crates" / "ui" / "ui" / "welcome.slint").exists(),
                         "S-087 landed: update the walkthrough wording and this test in the same release")
        for lang in ("ar", "en"):
            self.assertIn("Kawaro menu" if lang == "en" else "قائمة كوارو", self.steps(lang)[1].text(), lang)


class AC3Sample(unittest.TestCase):
    def button(self, lang):
        root = parse_file(page(lang))
        boxes = root.find_all("div.card.try-box")
        self.assertEqual(len(boxes), 1, lang)
        btns = boxes[0].find_all("a")
        self.assertEqual(len(btns), 1, lang)
        return boxes[0], btns[0]

    def test_AC3_sample_asset_is_linked_small_and_downloads_normally(self):
        asset = site() / SAMPLE
        self.assertTrue(asset.is_file(), f"{SAMPLE} missing from the built site")
        self.assertLessEqual(asset.stat().st_size, SIZE_CAP, "the sample is a few hundred kilobytes at most")
        for lang in ("ar", "en"):
            box, a = self.button(lang)
            self.assertTrue(a.attrs.get("href", "").endswith(SAMPLE), lang)
            self.assertEqual(a.attrs.get("download"), "kawaro-sample.png", lang)
            label = a.attrs.get("aria-label", "")
            self.assertGreaterEqual(len(label), 20, f"{lang}: descriptive accessible label")
            self.assertIn(TRY[lang][0], a.text(), lang)
            self.assertNotEqual(label.strip(), a.text().strip(), "the label explains more than the visible text")
            for want in TRY[lang][1]:
                self.assertIn(want, box.text(), f"{lang}: {want}")

    def test_AC3_the_page_and_the_app_show_the_same_sample(self):
        self.assertTrue(FIXTURE.is_file(), "tests/fixtures/images/kawaro-sample.png missing")
        self.assertEqual(FIXTURE.read_bytes(), (site() / SAMPLE).read_bytes(),
                         "site asset must be the same bytes as the fixture S-087 embeds")

    def test_AC3_check_links_resolves_the_asset(self):
        r = subprocess.run(["bash", str(CHECK_LINKS), str(lib.built())], capture_output=True, text=True, timeout=120)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn("check-links: ok", r.stdout)

    def test_AC3_check_links_fails_on_a_missing_asset(self):
        root = lib.TMP / "links-missing-asset"
        if root.exists():
            shutil.rmtree(root)
        (root / "site").mkdir(parents=True)
        (root / "site" / "index.html").write_text(f'<a href="{SAMPLE}">try</a>', encoding="utf-8")
        r = subprocess.run(["bash", str(CHECK_LINKS), str(root)], capture_output=True, text=True, timeout=120)
        self.assertEqual(r.returncode, 1, r.stdout + r.stderr)
        self.assertIn(SAMPLE, r.stdout + r.stderr)


class AC4Warning(unittest.TestCase):
    def note(self, lang):
        root = parse_file(page(lang))
        slabs = [s for s in root.find_all("div.download-slab") if s.attrs.get("data-card") == "win"]
        self.assertEqual(len(slabs), 1, lang)
        notes = slabs[0].find_all("p.download-note")
        self.assertEqual(len(notes), 1, lang)
        return slabs[0], notes[0]

    def test_AC4_windows_block_has_the_s083_note_and_guide(self):
        guide = {"en": "docs/windows-smartscreen.md", "ar": "docs/windows-smartscreen.ar.md"}
        for lang in ("ar", "en"):
            slab, note = self.note(lang)
            text = note.text()
            for line in NOTE_TEXT[lang]:
                self.assertIn(line, text, f"{lang}: {line}")
            links = note.find_all("a")
            self.assertEqual(len(links), 1, lang)
            self.assertIn(guide[lang], links[0].attrs.get("href", ""), lang)

    def test_AC4_claim_scan_no_signing_claim_no_disappearing_warning_no_disable(self):
        for lang in ("ar", "en"):
            _, note = self.note(lang)
            text = note.text()
            self.assertIsNone(re.search(r"(?i)signed|warning gone|disable", text), f"{lang}: the note claims more than it should")
            page_text = parse_file(page(lang)).text()
            page_text = page_text.replace("غير موقّعة", "")
            bad_en = re.search(r"(?i)\bdisable\b|turn off|bypass|warning (gone|disappears|will not|won.t)|"
                               r"no more warning|no warning|is signed|are signed|now signed|"
                               r"signed (build|installer|app|download|package)", page_text)
            self.assertIsNone(bad_en, f"{lang}: {bad_en.group(0) if bad_en else ''}")
            bad_ar = re.search(r"تعطيل|عطّل|أوقف التحذير|اختفى التحذير|التحذير يختفي|لن يظهر التحذير|بلا تحذير|موقّع", page_text)
            self.assertIsNone(bad_ar, f"{lang}: {bad_ar.group(0) if bad_ar else ''}")


class AC5Finish(unittest.TestCase):
    def test_AC5_security_headers_and_language_switch_lines_intact(self):
        for lang, other in (("ar", "en"), ("en", "ar")):
            t = page(lang).read_text(encoding="utf-8")
            self.assertEqual(t.count('rel="alternate"'), 3, f"{lang}: hreflang alternates")
            self.assertIn('class="nav-link lang-switch"', t, lang)
            self.assertIn(f'hreflang="{other}"', t, lang)
            root = parse_file(page(lang))
            langs = root.find_all("ul.footer-lang")
            self.assertEqual(len(langs), 1, f"{lang}: the footer language switch line")
            self.assertEqual(len(langs[0].find_all("a")), 1, lang)

    def test_AC5_no_long_dashes_in_the_touched_files(self):
        paths = [page("ar"), page("en"), site() / "css" / "film.css", site() / "css" / "pages.css",
                 REPO / "site" / "build.py", REPO / "CHANGELOG.md"]
        for p in paths:
            self.assertIsNone(re.search("[\u2013\u2014]", p.read_text(encoding="utf-8")), p.name)


class AC6Changelog(unittest.TestCase):
    def test_AC6_one_bilingual_line_in_the_link_form(self):
        lines = [l for l in (REPO / "CHANGELOG.md").read_text(encoding="utf-8").splitlines() if "S-088" in l]
        self.assertEqual(len(lines), 1, "exactly one CHANGELOG line for S-088")
        self.assertIn("[S-088](https://github.com/M7MMAD-OMAR/kawaro)", lines[0])
        self.assertRegex(lines[0], r"[A-Za-z]")
        self.assertRegex(lines[0], r"[؀-ۿ]")
        self.assertIsNone(re.search("[\u2013\u2014]", lines[0]))


if __name__ == "__main__":
    unittest.main(verbosity=2)
