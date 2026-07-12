# TODO

## Upstream Data Gaps (require .op simulation data)

These gaps block automatic parasitic budget derivation and EM verification in the feedback loop. Each requires parsing ngspice `.op` output to extract per-device operating point data.

### G10: DC bias current (Id) per device

- **Where needed**: wire sizing (S3), EM check (S4), feedback loop wire-width hints
- **Why**: can't size wires without knowing current; can't check EM compliance
- **Resolution**: parse `.op` → extract Id per device → add `BiasCurrentTag` to constraint record (struct exists at `constraints/src/metadata/bias_current.rs`, zero producers)
- **Unblocks**: EM/IR feedback in routing loop, wire sizing recommendations

### G11: Per-net impedance (R_node)

- **Where needed**: auto parasitic budget computation (S1)
- **Why**: `C_budget = 1/(2*pi*f*R_node)` — currently parasitic budgets are manual
- **Resolution**: compute from `.op` data; add to `NetClassification`
- **Unblocks**: automatic parasitic budget generation (currently hand-specified per net)

### G12: Per-device overdrive voltage (Vgs - Vth)

- **Where needed**: matching tier inference (S1), Pelgrom current mismatch
- **Why**: `sigma^2(dId)/Id^2 = 4*sigma^2(dVth)/(Vgs-Vth)^2 + sigma^2(d_beta/beta)` — can't compute current mismatch budget without it
- **Resolution**: extract from `.op` data; add to `CellRecord` metadata
- **Unblocks**: automatic matching tier assignment (currently manual annotation)

### G13: Current flow direction per device

- **Where needed**: Seebeck cancellation (S0 cell gen), matched routing (S3)
- **Why**: even number of segments with antiparallel current flow cancels thermoelectric EMF [Hastings Ch. 8]
- **Resolution**: add to `CellRecord` and `ConstraintRecord`; derive from `.op` sign of Id
- **Unblocks**: thermoelectric-aware interdigitation and matched routing

### Implementation path

1. Add ngspice `.op` parser (read `Operating Point` section → per-device Id, Vgs, Vth, Vds)
2. Wire parsed data into constraint extraction pipeline
3. Auto-derive parasitic budgets from R_node + bandwidth target
4. Auto-derive matching tiers from Vgs-Vth + Pelgrom coefficients
5. Feed bias current into EM check and wire sizing

This would be in the frontend/ in a new crate, frontend/budgetter/ to budget the parasitics allow for R and C.
