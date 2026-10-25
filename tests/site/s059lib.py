"""Shared helpers for the S-059 deploy tests. Standard library only.

The staged upload set is built in a COPY of the repo (temp dir) with
scripts/prepare-site-deploy.sh, so the real site/ and .deploy-out/ are never
touched. Planted file cases work on further temp copies of that staged folder.
"""
import fnmatch
import functools
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import lib  # noqa: E402

REPO = lib.REPO
TMP = lib.TMP
DOMAIN = "hive.sbarah.com"
CHECK = "scripts/site/check-deploy.sh"


def sh(cmd, cwd, env=None, timeout=900):
    e = dict(os.environ)
    e.update(env or {})
    try:
        return subprocess.run(cmd, cwd=cwd, env=e, capture_output=True, text=True, timeout=timeout)
    except FileNotFoundError as exc:
        # A missing script reads as exit 127, like a shell would report it.
        return subprocess.CompletedProcess(cmd, 127, "", f"not found: {exc}")
    except PermissionError as exc:
        return subprocess.CompletedProcess(cmd, 126, "", f"not executable: {exc}")


def make_root(name):
    """Copy what the deploy scripts read into TMP/name (a git repo)."""
    dst = lib.make_copy(name)
    for f in ("wrangler.jsonc", "SECURITY.md", "LICENSE"):
        if (REPO / f).exists():
            shutil.copy(REPO / f, dst / f)
    return dst


@functools.lru_cache(maxsize=None)
def staged_root():
    """A temp repo copy after scripts/prepare-site-deploy.sh. Returns (root, result)."""
    root = make_root("s059-staged")
    r = sh(["bash", "scripts/prepare-site-deploy.sh"], root)
    return root, r


def staged():
    root, r = staged_root()
    out = root / ".deploy-out"
    assert r.returncode == 0 and out.is_dir(), (
        f"prepare-site-deploy.sh failed (exit {r.returncode}): {r.stdout[-400:]} {r.stderr[-800:]}")
    return out


def run_check(root, *args):
    """Run scripts/site/check-deploy.sh from a repo root, executed directly."""
    return sh([str(root / CHECK), *args], root)


# ---- _headers ---------------------------------------------------------------
def parse_headers(text):
    """Cloudflare _headers: a path line at column 0, indented 'Name: value' lines."""
    rules = []
    for line in text.splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        if not line[0].isspace():
            rules.append((line.strip(), []))
        elif rules and ":" in line:
            k, v = line.strip().split(":", 1)
            rules[-1][1].append((k.strip(), v.strip()))
    return rules


def rule_regex(pattern):
    """Path pattern to a regex: '*' is a splat, ':name' one segment."""
    out = ""
    for part in re.split(r"(\*|:[A-Za-z]\w*)", pattern):
        if part == "*":
            out += ".*"
        elif part.startswith(":") and len(part) > 1:
            out += "[^/]+"
        else:
            out += re.escape(part)
    return re.compile("^" + out + "$")


def headers_for(rules, path):
    got = {}
    for pat, hs in rules:
        if rule_regex(pat).match(path):
            for k, v in hs:
                got[k] = (got[k] + ", " + v) if k in got else v
    return got


def csp_directives(csp):
    d = {}
    for part in csp.split(";"):
        toks = part.split()
        if toks:
            d[toks[0].lower()] = toks[1:]
    return d


# ---- .assetsignore matcher (test side, independent of the script) ----------
def ignore_patterns(folder):
    p = Path(folder) / ".assetsignore"
    if not p.exists():
        return []
    return [ln.strip() for ln in p.read_text(encoding="utf-8").splitlines()
            if ln.strip() and not ln.strip().startswith("#")]


def ignored(rel, patterns):
    parts = rel.split("/")
    for pat in patterns:
        if pat.endswith("/"):
            if pat.rstrip("/") in parts[:-1]:
                return True
            continue
        p = pat.lstrip("/")
        if "/" in p:
            if fnmatch.fnmatch(rel, p):
                return True
        elif any(fnmatch.fnmatch(seg, p) for seg in parts):
            return True
    return False


def no_problems(case, bad, limit=40):
    """Fail with one problem per line, so the real failure lines are readable."""
    if bad:
        more = f"\n... and {len(bad) - limit} more" if len(bad) > limit else ""
        case.fail(f"{len(bad)} problems:\n" + "\n".join(bad[:limit]) + more)
