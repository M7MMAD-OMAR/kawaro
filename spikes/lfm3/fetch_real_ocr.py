#!/usr/bin/python3.14 -I
"""Throwaway spike: real Arabic page images from open datasets via the datasets-server rows API.
Images stay outside the repository; only fixtures/cases/ocr-real-manifest.json is kept.
  arocrbench : ahmedheakl/arocrbench_arabicocr (licence not stated on the card; local evaluation only)
  books      : freococo/ocr_arabic_books (CC-BY-NC-ND-4.0; local evaluation only, never redistributed)
  synth_scan : loay/arabic-ocr-synthetic-scans-faker-300k (CC-BY-4.0; synthetic scans with noise levels)
"""
import json, urllib.parse, urllib.request
from pathlib import Path

DEST = Path.home() / ".local/share/kawaro-spike/lfm3/ocr-real"
HERE = Path(__file__).resolve().parent
SETS = [("arocrbench", "ahmedheakl/arocrbench_arabicocr", 12, lambda r: r["text"]),
        ("books", "freococo/ocr_arabic_books", 10, lambda r: r["body"]),
        ("synth_scan", "loay/arabic-ocr-synthetic-scans-faker-300k", 10, lambda r: r["text"])]
manifest = []
for name, ds, n, tf in SETS:
    out = DEST / name; out.mkdir(parents=True, exist_ok=True)
    q = urllib.parse.urlencode({"dataset": ds, "config": "default", "split": "train", "offset": 0, "length": n})
    d = json.load(urllib.request.urlopen(f"https://datasets-server.huggingface.co/rows?{q}", timeout=90))
    for i, r in enumerate(d.get("rows", [])):
        row = r["row"]
        f = out / f"{name}-{i:02d}.jpg"
        if not f.exists():
            f.write_bytes(urllib.request.urlopen(row["image"]["src"], timeout=90).read())
        f.with_suffix(".txt").write_text(tf(row))
        manifest.append({"set": name, "file": f.name, "dataset": ds, "row": r["row_idx"]})
    print(name, sum(m["set"] == name for m in manifest))
(HERE / "fixtures/cases/ocr-real-manifest.json").write_text(json.dumps(manifest, indent=1))
