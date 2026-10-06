# Philis

**Analog layout from a SPICE netlist.** Philis reads your circuit, works out its layout constraints itself (symmetry, matching, common-centroid, EM, IR drop, antenna, parasitics), then places, routes and signs off a GDS. No constraint file and no hand layout needed.

[![publish](https://github.com/UW-ASIC/Philis/actions/workflows/publish.yml/badge.svg)](https://github.com/UW-ASIC/Philis/actions/workflows/publish.yml)
![Rust](https://img.shields.io/badge/rust-stable-orange)
![PDKs](https://img.shields.io/badge/PDKs-sky130%20%7C%20gf180mcu%20%7C%20ihp__sg13g2-blue)

<p align="center"><img src="docs/progress/layouts/ota.svg" alt="OTA placed, routed and signed off by Philis on sky130" width="560"></p>

```sh
philis run ota.spice sky130 -o out/    # → out/ota.gds, signed off: DRC, LVS, ERC, PEX
```

## Why Philis

- **Reads intent from the circuit.** 91 recognised topologies (differential pairs, current mirrors, cascodes, cross-coupled latches, OTAs, …) become placement and routing constraints automatically. An ALIGN-style JSON sidecar (`--constraints`) can add or override them.
- **Only ships clean layouts.** Every candidate is checked in-process for DRC, LVS, ERC and PEX ([GPurify](https://github.com/UW-ASIC/GPurify)). The search keeps the best clean one, and the exit code says whether it is clean (0 clean, 1 not clean, 2 error), so CI can gate on it.
- **Analog-aware all the way down.** Matched sets and common-centroid arrays, mirrored routing of differential nets, shields, electromigration-sized wires, antenna diodes, guard rings and dummies.
- **Performance in the loop (optional).** Give it an ngspice operating point (`--op-lib`) and post-layout specs (`--perf`). It scores layouts on the simulated metrics, not only on geometry.
- **Several PDKs.** sky130, gf180mcu, ihp_sg13g2 and a generic FinFET deck are built in. Any other process is one JSON sidecar.
- **One Rust binary.** No Python stack and no license server.

## Quick start

Prerequisites: Rust (stable), CMake and a C++ compiler (for the HiGHS solver), or just `nix develop`, which provides all of them plus klayout, magic, netgen and ngspice for cross-checks.

```sh
git clone https://github.com/UW-ASIC/Philis && cd Philis
cargo build --release -p philis
./target/release/philis run benchmarks/fixtures/rc_filter.spice sky130 -o out/
```

```text
signoff CLEAN — cost 25.657
wrote out/{rc_filter.gds, rc_filter_ref.spice, rc_filter_pex.spice, signoff.txt, signoff.json, report.txt}
```

Every run writes:

| file | what |
|---|---|
| `<top>.gds` | the layout, with `.subckt` ports as labels on the deck's text layers |
| `<top>_ref.spice` | the LVS reference signoff compared against (dummies included) |
| `<top>_pex.spice` | extracted parasitics, when extraction allows |
| `signoff.txt` / `.json` | DRC, LVS, ERC and PEX results |
| `report.txt` | per-constraint budgets (met / violated / unknown), recognised topologies, run stats |

Common flags (run `philis` with no arguments for the full usage line, or see the header of [`frontend/cli/src/main.rs`](frontend/cli/src/main.rs)):

| flag | effect |
|---|---|
| `--seed N`, `--iters N`, `--starts N`, `--max-wall S` | search effort and reproducibility |
| `--constraints FILE` | ALIGN-style JSON constraints on top of the extracted ones |
| `--interface FILE` | die size and boundary pins, checked against the ports |
| `--op-lib PATH`, `--perf SPECS.json` | simulate the operating point and post-layout specs with ngspice |
| `--hierarchy flat\|auto\|bottom-up:N` | solve each sub-circuit once and place it as a block |

As a library:

```rust
let pdk = verify::Pdk::builtin("sky130").expect("built-in deck");
let sol = library::run(&spice, &pdk, &library::Macros::default(), &library::Config::default()).expect("flow");
let signoff = library::signoff(&sol, &pdk);
```

## How it works

```text
SPICE ─► annotator ─► device generators ─► global placement ─► detailed placement ─► global routing ─► detailed routing ─► signoff
          (91 topologies →                   (analytic)          (annealing,           (negotiated       (track lattice,     (DRC, LVS, ERC,
           constraints)                                           sequence pair)        congestion)       EM-sized wires)     PEX)
              ▲                                                                                                                │
              └──────────────── each epoch's violations and spec misses steer the next one (warm and cold restarts) ─────────┘
```

Design notes, audits and the implementation plan are in [`docs/plans/`](docs/plans/00-MASTER-PLAN.md). Device-physics background is in [`docs/LAYOUT-FUNDAMENTALS.md`](docs/LAYOUT-FUNDAMENTALS.md).

## Results

Every push to `main` re-runs the sample circuits and refreshes this section: speed, the feedback loop's candidates, and the layouts.

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

Pre-1.0 (`0.1.0`) and under active development. What that means today:

- Most sample circuits sign off clean. dac4 and tq_chain do not yet (see the table above and [#70](https://github.com/UW-ASIC/Philis/issues/70)).
- The CI gate ([`publish.yml`](.github/workflows/publish.yml)) requires DRC 0, ERC 0 and LVS MATCH on every benchmark circuit. It stays red until those circuits are fixed.
- Roadmap and known bugs: [issues](https://github.com/UW-ASIC/Philis/issues).

## Contributing

```sh
nix develop                      # toolchain + EDA tools
cargo test --workspace           # unit and integration tests
cargo run --release -p benchmark --bin bench local   # sign off every benchmark circuit
```

Issues labelled `enhancement` map one-to-one to plan items in `docs/plans/`. Each one lists its plan section, its dependencies and its "done when" check.
