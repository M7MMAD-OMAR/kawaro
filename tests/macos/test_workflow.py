"""S-069: structure checks for .github/workflows/macos.yml.

Run: python3 -m unittest -v tests/macos/test_workflow.py
"""
import pathlib
import unittest

import yaml

ROOT = pathlib.Path(__file__).resolve().parents[2]
WORKFLOW = ROOT / ".github" / "workflows" / "macos.yml"


def load():
    text = WORKFLOW.read_text(encoding="utf-8")
    return text, yaml.safe_load(text)


def steps(doc):
    jobs = doc["jobs"]
    return [s for job in jobs.values() for s in job["steps"]]


class MacosWorkflow(unittest.TestCase):
    def test_runner_is_macos_14(self):
        _, doc = load()
        jobs = doc["jobs"]
        self.assertEqual(len(jobs), 1)
        (job,) = jobs.values()
        self.assertEqual(job["runs-on"], "macos-14")

    def test_triggers(self):
        _, doc = load()
        # YAML 1.1 parses the key `on` as boolean True.
        on = doc.get("on", doc.get(True))
        self.assertEqual(on["push"]["branches"], ["story/**"])
        self.assertIn("workflow_dispatch", on)
        crons = [s["cron"] for s in on["schedule"]]
        self.assertTrue(crons)
        for cron in crons:
            self.assertEqual(len(cron.split()), 5)
        self.assertNotIn("pull_request", on)

    def test_cargo_steps_do_not_block(self):
        _, doc = load()
        found = {}
        for s in steps(doc):
            run = s.get("run", "")
            if "GITHUB_STEP_SUMMARY" in run:
                continue
            for cmd in ("cargo check --workspace --all-targets",
                        "cargo test --workspace --no-fail-fast"):
                if cmd in run:
                    found[cmd] = s
        self.assertEqual(len(found), 2)
        for s in found.values():
            self.assertIs(s.get("continue-on-error"), True)
            self.assertIn("tee", s["run"])

    def test_read_only_permissions(self):
        _, doc = load()
        self.assertEqual(doc["permissions"], {"contents": "read"})
        for job in doc["jobs"].values():
            self.assertIn(job.get("permissions", {"contents": "read"}),
                          [{"contents": "read"}])

    def test_no_secrets(self):
        text, _ = load()
        self.assertNotIn("secrets.", text)

    def test_artifact_upload(self):
        _, doc = load()
        uses = [s.get("uses", "") for s in steps(doc)]
        self.assertIn("actions/upload-artifact@v7", uses)
        up = [s for s in steps(doc)
              if s.get("uses") == "actions/upload-artifact@v7"][0]
        self.assertEqual(up.get("if"), "always()")

    def test_toolchain_and_checkout(self):
        _, doc = load()
        uses = [s.get("uses", "") for s in steps(doc)]
        self.assertIn("actions/checkout@v7", uses)
        self.assertIn("dtolnay/rust-toolchain@stable", uses)

    def test_step_summary_written(self):
        text, _ = load()
        self.assertIn("$GITHUB_STEP_SUMMARY", text)

    def test_no_long_dashes(self):
        text, _ = load()
        self.assertNotIn("\u2014", text)
        self.assertNotIn("\u2013", text)


if __name__ == "__main__":
    unittest.main()
