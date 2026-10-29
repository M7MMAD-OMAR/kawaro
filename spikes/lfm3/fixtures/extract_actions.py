#!/usr/bin/env python3
"""Extract a compact action catalog from actions/*.toml into actions.json.

Usage (from anywhere):
    python3 -I spikes/lfm3/fixtures/extract_actions.py

Stdlib only. Param types come from the TOML default value. Param ranges are
not in the TOML; the engines enforce them, so they are copied here from the
source of truth:
  image  quality 1..100            crates/engines/src/image.rs (quality)
  image  scale 1..1000 (percent)   crates/engines/src/image.rs (parse_size)
  image  max_side >= 1 (pixels)    crates/engines/src/image.rs (parse_size)
  image  degrees 90|180|270        docs/architecture.md (image rotate)
  image  axis horizontal|vertical  docs/architecture.md (image flip)
  image  page "fit"                crates/engines/src/image/pdf.rs
  pdf    dpi 36..600               crates/engines/src/pdf.rs (DPI_RANGE)
  pdf    degrees 90|180|270        crates/engines/src/pdf.rs (degrees_param)
  pdf    preset screen|ebook|printer  crates/engines/src/pdf.rs
  ffmpeg crf 0..63, fps 1..60, width 16..4096, max_height 16..8640
                                   crates/engines/src/ffmpeg.rs (int_param, float_param)
"""
import json
import pathlib
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[3]
OUT = pathlib.Path(__file__).resolve().parent / "actions.json"

# (group, param) -> range spec. Engines "pdf" and "ghostscript" share the pdf group.
RANGES = {
    ("image", "quality"): {"min": 1, "max": 100},
    ("image", "scale"): {"min": 1, "max": 1000},
    ("image", "max_side"): {"min": 1},
    ("image", "degrees"): {"enum": [90, 180, 270]},
    ("image", "axis"): {"enum": ["horizontal", "vertical"]},
    ("image", "page"): {"enum": ["fit"]},
    ("pdf", "dpi"): {"min": 36, "max": 600},
    ("pdf", "degrees"): {"enum": [90, 180, 270]},
    ("pdf", "preset"): {"enum": ["screen", "ebook", "printer"]},
    ("ffmpeg", "crf"): {"min": 0, "max": 63},
    ("ffmpeg", "fps"): {"min": 1, "max": 60},
    ("ffmpeg", "width"): {"min": 16, "max": 4096},
    ("ffmpeg", "max_height"): {"min": 16, "max": 8640},
}
GROUP = {"image": "image", "pdf": "pdf", "ghostscript": "pdf", "ffmpeg": "ffmpeg"}


def ptype(v):
    if isinstance(v, bool):
        return "bool"
    if isinstance(v, int):
        return "int"
    if isinstance(v, float):
        return "number"
    return "string"


def main():
    entries = []
    for path in sorted((ROOT / "actions").glob("*.toml")):
        data = tomllib.loads(path.read_text(encoding="utf-8"))
        for a in data.get("action", []):
            group = GROUP.get(a["engine"], a["engine"])
            params = {}
            for k, v in a.get("params", {}).items():
                rng = RANGES.get((group, k))
                if rng is None:
                    raise SystemExit(f"no range known for {a['id']}.{k}")
                params[k] = {"type": ptype(v), "default": v, **rng}
            e = {
                "id": a["id"],
                "accepts": a["accepts"],
                "label": a["label"],
                "output": a["output"],
            }
            if a.get("min_files", 1) > 1:
                e["min_files"] = a["min_files"]
            if a.get("max_files", 0) > 0:
                e["max_files"] = a["max_files"]
            if a.get("skip_ext"):
                e["skip_ext"] = a["skip_ext"]
            if params:
                e["params"] = params
            entries.append(e)
    OUT.write_text(
        json.dumps(entries, ensure_ascii=False, indent=1) + "\n", encoding="utf-8"
    )
    print(f"wrote {len(entries)} actions to {OUT}")


if __name__ == "__main__":
    main()
