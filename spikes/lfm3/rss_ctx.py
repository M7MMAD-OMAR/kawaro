#!/usr/bin/python3.14 -I
"""Peak and settled RSS of llama-server per context size after one tiny request."""
import json, os, secrets, signal, subprocess, sys, time, urllib.request
from pathlib import Path

HOME = Path.home() / ".local/share/kawaro-spike/lfm3"
SERVER = next((HOME / "runtime/b11972").rglob("llama-server"))
model = sys.argv[1]
extra = sys.argv[2:]  # e.g. --mmproj path
for ctx in (2048, 8192, 32768, 131072):
    key = secrets.token_hex(8)
    p = subprocess.Popen([str(SERVER), "-m", model, *extra, "--host", "127.0.0.1", "--port", "18392", "--api-key", key,
                          "--no-webui", "-c", str(ctx), "-np", "1", "-t", "4"],
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)
    try:
        for _ in range(240):
            try:
                urllib.request.urlopen("http://127.0.0.1:18392/health", timeout=2); break
            except Exception:
                time.sleep(0.5)
        req = urllib.request.Request("http://127.0.0.1:18392/v1/chat/completions", headers={
            "Content-Type": "application/json", "Authorization": f"Bearer {key}"},
            data=json.dumps({"messages": [{"role": "user", "content": "Say hi."}], "max_tokens": 8,
                             "chat_template_kwargs": {"enable_thinking": False}}).encode())
        urllib.request.urlopen(req, timeout=300).read()
        st = {l.split(":")[0]: int(l.split()[1]) for l in Path(f"/proc/{p.pid}/status").read_text().splitlines()
              if l.startswith(("VmHWM", "VmRSS"))}
        print(f"ctx {ctx}: settled {st['VmRSS']//1024} MB, peak {st['VmHWM']//1024} MB", flush=True)
    finally:
        os.killpg(p.pid, signal.SIGTERM); time.sleep(1)
