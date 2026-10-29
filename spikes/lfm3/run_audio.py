#!/usr/bin/python3.14 -I
"""Throwaway spike: ASR with LFM3-Omni-2B through llama-server (input_audio).
usage: run_audio.py MODEL MMPROJ TAG
"""
import base64, json, os, re, secrets, signal, subprocess, sys, time, unicodedata, urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
HOME = Path.home() / ".local/share/kawaro-spike/lfm3"
SERVER = next((HOME / "runtime/b11972").rglob("llama-server"))
model, mmproj, tag = sys.argv[1:4]
key = secrets.token_hex(16)
proc = subprocess.Popen([str(SERVER), "-m", model, "--mmproj", mmproj, "--host", "127.0.0.1", "--port", "18394",
                         "--api-key", key, "--no-webui", "-c", "8192", "-np", "1", "-t", "4", "--no-warmup"],
                        stdout=open(HERE / f"out/server-audio-{tag}.log", "w"), stderr=subprocess.STDOUT,
                        start_new_session=True)


def lev(a, b):
    prev = list(range(len(b) + 1))
    for i, ca in enumerate(a, 1):
        cur = [i]
        for j, cb in enumerate(b, 1):
            cur.append(min(prev[j] + 1, cur[j - 1] + 1, prev[j - 1] + (ca != cb)))
        prev = cur
    return prev[-1]


def norm(s, ar):
    s = unicodedata.normalize("NFKC", s).lower()
    if ar:
        s = re.sub(r"[ؐ-ًؚ-ٰٟـ]", "", s)
        s = re.sub(r"[آأإٱ]", "ا", s)
    s = re.sub(r"[^\w\s]", " ", s)
    return re.sub(r"\s+", " ", s).strip()


def post(body):
    req = urllib.request.Request("http://127.0.0.1:18394/v1/chat/completions", data=json.dumps(body).encode(),
                                 headers={"Content-Type": "application/json", "Authorization": f"Bearer {key}"})
    return json.load(urllib.request.urlopen(req, timeout=900))


try:
    while True:
        try:
            urllib.request.urlopen("http://127.0.0.1:18394/health", timeout=2); break
        except Exception:
            if proc.poll() is not None:
                sys.exit("server exited")
            time.sleep(0.5)
    rows = []
    for wav in sorted((HERE / "fixtures/cases/audio").glob("*.wav")):
        ref = wav.with_suffix(".txt").read_text()
        ar = wav.name.startswith("ar")
        b64 = base64.b64encode(wav.read_bytes()).decode()
        t = time.time()
        res = post({"messages": [
            {"role": "system", "content": "Perform ASR in " + ("Arabic." if ar else "English.")},
            {"role": "user", "content": [{"type": "input_audio", "input_audio": {"data": b64, "format": "wav"}}]}],
            "temperature": 0, "max_tokens": 300, "chat_template_kwargs": {"enable_thinking": False}})
        wall = time.time() - t
        out = res["choices"][0]["message"]["content"]
        h, r = norm(out, ar), norm(ref, ar)
        row = {"file": wav.name, "cer": round(lev(h, r) / max(len(r), 1), 3),
               "wer": round(lev(h.split(), r.split()) / max(len(r.split()), 1), 3), "wall_s": round(wall, 2),
               "ref": ref.strip(), "hyp": out.strip()}
        rows.append(row)
        print({k: row[k] for k in ("file", "cer", "wer", "wall_s")}, row["hyp"][:80], flush=True)
    rss = next(int(l.split()[1]) for l in Path(f"/proc/{proc.pid}/status").read_text().splitlines() if l.startswith("VmHWM"))
    (HERE / f"out/audio-{tag}.json").write_text(json.dumps({"tag": tag, "peak_rss_mb": rss // 1024, "rows": rows},
                                                          ensure_ascii=False, indent=1))
    print("peak_rss_mb", rss // 1024)
finally:
    os.killpg(proc.pid, signal.SIGTERM)
