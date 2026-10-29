# EasyOCR (Arabic + English) on the same real pages. Run with the venv-ocr python.
import json, os, re, sys, time, unicodedata
from pathlib import Path
import easyocr

HERE = Path(__file__).resolve().parent
root = Path(os.environ["OCR_DIR"]); tag = sys.argv[1]


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


import torch
torch.set_num_threads(4)
reader = easyocr.Reader(["ar", "en"], gpu=False, verbose=False)
rows = []
for img in sorted([*root.rglob("*.jpg"), *root.rglob("*.png")]):
    ref = img.with_suffix(".txt").read_text()
    t = time.time()
    res = reader.readtext(str(img), detail=1, paragraph=False)
    wall = time.time() - t
    # reading order for RTL: top to bottom by line, right to left inside a line
    items = sorted(res, key=lambda r: (round(sum(p[1] for p in r[0]) / 4 / 25), -sum(p[0] for p in r[0]) / 4))
    hyp = " ".join(r[1] for r in items)
    row = {"file": img.name, "easy_cer_norm": round(lev(norm_ar(hyp), norm_ar(ref)) / max(len(norm_ar(ref)), 1), 4), "easy_s": round(wall, 2)}
    rows.append(row); print(row, flush=True)
(HERE / f"out/ocr-{tag}.json").write_text(json.dumps({"tag": tag, "rows": rows}, indent=1))
