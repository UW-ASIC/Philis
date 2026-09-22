#!/usr/bin/env python3
"""PEX cross-check: recompute the GROUND-capacitance term of every fixture
independently from its GDS (per-rect area·coeff + perimeter·fringe, exactly the
engine's bookkeeping — polygons are NOT unioned, matching the store), and hold
GPurify's reported total against it.

The total also carries lateral/inter-layer coupling this script does not
mirror, so the check is a band, not an equality:

    total < ground        → impossible: bookkeeping/unit bug somewhere
    total > 4 × ground    → coupling dwarfing plate cap on cell-scale layouts
                            is suspicious; investigate

    python3 benchmarks/xcheck_pex.py [fixture ...]
"""

import json
import re
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def read_rects(data: bytes):
    """(layer, datatype, x, y, w, h) bboxes of BOUNDARY elements, nm."""
    out = []
    layer = dtype = 0
    i = 0
    n = len(data)
    while i + 4 <= n:
        (ln, rec) = struct.unpack(">HH", data[i : i + 4])
        if ln < 4 or i + ln > n:
            break
        body = data[i + 4 : i + ln]
        if rec == 0x0D02:
            layer = struct.unpack(">H", body[:2])[0]
        elif rec in (0x0E02, 0x1602):
            dtype = struct.unpack(">H", body[:2])[0]
        elif rec == 0x1003:
            pts = struct.unpack(f">{len(body)//4}i", body)
            xs, ys = pts[0::2], pts[1::2]
            x0, x1, y0, y1 = min(xs), max(xs), min(ys), max(ys)
            if x1 > x0 and y1 > y0:
                out.append((layer, dtype, x0, y0, x1 - x0, y1 - y0))
        i += ln
    return out


def main() -> int:
    deck = json.loads((ROOT / "pdks" / "sky130.json").read_text())
    layers = deck["layers"]  # name -> [layer, datatype]
    pex = deck["pex"]
    by_ld = {}
    for name, (l, d) in layers.items():
        row = pex.get(name)
        if row:
            a = row.get("area_cap_af_um2") or 0.0
            f = row.get("fringe_cap_af_um") or 0.0
            if a or f:
                by_ld[(l, d)] = (name, a, f)

    fixtures = sys.argv[1:]
    dbg = ROOT / "target" / "bench_debug"
    if not fixtures:
        fixtures = sorted(d.name for d in dbg.iterdir() if (d / f"{d.name}.gds").exists())

    print(f"{'fixture':16} {'ground py fF':>12} {'gpurify fF':>10} {'ratio':>6}")
    bad = 0
    for f in fixtures:
        gds = dbg / f / f"{f}.gds"
        sig = dbg / f / "signoff.txt"
        if not gds.exists():
            continue
        total_af = 0.0
        for (l, d, _x, _y, w, h) in read_rects(gds.read_bytes()):
            hit = by_ld.get((l, d))
            if not hit:
                continue
            _, a, fr = hit
            um_w, um_h = w / 1000.0, h / 1000.0
            total_af += a * um_w * um_h + fr * 2 * (um_w + um_h)
        ground_ff = total_af / 1000.0
        m = re.search(r"C ([0-9.]+) fF", sig.read_text()) if sig.exists() else None
        if not m:
            print(f"{f:16} {ground_ff:12.2f} {'?':>10}")
            continue
        gp = float(m.group(1))
        ratio = gp / ground_ff if ground_ff > 0 else float("inf")
        mark = "  "
        if gp + 1e-9 < ground_ff or ratio > 4.0:
            mark = "->"
            bad += 1
        print(f"{mark}{f:14} {ground_ff:12.2f} {gp:10.2f} {ratio:6.2f}")
    print("PEXCHECK", "PASS" if bad == 0 else f"{bad} OUT OF BAND")
    return 0 if bad == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
