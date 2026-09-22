# benchmark

Runs the full Philis flow (`library::run` → `library::signoff`, the same entry
points the `philis` CLI uses) over analog fixtures and prints a metrics table.

Built to benchmark against **MAGICAL** and **ALIGN** — and, where those suites
publish them, against known-optimal placements. That reference comparison is not
wired up yet; see [TODO.md](TODO.md).

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
count, LVS match, ERC count, PEX cost, area, utilisation, SA convergence. Then a
per-constraint-type satisfaction summary across all circuits.

Artifacts:

- `target/bench_debug/<name>/<name>.gds` + `signoff.txt`
- `assets/<name>.svg`

`gds2svg` is the standalone renderer for the same GDS files.

## Cross-checking GPurify (xcheck)

Three differentials hold `verify`'s numbers against KLayout — an independent
engine running the SAME rules, generated from `pdks/sky130.json` itself:

```sh
python3 benchmarks/xcheck.py            # DRC: per-rule counts vs signoff
python3 benchmarks/xcheck_lvs.py        # LVS verdicts vs fixture SPICE (MOS fixtures)
python3 benchmarks/xcheck_pex.py        # PEX: ground-cap lower bound + ratio band
python3 benchmarks/xcheck_selftest.py   # proves the pair can SEE violations:
                                        # KLayout-synthesised known-bad GDS, both
                                        # engines must report each planted fault
```

Run `bench local` first (the scripts read `target/bench_debug/`). KLayout is
found via `$KLAYOUT`, `PATH`, or nix. Rule kinds with no KLayout 1:1
(asymmetric enclosure, wide-metal, extensions, ERC) are listed as SKIPPED —
never silently passed. Counting conventions differ per engine (a square under
min-width is one finding to GPurify, two to KLayout), so the selftest asserts
*presence* agreement; the DRC diff reports raw counts.

Against the real PDK (optional, heavyweight): magic + sky130A via
`pip install volare && volare enable --pdk sky130 <version>`, then
`magic -dnull -noconsole -T sky130A` over the same GDS. That audits the
*deck's completeness* against real silicon rules, not the engine — expect
findings for rules the Philis deck deliberately simplifies.
