#!/usr/bin/python3.14 -I
"""Throwaway spike: OCR of the synthetic fixtures with LFM3-VL-2B (llama-mtmd-cli,
local paths only) against Tesseract. Writes out/ocr-<tag>.json.

usage: run_ocr.py VL_GGUF MMPROJ_GGUF TAG [THREADS]
"""
import json
import re
import subprocess
import sys
import time
import unicodedata
from pathlib import Path

HERE = Path(__file__).resolve().parent
HOME = Path.home() / ".local/share/kawaro-spike/lfm3"
CLI = next((HOME / "runtime/b11972").rglob("llama-mtmd-cli"))
model, mmproj, tag = sys.argv[1:4]
threads = sys.argv[4] if len(sys.argv) > 4 else "4"
PROMPT = "Transcribe all the text in this image exactly as written. Output only the text, nothing else."


def norm_raw(s):
    return re.sub(r"\s+", " ", unicodedata.normalize("NFC", s)).strip()


def norm_ar(s):
    s = unicodedata.normalize("NFKC", s)
    s = re.sub(r"[ؐ-ًؚ-ٰٟـ]", "", s)  # diacritics, tatweel
    s = re.sub(r"[آأإٱ]", "ا", s)  # alef forms
    return re.sub(r"\s+", " ", s).strip()


def lev(a, b):
    prev = list(range(len(b) + 1))
    for i, ca in enumerate(a, 1):
        cur = [i]
        for j, cb in enumerate(b, 1):
            cur.append(min(prev[j] + 1, cur[j - 1] + 1, prev[j - 1] + (ca != cb)))
        prev = cur
    return prev[-1]


def cer(hyp, ref, fn):
    h, r = fn(hyp), fn(ref)
    return round(lev(h, r) / max(len(r), 1), 4)


def timed(cmd):
    t = time.time()
    p = subprocess.run(["/usr/bin/time", "-v", *cmd], capture_output=True, text=True, timeout=900)
    wall = time.time() - t
    m = re.search(r"Maximum resident set size \(kbytes\): (\d+)", p.stderr)
    return p.stdout, p.stderr, round(wall, 2), round(int(m.group(1)) / 1024) if m else None


rows = []
import os
OCR_DIR = Path(os.environ["OCR_DIR"]) if os.environ.get("OCR_DIR") else HERE / "fixtures/ocr"
imgs = sorted([*OCR_DIR.rglob("*.png"), *OCR_DIR.rglob("*.jpg")])
for img in imgs:
    ref = img.with_suffix(".txt").read_text()
    lang = "eng" if img.name.startswith("en") else "ara"
    t_out, _, t_wall, t_rss = timed(["tesseract", str(img), "-", "-l", lang])
    v_out, v_err, v_wall, v_rss = timed([
        str(CLI), "-m", model, "--mmproj", mmproj, "--image", str(img), "-p", PROMPT,
        "--temp", "0", "-n", "2500", "-c", "8192", "-t", threads, "--no-warmup",
    ])
    norm = norm_ar if lang == "ara" else norm_raw
    row = {
        "file": img.name, "lang": lang,
        "tess_cer_raw": cer(t_out, ref, norm_raw), "tess_cer_norm": cer(t_out, ref, norm),
        "vl_cer_raw": cer(v_out, ref, norm_raw), "vl_cer_norm": cer(v_out, ref, norm),
        "tess_s": t_wall, "vl_s": v_wall, "tess_rss_mb": t_rss, "vl_rss_mb": v_rss,
        "vl_text_head": norm_raw(v_out)[:160],
    }
    rows.append(row)
    (HERE / "out" / f"ocr-{tag}-{img.stem}.vl.txt").write_text(v_out)
    print({k: row[k] for k in ("file", "tess_cer_norm", "vl_cer_norm", "vl_s", "vl_rss_mb")}, flush=True)


def mean(key, lang):
    xs = [r[key] for r in rows if r["lang"] == lang]
    return round(sum(xs) / len(xs), 4)


summary = {
    "tag": tag, "model": Path(model).name, "mmproj": Path(mmproj).name, "threads": threads,
    "synthetic_renders": not os.environ.get("OCR_DIR"),
    "mean_cer_norm": {l: {"tesseract": mean("tess_cer_norm", l), "vl": mean("vl_cer_norm", l)} for l in ("ara", "eng") if any(r["lang"] == l for r in rows)},
    "max_vl_s": max(r["vl_s"] for r in rows), "max_vl_rss_mb": max(r["vl_rss_mb"] or 0 for r in rows),
    "rows": rows,
}
(HERE / f"out/ocr-{tag}.json").write_text(json.dumps(summary, ensure_ascii=False, indent=1))
print({k: v for k, v in summary.items() if k != "rows"})
