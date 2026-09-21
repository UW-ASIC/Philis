#!/usr/bin/env python3
"""Cross-check GPurify signoff against KLayout — an independent engine running
THE SAME rules, generated straight from pdks/sky130.json.

    python3 benchmarks/xcheck.py [fixture ...]        # default: all with GDS

Per fixture it needs the bench artifacts (run `bench local` first):
    target/bench_debug/<f>/<f>.gds          the drawn layout
    target/bench_debug/<f>/drc_located.txt  GPurify's findings (rule\tlayer\t...)

It emits target/xcheck/deck.drc (KLayout DSL) once, runs KLayout headless per
fixture, and prints a per-rule count diff. Rule kinds KLayout cannot express
1:1 are listed as SKIPPED so silence never reads as agreement.

Environment: KLAYOUT=/path/to/klayout overrides discovery (PATH, then nix).
"""

import json
import os
import re
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DECK = ROOT / "pdks" / "sky130.json"
OUT = ROOT / "target" / "xcheck"

# Rule kinds with a faithful KLayout counterpart. Everything else is reported
# as skipped — a differential that silently ignores rules is worse than none.
# Residual drift below rule-kind granularity (all empirically probed on
# KLayout 0.30.8, presence-level agreement holds):
#   min_spacing_diff  corner-touch: GPurify calls point-contact distance 0 a
#                     violation; the KLayout `&` term is zero-area and silent.
#   overlap           KLayout merges the boolean before measuring, so one
#                     too-thin figure spanning several GPurify pairs is one
#                     finding, not one per pair (and GPurify measures the bbox
#                     meet, not the exact boolean). Touch-only pairs: GPurify
#                     calls a degenerate (zero-area) meet an under-sized
#                     overlap, the exact boolean is empty and silent — visible
#                     on pre-fix ota artifacts (grazed cuts), gone on current
#                     cell geometry.
FAITHFUL = {
    "min_width",
    "max_width",
    "min_spacing",
    "notch",
    "min_area",
    "min_enclosure",
    "asymmetric_enclosure",
    "min_extension",
    "min_spacing_diff",
    "min_enclosed_area",
    "overlap",
    "wide_dependent_spacing",
    "off_grid",
}
# Expressible but with known semantic drift (flagged in the table). Empty
# today; the kinds above carry their residual notes inline.
# Kinds whose KLayout arm has a KNOWN semantic drift: divergences print with
# a `~` and count as approx-drift, not DISAGREE.
#   asymmetric_enclosure — the interacting(...,2) side-count heuristic
#   miscounts fragmented union edges (a stub notched into a pad splits the
#   short side into two edge objects); verified false-positive on rc_filter
#   gate stubs where GPurify's per-axis bbox-margin semantics are correct.
APPROX = {"asymmetric_enclosure"}


def klayout_bin() -> str:
    if os.environ.get("KLAYOUT"):
        return os.environ["KLAYOUT"]
    from shutil import which

    if which("klayout"):
        return "klayout"
    r = subprocess.run(
        ["nix", "build", "nixpkgs#klayout", "--no-link", "--print-out-paths"],
        capture_output=True,
        text=True,
    )
    if r.returncode == 0 and r.stdout.strip():
        return r.stdout.strip().splitlines()[-1] + "/bin/klayout"
    sys.exit("no klayout found (PATH, $KLAYOUT, or nix)")


def um(nm: int) -> str:
    return f"{nm / 1000.0}"


def gen_drc(deck: dict) -> tuple[str, list[str], list[str]]:
    """The KLayout .drc text, the rule ids it checks, and the skipped ones."""
    layers = deck["layers"]
    lines = [
        "# GENERATED from pdks/sky130.json by benchmarks/xcheck.py — do not edit.",
        "source($input)",
        'report("xcheck", $report)',
        "",
    ]
    for name, (l, d) in layers.items():
        lines.append(f"_{name} = input({l}, {d})")
    # Derived layers, deck order (operands may be derived themselves).
    ops = {"and": "&", "or": "|", "not": "-"}
    for row in deck.get("derived", []):
        op = ops[row["op"]]
        expr = f" {op} ".join(f"_{x}" for x in row["layers"])
        lines.append(f"_{row['name']} = {expr}")
    lines.append("")

    checked, skipped = [], []
    for rid, spec in deck["rules"].items():
        kind = spec["kind"]
        lyr = spec.get("layers", [])
        p = spec.get("params", {})
        lim = p.get("limit", {}).get("nm")
        if kind not in FAITHFUL | APPROX:
            skipped.append(f"{rid} [{kind}]")
            continue
        if kind == "min_width":
            lines.append(
                f'_{lyr[0]}.width({um(lim)}.um, projection).output("{rid}", "{rid}")'
            )
        elif kind == "max_width":
            # Anything not touched by a sub-(limit+1nm) width marker is wider
            # than the limit everywhere — an over-size via cut.
            lines.append(
                f'_{lyr[0]}.not_interacting(_{lyr[0]}.width({um(lim + 1)}.um, '
                f'projection).polygons(0.001.um)).output("{rid}", "{rid}")'
            )
        elif kind == "min_spacing":
            # euclidian: GPurify measures the exact euclidean gap (corner-to-
            # corner included), not the projected one.
            lines.append(
                f'_{lyr[0]}.isolated({um(lim)}.um, euclidian).output("{rid}", "{rid}")'
            )
        elif kind == "notch":
            lines.append(
                f'_{lyr[0]}.notch({um(lim)}.um, projection).output("{rid}", "{rid}")'
            )
        elif kind == "min_area":
            # Deck states the limit as the SIDE of the minimum square (see
            # Pdk::min_area); KLayout wants the area itself.
            a = (lim / 1000.0) ** 2
            lines.append(f'_{lyr[0]}.with_area(0, {a}).output("{rid}", "{rid}")')
        elif kind == "min_enclosure":
            # enclosing() is silent on an inner with no outer at all; GPurify
            # calls an unhosted inner zero enclosure, so the not_inside term
            # supplies those.
            outer, inner = lyr[0], lyr[1]
            lines.append(
                f'(_{outer}.enclosing(_{inner}, {um(lim)}.um, projection)'
                f'.polygons(0.001.um) + _{inner}.not_inside(_{outer}))'
                f'.output("{rid}", "{rid}")'
            )
        elif kind == "asymmetric_enclosure":
            # GPurify passes iff EACH axis keeps one side at or above
            # min_one_side (two adjacent good sides). Violation: both edges of
            # some axis under-enclosed — second_edges of enclosing() are the
            # inner edges with enclosure < limit; 90° edges bound the
            # horizontal axis, 0° the vertical. Unhosted inners fail outright.
            outer, inner = lyr[0], lyr[1]
            v = p["min_one_side"]["nm"]
            lines.append(
                f'_ep_{rid} = _{outer}.enclosing(_{inner}, {um(v)}.um, '
                f'projection).second_edges'
            )
            lines.append(
                f'(_{inner}.interacting(_ep_{rid}.with_angle(90), 2)'
                f' | _{inner}.interacting(_ep_{rid}.with_angle(0), 2)'
                f' | _{inner}.not_inside(_{outer})).output("{rid}", "{rid}")'
            )
        elif kind == "min_extension":
            # lyr[0] must stick out past its meet with lyr[1] by the limit
            # (poly endcap / diff overhang). Bad edges: meet edges the layer
            # under-encloses, minus those lying on the layer's own boundary
            # (sides where the REFERENCE overhangs — not this rule's business).
            # A layer swallowed whole by the reference protrudes nowhere.
            lay, ref = lyr[0], lyr[1]
            lines.append(f'_g_{rid} = _{lay} & _{ref}')
            lines.append(
                f'_bad_{rid} = _{lay}.enclosing(_g_{rid}, {um(lim)}.um, '
                f'projection).second_edges.not(_{lay}.edges)'
            )
            lines.append(
                f'(_g_{rid}.interacting(_bad_{rid}) | _{lay}.inside(_{ref}))'
                f'.output("{rid}", "{rid}")'
            )
        elif kind == "min_spacing_diff":
            # separation() never reports overlapping polygons; GPurify calls
            # overlap zero spacing, so the boolean term supplies those.
            # Residual: a pure corner touch is zero-area and stays invisible.
            a_l, b_l = lyr[0], lyr[1]
            lines.append(
                f'(_{a_l}.separation(_{b_l}, {um(lim)}.um, euclidian)'
                f'.polygons(0.001.um) + (_{a_l} & _{b_l}))'
                f'.output("{rid}", "{rid}")'
            )
        elif kind == "min_enclosed_area":
            a = (lim / 1000.0) ** 2
            lines.append(
                f'_{lyr[0]}.holes.with_area(0, {a}).output("{rid}", "{rid}")'
            )
        elif kind == "off_grid":
            g = p.get("pitch", {}).get("nm", 5)
            lines.append(
                f'(_nwell + _diff + _tap + _poly + _licon + _li + _mcon + _met1 '
                f'+ _via1 + _met2).ongrid({g / 1000.0}.um).output("{rid}", "{rid}")'
            )
        elif kind == "overlap":
            # The smaller side of the intersection figure under the limit.
            # Residual: .merged fuses figures across GPurify's per-pair split.
            a_l, b_l = lyr[0], lyr[1]
            lines.append(
                f'(_{a_l} & _{b_l}).width({um(lim)}.um, projection)'
                f'.polygons(0.001.um).merged.output("{rid}", "{rid}")'
            )
        elif kind == "wide_dependent_spacing":
            # Pairs under the wide-metal limit where at least one side is a
            # wide plate (no point of it narrower than the threshold).
            thr = p["width_threshold"]["nm"]
            lines.append(
                f'_w_{rid} = _{lyr[0]}.not_interacting(_{lyr[0]}.width('
                f'{um(thr)}.um, projection).polygons(0.001.um))'
            )
            lines.append(
                f'_{lyr[0]}.isolated({um(lim)}.um, euclidian).polygons(0.001.um)'
                f'.interacting(_w_{rid}).output("{rid}", "{rid}")'
            )
        checked.append(rid)
    return "\n".join(lines) + "\n", checked, skipped


def klayout_counts(report: Path) -> dict[str, int]:
    """Category → item count out of a KLayout lyrdb."""
    tree = ET.parse(report)
    counts: dict[str, int] = {}
    # Items reference their category by name ('cat_name' tag per item).
    for item in tree.getroot().iter("item"):
        cat = item.findtext("category") or ""
        cat = cat.strip("'")
        counts[cat] = counts.get(cat, 0) + 1
    return counts


def gpurify_counts(located: Path) -> dict[str, int]:
    counts: dict[str, int] = {}
    if not located.exists():
        return counts
    for line in located.read_text().splitlines():
        rid = line.split("\t", 1)[0].strip()
        if rid:
            counts[rid] = counts.get(rid, 0) + 1
    return counts


def main() -> int:
    deck = json.loads(DECK.read_text())
    OUT.mkdir(parents=True, exist_ok=True)
    drc_text, checked, skipped = gen_drc(deck)
    drc_path = OUT / "deck.drc"
    drc_path.write_text(drc_text)

    fixtures = sys.argv[1:]
    if not fixtures:
        dbg = ROOT / "target" / "bench_debug"
        fixtures = sorted(
            d.name for d in dbg.iterdir() if (d / f"{d.name}.gds").exists()
        )
    kl = klayout_bin()

    print(f"rules checked: {len(checked)}   skipped (no KLayout 1:1): {len(skipped)}")
    for s in skipped:
        print(f"  SKIP {s}")
    print()

    exit_code = 0
    for f in fixtures:
        gds = ROOT / "target" / "bench_debug" / f / f"{f}.gds"
        located = ROOT / "target" / "bench_debug" / f / "drc_located.txt"
        if not gds.exists():
            print(f"== {f}: no GDS (run bench first)")
            continue
        report = OUT / f"{f}.lyrdb"
        r = subprocess.run(
            [
                kl,
                "-b",
                "-r",
                str(drc_path),
                "-rd",
                f"input={gds}",
                "-rd",
                f"report={report}",
            ],
            capture_output=True,
            text=True,
        )
        if r.returncode != 0:
            print(f"== {f}: klayout FAILED\n{r.stderr[-2000:]}")
            exit_code = 1
            continue
        kc = klayout_counts(report)
        gc = gpurify_counts(located)
        # Compare only rules the differential actually runs; GPurify-only rows
        # for skipped kinds are shown as unverified, not as disagreement.
        rows = sorted(set(kc) | {r for r in gc if r in checked})
        diffs = [(rid, gc.get(rid, 0), kc.get(rid, 0)) for rid in rows]
        approx_rids = {
            r
            for r in rows
            if deck["rules"].get(r, {}).get("kind") in APPROX
        }
        bad = [d for d in diffs if d[1] != d[2] and d[0] not in approx_rids]
        drift = [d for d in diffs if d[1] != d[2] and d[0] in approx_rids]
        unverified = {r: n for r, n in gc.items() if r not in checked}
        status = "AGREE" if not bad else f"{len(bad)} DISAGREE"
        if drift and not bad:
            status += f" ({len(drift)} approx-drift ~)"
        print(f"== {f}: {status}  (gpurify rows {sum(gc.values())}, klayout rows {sum(kc.values())})")
        for rid, g, k in diffs:
            mark = "  " if g == k else "->"
            tag = " ~" if any(rid == c and deck["rules"].get(rid, {}).get("kind") in APPROX for c in [rid]) else ""
            if g or k:
                print(f"  {mark} {rid:34} gpurify {g:4}  klayout {k:4}{tag}")
        for rid, n in sorted(unverified.items()):
            print(f"   ? {rid:34} gpurify {n:4}  (kind not in differential)")
        if bad:
            exit_code = 1
    return exit_code


if __name__ == "__main__":
    sys.exit(main())
