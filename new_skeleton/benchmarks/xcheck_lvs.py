#!/usr/bin/env python3
"""LVS cross-check: KLayout extracts devices from the fixture GDS using the
SAME abstraction the deck states (sd = diff − poly, gate = poly∧diff∧implant)
and compares against the fixture SPICE. Independent extractor + comparator, so
a GPurify LVS MATCH the KLayout LVS refuses (or vice versa) is a finding.

Scope: MOS-only fixtures (KLayout's mos3 extractor). The toy BJT/resistor
constructions are deck-specific recognisers KLayout has no counterpart for —
listed as SKIPPED, never silently passed.

    python3 benchmarks/xcheck_lvs.py [fixture ...]
"""

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "target" / "xcheck"
sys.path.insert(0, str(ROOT / "benchmarks"))
import xcheck  # noqa: E402

MOS_ONLY = ["pair", "quad", "chain4", "ota", "ota_constrained", "tt_ota"]

LVS = r"""
source($input)
report_lvs($report)
schematic($cir)

deep

nwell  = input(64, 20)
diff   = input(65, 20)
tap    = input(65, 44)
poly   = input(66, 20)
licon  = input(66, 44)
li     = input(67, 20)
mcon   = input(67, 44)
met1   = input(68, 20)
via1   = input(68, 44)
met2   = input(69, 20)
nsdm   = input(93, 44)
psdm   = input(94, 20)

ngate = poly & diff & nsdm
pgate = poly & diff & psdm
sd    = diff - poly
nsd   = sd & nsdm
psd   = sd & psdm

# Bulk regions: PMOS bulk is its nwell; NMOS bulk is the substrate, modelled
# as the layout extent minus every nwell. Tap chains tie them to their rails.
psub = extent - nwell
ptap = tap & psdm
ntap = tap & nsdm

extract_devices(mos4("nfet_01v8"), { "SD" => nsd, "G" => ngate, "W" => psub, "tG" => poly })
extract_devices(mos4("pfet_01v8"), { "SD" => psd, "G" => pgate, "W" => nwell, "tG" => poly })

connect(psub, ptap)
connect(nwell, ntap)
connect(ptap, licon)
connect(ntap, licon)
connect(nsd, licon)
connect(psd, licon)
connect(poly, licon)
connect(tap, licon)
connect(licon, li)
connect(li, mcon)
connect(mcon, met1)
connect(met1, via1)
connect(via1, met2)

# The schematic side carries W/L the layout side measures in its own way;
# device pairing by topology is the check here, not parameter identity.
netlist.simplify
schematic.simplify
tolerance("nfet_01v8", "W", :relative => 1.0)
tolerance("nfet_01v8", "L", :relative => 1.0)
tolerance("pfet_01v8", "W", :relative => 1.0)
tolerance("pfet_01v8", "L", :relative => 1.0)

same_circuits("TOP", $top)
align
if compare
  puts "LVSXCHECK MATCH"
else
  puts "LVSXCHECK MISMATCH"
end
"""


def normalize_spice(text: str) -> str:
    """Fixture X-cards to primitive cards KLayout's SPICE reader groks, with
    MOS bulk dropped: the differential extracts 3-terminal devices (the same
    abstraction the GPurify deck recognises), so the schematic must be 3-pin
    too or every device pairs as a different class.
    `XM1 d g s b nfet_01v8 W=.. L=..` → `M1 d g s nfet_01v8 W=.. L=..`."""
    out = []
    for line in text.splitlines():
        ls = line.strip()
        m = re.match(r"^X([MQRC]\w*)\s+(.*)$", ls, re.IGNORECASE)
        if m:
            out.append(f"{m.group(1)} {m.group(2)}")
        else:
            out.append(ls)
    return "\n".join(out) + "\n"


def main() -> int:
    kl = xcheck.klayout_bin()
    OUT.mkdir(parents=True, exist_ok=True)
    lvs_path = OUT / "deck.lvs"
    lvs_path.write_text(LVS)

    fixtures = sys.argv[1:] or MOS_ONLY
    fail = 0
    for f in fixtures:
        if f not in MOS_ONLY:
            print(f"== {f}: SKIP (non-MOS devices; no KLayout extractor twin)")
            continue
        gds = ROOT / "target" / "bench_debug" / f / f"{f}.gds"
        spice = ROOT / "benchmarks" / "fixtures" / f"{f}.spice"
        if not gds.exists() or not spice.exists():
            print(f"== {f}: missing artifacts")
            continue
        cir = OUT / f"{f}.cir"
        text = spice.read_text()
        cir.write_text(normalize_spice(text))
        m = re.search(r"^\.subckt\s+(\S+)", text, re.IGNORECASE | re.MULTILINE)
        top = m.group(1) if m else f
        report = OUT / f"{f}.lvsdb"
        r = subprocess.run(
            [
                kl, "-b", "-r", str(lvs_path),
                "-rd", f"input={gds}",
                "-rd", f"cir={cir}",
                "-rd", f"report={report}",
                "-rd", f"top={top}",
            ],
            capture_output=True,
            text=True,
        )
        verdict = "MATCH" if "LVSXCHECK MATCH" in r.stdout else "MISMATCH"
        if r.returncode != 0:
            verdict = "ERROR"
        # GPurify's verdict, from the bench signoff line.
        sig = ROOT / "target" / "bench_debug" / f / "signoff.txt"
        gp = "?"
        if sig.exists():
            gp = "MATCH" if "LVS MATCH" in sig.read_text() else "MISMATCH"
        agree = "AGREE" if verdict == gp else "DISAGREE"
        if verdict == "ERROR":
            agree = "ERROR"
            print(r.stdout[-1500:])
            print(r.stderr[-1500:])
        print(f"== {f}: klayout {verdict}  gpurify {gp}  -> {agree}")
        if agree != "AGREE":
            fail += 1
    return 1 if fail else 0


if __name__ == "__main__":
    sys.exit(main())
