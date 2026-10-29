#!/usr/bin/python3.14 -I
"""Throwaway spike: natural language to action with a grammar constrained
llama-server. Reads fixtures/requests.jsonl and fixtures/actions.json, writes
out/actions-<tag>.json with accuracy, prompt/decode speed and peak RSS.

usage: run_actions.py MODEL_GGUF TAG [THREADS]
"""
import json
import os
import secrets
import signal
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
HOME = Path.home() / ".local/share/kawaro-spike/lfm3"
SERVER = next((HOME / "runtime/b11972").rglob("llama-server"))
PORT = 18391

model, tag = sys.argv[1], sys.argv[2]
threads = sys.argv[3] if len(sys.argv) > 3 else "4"

catalog = json.loads((HERE / "fixtures/actions.json").read_text())
requests = [json.loads(line) for line in (HERE / "fixtures/requests.jsonl").read_text().splitlines() if line.strip()]
ids = [a["id"] for a in catalog] + ["none"]

SYSTEM = (
    "You pick one file action for a file converter. Reply with JSON only: "
    '{"action": <id or "none">, "params": {...}}. '
    'Use "none" when no listed action fits or the request is unclear. '
    "Use only ids and parameter names from the catalog. Never invent either.\n"
    "Catalog:\n" + json.dumps(catalog, ensure_ascii=False, separators=(",", ":"))
)
SCHEMA = {
    "type": "object",
    "properties": {
        "action": {"enum": ids},
        "params": {"type": "object", "additionalProperties": {"type": ["number", "string", "boolean"]}},
    },
    "required": ["action", "params"],
    "additionalProperties": False,
}

if os.environ.get("TUNED"):
    # tuned variant: one schema branch per action (only its own params), compact catalog
    def pschema(d):
        p = {"type": "integer" if d["type"] == "int" else "string"}
        if "enum" in d:
            p["enum"] = d["enum"]
        if "min" in d:
            p["minimum"] = d["min"]
        if "max" in d:
            p["maximum"] = d["max"]
        return p

    branches = [{"type": "object", "properties": {"action": {"const": a["id"]}, "params": {
        "type": "object", "properties": {k: pschema(d) for k, d in (a.get("params") or {}).items()},
        "additionalProperties": False}}, "required": ["action", "params"], "additionalProperties": False}
        for a in catalog]
    branches.append({"type": "object", "properties": {"action": {"const": "none"}, "params": {
        "type": "object", "maxProperties": 0}}, "required": ["action", "params"], "additionalProperties": False})
    SCHEMA = {"oneOf": branches}
    lines = []
    for a in catalog:
        ps = ",".join(f"{k}({d.get('min','')}-{d.get('max','')}{d.get('enum','')})" if 'enum' not in d else f"{k}{d['enum']}"
                      for k, d in (a.get("params") or {}).items())
        lines.append(f"{a['id']} [{'/'.join(a['accepts'])}] {a['label']['en']} / {a['label']['ar']}" + (f" params: {ps}" if ps else ""))
    SYSTEM = (
        "You pick one file action for a file converter. Reply with JSON only: "
        '{"action": <id or "none">, "params": {}}. Leave params empty unless the user states a value. '
        'Use "none" when no action fits or the request is unclear or unsafe.\nActions:\n' + "\n".join(lines))

key = secrets.token_hex(16)
log = open(HERE / f"out/server-{tag}.log", "w")
proc = subprocess.Popen(
    [str(SERVER), "-m", model, "--host", "127.0.0.1", "--port", str(PORT), "--api-key", key,
     "--no-webui", "-c", "8192", "-np", "1", "-t", threads, "--no-warmup"],
    stdout=log, stderr=subprocess.STDOUT, start_new_session=True,
)


def post(body):
    req = urllib.request.Request(
        f"http://127.0.0.1:{PORT}/v1/chat/completions",
        data=json.dumps(body).encode(),
        headers={"Content-Type": "application/json", "Authorization": f"Bearer {key}"},
    )
    with urllib.request.urlopen(req, timeout=600) as r:
        return json.load(r)


EXT = {"png": "image", "jpg": "image", "jpeg": "image", "webp": "image", "pdf": "pdf", "mp3": "audio",
       "wav": "audio", "m4a": "audio", "ogg": "audio", "mp4": "video", "mov": "video", "mkv": "video",
       "docx": "document", "odt": "document", "txt": "document"}
BYID = {a["id"]: a for a in catalog}


def registry_ok(out, files):
    """What the real registry would accept: known id, matching file kind, known params in range."""
    if out.get("action") == "none":
        return not out.get("params")
    a = BYID.get(out.get("action"))
    if a is None:
        return False
    kinds = {EXT.get(f.rsplit(".", 1)[-1].lower()) for f in files}
    if not kinds <= set(a["accepts"]):
        return False
    spec = a.get("params") or {}
    for k, v in (out.get("params") or {}).items():
        d = spec.get(k)
        if d is None:
            return False
        if "enum" in d and v not in d["enum"]:
            return False
        if d["type"] == "int" and (not isinstance(v, int) or isinstance(v, bool)
                                   or v < d.get("min", v) or v > d.get("max", v)):
            return False
    return True


def machine():
    cpu = next((l.split(":", 1)[1].strip() for l in Path("/proc/cpuinfo").read_text().splitlines()
                if l.startswith("model name")), "?")
    return {"cpu": cpu, "loadavg": os.getloadavg(), "affinity": sorted(os.sched_getaffinity(0))}


def peak_rss_kb():
    for line in Path(f"/proc/{proc.pid}/status").read_text().splitlines():
        if line.startswith("VmHWM"):
            return int(line.split()[1])
    return 0


try:
    t0 = time.time()
    while True:
        try:
            urllib.request.urlopen(urllib.request.Request(f"http://127.0.0.1:{PORT}/health"), timeout=2)
            break
        except Exception:
            if proc.poll() is not None:
                sys.exit("server exited, see out/server log")
            time.sleep(0.5)
    load_s = time.time() - t0

    rows = []
    cold = None
    for r in requests[: int(os.environ.get("LIMIT", "999"))]:
        user = f"Files: {', '.join(r.get('files', []))}\nRequest: {r['text']}"
        t = time.time()
        res = post({
            "messages": [{"role": "system", "content": SYSTEM}, {"role": "user", "content": user}],
            "temperature": 0, "max_tokens": 200, "chat_template_kwargs": {"enable_thinking": False}, "cache_prompt": cold is not None,
            "response_format": {"type": "json_schema", "json_schema": {"name": "pick", "schema": SCHEMA}},
        })
        wall = time.time() - t
        raw = res["choices"][0]["message"]["content"]
        try:
            out = json.loads(raw)
        except Exception:
            out = {"action": "<unparsable>", "params": {}}
        exp_p = r.get("expected_params", {})
        got_p = out.get("params", {})
        rows.append({
            "raw": raw, "msg": res["choices"][0]["message"], "finish": res["choices"][0].get("finish_reason"), "id": r["id"], "lang": r["lang"], "expected": r["expected_action"], "got": out.get("action"),
            "action_ok": out.get("action") == r["expected_action"],
            "params_ok": all(got_p.get(k) == v for k, v in exp_p.items()),
            "expected_params": exp_p, "got_params": got_p,
            "prompt_ms": res.get("timings", {}).get("prompt_ms"),
            "prompt_n": res.get("timings", {}).get("prompt_n"),
            "cache_n": res.get("timings", {}).get("cache_n"),
            "decode_tps": res.get("timings", {}).get("predicted_per_second"),
            "wall_s": round(wall, 2),
            "registry_ok": registry_ok(out, r.get("files", [])),
        })
        if cold is None:
            cold = {"wall_s": round(wall, 2), "prompt_n": rows[-1]["prompt_n"], "prompt_ms": rows[-1]["prompt_ms"]}
        print(rows[-1]["id"], rows[-1]["lang"], rows[-1]["expected"], "->", rows[-1]["got"], flush=True)

    def rate(sel):
        sel = list(sel)
        return round(sum(x["action_ok"] for x in sel) / len(sel), 3) if sel else None

    summary = {
        "tag": tag, "model": os.path.basename(model), "threads": threads, "load_s": round(load_s, 2),
        "action_acc": rate(rows), "action_acc_ar": rate(x for x in rows if x["lang"] == "ar"),
        "action_acc_en": rate(x for x in rows if x["lang"] == "en"),
        "params_acc": round(sum(x["params_ok"] for x in rows) / len(rows), 3),
        "peak_rss_mb": round(peak_rss_kb() / 1024),
        "cold_first_request": cold, "machine": machine(),
        "registry_rejected": sum(not x["registry_ok"] for x in rows),
        "none_cases_ok": [x["got"] == "none" for x in rows if x["expected"] == "none"],
        "median_wall_s": sorted(x["wall_s"] for x in rows)[len(rows) // 2],
        "rows": rows,
    }
    (HERE / f"out/actions-{tag}.json").write_text(json.dumps(summary, ensure_ascii=False, indent=1))
    print({k: v for k, v in summary.items() if k != "rows"})
finally:
    os.killpg(proc.pid, signal.SIGTERM)
