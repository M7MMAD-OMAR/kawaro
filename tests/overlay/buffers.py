#!/usr/bin/env python3
"""S-018 AC18: the buffers of a show are freed when the hive hides.

Reads a WAYLAND_DEBUG=1 trace of the daemon and the line ranges of each show
(the marks tests/overlay/kwin.sh takes), and prints, per show:

  SHOW <n> shm=<shm buffers made> px=<single pixel buffers made>
       pools=<wl_shm_pool made> layer_destroys=<zwlr_layer_surface_v1.destroy>
       at_hide=<buffers alive at the layer destroy>
       late_shm=<wl_shm buffers not freed at the hide> late_px=<the same for
       single pixel buffers> left_buffers=<count> left_pools=<wl_shm_pool a
       buffer was made from and never destroyed>

plus INFO lines that name every offending object with its trace line.

A buffer alive when the show's zwlr_layer_surface_v1 is destroyed (the hide)
must be destroyed by the client at the hide: its "-> wl_buffer#N.destroy()"
comes within HIDE_MS of the layer destroy and before any wl_buffer#N.release()
event the compositor sends for it. Destroying it only when the compositor
releases it is what the hive did before S-018f, and fails here. At the end of
the show no wl_buffer and no wl_shm_pool made in that show may still be alive.

Object ids are reused (the client frees and makes them again), and the trace
has several connections, so lifetimes are followed line by line, in order.

Usage: buffers.py TRACE START:END [START:END ...]   (1-based, END inclusive;
       a range covers lines START+1..END like kwin.sh's between()).
"""
import re
import sys

HIDE_MS = 250
REQ = r"^\[[0-9:.]+\]\s*-> "
ID = r"[#@](\d+)"
TS = re.compile(r"^\[(\d+):(\d+):(\d+)\.(\d+)\]")
POOL_NEW = re.compile(REQ + r"wl_shm" + ID + r"\.create_pool\(new id wl_shm_pool" + ID)
SHM_NEW = re.compile(REQ + r"wl_shm_pool" + ID + r"\.create_buffer\(new id wl_buffer" + ID)
PX_NEW = re.compile(REQ + r"wp_single_pixel_buffer_manager_v1" + ID + r"\.create_u32_rgba_buffer\(new id wl_buffer" + ID)
POOL_DEL = re.compile(REQ + r"wl_shm_pool" + ID + r"\.destroy\(")
BUF_DEL = re.compile(REQ + r"wl_buffer" + ID + r"\.destroy\(")
RELEASE = re.compile(r"^\[[0-9:.]+\] wl_buffer" + ID + r"\.release\(")
LAYER_NEW = re.compile(REQ + r"zwlr_layer_shell_v1" + ID + r"\.get_layer_surface\(new id zwlr_layer_surface_v1" + ID)
LAYER_DEL = re.compile(REQ + r"zwlr_layer_surface_v1" + ID + r"\.destroy\(")


def ms(line):
    m = TS.match(line.lstrip())
    if not m:
        return None
    h, mi, s, frac = m.groups()
    return ((int(h) * 60 + int(mi)) * 60 + int(s)) * 1000 + int(frac[:3].ljust(3, "0"))


def show(n, lines, start):
    alive = {}  # wl_buffer id -> (kind, trace line)
    pools = {}  # wl_shm_pool id -> trace line
    used = set()  # pools a wl_buffer was made from in this show
    made = {"shm": 0, "px": 0, "pools": 0}
    layers = set()
    layer_destroys = 0
    hide = None  # (trace line, ms, {id: (kind, line)})
    after = {}  # buffer id alive at hide -> first ("destroy"|"release", line, ms)
    info = []
    for k, line in enumerate(lines):
        ln = start + 1 + k
        m = POOL_NEW.match(line)
        if m:
            pools[m.group(2)] = ln
            made["pools"] += 1
            continue
        m = SHM_NEW.match(line) or PX_NEW.match(line)
        if m:
            kind = "shm" if "create_buffer" in line else "px"
            if kind == "shm":
                used.add(m.group(1))
            bid = m.group(2)
            if bid in alive:
                info.append(f"show {n}: wl_buffer#{bid} made again at line {ln} while alive since line {alive[bid][1]} (another connection?)")
            alive[bid] = (kind, ln)
            made[kind] += 1
            continue
        m = POOL_DEL.match(line)
        if m:
            pools.pop(m.group(1), None)
            used.discard(m.group(1))
            continue
        m = BUF_DEL.match(line)
        if m:
            bid = m.group(1)
            if hide and bid in hide[2] and bid not in after:
                after[bid] = ("destroy", ln, ms(line))
            alive.pop(bid, None)
            continue
        m = RELEASE.match(line)
        if m:
            bid = m.group(1)
            if hide and bid in hide[2] and bid not in after:
                after[bid] = ("release", ln, ms(line))
            continue
        m = LAYER_NEW.match(line)
        if m:
            layers.add(m.group(2))
            continue
        m = LAYER_DEL.match(line)
        if m and m.group(1) in layers:
            layer_destroys += 1
            if hide is None:
                hide = (ln, ms(line), dict(alive))
    late = []
    late_kind = {"shm": 0, "px": 0}
    if hide:
        hln, hms, at = hide
        for bid, (kind, mln) in sorted(at.items(), key=lambda x: x[1][1]):
            first = after.get(bid)
            n_late = len(late)
            if first is None:
                late.append(f"wl_buffer#{bid} ({kind}, made line {mln}): never destroyed after the hide at line {hln}")
            elif first[0] == "release":
                late.append(f"wl_buffer#{bid} ({kind}, made line {mln}): released by the compositor at line {first[1]} before the client destroyed it (hide at line {hln})")
            elif hms is not None and first[2] is not None and first[2] - hms > HIDE_MS:
                late.append(f"wl_buffer#{bid} ({kind}, made line {mln}): destroyed at line {first[1]}, {first[2] - hms} ms after the hide at line {hln}")
            if len(late) > n_late:
                late_kind[kind] += 1
        kinds = ", ".join(f"#{b} {k}" for b, (k, _) in sorted(at.items(), key=lambda x: x[1][1])) or "none"
        info.append(f"show {n}: hide (zwlr_layer_surface_v1.destroy) at line {hln}, buffers alive then: {kinds}")
    else:
        info.append(f"show {n}: no zwlr_layer_surface_v1.destroy of this show's layer surface")
    for x in late:
        info.append(f"show {n}: not freed at the hide: {x}")
    for bid, (kind, mln) in alive.items():
        info.append(f"show {n}: wl_buffer#{bid} ({kind}, made line {mln}) still alive at the end of the show")
    left_pools = {p: ln for p, ln in pools.items() if p in used}
    for pid, pln in left_pools.items():
        info.append(f"show {n}: wl_shm_pool#{pid} (made line {pln}) still alive at the end of the show")
    for x in info:
        print("INFO AC18 " + x)
    print(
        f"SHOW {n} shm={made['shm']} px={made['px']} pools={made['pools']} "
        f"layer_destroys={layer_destroys} at_hide={len(hide[2]) if hide else 0} "
        f"late_shm={late_kind['shm']} late_px={late_kind['px']} "
        f"left_buffers={len(alive)} left_pools={len(left_pools)}"
    )


def main():
    with open(sys.argv[1], encoding="utf-8", errors="replace") as f:
        trace = f.read().split("\n")
    for n, rng in enumerate(sys.argv[2:], 1):
        a, b = (int(x) for x in rng.split(":"))
        show(n, trace[a:b], a)


if __name__ == "__main__":
    main()
