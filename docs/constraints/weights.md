# Constraint Weighting Strategy

## Decision: Normalized Multi-Stage with Intra-Stage Graduated Weights

Output quality is the sole objective; wall-clock time is not a constraint.

---

## Problem

The engine's cost function combines multiple analog objectives into a single scalar for SA acceptance (`random() < exp(-Δcost / T)`). Today the weights are hardcoded, incommensurable constants:

| Term | Current weight | Units |
|------|---------------|-------|
| Symmetry | `4.0` | nm² (gradient scaling) |
| CC centroid | `2e-3` / `1e-3` | nm² × scaling |
| CC pattern slots | `0.5e-3` | nm² × scaling |
| Pulls (proximity/thermal) | per-rule `p.weight` | nm² × scaling |
| Alignment | `1.0` | nm (absolute) |
| Density lambda | ramps `×1.05` / epoch | unitless multiplier |
| Isolation (pushes) | binary hard | pass/fail |
| HPWL | `net_w` per net | nm |

These numbers are incommensurable: a weight of `4.0` on symmetry means different things depending on die size, cell count, and net topology. Tuning for one circuit breaks another. The ratio between terms controls which objective SA actually optimizes at each temperature level, so wrong ratios mean the solver optimizes the wrong thing at the wrong time.

---

## Options Considered

### 1. Fixed Scalar Sum (status quo)

`cost = w1*sym + w2*cc + w3*align + Σ(pull) + hpwl`

One number, constant weights. Incommensurable terms make weights meaningless across circuits. A "weight of 4.0" depends on die size. Tuning is per-circuit manual labor.

**Rejected:** does not generalize across circuits.

### 2. Normalized Fixed Weights

Divide each term by its initial value: `w_i * (term_i / term_i_0)`. All terms live in [0, 1], so `w_sym = 2.0` means "symmetry is twice as important as a unit-weight term" regardless of circuit.

**Verdict:** necessary foundation for any scheme, but fixed weights still can't express that different objectives matter at different optimization phases. Symmetry must be established early (fragile, hard to recover once broken); HPWL is smooth and recoverable late. A static compromise underweights one or the other.

### 3. Graduated / Scheduled Weights (single SA run)

Weights follow a time schedule during SA:

```
epoch 0-50:   sym=high, cc=high, hpwl=low     (establish topology)
epoch 50-150: sym=maintain, hpwl=ramp up       (optimize wirelength)
epoch 150+:   alignment=ramp up                (refinement)
```

Density lambda already does this (`×1.05` per epoch). Extending to all terms is straightforward.

**Problem:** competing objectives still share one SA run. At the crossover zone where symmetry ramps down and HPWL ramps up, neither dominates — SA wanders. The schedule is also static; a bandgap and a comparator get the same ramp.

**Rejected as standalone:** crossover interference within a single run degrades quality.

### 4. Adaptive Weights / Augmented Lagrangian

Weights self-tune per epoch based on violation:

```
w_i(t+1) = w_i(t) * (1 + α * violation_i / target_i)
```

Unsatisfied constraints get heavier; satisfied ones relax. The schedule emerges from the circuit.

**Problems in non-convex SA:**
- **Oscillation.** Violation high → weight up → solver over-corrects → violation low → weight down → violation returns. Requires damping, which is another tuning problem.
- **Cross-constraint chasing.** Increasing weight A moves cells to satisfy A, breaking B. Weight B rises, breaks A. Two constraints chase each other indefinitely.
- **No convergence guarantee.** Augmented Lagrangian converges for convex problems. SA + non-convex + multiple interacting constraints has no such guarantee.
- **Debugging.** When a layout is bad, the weight trace is a noisy signal across all epochs and all terms. Isolating the cause requires replaying the full optimization.

**Rejected:** fragile in practice despite theoretical elegance.

### 5. Constraint Promotion (soft → hard within one run)

Start all constraints as soft. Once a constraint stays satisfied for K consecutive epochs, promote it to a hard legality check so it can never regress.

**Premature promotion risk:** at high SA temperature, constraints get satisfied by chance. Promoting them locks in a possibly bad topology that the solver needed to explore past. Fixable by only promoting at low temperature after stabilization, but at that point the mechanism approximates multi-stage with extra state tracking.

**Rejected as standalone:** premature promotion risk; the careful guards needed to prevent it approach multi-stage complexity.

### 6. Pareto / Vector (no scalarization)

Maintain each objective as a separate value. Accept moves that are Pareto-improving (improve at least one objective without worsening any).

**Problems:**
- Requires maintaining archives of non-dominated solutions — major restructuring of the SA framework.
- Analog constraints ARE priority-ordered (matching > wirelength); Pareto doesn't express priority.
- Population-based methods (NSGA-II) are slow for the move-level granularity SA operates at.

**Rejected:** wrong framework for single-point SA.

### 7. Lexicographic / Priority Ordering

Optimize objectives in strict sequence: satisfy priority 1 fully, then optimize priority 2 within priority 1's feasible set, etc.

Natural for analog (designers think this way), but rigid: if priority 1 can't be perfectly satisfied, priority 2 never gets attention. Real circuits need soft tradeoffs within priority bands.

**Incorporated into multi-stage** as the inter-stage structure, with soft tradeoffs within each stage.

### 8. Normalized Multi-Stage (chosen)

See below.

---

## Chosen Approach: Normalized Multi-Stage

### Architecture

Three SA passes, each with a focused normalized cost function. Constraints promoted to hard at stage boundaries. Mild graduated weights within each stage for the 2-3 active terms.

```
Stage 1 — Topology Establishment
  Active terms: symmetry, CC centroid, CC pattern, proximity
  Hard constraints: die boundary
  Goal: establish analog structure (mirror axes, CC groups, proximity clusters)
  Cost: normalized weighted sum of active terms only
  No HPWL — wirelength must not compete with topology at this stage

Stage 2 — Placement Optimization
  Active terms: HPWL, pulls (proximity/thermal), density
  Hard constraints: symmetry, CC centroid (promoted from stage 1)
  Goal: optimize wirelength and area within the locked topology
  Symmetry/CC are now legality checks, not cost terms — SA rejects any
  move that breaks them, freeing the cost function to focus on HPWL

Stage 3 — Refinement
  Active terms: HPWL, alignment, fine proximity
  Hard constraints: everything from stages 1-2 plus isolation
  Goal: fine-tune alignment and remaining soft terms
  Narrow SA temperature range, small move window
```

### Why this produces the best layout quality

**1. The analog layout problem is hierarchical.**

Designers don't weigh matching against wirelength — they satisfy matching first, then optimize wirelength within that feasible space. Any scheme that scalarizes these into one number fights the problem's natural structure. Multi-stage respects it.

Reference: ALS ch7 Constraint Engineering — constraints have natural priority ordering driven by circuit function. AOAL ch8 Matching Rules — matching requirements are non-negotiable physical constraints, not soft preferences.

**2. Focused cost functions produce better SA results.**

With 6+ competing objectives in one cost function, a move that improves one term by 5% but worsens another by 4% looks like a 1% improvement. The solver can't distinguish signal from noise in the acceptance criterion. With 2-3 terms per stage, `Δcost` clearly reflects the objectives that matter at that phase. SA explores and exploits more effectively.

**3. Promotion at stage boundaries is deterministic.**

Adaptive promotion (option 5) risks premature lock-in during high-temperature exploration. Stage-boundary promotion happens after the full topology SA converges at low temperature — the topology is established, not accidental. This is the critical difference: promotion after convergence vs. promotion after K satisfied epochs (which can happen by chance).

**4. No oscillation or convergence risk.**

Weights within each stage are fixed (or mildly graduated). There is no feedback loop between violation and weight. The only dynamic is the SA temperature schedule, which is well-understood.

**5. Debuggable.**

When a layout has bad symmetry: look at stage 1. Bad wirelength: look at stage 2. Bad alignment: look at stage 3. Each stage has a focused cost function with 2-3 terms. With adaptive weights, a bad layout could be caused by any weight at any epoch — the diagnostic is replaying the full optimization trace.

### Circuit-awareness without adaptive complexity

The constraint extraction already produces per-pair metadata (`MatchingTier`, `ParasiticBudget`, `ThermalTag`). These drive stage configuration:

- **Initial weight magnitudes per stage:** tight matching tier → higher symmetry weight in stage 1. Tight parasitic budget → higher pull weight in stage 2.
- **Promotion thresholds:** tight matching → stricter symmetry satisfaction required to exit stage 1.
- **Per-stage iteration budget:** more constraints → more SA moves allocated to that stage.

The circuit-dependent variation is captured in stage configuration, not in dynamic weight updates. This is simpler, more predictable, and sufficient — the SCHEDULE of which-objective-when is similar across circuits; the MAGNITUDE of how-much-each-matters varies, and that's handled by initial weights.

### Intra-stage graduated weights

Within each stage, the 2-3 active terms may still benefit from mild ramping. For example, in stage 1:

- CC centroid weight starts higher than proximity weight (establish group structure before fine positioning)
- Proximity ramps up as CC converges

This is graduated weights (option 3) scoped to a small, non-competing set of terms. The crossover interference that plagues graduated weights in a single-run setting is minimal because the terms within a stage are cooperative, not adversarial.

### Normalization (foundation)

Every cost term is divided by its initial value at the start of each stage:

```
normalized_term_i = raw_term_i / raw_term_i(t=0)
```

This makes weights circuit-independent. `w = 2.0` means "twice as important" regardless of die size, cell count, or absolute error magnitudes. Without this, no weighting scheme works across circuits.

---

## Gaps and Further Reading Needed

- **ALS ch7 (Constraint Engineering):** likely contains formal constraint prioritization schemes that should validate or refine the stage structure. Not yet fully digested.
- **Intra-stage weight schedules:** the mild graduated ramps within each stage need empirical tuning or a principled schedule from the literature.
- **Routing-stage weighting:** this document covers placement only. The routing cost function (congestion vs. length-matching vs. shielding) needs its own analysis.
- **Feedback between stages:** if stage 2 cannot find a good HPWL solution within the hard constraints from stage 1, there is no mechanism to relax stage 1's topology. A backtracking or constraint-relaxation protocol may be needed; the books may describe approaches.
- **Thermal gradient interaction with matching:** AOAL ch08, FOLD ch7 — thermal constraints interact with matching constraints (thermal gradient across a matched pair degrades matching). The stage assignment of thermal terms (stage 1 topology vs. stage 2 placement) needs validation.
