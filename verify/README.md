# gdsverify

Rust DRC, LVS, PEX, ERC and reliability-signoff foundations over checked GDS/OASIS
inputs or directly constructed geometry.

Start with the canonical [verification engineering guide](../docs/verification/README.md).
It contains current capability claims, fail-closed contracts, source reading order,
contributor workflow, engine boundaries, known bugs/limitations, qualification policy,
and the detailed Wave 2–7 work packages.

## Local commands

Run from the repository root:

```bash
cargo test -p gdsverify --lib
cargo run -p gdsverify --bin generate_conformance -- verify/conformance  # fresh worktree
cargo run -p gdsverify --bin conformance -- verify/conformance
cargo test -p gdsverify --all-targets --no-run
cargo test -p pnr-core --lib --no-run
cargo test -p pnr-backend --lib --no-run
```

The generated conformance manifest/GDS/params are ignored files. Generate them once in
a fresh worktree. Changes to generator sources or expected physics still require the
focused regression and engineering-disposition policy in the contributor guide.

Optional GPU builds require the CUDA runtime/toolkit. A CPU fallback is not GPU
validation. KLayout comparison is a local regression aid, not foundry correlation:

```bash
python3 verify/conformance/klayout_compare.py verify/conformance
```

## Key source entry points

- [`src/lib.rs`](src/lib.rs): public facade and compatibility/strict loading policies
- [`src/geometry/exact.rs`](src/geometry/exact.rs): exact geometry foundation
- [`src/gds_lossless.rs`](src/gds_lossless.rs), [`src/oasis.rs`](src/oasis.rs): formats
- [`src/drc/mod.rs`](src/drc/mod.rs): DRC
- [`src/lvs/`](src/lvs/): extraction, netlist binding and comparison
- [`src/pex/mod.rs`](src/pex/mod.rs): current analytical PEX
- [`src/signoff/`](src/signoff/): antenna, CMP/density, IR/EM, aging and ESD/latch-up
- [`correlation/`](correlation/): neutral correlation/freeze schemas and fixtures

The audited capability score remains **24.5/112 (22%)** until independent
general-input correlation evidence is accepted. Local green tests do not mean
proprietary parity or foundry qualification.
