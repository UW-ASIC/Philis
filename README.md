# Philis

**Analog layout from a SPICE netlist.** Philis recognises the circuit's topology, derives the layout constraints an analog designer would apply, then places, routes and signs off a GDS (DRC, LVS, ERC, PEX).

[![publish](https://github.com/UW-ASIC/Philis/actions/workflows/publish.yml/badge.svg)](https://github.com/UW-ASIC/Philis/actions/workflows/publish.yml)
![Rust](https://img.shields.io/badge/rust-stable-orange)
![PDKs](https://img.shields.io/badge/PDKs-sky130%20%7C%20gf180mcu%20%7C%20ihp__sg13g2-blue)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)

<p align="center"><img src="docs/progress/layouts/ota.svg" alt="OTA placed, routed and signed off by Philis on sky130" width="520"></p>

## Quick start

```sh
nix develop    # or: Rust stable + CMake + a C++ compiler
cargo build --release -p philis
./target/release/philis run benchmarks/fixtures/rc_filter.spice sky130 -o out/
```

```text
signoff CLEAN — cost 25.657
wrote out/{rc_filter.gds, rc_filter_ref.spice, rc_filter_pex.spice, signoff.txt, signoff.json, report.txt}
```

- Exit code 0 = signoff clean, 1 = not clean, 2 = error.
- `report.txt` lists every constraint as met, violated or unknown.
- Built-in PDKs: sky130, gf180mcu, ihp_sg13g2 and generic_finfet. Any other process is one JSON sidecar.
- Optional inputs: `--constraints` (ALIGN-style JSON), `--interface` (die and pins), `--op-lib`/`--perf` (ngspice operating point and post-layout specs). Full flag list: [`frontend/cli/src/main.rs`](frontend/cli/src/main.rs).

## Analog constraints

The annotator matches 91 topologies (differential pairs, current mirrors, cascodes, cross-coupled latches, OTAs, …) and classifies every net. Each rule below is enforced at one of three tiers: **hard** (must hold), **budget** (shares a bounded allowance) or **cost** (minimised).

**Matching** (Pelgrom; Hastings ch. 13)
- Mismatch ledger per matched pair: random σ (Pelgrom) plus systematic gradient, thermal and LOD terms, all drawn against one offset allowance
- Matching classes (minimal / moderate / precise), each with the layout environment it requires (Hastings' class-limit table)
- Common-centroid 1-D and 2-D arrays and interdigitation, with first and second unit moments equalised (ABBA, ABBA/BAAB)
- Orientation: matched channels parallel, with equal mean source→drain current direction
- Layout-dependent effects: equal well-proximity (WPE) and STI/LOD stress distances across matched members, plus edge dummies
- Capacitor arrays (binary-weighted and split DACs): spiral, chessboard or moment-balanced unit placement under an oxide-gradient model
- Capacitor plates: equal lead capacitance per unit, and no bottom plate under a top plate
- Sizing-reach report: pairs whose mismatch is set by sizing, not by layout

**Placement**
- Mirror symmetry about shared axes, with each symmetry group forming one connected island (Balasa–Graeb)
- Proximity of related devices; declared utilisation floor
- Thermal: matched sets kept isothermal and away from power dissipators
- Substrate noise: noisy-to-sensitive spacing (Charbon); deep-trench isolation banding
- Guard rings by role (minority-carrier injector, noisy aggressor, sensitive victim) and construction (tap ring, electron-collecting ring, hole-collecting ring, isolated tub)
- Performance-driven placement: estimated ground capacitance on spec-sensitive nets, priced before any wire exists

**Routing**
- Differential nets: matched, mirrored routes
- Common nodes: resistance balanced across branches (e.g. a pair's shared source)
- Crosstalk: pairwise exclusion, plus total coupling per victim summed over every aggressor
- Shielding of sensitive nets by a reference net
- Parasitic budgets per net, and circuit specs as a shared parasitic budget over the routed nets
- No metal over matched gates or precision resistor bodies
- Electromigration (DC, per layer and via; wires and via arrays sized to current) — hard
- IR drop along current-carrying nets
- Antenna ratio, fixed with jumpers or inserted diodes — hard

**Net classes** decide which routing rules a net gets: signal, clock, supply, ground, substrate, sensitive, bias, reference, static and switching digital, noisy.

**Reliability at signoff**
- |V<sub>GS</sub>| and |V<sub>DS</sub>| against oxide and drain ratings at the operating point
- Forward-biased junctions
- ESD width floor on pad nets
- Latch-up deck rules

## How it works

```text
SPICE → annotator → device generators → placement (analytic → anneal / sequence pair)
      → routing (negotiated global → track-lattice detailed) → signoff (DRC, LVS, ERC, PEX)
        ↑__________ each epoch's violations and spec misses steer the next __________|
```

The search ships only the best clean candidate. Design notes and the roadmap are in [`docs/plans/`](docs/plans/00-MASTER-PLAN.md).

## Results

Re-run on every push to `main`.

<!-- progress:start -->
_Last snapshot: 2026-10-04 14:28, branch `m2 + progress-charts` at `3032483` (`bench local`, sky130, seed 1). Updated automatically by `benchmarks/progress.py`._

### Feedback loop

![feedback](docs/progress/feedback.svg)

### Speed

![speed](docs/progress/speed.svg)

| circuit | wall ms | iterations | ns/iter | DRC | LVS | ERC |
|---|---|---|---|---|---|---|
| bgr_core | 56,132 | 15 | 3,742,133,333 | 0 | MATCH | 0 |
| dac4 | 113,905 | 20 | 5,695,250,000 | 7 | PARTIAL(16) | 1 |
| ota | 344,260 | 20 | 17,213,000,000 | 0 | MATCH | 0 |
| pair | 14,169 | 5 | 2,833,800,000 | 0 | MATCH | 0 |
| rc_filter | 14,081 | 10 | 1,408,100,000 | 0 | MATCH | 0 |
| tq_chain | 155,605 | 20 | 7,780,250,000 | 18 | MATCH | 5 |

### Sample layouts

**ota**

![ota](docs/progress/layouts/ota.svg)

**dac4**

![dac4](docs/progress/layouts/dac4.svg)

**bgr_core**

![bgr_core](docs/progress/layouts/bgr_core.svg)

**rc_filter**

![rc_filter](docs/progress/layouts/rc_filter.svg)

**pair**

![pair](docs/progress/layouts/pair.svg)

**tq_chain**

![tq_chain](docs/progress/layouts/tq_chain.svg)
<!-- progress:end -->

## Status

Pre-1.0. Most sample circuits sign off clean; dac4 and tq_chain do not yet ([#70](https://github.com/UW-ASIC/Philis/issues/70)). The [`publish`](.github/workflows/publish.yml) gate requires DRC 0, ERC 0 and LVS MATCH on every benchmark circuit. Roadmap and bugs: [issues](https://github.com/UW-ASIC/Philis/issues).

## Contributing

```sh
nix develop && cargo test --workspace
cargo run --release -p benchmark --bin bench local   # sign off every benchmark circuit
```

## License

[MIT](LICENSE)
