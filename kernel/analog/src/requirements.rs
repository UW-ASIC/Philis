//! The applied rule-set an algorithm consumes.

use crate::rule::RuleBatch;

/// Every rule batch for one scored state `On`, partitioned by tier: which `Vec`
/// a batch sits in *is* its enforcement mode.
///
/// - `hard` (V): [`RuleBatch::violations`]. Legality and exact equalities
///   (repaired by [`RuleBatch::project`], never by weight).
/// - `budget` (Θ): [`RuleBatch::residual`], priced by `gp::Prices`.
/// - `cost` (PEX): [`RuleBatch::cost`].
///
/// A kind may sit in `hard` or `budget` **and** in `cost` (feasible set +
/// gradient toward it), never in both `hard` and `budget`.
///
/// **Order within an arm is a contract for untagged batches:** `gp::Prices`
/// re-finds a tagged batch by its `BatchMeta::id`, so order only matters for
/// untagged batches, where it re-finds by `(kind(), ordinal among untagged
/// same kind)` and reordering silently moves prices. In-loop DRC batches are
/// appended to `hard` and truncated off, keeping the prefix.
pub struct Requirements<On> {
    pub hard: Vec<Box<dyn RuleBatch<On>>>,
    pub budget: Vec<Box<dyn RuleBatch<On>>>,
    pub cost: Vec<Box<dyn RuleBatch<On>>>,
}

impl<On> Default for Requirements<On> {
    fn default() -> Self {
        Self {
            hard: Vec::new(),
            budget: Vec::new(),
            cost: Vec::new(),
        }
    }
}
