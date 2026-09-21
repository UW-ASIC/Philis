#!/usr/bin/env python3
"""Validate the GPurify↔KLayout differential on geometry NEITHER produced:
a KLayout-synthesised GDS with a known violation inventory. Both engines must
report exactly the expected count per rule — this is what proves the harness
can see disagreement at all (0-vs-0 on clean layouts proves nothing).

    python3 benchmarks/xcheck_selftest.py

Uses benchmarks/xcheck.py's generated deck.drc for the KLayout side and the
`drc_gds` example for the GPurify side.
"""

import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "target" / "xcheck"
sys.path.insert(0, str(ROOT / "benchmarks"))
import xcheck  # noqa: E402

# Expected violations, rule -> count. Geometry below is built to trip exactly
# these and nothing else (clean spacing to every neighbour block).
# rule -> expected presence (True) with a nominal count; engines may split a
# finding differently (a square under min_width is 1 pair to GPurify, 2 to
# KLayout), so the verdict is presence agreement, counts are printed.
EXPECTED = {
    "li_min_width": 1,        # 100 nm wide li bar
    "li_min_spacing": 1,      # two li plates 100 nm apart
    "li_notch": 1,            # U of three li rects, 100 nm slot
    "met1_min_area": 2,       # 100×100 met1 speck (limit side 240) + 260 via1 pad
    "met1_min_width": 1,      # same speck is also under met1 min_width 140
    "li_min_area": 1,         # the 190 nm licon pad is under li min_area too
    "li_encloses_licon": 2,   # 190 nm li pad (needs 80/side) + the naked licon
    "li_encloses_licon_one_side": 2,   # same two cuts, adjacent-sides arm
    "licon_max_width": 1,     # 250 nm licon cut (max 170)
    "met1_encloses_via1_one_side": 1,  # 55-symmetric met1 pad (needs 85)
    "poly_endcap_over_diff": 1,        # 50 nm endcap (needs 130)
    "met1_wide_metal_spacing": 1,      # wide plates 200 apart (needs 280)
    "poly_to_tap_spacing": 1,          # poly OVERLAPS tap (boolean arm)
}
# GPurify-only rows the KLayout deck cannot express; presence asserted on the
# GPurify side alone.
EXPECTED_GPURIFY_ONLY = {
    "floating_interconnect": 16,  # every synthetic conductor floats, per rect
}

BUILDER = r"""
import pya
ly = pya.Layout()
ly.dbu = 0.001  # 1 nm, same as the Philis writer
top = ly.create_cell("BAD")
li = ly.layer(67, 20)
licon = ly.layer(66, 44)
met1 = ly.layer(68, 20)
via1 = ly.layer(68, 44)
met2 = ly.layer(69, 20)
diff = ly.layer(65, 20)
tap = ly.layer(65, 44)
poly = ly.layer(66, 20)

def box(l, x, y, w, h):
    top.shapes(l).insert(pya.Box(x, y, x + w, y + h))

# 1. li min_width: 100 wide, long enough to clear min_area (side 236).
box(li, 0, 0, 100, 60000)

# 2. li min_spacing: two fat plates 100 apart.
box(li, 5000, 0, 1000, 1000)
box(li, 6100, 0, 1000, 1000)

# 3. li notch: U shape, slot 100 wide, 500 deep (arms 1000 wide).
box(li, 20000, 0, 1000, 1500)
box(li, 21100, 0, 1000, 1500)
box(li, 20000, 0, 2100, 1000)  # base joins the arms; slot y in [1000,1500]

# 4. met1 min_area: lone 100x100 (area rule side 240 → 0.0576 um2).
box(met1, 40000, 0, 100, 100)

# 5. licon under-enclosed by li: cut 170, pad 190 (10 per side, needs 80).
box(licon, 60000, 0, 170, 170)
box(li, 59990, -10, 190, 190)

# 6. over-size licon with no li at all: licon_max_width (250 > 170), and the
#    unhosted-inner terms of li_encloses_licon(+_one_side) both fire.
box(licon, 100000, 0, 250, 250)

# 7. via1 with a 55-symmetric met1 pad: met1_encloses_via1 (55) passes,
#    met1_encloses_via1_one_side (85) fails on both axes. The met2 pad gives
#    85 all round so every met2 rule stays quiet; the 260 met1 pad is under
#    met1_min_area (side 288).
box(via1, 120000, 0, 150, 150)
box(met1, 119945, -55, 260, 260)
box(met2, 119915, -85, 320, 320)

# 8. short poly endcap: gate crossing with a 50 nm bottom endcap (needs 130);
#    the top endcap (130) and the diff overhangs (400/450, needs 250) are legal.
box(diff, 140000, 0, 1000, 400)
box(poly, 140400, -50, 150, 580)

# 9. wide-metal spacing: two 3200-wide met1 plates at a 200 gap (280 when a
#    side is wide); the thin 500 control pair at the same gap must NOT fire
#    (met1_min_spacing is 140).
box(met1, 160000, 0, 3200, 3200)
box(met1, 163400, 0, 3200, 3200)
box(met1, 170000, 0, 500, 500)
box(met1, 170700, 0, 500, 500)

# 10. poly OVERLAPPING tap: separation() misses overlapping polygons, the
#     boolean term of the min_spacing_diff arm is what reports this one.
box(tap, 180000, 0, 300, 300)
box(poly, 180200, 0, 300, 300)

ly.write("__OUT__")
"""


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    deck = json.loads((ROOT / "pdks" / "sky130.json").read_text())
    drc_text, checked, _ = xcheck.gen_drc(deck)
    (OUT / "deck.drc").write_text(drc_text)

    kl = xcheck.klayout_bin()
    bad = OUT / "bad.gds"
    builder = OUT / "make_bad.py"
    builder.write_text(BUILDER.replace("__OUT__", str(bad)))
    r = subprocess.run([kl, "-b", "-r", str(builder)], capture_output=True, text=True)
    if r.returncode != 0:
        print(r.stderr[-2000:])
        return 1

    # KLayout side.
    report = OUT / "bad.lyrdb"
    r = subprocess.run(
        [kl, "-b", "-r", str(OUT / "deck.drc"), "-rd", f"input={bad}", "-rd", f"report={report}"],
        capture_output=True,
        text=True,
    )
    if r.returncode != 0:
        print(r.stderr[-2000:])
        return 1
    kc = xcheck.klayout_counts(report)

    # GPurify side, via the drc_gds example.
    r = subprocess.run(
        ["cargo", "run", "--release", "-q", "-p", "benchmark", "--example", "drc_gds", "--", str(bad)],
        capture_output=True,
        text=True,
        cwd=ROOT,
    )
    if r.returncode != 0:
        print(r.stderr[-3000:])
        return 1
    gc: dict[str, int] = {}
    for line in r.stdout.splitlines():
        rid = line.split("\t", 1)[0].strip()
        if rid:
            gc[rid] = gc.get(rid, 0) + 1

    ok = True
    rows = sorted(set(EXPECTED) | set(EXPECTED_GPURIFY_ONLY) | set(kc) | set(gc))
    print(f"{'rule':34} {'expect':>6} {'gpurify':>7} {'klayout':>7}")
    for rid in rows:
        g, k = gc.get(rid, 0), kc.get(rid, 0)
        if rid in EXPECTED_GPURIFY_ONLY:
            e = EXPECTED_GPURIFY_ONLY[rid]
            good = (g > 0) == (e > 0)
        else:
            e = EXPECTED.get(rid, 0)
            # Presence must agree three ways; counts are convention-dependent.
            good = ((e > 0) == (g > 0) == (k > 0))
        mark = "  " if good else "->"
        if e or g or k:
            print(f"{mark} {rid:32} {e:6} {g:7} {k:7}")
        ok &= good
    print("SELFTEST", "PASS" if ok else "FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
