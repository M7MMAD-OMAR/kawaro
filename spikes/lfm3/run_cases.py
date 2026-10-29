#!/usr/bin/python3.14 -I
"""Throwaway spike: capability survey. Runs text_cases or vl_cases through llama-server
(thinking off, temperature 0) and scores them with check_text.py / check_vl.py.

usage: run_cases.py text|vl MODEL TAG [MMPROJ]
"""
import base64, importlib.util, json, os, secrets, signal, subprocess, sys, time, urllib.request
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
HOME = Path.home() / ".local/share/kawaro-spike/lfm3"
SERVER = next((HOME / "runtime/b11972").rglob("llama-server"))
kind, model, tag = sys.argv[1:4]
mmproj = sys.argv[4] if len(sys.argv) > 4 else None
chk = importlib.util.spec_from_file_location("chk", HERE / f"check_{'text' if kind == 'text' else 'vl'}.py")
mod = importlib.util.module_from_spec(chk); chk.loader.exec_module(mod)
cases = [json.loads(l) for l in (HERE / f"fixtures/cases/{kind}_cases.jsonl").read_text().splitlines() if l.strip()]
key = secrets.token_hex(16)
cmd = [str(SERVER), "-m", model, "--host", "127.0.0.1", "--port", "18393", "--api-key", key, "--no-webui",
       "-c", "8192", "-np", "1", "-t", "4", "--no-warmup"]
if mmproj:
    cmd += ["--mmproj", mmproj]
proc = subprocess.Popen(cmd, stdout=open(HERE / f"out/server-cases-{tag}.log", "w"), stderr=subprocess.STDOUT, start_new_session=True)


def post(body):
    req = urllib.request.Request("http://127.0.0.1:18393/v1/chat/completions", data=json.dumps(body).encode(),
                                 headers={"Content-Type": "application/json", "Authorization": f"Bearer {key}"})
    return json.load(urllib.request.urlopen(req, timeout=900))


try:
    while True:
        try:
            urllib.request.urlopen("http://127.0.0.1:18393/health", timeout=2); break
        except Exception:
            if proc.poll() is not None:
                sys.exit("server exited")
            time.sleep(0.5)
    rows = []
    for c in cases:
        if kind == "text":
            msgs = [{"role": "system", "content": c["system"]}, {"role": "user", "content": c["user"]}]
        else:
            b64 = base64.b64encode((HERE / "fixtures/cases" / c["image"]).read_bytes()).decode()
            msgs = [{"role": "user", "content": [
                {"type": "image_url", "image_url": {"url": "data:image/png;base64," + b64}},
                {"type": "text", "text": c["prompt"]}]}]
        t = time.time()
        res = post({"messages": msgs, "temperature": 0, "max_tokens": c.get("max_tokens", 400),
                    "chat_template_kwargs": {"enable_thinking": False}})
        wall = time.time() - t
        out = res["choices"][0]["message"]["content"]
        ok, why = mod.score(c, out)
        tm = res.get("timings", {})
        rows.append({"id": c["id"], "category": c.get("category") or c.get("kind"), "lang": c["lang"], "ok": ok,
                     "why": why, "wall_s": round(wall, 2), "prompt_n": tm.get("prompt_n"),
                     "gen_n": tm.get("predicted_n"), "out": out[:400]})
        print(c["id"], ok, why[:60], round(wall, 1), flush=True)
    st = {l.split(":")[0]: int(l.split()[1]) for l in Path(f"/proc/{proc.pid}/status").read_text().splitlines()
          if l.startswith(("VmHWM",))}
    by = defaultdict(list)
    for r in rows:
        by[r["category"]].append(r["ok"])
        by["lang:" + r["lang"]].append(r["ok"])
    summ = {"tag": tag, "model": os.path.basename(model), "n": len(rows), "pass": sum(r["ok"] for r in rows),
            "by": {k: f"{sum(v)}/{len(v)}" for k, v in by.items()}, "peak_rss_mb": st["VmHWM"] // 1024,
            "median_wall_s": sorted(r["wall_s"] for r in rows)[len(rows) // 2], "rows": rows}
    (HERE / f"out/cases-{tag}.json").write_text(json.dumps(summ, ensure_ascii=False, indent=1))
    print({k: v for k, v in summ.items() if k != "rows"})
finally:
    os.killpg(proc.pid, signal.SIGTERM)
