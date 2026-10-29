# Independent reference: Whisper (faster-whisper, base) on the same clips. Run with the spike venv python.
import json, os, re, sys, time, unicodedata, wave
import numpy as np
from pathlib import Path
from faster_whisper import WhisperModel

root = Path(os.environ["AUDIO_DIR"]); size = sys.argv[1]; tag = sys.argv[2]
snap = Path(os.environ["WHISPER_PATH"]) if os.environ.get("WHISPER_PATH") else next(Path.home().glob(f".cache/huggingface/hub/models--Systran--faster-whisper-{size}/snapshots/*"), size)
m = WhisperModel(str(snap), device="cpu", compute_type="int8", cpu_threads=4)
exec(open(Path(__file__).with_name("run_audio.py")).read().split("def lev")[1].join(["def lev", ""]).split("def post")[0])
rows = []
for wav in sorted(root.rglob("*.wav")):
    ar = wav.name.startswith(("ar", "fleurs_ar", "syrian"))
    ref = wav.with_suffix(".txt").read_text()
    t = time.time()
    with wave.open(str(wav)) as w:
        assert w.getframerate() == 16000 and w.getnchannels() == 1, (wav, w.getframerate(), w.getnchannels())
        pcm = np.frombuffer(w.readframes(w.getnframes()), dtype=np.int16).astype(np.float32) / 32768
    segs, _ = m.transcribe(pcm, language="ar" if ar else "en", beam_size=1, temperature=0)
    hyp = " ".join(s.text for s in segs).strip()
    h, r = norm(hyp, ar), norm(ref, ar)
    rows.append({"file": wav.name, "cer": round(lev(h, r) / max(len(r), 1), 3),
                 "wer": round(lev(h.split(), r.split()) / max(len(r.split()), 1), 3), "wall_s": round(time.time() - t, 2),
                 "ref": ref.strip(), "hyp": hyp})
    print({k: rows[-1][k] for k in ("file", "cer", "wer", "wall_s")}, flush=True)
Path(__file__).parent.joinpath(f"out/audio-{tag}.json").write_text(json.dumps({"tag": tag, "rows": rows}, ensure_ascii=False, indent=1))
