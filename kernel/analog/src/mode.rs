//! Enforcement mode — *how strictly* a rule is applied.

/// How strictly a rule is applied, chosen when the rule is attached to targets,
/// **never intrinsic to the rule**.
///
/// Some rules are *pinned* by convention where they are applied — `DRC`/`LVS` are
/// always `Hard`, `PEX`/`ERC` always `Cost` — but that is a fact of the
/// application site, not of the rule type.
///
/// A rule lands in **exactly one** mode. The doc line above used to say the same rule
/// could appear in both (`Hard` floor plus `Cost` shaping), and the codebase did that
/// by registering one batch twice — which was the two-mode workaround for the missing
/// [`Mode::Budget`]. Double registration double-counts the constraint in the
/// lexicographic key and prices it twice, and it puts one constraint both above and
/// below the feasibility frontier, which makes the tier ordering meaningless.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    /// Legality. A violation makes the solution invalid; the engine may not
    /// return it.
    Hard,
    /// **Budget.** An accumulating allowance with a real spec and a margin: a coupling
    /// sum over every aggressor, a parasitic R/C cap, a thermal gradient, a
    /// matching-voltage limit. Priced by an augmented-Lagrangian multiplier λ and
    /// ramped toward hard as the run converges (PLAN §6).
    ///
    /// A **third kind** of constraint, not a severity between the other two — which is
    /// why two modes could not express it:
    ///
    /// - *Tradeable*, so it cannot be `Hard`: the search must be allowed to sit at a
    ///   positive residual while it negotiates the budget down.
    /// - *Bounded*, so it cannot be `Cost`: a spec exists and a run that finishes past
    ///   it has failed. An objective term has nothing to violate.
    /// - *Accumulating over a set*, which neither other mode expresses at all. PLAN
    ///   §4c: no pairwise rule can say "Σ over all aggressors ≤ budget", and a victim
    ///   flanked by five minimum-spaced aggressors passes every pairwise check while
    ///   blowing the budget 5×.
    ///
    /// **Exact equalities are not budgets**, though both are "not quite hard". A
    /// symmetry axis or integer ratio is not tradeable at any price, and pricing one is
    /// precisely PLAN §4a's failure mode: the penalty gradient reaches *near*-feasible
    /// and then parks one grid unit off forever, because it vanishes exactly where it
    /// would have to bite. Those stay `Hard`, enforced by representation and
    /// [`crate::Rule::project`].
    ///
    /// Classification may legitimately change mid-run, in one direction: a budget with
    /// comfortable slack stays cheap; one that becomes binding is promoted by raising
    /// ρ. λ is the shadow price that names which are binding (PLAN §6).
    Budget,
    /// Objective. Optimised toward, tradeable against other `Cost` rules by a
    /// theory-owned weight. **Never silently dropped** from the evaluated
    /// objective — that omission was a real bug in the old engine.
    Cost,
}
