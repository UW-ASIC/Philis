# Conformance Suite TODO

## DRC — engine missing

- [x] Width w/ length dependency (wide-metal)
- [x] PRL / length-dependent spacing
- [x] EOL spacing
- [x] Same-net vs different-net spacing (STRICT mode)
- [x] Via/contact asymmetric enclosure (one-of-two-sides)
- [x] Well/implant enclosure
- [x] Min enclosed-area (hole) / slotting
- [x] Cheesing / max-density slotting
- [x] Antenna (PAR single-layer); CAR + diode relief need multi-layer connectivity
- [x] Redundant/double via & min-cut
- [x] Via array spacing
- [x] Well spacing & max-distance-to-tap / latch-up
- [x] Multi-patterning (LELE, SADP, TPL)
- [x] Self-intersecting / degenerate polygon detection
- [x] STRICT vs nominal dual-mode infrastructure

## DRC — test cases missing (engine supports)

- [x] Fuzzing: off-grid from boolean ops, zero-width slivers, coincident edges
- [x] Notch vs spacing disambiguation on merged shapes
- [x] Enclosure across concave corner
- [x] Density gradient between adjacent windows

## ERC — engine

- [x] Floating net / floating gate
- [x] Floating well / floating substrate
- [x] Missing well tie / substrate tie
- [x] Tie-high / tie-low violations
- [x] Unconnected / dangling pins
- [x] Shorts to power/ground; P/G swap
- [x] Multiple drivers on a net
- [x] Soft connection (via-only / high-R)
- [x] Antenna-electrical (charge accumulation)
- [x] ESD topological checks
- [x] HV domain crossing / voltage-aware isolation
- [x] EM current-density
- [x] Point-to-point resistance

## LVS — engine missing

- [x] Wrong device type / flavor (Vt variants)
- [x] Parametric W/L tolerance (runner passes W/L from manifest)
- [x] Series merge / parallel reduction
- [x] Hierarchical vs flat compare
- [x] Black-box / abstract
- [x] Isomorphic-graph / automorphism trap
- [x] Label-driven vs geometry-driven conflict
- [x] STRICT mode

## LVS — test cases missing (engine supports)

- [x] Intentional open (net split)
- [x] Parallel fingers vs multiplier
- [x] VDD/VSS swap (needs net seeding) — net_seeds field on RefNetlist; isomorphic detection in comparator

## PEX — engine missing

- [x] Crossover / crossunder (3D coupling)
- [x] Fringe vs area ratio sweep
- [x] Corner & via resistance
- [x] Wide bus / comb / interdigitated
- [x] Dummy-fill impact
- [x] Floating-metal effects
- [x] Ground-plane & shielding
- [x] High-aspect-ratio stacks

## PEX — test cases missing (engine supports)

- [x] Per-net attribution (engine has `run_pex_by_net`, runner doesn't use it)
- [x] Spacing sweep (multiple S values)
- [x] Width sweep
- [x] Vertical parallel wires (only tested horizontal)

## Cross-cutting

- [x] Fuzzing layer: self-intersect, zero-width, acute slivers, coincident edges
- [x] STRICT / nominal dual-run infrastructure
- [x] Cross-tool differential testing infrastructure
- [x] Coverage report: rule → test matrix CSV (mirror GF180MCU `final_report.csv`)
