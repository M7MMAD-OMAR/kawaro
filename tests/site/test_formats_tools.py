"""The formats page names the tool that really runs each action, in both languages.

The generator maps an action engine to the tool that runs it. An engine that is
missing from that map falls back to the built-in label, so an external engine such
as poppler or qpdf would be published as built in, which is false and would tell a
user that the action needs no external program.

Every action is read from the real `actions/*.toml` files, so the test follows the
action definitions instead of hard coded table rows.
"""
import sys
import tomllib
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from lib import REPO, site  # noqa: E402

ACTIONS = REPO / "actions"


def all_actions():
    for f in sorted(ACTIONS.glob("*.toml")):
        for a in tomllib.loads(f.read_text(encoding="utf-8"))["action"]:
            yield a


def action_of(engine):
    for a in all_actions():
        if a["engine"] == engine:
            return a
    raise AssertionError(f"no action uses the engine {engine}")


def tool_of(md, action, lang):
    """The tool cell of the row that carries this action's label."""
    label = action["label"][lang]
    for line in md.splitlines():
        if not line.startswith("| "):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if cells and cells[0] == label:
            return cells[1]
    raise AssertionError(f"no row for the action label {label!r}")


def formats_md(lang):
    p = site() / ("en/docs/formats.md" if lang == "en" else "docs/formats.md")
    return p.read_text(encoding="utf-8")


def formats_html(lang):
    p = site() / ("en/docs/formats/index.html" if lang == "en" else "docs/formats/index.html")
    return p.read_text(encoding="utf-8")


class ExternalEngines(unittest.TestCase):
    """Poppler and qpdf are external programs, so they are named, not built in."""

    def test_poppler_is_named_as_an_external_tool_in_both_languages(self):
        action = action_of("poppler")
        for lang in ("en", "ar"):
            self.assertEqual(tool_of(formats_md(lang), action, lang), "Poppler", lang)

    def test_qpdf_is_named_as_an_external_tool_in_both_languages(self):
        action = action_of("qpdf")
        for lang in ("en", "ar"):
            self.assertEqual(tool_of(formats_md(lang), action, lang), "qpdf", lang)

    def test_the_published_page_carries_the_tool_names(self):
        for lang in ("en", "ar"):
            html = formats_html(lang)
            self.assertIn("Poppler", html, lang)
            self.assertIn("qpdf", html, lang)

    def test_the_llm_file_carries_the_tool_names(self):
        full = (site() / "llms-full.txt").read_text(encoding="utf-8")
        self.assertIn("Poppler", full)
        self.assertIn("qpdf", full)


class BuiltInEngines(unittest.TestCase):
    """The engines that run inside Kawaro keep the built-in label."""

    def test_the_image_engine_stays_built_in(self):
        action = action_of("image")
        self.assertEqual(tool_of(formats_md("en"), action, "en"), "built in")
        self.assertEqual(tool_of(formats_md("ar"), action, "ar"), "مدمجة")

    def test_the_pdf_engine_stays_built_in(self):
        action = action_of("pdf")
        self.assertEqual(tool_of(formats_md("en"), action, "en"), "built in")
        self.assertEqual(tool_of(formats_md("ar"), action, "ar"), "مدمجة")


if __name__ == "__main__":
    unittest.main()
