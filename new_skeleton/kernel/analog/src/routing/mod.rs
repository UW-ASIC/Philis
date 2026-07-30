//! # Routing-tier rules — **Net arity**, scored against [`crate::Routes`].
//!
//! Consumed by `backend/gr` and `backend/dr`. Scored rules are small `Copy`
//! [`crate::Rule`]s with `On = Routes`, registered in [`crate::apply_routing`].
//! [`current_flow::CurrentFlowTag`] is not scored — it is a directional tag that
//! *shapes* how the others apply. One item per file, with textbook provenance.

pub mod align;
pub mod antenna;
pub mod coupling;
pub mod crosstalk;
pub mod current_flow;
pub mod differential;
pub mod parasitic;

pub use align::StraightNet;
pub use antenna::Antenna;
pub use coupling::CouplingBudget;
pub use crosstalk::CrosstalkExclusion;
pub use current_flow::{CurrentDir, CurrentFlowTag};
pub use differential::Differential;
pub use parasitic::ParasiticBudget;
