#!/usr/bin/env python3
"""Pelgrom A_VT and VT tempco by simulating a PDK's own models in ngspice.

Mismatch: the PDK's mismatch corner, where every device instance draws its own
AGAUSS, so one ngspice run holds many devices; runs are chunked (parse time
grows with instances) and parallel. VT is the constant-current VGS at
Id = 0.1 uA * W/L, diode-connected (VDS = VGS). sigma(dVT) over the pairs is
fit through the origin against 1/sqrt(WL) (Pelgrom 1989 eq. 1).

Usage: characterize_mismatch.py [pdk] [pairs] [seed]     pdk in PDKS, default sky130
       characterize_mismatch.py tc [pdk]
The `tc` form instead fits dVT/dT (uV/K) at the typical corner over TEMPS.
"""
import glob, math, os, re, statistics, subprocess, sys, tempfile
from concurrent.futures import ThreadPoolExecutor

ICRIT_UA = 0.1
H = os.path.expanduser("~")


def first(pattern):
    hits = glob.glob(os.path.join(H, pattern))
    return hits[0] if hits else None


SKY = first(".volare/volare/sky130/versions/*/sky130A/libs.tech/ngspice/sky130.lib.spice")
GF = first(".volare/volare/gf180mcu/versions/*/gf180mcuD/libs.tech/ngspice")
IHP = first("pdks/IHP-Open-PDK/ihp-sg13g2/libs.tech/ngspice")

# Per PDK: model lines for the mismatch and typical corners, device names,
# instance suffix, .spiceinit, and W/L (um) sets at or above its Lmin.
PDKS = {
    "sky130": dict(
        mm=[f".lib {SKY} tt_mm"], tt=[f".lib {SKY} tt"],
        nfet="sky130_fd_pr__nfet_01v8", pfet="sky130_fd_pr__pfet_01v8",
        wl="W={w} L={l}", init="",
        geoms=[(1, 0.15), (2, 0.5), (4, 1), (8, 2), (1, 1), (4, 0.5)]),
    # design.ngspice defaults sw_stat_mismatch=0; the subckt .param blocks need hsa mode.
    "gf180mcu": dict(
        mm=[f".include {GF}/design.ngspice", f".lib {GF}/sm141064.ngspice typical", ".param sw_stat_mismatch=1"],
        tt=[f".include {GF}/design.ngspice", f".lib {GF}/sm141064.ngspice typical"],
        nfet="nfet_03v3", pfet="pfet_03v3", wl="w={w}u l={l}u", init="set ngbehavior=hsa\n",
        geoms=[(1, 0.28), (2, 0.5), (4, 1), (8, 2), (1, 1), (4, 0.5)]),
    # PSP103 via OSDI compiled from the repo's own verilog-a/psp103/psp103.va.
    "ihp_sg13g2": dict(
        mm=[f".lib {IHP}/models/cornerMOSlv.lib mos_tt_mismatch"], tt=[f".lib {IHP}/models/cornerMOSlv.lib mos_tt"],
        nfet="sg13_lv_nmos", pfet="sg13_lv_pmos", wl="w={w}u l={l}u mm_ok=1", init=f"osdi '{IHP}/osdi/psp103.osdi'\n",
        geoms=[(1, 0.13), (2, 0.5), (4, 1), (8, 2), (1, 1), (4, 0.5)]),
}


def spice(pdk, lines):
    """Run `lines` in ngspice with the PDK's .spiceinit; stdout."""
    with tempfile.TemporaryDirectory() as d:
        open(os.path.join(d, ".spiceinit"), "w").write(pdk["init"])
        open(os.path.join(d, "t.sp"), "w").write("\n".join(lines) + "\n")
        return subprocess.run(["ngspice", "-b", "t.sp"], cwd=d, capture_output=True, text=True).stdout


def devices(pdk, dev, geoms):
    """Diode-connected `dev` per (w, l), forced at ICRIT_UA * W/L; gate node g<k>."""
    n = dev == "nfet"
    out = []
    for k, (w, l) in enumerate(geoms):
        # nfet pulls current into its drain; pfet pushes it out.
        out.append(f"I{k} {'0 g' if n else 'g'}{k}{'' if n else ' 0'} {ICRIT_UA * w / l}u")
        out.append(f"X{k} g{k} g{k} 0 0 {pdk[dev]} " + pdk["wl"].format(w=w, l=l))
    return out


CHUNK = 50  # pairs per ngspice run: netlist parse time grows with instance count


def sim(pdk, dev, w, l, pairs, seed):
    """|VT| (V) of 2*pairs independent `dev` instances at W/L, one ngspice run."""
    sign = 1 if dev == "nfet" else -1
    lines = [f"* {dev} mismatch"] + pdk["mm"] + [f".option seed={seed}"]
    lines += devices(pdk, dev, [(w, l)] * (2 * pairs))
    lines += [".control", "op"] + [f"echo VG {k} $&v(g{k})" for k in range(2 * pairs)]
    out = spice(pdk, lines + [".endc", ".end"])
    vg = [sign * float(m[1]) for m in re.finditer(r"^VG \d+ (\S+)", out, re.M)]
    assert len(vg) == 2 * pairs, f"ngspice returned {len(vg)}/{2 * pairs} voltages:\n{out[-2000:]}"
    return vg


def run(pdk, dev, pairs, seed):
    GEOMS = pdk["geoms"]
    jobs = [(gi, c) for gi in range(len(GEOMS)) for c in range(math.ceil(pairs / CHUNK))]
    with ThreadPoolExecutor(os.cpu_count()) as ex:
        # every chunk its own seed: the same seed would redraw the same devices
        res = list(ex.map(lambda j: sim(pdk, dev, *GEOMS[j[0]], min(CHUNK, pairs - j[1] * CHUNK),
                                        seed * 1000 + j[0] * 100 + j[1]), jobs))
    rows = []
    for gi, (w, l) in enumerate(GEOMS):
        v = [x for (g, _), r in zip(jobs, res) if g == gi for x in r]
        dvt = [(v[2 * k] - v[2 * k + 1]) * 1e3 for k in range(len(v) // 2)]  # mV
        rows.append((w, l, 1 / math.sqrt(w * l), statistics.mean(v), statistics.stdev(dvt)))
    return rows


TEMPS = [0, 27, 85, 125]  # degC
TC_GEOMS = [(2, 0.5), (4, 1)]


def tempco(name):
    """dVT/dT per polarity and geometry, least squares over TEMPS at typical."""
    pdk = PDKS[name]
    print(f"pdk={name} {pdk['tt']} Id=0.1uA*W/L VDS=VGS temps={TEMPS}")
    for dev, sign in (("nfet", 1), ("pfet", -1)):
        lines = [f"* {dev} tempco"] + pdk["tt"] + devices(pdk, dev, TC_GEOMS) + [".control"]
        for t in TEMPS:
            lines += [f"option temp={t}", "op"] + [f"echo VT {t} {k} $&v(g{k})" for k in range(len(TC_GEOMS))]
        out = spice(pdk, lines + [".endc", ".end"])
        vt = {(int(m[1]), int(m[2])): sign * float(m[3]) for m in re.finditer(r"^VT (\S+) (\d+) (\S+)", out, re.M)}
        assert len(vt) == len(TEMPS) * len(TC_GEOMS), out[-2000:]
        for k, (w, l) in enumerate(TC_GEOMS):
            ys = [vt[(t, k)] for t in TEMPS]
            slope = statistics.linear_regression(TEMPS, ys).slope * 1e6
            print(f"{pdk[dev]} {w}/{l}: |VT| " + " ".join(f"{t}C={y:.4f}" for t, y in zip(TEMPS, ys))
                  + f"  d|VT|/dT = {slope:.0f} uV/K")


def main():
    if len(sys.argv) > 1 and sys.argv[1] == "tc":
        sys.argv.pop(1)
        return tempco(sys.argv[1] if len(sys.argv) > 1 else "sky130")
    name = sys.argv[1] if len(sys.argv) > 1 else "sky130"
    pdk = PDKS[name]
    pairs = int(sys.argv[2]) if len(sys.argv) > 2 else 250
    seed = int(sys.argv[3]) if len(sys.argv) > 3 else 1
    print(f"pdk={name} {pdk['mm']} pairs={pairs} seed={seed} Id=0.1uA*W/L VDS=VGS")
    for dev in ("nfet", "pfet"):
        rows = run(pdk, dev, pairs, seed)
        print(f"\n{pdk[dev]}   W/L(um)  1/sqrt(WL)  mean|VT|(V)  sigma_dVT(mV)  A_VT_point(mV.um)")
        for w, l, x, vt, s in rows:
            print(f"          {w}/{l:<5}  {x:9.3f}  {vt:10.4f}  {s:12.3f}  {s / x:10.3f}")
        # least squares through the origin: sigma = A * x
        a = sum(x * s for *_, x, _, s in rows) / sum(x * x for *_, x, _, s in rows)
        # sigma of a sample stdev is s/sqrt(2(n-1)); report the fit's own spread too
        resid = statistics.stdev([s / x for *_, x, _, s in rows])
        print(f"A_VT_{dev[0]} = {a:.2f} mV.um  (spread of point estimates {resid:.2f}; "
              f"per-point stat. error ~{100 / math.sqrt(2 * (pairs - 1)):.1f}%)")


if __name__ == "__main__":
    main()
