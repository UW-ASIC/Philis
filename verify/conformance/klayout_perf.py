#!/usr/bin/env python3
"""KLayout side of the combs benchmark — same geometry as `perf.rs combs:N`.

Builds the two interlocked single-polygon combs (fingers 200nm thick, all gaps
150nm, min_spacing rule 100nm => clean) and times Region.space(100).

Usage: klayout -b -r klayout_perf.py [-rd sizes=500,1000,2000]
"""
import time

import pya


def comb_points(n, right):
    """Vertices for one comb; mirrors perf.rs combs()."""
    w, fx_a, fx_b = 10_000, 9_600, 400
    if not right:  # comb A: spine left, fingers rightward
        pts = [(0, 0), (fx_a, 0)]
        spine_x, fx, y0 = 200, fx_a, 0
        end_x = 0
    else:  # comb B: spine right, fingers leftward, offset +350
        pts = [(w, 350), (fx_b, 350)]
        spine_x, fx, y0 = w - 200, fx_b, 350
        end_x = w
    for i in range(n):
        yt = i * 700 + 200 + y0
        yb_next = (i + 1) * 700 + y0
        pts.append((fx, yt))
        if i < n - 1:
            pts += [(spine_x, yt), (spine_x, yb_next), (fx, yb_next)]
        else:
            pts.append((end_x, yt))
    return pts


def run(n):
    region = pya.Region()
    for right in (False, True):
        region.insert(pya.Polygon([pya.Point(x, y) for x, y in comb_points(n, right)]))
    region.merged_semantics = True
    t0 = time.perf_counter()
    viol = region.space_check(100)
    ms = (time.perf_counter() - t0) * 1000.0
    print(f"combs:{n}  klayout: {viol.count()} violations in {ms:.0f} ms")


def run_bars(n):
    """Mirrors perf.rs bars(): n full-width bars, gaps cycling 85..115nm."""
    region = pya.Region()
    y = 0
    for i in range(n):
        region.insert(pya.Box(0, y, 20_000, y + 200))
        y += 200 + 85 + 5 * (i % 7)
    region.merged_semantics = True
    t0 = time.perf_counter()
    viol = region.space_check(100)
    ms = (time.perf_counter() - t0) * 1000.0
    print(f"bars:{n}  klayout: {viol.count()} violations in {ms:.0f} ms")


# under `klayout -b -r`, -rd sizes=... lands in the global namespace
_sizes = globals().get("sizes", "500,1000,2000")
for _n in str(_sizes).split(","):
    _n = _n.strip()
    if _n.startswith("bars"):
        run_bars(int(_n.split(":")[1]))
    elif _n.startswith("combs"):
        run(int(_n.split(":")[1]))
    else:
        run(int(_n))
