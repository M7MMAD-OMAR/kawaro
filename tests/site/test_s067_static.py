"""S-067 static checks: the Arabic hero film, its files, the headers it needs."""
import re
import subprocess
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from lib import REPO, site  # noqa: E402

MEDIA = ("hero.webm", "hero.mp4", "hero-poster.webp")


def read(rel):
    return (site() / rel).read_text(encoding="utf-8")


class AC1Film(unittest.TestCase):
    def test_AC1_arabic_home_has_one_film(self):
        t = read("index.html")
        self.assertEqual(t.count("<video"), 1)
        v = t.split("<video", 1)[1].split("</video>", 1)[0]
        for need in ('poster="media/hero-poster.webp"', "autoplay", "muted", "loop", "playsinline",
                     'src="media/hero.webm" type="video/webm"', 'src="media/hero.mp4" type="video/mp4"'):
            self.assertIn(need, v)
        self.assertRegex(v, r'aria-label="[^"]{20,}"')
        self.assertLess(v.index("hero.webm"), v.index("hero.mp4"), "WebM first")
        self.assertNotIn('class="demo"', t)

    def test_AC1_english_home_keeps_the_demo(self):
        t = read("en/index.html")
        self.assertEqual(t.count("<video"), 0)
        self.assertIn('<figure class="demo"', t)

    def test_AC1_media_files_exist_and_are_small(self):
        for name in MEDIA:
            p = site() / "media" / name
            self.assertTrue(p.is_file(), name)
            self.assertLess(p.stat().st_size, 2 * 1024 * 1024, f"{name} over 2 MiB")
        self.assertEqual(subprocess.run(["ffprobe", "-v", "error", "-select_streams", "a", "-show_entries",
                                         "stream=index", "-of", "csv=p=0", str(site() / "media" / "hero.mp4")],
                                        capture_output=True, text=True).stdout.strip(), "", "the film is silent")


class AC4Style(unittest.TestCase):
    def test_AC4_film_css_uses_tokens_only(self):
        c = (REPO / "site" / "css" / "film.css").read_text(encoding="utf-8")
        c = re.sub(r"/\*.*?\*/", "", c, flags=re.S)
        for bad in ("box-shadow", "gradient", "border-radius", "#"):
            self.assertNotIn(bad, c)
        self.assertNotRegex(c, r"\d(px|rem|em)\b")


class AC5Headers(unittest.TestCase):
    def test_AC5_headers_allow_same_origin_media_and_muted_autoplay(self):
        r = subprocess.run([sys.executable, str(REPO / "scripts" / "site-headers.py"), str(site())],
                           capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stderr)
        h = (site() / "_headers").read_text(encoding="utf-8")
        self.assertIn("media-src 'self'", h)
        self.assertIn("autoplay=(self)", h)
        self.assertRegex(h, r"/media/\*\n  Cache-Control: public, max-age=2592000")


if __name__ == "__main__":
    unittest.main(verbosity=1)
