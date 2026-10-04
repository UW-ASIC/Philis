# M2+ analog-matching: segment report

## MAT-14 runtime (card step "time gp and the ota flow before/after")
Before = `ac59bbe` (segment 3 base, pre-MAT-14, built from `git archive`); after = `5da7459` + review fixes 3 (test and
doc edits only, same release code). `--release`, alternated before/after on a shared 32-core host (load avg 10-13).

| run | before | after |
|---|---|---|
| `cargo test --release -p gp` (wall, 3 runs) | 0.11 / 0.12 / 0.12 s | 0.14 / 0.14 s (+ one 1.14 s outlier) |
| ota flow `library --test flow_smoke -- --exact matched_sets_are_reported` (harness time, 6 runs) | 3.20 3.19 4.79 3.23 3.21 3.21 s | 3.20 3.82 3.30 3.17 3.23 3.20 s |

Medians: gp 0.12 vs 0.14 s (10 unit tests, run time 0.01 s, noise-dominated); ota 3.21 vs 3.22 s (+0.3 %). Within
10 %. **Limit:** neither run heats a matched set. The ota run's rows report `mu_thermal == 0` for both pairs (probe
printed in a scratch edit, not committed), and gp's tests are tiny. The O(units·cells) `rise_at_point_mc` path in
`ledger_with` (`hot` is true only when some `power_uw != 0`) is therefore not measured by these benches. It runs once
per unit per `cost` call instead of twice per pair. The first benchmark with op-point power on a matched set has to
re-time dp; the stencil is the upgrade path.
