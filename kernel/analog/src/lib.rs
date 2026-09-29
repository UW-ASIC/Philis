//! `analog` — what good analog layout requires, independent of any algorithm.
//!
//! - [`cell`]: structural directives `cells` reads cold ([`Constraints`]).
//! - [`placement`]: [`Rule`]s scored against `pnr_core::Layout` (gp, dp).
//! - [`routing`]: [`Rule`]s scored against `pnr_core::Routes` (gr, dr).
//! - [`metadata`]: net classification the annotator turns into routing rules.
//!
//! Rules are `Copy` values in per-kind `Vec`s scored through a monomorphised
//! loop; the only `dyn` is [`RuleBatch`], once per kind. The annotator
//! assembles them into [`Requirements`], whose arm (hard / budget / cost) is
//! the rule's enforcement tier.

pub mod cell;
pub mod constraints;
pub mod metadata;
pub mod placement;
pub mod requirements;
pub mod routing;
pub mod rule;

pub use constraints::Constraints;
pub use requirements::Requirements;
pub use rule::{Rule, RuleBatch};
