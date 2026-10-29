#!/usr/bin/python3.14 -I
"""Tesseract preprocessing survey on the real Arabic pages. usage: run_tess.py"""
import json, os, re, subprocess, tempfile, time, unicodedata
from collections import defaultdict
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = Path.home() / ".local/share/kawaro-spike/lfm3/ocr-real"


def norm_ar(s):
    s = unicodedata.normalize("NFKC", s)
    s = re.sub(r"[ؐ-ًؚ-ٰٟـ]", "", s)
    s = re.sub(r"[آأإٱ]", "ا", s)
    return re.sub(r"\s+", " ", s).strip()


def lev(a, b):
    prev = list(range(len(b) + 1))
    for i, ca in enumerate(a, 1):
        cur = [i]
        for j, cb in enumerate(b, 1):
            cur.append(min(prev[j] + 1, cur[j - 1] + 1, prev[j - 1] + (ca != cb)))
        prev = cur
    return prev[-1]


VARIANTS = {
    "baseline": ([], []),
    "psm6": ([], ["--psm", "6"]),
    "gray_norm": (["-colorspace", "Gray", "-normalize"], []),
    "deskew_bin": (["-colorspace", "Gray", "-deskew", "40%", "-normalize", "-threshold", "55%"], []),
    "up2_deskew": (["-colorspace", "Gray", "-resize", "200%", "-deskew", "40%", "-normalize", "-sharpen", "0x1"], []),
    "up2_deskew_bin": (["-colorspace", "Gray", "-resize", "200%", "-deskew", "40%", "-normalize", "-threshold", "55%"], []),
    "clahe_up2": (["-colorspace", "Gray", "-resize", "200%", "-clahe", "25x25%+128+3", "-deskew", "40%"], []),
}
res = defaultdict(lambda: defaultdict(list))
secs = defaultdict(list)
for img in sorted([*ROOT.rglob("*.jpg")]):
    ref = norm_ar(img.with_suffix(".txt").read_text())
    s = img.name.rsplit("-", 1)[0]
    for name, (pre, extra) in VARIANTS.items():
        src = str(img)
        t = time.time()
        with tempfile.TemporaryDirectory() as d:
            if pre:
                src = f"{d}/p.png"
                subprocess.run(["convert", str(img), *pre, src], check=True, capture_output=True)
            out = subprocess.run(["tesseract", src, "-", "-l", "ara", *extra], capture_output=True, text=True).stdout
        secs[name].append(time.time() - t)
        res[name][s].append(lev(norm_ar(out), ref) / max(len(ref), 1))
    print(img.name, flush=True)
table = {n: {s: round(sum(v) / len(v), 3) for s, v in d.items()} | {"mean_s": round(sum(secs[n]) / len(secs[n]), 2)}
         for n, d in res.items()}
(HERE / "out/tesseract-preproc.json").write_text(json.dumps(table, indent=1))
for n, d in table.items():
    print(f"{n:16}", d)
