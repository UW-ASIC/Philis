//! [`Report`] — how well a solution satisfies its requirements.

/// The universal return value of every stage, and what `verify` fills at signoff.
///
/// The three fields are the three tiers of PLAN §3b's lexicographic objective,
/// separated **at the type level** so no caller has to reverse-engineer a tier by
/// string-matching a rule name:
///
/// ```text
/// lex-min ( V(x) , Θ(x) , PEX(x) )
///           │       │      └── cost: the objective proper (parasitics, HPWL)
///           │       └───────── budget_violations: budgets, with live residuals
///           └───────────────── hard_violations: strict legality
/// ```
///
/// The ordering is **strict**: no finite amount of `cost` improvement may buy past
/// a `budget_violations` entry, and no `budget_violations` slack may buy past a
/// `hard_violations` entry. A penalty that is finite is a bribe the optimizer will
/// accept, which is why the tiers are compared and not summed — see [`Report::lex`].
#[derive(Default)]
pub struct Report {
    /// **Tier V** — hard requirements violated. **Must be empty** for a legal
    /// solution. Strict legality: DRC, LVS structure, exact equalities
    /// (symmetry axes, integer ratios), disjunctive branches.
    pub hard_violations: Vec<Violation>,
    /// **Tier Θ** — budget requirements violated, each carrying its live residual
    /// in [`Violation::margin`]. Budgets differ from hard rules in *kind*, not just
    /// severity: they accumulate over a set (a coupling sum over every aggressor),
    /// they are priced (an augmented-Lagrangian multiplier λ tracks how binding
    /// each one is), and they are ramped toward hard as the run converges.
    ///
    /// A budget with a positive residual is not a legality failure — it is a
    /// constraint the search is still paying for. Separating it from
    /// `hard_violations` is what lets a caller tell "illegal" from "not yet
    /// converged", which a single violation list cannot express.
    pub budget_violations: Vec<Violation>,
    /// **Tier PEX** — achieved `Cost` objective (lower is better), for comparing
    /// runs/algorithms. Dominated by both tiers above.
    pub cost: f32,
}

impl Report {
    /// The lexicographic comparison key: `(|V|, Θ residual, PEX)`.
    ///
    /// Compare these tuples to rank two solutions; Rust's derived tuple ordering
    /// already gives the strict tier precedence. The orchestrator **sums** these
    /// across stages (placement + routing + in-loop DRC) exactly as it sums hard
    /// violations today, then compares once.
    ///
    /// Θ is the summed residual rather than a count on purpose: a budget missed by
    /// 1 nm and one missed by 1 µm are not equally bad, and a count cannot see the
    /// difference — so a count-based Θ would let the search sit on a large
    /// violation forever as long as it did not add a new one.
    #[must_use]
    pub fn lex(&self) -> (usize, f64, f32) {
        let theta = self.budget_violations.iter().map(|v| v.margin as f64).sum();
        (self.hard_violations.len(), theta, self.cost)
    }

    /// The **repair-monotone measure Φ**: `(|V|, Σ violation margin)`.
    ///
    /// Distinct from [`Report::lex`] and used for a different job. `lex` ranks
    /// *solutions*; Φ gates *repairs*. PLAN §3a: fixing rule A must not re-create
    /// rule B, and a naive "accept anything that reduces the violation count" rule
    /// admits exactly that cycle. Accepting only repairs that do not increase Φ
    /// lexicographically makes the repair-dependency graph acyclic by construction
    /// — the discipline of the feasibility pump, and the placement-side twin of
    /// PathFinder's history cost (`gr::Negotiation`).
    ///
    /// The second element is why a bare count is not enough: two layouts can have
    /// one violation each while one is 1 nm short and the other 1 µm short, and a
    /// count-only Φ lets a repair trade the first for the second forever. When no
    /// single-move repair lowers Φ, the caller escalates to a larger neighbourhood
    /// (rip up a whole group) rather than accepting a sideways move.
    #[must_use]
    pub fn phi(&self) -> (usize, f64) {
        let area = self.hard_violations.iter().map(|v| v.margin as f64).sum();
        (self.hard_violations.len(), area)
    }

    /// `true` when both violation tiers are empty — PLAN §5's feasibility half of
    /// the termination criterion (`V = 0` **and** `Θ = 0`). Feasibility alone is
    /// not termination; the caller must also check that no move improves PEX and
    /// that the constraint prices have gone stationary.
    #[must_use]
    pub fn feasible(&self) -> bool {
        self.hard_violations.is_empty() && self.budget_violations.is_empty()
    }
}

/// One violated requirement, with enough context to locate it.
pub struct Violation {
    /// Human-readable rule identity (e.g. `"spacing met1"`).
    pub rule: String,
    /// How badly it is violated (e.g. shortfall in `nm`); `0` means "boolean fail".
    ///
    /// For a [`Report::budget_violations`] entry this is the **live residual**
    /// `c(x)` the augmented-Lagrangian layer prices, so it must be a real measured
    /// overshoot and not a boolean `0` — a budget reporting `0` reads as satisfied
    /// to [`Report::lex`].
    pub margin: i64,
}
