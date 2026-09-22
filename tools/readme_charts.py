#!/usr/bin/env python3
"""Render the README charts from real run artifacts.

Usage:  python3 tools/readme_charts.py <out_dir> [<out_dir> ...]

Each <out_dir> is a directory written by `philis run -o <dir>`; the charts are
read straight out of global_trace.csv, detailed_trace.csv, feedback.jsonl,
route_report.txt and signoff.json. Nothing here is hand-entered, so a chart can
never drift away from the run that produced it.

ponytail: hand-rolled SVG instead of matplotlib -- three fixed charts do not
justify a plotting dependency in a Rust repo. Swap it out if the chart count
grows past "a handful".
"""

import json
import os
import re
import sys

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets")

# Readable against both the light and the dark GitHub theme.
AXIS = "#888888"
GRID = "#88888833"
SERIES = ["#3572b0", "#d1663a", "#2e9e6b", "#9b59b6", "#c9a227", "#c0392b"]
OK, BAD = "#2e9e6b", "#c0392b"


# ---------------------------------------------------------------- reading ---

def read_trace(path):
    """(iters, costs) from an SA trace CSV."""
    with open(path) as f:
        rows = [line.split(",") for line in f.read().splitlines()[1:] if line]
    return [int(r[0]) for r in rows], [float(r[1]) for r in rows]


def read_feedback(path):
    with open(path) as f:
        return [json.loads(line) for line in f if line.strip()]


def read_report(path):
    """Scrape the numbers the reports print as prose."""
    text = open(path).read()
    out = {}
    m = re.search(r"wirelength (\d+) nm, (\d+) vias", text)
    if m:
        out["wl"], out["vias"] = int(m.group(1)), int(m.group(2))
    m = re.search(r"contracts: (\d+) total \| (\d+) satisfied, (\d+) violated", text)
    if m:
        out["contracts"] = tuple(int(g) for g in m.groups())
    out["hard"] = len(re.findall(r"HARD VIOLATIONS: (.+)", text))
    return out


def load(dirname):
    d = {"name": os.path.basename(dirname.rstrip("/")).removeprefix("out_")}
    d["global"] = read_trace(os.path.join(dirname, "global_trace.csv"))
    d["detailed"] = read_trace(os.path.join(dirname, "detailed_trace.csv"))
    d["feedback"] = read_feedback(os.path.join(dirname, "feedback.jsonl"))
    d["route"] = read_report(os.path.join(dirname, "route_report.txt"))
    d["place"] = read_report(os.path.join(dirname, "report.txt"))
    with open(os.path.join(dirname, "signoff.json")) as f:
        d["signoff"] = json.load(f)
    with open(os.path.join(dirname, "placement.txt")) as f:
        d["devices"] = len(f.read().splitlines()) - 2  # die line + header row
    return d


# ------------------------------------------------------------------- svg ----

class Svg:
    def __init__(self, w, h):
        self.w, self.h, self.parts = w, h, []

    def add(self, s):
        self.parts.append(s)

    def text(self, x, y, s, size=11, fill=AXIS, anchor="start", weight="normal"):
        s = s.replace("&", "&amp;").replace("<", "&lt;")
        self.add(f'<text x="{x:.1f}" y="{y:.1f}" font-family="DejaVu Sans,Helvetica,Arial,sans-serif" '
                 f'font-size="{size}" fill="{fill}" text-anchor="{anchor}" '
                 f'font-weight="{weight}">{s}</text>')

    def line(self, x1, y1, x2, y2, stroke=AXIS, width=1):
        self.add(f'<line x1="{x1:.1f}" y1="{y1:.1f}" x2="{x2:.1f}" y2="{y2:.1f}" '
                 f'stroke="{stroke}" stroke-width="{width}"/>')

    def path(self, pts, stroke, width=1.6):
        d = "M" + " L".join(f"{x:.1f},{y:.1f}" for x, y in pts)
        self.add(f'<path d="{d}" fill="none" stroke="{stroke}" stroke-width="{width}" '
                 f'stroke-linejoin="round"/>')

    def rect(self, x, y, w, h, fill, rx=0):
        self.add(f'<rect x="{x:.1f}" y="{y:.1f}" width="{w:.1f}" height="{h:.1f}" '
                 f'fill="{fill}" rx="{rx}"/>')

    def dot(self, x, y, r, fill, opacity=1.0):
        self.add(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{r}" fill="{fill}" opacity="{opacity}"/>')

    def save(self, name):
        path = os.path.normpath(os.path.join(OUT, name))
        with open(path, "w") as f:
            f.write(f'<svg xmlns="http://www.w3.org/2000/svg" width="{self.w}" '
                    f'height="{self.h}" viewBox="0 0 {self.w} {self.h}">\n'
                    + "\n".join(self.parts) + "\n</svg>\n")
        print(f"wrote {path}")


def axes(svg, x0, y0, x1, y1, xmax, yticks, ylabel, xlabel):
    """Frame with horizontal gridlines. yticks is [(value, label), ...] in [0,1]."""
    for v, lab in yticks:
        y = y1 - v * (y1 - y0)
        svg.line(x0, y, x1, y, GRID)
        svg.text(x0 - 6, y + 3.5, lab, 10, anchor="end")
    svg.line(x0, y0, x0, y1)
    svg.line(x0, y1, x1, y1)
    for frac in (0, 0.25, 0.5, 0.75, 1.0):
        x = x0 + frac * (x1 - x0)
        svg.text(x, y1 + 14, str(int(frac * xmax)), 10, anchor="middle")
    svg.text((x0 + x1) / 2, y1 + 30, xlabel, 10, anchor="middle")
    svg.text(x0, y0 - 10, ylabel, 10)


# ---------------------------------------------------------------- charts ----

def chart_convergence(runs):
    """Global placement cost against iteration, each curve scaled to its own start.

    Only the global phase is plotted. The detailed phase anneals a different
    objective and its trace column is not comparable to this one, so drawing
    both on one axis would suggest a descent that is not there.
    """
    svg = Svg(900, 300)
    svg.text(0, 14, "Global placement: cost vs. iteration, normalised to each circuit's start",
             12, AXIS, weight="bold")
    x0, y0, x1, y1 = 55, 45, 880, 235
    xmax = max(len(r["global"][0]) for r in runs) - 1
    # Do not clip at 100%: the cost overshoots its own starting value on every
    # circuit, and clipping would hide exactly that.
    top = max(max(r["global"][1]) / r["global"][1][0] for r in runs)
    axes(svg, x0, y0, x1, y1, xmax,
         [(v / top, f"{v * 100:.0f}%") for v in (0, 0.25, 0.5, 0.75, 1.0, 1.25)
          if v / top <= 1.0],
         "cost, % of initial", "iteration")
    for i, r in enumerate(runs):
        it, cost = r["global"]
        base = cost[0] * top
        pts = [(x0 + (t / xmax) * (x1 - x0), y1 - (c / base) * (y1 - y0))
               for t, c in zip(it, cost)]
        svg.path(pts, SERIES[i % len(SERIES)], 1.4)
    for i, r in enumerate(runs):
        it, cost = r["global"]
        x = x0 + (i % 4) * 210
        svg.rect(x, 273, 18, 3, SERIES[i % len(SERIES)], rx=1.5)
        svg.text(x + 24, 278, f"{r['name']}  ({cost[-1] / cost[0] * 100:.0f}%)", 10)
    svg.save("chart_convergence.svg")


def chart_feedback(runs):
    """Outer feedback loop: every candidate layout it tried, area against wirelength."""
    svg = Svg(900, 345)
    svg.text(0, 14, "Feedback loop: every candidate layout tried, die area vs. wirelength",
             12, AXIS, weight="bold")
    x0, y0, x1, y1 = 60, 42, 560, 265
    pts_all = [(f["wirelength_nm"], f["die_area_nm2"] / 1e6) for r in runs for f in r["feedback"]]
    wmax = max(p[0] for p in pts_all) * 1.05
    amax = max(p[1] for p in pts_all) * 1.05
    axes(svg, x0, y0, x1, y1, int(wmax / 1000),
         [(v, f"{v * amax:.0f}") for v in (0, 0.25, 0.5, 0.75, 1.0)],
         "die area (um^2)", "routed wirelength (um)")
    def is_clean(f):
        return f["drc_blocking"] == 0 and f["lvs_matched"] and f["hard_violations"] == 0

    for i, r in enumerate(runs):
        col = SERIES[i % len(SERIES)]
        for f in r["feedback"]:
            x = x0 + (f["wirelength_nm"] / wmax) * (x1 - x0)
            y = y1 - (f["die_area_nm2"] / 1e6 / amax) * (y1 - y0)
            svg.dot(x, y, 2.0, col, 0.75 if is_clean(f) else 0.25)

    # clean-rate bars on the right
    bx0, bx1 = 640, 880
    svg.text(bx0, y0 - 10, "candidates passing DRC + LVS + hard constraints", 10)
    for i, r in enumerate(runs):
        fb = r["feedback"]
        clean = sum(1 for f in fb if is_clean(f))
        y = y0 + 8 + i * 34
        svg.text(bx0, y - 3, r["name"], 10)
        svg.rect(bx0, y + 2, bx1 - bx0, 10, GRID, rx=2)
        svg.rect(bx0, y + 2, (bx1 - bx0) * clean / len(fb), 10, SERIES[i % len(SERIES)], rx=2)
        svg.text(bx1, y - 3, f"{clean}/{len(fb)}", 10, anchor="end")

    svg.text(0, 322, "Solid dots clear every check; faint dots were rejected. The loop ships "
                      "the best clean candidate, so the rejects never reach GDS.", 10)
    svg.save("chart_feedback.svg")


def chart_signoff(runs):
    """Per-circuit signoff scorecard plus contract satisfaction."""
    rows = []
    for r in runs:
        s, adv = r["signoff"], r["signoff"]["advanced"]
        pc, ps, pv = r["place"]["contracts"]
        rc, rs, rv = r["route"]["contracts"]
        rows.append({
            "name": r["name"],
            "devices": r["devices"],
            "checks": [
                ("DRC", len(s["drc_violations"]) == 0, str(len(s["drc_violations"]))),
                ("LVS", s["lvs"]["matched"], "MATCH" if s["lvs"]["matched"] else "MISMATCH"),
                ("ERC", s["erc_blocking"] == 0, str(s["erc_blocking"])),
                ("PEX", s["pex"]["complete"], "done" if s["pex"]["complete"] else "partial"),
                ("adv", all(v["status"] == "Clean" for v in adv.values()),
                 f"{sum(v['status'] == 'Clean' for v in adv.values())}/{len(adv)}"),
            ],
            "contracts": (ps + rs, pc + rc),
            "hard": r["place"]["hard"] + r["route"]["hard"],
        })

    svg = Svg(900, 60 + 30 * len(rows))
    svg.text(0, 14, "Signoff per circuit (blocking violations only; waived checks excluded)",
             12, AXIS, weight="bold")
    cols = [("circuit", 0), ("dev", 190), ("DRC", 240), ("LVS", 320), ("ERC", 410),
            ("PEX", 480), ("advanced", 550), ("constraints met", 660)]
    for label, x in cols:
        svg.text(x, 38, label, 10)
    svg.line(0, 44, 900, 44)

    for i, row in enumerate(rows):
        y = 64 + i * 30
        svg.text(0, y, row["name"], 11)
        svg.text(190, y, str(row["devices"]), 11)
        for (label, ok, detail), (_, x) in zip(row["checks"], cols[2:]):
            svg.dot(x + 5, y - 4, 4, OK if ok else BAD)
            svg.text(x + 14, y, detail, 10)
        met, tot = row["contracts"]
        frac = met / tot if tot else 1.0
        svg.rect(660, y - 9, 160, 9, GRID, rx=2)
        svg.rect(660, y - 9, 160 * frac, 9, OK if row["hard"] == 0 else BAD, rx=2)
        svg.text(830, y, f"{met}/{tot}", 10)
        svg.line(0, y + 10, 900, y + 10, GRID)
    svg.save("chart_signoff.svg")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    # A run that died before signoff has no scorecard to draw; say so rather
    # than quietly charting a subset.
    dirs, skipped = [], []
    for d in sys.argv[1:]:
        (dirs if os.path.exists(os.path.join(d, "signoff.json")) else skipped).append(d)
    for d in skipped:
        print(f"skipping {d}: no signoff.json (run did not finish)")
    runs = sorted((load(d) for d in dirs), key=lambda r: r["devices"])
    chart_convergence(runs)
    chart_feedback(runs)
    chart_signoff(runs)
