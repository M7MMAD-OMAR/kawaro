#!/usr/bin/python3.14 -I
"""Per set mean WER/CER and time of two ASR result files. usage: compare_asr.py A B"""
import json, sys
from collections import defaultdict
a, b = sys.argv[1:3]
load = lambda t: {r["file"]: r for r in json.load(open(f"out/audio-{t}.json"))["rows"]}
A, B = load(a), load(b)
g = defaultdict(lambda: defaultdict(list))
for f in A:
    s = f.rsplit("-", 1)[0]
    for k, src in ((a, A), (b, B)):
        for m in ("wer", "cer", "wall_s"):
            g[s][f"{k}.{m}"].append(src[f][m])
    g[s]["n"].append(1)
for s, d in g.items():
    print(s, "n=%d" % len(d["n"]), {k: round(sum(v) / len(v), 3) for k, v in d.items() if k != "n"})
