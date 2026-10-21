"""S-019 AC3 and AC4 oracle for the shell tests (S-019c, S-019d).

Usage: anchor.py TOKENS W H X Y N [R1]
  TOKENS  design/tokens.json
  W H     output size in logical pixels
  X Y     anchor (pointer position) on that output
  N       cells the hive shows (18 at most)
  R1      ring-1 cells `assign` gives (default min(N, 6)); the counts below
          do not depend on it when it is at least the accepted ring-1 slots

Prints one line per min_rings value (1 and 2):
  MIN_RINGS CX CY COUNT BX BY BW BH
CX CY is the clamped anchor (the centre cell's centre on the output), COUNT
the cells the priority rules fill (AC4), and BX BY BW BH the hive
subsurface: the bounding box of the centre and every accepted slot in rings 1
to max(min_rings, rings used), each hexagon plus PAD (AC3), placed so the
centre is at CX CY (rounded as `plan_at` rounds: x = round(CX - origin)).
BW BH are computed in f64 and may differ by 1 px from the f32 ceiling of
`plan_at` (393 x 362 in the middle of the screen), so callers allow 1 px.
Rules from the acceptance criteria only, with the tokens: radius
hex.size.md / sqrt(3), spacing radius + hex.gap / sqrt(3), PAD space.1,
pointy-top axial offsets x = s * sqrt(3) * (q + r / 2), y = s * 1.5 * r.
"""

import json
import math
import sys

EPS = 0.01
MAX_RINGS = 4


def ring(k):
    # Axial ring k (any order: only counts and the box are printed).
    out = []
    q, r = -k, k
    for dq, dr in ((1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)):
        for _ in range(k):
            out.append((q, r))
            q, r = q + dq, r + dr
    return out


def main():
    tokens, w, h, x, y, n = sys.argv[1:7]
    t = json.load(open(tokens))
    w, h, x, y, n = float(w), float(h), float(x), float(y), int(n)
    r1 = int(sys.argv[7]) if len(sys.argv) > 7 else min(n, 6)
    size = t["hex"]["size"]["md"]
    radius = size / math.sqrt(3)
    s = radius + t["hex"]["gap"] / math.sqrt(3)
    pad = t["space"]["1"]
    mx, my = size / 2 + pad, radius + pad
    cx = min(max(x, mx), w - mx)
    cy = min(max(y, my), h - my)

    def off(a):
        q, r = a
        return s * math.sqrt(3) * (q + r / 2), s * 1.5 * r

    def fits(a):
        ox, oy = off(a)
        px, py = cx + ox, cy + oy
        return mx - EPS <= px <= w - mx + EPS and my - EPS <= py <= h - my + EPS

    acc = [[a for a in ring(k) if fits(a)] for k in range(1, MAX_RINGS + 1)]
    n1 = min(r1, len(acc[0]))
    rest = n - n1
    outer = [k for k in range(2, MAX_RINGS + 1) for _ in acc[k - 1]]
    n_out = min(rest, len(outer))
    used = outer[n_out - 1] if n_out > 0 else 1
    for min_rings in (1, 2):
        rings = max(min_rings, used)
        xs, ys = [0.0], [0.0]
        for k in range(rings):
            for a in acc[k]:
                ox, oy = off(a)
                xs.append(ox)
                ys.append(oy)
        bw = max(xs) - min(xs) + 2 * mx
        bh = max(ys) - min(ys) + 2 * my
        ox, oy = mx - min(xs), my - min(ys)
        bx, by = round(cx - ox), round(cy - oy)
        print(min_rings, round(cx, 2), round(cy, 2), n1 + n_out, bx, by, math.ceil(bw), math.ceil(bh))


main()
