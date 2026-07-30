//! # `dp` — detailed placement (an Algorithm-contract stage)
//!
//! [`DetailedPlacer`] legalises and finalises the coarse placement `gp` produced.
//! The real impl is [`Annealer`]: bounded-move Metropolis SA with a shrinking
//! displacement window, a variant-reshape move, and a final grid snap + symmetry
//! re-snap. Drop-in like every stage; the active algorithm is selected in
//! `frontend/library`'s `algorithms` module.
//!
//! **Analog-blind.** The objective is built-in HPWL plus the analog cost
//! `Σ reqs.cost[b].cost(layout)` and the priced budget residuals — no analog weight is
//! hardcoded. The returned result is legal iff `Report.hard_violations` is empty.
//!
//! **Acceptance is lexicographic, not scalar** (D10 / PLAN §3b). Every move is gated on
//! `(|V|, Φ margin, Θ residual)` being non-increasing, and Metropolis is consulted only
//! when all three are unchanged, on the PEX tier alone — see [`accept`]. Overlap moved
//! into the V tier with it: `mechanics::report` calls residual overlap a *hard*
//! violation, so pricing it in the objective with a finite ramped weight was precisely
//! PLAN §3b's "a penalty that is finite is a bribe the optimizer will accept" — the SA
//! could and did sell stacked geometry for wirelength.
//!
//! ponytail: Φ-monotone acceptance has a known ceiling. PLAN §3a pairs it with
//! *neighbourhood escalation* — when no single move lowers Φ, rip up a whole group —
//! and there is no escalation here, so a device that would have to cross a neighbour
//! to reach a better basin cannot get there by displacement. The swap move is the
//! only escape hatch today, and `legalize::separate_overlaps` is what still
//! guarantees the terminal legality. Upgrade path is the group rip-up, or the
//! sequence-pair representation that makes the crossing unrepresentable.
//!
//! Reuses `gp::mechanics` for the shared SA primitives (RNG, HPWL from macro
//! pins, overlap, objective/legality wiring, grid snap).

pub mod legalize;

/// Relaxation sweeps the terminal legalizer gets to clear accidental overlap.
/// It exits early once clean or stalled, so this is only a runaway guard.
const LEGALIZE_SWEEPS: u32 = 64;

use analog::Requirements;
use pnr_core::geom::LayerId;
use pnr_core::{Layout, Macro, Orient, Report};

use gp::mechanics::{
    analog_cost, analog_phi, analog_theta, analog_violations, choose_variants, hpwl,
    overlap_density, overlap_incident, report, snap, variant_extents, Nets, SplitMix64,
};

/// A detailed-placement algorithm.
pub trait DetailedPlacer {
    /// Refine `coarse` into a legal placement, deterministically for `seed`.
    ///
    /// `layers` is the PDK-permitted metal stack; detailed placement is
    /// layer-agnostic, so it is threaded for a uniform stage contract but unused.
    /// `fixed[i] == true` **pins** device `i` — a user-injected macro, kept at its
    /// coarse position while the auto-generated cells legalise around it.
    ///
    /// `macros[i]` is cell `i`'s drawn geometry, and it is here for its **pins**:
    /// connectivity is built from them ([`gp::mechanics::Nets`]), so this is what makes
    /// the HPWL term — and therefore the reshape move — score wirelength at all. A
    /// `Layout` alone carries centres and extents but no terminals, which is why this
    /// parameter exists rather than being derivable from `coarse`. Only cells with no
    /// enumerated alternative are read from here directly; everything else is taken from
    /// `variants[i].alternatives[coarse.variant[i]]`, so the measured geometry is always
    /// the one `variant` names.
    ///
    /// `variants[i]` are cell `i`'s pre-drawn alternatives, and **this stage is where
    /// they are searched.** A reshape move swaps `Layout::variant[i]` for another
    /// index and takes the new footprint's `hw`/`hh` in the same breath. This is
    /// `dp`'s job rather than `gp`'s because a variant change relocates pins, which
    /// changes routability and extracted parasitics *discontinuously* — there is no
    /// gradient to descend and no valid continuous relaxation, so it can only be a
    /// discrete move under an acceptance criterion (PLAN §2).
    ///
    /// `variants[i].lock` locks a matched set to one **shared** variant index; a
    /// reshape that draws a locked cell reshapes every member of its lock as one move.
    ///
    /// `prices` carries the augmented-Lagrangian state across epochs; see
    /// [`gp::Prices`]. Determinism is over `(inputs, seed, prices)`.
    fn place(
        &self,
        coarse: &Layout,
        macros: &[Macro],
        variants: &[gp::VariantSpace],
        reqs: &Requirements<Layout>,
        layers: &[LayerId],
        fixed: &[bool],
        prices: &mut gp::Prices,
        seed: u64,
    ) -> (Layout, Report);
}

/// Tunables for the legalising SA (defaults ported from `engine::DetailedCfg`
/// + `RefinementCfg`, docs "Stage 2/3").
#[derive(Clone, Debug)]
pub struct DetailedCfg {
    pub max_iters: u32,
    pub min_iters: u32,
    /// Inner moves per epoch = `moves_per_cell · n`.
    pub moves_per_cell: u32,
    /// Geometric cooling factor (`temp ← temp·alpha`).
    pub alpha: f64,
    /// Initial displacement window as a fraction of the die span.
    pub range0: f32,
    pub range_decay: f32,
    pub min_accept_rate: f32,
    pub grid: i32,
    /// Minimum edge-to-edge clearance between devices of different groups, nm.
    ///
    /// Bounding boxes only bound *drawn* geometry; the PDK still demands real
    /// spacing between the layers inside them — an nwell-to-nwell rule is on the
    /// order of a micron. Separating by a single grid step leaves neighbouring
    /// wells and diffusions close enough to merge into one polygon, which both
    /// showers the run with `min_spacing:nwell` violations and defeats device
    /// extraction: a channel polygon spanning two device types has no single
    /// matching implant, so LVS aborts.
    ///
    /// Set from the PDK by the caller; `0` reproduces the old touch-and-go
    /// behaviour.
    pub clearance_nm: i32,
}

impl Default for DetailedCfg {
    fn default() -> Self {
        Self {
            max_iters: 220,
            min_iters: 20,
            moves_per_cell: 60,
            alpha: 0.93,
            range0: 0.4,
            range_decay: 0.96,
            min_accept_rate: 0.02,
            grid: 5,
            // sky130's nwell-to-nwell rule is the binding inter-device spacing;
            // a caller holding the deck should override this from it.
            //
            // Measured effect: with this at 0, neighbouring devices' wells merge
            // into a single polygon spanning both device types, and extraction
            // aborts with "channel polygon has no matching MOS type implant" —
            // LVS can never pass. At 1300 nm extraction succeeds and LVS reaches
            // an actual comparison. It also surfaces `floating_well` ERC errors,
            // which are *honest*: the old merged super-well happened to touch one
            // tap, hiding the fact that each PMOS needs its own well tie.
            clearance_nm: 2000,
        }
    }
}

/// The real detailed placer: legalising Metropolis SA. Swap in at
/// `frontend/library`.
#[derive(Default)]
pub struct Annealer {
    pub cfg: DetailedCfg,
}

/// Bounded-move SA state kept between epochs.
///
/// `nets` is **owned**, not borrowed, because a reshape rewrites the pin geometry of
/// the cell it swaps: net *membership* is variant-invariant (an alternative is drawn
/// for the same terminals, so it lands on the same nets) and is built once, while pin
/// *offsets* are patched per accepted move and patched back per rejected one. See
/// [`Nets`] for the measured numbers behind that split.
struct Sa<'a> {
    nets: Nets,
    reqs: &'a Requirements<Layout>,
    /// The carried λ, read (never written) by the objective — see [`gp::Prices`].
    /// Immutable for the whole anneal on purpose: a price that moved mid-anneal
    /// would make the accept/reject decisions of epoch 1 and epoch 40
    /// incomparable, and Metropolis is only a valid sampler on a fixed energy.
    prices: &'a gp::Prices,
    cell_nets: Vec<Vec<u32>>,
}

impl<'a> Sa<'a> {
    /// Incremental HPWL delta for moving device `c` from its current centre to
    /// `(nx, ny)`: re-cost only the nets `c` touches (engine `SaCost::delta`).
    ///
    /// Terminals are pins, not centres ([`Nets::pin`]), so `c`'s own pin offset rides
    /// along with the centre — a displacement translates it, which is why the offset
    /// can be read from the table on both sides here.
    fn hpwl_delta(&self, l: &Layout, c: usize, nx: i32, ny: i32) -> f64 {
        let mut d = 0.0f64;
        for &ni in &self.cell_nets[c] {
            let (mut ox0, mut ox1, mut oy0, mut oy1) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
            let (mut x0, mut x1, mut y0, mut y1) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
            for k in self.nets.span(ni as usize) {
                let (opx, opy) = self.nets.pin(k, l);
                ox0 = ox0.min(opx);
                ox1 = ox1.max(opx);
                oy0 = oy0.min(opy);
                oy1 = oy1.max(opy);
                // The moved cell's terminal translates with its centre; every other
                // one stays where the table says.
                let (px, py) = if self.nets.dev(k) == c {
                    (opx - l.x[c] + nx, opy - l.y[c] + ny)
                } else {
                    (opx, opy)
                };
                x0 = x0.min(px);
                x1 = x1.max(px);
                y0 = y0.min(py);
                y1 = y1.max(py);
            }
            d += f64::from((x1 - x0) + (y1 - y0) - (ox1 - ox0) - (oy1 - oy0));
        }
        d
    }
}

impl DetailedPlacer for Annealer {
    fn place(
        &self,
        coarse: &Layout,
        macros: &[Macro],
        variants: &[gp::VariantSpace],
        reqs: &Requirements<Layout>,
        _layers: &[LayerId],
        fixed: &[bool],
        prices: &mut gp::Prices,
        seed: u64,
    ) -> (Layout, Report) {
        // Deliberately NOT implemented here: the disjunctive branch move (`DtiBand`
        // share-vs-isolate). `Layout::branch` now carries the per-pair Boolean and is
        // cloned and returned below, so the *state* exists — but no batch says which
        // `BranchId`s it owns (`RuleBatch` is the only `dyn` seam and `Rule::touches`
        // names device ids, not branch ids), so a flip here would be a guess at an index
        // and would price nothing. Every pair therefore ships on its `false` starting
        // commitment. Logged in `docs/API-WISH.md`; a half-typed workaround that flipped
        // arbitrary bits would be worse than the honest gap.

        // Detailed placement is layer-agnostic (see the trait doc): bounded-move SA
        // over HPWL + analog cost, neither of which names a routing layer.
        let cfg = &self.cfg;
        let n = coarse.x.len();
        let mut rng = SplitMix64::new(seed);

        // Own the layout we optimise (clone the coarse SoA + axis/groups).
        let mut l = Layout {
            x: coarse.x.clone(),
            y: coarse.y.clone(),
            hw: coarse.hw.clone(),
            hh: coarse.hh.clone(),
            variant: coarse.variant.clone(),
            axis: coarse.axis.clone(),
            branch: coarse.branch.clone(),
            groups: coarse.groups.clone(),
            orient: coarse.orient.clone(),
            power_uw: coarse.power_uw.clone(),
            temp_mc: coarse.temp_mc.clone(),
        };
        // Temperatures are stale the moment devices move, and every hard/headroom
        // check on a thermal rule reads them. Seed them for the coarse positions;
        // the epoch loop refreshes as the placement changes.
        l.refresh_temps();

        // Carried λ/ρ bound to this epoch's batch order before anything reads the
        // objective (D9). The dual step is the single `settle` after the anneal.
        prices.bind(reqs);

        // Connectivity, from the geometry `l.variant` **currently** names — not from
        // `macros` directly. `choose_variants` is the one shared resolver (`gp` seeds
        // the assignment through it), so a caller that hands us the macros of one
        // assignment and a `Layout` carrying another cannot make HPWL score geometry
        // the layout does not have; `macros[i]` is only reached for a cell with no
        // enumerated alternative.
        //
        // This is what the old `Nets::from_macros(&[])` could not do. With no macros
        // the table was empty, `hpwl` returned `0.0` for every layout, and detailed
        // placement had **no wirelength signal at all** — only overlap and analog cost.
        // Worse for the move this stage exists to make: a reshape was priced on the new
        // footprint's bbox alone, which is precisely the consequence PLAN §2 says is
        // *not* the interesting one. Pin relocation is.
        let drawn = choose_variants(macros, variants, &l.variant);
        let nets = Nets::from_macros(&drawn);

        if n == 0 {
            let rep = report(&nets, reqs, &l, prices);
            return (l, rep);
        }

        // Die span from the coarse footprint (keeps moves and clamps on-die).
        let (mut xmin, mut ymin, mut xmax, mut ymax) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for i in 0..n {
            xmin = xmin.min(l.x[i] - l.hw[i]);
            ymin = ymin.min(l.y[i] - l.hh[i]);
            xmax = xmax.max(l.x[i] + l.hw[i]);
            ymax = ymax.max(l.y[i] + l.hh[i]);
        }
        let span = (xmax - xmin).max(ymax - ymin).max(1) as f32;

        // The ramped overlap-density weight that used to live here is gone: overlap is a
        // hard violation (`mechanics::report` says so), so it is gated in the V tier of
        // `gate_key` instead of priced in the objective. See the module docs — a finite
        // price on a hard rule is a bribe, and the ramp was the schedule on which the SA
        // was willing to take it.
        let mut sa = Sa {
            cell_nets: nets.cell_nets(n),
            nets,
            reqs,
            prices,
        };
        // clamp windows are absolute coords; keep footprints within the coarse
        // bbox by clamping to [min+half, max-half].
        let clamp_x = |c: i32, half: i32| c.clamp(xmin + half, (xmax - half).max(xmin + half));
        let clamp_y = |c: i32, half: i32| c.clamp(ymin + half, (ymax - half).max(ymin + half));

        // ---- initial temperature: 0.02 · mean |Δ| over probe moves ----------
        // (engine run_detailed: SA starts from the GLOBAL solution, so a low t0
        //  refines rather than random-walking the input away).
        let mut range = cfg.range0;
        let probe_r = (range * span) as i32;
        let mut sum = 0.0f64;
        let mut cnt = 0u32;
        for _ in 0..128 {
            let c = rng.below(n);
            let nx = clamp_x(l.x[c] + rng.centered(probe_r as f32) as i32, l.hw[c]);
            let ny = clamp_y(l.y[c] + rng.centered(probe_r as f32) as i32, l.hh[c]);
            let d = sa.hpwl_delta(&l, c, nx, ny) + delta_analog(reqs, sa.prices, &mut l, c, nx, ny);
            sum += d.abs();
            cnt += 1;
        }
        let mut temp = (sum / f64::from(cnt.max(1))).max(1.0) * 0.02;

        // ---- SA epochs -------------------------------------------------------
        // A caller that built its `Layout` by hand may have left `orient` empty;
        // rotation is then simply not part of the move set for that run.
        let can_rotate = l.orient.len() == n;
        // Same shape for reshape: no `VariantSpace` (or a `variant` table that is not
        // device-length, which `Layout::debug_check` would already have caught in a
        // dev build) means the variant search is simply not in the move set. A caller
        // that passes `variants: &[]` therefore gets byte-identical behaviour to a
        // build without the move, which is what keeps the pre-variant tests honest.
        let can_reshape = variants.len() == n && l.variant.len() == n;
        // Who reshapes with whom. `mates[c]` is `c` plus every cell sharing its
        // `VariantSpace::lock`, sorted and deduplicated; an unlocked cell is `[c]`.
        //
        // Hoisted out of the move loop because it is a pure function of `variants`,
        // which no move changes — and it is `O(n²)` in the worst case, which the move
        // loop cannot afford ~5.7M times.
        let mates: Vec<Vec<usize>> = (0..n)
            .map(|c| match variants.get(c).and_then(|v| v.lock) {
                Some(id) => (0..n)
                    .filter(|&o| variants[o].lock == Some(id))
                    .collect(),
                None => vec![c],
            })
            .collect();
        let moves_per_epoch = (cfg.moves_per_cell as usize * n).max(1);
        let range_min = cfg.grid.max(1) as f32 / span.max(1.0);

        for iter in 0..cfg.max_iters {
            let r = (range * span) as i32;
            let mut proposed = 0u32;
            let mut accepted = 0u32;
            for _ in 0..moves_per_epoch {
                proposed += 1;
                // Move generator: 70% bounded displacement, else swap with
                // another device (engine SymSaCore ungrouped path, minus the
                // variant/orientation ops the new Layout has no field for).
                let roll = rng.f32();
                let c = rng.below(n);
                // Injected macros are pinned: never propose a move that displaces one.
                if fixed.get(c).copied().unwrap_or(false) {
                    continue;
                }
                if roll < 0.70 {
                    let nx = clamp_x(l.x[c] + rng.centered(r as f32) as i32, l.hw[c]);
                    let ny = clamp_y(l.y[c] + rng.centered(r as f32) as i32, l.hh[c]);
                    if try_move(&sa, &mut l, &mut rng, temp, c, nx, ny) {
                        accepted += 1;
                    }
                } else if roll < 0.90 {
                    let o = rng.below(n);
                    if o != c
                        && !fixed.get(o).copied().unwrap_or(false)
                        && try_swap(&sa, &mut l, &mut rng, temp, c, o, &clamp_x, &clamp_y)
                    {
                        accepted += 1;
                    }
                } else if can_reshape && roll >= 0.95 {
                    // The reshape move gets the top half of what was the rotate slice
                    // (10% → 5% each) rather than its own carve-out of the earlier
                    // thresholds, so a run with no `VariantSpace` consumes exactly the
                    // RNG stream it always did.
                    if try_reshape(
                        &mut sa,
                        &mut l,
                        &mut rng,
                        temp,
                        c,
                        &mates[c],
                        variants,
                        fixed,
                        &clamp_x,
                        &clamp_y,
                    ) {
                        accepted += 1;
                    }
                } else if can_rotate
                    && rotatable(&l, c)
                    && try_rotate(&sa, &mut l, &mut rng, temp, c, &clamp_x, &clamp_y)
                {
                    accepted += 1;
                }
            }

            // ---- exact projection, once per epoch ---------------------------
            // This is the feasibility-pump alternation of PLAN §3a made literal:
            // the move loop above is one *relaxation* sweep (Metropolis over the
            // priced objective), and this is the one *projection* that follows it.
            // Per epoch, never per move — a per-move projection would need a
            // batch→cell reverse map to know what to re-project, would invalidate
            // the Metropolis comparison mid-epoch (the energy landscape would
            // shift under the sampler), and costs O(moves × batches) for repairs
            // the next move can undo anyway.
            //
            // Consequence worth naming: the `open == 0` early-stop below becomes
            // *reachable* for symmetric circuits for the first time. An exact
            // equality can never anneal to satisfied — the SA drives the mirror
            // residual small and stops, so before this call `Symmetry::satisfied`
            // stayed false until the terminal projection, and every symmetric
            // design paid for all `max_iters` epochs.
            project_hard(reqs, &mut l, fixed, cfg.grid);

            // Refresh the thermal field once per epoch, never per move: ΔT is a
            // global property of the whole power distribution, so a trial move
            // cannot be scored against it cheaply (backend/TODO.md §1). Epoch
            // cadence is what makes `ThermalGradient::satisfied` a live function
            // of placement rather than a constant.
            l.refresh_temps();

            // cooling + range shrink (engine Geometric).
            temp *= cfg.alpha;
            range = (range * cfg.range_decay).max(range_min);

            let accept_rate = accepted as f32 / proposed.max(1) as f32;
            let od = overlap_density(&l);
            let open = analog_violations(reqs, &l);
            // Stop after min_iters when frozen, overlap gone, and hard clean
            // (engine SaStop / docs "Stage 2 stop").
            if iter + 1 >= cfg.min_iters
                && accept_rate < cfg.min_accept_rate
                && od <= 1e-4
                && open == 0
            {
                break;
            }
        }

        // ---- final grid snap -------------------------------------------------
        // Snap axes then centres to grid (docs "Grid snap and symmetry
        // restoration").
        for a in &mut l.axis {
            *a = snap(*a, cfg.grid);
        }
        for i in 0..n {
            l.x[i] = snap(l.x[i], cfg.grid);
            l.y[i] = snap(l.y[i], cfg.grid);
        }

        // ---- terminal legalization + exact projection ------------------------
        // Two closing steps that pull against each other, hence this order:
        //
        //  1. legalize — SA returns its *best-seen* state, so if the budget ran
        //     out with devices on top of each other that overlap would ship. Push
        //     the accidental (cross-group) overlaps apart; deliberate abutment
        //     inside a group is preserved (see `legalize`).
        //  2. project — equality constraints cannot be closed by penalty:
        //     annealing drives the mirror residual small and stops, so
        //     `Symmetry::satisfied` stays false forever. (The old note here
        //     claimed grid snap "preserves an already-satisfied mirror equation"
        //     — true, but nothing ever established one, so symmetry was never
        //     satisfied at all.) Each hard rule is asked to project itself onto
        //     its own feasible set; most decline, an inequality already having a
        //     usable gradient.
        //  3. legalize again — projection nudges devices and can re-introduce a
        //     little overlap. This pass is guarded on hard violations, so it only
        //     clears overlap it can clear *without* breaking the symmetry just
        //     established.
        //
        // Legalizing first and projecting second is deliberate: projection moves
        // a device by at most a grid step or two, while legalization can move it
        // far, so the small step goes last and survives.
        legalize::separate_overlaps(&mut l, reqs, fixed, cfg.grid, cfg.clearance_nm, LEGALIZE_SWEEPS);
        l.refresh_temps();

        // The epoch loop already projected, but `separate_overlaps` just moved
        // devices again and can break the symmetry it established, so the last
        // word on the equalities has to come after the last legalization pass.
        project_hard(reqs, &mut l, fixed, cfg.grid);

        legalize::separate_overlaps(&mut l, reqs, fixed, cfg.grid, cfg.clearance_nm, LEGALIZE_SWEEPS);
        l.refresh_temps();

        // Take the net table back out of the SA state (and drop the SA's read-only
        // borrow of `prices` with it, so the dual step below can take `&mut`).
        let Sa { nets, .. } = sa;
        // The dual step, once, at the primal optimum — after legalization and
        // projection, so the residual λ is priced on is the one the caller is actually
        // handed. See `gp::Prices::settle` for why this is per-`place` and not
        // per-epoch.
        prices.settle(reqs, &l);
        let rep = report(&nets, reqs, &l, prices);
        (l, rep)
    }
}

/// One **exact-projection sweep** over `reqs.hard` — the projection half of the
/// PLAN §3a feasibility pump, shared by the per-epoch call and the terminal
/// legalize→project→legalize sandwich.
///
/// Each *violated* hard batch is asked to project itself onto its own feasible
/// set; a satisfied batch is skipped outright, because `project` is not a no-op
/// on one — `SymmetryGroup::project` re-averages the shared axis from its pairs'
/// midpoints, so projecting a stage nothing violated would still slide the axis
/// (and every partner with it) between epochs for no repair at all.
///
/// Pinned cells are restored **before** the Φ re-check, deliberately: projection
/// cannot know a partner is a user-injected macro, so a mirror it establishes by
/// moving one is really a half-mirrored pair once the pin is honoured — and
/// measuring Φ *after* the restore is what makes that show up as the violation
/// it still is, rolling the whole sweep back instead of shipping it.
///
/// The rollback gate is **Φ-monotone**: the same `(violating batches, Σ residual)`
/// tuple [`accept`] compares, not the bare violation count the old terminal block
/// used. A projection that closes one batch's equality while worsening another's
/// residual at equal count is a Φ regression, and it must not survive here when
/// the acceptance gate would have refused it as a move.
///
/// Returns whether the projected coordinates were kept. On keep the thermal
/// field is refreshed — devices moved, and the hard checks that read derived
/// state (`ThermalGradient`) would otherwise score temperatures of positions the
/// devices no longer hold. On rollback the coordinates return to ones the
/// current field was computed for, so nothing needs recomputing. Consumes no
/// RNG: a run whose hard batches are all satisfied (or empty) is byte-identical
/// to one without this call.
fn project_hard(reqs: &Requirements<Layout>, l: &mut Layout, fixed: &[bool], grid: i32) -> bool {
    let before = analog_phi(reqs, l);
    // No violated batch ⇒ nothing would project ⇒ nothing to snapshot.
    if before.0 == 0 {
        return false;
    }
    let (px, py, paxis) = (l.x.clone(), l.y.clone(), l.axis.clone());
    for batch in &reqs.hard {
        // Re-checked per batch, not hoisted: an earlier batch's projection can
        // satisfy a later one, and the skip is exactly what keeps a satisfied
        // axis from being re-averaged.
        if batch.violations(l) > 0 {
            batch.project(l, grid);
        }
    }
    // Injected macros are pinned by contract; projection cannot know that.
    for i in 0..l.x.len() {
        if fixed.get(i).copied().unwrap_or(false) {
            l.x[i] = px[i];
            l.y[i] = py[i];
        }
    }
    if analog_phi(reqs, l) > before {
        l.x = px;
        l.y = py;
        l.axis = paxis;
        return false;
    }
    l.refresh_temps();
    true
}

/// Analog-cost delta for tentatively moving `c` to `(nx, ny)`: apply, measure
/// `Σ reqs.cost`, restore. Analog-blind (never names a term). `l` is `&mut` so
/// the probe mutates in place without cloning the whole SoA.
fn delta_analog(
    reqs: &Requirements<Layout>,
    prices: &gp::Prices,
    l: &mut Layout,
    c: usize,
    nx: i32,
    ny: i32,
) -> f64 {
    let before = analog_cost(reqs, l, prices);
    let (ox, oy) = (l.x[c], l.y[c]);
    l.x[c] = nx;
    l.y[c] = ny;
    let after = analog_cost(reqs, l, prices);
    l.x[c] = ox;
    l.y[c] = oy;
    f64::from(after - before)
}

/// The **lexicographic gate key** for a trial move: `(|V|, Φ margin, Θ residual)`,
/// restricted to what a move repositioning exactly `moved` can change.
///
/// The first two elements are `Report::phi()` — the repair measure of PLAN §3a — and
/// the third is `Report::lex()`'s middle tier. They are one tuple here rather than two
/// calls because [`accept`] compares them in exactly this order and nothing else reads
/// either half.
///
/// Every element is what `mechanics::report` would write, measured the same way:
/// `analog_phi` and `analog_theta` are shared with it so the per-move gate and the
/// returned `Report` cannot disagree, plus device overlap area, which `report` counts as
/// a hard violation.
///
/// The overlap term is `overlap_incident`, not `total_overlap`: pairs with no member in
/// `moved` are identical either side of the move and cancel in the comparison, so
/// restricting the scan is exact. Same argument `pex_cost`'s predecessor rested on, and
/// it is what makes a per-move gate affordable — at the ~430 devices of a MAGICAL block
/// the full scan is ~92k pair evaluations, and there are ~5.7M trial moves.
///
/// `report` also adds a *presence* entry to `|V|` whenever any overlap remains. That bit
/// is global and identical either side of every single move except the one that clears
/// the design's last overlap — and that move already lowers the margin element, so it is
/// accepted regardless. Folding the bit in would instead give it a way to bypass the
/// margin tier, so it is left out.
fn gate_key(sa: &Sa, l: &Layout, moved: &[usize]) -> (usize, f64, f64) {
    let (batches, margin) = analog_phi(sa.reqs, l);
    (batches, margin + overlap_incident(l, moved), analog_theta(sa.reqs, l))
}

/// The **lexicographic acceptance test** — D10 / PLAN §3b — replacing "collapse
/// everything into one scalar and take a Metropolis draw".
///
/// `lex-min (V, Θ, PEX)` with Φ's margin spliced into the V tier, compared as a tuple:
///
/// - **Φ must be non-increasing.** No amount of PEX improvement buys a hard violation,
///   which is the whole point — "a penalty that is finite is a bribe the optimizer will
///   accept". A move that strictly *lowers* Φ is taken unconditionally, even if it costs
///   PEX: that is the feasibility-pump discipline of PLAN §3a, and accepting only
///   Φ-non-increasing repairs is what makes the repair-dependency graph acyclic and
///   stops "fix A, break B" cycling.
/// - **Then Θ**, the summed budget residual. Above PEX because a budget is a constraint
///   that must hold, not a competing objective (PLAN §3b rejects the Pareto framing
///   above the feasibility frontier), and *below* V because a budget with a residual is
///   still being paid for rather than illegal.
/// - **Then PEX**, and only there does Metropolis get a vote. Uphill moves still explore
///   parasitics and can never spend legality or margin, because they are consulted only
///   when both tiers above are unchanged.
///
/// Note the asymmetry this creates and why it is correct: Θ is *gated*, and it is also
/// *priced* in PEX through `analog_cost`'s `−λᵀc` term. The gate stops a budget being
/// traded away; the price is what pulls a violated one back toward zero, since two
/// layouts with the same Θ are still separated by the priced term.
#[inline]
fn accept(
    before: (usize, f64, f64),
    after: (usize, f64, f64),
    d_pex: f64,
    temp: f64,
    rng: &mut SplitMix64,
) -> bool {
    if after != before {
        return after < before;
    }
    metropolis(d_pex, temp, rng)
}

/// Propose, Φ-gate, Metropolis-accept, commit one displacement.
fn try_move(
    sa: &Sa,
    l: &mut Layout,
    rng: &mut SplitMix64,
    temp: f64,
    c: usize,
    nx: i32,
    ny: i32,
) -> bool {
    let before_key = gate_key(sa, l, &[c]);
    // PEX only: overlap is Φ's business now, so the objective is HPWL + analog.
    let d = sa.hpwl_delta(l, c, nx, ny) + delta_analog(sa.reqs, sa.prices, l, c, nx, ny);
    let (ox, oy) = (l.x[c], l.y[c]);
    l.x[c] = nx;
    l.y[c] = ny;
    if accept(before_key, gate_key(sa, l, &[c]), d, temp, rng) {
        true
    } else {
        l.x[c] = ox;
        l.y[c] = oy;
        false
    }
}

/// Propose, Φ-gate, accept, commit a position swap of `c` and `o`.
fn try_swap(
    sa: &Sa,
    l: &mut Layout,
    rng: &mut SplitMix64,
    temp: f64,
    c: usize,
    o: usize,
    clamp_x: &impl Fn(i32, i32) -> i32,
    clamp_y: &impl Fn(i32, i32) -> i32,
) -> bool {
    // swap centres, each clamped to keep its own footprint on-die.
    let cx = clamp_x(l.x[o], l.hw[c]);
    let cy = clamp_y(l.y[o], l.hh[c]);
    let ox = clamp_x(l.x[c], l.hw[o]);
    let oy = clamp_y(l.y[c], l.hh[o]);

    // exact delta: apply, evaluate, revert on reject.
    let before_key = gate_key(sa, l, &[c, o]);
    let before_cost = pex_cost(sa, l);
    let (scx, scy, sox, soy) = (l.x[c], l.y[c], l.x[o], l.y[o]);
    l.x[c] = cx;
    l.y[c] = cy;
    l.x[o] = ox;
    l.y[o] = oy;
    let after_key = gate_key(sa, l, &[c, o]);
    if accept(before_key, after_key, pex_cost(sa, l) - before_cost, temp, rng) {
        true
    } else {
        l.x[c] = scx;
        l.y[c] = scy;
        l.x[o] = sox;
        l.y[o] = soy;
        false
    }
}

/// One 90° step **within the device's current reflection class** — `R0→R90→R180
/// →R270`, and the mirrored coset likewise.
///
/// The move set never *introduces* a mirror. A reflection inverts a
/// non-self-aligned (drain-extended) device's misalignment sensitivity, so
/// matched copies of one must be superimposable by translation alone
/// [AOAL ch13 13.2.1], and nothing in `Requirements` can express that today.
/// Mirrors stay reachable only if a caller seeds one.
fn quarter_turn(o: Orient) -> Orient {
    match o {
        Orient::R0 => Orient::R90,
        Orient::R90 => Orient::R180,
        Orient::R180 => Orient::R270,
        Orient::R270 => Orient::R0,
        Orient::Mx => Orient::Mx90,
        Orient::Mx90 => Orient::Mx180,
        Orient::Mx180 => Orient::Mx270,
        Orient::Mx270 => Orient::Mx,
    }
}

/// May device `c` be turned at all?
///
/// A device that shares a group is part of a matched structure, and matched
/// channels must all run parallel — orientation mismatch is the single largest
/// systematic error (up to ~15 % gm) [AOAL ch13 Rule 7]. A directional pocket
/// implant forbids the four 90° members outright regardless of matching
/// [AOAL ch12 12.2.7]. Neither fact reaches `dp`: `Requirements` has no
/// orientation rule, so the SA objective is blind to a turn that wrecks a pair.
///
/// Until that rule exists this is the conservative floor — matched devices do not
/// turn — not the final legality model. The real one is the per-transform table in
/// `docs/cells/mosfet.md` §4.1, and it needs the device's construction kind
/// (directional implant? self-aligned?) which the `Layout` does not carry.
fn rotatable(l: &Layout, c: usize) -> bool {
    !l.groups
        .iter()
        .any(|g| g.len() > 1 && g.iter().any(|d| d.0 as usize == c))
}

/// Propose, Φ-gate, accept, commit a 90° turn of `c`.
///
/// A turn moves no centre, and the main win is **aspect ratio**: a tall cell turned
/// wide settles into a channel no displacement move can reach. It is no longer HPWL-
/// neutral, though — terminals are pins now, and [`Nets::pin`] turns each offset by the
/// device's `orient`, so a turn that swings a gate contact to the other face is priced
/// like the wirelength change it is. `hw`/`hh` swap with the orientation — the invariant
/// [`Layout::orient`] documents — so every existing bbox/overlap read stays
/// correct without knowing a rotation happened, and [`gate_key`] prices the new
/// footprint's overlap for free.
fn try_rotate(
    sa: &Sa,
    l: &mut Layout,
    rng: &mut SplitMix64,
    temp: f64,
    c: usize,
    clamp_x: &impl Fn(i32, i32) -> i32,
    clamp_y: &impl Fn(i32, i32) -> i32,
) -> bool {
    let before_key = gate_key(sa, l, &[c]);
    let before_cost = pex_cost(sa, l);
    let saved = (l.orient[c], l.hw[c], l.hh[c], l.x[c], l.y[c]);

    l.orient[c] = quarter_turn(saved.0);
    l.hw[c] = saved.2;
    l.hh[c] = saved.1;
    // The turned footprint has the extents transposed, so a centre that was
    // on-die before the turn can hang off it after.
    l.x[c] = clamp_x(saved.3, l.hw[c]);
    l.y[c] = clamp_y(saved.4, l.hh[c]);

    if accept(before_key, gate_key(sa, l, &[c]), pex_cost(sa, l) - before_cost, temp, rng) {
        true
    } else {
        (l.orient[c], l.hw[c], l.hh[c], l.x[c], l.y[c]) = saved;
        false
    }
}

/// Propose, Φ-gate, accept, commit a **variant swap** — the reshape move, and the
/// mechanism that makes variant choice a search variable rather than a decision
/// frozen before placement (PLAN §2, D7/D8).
///
/// `l.variant[c]` and `l.hw[c]`/`l.hh[c]` are written **in the same breath**, from the
/// chosen alternative's bbox via the one shared `variant_extents` — the
/// [`Layout::variant`] invariant, and the same rule `orient` obeys two functions up.
/// Get it wrong and every bbox, overlap and spacing read downstream silently scores
/// the previous variant's footprint.
///
/// **Priced on where the pins land**, which is the whole point (PLAN §2). The chosen
/// alternative's pin offsets are written into `Sa::nets` before the objective is
/// evaluated, so two alternatives with the *same bbox* and different pin faces score
/// differently — under the old empty `Nets` they scored identically, and the move was
/// optimising a projection of its own objective. Only the offsets are rewritten: net
/// membership cannot change, because every alternative of a cell is drawn for the same
/// terminals (see [`Nets`]).
///
/// **Joint over a matched lock.** `group` is the set of cells that must hold the *same*
/// variant index — `VariantSpace::lock`, filled from
/// `Unitization::same_variant_required`. Every member takes the new index, and the whole
/// set is accepted or reverted as one move. The alternative — refusing to reshape a
/// locked cell — was rejected deliberately: it disables the feature exactly where
/// matching matters most, whereas the joint move *is* the same-variant case's correct
/// behaviour and is a step toward the real group collapse rather than a stub to delete.
/// Reshaping one half of a differential pair is invisible to DRC and to LVS, so nothing
/// downstream would ever report it.
///
/// Two things that are easy to get wrong, both load-bearing:
///
/// - **Variant index distance is meaningless.** Enumeration order is
///   `cells::Cell::enumerate`'s order after group collapse, which is not a similarity
///   ordering — `variant ± 1` can be a different metal-stack topology with different
///   pin faces (D7). So the new index is drawn **uniformly** from the other
///   alternatives, not stepped. The draw is `below(len - 1)` with a skip over the
///   current index rather than a rejection loop, so it costs exactly one RNG word and
///   keeps the trajectory reproducible for a seed.
/// - **Reshape is not a small move.** A footprint change can open or close overlaps
///   with every neighbour at once. The incremental overlap path stays valid — only
///   cell `c` changed — but only because [`gate_key`] is evaluated *after* the
///   extents are written, so it measures the NEW footprint. Any before/after delta
///   computed from `l.hw[c]` while the old extents were still in place would be
///   scoring a footprint the layout does not have.
///
/// A `fixed[c]` cell never reaches here: the move generator skips pinned cells
/// outright, because a user-injected macro's geometry is the user's and reshaping it
/// would silently substitute a different drawn cell for the one they asked for. A
/// *locked mate* that is pinned refuses the whole joint move for the same reason — the
/// alternative would be reshaping the free members and letting them drift away from the
/// pinned one, which is the matching break this move set exists to avoid.
#[allow(clippy::too_many_arguments)]
fn try_reshape(
    sa: &mut Sa,
    l: &mut Layout,
    rng: &mut SplitMix64,
    temp: f64,
    c: usize,
    group: &[usize],
    variants: &[gp::VariantSpace],
    fixed: &[bool],
    clamp_x: &impl Fn(i32, i32) -> i32,
    clamp_y: &impl Fn(i32, i32) -> i32,
) -> bool {
    // The joint space is only as deep as its shallowest member: `VariantSpace::lock`'s
    // contract says locked cells are index-compatible, but a producer that got that
    // wrong must not be able to make this index out of range. `min` over the group is
    // one line and turns a panic into a smaller search space.
    let depth = group
        .iter()
        .map(|&m| variants[m].alternatives.len())
        .min()
        .unwrap_or(0);
    // A cell with 0 or 1 drawn alternatives has no variant space to search. The
    // collapse of PLAN §2 leaves plenty of these — a group pinned to one legal joint
    // assignment is exactly what the same-variant and integer-ratio equalities are
    // *for* — so this is the common case, not an error.
    if depth < 2 {
        return false;
    }
    // A pinned mate vetoes the move; see the doc comment.
    if group.iter().any(|&m| fixed.get(m).copied().unwrap_or(false)) {
        return false;
    }
    let cur = l.variant[c] as usize;
    // Uniform over the alternatives other than the current one.
    let draw = rng.below(depth - 1);
    let next = if cur < depth && draw >= cur { draw + 1 } else { draw };

    let before_key = gate_key(sa, l, group);
    let before_cost = pex_cost(sa, l);
    let saved: Vec<(u16, i32, i32, i32, i32)> =
        group.iter().map(|&m| (l.variant[m], l.hw[m], l.hh[m], l.x[m], l.y[m])).collect();

    for &m in group {
        l.variant[m] = next as u16;
        // The two extent invariants **compose**: `hw`/`hh` are the chosen variant's bbox
        // *after* the current orientation's transform, because `Layout::orient` says the
        // extents it documents are post-transform. Writing the raw bbox here would leave
        // `orient` claiming a 90° turn that the extents no longer reflect — and the
        // single stamping site (`frontend/library::geometry`) still applies the turn, so
        // the drawn geometry would be transposed while every overlap and spacing read
        // scored it untransposed. `orient` may be shorter than the device table (see
        // `can_rotate`), in which case there is no turn to compose with.
        let alt = &variants[m].alternatives[next];
        let (w, h) = variant_extents(alt);
        (l.hw[m], l.hh[m]) = match l.orient.get(m) {
            Some(o) if o.swaps_axes() => (h, w),
            _ => (w, h),
        };
        // A wider or taller footprint can hang the centre off the die, exactly as a turn
        // can.
        l.x[m] = clamp_x(l.x[m], l.hw[m]);
        l.y[m] = clamp_y(l.y[m], l.hh[m]);
        // The pins moved with the footprint — the consequence PLAN §2 says *is* the
        // interesting one. Patch this cell's rows so the objective below prices the new
        // terminals and not the old ones.
        sa.nets.reshape_cell(m, &sa.cell_nets[m], alt);
    }

    if accept(before_key, gate_key(sa, l, group), pex_cost(sa, l) - before_cost, temp, rng) {
        true
    } else {
        for (i, &m) in group.iter().enumerate() {
            (l.variant[m], l.hw[m], l.hh[m], l.x[m], l.y[m]) = saved[i];
            if let Some(alt) = variants[m].alternatives.get(saved[i].0 as usize) {
                sa.nets.reshape_cell(m, &sa.cell_nets[m], alt);
            }
        }
        false
    }
}

/// The **PEX tier** at the current layout: HPWL + priced analog cost.
///
/// This is the innermost, lexicographically dominated term of PLAN §3b — the only one
/// Metropolis is allowed to vote on. Overlap used to be in here behind a ramped
/// weight and is now in [`gate_key`] instead; with it gone the term no longer depends
/// on *which* devices moved, which is why this takes no `moved` slice.
fn pex_cost(sa: &Sa, l: &Layout) -> f64 {
    hpwl(&sa.nets, l) + f64::from(analog_cost(sa.reqs, l, sa.prices))
}

/// Metropolis criterion (engine `slots::Metropolis`).
#[inline]
fn metropolis(delta: f64, temp: f64, rng: &mut SplitMix64) -> bool {
    delta <= 0.0 || (temp > 0.0 && rng.f64() < (-delta / temp).exp())
}

/// The default drop-in; replaced at the single selection point in
/// `frontend/library`.
#[derive(Default)]
pub struct Placeholder;

impl DetailedPlacer for Placeholder {
    fn place(
        &self,
        coarse: &Layout,
        macros: &[Macro],
        variants: &[gp::VariantSpace],
        reqs: &Requirements<Layout>,
        layers: &[LayerId],
        fixed: &[bool],
        prices: &mut gp::Prices,
        seed: u64,
    ) -> (Layout, Report) {
        Annealer::default().place(coarse, macros, variants, reqs, layers, fixed, prices, seed)
    }
}

#[cfg(test)]
mod rotate_tests {
    use super::*;
    use pnr_core::DeviceId;

    /// Four tall devices; `groups` marks 0 and 1 as one matched structure.
    fn bench() -> Layout {
        Layout {
            x: vec![0, 20_000, 40_000, 60_000],
            y: vec![0; 4],
            hw: vec![1_000; 4],
            hh: vec![8_000; 4],
            variant: vec![0; 4],
            axis: vec![0; 4],
            branch: vec![false; 4],
            groups: vec![
                vec![DeviceId(0), DeviceId(1)],
                vec![DeviceId(2)],
                vec![DeviceId(3)],
            ],
            orient: vec![Orient::default(); 4],
            power_uw: vec![0; 4],
            temp_mc: vec![0; 4],
        }
    }

    /// Matched channels must run parallel; the SA objective cannot see that, so
    /// the move set must refuse to turn a grouped device at all.
    #[test]
    fn matched_devices_never_turn() {
        let l = bench();
        assert!(!rotatable(&l, 0));
        assert!(!rotatable(&l, 1));
        assert!(rotatable(&l, 2));
        assert!(rotatable(&l, 3));
    }

    /// `hw`/`hh` must stay in lockstep with `orient` — `Layout::orient`'s
    /// documented invariant, and what every untouched bbox/overlap read relies on.
    /// A desync would silently place devices with the wrong footprint.
    #[test]
    fn extents_stay_in_lockstep_with_orientation() {
        let coarse = bench();
        let reqs = Requirements::<Layout>::default();
        let (l, _) = Annealer::default()
            .place(&coarse, &[], &[], &reqs, &[], &[false; 4], &mut gp::Prices::new(), 42);

        assert_eq!(l.orient.len(), 4);
        for i in 0..4 {
            let (w, h) = (coarse.hw[i], coarse.hh[i]);
            let want = if l.orient[i].swaps_axes() { (h, w) } else { (w, h) };
            assert_eq!((l.hw[i], l.hh[i]), want, "device {i} extents vs {:?}", l.orient[i]);
        }
        // The grouped pair must come back exactly as it went in.
        assert_eq!(l.orient[0], Orient::R0);
        assert_eq!(l.orient[1], Orient::R0);
    }

    /// The whole engine is seed-deterministic; adding a move type must not break
    /// that, or in-loop DRC feedback stops converging.
    #[test]
    fn rotation_is_deterministic_for_a_seed() {
        let coarse = bench();
        let reqs = Requirements::<Layout>::default();
        let run = || {
            Annealer::default()
                .place(&coarse, &[], &[], &reqs, &[], &[false; 4], &mut gp::Prices::new(), 7)
                .0
                .orient
        };
        assert_eq!(run(), run());
    }

    /// The move must actually fire. A rotate branch that never accepts would pass
    /// every invariant above while doing nothing, so pin a seed known to turn an
    /// ungrouped device.
    ///
    /// The seed moved from 42 to 0 when acceptance became Φ-monotone. A turn on this
    /// bench transposes a 2×16 µm footprint into 16×2 µm, so it now needs ~16 µm of
    /// *clear* neighbouring space instead of merely being worth its overlap penalty at
    /// whatever the ramp had reached. Measured over seeds 0..40, 35 still turn a device;
    /// 42 is one of the five where the displacement moves happen to leave no room.
    #[test]
    fn rotation_actually_happens() {
        let coarse = bench();
        let reqs = Requirements::<Layout>::default();
        let (l, _) = Annealer::default()
            .place(&coarse, &[], &[], &reqs, &[], &[false; 4], &mut gp::Prices::new(), 0);
        assert!(
            l.orient[2..].iter().any(|&o| o != Orient::R0),
            "no ungrouped device turned: {:?}",
            l.orient
        );
    }
}

#[cfg(test)]
mod variant_tests {
    use super::*;
    use gp::VariantSpace;
    use pnr_core::geom::Rect;
    use pnr_core::DeviceId;

    /// A drawn alternative is nothing but its bbox as far as *footprint* bookkeeping is
    /// concerned; the pin-priced tests below use [`pin_alt`] instead.
    fn alt(w: i32, h: i32) -> Macro {
        Macro { shapes: Vec::new(), pins: Vec::new(), bbox: Rect { x: 0, y: 0, w, h } }
    }

    /// Three alternatives per cell, deliberately *not* ordered by similarity — the
    /// enumeration order D7 warns is not a metric, so `variant ± 1` is a different
    /// aspect ratio, not a nearby one.
    fn spaces() -> Vec<VariantSpace> {
        let shapes = || VariantSpace {
            alternatives: vec![alt(2_000, 16_000), alt(16_000, 2_000), alt(6_000, 6_000)],
            lock: None,
        };
        vec![shapes(), shapes()]
    }

    /// The same three alternatives, with both cells **locked** to one shared variant
    /// index — a differential pair as `cellgen` now stamps it.
    fn locked_spaces() -> Vec<VariantSpace> {
        let mut v = spaces();
        for s in &mut v {
            s.lock = Some(0);
        }
        v
    }

    /// What a caller passes as `macros`: the geometry variant 0 names. Only the pins
    /// matter to `dp` (the extents come from `variants`), and these alternatives have
    /// none — which is what keeps the objective flat for the bookkeeping tests.
    fn drawn(variants: &[VariantSpace]) -> Vec<Macro> {
        variants.iter().map(|v| v.alternatives[0].clone()).collect()
    }

    /// Two cells, far enough apart that any alternative fits without overlapping, so
    /// the test is about the reshape bookkeeping and not about Φ refusing it.
    fn bench() -> Layout {
        Layout {
            x: vec![0, 200_000],
            y: vec![0, 0],
            hw: vec![1_000, 1_000],
            hh: vec![8_000, 8_000],
            variant: vec![0, 0],
            axis: vec![0, 0],
            branch: vec![false; 2],
            groups: vec![vec![DeviceId(0)], vec![DeviceId(1)]],
            orient: vec![Orient::default(); 2],
            power_uw: vec![0; 2],
            temp_mc: vec![0; 2],
        }
    }

    /// The move must fire, **and** `hw`/`hh` must be the chosen alternative's — the
    /// [`Layout::variant`] invariant, and the one most likely to rot, because nothing
    /// downstream can tell a stale extent from a real one: every bbox, overlap and
    /// spacing read just silently scores the previous variant's footprint.
    ///
    /// Swept over seeds rather than pinned to one, because with a flat objective every
    /// reshape is accepted and the final index is uniform over three alternatives — so
    /// any single seed lands back on `[0, 0]` about one time in nine. The invariant is
    /// checked on *every* seed; "it fired" only needs one.
    #[test]
    fn reshape_changes_the_variant_and_keeps_the_extent_invariant() {
        let coarse = bench();
        let variants = spaces();
        let reqs = Requirements::<Layout>::default();
        let mut fired = false;
        for seed in 0..8u64 {
            let (l, _) = Annealer::default().place(
                &coarse,
                &drawn(&variants),
                &variants,
                &reqs,
                &[],
                &[false; 2],
                &mut gp::Prices::new(),
                seed,
            );
            fired |= l.variant != coarse.variant;
            for i in 0..2 {
                // The variant and orient invariants compose: extents are the chosen
                // alternative's bbox *after* the orientation transform. Asserting the raw
                // bbox instead is what caught `try_reshape` un-transposing a turned cell.
                let (w, h) = gp::mechanics::variant_extents(
                    &variants[i].alternatives[l.variant[i] as usize],
                );
                let want = if l.orient[i].swaps_axes() { (h, w) } else { (w, h) };
                assert_eq!(
                    (l.hw[i], l.hh[i]),
                    want,
                    "seed {seed}: device {i} is drawn as variant {} orient {:?} but sized \
                     as something else",
                    l.variant[i],
                    l.orient[i]
                );
            }
        }
        assert!(fired, "no seed reshaped anything — the move never fired");
    }

    /// A pinned cell is a user-injected macro. Its *geometry* is the user's, not the
    /// placer's, so reshaping it would quietly substitute a different drawn cell for
    /// the one they asked for — a stronger violation than merely moving it.
    #[test]
    fn a_fixed_cell_never_reshapes() {
        let coarse = bench();
        let variants = spaces();
        let reqs = Requirements::<Layout>::default();
        // Cell 0 pinned, cell 1 free, over several seeds so this is not one lucky draw.
        for seed in 0..8u64 {
            let (l, _) = Annealer::default().place(
                &coarse,
                &drawn(&variants),
                &variants,
                &reqs,
                &[],
                &[true, false],
                &mut gp::Prices::new(),
                seed,
            );
            assert_eq!(l.variant[0], 0, "seed {seed}: pinned cell reshaped");
            // A pinned cell is also never rotated (the generator skips it), so its
            // extents must come back byte-identical.
            assert_eq!(
                (l.hw[0], l.hh[0]),
                (coarse.hw[0], coarse.hh[0]),
                "seed {seed}: pinned cell's extents moved"
            );
        }
    }

    /// **The matching test.** Two cells sharing a `VariantSpace::lock` are a matched
    /// pair, and the reshape move draws a cell *uniformly*, so without the joint move
    /// it refingers one half of a differential pair and leaves the other alone:
    /// different footprint, different pin faces, on devices whose entire purpose is to
    /// be identical. DRC cannot see it and LVS cannot see it, so this test is the only
    /// thing that can.
    #[test]
    fn a_locked_pair_reshapes_together() {
        let coarse = bench();
        let variants = locked_spaces();
        let reqs = Requirements::<Layout>::default();
        let mut fired = false;
        for seed in 0..8u64 {
            let (l, _) = Annealer::default().place(
                &coarse,
                &drawn(&variants),
                &variants,
                &reqs,
                &[],
                &[false; 2],
                &mut gp::Prices::new(),
                seed,
            );
            assert_eq!(
                l.variant[0], l.variant[1],
                "seed {seed}: the lock broke — one half of a matched pair holds variant \
                 {} and the other {}",
                l.variant[0], l.variant[1]
            );
            fired |= l.variant[0] != coarse.variant[0];
            // The extent invariant holds per member, not just for the drawn one: a joint
            // move that resized only the cell the RNG picked would leave the other's
            // footprint scoring the previous variant.
            for i in 0..2 {
                let (w, h) =
                    gp::mechanics::variant_extents(&variants[i].alternatives[l.variant[i] as usize]);
                let want = if l.orient[i].swaps_axes() { (h, w) } else { (w, h) };
                assert_eq!((l.hw[i], l.hh[i]), want, "seed {seed}: cell {i} sized off-variant");
            }
        }
        assert!(fired, "no seed reshaped the locked pair — the joint move never fired");
    }

    /// The other half of the lock: an **unlocked** cell must still reshape alone. A
    /// "reshape everything together" implementation would pass the test above and
    /// silently destroy the search, so the two indices must be seen to diverge.
    #[test]
    fn an_unlocked_cell_still_reshapes_alone() {
        let coarse = bench();
        let variants = spaces();
        let reqs = Requirements::<Layout>::default();
        let diverged = (0..16u64).any(|seed| {
            let (l, _) = Annealer::default().place(
                &coarse,
                &drawn(&variants),
                &variants,
                &reqs,
                &[],
                &[false; 2],
                &mut gp::Prices::new(),
                seed,
            );
            l.variant[0] != l.variant[1]
        });
        assert!(diverged, "no seed gave two unlocked cells different variants");
    }

    /// One alternative, one pin, on the face named by `x`. Every alternative here has
    /// the **same bbox**, so the only thing that distinguishes them is where the net
    /// attaches — which is exactly the discrimination a centre-based HPWL cannot make.
    fn pin_alt(x: i32) -> Macro {
        Macro {
            shapes: Vec::new(),
            pins: vec![pnr_core::Pin {
                name: "G".to_string(),
                net: pnr_core::NetId(0),
                at: Rect { x, y: 4_950, w: 100, h: 100 },
            }],
            bbox: Rect { x: 0, y: 0, w: 10_000, h: 10_000 },
        }
    }

    /// **The test that proves J1 did something.** Two alternatives with equal bboxes and
    /// different pin placement must produce different objectives — PLAN §2's entire
    /// argument for variant choice being a search variable is that a variant change
    /// *relocates pins*. While `dp` had no macros its `Nets` was empty, `hpwl` returned
    /// `0.0` for every layout, and this move was priced on the bbox alone: both
    /// directions below were free and both were taken.
    ///
    /// `temp = 0.0` is load-bearing. It makes Metropolis greedy, so an accept can only
    /// come from a strict PEX improvement and a reject can only come from a strict
    /// worsening — and with identical bboxes a bbox-priced move has `Δ = 0`, which
    /// `metropolis` accepts. So the *rejection* is what cannot be faked.
    #[test]
    fn reshape_is_priced_on_where_the_pins_land() {
        let space = || VariantSpace {
            // variant 0: pin on the left face; variant 1: pin on the right face.
            alternatives: vec![pin_alt(0), pin_alt(9_900)],
            lock: None,
        };
        let variants = vec![space(), space()];
        assert_eq!(
            variants[0].alternatives[0].bbox, variants[0].alternatives[1].bbox,
            "precondition: the two alternatives must be indistinguishable by bbox"
        );

        let mut l = Layout {
            x: vec![0, 100_000],
            y: vec![0, 0],
            hw: vec![5_000, 5_000],
            hh: vec![5_000, 5_000],
            variant: vec![0, 0],
            axis: vec![0, 0],
            branch: vec![false; 2],
            groups: vec![vec![DeviceId(0)], vec![DeviceId(1)]],
            orient: vec![Orient::default(); 2],
            power_uw: vec![0; 2],
            temp_mc: vec![0; 2],
        };
        let reqs = Requirements::<Layout>::default();
        let prices = gp::Prices::new();
        let nets = Nets::from_macros(&[pin_alt(0), pin_alt(0)]);
        assert_eq!(nets.count(), 1, "precondition: the two cells must share one net");
        let mut sa =
            Sa { cell_nets: nets.cell_nets(2), nets, reqs: &reqs, prices: &prices };
        let mut rng = SplitMix64::new(1);
        // Clamps that never bind: this is about pricing, not about the die edge.
        let free = |c: i32, _half: i32| c;

        // Cell 0's pin starts on its left face, pointing away from cell 1. Swinging it to
        // the right face shortens the net by the pin separation (9.9 µm) and nothing else
        // changes, so a greedy pass must take it.
        let before = hpwl(&sa.nets, &l);
        assert!(
            try_reshape(&mut sa, &mut l, &mut rng, 0.0, 0, &[0], &variants, &[], &free, &free),
            "a reshape that shortens the net by moving its pin must be accepted"
        );
        assert_eq!(l.variant[0], 1);
        let after = hpwl(&sa.nets, &l);
        assert!(
            after < before,
            "the accepted reshape must have lowered HPWL: {before} -> {after}"
        );
        assert_eq!(
            (l.hw[0], l.hh[0]),
            (5_000, 5_000),
            "the footprint is identical, so only the pins can have priced this"
        );

        // Now the reverse. Same bboxes, so a bbox-priced move sees Δ = 0 and is accepted
        // at any temperature; a pin-priced one sees +9.9 µm and refuses at temp 0.
        assert!(
            !try_reshape(&mut sa, &mut l, &mut rng, 0.0, 0, &[0], &variants, &[], &free, &free),
            "a reshape that lengthens the net must be refused — if this passes, the \
             objective is blind to pins again"
        );
        assert_eq!(l.variant[0], 1, "the refused reshape must have been reverted");
        assert_eq!(
            hpwl(&sa.nets, &l),
            after,
            "the refused reshape must have reverted the pin geometry too, or the net \
             table now describes a variant the layout does not hold"
        );
    }
}

#[cfg(test)]
mod acceptance_tests {
    use super::*;
    use analog::Rule;
    use gp::mechanics::total_overlap;
    use pnr_core::DeviceId;

    /// A `Cost` rule that pays for stacking: cost is the centre distance of two
    /// devices, minimised at zero separation. Physically it is a proximity/matching
    /// term with no floor — which is exactly the shape of the rules that exist.
    #[derive(Clone, Copy)]
    struct Attract;
    impl Rule for Attract {
        type On = Layout;
        fn cost(self, l: &Layout) -> f32 {
            ((l.x[0] - l.x[1]).abs() + (l.y[0] - l.y[1]).abs()) as f32
        }
    }

    fn bench() -> (Requirements<Layout>, Layout) {
        let reqs = Requirements {
            hard: Vec::new(),
            budget: Vec::new(),
            cost: vec![Box::new(vec![Attract])],
        };
        let l = Layout {
            x: vec![0, 4_000],
            y: vec![0, 0],
            hw: vec![1_000, 1_000],
            hh: vec![1_000, 1_000],
            variant: vec![0, 0],
            axis: vec![0, 0],
            branch: vec![false; 2],
            groups: vec![vec![DeviceId(0)], vec![DeviceId(1)]],
            orient: vec![Orient::default(); 2],
            power_uw: vec![0; 2],
            temp_mc: vec![0; 2],
        };
        (reqs, l)
    }

    /// **The bribe test.** Overlap is a hard violation (`mechanics::report` reports it
    /// as one), so no amount of objective improvement may buy it. Under the old scalar
    /// acceptance the overlap penalty was finite and ramped, so a strong enough `Cost`
    /// rule simply outbid it and the devices shipped stacked — PLAN §3b's "a penalty
    /// that is finite is a bribe the optimizer will accept", verbatim.
    ///
    /// `temp = 1e12` is the load-bearing part of the setup: at that temperature
    /// Metropolis accepts *everything*, so a rejection can only have come from the Φ
    /// tier and not from a lucky draw.
    #[test]
    fn phi_rejects_a_cost_lowering_move_that_stacks_geometry() {
        let (reqs, mut l) = bench();
        let nets = Nets::from_macros(&[]);
        let prices = gp::Prices::new();
        let sa = Sa { cell_nets: nets.cell_nets(2), nets, reqs: &reqs, prices: &prices };
        let mut rng = SplitMix64::new(1);

        // Precondition: sliding 0 onto 1 is a strict PEX improvement...
        let d = sa.hpwl_delta(&l, 0, 4_000, 0) + delta_analog(&reqs, &prices, &mut l, 0, 4_000, 0);
        assert!(d < 0.0, "precondition: the stacking move must look like a win ({d})");
        // ...and it stacks the two footprints exactly.
        assert_eq!(total_overlap(&l), 0.0, "precondition: nothing overlaps yet");

        assert!(
            !try_move(&sa, &mut l, &mut rng, 1e12, 0, 4_000, 0),
            "a move that raises Φ must be refused at any temperature"
        );
        assert_eq!(l.x[0], 0, "the refused move must have been reverted");
        assert_eq!(total_overlap(&l), 0.0);
    }

    /// The middle tier. A budget is a constraint that must hold, not a competing
    /// objective, so it also outranks PEX — and with `Requirements::budget` carrying a
    /// real `residual` there is finally something to rank. Before the budget arm existed
    /// this tier was structurally empty and a budget could only be *priced*, which is the
    /// finite penalty PLAN §3b calls a bribe.
    #[test]
    fn theta_outranks_pex_so_a_budget_is_never_traded_for_parasitics() {
        // A budget that is satisfied while the two devices are apart and overshoots once
        // they are within 2 µm — a minimum-separation matching budget in miniature.
        #[derive(Clone, Copy)]
        struct Separation;
        impl Rule for Separation {
            type On = Layout;
            fn cost(self, _: &Layout) -> f32 {
                0.0
            }
            fn residual(self, l: &Layout) -> f32 {
                let gap = (l.x[0] - l.x[1]).abs();
                ((2_000 - gap).max(0) as f32) / 2_000.0
            }
        }

        let (mut reqs, mut l) = bench();
        reqs.budget = vec![Box::new(vec![Separation])];
        let nets = Nets::from_macros(&[]);
        let prices = gp::Prices::new();
        let sa = Sa { cell_nets: nets.cell_nets(2), nets, reqs: &reqs, prices: &prices };
        let mut rng = SplitMix64::new(1);

        // Closing to a 1 µm gap: no overlap (footprints are 2 µm wide, so V is
        // unchanged), PEX strictly improves, and Θ goes from 0 to 0.5.
        assert_eq!(analog_theta(&reqs, &l), 0.0, "precondition: the budget is satisfied");
        let d = sa.hpwl_delta(&l, 0, 3_000, 0) + delta_analog(&reqs, &prices, &mut l, 0, 3_000, 0);
        assert!(d < 0.0, "precondition: closing the gap must look like a win ({d})");

        assert!(
            !try_move(&sa, &mut l, &mut rng, 1e12, 0, 3_000, 0),
            "a move that spends budget margin on parasitics must be refused"
        );
        assert_eq!(l.x[0], 0, "the refused move must have been reverted");
        assert_eq!(analog_theta(&reqs, &l), 0.0);
    }

    /// The other half of D10: Metropolis is *confined* to the PEX tier, not removed
    /// from it. A Φ-neutral move that costs PEX must still be reachable, or the search
    /// degenerates into greedy descent and stops escaping basins.
    #[test]
    fn metropolis_still_accepts_an_uphill_move_inside_the_pex_tier() {
        let (reqs, mut l) = bench();
        let nets = Nets::from_macros(&[]);
        let prices = gp::Prices::new();
        let sa = Sa { cell_nets: nets.cell_nets(2), nets, reqs: &reqs, prices: &prices };
        let mut rng = SplitMix64::new(1);

        // Away from device 1: no overlap either side (Φ unchanged), and PEX rises.
        let d = sa.hpwl_delta(&l, 0, -20_000, 0)
            + delta_analog(&reqs, &prices, &mut l, 0, -20_000, 0);
        assert!(d > 0.0, "precondition: the move must be uphill in PEX ({d})");
        assert!(
            try_move(&sa, &mut l, &mut rng, 1e12, 0, -20_000, 0),
            "a Φ-neutral uphill move must still be reachable at high temperature"
        );
        assert_eq!(l.x[0], -20_000);
    }
}

#[cfg(test)]
mod projection_tests {
    use super::*;
    use analog::placement::symmetry::{Symmetry, SymmetryGroup};
    use pnr_core::ids::{AxisId, Target};
    use pnr_core::DeviceId;

    /// A differential pair the annealer alone can never satisfy: `Symmetry` is an
    /// exact equality on integers, so SA drives the residual small and stops. The
    /// two devices start deliberately off-mirror (unequal `y`, `x` sum ≠ 2·axis)
    /// with plenty of clear die around them, so the only obstacle to satisfaction
    /// is whether projection runs at all. All coordinates are grid multiples
    /// (grid = 5), so the terminal snap is a no-op and byte-identity assertions
    /// are meaningful.
    fn bench() -> (Requirements<Layout>, Layout) {
        let reqs = Requirements {
            hard: vec![Box::new(SymmetryGroup(vec![Symmetry {
                a: Target::Device(DeviceId(0)),
                b: Target::Device(DeviceId(1)),
                axis: AxisId(0),
            }]))],
            budget: Vec::new(),
            cost: Vec::new(),
        };
        let l = Layout {
            x: vec![-30_000, 30_500],
            y: vec![0, 3_500],
            hw: vec![1_000; 2],
            hh: vec![1_000; 2],
            variant: vec![0; 2],
            axis: vec![0; 2],
            branch: vec![false; 2],
            groups: vec![vec![DeviceId(0)], vec![DeviceId(1)]],
            orient: vec![Orient::default(); 2],
            power_uw: vec![0; 2],
            temp_mc: vec![0; 2],
        };
        (reqs, l)
    }

    /// **The Phase-3 headline.** With projection inside the epoch loop the mirror
    /// equality holds *exactly* at exit — under penalty-only annealing this rule
    /// was violated on every run ever made, because no finite number of accepted
    /// moves closes an integer equality. Once established, Φ-monotone acceptance
    /// keeps it: any single-cell displacement off the mirror raises the violating
    /// batch count and is refused.
    #[test]
    fn symmetry_holds_at_exit_with_room_to_move() {
        let (reqs, coarse) = bench();
        let (l, _) = Annealer::default()
            .place(&coarse, &[], &[], &reqs, &[], &[false; 2], &mut gp::Prices::new(), 11);
        assert_eq!(
            analog_violations(&reqs, &l),
            0,
            "the mirror equality must hold exactly at exit: x = {:?}, y = {:?}, axis = {:?}",
            l.x,
            l.y,
            l.axis
        );
    }

    /// Projection moves both partners of a pair; a pinned partner must not be one
    /// of them. `project_hard` restores pinned cells before its Φ re-check, and
    /// every other stage (moves, legalize, snap-on-grid-multiples) already honours
    /// the pin — so the injected macro's coordinates come back byte-identical even
    /// though its free partner is being mirrored around every epoch.
    #[test]
    fn epoch_projection_never_moves_pinned_cells() {
        let (reqs, coarse) = bench();
        for seed in 0..4u64 {
            let (l, _) = Annealer::default().place(
                &coarse,
                &[],
                &[],
                &reqs,
                &[],
                &[true, false],
                &mut gp::Prices::new(),
                seed,
            );
            assert_eq!(
                (l.x[0], l.y[0]),
                (coarse.x[0], coarse.y[0]),
                "seed {seed}: pinned partner moved"
            );
        }
    }

    /// Projection consumes no RNG (`project_hard` never sees the generator), so
    /// determinism over `(inputs, seed)` survives it — this is the same guarantee
    /// `rotation_is_deterministic_for_a_seed` pins, re-checked on the code path
    /// where projection actually fires every epoch.
    #[test]
    fn projection_is_deterministic_for_a_seed() {
        let run = || {
            let (reqs, coarse) = bench();
            let (l, _) = Annealer::default()
                .place(&coarse, &[], &[], &reqs, &[], &[false; 2], &mut gp::Prices::new(), 7);
            (l.x, l.y, l.axis)
        };
        assert_eq!(run(), run());
    }
}
