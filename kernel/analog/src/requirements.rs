//! The applied rule-set an algorithm consumes, parameterised by scored state.

use crate::rule::RuleBatch;

/// Every rule applied to a circuit for a given scored state `On`, partitioned by
/// mode via **existence**: which `Vec` a batch sits in *is* its [`crate::Mode`], so
/// there is no mode field to branch on in the hot loop.
///
/// The three arms are the three tiers of PLAN §3b's lexicographic objective,
/// evaluated by three different questions:
///
/// ```text
/// lex-min ( V(x)  ,  Θ(x)   ,  PEX(x) )
///           hard     budget    cost
///           .violations()      .residual()      .cost()
///           "is it legal"      "how far past"   "how good"
/// ```
///
/// A batch belongs to **exactly one** arm — with one named exception. A constraint
/// may register in its owning tier (`hard` or `budget`) **and** in `cost`: the owning
/// copy says where the feasible set is, the `cost` copy is the gradient that leads
/// there (MAGICAL's `fSYM` beside its legalizer). No tier double-counts — `|V|`/Θ read
/// the first copy, PEX the second — and `gp::Prices` prices the `budget` arm only, so
/// the `cost` copy is never priced. What is **never** legal is one kind in both `hard`
/// and `budget`: that gates a batch as legality and prices it as tradeable at once,
/// and `gp::Prices::bind` debug-asserts against it.
///
/// `Requirements` are *assembled by the `annotator`*, which calls each rule's
/// [`crate::Rule::extract`], groups the results into homogeneous per-kind arrays,
/// boxes them as [`RuleBatch`], and partitions them by enforcement. `analog` owns the
/// rule *types* and their `extract`; it does not own assembly (that is why there is no
/// `apply_*` here — see ADR / annotator).
///
/// **Order within each arm is a contract, not an accident.** `gp::Prices` carries a
/// per-batch (λ, ρ) across epochs and re-finds a batch by
/// `(RuleBatch::kind(), ordinal among same-kind batches)`. Reordering silently
/// transfers one budget's accumulated price to another — a bug that yields plausible
/// wrong numbers rather than a crash. The in-loop DRC feedback batches are *appended*
/// to `hard` and truncated back off, which is what keeps the prefix stable.
pub struct Requirements<On> {
    /// Legality batches (tier **V**) — evaluated with [`RuleBatch::violations`].
    ///
    /// Strict legality *and* exact equalities. An equality lives here rather than in
    /// `budget` because it is not tradeable at any price: PLAN §4a shows a penalty on
    /// `|x_i + x_j − 2·axis|` converging to near-feasible and parking one grid unit
    /// off, since the gradient vanishes exactly where it would need to bite. Enforce
    /// by representation or [`RuleBatch::project`], never by weight.
    pub hard: Vec<Box<dyn RuleBatch<On>>>,
    /// Budget batches (tier **Θ**) — evaluated with [`RuleBatch::residual`] and priced
    /// by the augmented-Lagrangian layer.
    ///
    /// Residual, not a violation *count*: Θ is summed and compared, and a count makes
    /// a budget missed by 1 nm indistinguishable from one missed by 1 µm — so the
    /// search may sit on a large violation indefinitely as long as it adds no new one.
    pub budget: Vec<Box<dyn RuleBatch<On>>>,
    /// Objective batches (tier **PEX**) — evaluated with [`RuleBatch::cost`].
    pub cost: Vec<Box<dyn RuleBatch<On>>>,
}

impl<On> Default for Requirements<On> {
    fn default() -> Self {
        Self { hard: Vec::new(), budget: Vec::new(), cost: Vec::new() }
    }
}
