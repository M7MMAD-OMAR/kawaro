"""S-059 deploy tests: static and script checks, one test per row of the
story's test case table, as amended by 'Manager decision (2026-10-26)'.

Run with python3 -I from a neutral cwd (see run_s059.sh); heavy, so through
run_s059_heavy.sh. The staged upload set is built in a temp copy of the repo.
"""
import base64
import datetime
import hashlib
import json
import os
import re
import shutil
import socket
import subprocess
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import s059lib as S  # noqa: E402

REPO = S.REPO
TODAY = datetime.datetime.now(datetime.timezone.utc).date()
EIGHT = {
    "Content-Security-Policy", "Strict-Transport-Security", "X-Content-Type-Options",
    "Referrer-Policy", "Permissions-Policy", "Cross-Origin-Opener-Policy",
    "Cross-Origin-Resource-Policy", "X-Frame-Options",
}
PERMISSIONS = ("accelerometer=(), autoplay=(), browsing-topics=(), camera=(), clipboard-read=(), "
               "display-capture=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), "
               "midi=(), payment=(), usb=()")
# ADR 0007 section 7, script-src hash checked separately.
CSP_EXPECTED = {
    "default-src": ["'none'"], "style-src": ["'self'"], "style-src-attr": ["'unsafe-inline'"],
    "img-src": ["'self'"], "font-src": ["'self'"], "connect-src": ["'self'"],
    "worker-src": ["'self'"], "manifest-src": ["'self'"], "object-src": ["'none'"],
    "base-uri": ["'none'"], "form-action": ["'self'"], "frame-ancestors": ["'none'"],
    "upgrade-insecure-requests": [],
}
IMMUTABLE_PATHS = {"/pagefind/fragment/*", "/pagefind/index/*", "/pagefind/*.pf_meta"}
NAMED = ["docs/start/index.html", "en/docs/start/index.html", "llms.txt", "docs/start.md",
         ".well-known/security.txt", "pagefind/pagefind.js"]
IGNORE_REQUIRED = ["build.py", "*.py", "*.sh", "*.map", "*.toml", "*.lock", ".env*", "*.key",
                   "*.pem", ".git*", ".nojekyll", "node_modules/", "backlog/"]


def walk_strings(obj):
    if isinstance(obj, dict):
        for k, v in obj.items():
            yield str(k)
            yield from walk_strings(v)
    elif isinstance(obj, list):
        for v in obj:
            yield from walk_strings(v)
    else:
        yield str(obj)


def hardlink_copy(src, dst):
    shutil.copytree(src, dst, copy_function=os.link)


class AC1Wrangler(unittest.TestCase):
    def test_AC1_wrangler_values_forbidden_keys_and_planted_main(self):
        f = REPO / "wrangler.jsonc"
        r = subprocess.run(["jq", ".", str(f)], capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, f"jq cannot read wrangler.jsonc: {r.stderr}")
        cfg = json.loads(r.stdout)
        bad = []
        if cfg.get("name") != "sbarhive-site":
            bad.append(f"name={cfg.get('name')!r}")
        if (cfg.get("assets") or {}).get("directory") != "./.deploy-out":
            bad.append(f"assets.directory={(cfg.get('assets') or {}).get('directory')!r}")
        if cfg.get("workers_dev") is not False:
            bad.append(f"workers_dev={cfg.get('workers_dev')!r} (want false)")
        cd = str(cfg.get("compatibility_date", ""))
        if not re.fullmatch(r"\d{4}-\d{2}-\d{2}", cd) or datetime.date.fromisoformat(cd) > TODAY:
            bad.append(f"compatibility_date={cd!r}")
        for k in ("main", "account_id", "vars", "kv_namespaces", "r2_buckets", "d1_databases",
                  "services", "secrets"):
            if k in cfg:
                bad.append(f"forbidden key {k}")
        if cfg.get("routes") != [{"pattern": S.DOMAIN, "custom_domain": True}]:
            bad.append(f"routes={cfg.get('routes')!r}")
        for s in walk_strings(cfg):
            if re.search(r"[A-Fa-f0-9]{32}", s) or "token" in s.lower():
                bad.append(f"secret like value {s!r}")
        # Planted: a temp root whose wrangler.jsonc gains "main"; the checker must exit 1.
        out = S.staged()
        root = S.TMP / "ac1-main"
        shutil.rmtree(root, ignore_errors=True)
        (root / "scripts").mkdir(parents=True)
        shutil.copytree(REPO / "scripts" / "site", root / "scripts" / "site")
        hardlink_copy(out, root / ".deploy-out")
        planted = dict(cfg, main="src/index.js")
        (root / "wrangler.jsonc").write_text(json.dumps(planted, indent=2), encoding="utf-8")
        rc = S.run_check(root)
        if rc.returncode != 1:
            bad.append(f"checker with planted main: exit {rc.returncode}, want 1: {rc.stderr.strip()[-300:]}")
        S.no_problems(self, bad)


class AC2Headers(unittest.TestCase):
    def test_AC2_headers_values_csp_cache_and_limits(self):
        out = S.staged()
        f = out / "_headers"
        self.assertTrue(f.exists(), "no .deploy-out/_headers")
        text = f.read_text(encoding="utf-8")
        rules = S.parse_headers(text)
        bad = []
        star = [hs for p, hs in rules if p == "/*"]
        self.assertEqual(len(star), 1, "want exactly one /* rule")
        hs = dict(star[0])
        if set(hs) != EIGHT or len(star[0]) != 8:
            bad.append(f"/* header names {sorted(k for k, _ in star[0])}")
        want = {"X-Content-Type-Options": "nosniff", "Referrer-Policy": "no-referrer",
                "Cross-Origin-Opener-Policy": "same-origin", "Cross-Origin-Resource-Policy": "same-origin",
                "X-Frame-Options": "DENY", "Permissions-Policy": PERMISSIONS}
        for k, v in want.items():
            if hs.get(k) != v:
                bad.append(f"{k}={hs.get(k)!r}")
        hsts = hs.get("Strict-Transport-Security")
        if hsts not in ("max-age=86400; includeSubDomains", "max-age=31536000; includeSubDomains"):
            bad.append(f"Strict-Transport-Security={hsts!r}")
        csp = hs.get("Content-Security-Policy", "")
        d = S.csp_directives(csp)
        for k, v in CSP_EXPECTED.items():
            if d.get(k) != v:
                bad.append(f"CSP {k}={d.get(k)!r} want {v!r}")
        ss = d.get("script-src", [])
        hashes = [t for t in ss if re.fullmatch(r"'sha256-[A-Za-z0-9+/]{43}='", t)]
        if ss[:2] != ["'self'", "'wasm-unsafe-eval'"] or len(hashes) != 1 or len(ss) != 3:
            bad.append(f"CSP script-src={ss!r}")
        if set(d) != set(CSP_EXPECTED) | {"script-src"}:
            bad.append(f"CSP directives {sorted(d)}")
        toks = [t for v in d.values() for t in v]
        for t in toks:
            if t == "*" or "*" in t or t.startswith(("http:", "https:", "data:")) or t in ("http:", "https:", "data:"):
                bad.append(f"CSP forbidden source {t}")
            if t == "'unsafe-eval'":
                bad.append("CSP 'unsafe-eval'")
        if "'unsafe-inline'" in ss:
            bad.append("script-src 'unsafe-inline'")
        imm = {p for p, h in rules for k, v in h if k.lower() == "cache-control" and "immutable" in v}
        if imm != IMMUTABLE_PATHS:
            bad.append(f"immutable on {sorted(imm)}")
        fonts = [v for p, h in rules if p == "/fonts/*" for k, v in h if k.lower() == "cache-control"]
        if not fonts or "max-age=2592000" not in fonts[0]:
            bad.append(f"/fonts/* cache-control {fonts!r}")
        if len(rules) > 100:
            bad.append(f"{len(rules)} rules")
        long_lines = [i for i, ln in enumerate(text.splitlines(), 1) if len(ln) > 2000]
        if long_lines:
            bad.append(f"lines over 2000 chars: {long_lines}")
        S.no_problems(self, bad)


SCRIPT_RE = re.compile(r"<script\b([^>]*)>(.*?)</script\s*>", re.S | re.I)
TAG_RE = re.compile(r"<([a-zA-Z][\w-]*)\b([^>]*)>", re.S)
ATTR_RE = re.compile(r"""([^\s=/>]+)(?:\s*=\s*("[^"]*"|'[^']*'|[^\s>]+))?""")


def attrs_of(s):
    return [(m.group(1).lower(), (m.group(2) or "").strip("\"'")) for m in ATTR_RE.finditer(s)]


def foreign(url):
    m = re.match(r"^(?:https?:)?//([^/?#]+)", url.strip(), re.I)
    return bool(m) and m.group(1).lower() != S.DOMAIN


class AC3BuiltOutput(unittest.TestCase):
    def test_AC3_staged_output_matches_csp_and_domain(self):
        out = S.staged()
        bad = []
        for p in out.rglob("*"):
            if p.is_file() and p.suffix in (".html", ".xml", ".txt", ".md"):
                t = p.read_text(encoding="utf-8", errors="replace")
                n = t.count("github.io")
                if n:
                    bad.append(f"github.io x{n} in {p.relative_to(out)}")
        # D3: the .nojekyll check applies to the staged folder (the upload set).
        if (out / ".nojekyll").exists():
            bad.append(".deploy-out/.nojekyll exists")
        rules = S.parse_headers((out / "_headers").read_text(encoding="utf-8")) if (out / "_headers").exists() else []
        csp = S.headers_for(rules, "/docs/start/").get("Content-Security-Policy", "")
        allowed = set(re.findall(r"'sha256-([A-Za-z0-9+/=]+)'", csp))
        seen_hashes = set()
        canon = 0
        htmls = sorted(p for p in out.rglob("*.html"))
        self.assertGreater(len(htmls), 10, "too few html files")
        for p in htmls:
            rel = str(p.relative_to(out))
            t = p.read_text(encoding="utf-8")
            for m in SCRIPT_RE.finditer(t):
                a = dict(attrs_of(m.group(1)))
                if "src" in a:
                    continue
                if a.get("type", "").lower() in ("application/ld+json", "application/json"):
                    continue
                h = base64.b64encode(hashlib.sha256(m.group(2).encode("utf-8")).digest()).decode()
                seen_hashes.add(h)
                if h not in allowed:
                    bad.append(f"inline script sha256-{h} not in CSP: {rel}")
            for tm in TAG_RE.finditer(t):
                tag = tm.group(1).lower()
                attrs = attrs_of(tm.group(2))
                ad = dict(attrs)
                for k, v in attrs:
                    if re.fullmatch(r"on[a-z]+", k):
                        bad.append(f"{k}= on <{tag}> in {rel}")
                    if k == "style" and not re.fullmatch(r"--[a-z-]+:-?[0-9.]+;?", v):
                        bad.append(f"style={v!r} in {rel}")
                    if k == "src" and foreign(v):
                        bad.append(f"foreign src {v} in {rel}")
                if tag == "link" and foreign(ad.get("href", "")):
                    bad.append(f"foreign link href {ad.get('href')} in {rel}")
                rel_attr = ad.get("rel", "").lower().split()
                url = None
                if tag == "link" and ("canonical" in rel_attr or "hreflang" in ad):
                    url = ad.get("href", "")
                if tag == "meta" and ad.get("property") == "og:url":
                    url = ad.get("content", "")
                if url is not None:
                    canon += 1
                    if not url.startswith(f"https://{S.DOMAIN}/"):
                        bad.append(f"canonical/hreflang/og:url {url} in {rel}")
            for im in re.finditer(r"""\bimport\b\s*\(?\s*["']([^"']+)["']""", t):
                if foreign(im.group(1)):
                    bad.append(f"foreign import {im.group(1)} in {rel}")
        if canon == 0:
            bad.append("no canonical, hreflang or og:url found")
        if len(seen_hashes) != 1:
            bad.append(f"{len(seen_hashes)} distinct inline scripts, want 1")
        sm = out / "sitemap.xml"
        locs = re.findall(r"<loc>\s*([^<]+?)\s*</loc>", sm.read_text(encoding="utf-8")) if sm.exists() else []
        if not locs:
            bad.append("no sitemap <loc>")
        bad += [f"sitemap loc {u}" for u in locs if not u.startswith(f"https://{S.DOMAIN}/")]
        S.no_problems(self, bad)


PLANTS = {
    "build.py": b"print(1)\n",
    ".env": b"A=1\n",
    ".git/config": b"[core]\n",
    "app.js.map": b"{}\n",
    "notes.toml": b"a = 1\n",
    "x.key": b"k\n",
    "docs/keyfile.txt": b"-----BEGIN PRIVATE KEY-----\nMIIB\n-----END PRIVATE KEY-----\n",
    "big.png": None,
    "tool.exe": b"MZ\n",
}


class AC4UploadSet(unittest.TestCase):
    def test_AC4_assetsignore_real_tree_planted_files_missing_inputs_shellcheck(self):
        out = S.staged()
        root = out.parent
        bad = []
        # .assetsignore: the source in site/ and its copy in the staged folder.
        for base in (REPO / "site", out):
            pats = S.ignore_patterns(base)
            if not pats:
                bad.append(f"no {base.name}/.assetsignore")
                continue
            miss = [p for p in IGNORE_REQUIRED if p not in pats]
            if miss:
                bad.append(f"{base.name}/.assetsignore misses {miss}")
            if "*.md" in pats or ".*" in pats:
                bad.append(f"{base.name}/.assetsignore lists *.md or .*")
        script = REPO / S.CHECK
        if not script.exists():
            bad.append(f"{S.CHECK} does not exist")
            S.no_problems(self, bad)
        if not os.access(script, os.X_OK):
            bad.append(f"{S.CHECK} is not executable")
        txt = script.read_text(encoding="utf-8")
        if "--dry-run" in txt:
            bad.append("script relies on wrangler --dry-run")
        # Real (staged) tree, check-only mode.
        r = S.run_check(root)
        m = re.search(r"upload set: (\d+) files", r.stdout + r.stderr)
        if r.returncode != 0 or not m or int(m.group(1)) < 40:
            bad.append(f"real tree: exit {r.returncode}, out {(r.stdout + r.stderr).strip()[-300:]!r}")
        pats = S.ignore_patterns(out)
        for n in NAMED:
            if not (out / n).is_file() or S.ignored(n, pats):
                bad.append(f"named path not in upload set: {n}")
        # Planted files, each in its own temp copy, folder passed as argument.
        for name, data in PLANTS.items():
            d = S.TMP / ("ac4-" + re.sub(r"\W", "_", name))
            shutil.rmtree(d, ignore_errors=True)
            hardlink_copy(out, d)
            p = d / name
            p.parent.mkdir(parents=True, exist_ok=True)
            if data is None:
                with open(p, "wb") as fh:
                    fh.truncate(26 * 1024 * 1024)
            else:
                p.write_bytes(data)
            rr = S.run_check(root, str(d))
            if rr.returncode != 1 or name not in rr.stderr:
                bad.append(f"planted {name}: exit {rr.returncode} (want 1), path in stderr={name in rr.stderr}")
            shutil.rmtree(d, ignore_errors=True)
        # Missing inputs: exit 2 with a message.
        d = S.TMP / "ac4-noheaders"
        shutil.rmtree(d, ignore_errors=True)
        hardlink_copy(out, d)
        (d / "_headers").unlink(missing_ok=True)
        rr = S.run_check(root, str(d))
        if rr.returncode != 2 or not (rr.stderr + rr.stdout).strip():
            bad.append(f"missing _headers: exit {rr.returncode}, want 2 with a message")
        nw = S.TMP / "ac4-nowrangler"
        shutil.rmtree(nw, ignore_errors=True)
        (nw / "scripts").mkdir(parents=True)
        shutil.copytree(REPO / "scripts" / "site", nw / "scripts" / "site")
        hardlink_copy(out, nw / ".deploy-out")
        rr = S.run_check(nw)
        if rr.returncode != 2 or not (rr.stderr + rr.stdout).strip():
            bad.append(f"missing wrangler.jsonc: exit {rr.returncode}, want 2 with a message")
        # --build in its own copy: prepares, then checks.
        broot = S.make_root("s059-build")
        rb = S.run_check(broot, "--build")
        if rb.returncode != 0 or "upload set:" not in rb.stdout + rb.stderr or not (broot / ".deploy-out" / "_headers").exists():
            bad.append(f"--build: exit {rb.returncode}, {(rb.stdout + rb.stderr).strip()[-300:]!r}")
        sc = subprocess.run(["shellcheck", str(script)], capture_output=True, text=True)
        if sc.returncode != 0:
            bad.append(f"shellcheck exit {sc.returncode}: {sc.stdout[-300:]}")
        S.no_problems(self, bad)


class AC5SecurityTxt(unittest.TestCase):
    def test_AC5_security_txt_fields_and_window(self):
        out = S.staged()
        f = out / ".well-known" / "security.txt"
        self.assertTrue(f.is_file(), "no .deploy-out/.well-known/security.txt")
        text = f.read_text(encoding="utf-8")
        fields = {}
        for ln in text.splitlines():
            if ":" in ln and not ln.startswith("#"):
                k, v = ln.split(":", 1)
                fields.setdefault(k.strip(), v.strip())
        bad = []
        if fields.get("Contact") != "https://github.com/M7MMAD-OMAR/sbarhive/security/advisories/new":
            bad.append(f"Contact={fields.get('Contact')!r}")
        if fields.get("Preferred-Languages") != "en, ar":
            bad.append(f"Preferred-Languages={fields.get('Preferred-Languages')!r}")
        if fields.get("Canonical") != f"https://{S.DOMAIN}/.well-known/security.txt":
            bad.append(f"Canonical={fields.get('Canonical')!r}")
        if not re.fullmatch(r"https://github\.com/M7MMAD-OMAR/sbarhive/blob/main/SECURITY\.md", fields.get("Policy", "")):
            bad.append(f"Policy={fields.get('Policy')!r}")
        exp = fields.get("Expires", "")
        m = re.fullmatch(r"(\d{4}-\d{2}-\d{2})T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z", exp)
        if not m:
            bad.append(f"Expires not RFC 3339 UTC: {exp!r}")
        else:
            days = (datetime.date.fromisoformat(m.group(1)) - TODAY).days
            if not 170 <= days <= 190:
                bad.append(f"Expires {days} days after build date")
        if "mailto:" in text:
            bad.append("mailto: present")
        if S.ignored(".well-known/security.txt", S.ignore_patterns(out)):
            bad.append("security.txt is ignored by .assetsignore")
        S.no_problems(self, bad)


def scan_workflows(wf_dir):
    """Forbidden strings across all workflows; secrets only in packages.yml update-manifest."""
    import yaml
    bad = []
    for f in sorted(Path(wf_dir).glob("*.y*ml")):
        t = f.read_text(encoding="utf-8")
        for s in ("pull_request_target", "CLOUDFLARE", "wrangler-action"):
            if s in t:
                bad.append(f"{s} in {f.name}")
        if "secrets." not in t:
            continue
        if f.name != "packages.yml":
            bad.append(f"secrets. in {f.name}")
            continue
        doc = yaml.safe_load(t)
        for job, body in (doc.get("jobs") or {}).items():
            if "secrets." in yaml.safe_dump(body) and job != "update-manifest":
                bad.append(f"secrets. in packages.yml job {job}")
        for name in re.findall(r"secrets\.([A-Za-z0-9_]+)", t):
            if name not in ("UPDATE_SIGNING_KEY", "UPDATE_SIGNING_PASSWORD"):
                bad.append(f"secrets.{name} in packages.yml")
    return bad


class AC6Workflows(unittest.TestCase):
    def test_AC6_site_check_workflow_and_forbidden_strings(self):
        try:
            import yaml
        except ImportError:
            self.fail("PyYAML missing: cannot parse workflows (not installed globally on purpose)")
        wf = REPO / ".github" / "workflows"
        f = wf / "site-check.yml"
        bad = []
        if not f.exists():
            bad.append("no .github/workflows/site-check.yml")
        else:
            raw = f.read_text(encoding="utf-8")
            doc = yaml.safe_load(raw)
            on = doc.get("on", doc.get(True)) or {}
            if "pull_request" not in on:
                bad.append("no pull_request trigger")
            push = (on.get("push") or {}) if isinstance(on, dict) else {}
            if "main" not in (push.get("branches") or []):
                bad.append("no push to main trigger")
            if doc.get("permissions") != {}:
                bad.append(f"top permissions={doc.get('permissions')!r}")
            runs = False
            for name, job in (doc.get("jobs") or {}).items():
                if job.get("permissions") != {"contents": "read"}:
                    bad.append(f"job {name} permissions={job.get('permissions')!r}")
                for st in job.get("steps") or []:
                    if "scripts/site/check-deploy.sh --build" in str(st.get("run", "")):
                        runs = True
                    if str(st.get("uses", "")).startswith("actions/checkout@"):
                        if (st.get("with") or {}).get("persist-credentials") is not False:
                            bad.append("checkout without persist-credentials: false")
            if not runs:
                bad.append("no step runs scripts/site/check-deploy.sh --build")
            uses = [ln for ln in raw.splitlines() if re.match(r"^\s*(-\s*)?uses:", ln)]
            if not uses:
                bad.append("no uses: lines")
            for ln in uses:
                if not re.search(r"@[0-9a-f]{40}\s+#\s*v\d", ln):
                    bad.append(f"unpinned uses: {ln.strip()}")
        bad += scan_workflows(wf)
        # The scan must catch a planted pull_request_target in a temp copy.
        tmp = S.TMP / "ac6-wf"
        shutil.rmtree(tmp, ignore_errors=True)
        shutil.copytree(wf, tmp)
        (tmp / "evil.yml").write_text("on:\n  pull_request_target:\njobs: {}\n", encoding="utf-8")
        if not any("pull_request_target" in b for b in scan_workflows(tmp)):
            bad.append("scan did not catch planted pull_request_target")
        S.no_problems(self, bad)


class AC7RepoRules(unittest.TestCase):
    def test_AC7_codeowners_and_dependabot(self):
        bad = []
        co = REPO / ".github" / "CODEOWNERS"
        if not co.exists():
            bad.append("no .github/CODEOWNERS")
        else:
            owners = {}
            for ln in co.read_text(encoding="utf-8").splitlines():
                ln = ln.split("#", 1)[0].strip()
                if ln:
                    parts = ln.split()
                    owners[parts[0].lstrip("/")] = parts[1:]
            for p in (".github/", "site/", "wrangler.jsonc", "scripts/site/", "docs/adr/", "packaging/keys/"):
                if "@M7MMAD-OMAR" not in owners.get(p, []):
                    bad.append(f"CODEOWNERS: {p} not owned by @M7MMAD-OMAR")
            for p in owners:
                if p != "*" and not (REPO / p.rstrip("/")).exists():
                    bad.append(f"CODEOWNERS path missing in repo: {p}")
        dep = REPO / ".github" / "dependabot.yml"
        if not dep.exists():
            bad.append("no .github/dependabot.yml")
        else:
            import yaml
            doc = yaml.safe_load(dep.read_text(encoding="utf-8")) or {}
            ok = [u for u in doc.get("updates") or []
                  if u.get("package-ecosystem") == "github-actions" and u.get("directory") == "/"
                  and (u.get("schedule") or {}).get("interval") == "weekly"]
            if doc.get("version") != 2 or not ok:
                bad.append("dependabot: no github-actions, directory /, weekly entry")
        S.no_problems(self, bad)


def owner_steps_from_story():
    story = (REPO / "backlog" / "S-059-site-deploy-cloudflare.md").read_text(encoding="utf-8")
    m = re.search(r"^## Owner steps, not automated\n.*?(?=^## )", story, re.S | re.M)
    return m.group(0).rstrip() if m else None


class AC8SecurityDocs(unittest.TestCase):
    def test_AC8_site_security_docs(self):
        en = REPO / "docs" / "site-security.md"
        ar = REPO / "docs" / "site-security.ar.md"
        bad = []
        missing = [p.name for p in (en, ar) if not p.exists()]
        self.assertEqual(missing, [], f"missing docs: {missing}")
        first = "[English](site-security.md) | [العربية](site-security.ar.md)"
        host = socket.gethostname().split(".")[0].lower()
        texts = {}
        for p in (en, ar):
            t = p.read_text(encoding="utf-8")
            texts[p.name] = t
            if t.splitlines()[0] != first:
                bad.append(f"{p.name}: first line {t.splitlines()[0]!r}")
            n = len(re.findall(r"^## ", t, re.M))
            if n != 9:
                bad.append(f"{p.name}: {n} level-2 headings, want 9")
            if re.search(r"[0-9a-fA-F]{32}", t):
                bad.append(f"{p.name}: account id like value")
            if re.search(r"[\w.+-]+@[\w-]+\.[A-Za-z]{2,}", t):
                bad.append(f"{p.name}: email address")
            if host and host in t.lower():
                bad.append(f"{p.name}: machine name {host}")
            for ph in ("<ACCOUNT_ID>", "<ZONE_ID>", "<EXPIRY_DATE>"):
                if ph not in t:
                    bad.append(f"{p.name}: no placeholder {ph}")
        t = texts.get("site-security.md", "")
        for kw in ("Workers", "token", "Content-Security-Policy", "Strict-Transport-Security", "DNS",
                   "TLS", "rivacy", "ollback", "rotat", "routes"):
            if kw not in t:
                bad.append(f"site-security.md does not mention {kw}")
        steps = owner_steps_from_story()
        secs = re.split(r"(?=^## )", t, flags=re.M)
        if steps is None or not secs or secs[-1].rstrip() != steps:
            bad.append("last section of site-security.md is not the story's Owner steps byte for byte")
        S.no_problems(self, bad)


class AC9Finish(unittest.TestCase):
    def test_AC9_changelog_design_lint_and_no_long_dashes(self):
        bad = []
        base = subprocess.run(["git", "merge-base", "main", "HEAD"], cwd=REPO, capture_output=True, text=True).stdout.strip()
        self.assertTrue(base, "no merge base with main")
        cl = subprocess.run(["git", "diff", base, "--", "CHANGELOG.md"], cwd=REPO, capture_output=True, text=True).stdout
        added = [ln for ln in cl.splitlines() if ln.startswith("+") and not ln.startswith("+++") and "S-059" in ln]
        if len(added) != 1 or not re.search(r"[\u0600-\u06FF]", added[0]) or not re.search(r"[A-Za-z]{4}", added[0]):
            bad.append(f"CHANGELOG: {len(added)} added S-059 lines with English and Arabic, want 1")
        dl = subprocess.run(["bash", "scripts/design-lint.sh"], cwd=REPO, capture_output=True, text=True)
        if dl.returncode != 0:
            bad.append(f"design-lint exit {dl.returncode}: {(dl.stdout + dl.stderr)[-300:]}")
        diff = subprocess.run(["git", "diff", base], cwd=REPO, capture_output=True, text=True).stdout
        dashes = [ln for ln in diff.splitlines() if ln.startswith("+") and re.search("[\u2013\u2014]", ln)]
        if dashes:
            bad.append(f"{len(dashes)} added lines with long dashes")
        S.no_problems(self, bad)


if __name__ == "__main__":
    unittest.main(verbosity=2)
