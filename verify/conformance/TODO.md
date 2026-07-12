# Conformance Suite TODO

## Open

- [x] GPU: device edge-pool buffers now content-hash cached (repeat uploads ~1µs)
  and chunked launches pipelined (queue all, read after). Measured on RTX 4060:
  upload was never the bottleneck (79µs); the pair kernel is compute-bound at
  ~25G evals/s. Thread-tiling (8 evals/thread) measured SLOWER (occupancy) and
  was reverted — see comment in pair_near_kernel.
- [ ] GPU next headroom (in measured-payoff order): (1) the far-mask enumerates
  the full |Ea|×|Eb| brute product while the CPU grid path is linear — CPU
  overtakes GPU around combs:16k fingers; feeding the kernel grid-localized edge
  pairs instead would make GPU work linear too. (2) Line<f32> vectorized loads /
  shared-memory tiling for the remaining ~6x to peak FLOPS. (3) calibration probe
  for GPU_MIN_PAIR_WORK instead of the 4060 constant.

- [ ] Boolean merge engine: min_enclosed_area / cheesing detect nested-poly holes
  only. Holes formed by a RING of touching polys (or by keyhole slits post-merge)
  need real polygon booleans (union + hole extraction). Also unlocks
  merged-geometry width/spacing without the merge_groups compound heuristics.

- [x] Shrink klayout_compare.py skip list — closed `prl_spacing` (space_check
  projection metric + min_projection), `wide_dependent_spacing` (bbox min-dim
  partition; the sized(-t/2).sized(t/2) trick collapses shapes exactly at the
  threshold), `eol_spacing` (short edges via 3-arg with_length + edge
  separation_check with projection metric), `min_density`/`max_density` (manual
  window loop mirroring check_density), `cheesing` (raw-shape containment —
  holes() can't see same-polarity slot markers), `redundant_via` (python center
  distance oracle). All 21 new cases agree exactly (75/75). Still skipped:
  antenna/antenna_car, asymmetric_enclosure, max_distance_to_tap,
  multi_patterning, polygon_validity, strict variants, via_array_spacing.
- [x] PEX W1: every supported rectilinear conductor gets both R and ground C.
  `PEX_PER_NET` pins 10 aF area + 176 aF fringe + 200 aF mutual = 386 aF per net.
  Unsupported R/C geometry is diagnostic and blocks checked per-net extraction.

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
- [ ] Production hierarchy-preserving compare with bound child ports/instances
- [ ] Black-box/equated-cell semantics beyond the current API placeholders
- [x] Isomorphic-graph / automorphism trap
- [x] Label-driven vs geometry-driven conflict
- [x] STRICT mode

## LVS — test cases missing (engine supports)

- [x] Intentional open (net split)
- [x] Parallel fingers vs multiplier
- [ ] VDD/VSS/name seeding that binds layout labels to reference net identities

## PEX — engine missing

- [x] Crossover / crossunder (3D coupling)
- [x] Fringe vs area ratio sweep
- [x] Corner & via resistance
- [x] Wide bus / comb / interdigitated
- [ ] Explicit process-calibrated fill impact (no size-based inference)
- [x] Floating-metal effects
- [ ] Ground-plane and shielding model from an explicit process stack
- [ ] High-aspect-ratio/distributed conductor and via topology

## PEX — test cases missing (engine supports)

- [x] Per-net attribution, including checked extraction diagnostics and floating sentinel nets
- [x] Spacing sweep (multiple S values)
- [x] Width sweep
- [x] Vertical parallel wires (only tested horizontal)

## Cross-cutting

- [x] Fuzzing layer: self-intersect, zero-width, acute slivers, coincident edges
- [x] STRICT / nominal dual-run infrastructure
- [x] Cross-tool differential testing infrastructure
- [x] Coverage report: rule → test matrix CSV (mirror GF180MCU `final_report.csv`)
