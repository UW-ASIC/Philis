//! # `analog` — the analog-layout theory of Philis
//!
//! **Zero dependencies.** This crate owns *what* good analog layout requires and
//! knows nothing about *how* any algorithm achieves it. Algorithms depend on
//! `analog`; `analog` depends on nothing. That inversion is the whole design: a
//! new rule costs the algorithms nothing, and a new algorithm costs the theory
//! nothing. See `docs/adr/0001-mechanism-policy-split.md`.
//!
//! ## The constraint catalogue — four tiers, one file per constraint
//!
//! Every analog constraint is documented in its own file with the analog theory
//! and its **textbook provenance** (`AOAL`/`FOLD`/`ALS`/`PNR_ANALOG`, indexed in
//! `docs/books/`). The tiers mirror the consumers:
//!
//! - [`cell`] — **structural** directives read cold by `cells` (dummies, guard
//!   rings, unit fingers, LDE, stress). Data, not scored.
//! - [`placement`] — **Device↔Device** [`Rule`]s scored against [`Layout`], for
//!   `backend/gp` + `dp`.
//! - [`routing`] — **Net-arity** [`Rule`]s scored against [`Routes`], for
//!   `backend/gr` + `dr`.
//! - [`metadata`] — **classification tags** (net class, bias, ESD) that the
//!   `annotator` turns into the constraints/rules above.
//!
//! ## The extraction pipeline (who produces what)
//!
//! ```text
//! netlist ──annotator (recognise structure)──▶ [Block]
//!                                                │  cells::extract()  ← the extraction
//!                                                ▼
//!                                           Constraints
//!             ┌──────────────────────────────────┼──────────────────────────────┐
//!             ▼                                   ▼                              ▼
//!    cells::enumerate                     apply_placement                 apply_routing
//!    (pick cell variants)                 Requirements<Layout>            Requirements<Routes>
//! ```
//!
//! Extraction (*structure → `Constraints`*) is **mechanism** and lives in
//! `cells::extract`, not here — `cells` is its first consumer (it picks device
//! variants from the result). This crate only supplies the [`Constraints`] type
//! and, via [`apply_placement`] / [`apply_routing`], the *`Constraints` → hot
//! `Rule`s* mapping. `Block`/[`BlockKind`] are the vocabulary the annotator fills
//! and `cells::extract` reads.
//!
//! ## The two seams, split by temperature
//!
//! - **Theory → algorithm (HOT, per move):** flat [`Rule`] data in per-kind SoA
//!   arrays; the engine is generic over [`Rule`] (static dispatch, inlines,
//!   vectorises). [`Rule::On`] is the scored state — [`Layout`] or [`Routes`].
//!   The only `dyn` is [`RuleBatch`], one cold call per kind. See `docs/adr/0002`.
//! - **Algorithm selection (COLD, once per run):** the `GlobalPlacer` /
//!   `DetailedRouter` traits in `backend/*` — swap whole algorithms as drop-ins.
//!
//! ## Enforcement mode — three tiers, one per rule
//!
//! [`Mode::Hard`] = legality and exact equalities (violation → solution rejected);
//! [`Mode::Budget`] = an accumulating allowance with a spec *and* a margin, priced and
//! ramped toward hard; [`Mode::Cost`] = objective (tradeable, never silently dropped).
//! These are PLAN §3b's `lex-min(V, Θ, PEX)` made structural, and
//! [`Requirements`]' three arms are how a batch declares which it is.
//!
//! Mode is bound *at application*, not on the rule type — but a rule lands in **exactly
//! one** arm. Registering one batch in two (which is what the codebase did before
//! `Budget` existed) counts it twice in the lex key, prices it twice, and puts one
//! constraint simultaneously above and below the feasibility frontier.
//!
//! ## The three questions a batch answers
//!
//! [`RuleBatch::violations`] for V, [`RuleBatch::residual`] for Θ, [`RuleBatch::cost`]
//! for PEX. `residual` is **normalised by the rule's own budget**, which is the only
//! thing that makes Θ summable: routing overflow is a dimensionless node count and a
//! spacing shortfall is nm, so added raw a 1-node overflow would weigh the same as a
//! 1 nm miss. Every `Mode::Budget` rule overrides it; a rule that cannot is arguably not
//! a budget. Exact equalities keep the `0.0`/`1.0` default on purpose — an equality has
//! no allowance to be a fraction of, and its repair is [`Rule::project`], not a smaller
//! number.

#![allow(dead_code)]

pub mod cell;
pub mod constraints;
pub mod metadata;
pub mod mode;
pub mod placement;
pub mod requirements;
pub mod routing;
pub mod rule;

pub use constraints::Constraints;
pub use mode::Mode;
pub use placement::matching_pair::Matching;
pub use requirements::Requirements;
pub use rule::{Rule, RuleBatch};
