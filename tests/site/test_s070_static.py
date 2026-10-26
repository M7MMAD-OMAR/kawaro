"""S-070 static checks: the English hero film, its files, and that Arabic is unchanged."""
import subprocess
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from lib import site  # noqa: E402

MEDIA = ("hero-en.webm", "hero-en.mp4", "hero-en-poster.webp")


def read(rel):
    return (site() / rel).read_text(encoding="utf-8")


class AC1EnglishFilm(unittest.TestCase):
    def test_AC1_english_home_has_one_film(self):
        t = read("en/index.html")
        self.assertEqual(t.count("<video"), 1)
        v = t.split("<video", 1)[1].split("</video>", 1)[0]
        for need in ('poster="../media/hero-en-poster.webp"', "autoplay", "muted", "loop", "playsinline",
                     'src="../media/hero-en.webm" type="video/webm"', 'src="../media/hero-en.mp4" type="video/mp4"'):
            self.assertIn(need, v)
        self.assertLess(v.index("hero-en.webm"), v.index("hero-en.mp4"), "WebM first")
        self.assertNotIn('class="demo"', t)

    def test_AC2_aria_label_is_english(self):
        t = read("en/index.html")
        v = t.split("<video", 1)[1].split(">", 1)[0]
        label = v.split('aria-label="', 1)[1].split('"', 1)[0]
        self.assertGreaterEqual(len(label), 40)
        self.assertTrue(label.isascii(), "English label has no Arabic text")
        self.assertIn("silent", label.lower())

    def test_AC3_arabic_home_still_uses_the_arabic_film(self):
        t = read("index.html")
        self.assertIn("media/hero.webm", t)
        self.assertNotIn("hero-en", t)

    def test_AC4_media_files_exist_small_silent(self):
        for name in MEDIA:
            p = site() / "media" / name
            self.assertTrue(p.is_file(), name)
            self.assertLess(p.stat().st_size, 2 * 1024 * 1024, f"{name} over 2 MiB")
        for name in ("hero-en.mp4", "hero-en.webm"):
            out = subprocess.run(["ffprobe", "-v", "error", "-select_streams", "a", "-show_entries",
                                  "stream=index", "-of", "csv=p=0", str(site() / "media" / name)],
                                 capture_output=True, text=True).stdout.strip()
            self.assertEqual(out, "", f"{name} is silent")

    def test_AC4_video_is_720p(self):
        out = subprocess.run(["ffprobe", "-v", "error", "-select_streams", "v:0", "-show_entries",
                              "stream=width,height", "-of", "csv=p=0", str(site() / "media" / "hero-en.mp4")],
                             capture_output=True, text=True).stdout.strip()
        self.assertEqual(out, "1280,720")


if __name__ == "__main__":
    unittest.main(verbosity=1)
