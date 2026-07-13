//! The one rule abstraction every engine implements.
//!
//! DRC, ERC, LVS, PEX, and signoff all answer the same shape of question: run
//! every rule over some context and sum the findings. A rule is a pure
//! function from its context to a list of findings; the engine only differs
//! in what the context and finding types are.
//!
//! Backend dispatch is implicit: a rule receives the requested [`Backend`]
//! and may consult its engine's GPU prefilters (`drc::gpu`, `erc::gpu`, ...),
//! which degrade to `None` (=> exact CPU path) when no device is usable.

use rayon::prelude::*;

use crate::backend::Backend;
use crate::geometry::GeometryStore;
use crate::params::Deck;

/// One verification rule over an engine-specific context.
pub trait Rule<Ctx: ?Sized>: Send + Sync {
    /// What one hit looks like: a DRC violation, an ERC violation, a
    /// parasitic, an LVS mismatch, a signoff check report.
    type Finding;
    /// Stable rule identifier for reports and deduplication.
    fn id(&self) -> &str;
    /// Run the rule. Exact by contract; `backend` only enables pruning.
    fn check(&self, ctx: &Ctx, backend: Backend) -> Vec<Self::Finding>;
}

/// Boxed rule with the finding type fixed — what the generated rule
/// registries hand back.
pub type DynRule<Ctx, F> = Box<dyn Rule<Ctx, Finding = F>>;

/// Run every rule against the context in parallel and sum the findings.
/// Ordering across rules follows the rules slice (rayon preserves it).
/// `R` may be a trait object (including a higher-ranked one, e.g.
/// `dyn for<'a> Rule<DrcCtx<'a>, Finding = Violation>`).
pub fn run_rules<Ctx, F, R>(rules: &[Box<R>], ctx: &Ctx, backend: Backend) -> Vec<F>
where
    Ctx: Sync,
    F: Send,
    R: Rule<Ctx, Finding = F> + ?Sized,
{
    rules
        .par_iter()
        .flat_map_iter(|rule| rule.check(ctx, backend))
        .collect()
}

/// A verification check that can run on CPU or GPU.
///
/// Legacy trait predating [`Rule`]; engines are migrating rule by rule.
pub trait VerifyCheck {
    type Output;
    fn id(&self) -> &str;
    fn run(&self, store: &GeometryStore, deck: &Deck, backend: Backend) -> Self::Output;
}
