#!/usr/bin/env python3
"""Progress snapshot: bench log -> README sample layouts + speed history graph.

    python3 benchmarks/progress.py <bench.log> <assets_dir> <sha> [label]
    python3 benchmarks/progress.py --gate <bench.log>   (publish gate)

Parses `bench local` rows ("[Suite/deck] name  N ms ... proposals P"),
appends ns per iteration (wall ns / epochs; one epoch = place, route, sign off) and wall ms per circuit
to docs/progress/speed.csv, copies the sample layouts to docs/progress/layouts/,
redraws docs/progress/speed.svg and rewrites README.md between its markers.
No third-party packages: the chart is hand-written SVG.
"""
import csv, html, os, re, shutil, sys, time

SAMPLES = ["ota", "dac4", "bgr_core", "rc_filter", "pair", "tq_chain"]
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PROG = os.path.join(ROOT, "docs", "progress")
ROW = re.compile(r"\]\s+(\S+)\s+(\d+) ms .*?\| DRC (\d+) \| LVS (\S+(?: \S+)?) \| ERC (\d+).*?\| best \d+/(\d+)")


def parse(log):
    rows = {}
    for line in open(log, errors="replace"):
        m = ROW.search(line)
        if m:
            name, ms, drc, lvs, erc, ep = m.groups()
            rows[name] = dict(ms=int(ms), drc=int(drc), lvs=lvs.split(" |")[0], erc=int(erc), epochs=int(ep))
    return rows


def gate(log):
    """Exit 1 unless every circuit ran and signed off DRC 0, ERC 0, LVS MATCH."""
    rows = parse(log)
    ran = re.search(r"(\d+)/(\d+) circuits placed\+routed", open(log, errors="replace").read())
    bad = [f"{c}: DRC {r['drc']}, ERC {r['erc']}, LVS {r['lvs']}" for c, r in sorted(rows.items())
           if r["drc"] or r["erc"] or r["lvs"] != "MATCH"]
    if not ran or ran.group(1) != ran.group(2) or len(rows) != int(ran.group(2)):
        bad.append(f"not every circuit signed off: {ran.group(0) if ran else 'no summary line'}, {len(rows)} rows")
    print("\n".join(bad) or f"{len(rows)} circuits: DRC 0, ERC 0, LVS MATCH")
    sys.exit(1 if bad else 0)


def chart(hist, path):
    # hist: list of (label, {circuit: ms_per_iter})
    circuits = sorted({c for _, r in hist for c in r})
    W, H, L, B, T, R = 760, 360, 70, 50, 30, 150
    vals = [v for _, r in hist for v in r.values()] or [1]
    vmax = max(vals) * 1.1
    n = max(len(hist) - 1, 1)
    x = lambda i: L + (W - L - R) * i / n
    y = lambda v: T + (H - T - B) * (1 - v / vmax)
    pal = ["#2a6fdb", "#e0542b", "#2f9e44", "#9c36b5", "#f59f00", "#0b7285", "#c2255c", "#5c940d", "#495057", "#1864ab", "#d9480f", "#087f5b"]
    out = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" font-family="sans-serif" font-size="11">',
           f'<rect width="{W}" height="{H}" fill="white"/>',
           f'<text x="{L}" y="18" font-size="13" font-weight="bold">Speed: ms per iteration = one place/route/signoff epoch (lower is better)</text>']
    for k in range(5):
        v = vmax * k / 4
        out.append(f'<line x1="{L}" x2="{W - R}" y1="{y(v):.1f}" y2="{y(v):.1f}" stroke="#e9ecef"/>')
        out.append(f'<text x="{L - 6}" y="{y(v) + 4:.1f}" text-anchor="end" fill="#495057">{v:,.0f}</text>')
    for i, (lab, _) in enumerate(hist):
        out.append(f'<text x="{x(i):.1f}" y="{H - B + 16}" text-anchor="middle" fill="#495057">{html.escape(lab)}</text>')
    for ci, c in enumerate(circuits):
        col = pal[ci % len(pal)]
        pts = [(x(i), y(r[c])) for i, (_, r) in enumerate(hist) if c in r]
        if len(pts) > 1:
            out.append(f'<polyline fill="none" stroke="{col}" stroke-width="2" points="{" ".join(f"{a:.1f},{b:.1f}" for a, b in pts)}"/>')
        for a, b in pts:
            out.append(f'<circle cx="{a:.1f}" cy="{b:.1f}" r="3" fill="{col}"/>')
        out.append(f'<rect x="{W - R + 12}" y="{T + 14 * ci}" width="10" height="10" fill="{col}"/>'
                   f'<text x="{W - R + 26}" y="{T + 14 * ci + 9}">{html.escape(c)}</text>')
    out.append("</svg>")
    open(path, "w").write("\n".join(out))


FEEDBACK = ["ota", "dac4", "bgr_core", "rc_filter", "pair"]  # tq_chain is ~5x larger; it would flatten the rest


def feedback_chart(assets, path):
    """Every candidate layout each circuit's loop tried: die area vs routed
    wirelength; solid = clean (|V| = 0), faint = rejected. Style of the old
    tools/readme_charts.py chart_feedback (7a16245)."""
    runs = []
    for c in FEEDBACK:
        p = os.path.join(assets, f"{c}.candidates.csv")
        if os.path.exists(p):
            rows = [(float(r["wl_um"]), float(r["area_um2"]), r["clean"] == "1") for r in csv.DictReader(open(p))]
            if rows:
                runs.append((c, rows))
    if not runs:
        return False
    AX, GRID = "#888888", "#88888833"
    pal = ["#3572b0", "#d1663a", "#2e9e6b", "#9b59b6", "#c9a227", "#c0392b"]
    W, H, x0, y0, x1, y1 = 900, 345, 60, 42, 560, 265
    wmax = max(w for _, r in runs for w, _, _ in r) * 1.05 or 1
    amax = max(a for _, r in runs for _, a, _ in r) * 1.05 or 1
    o = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" font-family="DejaVu Sans,Helvetica,Arial,sans-serif" font-size="10" fill="{AX}">',
         f'<text x="0" y="14" font-size="12" font-weight="bold">Feedback loop: every candidate layout tried, die area vs. wirelength</text>']
    for k in range(5):
        f = k / 4
        yy = y1 - f * (y1 - y0); xx = x0 + f * (x1 - x0)
        o.append(f'<line x1="{x0}" x2="{x1}" y1="{yy:.1f}" y2="{yy:.1f}" stroke="{GRID}"/>')
        o.append(f'<text x="{x0 - 6}" y="{yy + 3:.1f}" text-anchor="end">{f * amax:,.0f}</text>')
        o.append(f'<text x="{xx:.1f}" y="{y1 + 14}" text-anchor="middle">{f * wmax:,.0f}</text>')
    o.append(f'<line x1="{x0}" x2="{x1}" y1="{y1}" y2="{y1}" stroke="{AX}"/><line x1="{x0}" x2="{x0}" y1="{y0}" y2="{y1}" stroke="{AX}"/>')
    o.append(f'<text x="{(x0 + x1) / 2}" y="{y1 + 30}" text-anchor="middle">routed wirelength (um)</text><text x="{x0}" y="{y0 - 10}">die area (um^2)</text>')
    for i, (c, rows) in enumerate(runs):
        col = pal[i % len(pal)]
        for w, a, ok in rows:
            o.append(f'<circle cx="{x0 + w / wmax * (x1 - x0):.1f}" cy="{y1 - a / amax * (y1 - y0):.1f}" r="2" fill="{col}" fill-opacity="{0.75 if ok else 0.25}"/>')
    bx0, bx1 = 640, 880
    o.append(f'<text x="{bx0}" y="{y0 - 10}">candidates passing DRC + LVS + hard constraints</text>')
    for i, (c, rows) in enumerate(runs):
        n = len(rows); ok = sum(1 for r in rows if r[2]); y = y0 + 8 + i * 34
        o.append(f'<text x="{bx0}" y="{y - 3}">{html.escape(c)}</text><text x="{bx1}" y="{y - 3}" text-anchor="end">{ok}/{n}</text>')
        o.append(f'<rect x="{bx0}" y="{y + 2}" width="{bx1 - bx0}" height="10" rx="2" fill="{GRID}"/>')
        o.append(f'<rect x="{bx0}" y="{y + 2}" width="{(bx1 - bx0) * ok / n:.1f}" height="10" rx="2" fill="{pal[i % len(pal)]}"/>')
    o.append('<text x="0" y="322">Solid dots clear every check; faint dots were rejected. The loop ships the best clean candidate, so the rejects never reach GDS.</text>')
    o.append("</svg>")
    open(path, "w").write("\n".join(o))
    return True


def main():
    if sys.argv[1] == "--gate":
        gate(sys.argv[2])
    log, assets, sha = sys.argv[1:4]
    label = sys.argv[4] if len(sys.argv) > 4 else sha[:7]
    rows = parse(log)
    if not rows:
        sys.exit("no bench rows parsed")
    lay = os.path.join(PROG, "layouts")
    shutil.rmtree(lay, ignore_errors=True)  # regenerate: a circuit that failed this run shows no stale image
    os.makedirs(lay)
    csvp = os.path.join(PROG, "speed.csv")
    new = not os.path.exists(csvp)
    stamp = time.strftime("%Y-%m-%d %H:%M")
    with open(csvp, "a", newline="") as f:
        w = csv.writer(f)
        if new:
            w.writerow(["time", "label", "sha", "circuit", "wall_ms", "epochs", "ns_per_iter", "drc", "lvs", "erc"])
        for c, r in sorted(rows.items()):
            nspi = r["ms"] * 1e6 / r["epochs"] if r["epochs"] else 0
            w.writerow([stamp, label, sha, c, r["ms"], r["epochs"], f"{nspi:.0f}", r["drc"], r["lvs"], r["erc"]])
    hist = {}
    for r in csv.DictReader(open(csvp)):
        hist.setdefault(r["label"], {})[r["circuit"]] = float(r["ns_per_iter"]) / 1e6
    chart(list(hist.items()), os.path.join(PROG, "speed.svg"))
    fb = feedback_chart(assets, os.path.join(PROG, "feedback.svg"))
    shown = []
    for c in SAMPLES:
        src = os.path.join(assets, f"{c}.svg")
        if os.path.exists(src):
            shutil.copy(src, os.path.join(PROG, "layouts", f"{c}.svg"))
            shown.append(c)
    table = ["| circuit | wall ms | iterations | ns/iter | DRC | LVS | ERC |", "|---|---|---|---|---|---|---|"]
    for c, r in sorted(rows.items()):
        nspi = r["ms"] * 1e6 / r["epochs"] if r["epochs"] else 0
        table.append(f"| {c} | {r['ms']:,} | {r['epochs']} | {nspi:,.0f} | {r['drc']} | {r['lvs']} | {r['erc']} |")
    gallery = "\n\n".join(f"**{c}**\n\n![{c}](docs/progress/layouts/{c}.svg)" for c in shown)
    block = (f"<!-- progress:start -->\n_Last snapshot: {stamp}, branch `{os.environ.get('PROGRESS_BRANCH', 'main')}` at `{sha[:7]}` "
             f"(`bench local`, sky130, seed 1). Updated automatically by `benchmarks/progress.py`._\n\n" +
             (f"### Feedback loop\n\n![feedback](docs/progress/feedback.svg)\n\n" if fb else "") +
             f"### Speed\n\n![speed](docs/progress/speed.svg)\n\n" + "\n".join(table) +
             f"\n\n### Sample layouts\n\n{gallery}\n<!-- progress:end -->")
    rp = os.path.join(ROOT, "README.md")
    txt = open(rp).read() if os.path.exists(rp) else (
        "# Philis\n\nConstraint-aware analog place-and-route in Rust: SPICE in, placed, routed and "
        "signed-off GDS out (`philis run <netlist.sp> sky130 -o out/`). Plans and audits: "
        "[docs/plans/00-MASTER-PLAN.md](docs/plans/00-MASTER-PLAN.md).\n\n## Progress\n\n"
        "<!-- progress:start -->\n<!-- progress:end -->\n")
    txt = re.sub(r"<!-- progress:start -->.*?<!-- progress:end -->", lambda _: block, txt, flags=re.S)
    open(rp, "w").write(txt)
    print(f"{len(rows)} circuits, {len(shown)} layouts, history points {len(hist)}")


if __name__ == "__main__":
    main()
