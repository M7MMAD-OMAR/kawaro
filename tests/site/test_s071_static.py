"""S-071 static checks: Markdown twin links, the 404 page, no tracked caches."""
import re
import subprocess
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from lib import REPO, site  # noqa: E402


class AC1MarkdownLinks(unittest.TestCase):
    def test_AC1_markdown_link_resolves_to_the_twin(self):
        pages = sorted(site().glob("docs/*/index.html")) + sorted(site().glob("en/docs/*/index.html"))
        self.assertGreaterEqual(len(pages), 18)
        for page in pages:
            hrefs = re.findall(r'href="((?!https?:)[^"]+\.md)"', page.read_text(encoding="utf-8"))
            self.assertEqual(len(hrefs), 1, page)
            target = (page.parent / hrefs[0]).resolve()
            self.assertTrue(target.is_file(), f"{page.relative_to(site())} links to {hrefs[0]}: missing")


class AC2NotFoundPage(unittest.TestCase):
    def test_AC2_404_page_exists_and_is_root_absolute(self):
        t = (site() / "404.html").read_text(encoding="utf-8")
        self.assertIn("<h1>", t)
        self.assertIn('name="robots" content="noindex"', t)
        self.assertNotIn('rel="canonical"', t)
        bad = [m for m in re.findall(r'(?:href|src)="([^"]*)"', t)
               if not re.match(r"(https://|/|#)", m)]
        self.assertEqual(bad, [], "the page is served at any depth, so every URL is root absolute")
        self.assertIn('href="/en/"', t)

    def test_AC2_wrangler_serves_it(self):
        cfg = (REPO / "wrangler.jsonc").read_text(encoding="utf-8")
        self.assertIn('"not_found_handling": "404-page"', cfg)

    def test_AC2_404_is_not_in_the_sitemap(self):
        self.assertNotIn("404", (site() / "sitemap.xml").read_text(encoding="utf-8"))


class AC3Hygiene(unittest.TestCase):
    def test_AC3_no_tracked_bytecode(self):
        out = subprocess.run(["git", "ls-files"], cwd=REPO, capture_output=True, text=True).stdout.split("\n")
        self.assertEqual([f for f in out if "__pycache__" in f or f.endswith(".pyc")], [])


if __name__ == "__main__":
    unittest.main()
