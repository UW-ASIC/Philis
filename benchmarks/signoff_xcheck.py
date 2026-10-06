#!/usr/bin/env python3
"""Foundry signoff cross-check (PERF-19): every `target/bench_debug/<f>/` that
`bench local` wrote, run through the sky130A foundry decks (KLayout DRC + LVS,
magic DRC + extraction) and compared with GPurify's own signoff.

    python3 benchmarks/signoff_xcheck.py              # all fixtures -> target/xcheck/summary.json
    python3 benchmarks/signoff_xcheck.py --drc-only X.gds [--top TOP]   # prints {"drc": n}

Needs `$PDK_ROOT/sky130A/libs.tech` and klayout + magic on PATH (nix-shell -p
klayout magic-vlsi). Exit 1 on any foundry DRC error, or a foundry LVS mismatch
where GPurify said match. Stdlib only.
"""
import json
import os
import re
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def vol() -> Path:
    pdk = os.environ.get("PDK_ROOT")
    if not pdk:
        sys.exit("PDK_ROOT unset")
    v = Path(pdk) / "sky130A" / "libs.tech"
    if not v.is_dir():
        sys.exit(f"{v} missing")
    return v


def drc_argv(gds: Path, top: str, report: Path) -> list:
    argv = ["klayout", "-b", "-r", str(vol() / "klayout/drc/sky130A_mr.drc"),
            "-rd", f"input={gds}", "-rd", f"top_cell={top}", "-rd", f"report={report}",
            "-rd", "feol=1", "-rd", "beol=1", "-rd", "offgrid=1", "-rd", f"thr={os.cpu_count() or 4}"]
    # feol/beol default to false in sky130A_mr.drc: without both the deck checks
    # nothing and reports 0 (pinned by tests/xcheck_smoke.rs).
    assert "feol=1" in argv and "beol=1" in argv
    return argv


def lyrdb_items(report: Path) -> list:
    """`(category, cell)` per `<item>` of a KLayout report database."""
    root = ET.parse(report).getroot()
    return [((i.findtext("category") or "").strip("'"), (i.findtext("cell") or "").strip("'"))
            for i in root.iter("item")]


def klayout_drc(gds: Path, top: str) -> list:
    report = gds.with_suffix(".drc.lyrdb")
    r = subprocess.run(drc_argv(gds, top, report), capture_output=True, text=True)
    if r.returncode != 0 or not report.is_file():
        sys.exit(f"klayout DRC failed on {gds}:\n{r.stdout[-2000:]}{r.stderr[-2000:]}")
    return lyrdb_items(report)


def klayout_lvs(d: Path, top: str) -> str:
    """`match` / `mismatch` from sky130.lvs called directly (run_lvs.py imports
    docopt and overrides target_netlist; every other deck variable is optional)."""
    gds, ref = d / f"{top}.lvs.gds", d / f"{top}.ref.spice"
    if not gds.is_file() or not ref.is_file():
        return "missing"
    r = subprocess.run(["klayout", "-b", "-r", str(vol() / "klayout/lvs/sky130.lvs"),
                        "-rd", f"input={gds}", "-rd", f"report={d / (top + '.lvsdb')}",
                        "-rd", f"schematic={ref}", "-rd", f"target_netlist={d / (top + '.ext.cir')}",
                        "-rd", "run_mode=deep", "-rd", f"thr={os.cpu_count() or 4}"],
                       capture_output=True, text=True)
    log = r.stdout + r.stderr
    if "Netlists match" in log:
        return "match"
    if "Netlists don't match" in log:
        return "mismatch"
    return f"error (exit {r.returncode}): {log.strip().splitlines()[-1:] or ''}"


SI = {"a": 1e-3, "f": 1.0, "p": 1e3, "n": 1e6, "u": 1e9}
CAP = re.compile(r"^C\S*\s+(\S+)\s+(\S+)\s+([0-9.eE+-]+)([afpnu]?)F?\b", re.I)
GROUND = {"0", "vsubs", "sub", "gnd"}


def magic(d: Path, top: str) -> dict:
    """magic DRC count and per-net ground C (fF) plus coupling summed per pair,
    from `<top>.spice` its extraction writes into the fixture dir."""
    tech = vol() / "magic/sky130A.tech"
    script = (f"gds read {top}.gds\nload {top}\nselect top cell\ndrc style drc(full)\ndrc check\n"
              "drc catchup\ndrc count total\nextract all\next2spice cthresh 0\next2spice\nquit -noprompt\n")
    r = subprocess.run(["magic", "-dnull", "-noconsole", "-T", str(tech)], input=script,
                       capture_output=True, text=True, cwd=d)
    m = re.search(r"Total DRC errors found:\s*(\d+)", r.stdout)
    spice = d / f"{top}.spice"
    if m is None or not spice.is_file():
        sys.exit(f"magic failed on {d}:\n{r.stdout[-2000:]}{r.stderr[-2000:]}")
    ground, coupling = {}, {}
    for line in spice.read_text().splitlines():
        c = CAP.match(line)
        if not c:
            continue
        a, b, v = c[1], c[2], float(c[3]) * SI[c[4].lower() or "f"]
        if b.lower() in GROUND:
            ground[a] = ground.get(a, 0.0) + v
        elif a.lower() in GROUND:
            ground[b] = ground.get(b, 0.0) + v
        else:
            k = "|".join(sorted((a, b)))
            coupling[k] = coupling.get(k, 0.0) + v
    return {"drc": int(m[1]), "ground": ground, "coupling": coupling}


def gpurify(d: Path) -> dict:
    rows = (d / "violations.txt").read_text().splitlines() if (d / "violations.txt").is_file() else []
    caps = json.loads((d / "caps.json").read_text()) if (d / "caps.json").is_file() else []
    return {"drc": sum(r.startswith("drc/") for r in rows),
            "lvs": "mismatch" if any(r.startswith("lvs/") for r in rows) else "match",
            "ground": {n: c for n, o, c in caps if o is None}}


def routing_layers() -> set:
    deck = json.loads((ROOT / "pdks/sky130.json").read_text())["cell"]["layers"]
    return set(deck["routing_metals"]) | set(deck["routing_vias"])


def layer_of(rule: str) -> str:
    """Foundry rule id (`m1.2`, `via2.1a`, `ct.4`, `li.3`) -> deck layer name."""
    p = rule.split(".")[0].lower()
    return {"ct": "mcon", "li": "li"}.get(p) or (re.sub(r"^m(\d)$", r"met\1", p))


def main() -> int:
    if len(sys.argv) >= 3 and sys.argv[1] == "--drc-only":
        top = sys.argv[sys.argv.index("--top") + 1] if "--top" in sys.argv else "TOP"
        with tempfile.TemporaryDirectory() as t:
            gds = Path(t) / "x.gds"
            gds.write_bytes(Path(sys.argv[2]).read_bytes())
            print(json.dumps({"drc": len(klayout_drc(gds, top))}))
        return 0
    routing = routing_layers()
    summary, bad = {}, []
    for d in sorted((ROOT / "target/bench_debug").iterdir()):
        top = d.name
        if not (d / f"{top}.gds").is_file():
            continue
        items = klayout_drc(d / f"{top}.gds", top)
        mg, gp = magic(d, top), gpurify(d)
        lvs = klayout_lvs(d, top)
        findings = [{"rule": r, "cell": c, "origin": "routing" if layer_of(r) in routing else "cells/assembly"}
                    for r, c in items]
        caps = [{"net": n, "gpurify": c, "magic": mg["ground"].get(n)} for n, c in sorted(gp["ground"].items())]
        summary[top] = {"drc": {"foundry": len(items), "magic": mg["drc"], "gpurify": gp["drc"]},
                        "lvs": {"foundry": lvs, "gpurify": gp["lvs"]},
                        "foundry_findings": findings, "caps": caps}
        if items:
            bad.append(f"{top}: {len(items)} foundry DRC errors")
        if lvs != "match" and gp["lvs"] == "match":
            bad.append(f"{top}: foundry LVS {lvs}, GPurify match")
    out = ROOT / "target/xcheck"
    out.mkdir(parents=True, exist_ok=True)
    (out / "summary.json").write_text(json.dumps(summary, indent=1))
    print(f"{'fixture':<18}{'DRC fdry':>9}{'magic':>7}{'GP':>5}  {'LVS fdry':<10}{'GP':<9}findings by origin/rule")
    for top, s in summary.items():
        by = {}
        for f in s["foundry_findings"]:
            k = f"{f['origin']}:{f['rule']}"
            by[k] = by.get(k, 0) + 1
        print(f"{top:<18}{s['drc']['foundry']:>9}{s['drc']['magic']:>7}{s['drc']['gpurify']:>5}  "
              f"{s['lvs']['foundry']:<10}{s['lvs']['gpurify']:<9}{' '.join(f'{k}={v}' for k, v in sorted(by.items()))}")
    for b in bad:
        print("FAIL", b)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
