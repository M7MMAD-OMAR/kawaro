#!/usr/bin/env bash
# Checks the website upload set before a deploy (S-059, ADR 0007 section 6).
#   scripts/site/check-deploy.sh            check the assets directory named in wrangler.jsonc
#   scripts/site/check-deploy.sh --build    stage it first with scripts/prepare-site-deploy.sh
#   scripts/site/check-deploy.sh FOLDER     check another folder (used by the tests)
# Exit 0 and "upload set: N files" when clean; exit 1 naming each bad file or
# setting; exit 2 when wrangler.jsonc or the folder's _headers is missing.
# Uses its own matcher, not the output of `wrangler deploy` in dry run mode.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
FOLDER=""
case "${1:-}" in
  --build)
    if ! "$ROOT/scripts/prepare-site-deploy.sh"; then
      echo "check-deploy: build failed" >&2
      exit 1
    fi
    ;;
  "") ;;
  *) FOLDER="$1" ;;
esac
if [ ! -f "$ROOT/wrangler.jsonc" ]; then
  echo "check-deploy: wrangler.jsonc is missing in $ROOT" >&2
  exit 2
fi
exec python3 - "$ROOT" "$FOLDER" <<'PY'
import fnmatch, json, re, sys
from pathlib import Path

root, folder = Path(sys.argv[1]), sys.argv[2]
bad = []
cfg = json.loads((root / "wrangler.jsonc").read_text(encoding="utf-8"))
for key in ("main", "account_id", "vars", "kv_namespaces", "r2_buckets", "d1_databases", "services", "secrets"):
    if key in cfg:
        bad.append(f"wrangler.jsonc: forbidden key {key}")
if cfg.get("workers_dev") is not False:
    bad.append("wrangler.jsonc: workers_dev must be false")
if cfg.get("routes", []) not in ([], [{"pattern": "kawaro.sbarah.com", "custom_domain": True}]):
    bad.append("wrangler.jsonc: routes must be the one kawaro.sbarah.com custom domain")
for value in re.findall(r'"[^"]*"', json.dumps(cfg)):
    if re.search(r"[A-Fa-f0-9]{32}", value) or "token" in value.lower():
        bad.append(f"wrangler.jsonc: secret like value {value}")
assets = Path(folder) if folder else root / (cfg.get("assets") or {}).get("directory", "")
if not (assets / "_headers").is_file():
    print(f"check-deploy: {assets}/_headers is missing (run with --build)", file=sys.stderr)
    sys.exit(2)

ALLOWED_EXT = {"html", "css", "js", "json", "xml", "txt", "md", "svg", "png", "webp", "mp4", "webm", "woff2", "webmanifest",
               "pagefind", "pf_meta", "pf_index", "pf_fragment", "pf_filter", "pf_dir"}
SPECIAL = {"_headers", ".assetsignore", "OFL.txt"}
FORBIDDEN_DIRS = {".git", "node_modules", "backlog", "__pycache__"}
KEY = re.compile(rb"-----BEGIN [A-Z ]*PRIVATE KEY-----")
MAX_BYTES = 25 * 1024 * 1024
ignore = []
if (assets / ".assetsignore").is_file():
    ignore = [ln.strip() for ln in (assets / ".assetsignore").read_text(encoding="utf-8").splitlines()
              if ln.strip() and not ln.strip().startswith("#")]

def ignored(rel):
    parts = rel.split("/")
    for pat in ignore:
        if pat.endswith("/"):
            if pat.rstrip("/") in parts[:-1]:
                return True
        elif any(fnmatch.fnmatchcase(seg, pat.lstrip("/")) for seg in parts):
            return True
    return False

count = 0
for p in sorted(assets.rglob("*")):
    if not p.is_file() and not p.is_symlink():
        continue
    rel = p.relative_to(assets).as_posix()
    parts = rel.split("/")
    if p.is_symlink():
        bad.append(f"{rel}: symbolic link")
        continue
    if FORBIDDEN_DIRS.intersection(parts[:-1]):
        bad.append(f"{rel}: under a forbidden directory")
        continue
    if ignored(rel):
        bad.append(f"{rel}: matches .assetsignore, must not be in the staged folder")
        continue
    count += 1
    name = parts[-1]
    ext = name.rsplit(".", 1)[-1] if "." in name.lstrip(".") else ""
    if name not in SPECIAL and ext not in ALLOWED_EXT:
        bad.append(f"{rel}: extension not allowed")
    size = p.stat().st_size
    if size > MAX_BYTES:
        bad.append(f"{rel}: {size} bytes, over 25 MiB")
        continue
    if KEY.search(p.read_bytes()):
        bad.append(f"{rel}: contains a private key")
if count > 20000:
    bad.append(f"{count} files, over 20000")
for line in bad:
    print(f"check-deploy: {line}", file=sys.stderr)
if bad:
    sys.exit(1)
print(f"upload set: {count} files")
PY
