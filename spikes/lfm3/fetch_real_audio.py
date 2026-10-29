#!/usr/bin/python3.14 -I
"""Throwaway spike: pull a few real speech clips from open Hugging Face datasets through the
datasets-server rows API. Audio stays outside the repository; only manifest.json is kept.

Sources (see manifest for the licence each carries):
  fleurs_ar : google/fleurs ar_eg test, CC-BY-4.0, read speech by Egyptian speakers
  syrian    : Overtech/syrian-arabic-speech-dataset, licence "other", podcast speech
  librispeech_en : hf-internal-testing/librispeech_asr_dummy, LibriSpeech read English, CC-BY-4.0 (FLEURS English was refused by the API)
"""
import json, sys, urllib.request, urllib.parse
from pathlib import Path

DEST = Path.home() / ".local/share/kawaro-spike/lfm3/audio-real"
HERE = Path(__file__).resolve().parent
SETS = [("fleurs_ar", "google/fleurs", "ar_eg", "test", 12, "transcription", "CC-BY-4.0"),
        ("syrian", "Overtech/syrian-arabic-speech-dataset", "default", "train", 12, "transcript_text", "other (see dataset LICENSE)"),
        ("librispeech_en", "hf-internal-testing/librispeech_asr_dummy", "clean", "validation", 8, "text", "CC-BY-4.0 (LibriSpeech)")]
manifest = []
for name, ds, cfg, split, n, tcol, lic in SETS:
    out = DEST / name
    out.mkdir(parents=True, exist_ok=True)
    q = urllib.parse.urlencode({"dataset": ds, "config": cfg, "split": split, "offset": 0, "length": n})
    try:
        d = json.load(urllib.request.urlopen(f"https://datasets-server.huggingface.co/rows?{q}", timeout=90))
    except Exception as e:
        print(name, "failed:", e); continue
    if "rows" not in d:
        print(name, "refused:", str(d)[:120]); continue
    for i, r in enumerate(d["rows"]):
        row = r["row"]
        dur = row.get("duration") or row.get("num_samples", 0) / 16000
        if dur > 25:
            continue
        f = out / f"{name}-{i:02d}.wav"
        if not f.exists():
            f.write_bytes(urllib.request.urlopen(row["audio"][0]["src"], timeout=90).read())
        f.with_suffix(".txt").write_text(row[tcol])
        manifest.append({"set": name, "file": f.name, "dataset": ds, "config": cfg, "split": split, "row": r["row_idx"],
                         "seconds": round(dur, 1), "licence": lic})
    print(name, "ok", sum(m["set"] == name for m in manifest))
(HERE / "fixtures/cases/audio-real-manifest.json").write_text(json.dumps(manifest, indent=1))
