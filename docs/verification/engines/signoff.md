# Reliability signoff analyzers

[`signoff/mod.rs`](../../../verify/src/signoff/mod.rs) aggregates six families with
`CLEAN`, `VIOLATIONS`, `NOT_RUN` and `ERROR`. `all_clean()` is true only when every
required family actually ran with complete valid evidence and passed.

| Family | Current analysis core | Missing production integration |
|---|---|---|
| antenna | per-connected-net gate/collector metrics, area/sidewall mode, cumulative layers, diode correction and stable rule IDs | derive fabrication-stage networks and foundry gate/diode equations |
| density/CMP | explicit die, covering full/partial windows, exact rectangular union, min/max/gradient and first-order calibrated thickness model | fill/exclusions, multilevel foundry calibration and design extraction |
| IR drop | resistive DC solve, convergence/residual, absolute/percent/overvoltage limits | extracted supply graph and vector/vectorless currents/thermal map |
| electromigration | branch current, metal J, via-cut current, temperature derating, optional Blech and temperature limits | calibrated process limits, real geometry/current/thermal evidence |
| reliability | voltage/thermal limits and named inverse-power/Arrhenius lifetime models | device terminal histories, mission profiles and named foundry aging mechanisms |
| ESD/latch-up | directed capacity-qualified paths; clamps/R; guard-ring existence/continuity/width/bias/distance/taps | context-aware device/path extraction and substrate/well injector/victim evidence |

These are analysis cores, not foundry qualification. ESD topology/geometry verification
does not certify device-level HBM/CDM performance. Missing inputs/configuration are
`NOT_RUN`; malformed, ambiguous, unsupported or singular evidence is `ERROR`.

Wave 6 must derive evidence end to end from layout, LVS, PEX, activity and mission
profiles. Every result must retain geometry, net/device/terminal, stimulus/corner,
model revision and limit revision. See [Wave 6](../work-packages/wave-6.md).
