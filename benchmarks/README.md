# benchmark

Runs the full Philis flow (`library::run` → `library::signoff`, the same entry
points the `philis` CLI uses) over analog fixtures and prints a metrics table.

Built to benchmark against **MAGICAL** and **ALIGN** — and, where those suites
publish them, against known-optimal placements. That reference comparison is not
wired up yet.

## Run

```sh
cargo run --release -p benchmark --bin bench [local|align|magical|tinytapeout|all]
```

Default suite is `local`. Extra args filter: a single number takes the first N
circuits, anything else matches circuit names exactly.

```sh
cargo run --release -p benchmark --bin bench local ota tt_ota   # named circuits
cargo run --release -p benchmark --bin bench magical 5          # first 5
```

## Environment

| Var | Default | Meaning |
| --- | --- | --- |
| `PNR_BENCH_SEED` | `1` | SA seed. Results are seed-noisy — vary it before quoting numbers. |
| `PNR_BENCH_PDK` | per-suite | Deck override: a name under `pdks/` (`sky130`, `generic_finfet`) or a path. |

Without the override, ALIGN and MAGICAL run on `generic_finfet` and everything
else on `sky130`. Set `PNR_BENCH_PDK` to run the same fixtures against both:

```sh
PNR_BENCH_PDK=sky130         cargo run --release -p benchmark --bin bench align
PNR_BENCH_PDK=generic_finfet cargo run --release -p benchmark --bin bench align
```

Each row is tagged `[Suite/deck]` so the two runs are distinguishable.

Caveat: the finfet netlists specify `nfin`, which is mapped to a planar width at
0.1 µm/fin. Sky130 numbers for those circuits are internally consistent but are
not comparable to published finfet results.

## Fixtures

- **local** — `.spice` / `.sp` files sitting directly in `fixtures/`. No network.
- **align**, **magical** — git submodules under `competition/`, pinned in
  `.gitmodules`. They are the reference we benchmark against, so they stay
  checked out. First clone:

  ```sh
  git submodule update --init --depth 1
  ```

- **tinytapeout** — cloned on demand into `fixtures/` at pinned commits
  (`fixtures.rs: REPOS`) and removed again when the run ends. Local fixtures and
  `competition/` survive cleanup.

Netlists are preprocessed before parsing: backslash-continuation joining,
`.param` resolution, bare R/C rewritten to real PDK devices via cap density and
sheet resistance, and `nfin` → `W` synthesis.

## Output

Per circuit: cell/net counts, wirelength, unrouted nets, routing overuse, DRC
count, LVS match, ERC count, PEX cost, area, utilisation, SA convergence, the
winner's placement metrics (`RunStats::place`: area usage = footprint / Σ cell
bbox, lattice offenders, overlap and clearance residue nm², matched-geometry
mismatches) and dp's anneal counters (`RunStats::dp`). Then a
per-constraint-type satisfaction summary across all circuits.

Artifacts:

- `target/bench_debug/<name>/`: `<name>.gds`, `signoff.txt`, `violations.txt`, `drc_located.txt`
- `assets/<name>.svg`

`gds2svg` is the standalone renderer for the same GDS files.

## Cross-checking GPurify (foundry signoff)

`benchmarks/signoff_xcheck.py` holds GPurify's signoff against the sky130A
foundry decks (`$PDK_ROOT/sky130A/libs.tech`), engines Philis did not write:

```sh
cargo run --release -p benchmark --bin bench local
nix-shell -p klayout magic-vlsi --run 'python3 benchmarks/signoff_xcheck.py'
```

Per `target/bench_debug/<f>/` it runs KLayout `sky130A_mr.drc` on `<f>.gds`,
KLayout `sky130.lvs` (called directly; `run_lvs.py` needs docopt) on
`<f>.lvs.gds` (schematic ports labelled) against `<f>.ref.spice` (the LVS
reference, dummies included), and magic `drc(full)` + extraction (tech file
passed with `-T`: volare's `.magicrc` hard-codes its build path).
`target/xcheck/summary.json` holds per fixture DRC counts {foundry, magic,
gpurify}, LVS verdicts {foundry, gpurify}, every foundry finding with its
origin (routing vs cells/assembly, by layer), and ground C per net {gpurify
(`caps.json`), magic}. Exit 1 on any foundry DRC error, or a foundry LVS
mismatch where GPurify said match.

`feol=1 beol=1` are mandatory: `sky130A_mr.drc` defaults both to false and
then checks nothing, reporting 0 (`tests/xcheck_smoke.rs` pins both halves).
`--drc-only <gds> [--top T]` prints `{"drc": n}` for one GDS.

`cargo run --release -p benchmark --example net_dump <fixture>` prints the
extraction net by net and which extracted net each routed net lands on — a
routed net split across two extracted nets is an open.
