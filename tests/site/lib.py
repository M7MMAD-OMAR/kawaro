"""Shared helpers for the S-061 site tests. Standard library only.

Every test works on a COPY of the repo in a temp dir, so the committed site/
output is never touched. Builds are cached per variant for the whole run.
Browser tests import playwright lazily (run those without -I, see run_all.sh).
"""
import atexit
import functools
import html.parser
import http.server
import os
import re
import shutil
import subprocess
import tempfile
import threading
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
TMP = Path(os.environ.get("S061_TMP") or tempfile.mkdtemp(prefix="s061-"))
atexit.register(lambda: None if os.environ.get("S061_KEEP") else shutil.rmtree(TMP, ignore_errors=True))

VERSION = "0.5.0"
TAG = "v" + VERSION
RELEASES = "https://github.com/M7MMAD-OMAR/kawaro/releases"
SUPPORT = "https://sbarah.com/support"
LANGS = ("ar", "en")
WIDTHS = (375, 768, 1280)


def sh(cmd, cwd, env=None, check=False):
    e = dict(os.environ)
    e.update(env or {})
    return subprocess.run(cmd, cwd=cwd, env=e, capture_output=True, text=True, check=check)


def make_copy(name, tag=False):
    """Copy what the build reads into TMP/name and make it a git repo."""
    dst = TMP / name
    if dst.exists():
        shutil.rmtree(dst)
    dst.mkdir(parents=True)
    ign = shutil.ignore_patterns("pagefind", "target", ".git", "__pycache__")
    for d in ("site", "scripts", "design", "docs", "actions"):
        shutil.copytree(REPO / d, dst / d, ignore=ign)
    for f in ["CHANGELOG.md", "Cargo.toml", "Cargo.lock"] + [p.name for p in REPO.glob("README*.md")]:
        if (REPO / f).exists():
            shutil.copy(REPO / f, dst / f)
    sh(["git", "init", "-q"], dst, check=True)
    sh(["git", "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", "x"], dst, check=True)
    if tag:
        sh(["git", "tag", TAG], dst, check=True)
    return dst


def run_build(root, published=None):
    env = {}
    if published is not None:
        env["RELEASE_PUBLISHED"] = str(published)
    else:
        env["RELEASE_PUBLISHED"] = ""
        env.pop("RELEASE_PUBLISHED")
    e = dict(os.environ)
    e.pop("RELEASE_PUBLISHED", None)
    e.update({k: v for k, v in env.items() if v != ""})
    r1 = subprocess.run(["python3", "scripts/tokens-to-css.py"], cwd=root, env=e, capture_output=True, text=True)
    if r1.returncode:
        return r1
    return subprocess.run(["python3", "site/build.py"], cwd=root, env=e, capture_output=True, text=True)


@functools.lru_cache(maxsize=None)
def built(variant="0"):
    """variant: '0' unset env (default), 'e0' explicit 0, '1' published with tag, 'rep' second copy of default."""
    if variant == "1":
        root = make_copy("b1", tag=True)
        r = run_build(root, 1)
    elif variant == "e0":
        root = make_copy("be0")
        r = run_build(root, 0)
    elif variant == "rep":
        root = make_copy("brep")
        r = run_build(root, None)
    else:
        root = make_copy("b0")
        r = run_build(root, None)
    assert r.returncode == 0, f"build {variant} failed: {r.stdout[-400:]} {r.stderr[-800:]}"
    return root


def site(variant="0"):
    return built(variant) / "site"


def page_path(lang, page=""):
    parts = (["en"] if lang == "en" else []) + ([page] if page else [])
    return "/" + "/".join(parts) + ("/" if parts else "")


def page_file(lang, page="", variant="0"):
    p = site(variant)
    if lang == "en":
        p = p / "en"
    if page:
        p = p / page
    return p / "index.html"


def html_files(variant="0"):
    return sorted(p for p in site(variant).rglob("*.html") if "pagefind" not in p.parts)


# ---- tiny DOM -------------------------------------------------------------
VOID = {"area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"}


class Node:
    def __init__(self, tag, attrs, parent):
        self.tag, self.attrs, self.parent, self.children = tag, dict(attrs), parent, []

    def cls(self):
        return (self.attrs.get("class") or "").split()

    def walk(self):
        yield self
        for c in self.children:
            if isinstance(c, Node):
                yield from c.walk()

    def find_all(self, sel):
        return [n for n in self.walk() if n is not self and match(n, sel)]

    def text(self):
        out = []
        for c in self.children:
            out.append(c.text() if isinstance(c, Node) else c)
        return "".join(out)

    def ancestors(self):
        n = self.parent
        while n is not None:
            yield n
            n = n.parent


def match(n, sel):
    """Selectors: tag, .class, #id, tag.class, [attr], [attr=value], combos without spaces."""
    for m in re.finditer(r"([a-z0-9]+)|\.([\w-]+)|#([\w-]+)|\[([\w-]+)(?:=\"?([^\]\"]*)\"?)?\]", sel):
        tag, c, i, a, v = m.groups()
        if tag and n.tag != tag:
            return False
        if c and c not in n.cls():
            return False
        if i and n.attrs.get("id") != i:
            return False
        if a and (a not in n.attrs or (v is not None and n.attrs[a] != v)):
            return False
    return True


class _P(html.parser.HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.root = Node("#root", {}, None)
        self.cur = self.root

    def handle_starttag(self, tag, attrs):
        n = Node(tag, [(k, v if v is not None else "") for k, v in attrs], self.cur)
        self.cur.children.append(n)
        if tag not in VOID:
            self.cur = n

    def handle_startendtag(self, tag, attrs):
        n = Node(tag, [(k, v if v is not None else "") for k, v in attrs], self.cur)
        self.cur.children.append(n)

    def handle_endtag(self, tag):
        n = self.cur
        while n is not None and n.tag != tag:
            n = n.parent
        if n is not None and n.parent is not None:
            self.cur = n.parent

    def handle_data(self, data):
        self.cur.children.append(data)


def parse(text):
    p = _P()
    p.feed(text)
    return p.root


def parse_file(path):
    return parse(Path(path).read_text(encoding="utf-8"))


# ---- CSS ------------------------------------------------------------------
def strip_css_comments(css):
    return re.sub(r"/\*.*?\*/", "", css, flags=re.S)


def css_rules(css):
    """Yield (selector, {prop: value}) for every innermost rule, @media flattened."""
    css = strip_css_comments(css)
    for m in re.finditer(r"([^{}]+)\{([^{}]*)\}", css):
        sel = m.group(1).strip()
        if sel.startswith("@"):
            continue
        decls = {}
        for d in m.group(2).split(";"):
            if ":" in d:
                k, v = d.split(":", 1)
                decls[k.strip()] = v.strip()
        yield sel, decls


SPACING_PROP = re.compile(r"^(margin|padding|inset)(-[a-z-]+)?$|^(gap|row-gap|column-gap)$")
ALLOWED_TOKEN = re.compile(r"^(0|auto|var\(--space-\d+\)|calc\((?:[\s*+\-/\d.]|var\(--space-\d+\))+\))$")


def split_top(value):
    out, depth, cur = [], 0, ""
    for ch in value:
        depth += ch == "("
        depth -= ch == ")"
        if ch.isspace() and depth == 0:
            if cur:
                out.append(cur)
            cur = ""
        else:
            cur += ch
    if cur:
        out.append(cur)
    return out


def spacing_violations(css):
    bad = []
    for sel, decls in css_rules(css):
        for prop, val in decls.items():
            if SPACING_PROP.match(prop):
                for tok in split_top(val.replace("!important", "")):
                    if not ALLOWED_TOKEN.match(tok):
                        bad.append(f"{sel} {{ {prop}: {val} }}")
                        break
    return bad


# ---- static server + browser ---------------------------------------------
class _Quiet(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *a):
        pass


def serve(directory):
    h = functools.partial(_Quiet, directory=str(directory))
    srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), h)
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    return srv, f"http://127.0.0.1:{srv.server_address[1]}"


class Browser:
    """Context manager: serves a built site and gives a Playwright browser."""

    def __init__(self, variant="0"):
        self.variant = variant

    def __enter__(self):
        from playwright.sync_api import sync_playwright
        self.srv, self.base = serve(site(self.variant))
        self.pw = sync_playwright().start()
        import glob
        found = sorted(glob.glob(os.path.expanduser("~/.cache/ms-playwright/chromium-*/chrome-linux64/chrome")))
        exe = os.environ.get("S061_CHROMIUM") or (found[-1] if found else None)
        self.browser = self.pw.chromium.launch(executable_path=exe)
        return self

    def __exit__(self, *a):
        self.browser.close()
        self.pw.stop()
        self.srv.shutdown()

    def page(self, path, width=1280, height=800, scheme="light", lang_ua=None, reduced=False, js=True, user_agent=None):
        ctx = self.browser.new_context(
            viewport={"width": width, "height": height},
            color_scheme=scheme,
            reduced_motion="reduce" if reduced else "no-preference",
            java_script_enabled=js,
            user_agent=user_agent,
        )
        ctx.set_default_timeout(15000)
        pg = ctx.new_page()
        pg.requests = []
        pg.on("request", lambda r: pg.requests.append(r.url))
        pg.goto(self.base + path, wait_until="load")
        return pg
