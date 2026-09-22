//! Routing-tier rules, scored against [`pnr_core::Routes`] (gr, dr).

pub mod align;
pub mod antenna;
pub mod coupling;
pub mod crosstalk;
pub mod differential;
pub mod parasitic;

pub use align::StraightNet;
pub use antenna::Antenna;
pub use coupling::CouplingBudget;
pub use crosstalk::CrosstalkExclusion;
pub use differential::Differential;
pub use parasitic::ParasiticBudget;
