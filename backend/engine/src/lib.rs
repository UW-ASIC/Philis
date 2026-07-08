//! pnr-engine — P&R search algorithms + block-level orchestration.
//!
//! - [`traits`] — Domain, 8 slot traits, Stage driver, reusable slots, RNG
//! - [`placement`] — placement domain types, cost math, slot impls
//! - [`routing`] — routing domain types, graphs, slot impls
//! - [`digest`] — constraint digestion: raw constraint crate → placement/routing-ready views
//! - [`block`] — `run_block()` feedback-loop orchestrator

mod traits;
pub use traits::*;

pub mod placement;
pub mod routing;
pub mod digest;
pub mod block;
