//! Routing-tier rule assembly — **call each routing rule's `extract`, batch,
//! partition**.
//!
//! Since Q6a the *placement* tier is recognition-driven (see [`crate::emit`]): the
//! annotator emits placement constraints from the recognised block hierarchy, not
//! from per-rule `extract`. Routing rules still self-extract over nets here (they
//! key off net arity, not device grouping), so this module keeps the batch/partition
//! machinery for the routing tier only.
//!
//! Each `Rule::extract` may still be handed a `UnionFind`; routing rules don't group,
//! so a throwaway one is passed and discarded.

use analog::routing::{
    Antenna, CouplingBudget, CrosstalkExclusion, Differential, ParasiticBudget, StraightNet,
};
use analog::rule::Rule;
use analog::{Requirements, RuleBatch};
use pnr_core::{BipartiteHypergraph, Routes, UnionFind};

/// Which partition a rule's batch lands in — a rule's `Enforcement:` doc line.
///
/// Three arms, matching PLAN §3b's three tiers. The old two-arm version could not
/// express a **budget**, and the codebase worked around that by registering the same
/// batch in *both* `hard` and `cost` (via `clone`) — simultaneously a legality gate
/// and an objective term. Every such double-registration is a budget that had nowhere
/// to live.
///
/// **The migration is not a mechanical sweep of the double-registrations**, and this
/// is the trap: `emit.rs` also double-registers `SymmetryGroup`, and symmetry is
/// emphatically **not** a budget. A budget is by definition *tradeable* — it is priced,
/// its multiplier says how binding it is, and the search is allowed to sit at a
/// positive residual while it converges. An exact equality is not tradeable at any
/// price: PLAN §4a shows a penalty gradient on `|x_i + x_j − 2·axis|` reaches
/// *near*-feasible and then parks one grid unit off forever, because the gradient
/// vanishes exactly where it would need to bite. Demoting symmetry to a budget would
/// make that permanent and call it convergence.
///
/// So classify by what the constraint *is*:
/// - `Hard` — strict legality and exact equalities. DRC, LVS structure, symmetry
///   axes, integer ratios, disjunctive branches. Enforced by construction,
///   projection, or branching; never priced.
/// - `Budget` — accumulating set-level allowances with a real spec and a margin:
///   coupling sums, parasitic R/C, thermal gradients, matching voltage. Priced by
///   `gp::Prices` (λ, ρ) and ramped toward hard as the run converges.
/// - `Cost` — the objective proper. Shaping terms with no spec to violate.
#[derive(Clone, Copy)]
enum Enforce {
    Hard,
    Budget,
    Cost,
}

/// Push `R`'s extracted instances as one boxed batch onto exactly one partition.
///
/// **Exactly one.** A batch that appears in two partitions is counted twice by
/// `Report::lex`, priced twice by `gp::Prices`, and — worse — makes the tier ordering
/// meaningless, since the same constraint then sits both above and below the
/// feasibility frontier.
fn add<R>(
    hg: &BipartiteHypergraph,
    uf: &mut UnionFind,
    hard: &mut Vec<Box<dyn RuleBatch<R::On>>>,
    budget: &mut Vec<Box<dyn RuleBatch<R::On>>>,
    cost: &mut Vec<Box<dyn RuleBatch<R::On>>>,
    enforcement: Enforce,
) where
    R: Rule + 'static,
{
    let batch: Vec<R> = R::extract(hg, uf);
    // The `match` picks *one* arm and `push_one` is the only way in, so "exactly one
    // partition" is structural here rather than a rule someone has to remember.
    let arm = match enforcement {
        Enforce::Hard => hard,
        Enforce::Budget => budget,
        Enforce::Cost => cost,
    };
    push_one(arm, Box::new(batch));
}

/// Push one boxed batch onto one partition, asserting the schedule stays **keyable**.
///
/// Batch order and identity have to survive across epochs, because `gp::Prices` carries
/// a per-batch (λ, ρ) and re-finds its batch by
/// `(RuleBatch::kind(), ordinal among same-kind batches)`. Keying by kind instead of by
/// position already removes the dependency on where a batch sits — but only as long as
/// the *ordinal* is not load-bearing, and the ordinal stops being load-bearing exactly
/// when no kind appears twice in one arm. So that is what is asserted: with every kind
/// unique per partition the key is reorder-proof, which is strictly stronger than "the
/// `add` calls happen to be in a fixed order today".
///
/// It is asserted rather than left as a property nobody stated because the failure is
/// silent: a reorder would transfer one budget's accumulated price to another and the
/// run would keep producing plausible, wrong numbers instead of crashing.
///
/// `debug_assert`, for `gp::Prices::bind`'s reason — the diagnosis matters, the run is
/// not corrupt. Note this covers the **routing** tier only: [`crate::emit`] deliberately
/// emits one placement batch per recognised block, so `ThermalGradient` legitimately
/// repeats and its ordinal is the block order, which `pattern::recognize` fixes by
/// returning matches in a deterministic priority/instance order.
fn push_one<On>(arm: &mut Vec<Box<dyn RuleBatch<On>>>, batch: Box<dyn RuleBatch<On>>) {
    debug_assert!(
        !arm.iter().any(|b| b.kind() == batch.kind()),
        "annotator: a second `{}` batch in one routing partition — `gp::Prices` keys a \
         carried (λ, ρ) by (kind, ordinal among same-kind batches), so a duplicated kind \
         makes that key depend on push order and any later reordering silently transfers \
         one budget's accumulated price to another",
        batch.kind()
    );
    arm.push(batch);
}

/// Assemble the routing [`Requirements`] by calling every routing rule's `extract`.
///
/// | rule                 | mode   | source        |
/// |----------------------|--------|---------------|
/// | `Antenna`            | Hard   | legality      |
/// | `Differential`       | Hard   | legality      |
/// | `CrosstalkExclusion` | Budget | spec + margin |
/// | `ParasiticBudget`    | Budget | spec + margin |
/// | `StraightNet`        | Cost   | objective     |
#[must_use]
pub fn routing(netlist: &pnr_core::Netlist) -> Requirements<Routes> {
    let hg = BipartiteHypergraph::from_netlist(netlist);
    let mut uf = UnionFind::new(hg.device_count()); // routing rules don't group
    let mut r = Requirements::<Routes>::default();
    let (h, b, c) = (&mut r.hard, &mut r.budget, &mut r.cost); // one per partition
    add::<Antenna>(&hg, &mut uf, h, b, c, Enforce::Hard);
    add::<Differential>(&hg, &mut uf, h, b, c, Enforce::Hard);
    add::<CrosstalkExclusion>(&hg, &mut uf, h, b, c, Enforce::Budget);
    add::<ParasiticBudget>(&hg, &mut uf, h, b, c, Enforce::Budget);
    add::<StraightNet>(&hg, &mut uf, h, b, c, Enforce::Cost);
    r
}

/// [`routing`], with every per-net budget keyed to the net's **class** instead of
/// a single placeholder.
///
/// Self-extraction finds *where* a rule applies; it cannot know what the budget
/// should be, because that depends on what the net is *for*. A bias reference and
/// a dumb digital-ish signal both get a `ParasiticBudget` from `extract`, but they
/// should not get the same number — which is exactly the gap the `metadata` tier
/// was staged to fill (`backend/TODO.md` §5).
///
/// Budgets are also **derated by the class margin** before they reach the rule, so
/// the optimiser targets the derated value while the raw spec stays the terminal
/// floor (§5b).
#[must_use]
pub fn routing_classified(
    netlist: &pnr_core::Netlist,
    classes: &[analog::metadata::NetClassification],
) -> Requirements<Routes> {
    let hg = BipartiteHypergraph::from_netlist(netlist);
    let mut uf = UnionFind::new(hg.device_count());
    let mut r = Requirements::<Routes>::default();

    // Structural rules: recognition alone decides where these apply.
    let (h, b, c) = (&mut r.hard, &mut r.budget, &mut r.cost); // one per partition
    add::<Antenna>(&hg, &mut uf, h, b, c, Enforce::Hard);
    add::<Differential>(&hg, &mut uf, h, b, c, Enforce::Hard);
    add::<StraightNet>(&hg, &mut uf, h, b, c, Enforce::Cost);

    // Crosstalk: self-extraction recognises the diff-stage input↔output pairs,
    // then each pair's floor is tightened to whatever its *victim* class demands.
    let mut xtalk: Vec<CrosstalkExclusion> = CrosstalkExclusion::extract(&hg, &mut uf);
    for x in &mut xtalk {
        let victim = class_of(classes, x.a).min_spacing_nm().max(class_of(classes, x.b).min_spacing_nm());
        x.min_spacing_nm = x.min_spacing_nm.max(victim);
    }
    // A crosstalk exclusion has a spec (`min_spacing_nm`, tightened to the victim
    // class's demand) and a residual, and the run is allowed to sit at a positive
    // residual while negotiating it down — that is a budget, not legality. It was
    // registered in `hard` *and* `cost`, which was the two-arm workaround for the
    // missing middle tier; one push replaces both.
    push_one(&mut r.budget, Box::new(xtalk));

    // Parasitic budgets: one per classified, routed net.
    let mut par: Vec<ParasiticBudget> = Vec::new();
    for (n, devs) in hg.net_devices.iter().enumerate() {
        if devs.len() < 2 {
            continue; // nothing to route ⇒ nothing to budget
        }
        let Some(c) = classes.get(n) else { continue };
        let margin_pct = margin_for(c.class);
        par.push(ParasiticBudget {
            net: pnr_core::NetId(n as u16),
            max_r_mohm: c.r_budget_mohm.unwrap_or(1_000_000),
            max_c_af: c.c_budget_af.unwrap_or(100_000_000),
            // Lower the electrical budget into a routing resource (§5a): a
            // resistance cap is a length cap once sheet resistance is known.
            // ponytail: 100 mΩ/µm stand-in until the PDK carries sheet_r.
            max_len_nm: (c.r_budget_mohm.unwrap_or(1_000_000) / 100) * 1_000,
            margin_pct,
        });
    }
    // The clearest budget in the codebase — it carries both a spec
    // (`max_r_mohm`/`max_c_af`/`max_len_nm`) and a `margin_pct`, which is exactly the
    // (budget, margin) pair a residual needs. So this batch and `CouplingBudget` below are
    // the two that report a *real* normalised overshoot into Θ
    // (`ParasiticBudget::residual`, `Vec<CouplingBudget>::residual`) instead of the
    // violation count every other batch still falls back to.
    push_one(&mut r.budget, Box::new(par));

    // Total coupling per victim. Pairwise spacing (above) cannot express this:
    // several aggressors each at the legal minimum still sum past the budget, and
    // the sum is what injects noise. This is what consumes the classifier's
    // `max_coupling_af`.
    let mut coup: Vec<CouplingBudget> = Vec::new();
    for (n, devs) in hg.net_devices.iter().enumerate() {
        if devs.len() < 2 {
            continue;
        }
        let Some(c) = classes.get(n) else { continue };
        let Some(max_coupling_af) = c.max_coupling_af else { continue };
        coup.push(CouplingBudget {
            net: pnr_core::NetId(n as u16),
            max_coupling_af,
            margin_pct: margin_for(c.class),
        });
    }
    // Note *why* this rule exists at all, because it is the one PLAN §4c singles out: no
    // pairwise spacing rule can express "Σ over all aggressors ≤ budget". A victim
    // flanked by five minimum-spaced aggressors passes `CrosstalkExclusion` on every pair
    // and blows the coupling budget 5×. A set-level sum is not a stricter pairwise rule;
    // it is a different kind of constraint, which is precisely why it needs the budget
    // tier rather than a tighter `hard` threshold — and why its residual is the sum's
    // overshoot, not a count of victims.
    push_one(&mut r.budget, Box::new(coup));

    r
}

/// Class of a net, defaulting to `Signal` for an unclassified id.
fn class_of(
    classes: &[analog::metadata::NetClassification],
    net: pnr_core::NetId,
) -> analog::metadata::NetClass {
    classes
        .get(net.0 as usize)
        .map_or(analog::metadata::NetClass::Signal, |c| c.class)
}

/// Safety margin per class, percent. A reference holds back more than a signal
/// because its budget is what sets the circuit's precision.
fn margin_for(class: analog::metadata::NetClass) -> u8 {
    use analog::metadata::NetClass as C;
    match class {
        C::Sensitive => 35,
        C::Clock => 30,
        C::Supply | C::Ground => 25,
        _ => 20,
    }
}

/// Minimum run-adjacent spacing a class demands, nm.
trait ClassSpacing {
    fn min_spacing_nm(self) -> i32;
}

impl ClassSpacing for analog::metadata::NetClass {
    fn min_spacing_nm(self) -> i32 {
        use analog::metadata::NetClass as C;
        match self {
            C::Sensitive => 1_200, // a reference buys quiet with area
            C::Clock => 1_000,     // worst aggressor
            C::Signal => 400,
            _ => 200,
        }
    }
}
